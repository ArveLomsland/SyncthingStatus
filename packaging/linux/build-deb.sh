#!/usr/bin/env bash
# Builds a .deb package for Ubuntu/Debian.
# Run on a Linux system (or in WSL) from the project root:
#   ./packaging/linux/build-deb.sh
set -euo pipefail

cd "$(dirname "$0")/../.."

need_apt=()
for pkg in build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev; do
    dpkg -s "$pkg" >/dev/null 2>&1 || need_apt+=("$pkg")
done

if [ ${#need_apt[@]} -gt 0 ]; then
    echo ">> Installing build dependencies: ${need_apt[*]}"
    sudo apt-get update
    # Older Ubuntu releases ship libappindicator3-dev instead of the ayatana variant
    sudo apt-get install -y "${need_apt[@]}" \
        || sudo apt-get install -y build-essential pkg-config libgtk-3-dev libappindicator3-dev
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "!! cargo not found. Install Rust: https://rustup.rs" >&2
    exit 1
fi

if ! cargo deb --version >/dev/null 2>&1; then
    echo ">> Installing cargo-deb"
    cargo install cargo-deb --locked
fi

mkdir -p dist
echo ">> Building package"
cargo deb --output dist/

echo
echo "Done:"
ls -lh dist/*.deb
echo
echo "Install with:  sudo apt install ./dist/syncthing-status_*.deb"
