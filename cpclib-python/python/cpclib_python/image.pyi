from typing import TypedDict

from ._types import OutputCallback

class RenderedScreen(TypedDict, total=False):
    png: bytes
    address: int
    width: int
    height: int
    mode: int
    encoding: str
    palette: str

def screen_address(r12: int, r13: int) -> int: ...
def render_screen(
    memory: bytes,
    *,
    address: int = 0xC000,
    mode: int = 1,
    width: int | None = None,
    height: int | None = None,
    palette: list[int] | None = None,
    palette_override: list[int | None] | None = None,
    lines_per_char_row: int = 8,
    encoding: str = "screen",
) -> RenderedScreen: ...

def convert(
    source: str,
    to: str,
    output: str,
    *,
    mode: int | None = None,
    pens: dict[int, int] | None = None,
    crop: bool = False,
    fullscreen: bool = False,
    overscan: bool = False,
    standard: bool = False,
    dither: str | None = None,
    colors: int | None = None,
    resize_filter: str | None = None,
    extra_args: list[str] | None = None,
    target_args: list[str] | None = None,
    on_output: OutputCallback | None = None,
) -> None: ...
