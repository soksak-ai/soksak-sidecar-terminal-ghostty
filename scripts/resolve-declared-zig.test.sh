#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT HUP INT TERM
ambient=$fixture/ambient
versions=$fixture/versions
exact=$versions/zig-aarch64-macos-0.16.0/zig
mkdir -p "$ambient" "$(dirname -- "$exact")"
printf '%s\n' '#!/bin/sh' 'printf "0.15.2\n"' > "$ambient/zig"
printf '%s\n' '#!/bin/sh' 'printf "0.16.0\n"' > "$exact"
chmod +x "$ambient/zig" "$exact"

selected=$(SOKSAK_ZIG_ROOT="$versions" PATH="$ambient:/usr/bin:/bin" \
  "$root/scripts/resolve-declared-zig.sh" aarch64-apple-darwin 0.16.0)
[ "$selected" = "$exact" ]
[ "$($selected version)" = 0.16.0 ]

if SOKSAK_ZIG_ROOT="$fixture/missing" PATH="$ambient:/usr/bin:/bin" \
  "$root/scripts/resolve-declared-zig.sh" aarch64-apple-darwin 0.16.0 > "$fixture/out" 2>&1; then
  echo 'resolver accepted ambient Zig 0.15 for a declared Zig 0.16 build' >&2
  exit 1
fi
grep -F 'ZIG_TOOLCHAIN_MISSING: target=aarch64-apple-darwin version=0.16.0' "$fixture/out" >/dev/null
