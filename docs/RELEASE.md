# Release process — Pi 5

1. Complete [HARDWARE_ACCEPTANCE.md](HARDWARE_ACCEPTANCE.md) and [acceptance/RESULTS-pi.md](acceptance/RESULTS-pi.md) on a venue Pi.
2. Run `make finalize` (or CI green on `main`).
3. Bump `[workspace.package].version` in root `Cargo.toml`.
4. Commit and tag `vX.Y.Z`.
5. Push tag → GitHub Actions uploads **pi5-aarch64** tarball.

## CI artifacts

| Workflow | Output |
|----------|--------|
| `ci.yml` | aarch64 build + tests |
| `acceptance.yml` | HTTP MVP + soak smoke |
| `release.yml` | `InstantReplay-*-pi5-aarch64.tar.gz` |

## Operator bundle

Each release includes `OPERATOR.md`, `CONFIG.md`, and `PI_ONLY.md` beside the tarball in GitHub Releases.
