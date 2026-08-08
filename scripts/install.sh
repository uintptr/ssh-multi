#!/bin/bash
set -euo pipefail

REPO="uintptr/ssh-multi"
INSTALL_DIR="${HOME}/.local/bin"
TMP_DIR=""

cleanup() {
    if [ -n "${TMP_DIR}" ] && [ -d "${TMP_DIR}" ]; then
        rm -rf "${TMP_DIR}"
    fi
}
trap cleanup EXIT INT TERM

# Detect the release asset name for this platform.
# Releases ship x86_64 and arm64 musl builds for Linux, arm64 for macOS.
detect_asset() {
    local os

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        *)
            echo "Error: Unsupported operating system: $(uname -s)" >&2
            exit 1
            ;;
    esac

    if [ "${os}" = "darwin" ]; then
        case "$(uname -m)" in
            arm64|aarch64) ;;
            *)
                echo "Error: Unsupported architecture for macOS: $(uname -m)" >&2
                exit 1
                ;;
        esac

        echo "ssh-multi-macos"
        return
    fi

    case "$(uname -m)" in
        x86_64|amd64)  echo "ssh-multi-linux" ;;
        arm64|aarch64) echo "ssh-multi-linux-arm64" ;;
        *)
            echo "Error: Unsupported architecture for Linux: $(uname -m)" >&2
            exit 1
            ;;
    esac
}

# Get latest release tag from GitHub
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

main() {
    local asset version ssh_multi_url

    echo "Installing ssh-multi..."

    asset=$(detect_asset)
    echo "Detected asset: ${asset}"

    version=$(get_latest_version)
    if [ -z "$version" ]; then
        echo "Error: Could not determine latest version" >&2
        exit 1
    fi
    echo "Latest version: ${version}"

    ssh_multi_url="https://github.com/${REPO}/releases/download/${version}/${asset}"

    # Create install directory if it doesn't exist
    mkdir -p "${INSTALL_DIR}"

    # Download binary
    TMP_DIR=$(mktemp -d)

    echo "Downloading ssh-multi from ${ssh_multi_url}..."
    if ! curl -fsSL -o "${TMP_DIR}/ssh-multi" "${ssh_multi_url}"; then
        echo "Error: Failed to download ssh-multi" >&2
        exit 1
    fi

    # Install binary
    chmod +x "${TMP_DIR}/ssh-multi"
    mv "${TMP_DIR}/ssh-multi" "${INSTALL_DIR}/ssh-multi"

    echo "Successfully installed ssh-multi to ${INSTALL_DIR}"

    # Check if install dir is in PATH
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        echo ""
        echo "Note: ${INSTALL_DIR} is not in your PATH."
        echo "Add it by running:"
        echo "  echo 'export PATH=\"\${HOME}/.local/bin:\${PATH}\"' >> ~/.bashrc"
        echo "  source ~/.bashrc"
    fi
}

main
