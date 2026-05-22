# Pi 5 only — product scope

Instant Replay is a **Raspberry Pi 5 appliance** for table-tennis venues. This repository no longer targets macOS, Windows, or generic Linux desktops.

## In scope

- Raspberry Pi 5, **Raspberry Pi OS 64-bit** (Bookworm)
- UVC / Cam Link capture via **V4L2**
- Rolling buffer on **USB3 SSD** (`/var/lib/instant-replay`)
- Live + replay on **audience HDMI** (fullscreen)
- Operator control: **Pi touch display** (local web UI) + **USB keyboard** hotkeys
- **systemd** autostart (`--appliance`)

## Out of scope (non-goals)

- macOS, Windows, Intel/AMD Linux desktop builds
- Flutter control app
- gRPC / remote control from another machine
- GPIO physical buttons (planned v1.1)
- Cloud sync, multi-venue management

## GitHub

**https://github.com/madgunman/InstaReplayPi.git** — push this tree, tag `v*` for Pi tarball releases, install on device via [GITHUB_PI.md](GITHUB_PI.md).

## Docs map

| Doc | Purpose |
|-----|---------|
| [GITHUB_PI.md](GITHUB_PI.md) | Clone / release install on Pi |
| [PI_DEPLOYMENT.md](PI_DEPLOYMENT.md) | Install, SSD, v4l2 tuning |
| [OPERATOR.md](OPERATOR.md) | Match-day touch + keyboard |
| [HARDWARE_ACCEPTANCE.md](HARDWARE_ACCEPTANCE.md) | Venue sign-off |
| [CONFIG.md](CONFIG.md) | `/etc/instant-replay/config.toml` |
| [archive/](archive/) | Retired cross-platform material |

## Pre-migration tag

Cross-platform release line is preserved as git tag **`v0.1.0-cross-platform`** (create before merging Pi-only default branch if not already tagged).
