#!/bin/sh
set -eu

[ "$#" -eq 2 ] && [ -n "$1" ] && [ -n "$2" ] || {
  echo 'usage: resolve-declared-zig.sh <target> <version>' >&2
  exit 78
}
target=$1
version=$2
case "$version" in *[!0-9.]*|.*|*..*|*.) echo "invalid Zig version: $version" >&2; exit 78 ;; esac
case "$target" in
  aarch64-apple-darwin) archive=aarch64-macos; executable=zig ;;
  x86_64-apple-darwin) archive=x86_64-macos; executable=zig ;;
  aarch64-unknown-linux-gnu) archive=aarch64-linux; executable=zig ;;
  x86_64-unknown-linux-gnu) archive=x86_64-linux; executable=zig ;;
  x86_64-pc-windows-msvc) archive=x86_64-windows; executable=zig.exe ;;
  *) echo "unsupported Zig host target: $target" >&2; exit 78 ;;
esac

matches() {
  candidate=$1
  [ -f "$candidate" ] && [ -x "$candidate" ] && [ ! -L "$candidate" ] &&
    [ "$("$candidate" version 2>/dev/null || true)" = "$version" ]
}

ambient=$(command -v zig 2>/dev/null || true)
case "$ambient" in
  /*) if matches "$ambient"; then printf '%s\n' "$ambient"; exit 0; fi ;;
esac

if [ -n "${SOKSAK_ZIG_ROOT:-}" ]; then
  zig_root=$SOKSAK_ZIG_ROOT
elif [ -n "${XDG_DATA_HOME:-}" ]; then
  zig_root=$XDG_DATA_HOME/zig
elif [ -n "${HOME:-}" ]; then
  zig_root=$HOME/.local/zig
else
  zig_root=
fi
case "$zig_root" in
  /*) ;;
  *) echo "ZIG_TOOLCHAIN_MISSING: target=$target version=$version" >&2; exit 78 ;;
esac
[ ! -L "$zig_root" ] || { echo "ZIG_TOOLCHAIN_REFUSED: root is a symbolic link: $zig_root" >&2; exit 78; }
declared=$zig_root/zig-$archive-$version/$executable
if matches "$declared"; then
  printf '%s\n' "$declared"
  exit 0
fi
echo "ZIG_TOOLCHAIN_MISSING: target=$target version=$version" >&2
exit 78
