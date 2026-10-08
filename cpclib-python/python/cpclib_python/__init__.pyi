from typing import Any

from . import (
    analysis as analysis,
    asm as asm,
    basic as basic,
    basm as basm,
    bdasm as bdasm,
    bndbuild as bndbuild,
    build as build,
    cpr as cpr,
    crunchers as crunchers,
    csl as csl,
    disc as disc,
    emu as emu,
    fmt as fmt,
    image as image,
    music as music,
    orgams as orgams,
    sna as sna,
    tools as tools,
    xfer as xfer,
)

def hello() -> str: ...
def crate_info() -> dict[str, Any]: ...
