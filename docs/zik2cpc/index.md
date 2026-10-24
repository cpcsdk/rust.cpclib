# zik2cpc

`zik2cpc` converts, packs and plays CPC music. It gathers in one command line tool the music
conversions that bndbuild pipelines (and the vscode extension) use.

Supported songs: Arkos Tracker (`.aks`), STarKos (`.sks`), Soundtrakker (`.128`),
Vortex Tracker II (`.vt2`), Wyz Tracker (`.wyz`), CHIPNSFX (`.chp`) and YM (`.ym`).
The external tools (Arkos Tracker 3 converters, chipnsfx, FAP, AYT, MinYMiser) are downloaded on demand.

`zik2cpc` is available as a standalone binary and as the `zik2cpc` bndbuild command.

## Commands

| Command | Purpose |
|---|---|
| `convert <INPUT> <OUTPUT> --to <FORMAT>` | convert a song (akg, akm, aky, aky-source, ym, ym3, vgm, wav, raw, events, sound-effects, chp-z80, fap, ayt, miny) |
| `build <INPUT> <OUTPUT> [--format dsk\|sna\|bin] [--player P]` | build a standalone player program |
| `play <INPUT> [--emulator E] [--player P]` | build a player and run it in an emulator |
| `compare <INPUT>` | build the song with every compatible player and compare sizes |
| `players <INPUT>` | list the players that can play the song |
| `info <INPUT>` | show title, author, composer and comment |

Players: `auto` (default, the song decides), `akg`, `akm`, `akys`, `akyu`, `chipnsfx`, `fap`, `ayt`, `miny`.

## Examples

```bash
zik2cpc convert song.aks song.akg --to akg
zik2cpc convert song.aks song.ym3 --to ym3
zik2cpc build song.aks song.dsk
zik2cpc build song.aks song.sna --format sna --player akm
zik2cpc compare song.aks
```

In a `build.bnd` file:

```yaml
- tgt: song.akg
  dep: song.aks
  cmd: zik2cpc convert song.aks song.akg --to akg
```
