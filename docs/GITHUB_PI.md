# GitHub — Pi install from [InstaReplayPi](https://github.com/madgunman/InstaReplayPi)

Canonical repository for the Pi 5 appliance:

**https://github.com/madgunman/InstaReplayPi.git**

## One-time: push this project to GitHub

From your development machine (this tree):

```bash
cd /path/to/InstantReplay-Software
git init
git add .
git commit -m "Pi 5 appliance: replay-engine, touch UI, Pi packaging"
git branch -M main
git remote add origin https://github.com/madgunman/InstaReplayPi.git
git push -u origin main
```

Create a release (builds the Pi tarball in CI):

```bash
git tag v0.1.0
git push origin v0.1.0
```

Wait for **Actions → Release** to finish, then install on the Pi from the release asset.

## On the Raspberry Pi 5

### Option A — Release tarball (recommended)

After `v*` is published on GitHub:

```bash
curl -fsSL https://raw.githubusercontent.com/madgunman/InstaReplayPi/main/scripts/install-from-github.sh -o /tmp/install-ir.sh
chmod +x /tmp/install-ir.sh
/tmp/install-ir.sh --release
# or a specific tag:
/tmp/install-ir.sh --release v0.1.0
```

### Option B — Clone and build on the Pi

```bash
git clone https://github.com/madgunman/InstaReplayPi.git ~/InstaReplayPi
cd ~/InstaReplayPi
./scripts/install-from-github.sh --build
```

### Option C — Manual (same as QUICKSTART)

```bash
git clone https://github.com/madgunman/InstaReplayPi.git
cd InstaReplayPi
./scripts/install-deps-raspberry-pi.sh
make package-pi
cd dist/InstantReplay-*-pi5-aarch64
./install-on-pi.sh
sudo systemctl start replay-engine
sudo systemctl enable --now instant-replay-kiosk
```

## After install

1. Mount USB3 SSD at `/var/lib/instant-replay`
2. Edit `/etc/instant-replay/config.toml` (see [CONFIG.md](CONFIG.md))
3. `sudo systemctl start replay-engine`
4. Operator UI: **http://127.0.0.1:8080**
5. Health check: `./scripts/doctor-pi.sh` (from cloned repo) or see [PI_DEPLOYMENT.md](PI_DEPLOYMENT.md)

## Updating the Pi

**From release:**

```bash
/tmp/install-ir.sh --release v0.2.0
sudo systemctl restart replay-engine
```

**From git:**

```bash
cd ~/InstaReplayPi && git pull && ./scripts/install-from-github.sh --build
sudo systemctl restart replay-engine
```
