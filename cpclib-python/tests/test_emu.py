"""Driving an emulator. The live test boots a real one (a window, sound): it
runs only with CPCLIB_REAL_EMULATOR=1."""
import os
import time

import pytest

from cpclib_python import emu


def test_the_known_emulators():
    assert {"ace", "amspiritlite", "sugarbox", "winape"} <= set(emu.emulators())


def test_an_unknown_emulator_is_a_value_error():
    with pytest.raises(ValueError, match="expected one of"):
        emu.Emulator("nope")


@pytest.mark.skipif(
    os.environ.get("CPCLIB_REAL_EMULATOR") != "1",
    reason="opens a real emulator: set CPCLIB_REAL_EMULATOR=1",
)
def test_a_live_emulator_shows_memory_and_screen(tmp_path):
    from cpclib_python import asm

    sna = tmp_path / "p.sna"
    asm.assemble_file(_write(tmp_path / "p.asm", "org #4000\n run $\n jp $\n"), snapshot=str(sna))
    with emu.Emulator("amspiritlite", snapshot=str(sna)) as live:
        # the snapshot may still be loading: give it a few seconds
        for _ in range(20):
            if live.read_memory(0x4000, 3) == b"\xc3\x00\x40":
                break
            time.sleep(0.25)
        assert live.read_memory(0x4000, 3) == b"\xc3\x00\x40"
        assert live.screenshot()[:8] == b"\x89PNG\r\n\x1a\n"
    with pytest.raises(RuntimeError, match="closed"):
        live.read_memory(0, 1)


def _write(path, text):
    path.write_text(text)
    return str(path)
