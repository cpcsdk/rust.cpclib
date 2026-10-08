from ._types import OutputCallback

class Assembled:
    @property
    def data(self) -> bytes | None: ...
    @property
    def symbols(self) -> dict[str, int]: ...
    @property
    def listing(self) -> str | None: ...
    @property
    def messages(self) -> str: ...

def assemble(
    source: str,
    *,
    defines: dict[str, object] | None = None,
    include_dirs: list[str] | None = None,
    case_insensitive: bool = False,
    symbols: bool = True,
    listing: bool = False,
    extra_args: list[str] | None = None,
    on_output: OutputCallback | None = None,
) -> Assembled: ...
def assemble_file(
    path: str,
    *,
    defines: dict[str, object] | None = None,
    include_dirs: list[str] | None = None,
    case_insensitive: bool = False,
    symbols: bool = True,
    listing: bool = False,
    snapshot: str | None = None,
    cartridge: str | None = None,
    extra_args: list[str] | None = None,
    on_output: OutputCallback | None = None,
) -> Assembled: ...
def asm_info() -> str: ...
