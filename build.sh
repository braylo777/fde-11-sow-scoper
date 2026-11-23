#!/bin/sh
# Build the WASM core for the demo page, then run the tests.
set -eu
cd "$(dirname "$0")"
cargo test --locked
cargo build --locked --release --lib --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/fde_11_sow_scoper.wasm web/app.wasm
ls -l web/app.wasm
