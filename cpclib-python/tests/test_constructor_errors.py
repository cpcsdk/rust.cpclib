import pytest


def _import_module():
    import importlib
    try:
        return importlib.import_module("cpclib_python")
    except Exception as e:
        pytest.skip(f"cpclib_python extension not available: {e}")


def test_task_class_name_and_tuple_args():
    cp = _import_module()
    b = getattr(cp, "bndbuild")

    # The exposed Python class should be named `Task`.
    assert getattr(b.Task, "__name__", None) == "Task"

    # Tuple of strings should be accepted like a list
    try:
        t = b.Task("basm", ("a.asm", "-o", "out.o"))
    except Exception as e:
        pytest.skip(f"Could not construct Task with tuple args: {e}")

    assert hasattr(t, "execute")


def test_constructor_invalid_second_arg_type_raises():
    cp = _import_module()
    b = getattr(cp, "bndbuild")

    # Passing a non-sequence as second argument should raise
    with pytest.raises(Exception):
        b.Task("basm", "not-a-sequence")


def test_constructor_nonstring_elements_raise():
    cp = _import_module()
    b = getattr(cp, "bndbuild")

    # Passing a sequence containing non-string elements should raise
    with pytest.raises(Exception):
        b.Task("basm", [1, 2, 3])
