"""The `music` submodule: songs to standalone CPC players, with any player.

The quick tests need nothing but the extension. The ones that really build a
player download the conversion tools on first use (and need `wine` on Linux
for CHIPNSFX, AYT and MinYMiser): they run only with CPCLIB_REAL_TOOLS=1.
"""
import os
from pathlib import Path

import pytest

from cpclib_python import music

BNDBUILD_TESTS = Path(__file__).resolve().parents[2] / "cpclib-bndbuild" / "tests"
YM = BNDBUILD_TESTS / "ay_players" / "ym" / "Targhan - Hocus Pocus.ym"
CHP = BNDBUILD_TESTS / "chipnsfx" / "WINGSOD5.CHP"

real_tools = pytest.mark.skipif(
    os.environ.get("CPCLIB_REAL_TOOLS") != "1",
    reason="downloads and runs the conversion tools: set CPCLIB_REAL_TOOLS=1",
)


def test_players_lists_every_choice_with_auto_first():
    names = music.players()
    assert names[0] == "auto"
    assert {"akg", "akm", "akys", "akyu", "chipnsfx", "fap", "ayt", "miny"} <= set(names)


def test_compatible_players_depend_on_the_kind_of_song():
    assert music.compatible_players(str(YM)) == ["fap", "ayt", "miny"]
    assert music.compatible_players(str(CHP)) == ["chipnsfx", "fap", "ayt", "miny"]
    assert music.compatible_players("tune.aks") == [
        "akg", "akm", "akys", "akyu", "fap", "ayt", "miny",
    ]


def test_song_info_of_a_ym_and_a_chp():
    ym = music.song_info(str(YM))
    assert ym["kind"] == "ym"
    assert ym["tracker"] is None
    assert (ym["title"], ym["author"]) == ("Hocus Pocus - Main", "Targhan")
    assert ym["uses_sid"] is False

    chp = music.song_info(str(CHP))
    assert chp["kind"] == "chipnsfx"
    assert chp["tracker"] == "CHIPNSFX"
    assert chp["title"] == "Wings of Death #5 1990 Thalion"


def test_an_unknown_player_is_a_value_error():
    with pytest.raises(ValueError, match="expected one of"):
        music.build(str(YM), "nope")


def test_a_missing_song_is_a_runtime_error():
    with pytest.raises(RuntimeError, match="does not exist"):
        music.build("/no/such/song.ym", "fap")


def test_a_player_cannot_play_every_song():
    with pytest.raises(RuntimeError, match="needs an Arkos Tracker song"):
        music.build(str(YM), "akg")


@real_tools
def test_build_reports_what_the_packer_says_and_writes_a_snapshot(tmp_path):
    sna = tmp_path / "hocus.sna"
    build = music.build(str(YM), "fap", snapshot=str(sna))
    assert build.player == "fap"
    assert build.load_address == 0x500
    assert build.song_bytes == 11924
    assert build.buffer_bytes == 3144
    assert build.play_nops == 712
    assert build.player_bytes > 100
    assert isinstance(build.program, bytes)
    assert build.program_bytes == len(build.program) > build.song_bytes
    assert build.snapshot == str(sna) and sna.is_file()


@real_tools
def test_build_dsk_writes_a_disc(tmp_path):
    dsk = tmp_path / "hocus.dsk"
    assert music.build_dsk(str(YM), str(dsk), "miny") == str(dsk)
    assert dsk.stat().st_size > 0


@real_tools
def test_compare_measures_every_compatible_player():
    comparisons = music.compare(str(CHP))
    assert [c.player for c in comparisons] == ["chipnsfx", "fap", "ayt", "miny"]
    for c in comparisons:
        assert c.error is None, (c.player, c.error)
        assert c.build.program_bytes > 0
    chipnsfx = comparisons[0].build
    assert chipnsfx.song_bytes is None  # its song is Z80 source

    table = music.compare_table(str(YM))
    assert table.splitlines()[0].split() == ["player", "song", "player", "code", "program"]
    assert {"fap", "ayt", "miny"} <= {line.split()[0] for line in table.splitlines()[1:4]}
