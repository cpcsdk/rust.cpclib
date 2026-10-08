"""asm, basic, bdasm, cpr, image, analysis and build."""
import pytest

from cpclib_python import analysis, asm, basic, bdasm, image
from cpclib_python.bndbuild import Task
from cpclib_python.build import Build
from cpclib_python.cpr import Cartridge
from cpclib_python.sna import Snapshot


# ---- asm --------------------------------------------------------------------

def test_assemble_returns_bytes_and_symbols():
    built = asm.assemble("org #4000\nstart: db 1,2,3\n dw start", listing=True)
    assert built.data == b"\x01\x02\x03\x00\x40"
    assert built.symbols["start"] == 0x4000
    assert "start" in built.listing
    assert "Assembled" in built.messages


def test_assemble_takes_defines_and_case_insensitive():
    built = asm.assemble("org 0\n db FOO", defines={"FOO": 7})
    assert built.data == b"\x07"
    assert asm.assemble("org 0\nLabel: db 1\n dw label", case_insensitive=True).data == b"\x01\x00\x00"


def test_assemble_errors_carry_the_assemblers_message():
    with pytest.raises(RuntimeError, match="(?i)error"):
        asm.assemble("this is not z80")


def test_assemble_file_with_includes_and_a_snapshot(tmp_path):
    (tmp_path / "lib").mkdir()
    (tmp_path / "lib" / "data.asm").write_text("db 9,8,7\n")
    main = tmp_path / "main.asm"
    main.write_text("org #4000\n include \"data.asm\"\n")

    built = asm.assemble_file(str(main), include_dirs=[str(tmp_path / "lib")])
    assert built.data == b"\x09\x08\x07"

    sna = tmp_path / "main.sna"
    snapshot_build = asm.assemble_file(
        str(main), include_dirs=[str(tmp_path / "lib")], snapshot=str(sna)
    )
    assert snapshot_build.data is None
    assert Snapshot.load(str(sna)).read(0x4000, 3) == b"\x09\x08\x07"


def test_on_output_receives_what_the_assembler_says():
    seen = []
    asm.assemble("org 0\n db 1", on_output=lambda kind, text: seen.append((kind, text)))
    assert any(kind == "stdout" and "Assembled" in text for kind, text in seen)


# ---- bndbuild.Task ----------------------------------------------------------

def test_task_capture_and_callback():
    task = Task("basm", ["--inline", "org 0 : db 1"])
    captured = task.execute(capture=True)
    assert "Assembled" in captured["stdout"]
    events = []
    task.execute(on_output=lambda k, t: events.append(k))
    assert "stdout" in events


# ---- basic ------------------------------------------------------------------

def test_basic_round_trip():
    code = '10 PRINT "HELLO"\n20 GOTO 10\n'
    data = basic.tokenize(code)
    assert isinstance(data, bytes) and data
    text = basic.detokenize(data)
    assert "PRINT" in text and "GOTO" in text
    assert basic.line_numbers(code) == [10, 20]


def test_basic_errors_are_value_errors():
    with pytest.raises(ValueError):
        basic.tokenize("10 PRINT (")


# ---- bdasm ------------------------------------------------------------------

def test_disassemble():
    code = bdasm.disassemble(b"\x3e\x01\xc9", origin=0x4000)
    assert [i["address"] for i in code] == [0x4000, 0x4002]
    assert [i["bytes"] for i in code] == [b"\x3e\x01", b"\xc9"]
    assert code[0]["text"].upper().startswith("LD A")
    assert code[0]["nops"] == 2
    source = bdasm.disassemble_to_source(b"\x3e\x01\xc9", origin=0x4000)
    assert "org" in source and "RET" in source.upper()


def test_assembling_the_disassembly_gives_the_bytes_back():
    original = b"\x3e\x01\x06\x02\x80\xc9"
    again = asm.assemble(bdasm.disassemble_to_source(original, origin=0)).data
    assert again == original


# ---- cpr --------------------------------------------------------------------

def test_cartridge_banks(tmp_path):
    cpr = Cartridge()
    assert cpr.banks() == []
    cpr.set_bank(0, b"\x01\x02\x03")
    cpr.set_bank(2, b"\xff" * 0x4000)
    assert cpr.banks() == [0, 2]
    assert cpr.bank(0)[:4] == b"\x01\x02\x03\x00" and len(cpr.bank(0)) == 0x4000

    path = tmp_path / "game.cpr"
    cpr.save(str(path))
    again = Cartridge.load(str(path))
    assert again.banks() == [0, 2]
    assert again.bank(2) == b"\xff" * 0x4000
    assert Cartridge.from_bytes(cpr.to_bytes()).banks() == [0, 2]

    assert again.remove_bank(0) and not again.remove_bank(0)
    with pytest.raises(KeyError):
        again.bank(0)
    with pytest.raises(ValueError):
        again.set_bank(0, bytes(0x4001))


# ---- image ------------------------------------------------------------------

def test_render_screen_makes_a_png():
    rendered = image.render_screen(bytes(0x10000), mode=1)
    assert rendered["png"][:8] == b"\x89PNG\r\n\x1a\n"
    assert rendered["mode"] == 1
    with pytest.raises(ValueError, match="encoding"):
        image.render_screen(bytes(16), encoding="nope")
    with pytest.raises(ValueError, match="ink"):
        image.render_screen(bytes(16), palette=[99])


def test_screen_address_from_crtc_registers():
    assert image.screen_address(0x30, 0x00) == 0xC000


# ---- analysis ---------------------------------------------------------------

def test_check_tells_good_from_bad_source():
    assert analysis.check(code="ld a,1\n ret\n")["ok"] is True
    bad = analysis.check(code="ld a,\n")
    assert bad["ok"] is False and bad["diagnostics"]


def test_count_nops_of_a_range():
    summary = analysis.count_nops(1, 2, code="ld a,1\n inc a\n")
    assert summary["min_nops"] == 3 == summary["max_nops"]
    with pytest.raises(ValueError):
        analysis.count_nops(0, 1, code="nop")


SOURCE = "start:\n    ld b, b\n    inc hl\n    ret\n"


def test_suggest_optimizations_finds_a_peephole(tmp_path):
    src = tmp_path / "a.asm"
    src.write_text(SOURCE)
    found = analysis.suggest_optimizations(str(src))
    assert found["suggestion_count"] == 1
    assert found["suggestions"][0]["rule_name"] == "unnecessary-ld-to-itself"
    assert found["suggestions"][0]["line"] == 2
    with pytest.raises(ValueError, match="goal"):
        analysis.suggest_optimizations(str(src), goal="nope")


def test_apply_optimizations_rewrites_the_file(tmp_path):
    src = tmp_path / "a.asm"
    src.write_text(SOURCE)
    done = analysis.apply_optimizations_in_place(str(src))
    assert done["total_applied"] == 1
    assert "ld b, b" not in src.read_text()
    assert analysis.suggest_optimizations(str(src))["suggestion_count"] == 0


# ---- build ------------------------------------------------------------------

BUILD = """
- tgt: out.bin
  dep: in.asm
  cmd: basm in.asm -o out.bin

- tgt: all
  dep: out.bin
  phony: true
  cmd: echo done
"""


def test_a_build_lists_runs_and_skips(tmp_path):
    (tmp_path / "in.asm").write_text("org 0\n db 1,2,3\n")
    (tmp_path / "build.bnd").write_text(BUILD)

    build = Build(str(tmp_path))
    assert build.resolved_path.endswith("build.bnd")
    targets = {t["target"]: t["dependencies"] for t in build.targets()}
    assert targets["out.bin"] == ["in.asm"] and targets["all"] == ["out.bin"]
    assert build.default_target in targets
    assert build.is_outdated("out.bin")

    events = []
    result = build.run("all", on_output=lambda kind, text: events.append((kind, text)))
    assert result["target"] == "all"
    assert (tmp_path / "out.bin").read_bytes() == b"\x01\x02\x03"
    assert ("rule-start", "out.bin") in events
    assert not build.is_outdated("out.bin")

    events.clear()
    build.run("all", on_output=lambda kind, text: events.append((kind, text)))
    assert ("rule-skipped", "out.bin") in events

    with pytest.raises(KeyError):
        build.is_outdated("nope")


def test_a_failing_build_raises(tmp_path):
    (tmp_path / "build.bnd").write_text("- tgt: bad\n  cmd: basm missing.asm -o x.bin\n")
    with pytest.raises(RuntimeError):
        Build(str(tmp_path)).run("bad")
