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
    disc as disc,
    emu as emu,
    image as image,
    music as music,
    sna as sna,
)

def hello() -> str: ...
def crate_info() -> dict[str, Any]: ...
