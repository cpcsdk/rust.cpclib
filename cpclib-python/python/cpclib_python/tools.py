"""Every tool bndbuild knows, as a function.

    from cpclib_python import tools

    tools.commands()                         # every tool: name, aliases, description, synopsis
    tools.run("img2cpc", "pic.png", "--mode", "1", "scr", "-o", "pic.scr")
    tools.catalog("game.dsk", "--list", capture=True)["stdout"]
    tools.basm("main.asm", "-o", "main.bin", on_output=print)

A function takes the tool's command line arguments (as strings) and runs it
in-process or as the downloaded tool: its output goes to the console, to
`on_output(kind, text)` if given, and with `capture=True` is returned as
`{"stdout": ..., "stderr": ...}`. It raises `RuntimeError` if the tool fails.

The modules of the package (`asm`, `disc`, `sna`, `crunchers`, ...) wrap the
common ones with typed arguments; this is for all the others, and for every
option the wrappers do not name.
"""
from __future__ import annotations

import keyword
from typing import Any, Callable

from .cpclib_python import _tools

__all__ = ["commands", "run"]


def commands() -> list[dict[str, Any]]:
    """Every command: `{"name", "aliases", "description", "synopsis", "example"}`."""
    return _tools.commands()


def run(
    command: str,
    *args: str,
    on_output: Callable[[str, str], None] | None = None,
    capture: bool = False,
) -> dict[str, str] | None:
    """Runs the tool `command` (a name or an alias of `commands()`) with `args`."""
    return _tools.run(command, list(args), on_output, capture)


def _identifier(name: str) -> str:
    return name.replace("-", "_")


def _function(command: str) -> Callable[..., dict[str, str] | None]:
    def tool(
        *args: str,
        on_output: Callable[[str, str], None] | None = None,
        capture: bool = False,
    ) -> dict[str, str] | None:
        return run(command, *args, on_output=on_output, capture=capture)

    tool.__name__ = tool.__qualname__ = _identifier(command)
    tool.__doc__ = f"Runs `{command}` with its command line arguments - see `commands()`."
    return tool


_FUNCTIONS: dict[str, Callable[..., dict[str, str] | None]] = {}
for _entry in commands():
    for _name in (_entry["name"], *_entry["aliases"]):
        if _identifier(_name).isidentifier() and not keyword.iskeyword(_identifier(_name)):
            _FUNCTIONS.setdefault(_identifier(_name), _function(_name))


def __getattr__(name: str) -> Callable[..., dict[str, str] | None]:
    try:
        return _FUNCTIONS[name]
    except KeyError:
        raise AttributeError(f"module 'cpclib_python.tools' has no tool '{name}'") from None


def __dir__() -> list[str]:
    return sorted(set(__all__) | set(_FUNCTIONS))
