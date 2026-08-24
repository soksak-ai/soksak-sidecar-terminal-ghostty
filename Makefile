SHELL := /bin/sh

BUILD_DEPENDENCY_ROOT := target/build-dependencies/ghostty-vt-sdk
OUT ?= dist

.PHONY: require-target preflight prepare build verify stage

require-target:
	@test '$(origin TARGET)' = 'command line' && test -n '$(TARGET)' || { echo 'TARGET must be an explicit Make command-line variable' >&2; exit 2; }

preflight: require-target
	@scripts/check-build-environment.sh '$(TARGET)'
	@soksak-validate build-dependencies build-dependencies.json --dependency ghostty-vt-sdk --target '$(TARGET)' >/dev/null

prepare: preflight
	@scripts/prepare-ghostty-sdk.sh '$(TARGET)' '$(BUILD_DEPENDENCY_ROOT)'

build: prepare
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' cargo build --locked --release --target '$(TARGET)' --bin soksak-sidecar-terminal-ghostty

verify: build
	@node scripts/check-build-config.mjs
	@soksak-validate build-receipt '$(BUILD_DEPENDENCY_ROOT)/receipts/$(TARGET).json' --dependencies build-dependencies.json --output-root '$(BUILD_DEPENDENCY_ROOT)'
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' scripts/gate.sh '$(TARGET)'

stage: build
	@SOKSAK_BUILD_DEPENDENCY_ROOT='$(CURDIR)/$(BUILD_DEPENDENCY_ROOT)' scripts/stage-built.sh '$(OUT)' '$(TARGET)'
