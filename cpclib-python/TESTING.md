# Testing `cpclib-python` (PyBndTask)

This document shows how to build the Python extension and run the pytest suite that verifies the `PyBndTask` API.

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
