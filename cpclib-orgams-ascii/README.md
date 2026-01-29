# cpclib-orgams-ascii

Rust library for reading and writing Orgams binary files (.O format).

## What is Orgams?

Orgams is a Z80 assembler for the Amstrad CPC. It uses a preprocessed binary format (.O files) that is more compact than plain ASCII assembly (.Z80 files).

## Progress

The library is not ready at all.
But we are currently able to parse some files:

- tests/orgams-main/bricbrac/STRING.O
- tests/orgams-main/CONST.I
- tests/orgams-main/MACRO.I
- tests/orgams-main/MEMMAP.I
- tests/orgams-main/SWAPI.I
- tests/orgams-main/bricbrac/BRICMAP.I
- tests/orgams-main/bricbrac/CONV.O
