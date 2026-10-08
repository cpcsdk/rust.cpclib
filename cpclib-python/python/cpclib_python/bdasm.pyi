from typing import TypedDict

class Instruction(TypedDict):
    address: int
    bytes: bytes
    text: str
    nops: int | None

def disassemble(data: bytes, origin: int = 0) -> list[Instruction]: ...
def disassemble_to_source(data: bytes, origin: int = 0) -> str: ...
def bdasm_info() -> str: ...
