# openlawsvpn — AWS Client VPN for Linux

> **Archived:** This repo contains the original C++ library (`libopenlawsvpn`) and Flutter GUI. Active development has moved to [go-openlawsvpn](https://github.com/openlawsvpn/go-openlawsvpn) (pure Go daemon + GTK4 GUI).

A specialized OpenVPN 3 client for AWS Client VPN endpoints with native SAML/SSO authentication. Supports Okta, Azure AD, Google Workspace, and any SAML 2.0 IdP.

## Key Features

- **Native SAML support** — handles the AWS CRV1 protocol: opens a browser, captures the token, completes the handshake automatically.
- **GTK4 + libadwaita GUI** — native Linux desktop app with system tray (StatusNotifierItem), profile management, and live connection log.
- **Rootless operation** — integrates with `openvpn3-linux` D-Bus services; no root or `sudo` required.
- **Standalone mode** — fully static CLI binary, no dependencies; runs with root or `CAP_NET_ADMIN`+`CAP_NET_RAW`.
- **Headless / CI-friendly** — CLI works on servers and in pipelines with no display.

## Install

### Fedora / RHEL (COPR) — amd64 · arm64 · ppc64le · FC43/44/rawhide

```bash
# CLI (rootless D-Bus mode)
sudo dnf copr enable vorona/openlawsvpn -y
sudo dnf install openlawsvpn-cli openvpn3 -y

# GTK4 GUI
sudo dnf install openlawsvpn-gui -y
```

### Static binary (amd64 / arm64)

Pre-built static CLI binaries are attached to each [GitHub Release](https://github.com/openlawsvpn/openlawsvpn/releases):

```bash
BASE=https://github.com/openlawsvpn/openlawsvpn/releases/latest/download
curl -LO $BASE/openlawsvpn-cli-linux-amd64   # or openlawsvpn-cli-linux-arm64
chmod +x openlawsvpn-cli-linux-amd64
sudo mv openlawsvpn-cli-linux-amd64 /usr/local/bin/openlawsvpn-cli
```

Static binaries run in standalone mode only (requires root or `CAP_NET_ADMIN`+`CAP_NET_RAW`).
For ppc64le or rootless D-Bus mode use the COPR packages.

## Build from source

### Requirements

- **C++ core**: `cmake`, `ninja-build`, `gcc-c++`, `openssl-devel`, `lz4-devel`, `glib2-devel`
- **GUI**: `gtk4-devel`, `libadwaita-devel`, `cargo`, `rust-zbus+tokio-devel`
- **Rootless runtime**: `openvpn3` + `gdbuspp` from COPR

### Build

```bash
git clone --recurse-submodules https://github.com/openlawsvpn/openlawsvpn
make linux          # C++ CLI + shared lib (D-Bus mode)
make gui            # GTK4 GUI (requires make linux first)
make linux-static   # fully static CLI via Alpine container (requires podman or docker)
```

Artifacts:
- CLI (dynamic): `build/bin/openlawsvpn-cli`
- CLI (static):  `build/linux-static/openlawsvpn-cli-static`
- GUI:           `build/gui/openlawsvpn-gui`

## Usage

### GUI

```bash
openlawsvpn-gui
# or from build directory:
LD_LIBRARY_PATH=build/lib build/gui/openlawsvpn-gui
```

1. Add a `.ovpn` profile in the Connect tab.
2. Click **Connect** — a browser opens for SAML login.
3. After authentication the tunnel comes up automatically.

### CLI

```bash
# Rootless (D-Bus mode — requires openvpn3 running)
openlawsvpn-cli connect ~/Downloads/client-config.ovpn

# Standalone (root / static binary)
sudo openlawsvpn-cli connect /path/to/client-config.ovpn --standalone
```

## Project Structure

```
linux/          C++ core, D-Bus client, shared lib, static CLI
gui-gtk/        GTK4 + libadwaita + Rust desktop GUI
provision/
  spec/
    openvpn3/   openvpn3 RPM spec (with FC44/GCC-16 patch)
openvpn3-core/  openvpn3-core submodule (header-only library)
website/        openlawsvpn.com static site
```

## Android

An Android app is pending [F-Droid](https://f-droid.org/) review.
See the [openlawsvpn-android](https://github.com/openlawsvpn/openlawsvpn-android) repo.
iOS / macOS support is planned for a future release.

## Contributing

Contributions are welcome. See [CONTRIBUTORS.md](CONTRIBUTORS.md) for the CLA and getting-started guide.

## Security

See [SECURITY.md](SECURITY.md) for responsible disclosure instructions or email [security@openlawsvpn.com](mailto:security@openlawsvpn.com).

## License

openlawsvpn is licensed under the **GNU Lesser General Public License v2.1 or later** ([LGPL-2.1-or-later](LICENSE)).

You may use this software freely in any environment — cloud VMs, containers, CI/CD pipelines, automation scripts — without source-disclosure obligations on your own code. See [LICENSE_USAGE_EXCEPTION](LICENSE_USAGE_EXCEPTION) for the explicit grant.

Modifications to openlawsvpn itself must be released under LGPL-2.1-or-later.

Commercial licenses (no copyleft obligations, enterprise support SLA) are available. Contact the maintainers.
