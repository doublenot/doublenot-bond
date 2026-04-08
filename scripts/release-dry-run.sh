#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="${1:-$repo_root/target/release-dry-run}"
target="${BOND_RELEASE_TARGET:-x86_64-unknown-linux-gnu}"
version="${BOND_RELEASE_VERSION:-dry-run-$(git -C "$repo_root" rev-parse --short HEAD)}"
artifact_root="doublenot-bond-${target}"
source_archive="doublenot-bond-${version}.tar.gz"

rm -rf "$dist_dir"
mkdir -p "$dist_dir"

pushd "$repo_root" >/dev/null

cargo build --release --locked --target "$target"
cargo publish --dry-run --locked --allow-dirty

artifact_dir="$dist_dir/$artifact_root"
mkdir -p "$artifact_dir"
cp "target/$target/release/doublenot-bond" "$artifact_dir/"
cp README.md "$artifact_dir/"
cp install.sh "$artifact_dir/"

tar -czf "$dist_dir/${artifact_root}.tar.gz" -C "$dist_dir" "$artifact_root"
git archive --format=tar.gz --prefix="doublenot-bond-${version}/" -o "$dist_dir/$source_archive" HEAD

find "$dist_dir" -maxdepth 1 -type f \( -name '*.tar.gz' -o -name '*.zip' \) -print0 \
  | sort -z \
  | xargs -0 sha256sum \
  | sed "s#${dist_dir}/##" > "$dist_dir/doublenot-bond-checksums.txt"

popd >/dev/null

echo "release dry-run artifacts written to $dist_dir"
