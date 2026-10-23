#!/usr/bin/env bash
# Kolay'ı derler ve 'kolay' komutunu ~/.local/bin içine bağlar.
# Kaldırmak için: rm ~/.local/bin/kolay
set -e
cd "$(dirname "$0")"
cargo build --release
mkdir -p "$HOME/.local/bin"
ln -sf "$(pwd)/target/release/kolay" "$HOME/.local/bin/kolay"
echo "Kuruldu: $HOME/.local/bin/kolay"
echo "Denemek için: kolay ornekler/01_merhaba.kolay"
