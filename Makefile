SHELL := /bin/sh

BUILD_DEPENDENCY_ROOT := target/build-dependencies/ghostty-vt-sdk
OUT ?= dist

.PHONY: require-target preflight lock prepare build verify stage benchmark

require-target:
	@test '$(origin TARGET)' = 'command line' && test -n '$(TARGET)' || { echo 'TARGET must be an explicit Make command-line variable' >&2; exit 2; }

preflight: require-target
	@scripts/check-build-environment.sh '$(TARGET)'
	@soksak-validate build-dependencies build-dependencies.json --dependency ghostty-vt-sdk --target '$(TARGET)' >/dev/null

lock: preflight
	@cargo metadata --format-version 1 > /dev/null

prepare: preflight
	@scripts/prepare-ghostty-sdk.sh '$(TARGET)' '$(BUILD_DEPENDENCY_ROOT)'

build: prepare
	@node scripts/check-cursor-contract.mjs
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' cargo build --locked --release --target '$(TARGET)' --bin soksak-sidecar-terminal-ghostty

verify: build
	@node scripts/check-build-config.mjs
	@soksak-validate build-receipt '$(BUILD_DEPENDENCY_ROOT)/receipts/$(TARGET).json' --dependencies build-dependencies.json --output-root '$(BUILD_DEPENDENCY_ROOT)'
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' scripts/gate.sh '$(TARGET)'

stage: build
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' scripts/stage-built.sh '$(OUT)' '$(TARGET)'

benchmark: verify
	@case '$(BENCH_OUT)' in /*) ;; *) echo 'BENCH_OUT must be an explicit absolute output directory' >&2; exit 2 ;; esac
	@test -x "$$SOKSAK_PTYD_BIN" || { echo 'SOKSAK_PTYD_BIN must name the product-owned PTY executable' >&2; exit 2; }
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' SOKSAK_BENCH_OUT='$(BENCH_OUT)' cargo test --locked --release --target '$(TARGET)' --test bench -- --ignored --nocapture
