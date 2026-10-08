from typing import Any

def check(path: str | None = None, code: str | None = None) -> dict[str, Any]: ...
def count_nops(
    start_line: int,
    end_line: int,
    path: str | None = None,
    code: str | None = None,
) -> dict[str, Any]: ...
def suggest_optimizations(
    path: str,
    goal: str | None = None,
    disabled_rules: list[str] | None = None,
    include_dirs: list[str] | None = None,
    defines: list[str] | None = None,
    include_project: bool = False,
) -> dict[str, Any]: ...
def apply_optimizations_in_place(
    path: str,
    goal: str | None = None,
    disabled_rules: list[str] | None = None,
    include_dirs: list[str] | None = None,
    defines: list[str] | None = None,
) -> dict[str, Any]: ...
def apply_optimizations_project_in_place(
    project_dir: str,
    goal: str | None = None,
    disabled_rules: list[str] | None = None,
    include_dirs: list[str] | None = None,
    defines: list[str] | None = None,
) -> dict[str, Any]: ...
