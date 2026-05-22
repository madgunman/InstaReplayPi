# Quick Start

## One-time setup

```bash
./scripts/install-deps-macos.sh
dart pub global activate protoc_plugin
export PATH="$PATH:$HOME/.pub-cache/bin"
make proto
cargo build -p replay-engine --release
cd flutter/replay_control && flutter pub get
```

Add to `~/.zshrc`:

```bash
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:$PKG_CONFIG_PATH"
export GST_PLUGIN_PATH="/opt/homebrew/lib/gstreamer-1.0:$GST_PLUGIN_PATH"
export PATH="$PATH:$HOME/.pub-cache/bin"
```

## Run

**Terminal 1 — engine**

```bash
./scripts/run-mac.sh test    # no camera
# or
./scripts/run-mac.sh live    # Cam Link / webcam
```

**Terminal 2 — control UI**

```bash
./scripts/run-mac.sh ui
```

If the UI shows **Operation not permitted** connecting to `127.0.0.1:50051`, rebuild after entitlements update (network client is enabled in `macos/Runner/*.entitlements`):

```bash
cd flutter/replay_control && flutter clean && flutter run -d macos
```

Click **Start Live**, then use buttons or keys **M / R / Space / L / C**.

## Stop

```bash
./scripts/run-mac.sh stop
```

## Config

`~/Library/Application Support/InstantReplay/config.json`

## Distributable bundle

```bash
make package-macos
open dist/InstantReplay-macos-0.1.0-arm64/Start\ Instant\ Replay.command
```

See [docs/PACKAGING.md](docs/PACKAGING.md) for Linux, Windows, and Pi.

## Linux

```bash
./scripts/install-deps-linux.sh
./scripts/run-linux.sh test    # terminal 1
./scripts/run-linux.sh ui      # terminal 2
```

Or after `.deb` install: `instant-replay-launch` from the applications menu.

## Windows

```powershell
.\scripts\run-windows.ps1 test
.\scripts\run-windows.ps1 ui
```

Or `dist\InstantReplay-windows-*\Start Instant Replay.bat` after `packaging\windows\bundle.ps1`.

## Raspberry Pi 5

```bash
./scripts/package-pi.sh && ./install-on-pi.sh
sudo systemctl start replay-engine
```

Engine-only hotkeys work without Flutter. See [docs/PI_DEPLOYMENT.md](docs/PI_DEPLOYMENT.md).

## Full MVP validation

See [docs/MVP_CHECKLIST.md](docs/MVP_CHECKLIST.md).
