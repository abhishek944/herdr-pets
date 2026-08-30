#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

npm ci
npm run tauri build -- --no-bundle

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) package_dir="$ROOT/bin/macos-arm64" ;;
  Darwin:x86_64) package_dir="$ROOT/bin/macos-x64" ;;
  *) package_dir="" ;;
esac

if [ -n "$package_dir" ]; then
  mkdir -p "$package_dir"
  install -m 755 "$ROOT/src-tauri/target/release/herdr-pets" "$package_dir/herdr-pets"
  echo "packaged: $package_dir/herdr-pets"
fi

echo "built: $ROOT/src-tauri/target/release/herdr-pets"
