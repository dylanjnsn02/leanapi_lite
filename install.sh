#!/bin/sh
# Installer for leanapi_lite.
#
#   curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/dylanjnsn02/leanapi_lite/main/install.sh | sh
#
# Downloads the right prebuilt binary from GitHub Releases for your OS/arch,
# installs it as `leanapi_lite` into $LEANAPI_LITE_INSTALL_DIR (default
# ~/.local/bin). Pin a specific release with LEANAPI_LITE_VERSION=v0.1.0.

set -eu

REPO="dylanjnsn02/leanapi_lite"
BIN_NAME="leanapi_lite"
INSTALL_DIR="${LEANAPI_LITE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${LEANAPI_LITE_VERSION:-latest}"

err() {
    echo "error: $1" >&2
    exit 1
}

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "'$1' is required but not found"
    fi
}

need_cmd curl
need_cmd uname
need_cmd mktemp

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Darwin) os_name="darwin" ;;
    Linux) os_name="linux" ;;
    *) err "unsupported OS: $os (leanapi_lite ships prebuilt binaries for macOS and Linux only -- Windows users should grab the .exe from the Releases page)" ;;
esac

case "$arch" in
    x86_64 | amd64) arch_name="amd64" ;;
    arm64 | aarch64) arch_name="arm64" ;;
    *) err "unsupported architecture: $arch" ;;
esac

asset="leanapi-lite-${os_name}-${arch_name}"

if [ "$VERSION" = "latest" ]; then
    url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
    url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

echo "Downloading ${asset} (${VERSION})..."
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
tmp_file="${tmp_dir}/${BIN_NAME}"

if ! curl --proto '=https' --tlsv1.2 -sSf -L "$url" -o "$tmp_file"; then
    err "download failed -- is ${url} reachable? (check LEANAPI_LITE_VERSION if you set one)"
fi

mkdir -p "$INSTALL_DIR"
chmod 755 "$tmp_file"
mv "$tmp_file" "${INSTALL_DIR}/${BIN_NAME}"

echo "Installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}"

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo ""
        echo "NOTE: ${INSTALL_DIR} is not on your PATH. Add this to your shell profile:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

echo ""
echo "Run '${BIN_NAME}' to get started."
