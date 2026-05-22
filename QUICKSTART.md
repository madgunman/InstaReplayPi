# Quick start — Raspberry Pi 5

Repository: **https://github.com/madgunman/InstaReplayPi.git**

Full GitHub install paths: [docs/GITHUB_PI.md](docs/GITHUB_PI.md)

## Fastest path (on the Pi — one command)

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-instant-replay.sh | bash
```

Set **Desktop Autologin** for your user, then `sudo reboot`.

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

Native operator window opens on the Pi touch display (no browser).

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
