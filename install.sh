#!/usr/bin/env bash

set -euo pipefail

BINARY_NAME="dsync"
DEFAULT_REPO="feliperbroering/dsync"
DEFAULT_INSTALL_DIR="${HOME}/.local/bin"

REPO="${DSYNC_REPO:-$DEFAULT_REPO}"
INSTALL_DIR="${DSYNC_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
VERSION="${DSYNC_VERSION:-latest}"
FORCE_CARGO="${DSYNC_FORCE_CARGO:-0}"

usage() {
  cat <<'EOF'
Install dsync from GitHub releases or, if no matching release asset exists yet, from source via Cargo.

Usage:
  install.sh [--version <tag>] [--install-dir <dir>] [--repo <owner/name>] [--force-cargo]

Options:
  --version <tag>      Install a specific release tag instead of the latest release.
  --install-dir <dir>  Directory where the dsync binary will be installed.
  --repo <owner/name>  GitHub repository to install from.
  --force-cargo        Skip release download and install via Cargo directly.
  -h, --help           Show this help message.

Environment overrides:
  DSYNC_VERSION
  DSYNC_INSTALL_DIR
  DSYNC_REPO
  DSYNC_FORCE_CARGO
EOF
}

info() {
  printf '==> %s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

has_command() {
  command -v "$1" >/dev/null 2>&1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version)
        [[ $# -ge 2 ]] || die "--version requires a value"
        VERSION="$2"
        shift 2
        ;;
      --install-dir)
        [[ $# -ge 2 ]] || die "--install-dir requires a value"
        INSTALL_DIR="$2"
        shift 2
        ;;
      --repo)
        [[ $# -ge 2 ]] || die "--repo requires a value"
        REPO="$2"
        shift 2
        ;;
      --force-cargo)
        FORCE_CARGO="1"
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        die "Unknown argument: $1"
        ;;
    esac
  done
}

detect_target() {
  local os
  local arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin) os="apple-darwin" ;;
    Linux) os="unknown-linux-gnu" ;;
    *)
      return 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)
      return 1
      ;;
  esac

  printf '%s-%s\n' "$arch" "$os"
}

release_url() {
  local asset_name="$1"

  if [[ "$VERSION" == "latest" ]]; then
    printf 'https://github.com/%s/releases/latest/download/%s\n' "$REPO" "$asset_name"
  else
    printf 'https://github.com/%s/releases/download/%s/%s\n' "$REPO" "$VERSION" "$asset_name"
  fi
}

install_from_release() {
  local target
  local asset_name
  local url
  local tmpdir
  local extracted_binary

  if ! has_command curl; then
    warn "curl is not installed, skipping release download"
    return 1
  fi

  if ! has_command tar; then
    warn "tar is not installed, skipping release download"
    return 1
  fi

  if ! target="$(detect_target)"; then
    warn "no prebuilt release target is configured for this platform, falling back to Cargo"
    return 1
  fi

  asset_name="${BINARY_NAME}-${target}.tar.gz"
  url="$(release_url "$asset_name")"
  tmpdir="$(mktemp -d)"

  info "Trying release asset ${asset_name}"
  if ! curl -fsSL "$url" -o "${tmpdir}/${asset_name}"; then
    warn "release asset not available at ${url}"
    rm -rf "$tmpdir"
    return 1
  fi

  if ! tar -xzf "${tmpdir}/${asset_name}" -C "$tmpdir"; then
    warn "failed to extract ${asset_name}"
    rm -rf "$tmpdir"
    return 1
  fi

  extracted_binary="$(find "$tmpdir" -type f -name "$BINARY_NAME" | head -n 1 || true)"
  if [[ -z "$extracted_binary" ]]; then
    warn "release archive did not contain ${BINARY_NAME}"
    rm -rf "$tmpdir"
    return 1
  fi

  mkdir -p "$INSTALL_DIR"
  install -m 0755 "$extracted_binary" "${INSTALL_DIR}/${BINARY_NAME}"
  rm -rf "$tmpdir"
  info "Installed ${BINARY_NAME} from release to ${INSTALL_DIR}/${BINARY_NAME}"
}

print_path_hint() {
  if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    printf '\nAdd %s to your PATH if it is not there already:\n' "$INSTALL_DIR"
    printf "  export PATH=\"%s:\$PATH\"\n" "$INSTALL_DIR"
  fi
}

local_checkout_dir() {
  local source_file
  local repo_dir

  source_file="${BASH_SOURCE[0]:-}"
  [[ -n "$source_file" ]] || return 1
  [[ "$VERSION" == "latest" ]] || return 1
  [[ "$REPO" == "$DEFAULT_REPO" ]] || return 1

  repo_dir="$(cd "$(dirname "$source_file")" && pwd)"
  [[ -f "${repo_dir}/Cargo.toml" ]] || return 1

  printf '%s\n' "$repo_dir"
}

install_from_source() {
  local build_dir
  local repo_dir
  local clone_dir
  local install_source

  has_command cargo || die "cargo is required when no matching release asset is available"

  build_dir="$(mktemp -d)"

  if repo_dir="$(local_checkout_dir)"; then
    info "Installing ${BINARY_NAME} from the local checkout"
    install_source="$repo_dir"
  else
    has_command git || die "git is required to install from source"

    clone_dir="$(mktemp -d)"
    install_source="${clone_dir}/repo"
    info "Cloning ${REPO} to build ${BINARY_NAME} from source"

    if [[ "$VERSION" == "latest" ]]; then
      git clone --depth 1 "https://github.com/${REPO}.git" "$install_source"
    else
      git clone --depth 1 --branch "$VERSION" "https://github.com/${REPO}.git" "$install_source"
    fi
  fi

  info "Installing ${BINARY_NAME} via Cargo"
  cargo install --locked --path "$install_source" --root "$build_dir" --bin "$BINARY_NAME"

  mkdir -p "$INSTALL_DIR"
  install -m 0755 "${build_dir}/bin/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

  rm -rf "$build_dir"
  if [[ -n "${clone_dir:-}" ]]; then
    rm -rf "$clone_dir"
  fi

  info "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
}

main() {
  parse_args "$@"

  if [[ "$FORCE_CARGO" == "1" ]]; then
    install_from_source
  elif ! install_from_release; then
    install_from_source
  fi

  print_path_hint
  printf "\nRun \`%s --help\` to get started.\n" "$BINARY_NAME"
}

main "$@"
