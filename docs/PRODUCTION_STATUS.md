# Production status — Pi 5

| Gate | Status |
|------|--------|
| `cargo test --workspace` | Required green |
| `make accept-full` (HTTP) | Required green on CI / dev |
| Soak 120 s (CI) | Required green |
| Soak 3600 s (Pi hardware) | Required for venue sign-off |
| `docs/acceptance/RESULTS-pi.md` | Manual sign-off on real hardware |

## Operator surfaces

| Surface | v1 |
|---------|-----|
| Native egui touch UI | Yes |
| Loopback HTTP (`127.0.0.1:8080`) | Yes (acceptance / soak only) |
| USB keyboard hotkeys | Yes |
| GPIO | v1.1 backlog |
| Flutter / gRPC | Removed |

## Known limitations

- Multi-segment replay at 0.5× may skip rate on concat (documented for operators)
- `global-hotkey` on Pi Wayland may need evdev fallback (touch UI primary)
