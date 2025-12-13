cpclib-python
===============

Minimal Python bindings for selected cpclib crates using pyo3.

This crate currently provides placeholder functions and small submodules for:
- `cpclib-asm`
- `cpclib-basic`
- `cpclib-basm`
- `cpclib-bdasm`
- `cpclib-bndbuild`
- `cpclib-cpr`
- `cpclib-crunchers`

Extend the submodules to call into the underlying Rust crate APIs.

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

