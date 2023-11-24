#!/bin/bash
set -e

cargo build -p test_wasm_foo --release --target wasm32-unknown-unknown --target-dir ./fixture/zomes/foo/target
hc dna pack ./fixture -o ./fixture/test.dna
hc app pack ./fixture -o ./fixture/test.happ
