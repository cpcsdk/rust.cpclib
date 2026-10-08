"""crunchers, sna and disc: the libraries behind them, from Python."""
import pytest

from cpclib_python import crunchers
from cpclib_python.disc import Disc
from cpclib_python.sna import Snapshot

DATA = bytes(range(256)) * 8


# ---- crunchers --------------------------------------------------------------

def test_formats_include_the_usual_crunchers():
    assert {"zx0", "exomizer", "lzsa1", "shrinkler", "upkr"} <= set(crunchers.formats())


def test_compress_makes_it_smaller_and_reports_its_format():
    result = crunchers.compress(DATA, "zx0")
    assert result.format == "zx0"
    assert 0 < len(result) < len(DATA)
    assert isinstance(result.data, bytes) and len(result.data) == len(result)


def test_none_keeps_the_data():
    assert crunchers.compress(DATA, "none").data == DATA


def test_unknown_format_is_a_value_error():
    with pytest.raises(ValueError, match="expected one of"):
        crunchers.compress(DATA, "nope")


def test_compare_sorts_smallest_first():
    rows = crunchers.compare(DATA, ["zx0", "lz4", "none"])
    assert [r[0] for r in rows][-1] == "none"
    sizes = [r[1] for r in rows]
    assert sizes == sorted(sizes)
    assert all(r[2] is None for r in rows)


# ---- sna --------------------------------------------------------------------

def test_a_new_snapshot_has_memory_and_registers():
    sna = Snapshot()
    assert sna.memory_kb in (64, 128)
    assert isinstance(sna.get("Z80_PC"), int)
    assert len(sna.get("CRTC_REG")) == 18
    assert "Z80_PC" in sna.flags()


def test_patching_a_snapshot_survives_a_save_and_load(tmp_path):
    sna = Snapshot()
    sna.write(0x4000, b"\x01\x02\x03")
    sna.set("Z80_PC", 0x4000)
    sna.set("CRTC_REG:1", 40)
    path = tmp_path / "patched.sna"
    sna.save(str(path))

    again = Snapshot.load(str(path))
    assert again.read(0x4000, 3) == b"\x01\x02\x03"
    assert again.get("Z80_PC") == 0x4000
    assert again.get("CRTC_REG:1") == 40
    assert len(again.memory()) >= 64 * 1024
    assert Snapshot.from_bytes(sna.to_bytes()).read(0x4000, 3) == b"\x01\x02\x03"


def test_snapshot_errors():
    sna = Snapshot()
    with pytest.raises(ValueError, match="flags look like"):
        sna.get("NOPE")
    with pytest.raises(IndexError):
        sna.read(0xFFFFF, 4)
    with pytest.raises(ValueError, match="version"):
        sna.save("/tmp/never.sna", 9)


# ---- disc -------------------------------------------------------------------

def test_a_disc_keeps_the_files_added_to_it(tmp_path):
    path = str(tmp_path / "game.dsk")
    dsk = Disc.create(path)
    assert dsk.catalog() == []

    dsk.add_binary(b"\xc9" * 300, "GAME.BIN", 0x4000, 0x4010)
    dsk.add_ascii(b"hello", "README.TXT")
    dsk.save()

    again = Disc.open(path)
    assert {e["filename"] for e in again.catalog()} == {"GAME.BIN", "README.TXT"}
    assert again.extract("GAME.BIN") == b"\xc9" * 300
    assert again.file_info("GAME.BIN") == {
        "length": 300, "type": "binary", "load_address": 0x4000, "execution_address": 0x4010,
    }
    assert again.file_info("README.TXT")["type"] == "ascii"


def test_erase_and_rename(tmp_path):
    dsk = Disc.create(str(tmp_path / "a.dsk"))
    dsk.add_ascii(b"one", "ONE.TXT")
    dsk.rename("ONE.TXT", "TWO.TXT")
    assert [e["filename"] for e in dsk.catalog()] == ["TWO.TXT"]
    dsk.erase("TWO.TXT")
    assert dsk.catalog() == []
    with pytest.raises(KeyError):
        dsk.extract("TWO.TXT")


def test_adding_twice_without_replace_fails(tmp_path):
    dsk = Disc.create(str(tmp_path / "b.dsk"))
    dsk.add_ascii(b"x", "X.TXT")
    with pytest.raises(RuntimeError):
        dsk.add_ascii(b"y", "X.TXT", replace=False)


def test_sectors_are_reachable(tmp_path):
    dsk = Disc.create(str(tmp_path / "c.dsk"))
    sector = dsk.read_sector(0, 0, 0xC1)
    assert len(sector) == 512
    with pytest.raises(KeyError):
        dsk.read_sector(0, 0, 0x99)
