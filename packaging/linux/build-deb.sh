#!/usr/bin/env bash
# Bygger en .deb-pakke for Ubuntu/Debian.
# Kjøres på et Linux-system (eller i WSL) fra prosjektroten:
#   ./packaging/linux/build-deb.sh
set -euo pipefail

cd "$(dirname "$0")/../.."

need_apt=()
for pkg in build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev; do
    dpkg -s "$pkg" >/dev/null 2>&1 || need_apt+=("$pkg")
done

if [ ${#need_apt[@]} -gt 0 ]; then
    echo ">> Installerer byggeavhengigheter: ${need_apt[*]}"
    sudo apt-get update
    # Eldre Ubuntu har libappindicator3-dev i stedet for ayatana-varianten
    sudo apt-get install -y "${need_apt[@]}" \
        || sudo apt-get install -y build-essential pkg-config libgtk-3-dev libappindicator3-dev
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "!! cargo mangler. Installer Rust: https://rustup.rs" >&2
    exit 1
fi

if ! cargo deb --version >/dev/null 2>&1; then
    echo ">> Installerer cargo-deb"
    cargo install cargo-deb --locked
fi

mkdir -p dist
echo ">> Bygger pakke"
cargo deb --output dist/

echo
echo "Ferdig:"
ls -lh dist/*.deb
echo
echo "Installer med:  sudo apt install ./dist/syncthing-status_*.deb"
