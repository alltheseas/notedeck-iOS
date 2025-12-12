#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "${SCRIPT_DIR}/../../.." && pwd)"

# Set iOS deployment target for C dependencies
export IPHONEOS_DEPLOYMENT_TARGET=14.0

pushd "${PROJECT_ROOT}" >/dev/null

echo "Building iOS simulator static library (release)..."
cargo build -p notedeck_ios --target aarch64-apple-ios-sim --release

echo "Building iOS device static library (release)..."
cargo build -p notedeck_ios --target aarch64-apple-ios --release

SIM_LIB="target/aarch64-apple-ios-sim/release/libnotedeck_ios.a"
IOS_LIB="target/aarch64-apple-ios/release/libnotedeck_ios.a"

for lib in "$SIM_LIB" "$IOS_LIB"; do
  if [ ! -f "$lib" ]; then
    echo "Missing build artifact: $lib" >&2
    exit 1
  fi
done

echo "Generating Swift package with swift-bridge..."
swift-bridge-cli create-package \
  --bridges-dir ./crates/notedeck_ios/generated \
  --out-dir ./crates/notedeck_ios/NotedeckMobile \
  --simulator "$SIM_LIB" \
  --ios "$IOS_LIB" \
  --name NotedeckMobile

popd >/dev/null

echo "Release build complete! Swift package created at crates/notedeck_ios/NotedeckMobile"
