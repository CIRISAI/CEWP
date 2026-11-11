#!/bin/sh
# Builds the scale kernel to WebAssembly and places it where docs/globe.html loads it.
set -e
# Panic paths would carry the build host's home directory into a public binary; remap them.
cd "$(dirname "$0")"
PATH="/home/emoore/.cache/puccinialin/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:/home/emoore/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" RUSTFLAGS="--remap-path-prefix=$HOME/.rustup=/rustup --remap-path-prefix=$HOME/.cargo=/cargo --remap-path-prefix=$(cd .. && pwd)=/cewp" cargo build --profile wasm --target wasm32-unknown-unknown
if [ -f ../target/wasm32-unknown-unknown/wasm/cewp_kernel_scale.wasm ]; then
  cp ../target/wasm32-unknown-unknown/wasm/cewp_kernel_scale.wasm ../docs/kernel/kernel_scale.wasm
elif [ -f target/wasm32-unknown-unknown/wasm/cewp_kernel_scale.wasm ]; then
  cp target/wasm32-unknown-unknown/wasm/cewp_kernel_scale.wasm ../docs/kernel/kernel_scale.wasm
fi
ls -la ../docs/kernel/kernel_scale.wasm
