#!/bin/bash

cd fixture/zomes/foo
cargo build --release --target wasm32-unknown-unknown --target-dir ./target
cd ../.. # into fixtures
hc dna pack . -o test.dna
hc app pack . -o test.happ
cd .. # into root folder
