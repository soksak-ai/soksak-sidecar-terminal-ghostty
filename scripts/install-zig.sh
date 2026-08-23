#!/bin/sh
set -eu

version=0.16.0
root=${RUNNER_TEMP:?RUNNER_TEMP is required}/zig-$version
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) archive=zig-aarch64-macos-$version.tar.xz; sha=b23d70deaa879b5c2d486ed3316f7eaa53e84acf6fc9cc747de152450d401489 ;;
  Darwin-x86_64) archive=zig-x86_64-macos-$version.tar.xz; sha=0387557ed1877bc6a2e1802c8391953baddba76081876301c522f52977b52ba7 ;;
  Linux-aarch64) archive=zig-aarch64-linux-$version.tar.xz; sha=ea4b09bfb22ec6f6c6ceac57ab63efb6b46e17ab08d21f69f3a48b38e1534f17 ;;
  Linux-x86_64) archive=zig-x86_64-linux-$version.tar.xz; sha=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00 ;;
  MINGW*-x86_64|MSYS*-x86_64) archive=zig-x86_64-windows-$version.zip; sha=68659eb5f1e4eb1437a722f1dd889c5a322c9954607f5edcf337bc3684a75a7e ;;
  *) echo "unsupported Zig host: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

download=$RUNNER_TEMP/$archive
curl -fL --retry 3 --retry-delay 3 -o "$download" "https://ziglang.org/download/$version/$archive"
if command -v sha256sum >/dev/null 2>&1; then
  printf '%s  %s\n' "$sha" "$download" | sha256sum -c -
else
  test "$(shasum -a 256 "$download" | awk '{print $1}')" = "$sha"
fi
mkdir -p "$root"
case "$archive" in
  *.zip) unzip -q "$download" -d "$root" ;;
  *) tar -xJf "$download" -C "$root" ;;
esac
zig=$(find "$root" -type f \( -name zig -o -name zig.exe \) -print -quit)
test -n "$zig"
directory=$(dirname "$zig")
printf '%s\n' "$directory" >> "${GITHUB_PATH:?GITHUB_PATH is required}"
"$zig" version | grep -Fx "$version"
