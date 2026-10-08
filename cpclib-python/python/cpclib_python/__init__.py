"""Python bindings for cpclib, the Amstrad CPC cross-development library.

The submodules:

- `asm`, `basm`: the assembler;
- `analysis`: diagnostics, NOP counts and peephole optimisation of Z80 sources;
- `basic`: Locomotive BASIC, tokenized and back;
- `bdasm`: the Z80 disassembler;
- `bndbuild`: bndbuild tasks (every tool bndbuild knows), with builders;
- `build`: bndbuild build files;
- `cpr`: Plus-range cartridges;
- `crunchers`: ZX0, ZX7, Exomizer, LZSA, Shrinkler... compression;
- `disc`: `.dsk` images and their AMSDOS catalog;
- `emu`: drive a live emulator;
- `image`: screen memory to PNG;
- `music`: songs to standalone players, with any of the players;
- `sna`: snapshots.
"""
from . import cpclib_python as _native
from .cpclib_python import crate_info, hello

# the submodules, imported by name so that tools can see them
from .cpclib_python import (  # noqa: F401
    analysis,
    asm,
    basic,
    basm,
    bdasm,
    bndbuild,
    build,
    cpr,
    crunchers,
    disc,
    emu,
    image,
    music,
    sna,
)

__all__ = [
    "analysis", "asm", "basic", "basm", "bdasm", "bndbuild", "build", "cpr",
    "crunchers", "crate_info", "disc", "emu", "hello", "image", "music", "sna",
]
