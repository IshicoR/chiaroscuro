#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
flatpak_dir="$repo_root/packaging/flatpak"
build_dir="$flatpak_dir/build"
repo_dir="$flatpak_dir/repo"
bundle_path="$repo_root/dist/fedora/Chiaroscuro.flatpak"

command -v cargo >/dev/null || {
    echo "cargo is required." >&2
    exit 1
}
command -v flatpak-builder >/dev/null || {
    echo "flatpak-builder is required. Install it with: sudo dnf install flatpak-builder" >&2
    exit 1
}

mkdir -p "$repo_root/dist/fedora"
rm -rf "$flatpak_dir/vendor" "$build_dir" "$repo_dir"

(
    cd "$repo_root"
    cargo vendor --locked "$flatpak_dir/vendor" >/dev/null
)

flatpak-builder --force-clean --install-deps-from=flathub --repo="$repo_dir" "$build_dir" "$flatpak_dir/io.github.IshicoR.Chiaroscuro.yml"
flatpak build-bundle "$repo_dir" "$bundle_path" io.github.IshicoR.Chiaroscuro

echo "Unsigned Fedora bundle created at $bundle_path"
