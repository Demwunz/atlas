#!/usr/bin/env bash
# Atlas installer — download the latest release binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/demwunz/atlas/main/install.sh | bash
#
# Environment variables:
#   ATLAS_VERSION   — specific version to install (default: latest)
#   ATLAS_INSTALL   — installation directory (default: ~/.local/bin)

set -euo pipefail

REPO="demwunz/atlas"
INSTALL_DIR="${ATLAS_INSTALL:-$HOME/.local/bin}"

detect_platform() {
    local os arch target

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        *)
            echo "Error: unsupported OS: $os" >&2
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64 | amd64) arch="x86_64" ;;
        aarch64 | arm64) arch="aarch64" ;;
        *)
            echo "Error: unsupported architecture: $arch" >&2
            exit 1
            ;;
    esac

    target="${arch}-${os}"
    echo "$target"
}

get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    echo "$version"
}

main() {
    local target version archive_name url tmp_dir

    target="$(detect_platform)"
    version="${ATLAS_VERSION:-$(get_latest_version)}"

    if [ -z "$version" ]; then
        echo "Error: could not determine latest version." >&2
        echo "Set ATLAS_VERSION explicitly, e.g.: ATLAS_VERSION=v0.1.0 $0" >&2
        exit 1
    fi

    echo "Installing atlas ${version} for ${target}..."

    archive_name="atlas-${version}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    echo "Downloading ${url}..."
    curl -fsSL "$url" -o "${tmp_dir}/${archive_name}"

    echo "Extracting..."
    tar -xzf "${tmp_dir}/${archive_name}" -C "$tmp_dir"

    mkdir -p "$INSTALL_DIR"
    mv "${tmp_dir}/atlas" "${INSTALL_DIR}/atlas"
    chmod +x "${INSTALL_DIR}/atlas"

    echo ""
    echo "atlas installed to ${INSTALL_DIR}/atlas"

    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo ""
        echo "Add to your PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi

    echo ""
    "${INSTALL_DIR}/atlas" --version || true
    echo "Done."
}

main "$@"
