from typing import TypedDict

class CatalogEntry(TypedDict):
    filename: str
    user: int
    size_kb: int
    read_only: bool
    system: bool

class FileInfo(TypedDict, total=False):
    length: int
    type: str
    load_address: int
    execution_address: int

class Disc:
    @staticmethod
    def open(path: str) -> Disc: ...
    @staticmethod
    def create(path: str, format: str = "data") -> Disc: ...
    @property
    def path(self) -> str | None: ...
    def catalog(self) -> list[CatalogEntry]: ...
    def extract(self, name: str, user: int = 0) -> bytes: ...
    def file_info(self, name: str, user: int = 0) -> FileInfo: ...
    def add_file(
        self, path: str, name: str | None = None, user: int = 0, replace: bool = True
    ) -> None: ...
    def add_binary(
        self,
        data: bytes,
        name: str,
        load_address: int,
        execution_address: int | None = None,
        user: int = 0,
        replace: bool = True,
    ) -> None: ...
    def add_basic(
        self, data: bytes, name: str, user: int = 0, replace: bool = True
    ) -> None: ...
    def add_ascii(
        self, data: bytes, name: str, user: int = 0, replace: bool = True
    ) -> None: ...
    def erase(self, name: str, user: int = 0, wipe: bool = False) -> None: ...
    def rename(self, name: str, new_name: str, user: int = 0) -> None: ...
    def read_sector(self, head: int, track: int, sector_id: int) -> bytes: ...
    def write_sector(
        self, head: int, track: int, sector_id: int, data: bytes
    ) -> None: ...
    def save(self, path: str | None = None) -> None: ...

class AmsdosHeaderInfo(TypedDict, total=False):
    type: str
    length: int
    load_address: int
    execution_address: int

def add_amsdos_header(
    data: bytes, name: str, load_address: int, execution_address: int | None = None
) -> bytes: ...
def read_amsdos_header(data: bytes) -> tuple[AmsdosHeaderInfo | None, bytes]: ...
