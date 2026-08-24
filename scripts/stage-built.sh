#!/bin/sh
set -eu

[ "$#" -eq 2 ] && [ -n "$1" ] && [ -n "$2" ] || { echo 'usage: stage-built.sh <out> <target>' >&2; exit 2; }
out=$1
target=$2
repository=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# An absolute candidate output is allowed only outside the source repository.
case "$out" in ''|/|.|*..*|"$repository"|"$repository"/*) echo 'stage output is unsafe or inside the source repository' >&2; exit 2 ;; esac
name=soksak-sidecar-terminal-ghostty
case "$target" in *windows*) extension=.exe ;; *) extension= ;; esac
binary=target/$target/release/$name$extension
[ -f "$binary" ] || { echo "release binary is missing: $binary" >&2; exit 1; }
mkdir -p "$out"
[ ! -L "$out" ] || { echo 'stage output must not be a symbolic link' >&2; exit 2; }
next=$out/.$name.next.$$
trap 'rm -f "$next" "$out/.sidecar.json.next.$$"' EXIT HUP INT TERM
cp "$binary" "$next"
chmod +x "$next"
if [ -e "$out/$name$extension" ]; then
  cmp -s "$next" "$out/$name$extension" || { echo 'staged binary conflicts with current build' >&2; exit 1; }
  rm -f "$next"
else
  mv "$next" "$out/$name$extension"
fi
manifest=$out/.sidecar.json.next.$$
sed "s#\"process\": \"dist/$name\"#\"process\": \"dist/$name$extension\"#" sidecar.json > "$manifest"
if [ -e "$out/sidecar.json" ]; then
  cmp -s "$manifest" "$out/sidecar.json" || { echo 'staged manifest conflicts with source' >&2; exit 1; }
  rm -f "$manifest"
else
  mv "$manifest" "$out/sidecar.json"
fi
echo "SIDECAR_STAGED target=$target output=$out/$name$extension"
