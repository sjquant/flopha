#!/usr/bin/env bash
set -euo pipefail

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS/$ARCH" in
  Linux/x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
  Darwin/x86_64) TARGET="x86_64-apple-darwin" ;;
  Darwin/arm64)  TARGET="aarch64-apple-darwin" ;;
  *)
    echo "::error::flopha: unsupported platform $OS/$ARCH"
    echo "::error::Supported: Linux/x86_64, Darwin/x86_64, Darwin/arm64"
    echo "::error::Windows runners are not yet supported."
    exit 1
    ;;
esac

BIN_DIR="${HOME}/.flopha/bin"

# Skip download if flopha is already on PATH (e.g. built from source in CI)
if command -v flopha >/dev/null 2>&1; then
  echo "flopha $(flopha --version) already on PATH at $(command -v flopha), skipping download"
  echo "$BIN_DIR" >> "$GITHUB_PATH"
  exit 0
fi

VERSION="${FLOPHA_VERSION:-latest}"
if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/sjquant/flopha/releases/latest/download/flopha-${TARGET}.tar.gz"
else
  URL="https://github.com/sjquant/flopha/releases/download/${VERSION}/flopha-${TARGET}.tar.gz"
fi

mkdir -p "$BIN_DIR"

echo "Installing flopha ${VERSION} (${TARGET})..."
curl -sfL "$URL" | tar -xz -C "$BIN_DIR"
chmod +x "$BIN_DIR/flopha"
echo "$BIN_DIR" >> "$GITHUB_PATH"

export PATH="$BIN_DIR:$PATH"
echo "Installed $(flopha --version)"
