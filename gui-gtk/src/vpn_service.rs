// SPDX-License-Identifier: LGPL-2.1-or-later
//
// VPN service layer — talks to the openlawsvpn-daemon over D-Bus instead of
// calling libopenlawsvpn via C FFI.  The public types (VpnEvent, VpnState,
// VpnCommand) and the VpnService API are unchanged; only the transport changes.
//
// Architecture:
//   GTK main thread (glib executor)
//     ↕ futures_channel::mpsc
//   Tokio task (background thread)
//     ↕ zbus D-Bus proxy  →  com.openlawsvpn.Daemon  →  openlawsvpn-daemon

use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt as _;
use zbus::{proxy, Connection};

// ── Public event/command types (unchanged from FFI version) ─────────────────

#[derive(Debug, Clone)]
pub enum VpnEvent {
    StateChanged(VpnState),
    LogLine(String),
    StatsUpdate { bytes_sent: u64, bytes_recv: u64, uptime_secs: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum VpnState {
    Idle,
    Connecting,
    WaitingSaml { saml_url: String },
    Connected { server_ip: String, assigned_ip: String },
    Disconnecting,
    NeedReauth,
    Error(String),
}

#[derive(Debug)]
pub enum VpnCommand {
    Connect { config_path: String },
    Disconnect,
}

// ── D-Bus proxy (generated at compile time by zbus) ─────────────────────────

#[proxy(
    interface = "com.openlawsvpn.Daemon",
    default_service = "com.openlawsvpn.Daemon",
    default_path = "/com/openlawsvpn/Daemon"
)]
trait VpnDaemon {
    fn connect(&self, profile_path: &str) -> zbus::Result<()>;
    fn disconnect(&self) -> zbus::Result<()>;
    fn status(&self) -> zbus::Result<(String, String, String)>;

    #[zbus(signal)]
    fn state_changed(&self, state: &str, server_ip: &str, assigned_ip: &str) -> zbus::Result<()>;
    #[zbus(signal)]
    fn log_line(&self, line: &str) -> zbus::Result<()>;
    #[zbus(signal)]
    fn stats_update(&self, bytes_sent: u64, bytes_recv: u64, uptime_secs: u64) -> zbus::Result<()>;
    #[zbus(signal)]
    fn saml_required(&self, url: &str) -> zbus::Result<()>;
}

// ── VpnService ───────────────────────────────────────────────────────────────

pub struct VpnService {
    pub cmd_tx: tokio::sync::mpsc::Sender<VpnCommand>,
    pub rt_handle: tokio::runtime::Handle,
    event_rx: std::cell::RefCell<Option<UnboundedReceiver<VpnEvent>>>,
}

impl VpnService {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<VpnCommand>(8);
        let (event_tx, event_rx) = mpsc::unbounded::<VpnEvent>();
        let (handle_tx, handle_rx) = std::sync::mpsc::channel::<tokio::runtime::Handle>();

        std::thread::spawn(move || {
            service_thread(cmd_rx, event_tx, handle_tx);
        });

        let rt_handle = handle_rx.recv().expect("service thread did not send Handle");

        Self {
            cmd_tx,
            rt_handle,
            event_rx: std::cell::RefCell::new(Some(event_rx)),
        }
    }

    pub fn take_event_rx(&self) -> UnboundedReceiver<VpnEvent> {
        self.event_rx
            .borrow_mut()
            .take()
            .expect("take_event_rx() called more than once")
    }
}

// ── Background service thread ────────────────────────────────────────────────

fn service_thread(
    mut cmd_rx: tokio::sync::mpsc::Receiver<VpnCommand>,
    event_tx: UnboundedSender<VpnEvent>,
    handle_tx: std::sync::mpsc::Sender<tokio::runtime::Handle>,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio rt");

    handle_tx.send(rt.handle().clone()).ok();

    rt.block_on(async move {
        // Connect to session bus once; reuse for all commands.
        let dbus_conn = match Connection::session().await {
            Ok(c) => c,
            Err(e) => {
                emit(&event_tx, VpnEvent::StateChanged(VpnState::Error(
                    format!("Cannot connect to session bus: {e}"),
                )));
                return;
            }
        };

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                VpnCommand::Connect { config_path } => {
                    handle_connect(&dbus_conn, &event_tx, config_path).await;
                }
                VpnCommand::Disconnect => {
                    handle_disconnect(&dbus_conn, &event_tx).await;
                }
            }
        }
    });
}

async fn handle_connect(
    dbus_conn: &Connection,
    event_tx: &UnboundedSender<VpnEvent>,
    config_path: String,
) {
    let proxy = match VpnDaemonProxy::new(dbus_conn).await {
        Ok(p) => p,
        Err(e) => {
            emit(event_tx, VpnEvent::StateChanged(VpnState::Error(
                format!("D-Bus proxy: {e} — is openlawsvpn-daemon running?"),
            )));
            return;
        }
    };

    // Subscribe to all signals before calling Connect so we never miss the first one.
    let mut state_stream = match proxy.receive_state_changed().await {
        Ok(s) => s,
        Err(e) => {
            emit(event_tx, VpnEvent::StateChanged(VpnState::Error(format!("subscribe StateChanged: {e}"))));
            return;
        }
    };
    let mut log_stream = match proxy.receive_log_line().await {
        Ok(s) => s,
        Err(e) => {
            emit(event_tx, VpnEvent::StateChanged(VpnState::Error(format!("subscribe LogLine: {e}"))));
            return;
        }
    };
    let mut stats_stream = match proxy.receive_stats_update().await {
        Ok(s) => s,
        Err(e) => {
            emit(event_tx, VpnEvent::StateChanged(VpnState::Error(format!("subscribe StatsUpdate: {e}"))));
            return;
        }
    };
    let mut saml_stream = match proxy.receive_saml_required().await {
        Ok(s) => s,
        Err(e) => {
            emit(event_tx, VpnEvent::StateChanged(VpnState::Error(format!("subscribe SAMLRequired: {e}"))));
            return;
        }
    };

    // Tell the daemon to connect.
    if let Err(e) = proxy.connect(&config_path).await {
        emit(event_tx, VpnEvent::StateChanged(VpnState::Error(
            format!("daemon Connect: {e}"),
        )));
        return;
    }

    // Drain all four signal streams until we reach a terminal state (idle / error).
    let tx = event_tx.clone();
    loop {
        tokio::select! {
            Some(sig) = state_stream.next() => {
                if let Ok(args) = sig.args() {
                    let state = parse_state(args.state, args.server_ip, args.assigned_ip);
                    let terminal = matches!(state, VpnState::Idle | VpnState::Error(_));
                    emit(&tx, VpnEvent::StateChanged(state));
                    if terminal { return; }
                }
            }
            Some(sig) = log_stream.next() => {
                if let Ok(args) = sig.args() {
                    emit(&tx, VpnEvent::LogLine(args.line.to_string()));
                }
            }
            Some(sig) = stats_stream.next() => {
                if let Ok(args) = sig.args() {
                    emit(&tx, VpnEvent::StatsUpdate {
                        bytes_sent: args.bytes_sent,
                        bytes_recv: args.bytes_recv,
                        uptime_secs: args.uptime_secs,
                    });
                }
            }
            Some(sig) = saml_stream.next() => {
                if let Ok(args) = sig.args() {
                    // Open browser; daemon runs the ACS server.
                    std::process::Command::new("xdg-open").arg(args.url).spawn().ok();
                    emit(&tx, VpnEvent::StateChanged(VpnState::WaitingSaml {
                        saml_url: args.url.to_string(),
                    }));
                }
            }
            else => break,
        }
    }
}

async fn handle_disconnect(dbus_conn: &Connection, event_tx: &UnboundedSender<VpnEvent>) {
    let proxy = match VpnDaemonProxy::new(dbus_conn).await {
        Ok(p) => p,
        Err(e) => {
            emit(event_tx, VpnEvent::LogLine(format!("D-Bus proxy error: {e}")));
            return;
        }
    };
    emit(event_tx, VpnEvent::StateChanged(VpnState::Disconnecting));
    if let Err(e) = proxy.disconnect().await {
        emit(event_tx, VpnEvent::LogLine(format!("daemon Disconnect: {e}")));
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_state(state: &str, server_ip: &str, assigned_ip: &str) -> VpnState {
    match state {
        "idle" => VpnState::Idle,
        "connecting" => VpnState::Connecting,
        "waiting_saml" => VpnState::WaitingSaml { saml_url: String::new() },
        "connected" => VpnState::Connected {
            server_ip: server_ip.to_string(),
            assigned_ip: assigned_ip.to_string(),
        },
        "disconnecting" => VpnState::Disconnecting,
        "error" => VpnState::Error(assigned_ip.to_string()),
        other => VpnState::Error(format!("unknown daemon state: {other}")),
    }
}

fn emit(tx: &UnboundedSender<VpnEvent>, event: VpnEvent) {
    tx.unbounded_send(event).ok();
}
