# Acceptance — macOS automated smoke

| Field | Value |
|-------|-------|
| Platform | macOS (Darwin) — automated smoke |
| Date | 2026-05-20 |
| Script | `./scripts/mvp_accept-full.sh` |
| Engine mode | `--test` (headless program output) |
| Result | **pass=24 fail=0 skip=0** (exit 0) |

## Prerequisites

```bash
brew install grpcurl jq   # or ./scripts/install-deps-macos.sh
cargo build -p replay-engine --release
./scripts/mvp_accept-full.sh
```

## Automated coverage (this run)

| Step | Result |
|------|--------|
| Health RPC reachable | PASS |
| ListDevices / ListDisplays / ListFormats / GetConfig | PASS |
| StartLive (test pattern) | PASS |
| Replay before buffer ready (rejected) | PASS |
| Rolling buffer (≥ 1.5s) | PASS |
| Health ready after live + buffer | PASS |
| Mark → ClearMark (no replay) | PASS |
| Mark → Replay from mark | PASS |
| ReturnLive | PASS |
| ReplayLast | PASS |
| Stop | PASS |

## Finalize gate

```bash
./scripts/finalize.sh
```

## Hardware sign-off still required

See [RESULTS-macos.md](RESULTS-macos.md) and [HARDWARE_ACCEPTANCE.md](../HARDWARE_ACCEPTANCE.md):

- Cam Link / UVC at 1080p50/60 on external HDMI  
- Hotkeys without Flutter  
- Cable disconnect / NO SIGNAL  
- 60 min soak: `SOAK_SECONDS=3600 ./scripts/soak_test.sh`  
- Camera permission: `./scripts/check-camera-macos.sh`
