#!/usr/bin/env python3
"""Compare what a song weighs with every player that can play it.

    python music_compare.py tune.aks [more songs...]

A song is turned into a standalone CPC player by each of the players bndbuild
knows (Arkos Tracker AKG/AKM/AKY, CHIPNSFX, FAP, AYT, MinYMiser); songs of
another tracker are converted to the player's format first.
"""
import sys

from cpclib_python import music


def main(songs):
    for song in songs:
        info = music.song_info(song)
        print(f"{song}: {info['title'] or '?'} ({info['tracker'] or info['kind']})")

        rows = []
        for c in music.compare(song):
            if c.error:
                print(f"  {c.player}: failed ({c.error.splitlines()[0]})")
                continue
            b = c.build
            rows.append((b.program_bytes, c.player, b))

        for program, player, b in sorted(rows):
            print(
                f"  {player:9} program={program:6}  song={b.song_bytes!s:>6}"
                f"  player={b.player_bytes!s:>5}  buffer={b.buffer_bytes!s:>5}"
                f"  nops={b.play_nops!s:>5}"
            )


if __name__ == "__main__":
    main(sys.argv[1:] or sys.exit(__doc__))
