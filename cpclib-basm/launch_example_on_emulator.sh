#!/bin/bash

# example:
# ./launch_example_on_emulator.sh good_zx0_backward_decrunch

EXAMPLE="$1"

cargo run -- "tests/asm/$EXAMPLE.asm" --snapshot -o test.sna --lst test.lst && \
cargo run --bin bndbuild --manifest-path ../cpclib-bndbuild/Cargo.toml -- --direct -- ace test.sna