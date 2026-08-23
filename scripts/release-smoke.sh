#!/bin/bash
set -euo pipefail

target=${1:?target is required}
dist=${2:?dist directory is required}
ext=""
case "$target" in *windows*) ext=".exe" ;; esac
binary=$dist/soksak-sidecar-terminal-ghostty$ext
test -x "$binary" || { echo "staged executable is missing: $binary" >&2; exit 1; }
home=$RUNNER_TEMP/soksak-ghostty-smoke-home
runtime=$RUNNER_TEMP/soksak-ghostty-smoke-runtime
mkdir -p "$home" "$runtime"
stdout=$RUNNER_TEMP/soksak-ghostty-smoke.stdout
stderr=$RUNNER_TEMP/soksak-ghostty-smoke.stderr
set +e
"$binary" -home "$home" -runtime "$runtime" >"$stdout" 2>"$stderr"
status=$?
set -e
test "$status" -ne 0
grep -F "PTY token unreadable" "$stderr" >/dev/null
test ! -s "$stdout"
