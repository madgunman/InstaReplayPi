.PHONY: sync-version verify-package test test-ignored build-engine soak accept accept-full finalize signoff package-pi doctor-pi

PKG_CONFIG_PATH ?= /opt/homebrew/lib/pkgconfig:/usr/lib/pkgconfig

export PKG_CONFIG_PATH

sync-version:
	./scripts/sync-version.sh

verify-package:
	./scripts/verify-package.sh

test:
	cargo test --workspace

test-ignored:
	cargo test -p replay-engine --test headless_flow -- --ignored

build-engine:
	cargo build -p replay-engine --release

soak:
	SOAK_SECONDS=3600 ./scripts/soak_test.sh

soak-ci:
	SOAK_SECONDS=120 SOAK_INTERVAL=10 ./scripts/soak_test.sh

accept:
	./scripts/mvp_accept.sh

accept-full:
	./scripts/mvp_accept-full.sh

finalize:
	chmod +x scripts/finalize.sh
	./scripts/finalize.sh

signoff:
	./scripts/hardware_signoff.sh

package-pi:
	./scripts/package-pi.sh

doctor-pi:
	./scripts/doctor-pi.sh
