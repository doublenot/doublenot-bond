#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_toml="$repo_root/Cargo.toml"

usage() {
  cat <<'EOF'
Usage: ./scripts/release-prep.sh <version>

Examples:
  ./scripts/release-prep.sh 0.1.1
  ./scripts/release-prep.sh v0.1.1

This updates the package version in Cargo.toml and prints the next git steps.
EOF
}

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 1
fi

requested_version="$1"
version="${requested_version#v}"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "error: version must look like semver, for example 0.1.1 or v0.1.1" >&2
  exit 1
fi

current_version="$(awk '
  /^\[package\]$/ { in_package=1; next }
  /^\[/ && $0 != "[package]" { in_package=0 }
  in_package && /^version = "/ {
    gsub(/^version = "/, "")
    gsub(/"$/, "")
    print
    exit
  }
' "$cargo_toml")"

if [[ -z "$current_version" ]]; then
  echo "error: failed to locate [package].version in Cargo.toml" >&2
  exit 1
fi

if [[ "$current_version" == "$version" ]]; then
  echo "Cargo.toml is already at version $version"
  echo "Next steps:"
  echo "  git tag v$version"
  echo "  git push origin v$version"
  exit 0
fi

tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT

awk -v version="$version" '
  BEGIN { in_package=0; updated=0 }
  /^\[package\]$/ { in_package=1; print; next }
  /^\[/ && $0 != "[package]" { in_package=0 }
  in_package && /^version = "/ && !updated {
    print "version = \"" version "\""
    updated=1
    next
  }
  { print }
  END {
    if (!updated) {
      exit 1
    }
  }
' "$cargo_toml" > "$tmp_file"

mv "$tmp_file" "$cargo_toml"
trap - EXIT

echo "Updated Cargo.toml version: $current_version -> $version"
echo "Next steps:"
echo "  cargo test"
echo "  git add Cargo.toml Cargo.lock"
echo "  git commit -m \"Release v$version\""
echo "  git tag v$version"
echo "  git push origin main --follow-tags"
