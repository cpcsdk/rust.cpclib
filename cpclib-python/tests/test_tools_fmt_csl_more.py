"""tools, fmt, csl, orgams, xfer, image.convert, disc headers and basic.to_snapshot."""
import re
from pathlib import Path

import pytest

from cpclib_python import asm, basic, csl, disc, fmt, image, orgams, tools, xfer
from cpclib_python.sna import Snapshot

STUB = Path(__file__).resolve().parents[1] / "python" / "cpclib_python" / "tools.pyi"


# ---- tools ------------------------------------------------------------------

def test_commands_describe_every_tool():
    names = {c["name"] for c in tools.commands()}
    assert {"basm", "dsk", "img2cpc", "catalog", "hideur", "xfer", "sna", "crunch"} <= names
    basm = next(c for c in tools.commands() if c["name"] == "basm")
    assert "assemble" in basm["aliases"] and basm["synopsis"].startswith("basm")


def test_a_tool_runs_as_a_function_and_captures_its_output():
    assert "hello" in tools.echo("hello", capture=True)["stdout"]
    seen = []
    tools.run("echo", "hi", on_output=lambda kind, text: seen.append((kind, text)))
    assert ("stdout", "hi") in [(k, t.strip()) for k, t in seen]


def test_aliases_and_unknown_tools():
    assert tools.assemble is not None and tools.basm.__name__ == "basm"
    with pytest.raises(AttributeError):
        tools.nonexistent
    with pytest.raises(ValueError):
        tools.run("nonexistent-tool")


def test_every_tool_has_a_function_in_the_stub():
    stubbed = set(re.findall(r"^def (\w+)\(", STUB.read_text(), re.M))
    import keyword
    for entry in tools.commands():
        for name in (entry["name"], *entry["aliases"]):
            ident = name.replace("-", "_")
            if ident.isidentifier() and not keyword.iskeyword(ident):
                assert ident in stubbed, f"{ident} is missing from tools.pyi"


# ---- fmt --------------------------------------------------------------------

def test_format_source_and_options():
    out = fmt.format_source("ld a,1\nloop:djnz loop\n")
    assert out == "    LD A,1\nloop:\n    DJNZ loop\n"
    assert fmt.default_options()["indent_size"] == 4
    wide = fmt.format_source("start:\nld a,1\n", {"indent_size": 8})
    assert "        LD A,1" in wide
    lower = fmt.format_source("LD A,1\n", {"mnemonic_case": "LowerCase", "register_case": "LowerCase"})
    assert "ld a,1" in lower
    with pytest.raises(ValueError, match="options"):
        fmt.format_source("nop", {"indent_size": "wide"})


def test_format_range_leaves_the_rest_alone():
    src = "ld a,1\nld b,2\nld c,3\n"
    out = fmt.format_range(src, 2, 2)
    assert out.splitlines()[0] == "ld a,1" and out.splitlines()[2] == "ld c,3"
    assert out.splitlines()[1].strip() == "LD B,2"


# ---- csl --------------------------------------------------------------------

def test_csl_parse_and_errors():
    script = "csl_version 1.0\nreset soft\nwait 1000000\n"
    instructions = csl.parse(script)
    assert instructions[0].startswith("csl_version") and len(instructions) == 3
    assert "reset" in csl.normalize(script)
    with pytest.raises(ValueError):
        csl.parse("this is not csl")


# ---- orgams / xfer ----------------------------------------------------------

def test_orgams_rejects_what_is_not_an_orgams_source():
    with pytest.raises(ValueError):
        orgams.to_utf8(b"definitely not orgams")


def test_xfer_wraps_the_m4_without_touching_the_network():
    m4 = xfer.M4("192.0.2.1")
    assert m4.hostname == "192.0.2.1" and "192.0.2.1" in repr(m4)
    with pytest.raises(ValueError, match="file type"):
        m4.upload("missing.bin", header=("nonsense", 0, 0))


# ---- image.convert ----------------------------------------------------------

def test_convert_rejects_bad_options(tmp_path):
    with pytest.raises(ValueError, match="mode"):
        image.convert("x.png", "scr", "x.scr", mode=5)
    with pytest.raises(ValueError, match="target"):
        image.convert("x.png", "nope", "x")
    with pytest.raises(ValueError, match="pens"):
        image.convert("x.png", "scr", "x.scr", pens={16: 0})


def _png(path, width=160, height=200):
    """A small valid PNG, without any dependency."""
    import struct
    import zlib

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    row = b"\x00" + bytes([0, 0, 0]) * (width // 2) + bytes([255, 255, 0]) * (width - width // 2)
    raw = row * height
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


def test_convert_a_picture_to_a_screen_and_a_snapshot(tmp_path):
    png = tmp_path / "pic.png"
    _png(png)
    scr = tmp_path / "pic.scr"
    image.convert(str(png), "scr", str(scr), mode=1, pens={0: 0, 1: 24}, crop=True)
    assert scr.stat().st_size > 0

    sna = tmp_path / "pic.sna"
    image.convert(str(png), "sna", str(sna), mode=1, crop=True)
    assert Snapshot.load(str(sna)).memory_kb >= 64


# ---- disc: AMSDOS headers ---------------------------------------------------

def test_amsdos_header_round_trip():
    with_header = disc.add_amsdos_header(b"\x01\x02\x03", "TEST.BIN", 0x4000, 0x4001)
    assert len(with_header) == 128 + 3
    info, content = disc.read_amsdos_header(with_header)
    assert content == b"\x01\x02\x03"
    assert info == {"type": "binary", "length": 3, "load_address": 0x4000, "execution_address": 0x4001}
    info, content = disc.read_amsdos_header(b"plain text")
    assert info is None and content == b"plain text"


# ---- basic.to_snapshot ------------------------------------------------------

def test_a_basic_program_in_a_snapshot():
    sna = basic.to_snapshot('10 PRINT "HI"\n')
    assert isinstance(sna, Snapshot)
    assert sna.memory_kb >= 64
