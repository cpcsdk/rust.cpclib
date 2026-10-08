# Testing `cpclib-python`

This document shows how to build the Python extension and run its pytest suite.

Prerequisites
- Python 3.8+ with `pip`.
- `maturin` to build/install the pyo3 extension: `pip install maturin`
- `pytest`: `pip install pytest`

Quick steps

1. Build and install the extension into your active Python environment (this runs a local build in editable mode):

```bash
cd /home/romain/Perso/CPC/rust.cpcdemotools/cpclib-python
maturin develop --release
```

2. Run the tests using pytest:

```bash
pytest -q
```

Notes
- The test suite is permissive: if the `bndbuild` task parser is not available in the compiled extension, the test will be skipped rather than fail. This makes it safe to run the tests on partial builds.
- If you prefer to run the built wheel instead of `maturin develop`, build a wheel and install it into a virtualenv:

```bash
maturin build --release
pip install target/wheels/cpclib_python-*.whl
```

- If you need assistance packaging or running the tests in CI, tell me your CI environment and I can produce a minimal `workflow` or `tox` file.


Tests that need more than the extension
- `CPCLIB_REAL_TOOLS=1`: the tests that really build players (`tests/test_music.py`) download the
  conversion tools on first use, and need `wine` on Linux for CHIPNSFX, AYT and MinYMiser.
- `CPCLIB_REAL_EMULATOR=1`: `tests/test_emu.py` boots a real emulator (a window, sound).
- `tests/test_tools_help.py` starts every tool, GUI emulators included: it needs a machine
  with a display and OpenGL.

Type stubs
- `python/cpclib_python/tools.pyi` is generated: after bndbuild gains or loses a tool, run `python dev/gen_tools_stub.py` (a test checks it covers every tool).
- The stubs in `python/cpclib_python/*.pyi` follow the Rust signatures by hand. After changing one,
  check them: `pip install mypy && mypy --strict python/cpclib_python/*.pyi`.
