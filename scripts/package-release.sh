#!/usr/bin/env bash

set -euo pipefail

BINARY_NAME="dsync"
TARGET="${1:?usage: package-release.sh <target> [output-dir]}"
OUTPUT_DIR="${2:-dist}"
BINARY_PATH="target/${TARGET}/release/${BINARY_NAME}"
PACKAGE_ROOT=""

checksum_file() {
  local file_path="$1"
  local output_path="$2"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file_path" > "$output_path"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file_path" > "$output_path"
  else
    printf 'warning: no SHA-256 checksum tool found, skipping checksum generation\n' >&2
  fi
}

main() {
  local archive_name
  local package_dir
  local archive_path
  local checksum_path

  archive_name="${BINARY_NAME}-${TARGET}.tar.gz"
  PACKAGE_ROOT="$(mktemp -d)"
  package_dir="${PACKAGE_ROOT}/${BINARY_NAME}-${TARGET}"
  archive_path="${OUTPUT_DIR}/${archive_name}"
  checksum_path="${archive_path}.sha256"

  trap 'rm -rf "${PACKAGE_ROOT}"' EXIT

  mkdir -p "$OUTPUT_DIR" "$package_dir"

  cargo build --locked --release --target "$TARGET"

  [[ -f "$BINARY_PATH" ]] || {
    printf 'error: binary not found at %s\n' "$BINARY_PATH" >&2
    exit 1
  }

  cp "$BINARY_PATH" "${package_dir}/${BINARY_NAME}"
  cp LICENSE README.md "$package_dir/"

  tar -C "$PACKAGE_ROOT" -czf "$archive_path" "$(basename "$package_dir")"
  checksum_file "$archive_path" "$checksum_path"

  printf 'Created %s\n' "$archive_path"
}

main
