#!/bin/sh
set -eu

[ "$#" -eq 2 ] && [ -n "$1" ] && [ -n "$2" ] || { echo 'usage: prepare-ghostty-sdk.sh <target> <build-root>' >&2; exit 2; }
target=$1
build_root=$2
case "$build_root" in /*|*..*) echo 'build root must be a safe repository-relative path' >&2; exit 2 ;; esac
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
build_root=$root/$build_root
receipt=$build_root/receipts/$target.json

if [ -f "$receipt" ]; then
  soksak-validate build-receipt "$receipt" --dependencies "$root/build-dependencies.json" --output-root "$build_root"
  echo "GHOSTTY_SDK_REUSED target=$target"
  exit 0
fi
if [ -e "$build_root/targets/$target" ]; then
  echo "unverified Ghostty SDK output already exists for $target" >&2
  exit 79
fi

mkdir -p "$build_root/sources" "$build_root/.transactions"
transaction=$build_root/.transactions/prepare.$target.$$
source_next=$build_root/sources/.next.$$
stage=$build_root/builds/$target
cleanup() {
  for candidate in "$transaction" "$source_next"; do
    case "$candidate" in "$build_root"/.transactions/*|"$build_root"/sources/.next.*) rm -rf -- "$candidate" ;; esac
  done
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$transaction/targets/$target/lib"

resolution=$transaction/resolution.json
soksak-validate build-dependencies "$root/build-dependencies.json" --dependency ghostty-vt-sdk --target "$target" > "$resolution"
repository=$(node -e 'const v=require(process.argv[1]);process.stdout.write(v.repository)' "$resolution")
commit=$(node -e 'const v=require(process.argv[1]);process.stdout.write(v.commit)' "$resolution")
zig_version=$(node -e 'const v=require(process.argv[1]);process.stdout.write(v.tools.zig)' "$resolution")
zig_bin=$("$root/scripts/resolve-declared-zig.sh" "$target" "$zig_version")
source=$build_root/sources/$commit

if [ -e "$source" ]; then
  [ -d "$source/.git" ] && [ "$(git -C "$source" remote get-url origin)" = "$repository" ] && \
    [ "$(git -C "$source" rev-parse HEAD)" = "$commit" ] && [ -z "$(git -C "$source" status --porcelain)" ] || {
      echo "cached Ghostty source differs from build-dependencies.json" >&2
      exit 79
    }
else
  git init -q "$source_next"
  git -C "$source_next" remote add origin "$repository"
  git -C "$source_next" fetch -q --depth 1 origin "$commit"
  git -C "$source_next" -c advice.detachedHead=false checkout -q FETCH_HEAD
  [ "$(git -C "$source_next" rev-parse HEAD)" = "$commit" ] && [ -z "$(git -C "$source_next" status --porcelain)" ] || {
    echo "Ghostty source checkout did not materialize the declared commit" >&2
    exit 79
  }
  mv "$source_next" "$source"
fi

if [ -e "$stage" ]; then
  [ -d "$stage" ] && [ -f "$stage/.soksak-build-resolution.json" ] && \
    cmp -s "$resolution" "$stage/.soksak-build-resolution.json" || {
      echo "Ghostty build cache differs from the declared inputs" >&2
      exit 79
    }
else
  mkdir -p "$stage"
  git -C "$source" archive --format=tar "$commit" | tar -xf - -C "$stage"
  cp "$resolution" "$stage/.soksak-build-resolution.json"
fi
(cd "$stage" && "$zig_bin" build -Demit-lib-vt=true -Doptimize=ReleaseFast -Dcpu=baseline)
case "$target" in *windows*) archive=ghostty-vt-static.lib ;; *) archive=libghostty-vt.a ;; esac
[ -f "$stage/zig-out/lib/$archive" ] || { echo "Ghostty SDK archive is missing: $archive" >&2; exit 79; }
cp "$stage/zig-out/lib/$archive" "$transaction/targets/$target/lib/$archive"
printf '%s\n' "$commit" > "$transaction/targets/$target/source-commit.txt"
printf '%s\n' "$zig_version" > "$transaction/targets/$target/zig-version.txt"
if find "$transaction/targets/$target" -type l -print -quit | grep -q .; then
  echo "Ghostty SDK outputs contain a symbolic link" >&2
  exit 79
fi
mkdir -p "$transaction/receipts"
soksak-validate build-receipt-create "$root/build-dependencies.json" --dependency ghostty-vt-sdk \
  --target "$target" --output-root "$transaction" --out "$transaction/receipts/$target.json"
soksak-validate build-receipt "$transaction/receipts/$target.json" \
  --dependencies "$root/build-dependencies.json" --output-root "$transaction"

mkdir -p "$build_root/targets" "$build_root/receipts"
[ ! -e "$build_root/targets/$target" ] && [ ! -e "$receipt" ] || { echo "Ghostty SDK output appeared concurrently" >&2; exit 79; }
mv "$transaction/targets/$target" "$build_root/targets/$target"
mv "$transaction/receipts/$target.json" "$receipt"
soksak-validate build-receipt "$receipt" --dependencies "$root/build-dependencies.json" --output-root "$build_root"
echo "GHOSTTY_SDK_READY target=$target"
