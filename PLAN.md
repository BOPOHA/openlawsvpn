# openlawsvpn — Project Status

> **This file tracks the main `openlawsvpn` repo (C++ core + GTK4 GUI).**
> For the Go relay stack see `go-openlawsvpn/PLAN.md`.
> For the roadmap across all repos see `openlawsvpn-private/ROADMAP.md`.

---

## Current state  (2026-05-03)

| Component | Status |
|-----------|--------|
| C++ core (`libopenlawsvpn`) | ✅ Working — CRV1/SAML, Phase 1 + Phase 2, sticky IP |
| CLI standalone mode | ✅ Working — static binary, root / CAP\_NET\_ADMIN |
| CLI D-Bus mode | ✅ Working — rootless via openvpn3-linux |
| GTK4 GUI (`gui-gtk/`) | ✅ Working — Rust + libadwaita, D-Bus proxy to daemon |
| RPM packaging (COPR) | ✅ Working — FC43/44/rawhide, amd64/arm64/ppc64le |
| Website (openlawsvpn.com) | ✅ Live |

---

## Architecture

```
[openlawsvpn-gui  Rust/GTK4]  ←── D-Bus ──→  [openlawsvpn-daemon  Go]
                                                      │
                                              [go-openlawsvpn client.go]
                                                      │
                                              [Linux TUN / netlink / DNS]
```

- **C++ core** (`linux/`) — `libopenlawsvpn.so` + CLI binary. Handles protocol, SAML, D-Bus.
- **GTK4 GUI** (`gui-gtk/`) — Rust, libadwaita. Communicates with Go daemon via D-Bus session bus. Zero C dependencies in GUI binary.
- **Go daemon** (`go-openlawsvpn/cmd/daemon`) — systemd user service, holds `CAP_NET_ADMIN`. See `go-openlawsvpn/PLAN.md`.
- **Relay CLI** (`go-openlawsvpn/cmd/cli`) — headless agent for CI/CD; registers with relay server, receives Phase 2 credentials from mobile app.

---

## License

LGPL-2.1-or-later with usage exception. See `LICENSE` and `LICENSE_USAGE_EXCEPTION`.

---

## What was replaced

| Old | New | When |
|-----|-----|------|
| Flutter/Dart GUI (Material 3) | Rust GTK4 + libadwaita (`gui-gtk/`) | 2026-04 |
| Dart FFI → libopenlawsvpn | D-Bus proxy → openlawsvpn-daemon | 2026-04 |
| `openvpn3-linux` as runtime dep | Go daemon (`go-openlawsvpn/cmd/daemon`) | 2026-04 |
