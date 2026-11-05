#!/bin/sh
# Builds the scale kernel to WebAssembly and places it where docs/globe.html loads it.
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/cewp_kernel_scale.wasm ../docs/kernel/kernel_scale.wasm
ls -la ../docs/kernel/kernel_scale.wasm
