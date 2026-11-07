#!/bin/sh
# Builds the scale kernel to WebAssembly and places it where docs/globe.html loads it.
set -e
cd "$(dirname "$0")"
PATH="/home/emoore/.cache/puccinialin/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:/home/emoore/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo build --release --target wasm32-unknown-unknown
if [ -f ../target/wasm32-unknown-unknown/release/cewp_kernel_scale.wasm ]; then
  cp ../target/wasm32-unknown-unknown/release/cewp_kernel_scale.wasm ../docs/kernel/kernel_scale.wasm
elif [ -f target/wasm32-unknown-unknown/release/cewp_kernel_scale.wasm ]; then
  cp target/wasm32-unknown-unknown/release/cewp_kernel_scale.wasm ../docs/kernel/kernel_scale.wasm
fi
ls -la ../docs/kernel/kernel_scale.wasm
