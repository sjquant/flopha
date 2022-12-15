#!/bin/bash
set -e -x

TARGET="x86_64-unknown-linux-musl"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift
      shift
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Build
rustup target add ${TARGET}
cargo build --verbose --release --target ${TARGET}

# Package
STAGING="dist/flopha-${TARGET}"
mkdir -p "$STAGING"
cp {README.md,LICENSE} "$STAGING"
cp "target/${TARGET}/release/flopha" "$STAGING/"
tar -C "$STAGING" -cvzf "$STAGING.tar.gz" .