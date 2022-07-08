#!/bin/sh
set -e

if ! command -v unzip >/dev/null; then
	echo "Error: unzip is required to install 'flopha'" 1>&2
	exit 1
fi

case $(uname -sm) in
"Darwin x86_64") target="x86_64-apple-darwin" ;;
"Darwin arm64") target="aarch64-apple-darwin" ;;
*) target="x86_64-unknown-linux-gnu" ;;
esac

if [ $# -eq 0 ]; then
	flopha_uri="https://github.com/sjquant/flopha/releases/latest/download/flopha-${target}.tar.gz"
else
	flopha_uri="https://github.com/sjquant/flopha/releases/download/${1}/flopha-${target}.tar.gz"
fi

flopha_install="${FLOPHA_INSTALL:-$HOME/.flopha}"
bin_dir="$flopha_install/bin"
exe="$bin_dir/flopha"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
fi

curl --fail --location --progress-bar --output "$exe.tar.gz" "$flopha_uri"
tar -xvzf "$exe.tar.gz" -C "$bin_dir"
chmod +x "$exe"
rm "$exe.tar.gz"

echo "Flopha was installed successfully to $exe"
if command -v flopha >/dev/null; then
	echo "Run 'flopha --help' to get started"
else
	case $SHELL in
	/bin/zsh) shell_profile=".zshrc" ;;
	*) shell_profile=".bashrc" ;;
	esac
	echo "Manually add the directory to your \$HOME/$shell_profile (or similar)"
	echo "  export FLOPHA_INSTALL=\"$flopha_install\""
	echo "  export PATH=\"\$FLOPHA_INSTALL/bin:\$PATH\""
	echo "Run '$exe --help' to get started"
fi