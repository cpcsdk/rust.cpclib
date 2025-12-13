import pytest

# Basic smoke test for basm. This will only run if the extension is available in the environment.

def test_assemble_simple():
    import sys
    try:
        import cpclib_python
    except Exception as e:
        pytest.skip(f"cpclib_python not importable: {e}")

    code = """
    org 0
    db 1,2,3,4
    """

    b = cpclib_python.basm.assemble(code)
    assert isinstance(b, (bytes, bytearray))
    assert list(b) == [1, 2, 3, 4]


def test_assemble_directives_python():
    try:
        import cpclib_python
    except Exception as e:
        import pytest
        pytest.skip(f"cpclib_python not importable: {e}")

    code = """
    org &400
MYVAL:
    db MYVAL, MYVAL+1, MYVAL*2
    if MYVAL > 5
    db 99
    endif
label:
    dw label
    db 0xFF
"""

    b = cpclib_python.basm.assemble(code)
    assert isinstance(b, (bytes, bytearray))
    assert list(b) == [0, 1, 0, 99, 4, 4, 255]
