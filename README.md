# Instant Replay

**Raspberry Pi 5 appliance only** — table-tennis instant replay: live 1080p50/60 on audience HDMI, rolling ~20 s buffer on USB3 SSD, Mark / Replay / Last / Live via **touch UI** and **USB keyboard hotkeys**.

**GitHub (Pi):** [github.com/madgunman/InstaReplayPi](https://github.com/madgunman/InstaReplayPi) — clone, releases, and `scripts/install-from-github.sh` on the device. See [docs/GITHUB_PI.md](docs/GITHUB_PI.md).

> Cross-platform Flutter/gRPC builds are archived. See [docs/PI_ONLY.md](docs/PI_ONLY.md).

## Architecture

- `replay-engine` — GStreamer capture (`v4l2src`), rolling buffer, replay, HDMI program output
- Embedded HTTP on `127.0.0.1:8080` — touch UI in `assets/touch/` (Chromium kiosk on Pi display)
- Video never passes through the browser

## Quick start (Pi 5)

See **[QUICKSTART.md](QUICKSTART.md)** and **[docs/PI_DEPLOYMENT.md](docs/PI_DEPLOYMENT.md)**.

```bash
./scripts/install-deps-raspberry-pi.sh   # on the Pi
cargo build -p replay-engine --release
sudo cp config/default.toml /etc/instant-replay/config.toml   # edit device + SSD path
./target/release/replay-engine --appliance
# Touch UI: http://127.0.0.1:8080
```

Production install (one command on the Pi):

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

See [docs/GITHUB_PI.md](docs/GITHUB_PI.md).

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| M | Mark start |
| R | Replay from mark (or last X) |
| Space | Replay last 10s |
| L | Return live |
| C | Clear mark |

Requires Pi desktop session (X11/Wayland) for global hotkeys.

## MVP acceptance

```bash
make finalize             # tests + mvp_accept-full + soak smoke (120s)
make accept-full          # start engine + HTTP smoke tests
SOAK_SECONDS=3600 make soak   # 60 min stability (engine must be running)
make doctor-pi              # on-device health check
```

See [docs/PRODUCTION_STATUS.md](docs/PRODUCTION_STATUS.md), [docs/MVP_CHECKLIST.md](docs/MVP_CHECKLIST.md), and [docs/HARDWARE_ACCEPTANCE.md](docs/HARDWARE_ACCEPTANCE.md).

## Packaging

```bash
make package-pi    # aarch64 tarball + install-on-pi.sh
```

Tag `v*` for GitHub Releases (Pi artifact only). See [docs/RELEASE.md](docs/RELEASE.md) and [docs/PACKAGING.md](docs/PACKAGING.md).

## Docs

- [docs/PI_ONLY.md](docs/PI_ONLY.md) — scope and non-goals
- [docs/OPERATOR.md](docs/OPERATOR.md) — venue runbook
- [docs/CONFIG.md](docs/CONFIG.md) — `/etc/instant-replay/config.toml`
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/PI_DEPLOYMENT.md](docs/PI_DEPLOYMENT.md)
- [docs/HARDWARE_ACCEPTANCE.md](docs/HARDWARE_ACCEPTANCE.md)

## License

See repository license file.
