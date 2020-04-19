#!/bin/bash

 # I have partly followed the instructions there
 # https://www.reddit.com/r/rust/comments/6rxoty/tutorial_cross_compiling_from_linux_for_osx/
 # and
 # there
 # https://wapl.es/rust/2019/02/17/rust-cross-compile-linux-to-macos.html

export PATH=~/src/osxcross/target/bin:$PATH
export PKG_CONFIG_ALLOW_CROSS=1
export CC=o64-clang
export CXX=o64-clang++
export LIBZ_SYS_STATIC=1

#cd cpclib
cargo build --target=x86_64-apple-darwin --release --all-features