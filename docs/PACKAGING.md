# Packaging — Raspberry Pi 5 only

## Build tarball

On a Pi 5 or `ubuntu-24.04-arm` CI runner:

```bash
make package-pi
# → dist/InstantReplay-<version>-pi5-aarch64.tar.gz
```

Contents:

| Path | Purpose |
|------|---------|
| `bin/replay-engine` | Main daemon |
| `bin/instant-replay` | Launcher (GStreamer env) |
| `assets/touch/` | Operator web UI |
| `systemd/replay-engine.service` | Appliance autostart |
| `systemd/instant-replay-kiosk.service` | Chromium fullscreen touch UI |
| `etc/instant-replay/config.toml.example` | Copy to `/etc/instant-replay/config.toml` |
| `install-on-pi.sh` | System install |
| `scripts/doctor-pi.sh` | Pre-match health check |

## Install on Pi

```bash
tar xzf InstantReplay-*-pi5-aarch64.tar.gz
cd InstantReplay-*-pi5-aarch64
./install-on-pi.sh
sudo systemctl start replay-engine
sudo systemctl enable --now instant-replay-kiosk   # optional
./scripts/doctor-pi.sh
```

## Version

```bash
./scripts/sync-version.sh    # prints Cargo workspace version
```

## Release

Push tag `v*` → GitHub Actions builds **pi5-aarch64** artifact only. See [RELEASE.md](RELEASE.md).

## Archived platforms

macOS, Windows, Linux desktop, and Flutter bundles are removed from this repo. Pre-migration tag: `v0.1.0-cross-platform` (if tagged on your clone).
