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

if [ -n "${FLOPHA_BINARY:-}" ]; then
  mkdir -p "$BIN_DIR"
  cp "$FLOPHA_BINARY" "$BIN_DIR/flopha"
  chmod +x "$BIN_DIR/flopha"
  echo "$BIN_DIR" >> "$GITHUB_PATH"

  export PATH="$BIN_DIR:$PATH"
  INSTALLED="$(flopha --version)"
  echo "Installed $INSTALLED from $FLOPHA_BINARY"
  exit 0
fi

# Skip download if flopha is already on PATH (e.g. built from source in CI)
if command -v flopha >/dev/null 2>&1; then
  FOUND="$(command -v flopha)"
  VERSION_STR="$(flopha --version)"
  echo "$VERSION_STR already on PATH at $FOUND, skipping download"
  mkdir -p "$BIN_DIR"
  # Symlink into BIN_DIR so run.sh's PATH prepend always finds the binary
  # regardless of where it was originally installed.
  if [ "$FOUND" != "$BIN_DIR/flopha" ]; then
    ln -sf "$FOUND" "$BIN_DIR/flopha"
  fi
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
TMP=$(mktemp)
if ! curl -fsSL "$URL" -o "$TMP"; then
  rm -f "$TMP"
  echo "::error::Failed to download flopha from $URL"
  exit 1
fi
tar -xz -C "$BIN_DIR" < "$TMP"
rm -f "$TMP"
chmod +x "$BIN_DIR/flopha"
echo "$BIN_DIR" >> "$GITHUB_PATH"

export PATH="$BIN_DIR:$PATH"
INSTALLED="$(flopha --version)"
echo "Installed $INSTALLED"
