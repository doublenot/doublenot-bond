#!/usr/bin/env bash
set -euo pipefail

repo="${BOND_REPO:-doublenot/doublenot-bond}"
install_dir="${BOND_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux) os_part="unknown-linux-gnu" ;;
  Darwin) os_part="apple-darwin" ;;
  *)
    echo "unsupported operating system: $os" >&2
    exit 1
    ;;
esac

case "$arch" in
  x86_64|amd64) arch_part="x86_64" ;;
  arm64|aarch64) arch_part="aarch64" ;;
  *)
    echo "unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

asset="doublenot-bond-${arch_part}-${os_part}.tar.gz"
url="https://github.com/${repo}/releases/latest/download/${asset}"
checksums_url="https://github.com/${repo}/releases/latest/download/doublenot-bond-checksums.txt"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

mkdir -p "$install_dir"
curl -fsSL "$url" -o "$tmp_dir/$asset"
curl -fsSL "$checksums_url" -o "$tmp_dir/doublenot-bond-checksums.txt"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmp_dir" && grep " ${asset}$" doublenot-bond-checksums.txt | sha256sum -c -)
elif command -v shasum >/dev/null 2>&1; then
  expected="$(grep " ${asset}$" "$tmp_dir/doublenot-bond-checksums.txt" | awk '{print $1}')"
  actual="$(shasum -a 256 "$tmp_dir/$asset" | awk '{print $1}')"
  [[ "$expected" == "$actual" ]]
else
  echo "warning: no sha256 verifier found; skipping checksum verification" >&2
fi

tar -xzf "$tmp_dir/$asset" -C "$tmp_dir"
find "$tmp_dir" -type f -name doublenot-bond -exec cp {} "$install_dir/doublenot-bond" \;
chmod +x "$install_dir/doublenot-bond"

echo "installed doublenot-bond to $install_dir/doublenot-bond"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) echo "add $install_dir to PATH to run doublenot-bond directly" ;;
esac
