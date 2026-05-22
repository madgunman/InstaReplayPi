# Quick start — Raspberry Pi 5

Repository: **https://github.com/madgunman/InstaReplayPi.git**

Full GitHub install paths: [docs/GITHUB_PI.md](docs/GITHUB_PI.md)

## Fastest path (on the Pi, after you pushed a `v*` release)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-from-github.sh -o /tmp/install-ir.sh
chmod +x /tmp/install-ir.sh
/tmp/install-ir.sh --release
# Autostart is configured by install — set Desktop Autologin, then:
sudo reboot
```

## Clone and build on the Pi

```bash
git clone https://github.com/madgunman/InstaReplayPi.git
cd InstaReplayPi
./scripts/install-deps-raspberry-pi.sh
./scripts/install-from-github.sh --build
```

Or manually:

```bash
git clone https://github.com/madgunman/InstaReplayPi.git
cd InstaReplayPi
./scripts/install-deps-raspberry-pi.sh
cargo build -p replay-engine --release
```

## Config

```bash
sudo mkdir -p /etc/instant-replay /var/lib/instant-replay
sudo cp config/default.toml /etc/instant-replay/config.toml
# Edit: device_id, buffer_path (USB SSD), resolution/fps
```

## Run (development)

```bash
./target/release/replay-engine --appliance
```

Touch UI: **http://127.0.0.1:8080**

Test pattern (no camera):

```bash
./target/release/replay-engine --test
```

## Verify

```bash
./scripts/doctor-pi.sh
./scripts/mvp_accept-full.sh    # optional automated smoke
```

Hardware sign-off: [docs/HARDWARE_ACCEPTANCE.md](docs/HARDWARE_ACCEPTANCE.md)
