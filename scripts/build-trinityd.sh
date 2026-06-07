#!/usr/bin/env bash
# Build Trinity daemon from source — https://github.com/5mil/Trinity
set -euo pipefail

SOURCE_URL="https://github.com/5mil/Trinity/archive/refs/heads/master.tar.gz"
BUILD_DIR="/tmp/trinityd-build"
OUT_DIR="$HOME/.trinityd"

echo "[build-trinityd] Installing build deps..."
sudo apt-get install -y -q build-essential libssl-dev libboost-all-dev libdb5.3++-dev \
  libminiupnpc-dev libzmq3-dev pkg-config libevent-dev automake libtool

mkdir -p "$BUILD_DIR" "$OUT_DIR"

echo "[build-trinityd] Downloading source from 5mil/Trinity..."
curl -sL "$SOURCE_URL" | tar -xz -C "$BUILD_DIR" --strip-components=1

cd "$BUILD_DIR"
echo "[build-trinityd] Running autogen..."
./autogen.sh

echo "[build-trinityd] Configuring..."
./configure --disable-wallet --without-gui --disable-tests --disable-bench \
  --with-incompatible-bdb CXXFLAGS="-O2 -march=native"

echo "[build-trinityd] Building (this takes ~5-10 min)..."
make -j$(nproc) src/trinityd

cp src/trinityd "$OUT_DIR/trinityd"
echo "[build-trinityd] Done: $OUT_DIR/trinityd"
