cpclib-python
=============

Python bindings for cpclib, the Amstrad CPC cross-development library: assemble
and analyse Z80, crunch, build discs, snapshots and cartridges, convert
Arkos Tracker / CHIPNSFX / YM songs into players, run bndbuild builds and drive a
live emulator - from Python.

```python
from cpclib_python import asm, crunchers, music
from cpclib_python.disc import Disc

code = asm.assemble_file("main.asm", include_dirs=["lib"], defines={"DEBUG": 1})
packed = crunchers.compress(code.data, "zx0")

dsk = Disc.create("demo.dsk")
dsk.add_binary(packed.data, "DEMO.BIN", 0x4000)
dsk.save()

print(music.compare_table("tune.aks"))
```

The package ships type stubs (`py.typed`): editors complete and type-check it.

Modules
-------

| Module | What it does |
|--------|--------------|
| `asm` | The basm assembler with its options: defines, include dirs, symbols, listing, snapshot, cartridge. `assemble(source)`, `assemble_file(path)`. |
| `analysis` | Diagnostics (`check`), control-flow-aware NOP counts (`count_nops`) and peephole optimisation (`suggest_optimizations`, `apply_optimizations_in_place`). |
| `basic` | Locomotive BASIC: `tokenize`, `detokenize`, `line_numbers`. |
| `bdasm` | The Z80 disassembler: `disassemble` (address, bytes, text, NOPs), `disassemble_to_source`. |
| `bndbuild` | `Task`: any bndbuild tool as a task (`Task("img2cpc", [...]).execute(capture=True)`); builders for snapshots, Arkos Tracker, CHIPNSFX, MinYMiser, AYT. |
| `build` | bndbuild build files: `Build(path)` - `targets()`, `is_outdated()`, `run()`. |
| `cpr` | Plus-range cartridges: `Cartridge` - banks, `load`/`save`. |
| `crunchers` | ZX0, ZX7, Exomizer, LZSA, Shrinkler, apultra, UPKR, pucrunch, LZ4/48/49: `compress`, `compare`, `formats`. |
| `disc` | `.dsk` images: `Disc` - `create`, `catalog`, `add_binary`/`add_basic`/`add_ascii`/`add_file`, `extract`, `file_info`, `erase`, `rename`, sectors. |
| `emu` | `Emulator`: boot a snapshot or disc in a real emulator, read and write memory, type, take screenshots. |
| `image` | `render_screen`: screen memory to PNG. |
| `music` | Songs to standalone players with any of the players (AKG, AKM, AKY, CHIPNSFX, FAP, AYT, MinYMiser), measured: `build`, `compare`, `build_dsk`, `play`. |
| `sna` | Snapshots: `Snapshot` - `load`, flags, memory, `save`. |

Whatever runs a tool prints to the console, unless given `on_output=callback`:
a `callback(kind, text)` that receives `stdout`, `stderr` and, for builds,
`rule-start`/`rule-stop`/`rule-skipped`/`rule-failed`/`task-start`/`task-stop`.
`Task.execute(capture=True)` returns what was written instead.

The tools the pipelines need (Arkos Tracker 3, CHIPNSFX, FAP, AYT, MinYMiser,
emulators) are downloaded on first use; the Windows ones (CHIPNSFX, AYT,
MinYMiser) need `wine` on Linux.

Quick dev commands
------------------

This crate exposes pyo3-based bindings. Below are recommended steps to build, install into a Python
virtualenv, and run the Python-level tests. The steps assume a Linux environment; adjust package
installation commands for other distros.

Prerequisites
 - Install Rust (the workspace uses the toolchain pinned in `rust-toolchain.toml`; `rustup` will pick it)
 - Python with development headers (e.g. Debian/Ubuntu: `python3-dev` matching your Python minor version)
 - `maturin` for building/installing the pyo3 extension into a venv

Quick local build & test (recommended)

```bash
# from repository root
cd cpclib-python

# create and activate a virtualenv
python3 -m venv .venv
source .venv/bin/activate

# ensure pip/setuptools and maturin are available
pip install --upgrade pip setuptools maturin

# build & install the extension into the active venv (release is recommended for speed)
maturin develop --release

# run the python tests shipped in this crate
pytest -q tests

# when done, deactivate the venv
deactivate
```

If you prefer not to install into a venv, you can build the extension artifact with `maturin build` or
`cargo build -p cpclib-python` and import the produced module, but `maturin develop` is the simplest
workflow during development.

Run the Rust-side test harness (fallback)

If you only want to run the Rust unit tests for the bindings (they do not require installing Python
into a venv), run:

```bash
cd /home/romain/Perso/CPC/rust.cpcdemotools
cargo test -p cpclib-python
```

Troubleshooting
---------------
- Missing Python shared library at test runtime (example error: `libpython3.11.so.1.0: cannot open shared object file`):
	- Install the Python development package for your distribution (Debian/Ubuntu example:
		`sudo apt-get install python3.11-dev`) or ensure your `python3` and the dev headers match.
	- Use a virtualenv and `maturin develop` which will link against the venv Python binary.

- `maturin` build failures:
	- Ensure cargo and rust toolchain are available and match the workspace `rust-toolchain.toml`.
	- If you encounter platform-specific linking issues, check `maturin` docs and ensure `pkg-config`
		and `build-essential` (or equivalent) are installed.

