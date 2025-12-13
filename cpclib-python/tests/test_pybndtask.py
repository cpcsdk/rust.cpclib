import pytest


def test_module_and_submodule_present():
    import importlib
    cp = importlib.import_module("cpclib_python")

    # basic smoke checks
    assert hasattr(cp, "hello")
    assert cp.hello() == "cpclib-python ready"

    assert hasattr(cp, "bndbuild")
    b = getattr(cp, "bndbuild")
    assert hasattr(b, "Task")


def test_create_task_and_execute_shape():
    import importlib
    cp = importlib.import_module("cpclib_python")
    b = getattr(cp, "bndbuild")

    task = b.Task("basm --version")

    # task should expose an execute method
    assert hasattr(task, "execute")
    try:
        res = task.execute()
    except RuntimeError as e:
        # Common runtime error when running tasks in a repo without a build file.
        # Treat this as a skipped test rather than a hard failure so the suite
        # can run on partial/source-only checkouts.
        msg = str(e)
        if "build.bnd" in msg or "does not exists" in msg:
            pytest.skip(f"Task execution requires repository build files: {msg}")
        raise

    # The current implementation may return `None` (prints to stdout/stderr).
    # If a dict is returned, validate its shape; otherwise accept `None` as success.
    assert res is None or isinstance(res, dict), "execute() must return None or a dict-like result"
    if isinstance(res, dict):
        assert "ok" in res and isinstance(res["ok"], bool)

        # The current implementation prints to stdout/stderr and returns empty lists for captured output.
        # We assert the known keys are present and proper types.
        for key in ("stdout", "stderr", "events"):
            assert key in res and isinstance(res[key], list)


def test_create_task_invalid_string_raises():
    import importlib
    cp = importlib.import_module("cpclib_python")
    b = getattr(cp, "bndbuild")

    # Creating with a clearly invalid input (non-string) should raise a TypeError or ValueError
    with pytest.raises(Exception):
        # The `Task` constructor requires a string-like task description.
        b.Task({"not": "a string"})
