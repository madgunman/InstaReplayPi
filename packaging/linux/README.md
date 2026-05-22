# Linux packaging (x86_64 + Pi)

## Tarball / `.deb`

```bash
./scripts/package-linux.sh
```

- **Tarball:** `dist/InstantReplay-<version>-linux-<arch>.tar.gz`
- **Debian:** `dist/instant-replay_<version>_<arch>.deb` (when `dpkg-deb` is available)

## systemd

```bash
sudo cp systemd/replay-engine.service /etc/systemd/system/
# Pi venue: create drop-in for user
sudo mkdir -p /etc/systemd/system/replay-engine.service.d
echo -e '[Service]\nUser=pi' | sudo tee /etc/systemd/system/replay-engine.service.d/user.conf
sudo cp packaging/linux/replay-engine.default /etc/default/replay-engine
sudo systemctl daemon-reload
sudo systemctl enable --now replay-engine
```

The unit runs `/usr/bin/instant-replay --appliance` (wrapper sets GStreamer paths).

## Desktop launcher

After `.deb` install or manual copy:

```bash
sudo cp packaging/linux/instant-replay.desktop /usr/share/applications/
sudo cp packaging/linux/instant-replay-launch.sh /usr/bin/instant-replay-launch
sudo chmod +x /usr/bin/instant-replay-launch
```

Buffer path for venues: `/var/lib/instant-replay` (see [docs/PI_DEPLOYMENT.md](../../docs/PI_DEPLOYMENT.md)).

## Hardware sign-off

Record results in [docs/acceptance/RESULTS-linux.md](../../docs/acceptance/RESULTS-linux.md).
