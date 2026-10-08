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
- `csl`: CSL scenario scripts;
- `disc`: `.dsk` images and their AMSDOS catalog;
- `emu`: drive a live emulator;
- `fmt`: the source formatter;
- `image`: picture conversion, screen memory to PNG;
- `music`: songs to standalone players, with any of the players;
- `orgams`: Orgams binary sources;
- `sna`: snapshots;
- `tools`: every other tool bndbuild knows, as a function;
- `xfer`: the M4 board / CPC Wifi.
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
    csl,
    disc,
    emu,
    fmt,
    image,
    music,
    orgams,
    sna,
    xfer,
)
from . import tools

__all__ = [
    "analysis", "asm", "basic", "basm", "bdasm", "bndbuild", "build", "cpr",
    "crunchers", "crate_info", "csl", "disc", "emu", "fmt", "hello", "image",
    "music", "orgams", "sna", "tools", "xfer",
]
