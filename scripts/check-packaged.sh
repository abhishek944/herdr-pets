#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ARM="$ROOT/bin/macos-arm64/herdr-pets"
X64="$ROOT/bin/macos-x64/herdr-pets"

[ -x "$ARM" ] && [ -x "$X64" ]
file "$ARM" | grep -q 'Mach-O 64-bit executable arm64'
file "$X64" | grep -q 'Mach-O 64-bit executable x86_64'

case "$(uname -m)" in
  arm64) cmp "$ROOT/src-tauri/target/release/herdr-pets" "$ARM" ;;
  x86_64) cmp "$ROOT/src-tauri/target/release/herdr-pets" "$X64" ;;
esac

if [ -x "$ROOT/src-tauri/target/x86_64-apple-darwin/release/herdr-pets" ]; then
  cmp "$ROOT/src-tauri/target/x86_64-apple-darwin/release/herdr-pets" "$X64"
fi
if [ -x "$ROOT/src-tauri/target/aarch64-apple-darwin/release/herdr-pets" ]; then
  cmp "$ROOT/src-tauri/target/aarch64-apple-darwin/release/herdr-pets" "$ARM"
fi

echo "packaged macOS binaries: pass"
