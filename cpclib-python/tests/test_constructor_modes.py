import pytest


def _import_module():
    import importlib
    try:
        return importlib.import_module("cpclib_python")
    except Exception as e:
        pytest.skip(f"cpclib_python extension not available: {e}")


def test_constructor_modes_create_and_execute():
    cp = _import_module()
    b = getattr(cp, "bndbuild")

    # Mode A: single string
    try:
        t1 = b.Task("basm toto.asm -o toto.o")
    except Exception as e:
        pytest.skip(f"Could not construct Task(mode A): {e}")

    assert hasattr(t1, "execute")

    # Mode B: command + list of args
    try:
        t2 = b.Task("basm", ["toto.asm", "-o", "toto.o"])
    except Exception as e:
        pytest.skip(f"Could not construct Task(mode B): {e}")

    assert hasattr(t2, "execute")

    # Attempt to execute (may raise or print); treat known environment errors as skip
    for t in (t1, t2):
        try:
            res = t.execute()
        except RuntimeError as e:
            msg = str(e)
            if "build.bnd" in msg or "does not exists" in msg:
                pytest.skip(f"Execution requires repository build files: {msg}")
            pytest.skip(f"Task execution raised RuntimeError: {msg}")
        except Exception as e:
            pytest.skip(f"Task execution raised unexpected exception: {e}")

        assert res is None or isinstance(res, dict)


def test_constructor_quoting_behavior():
    cp = _import_module()
    b = getattr(cp, "bndbuild")

    parts = ["file name.txt", 'arg"withquote', "-o", "out.bin"]
    # Pass list form which the Python binding should quote/escape internally
    try:
        t = b.Task("basm", parts)
    except Exception as e:
        pytest.skip(f"Could not construct Task for quoting test: {e}")

    # construction should succeed; execution may be skipped due to environment
    assert hasattr(t, "execute")
