#!/usr/bin/env bash

set -e


PROGRAM="basm"

# STEP 0: Make sure there is no left-over profiling data from previous runs
rm -rf /tmp/pgo-data || true

# STEP 1: Build the instrumented binaries
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" \
  cargo build --release

# STEP 2: Run the instrumented binaries with some typical data
../target/release/$PROGRAM ../../z80_assemblers_benchmark/z80/include_files.asm > /dev/null
../target/release/$PROGRAM ./tests/asm/roudoudou/rasm_sprites.asm  > /dev/null

# STEP 3: Merge the `.profraw` files into a `.profdata` file
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

# STEP 4: Use the `.profdata` file for guiding optimizations
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" \
    cargo build --release