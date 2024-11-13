#!/bin/bash
set -e

cargo build --release
cargo run --bin uniffi-bindgen generate --library target/release/libmath.so --language kotlin --out-dir bindings
cargo run --bin uniffi-bindgen generate --library target/release/libmath.so --language swift --out-dir bindings
cargo run --bin uniffi-bindgen generate --library target/release/libmath.so --language python --out-dir bindings