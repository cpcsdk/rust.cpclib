from typing import TypedDict

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
