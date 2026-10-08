from typing import TypedDict

from ._types import OutputCallback

class Target(TypedDict):
    target: str
    dependencies: list[str]

class RunResult(TypedDict):
    target: str
    duration_ms: int
    stdout: str
    stderr: str

class Build:
    def __init__(self, path: str) -> None: ...
    @property
    def resolved_path(self) -> str: ...
    @property
    def default_target(self) -> str | None: ...
    def targets(self) -> list[Target]: ...
    def is_outdated(self, target: str) -> bool: ...
    def run(
        self, target: str | None = None, on_output: OutputCallback | None = None
    ) -> RunResult: ...
