#!/bin/bash

# shellcheck disable=SC2164

set -e

export HOME=/root
export TERM=xterm
# shellcheck disable=SC2155
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin:/usr/lib/gcc/$(arch)-linux-gnu/4.8"
# shellcheck disable=SC2155
export LD_LIBRARY_PATH="/usr/lib/gcc/$(arch)-linux-gnu/4.8"

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.76.0 -y

apt-get install -y autoconf automake build-essential curl gawk gettext texinfo libgbm-dev libdrm-dev libudev-dev libdbus-1-dev libegl1-mesa-dev pkg-config
ln -sf clang /usr/bin/cc
rm -f /usr/lib/x86_64-linux-gnu/libEGL.so{,.1}
cp -Lf /usr/lib/$(arch)-linux-gnu/mesa-egl/libEGL.so /usr/lib/$(arch)-linux-gnu/libEGL.so.1
ln -sf libEGL.so.1 /usr/lib/$(arch)-linux-gnu/libEGL.so
cp "$SRC_PATH"/support/clang /usr/bin/
cp "$SRC_PATH"/support/ar /usr/bin/

cd "$OUT_PATH"
curl -LO https://ziglang.org/download/0.12.0/zig-linux-$(arch)-0.12.0.tar.xz
tar xf zig-linux-*.tar.xz && rm zig-linux*.tar.xz
mkdir /app && mv zig-linux* /app/zig

export PATH="$HOME/.cargo/bin:/app/zig:$PATH"
cd "$SRC_PATH"/src/sys_info_v2/gatherer
rm -rf target
cargo build --release
install -v -p -m 755 target/release/gatherer "$OUT_PATH"/missioncenter-gatherer

