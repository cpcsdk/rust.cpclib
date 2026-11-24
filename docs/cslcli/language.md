# CSL Language Reference

CSL (CPC Script Language) scripts an emulator: it simulates user actions (reset, insert media,
type keys, wait) and takes screenshots and snapshots. The standard is written by Longshot /
Logon System ([CSL-STANDARD-EN.pdf](https://shaker.logonsystem.eu/Shaker_CSL/CSL-STANDARD-EN.pdf),
V1.6, November 2026). `cpclib-csl` implements it, plus one extension that the standard does not have:
the zoom of `screenshot`, which we number 1.6 too.

## Syntax

- One instruction per line; instruction names are lower case.
- A semicolon starts a comment: everything after it on the line is ignored.
- Text arguments (paths, key strokes) are in **single quotes**.
- `csl_version` is optional. When present, it must come before any other instruction (comments
  and blank lines may precede it). Without it, the latest version is assumed.
- An instruction newer than the declared version is an error:
  `Instruction 'wait_ssm' requires CSL version 1.5 or higher, but script uses version 1.4`.

```csl
; boot a disc and take a picture
csl_version 1.4
crtc_select 1
reset
wait 3000000
disk_insert 'SHAKER25.DSK'
key_output 'RUN"SHAKE25A"\(RET)'
wait 10000000
screenshot_name 'screen01'
screenshot
```

## Versions

| Version | Adds |
|---|---|
| 1.0 | `csl_version`, `reset`, `crtc_select`, media, key strokes, `wait*`, `screenshot*`, `snapshot*`, `csl_load` |
| 1.1 | `gate_array`, `cpc_model`, `memory_exp`, `rom_dir`, `rom_config` |
| 1.2 | `keyboard_write` |
| 1.4 | `screenshot_name`, `snapshot_name` |
| 1.5 | `wait_ssm` |
| 1.6 | `joystick` (from Krusty); a zoomed `screenshot` (a cpclib extension, not in the standard) |

## Miscellaneous

| Instruction | Meaning |
|---|---|
| `csl_version <major>.<minor>` | The version of the format the script is written for. |
| `reset [soft\|hard]` | Reset the emulator. `soft`: memory cleared by the ROM, only the 64K of central RAM. `hard` (default): a power off/on, every component. |

## Machine configuration

| Instruction | Meaning |
|---|---|
| `crtc_select <0\|1\|1A\|1B\|2\|3\|4>` | Select the CRTC model (`1A` and `1B` are `1`). An unknown CRTC stops the script. |
| `gate_array <40007\|40008\|40010>` | *(1.1)* Select the Gate Array. |
| `cpc_model <n>` | *(1.1)* `0`=464, `1`=664, `2`=6128, `4`=6128+, `5`=464+, `6`=GX4000. |
| `memory_exp <n>` | *(1.1)* `0`=128k (C4..C7), `1`=256k (C4..DF), `2`=256k (silicon E4-FF), `3`=4M, `4`=512k (DK'Tronics). |
| `rom_dir '<directory>'` | *(1.1)* Directory of the ROM files, used as a prefix. |
| `rom_config <type> <num> '<file>'` | *(1.1)* `type`: `U` upper ROM, `L` lower ROM, `C` cartridge, `M` Multiface 2. `num` is the ROM number (`0` for `L` and `C`). |

```csl
cpc_model 2
rom_config U 7 'Amsdos.rom'
rom_config C 0 'EerieForest.cpr'
```

## Media

| Instruction | Meaning |
|---|---|
| `disk_insert [A\|B] '<file>'` | Insert a disc image (drive A when omitted). |
| `disk_dir '<directory>'` | Directory of the disc images, used as a prefix. |
| `tape_insert '<file>'` | Insert a tape image. |
| `tape_dir '<directory>'` | Directory of the tapes, used as a prefix. |
| `tape_play`, `tape_stop`, `tape_rewind` | Play, stop, rewind the tape. |
| `snapshot_load '<file>'` | Load a snapshot. |
| `snapshot_dir '<directory>'` | Directory of the snapshots, used as a prefix. |

## Key strokes

| Instruction | Meaning |
|---|---|
| `key_delay <press> <between> [<after CR>]` | Delays in emulated µs: key held, wait between two keys, wait after a CR (the one between keys when omitted). Default delay: 19968 µs. |
| `key_output '<text>'` | Type the characters one by one. Unknown characters are skipped. |
| `key_from_file '<file>'` | Type the characters of an ASCII file. |
| `keyboard_write <row0>,...,<row9>` | *(1.2)* Set the 10 rows of the keyboard matrix; a bit at 0 is a key down. |
| `joystick <0\|1>[,up][,down][,left][,right][,fire1][,fire2][,fire3]` | *(1.6)* Hold these directions and buttons down on a joystick, everything not named released, until its next `joystick` line. The number alone releases it (centres the stick). |

In `key_output`, `\(CODE)` types a special key, and braces press keys together:

| Key | Code | Key | Code |
|---|---|---|---|
| ESC | `\(ESC)` | CLR | `\(CLR)` |
| TAB | `\(TAB)` | DEL | `\(DEL)` |
| CAPS LOCK | `\(CAP)` | RETURN | `\(RET)` |
| SHIFT | `\(SHI)` | ENTER | `\(ENT)` |
| CTRL | `\(CTR)` | cursors | `\(ARL)` `\(ARR)` `\(ARU)` `\(ARD)` |
| COPY | `\(COP)` | F0..F9 | `\(FN0)`..`\(FN9)` |
| `{` `}` `\` `'` | `\({)` `\(})` `\(\)` `\(')` | no delay after the next key | `\(KOF)` |

```csl
key_delay 500000 500000 1000000    ; 0.5 s between keys, 1 s after a CR
key_output 'RUN"SHAKE25A"\(RET)'
key_output '{\(SHI)1}'             ; SHIFT + 1
```

### Joystick matrix lines

`joystick 0` is line 9 and `joystick 1` line 6 of the keyboard matrix (shared with keys). A bit at 0
is held down: bit 0 up, 1 down, 2 left, 3 right, 4 fire 2, 5 fire 1, 6 fire 3.
`joystick 0,right,fire1` gives line 9 = `0xD7`. The words may come in any order, and each `joystick`
line replaces the whole state of that joystick. The standard writes the words after commas
(`Joystick 0,right,up,fire2`); `cpclib` accepts blanks too, and any case.

```csl
joystick 0,up           ; up
wait 200000
joystick 0,up,fire1     ; up and fire 1
wait 200000
joystick 0              ; released: the stick is centred
```

## Synchronization

| Instruction | Meaning |
|---|---|
| `wait <µs>` | Wait for emulated microseconds (not real time). |
| `wait_driveonoff [<n>]` | Wait for the drive motor to be started then stopped `n` times (1 when omitted). |
| `wait_vsyncoffon` | Wait for the VSYNC to go from off to on; if it is already on, wait for it to go off first. |
| `wait_ssm0000` | Wait for the SSM code `#0000` (`ED 00 ED 00`). |
| `wait_ssm <0xHHHH>` | *(1.5)* Wait for that SSM code, in hexadecimal prefixed by `0x`. |

## Exports

| Instruction | Meaning |
|---|---|
| `screenshot_name '<name>'` | *(1.4)* Name (no extension) of the next screenshot, taken by `screenshot` or by the SSM code `#FFFE` (`ED FE ED FF`). |
| `screenshot_dir '<directory>'` | Directory where screenshots are stored. |
| `screenshot [vsync] [<zoom>]` | Take a screenshot. With `vsync`, as soon as the VSYNC goes from inactive to active. |
| `snapshot_name '<name>'` | *(1.4)* Name (no extension) of the next snapshot, taken by `snapshot` or by the SSM code `#FFFF` (`ED FF ED FF`). |
| `snapshot [vsync]` | Take a snapshot, with `vsync` as for `screenshot`. |
| `snapshot_version <1\|2\|3>` | Version of the snapshots written (the emulator's own when not used). |
| `csl_load '<file>'` | Load and run another CSL file, to chain scripts. |

The zoom of `screenshot` *(a cpclib extension, 1.6 for cpclib)* is a factor (`0.5`) or a width in pixels (`440px`, the height in
proportion):

```csl
csl_version 1.6
screenshot 0.5
screenshot vsync 440px
```

## Errors

An emulator should stop a failing script and tell: the script's name, the line number, the
instruction, the reason, the script's version and the supported version. `cslcli` and the language
server report syntax errors with the offending line, and flag an instruction newer than the
declared `csl_version`.

## See Also

- [CSLCLI Command Reference](cmdline.md)
- [Examples](examples.md)
