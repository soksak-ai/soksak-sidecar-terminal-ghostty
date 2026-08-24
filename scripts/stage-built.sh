#!/bin/sh
set -eu

[ "$#" -eq 2 ] && [ -n "$1" ] && [ -n "$2" ] || { echo 'usage: stage-built.sh <out> <target>' >&2; exit 2; }
out=$1
target=$2
case "$out" in /*|*..*) echo 'stage output must be repository-relative' >&2; exit 2 ;; esac
name=soksak-sidecar-terminal-ghostty
case "$target" in *windows*) extension=.exe ;; *) extension= ;; esac
binary=target/$target/release/$name$extension
[ -f "$binary" ] || { echo "release binary is missing: $binary" >&2; exit 1; }
mkdir -p "$out"
next=$out/.$name.next.$$
cp "$binary" "$next"
chmod +x "$next"
mv "$next" "$out/$name$extension"
sed "s#\"process\": \"dist/$name\"#\"process\": \"dist/$name$extension\"#" sidecar.json > "$out/sidecar.json"
echo "SIDECAR_STAGED target=$target output=$out/$name$extension"
