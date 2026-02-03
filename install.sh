#!/usr/bin/env bash
# Wrapped in main() to prevent curl|bash race condition
set -euo pipefail

main() {
  REPO="rawwerks/dirpack"
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"

  fail() {
    echo "install.sh: $*" >&2
    exit 1
  }

  require_cmd() {
    command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
  }

  require_cmd curl
  require_cmd uname

  tag="$(
    curl -fsSL "$API_URL" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
  )"

  if [ -z "$tag" ]; then
    fail "could not determine latest release tag"
  fi

  os="$(uname -s)"
  case "$os" in
    Linux) os="linux" ;;
    Darwin) os="macos" ;;
    *) fail "unsupported OS: $os" ;;
  esac

  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) fail "unsupported architecture: $arch" ;;
  esac

  asset="dirpack-${os}-${arch}"
  url="https://github.com/${REPO}/releases/download/${tag}/${asset}"

  tmpfile="$(mktemp)"
  cleanup() {
    rm -f "$tmpfile"
  }
  trap cleanup EXIT

  echo "Downloading ${asset} from ${url}"
  curl -fL --progress-bar "$url" -o "$tmpfile" || fail "download failed"

  chmod +x "$tmpfile"

  install_dir="${HOME}/.local/bin"
  use_sudo=false

  if [ ! -d "$install_dir" ]; then
    mkdir -p "$install_dir" 2>/dev/null || true
  fi

  if [ ! -w "$install_dir" ]; then
    install_dir="/usr/local/bin"
    use_sudo=true
  fi

  dest="${install_dir}/dirpack"

  if [ "$use_sudo" = true ]; then
    require_cmd sudo
    sudo mv "$tmpfile" "$dest"
  else
    mv "$tmpfile" "$dest"
  fi

  version="$("$dest" --version 2>/dev/null || true)"
  if [ -z "$version" ]; then
    fail "dirpack did not run successfully after install"
  fi

  echo "Installed ${version} to ${dest}"

  if ! command -v dirpack >/dev/null 2>&1; then
    echo "Note: ${install_dir} is not on your PATH."
    echo "Add this to your shell profile:"
    echo "  export PATH=\"${install_dir}:\$PATH\""
  fi
}

# Call main only after entire script is downloaded
main "$@"
