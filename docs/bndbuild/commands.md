# Bndbuild Commands

The `--direct -- COMMAND [ARG...]` allows to directly launch a command without managing a build file. `COMMAND` can be any command accepted in `cmd` key (they are listed in the documentation of `--help`).

## Command Types

Commands available in bndbuild fall into three categories:

1. **Embedded-only commands** - Built into bndbuild for convenience (e.g., `echo`, `cp`, `rm`, `mv`, `mkdir`, `extern`)
2. **Standalone cpclib tools** - Available both as standalone binaries and embedded in bndbuild (e.g., `basm`, `bdasm`, `img2cpc`, `catalog`, `snapshot`, `fade`, etc.). These tools can be run independently outside of bndbuild.
3. **External tools** - Third-party programs that bndbuild can download and integrate (e.g., `rasm`, `martine`, `winape`, etc.)

Most cpclib tools documented here exist as **standalone command-line programs** that can be used independently. See their individual documentation pages for standalone usage.

Several commands need to be downloaded (so internet is required), assembled (so their prerequisites need to be installed).
There is no (yet) cleanup if download/compilation fail. So think to do `bndbuild --clear <cmd>` to cleanup manually.

## Display

### Display management: echo (echo,print)
```
Print the arguments.

Usage: echo [arguments]...

Arguments:
  [arguments]...
          Words to print
```

## External Programs

### External program management: extern (extern)

```
Launch an external command.

Usage: extern <program> [arguments]...

Arguments:
  <program>
          The program to execute
  [arguments]...
          The arguments of the program
```

## File Management

### File management: cp (cp,copy)

```
Copy files.


Usage: cp [arguments]...

Arguments:
  [arguments]...
          Files to copy. Last one being the destination

Inner command of cpclib-bndbuild 0.6.0
```

### File management: rm (rm, del)

```
Delete files.


Usage: rm [arguments]...

Arguments:
  [arguments]...
          Files to delete.

Inner command of cpclib-bndbuild 0.6.0
```

### File management: mv (mv,move,rename)

```
Rename files.

Usage: mv [arguments]...

Arguments:
  [arguments]...
          Files to move. With 2 files, first one is renamed as second one. With more than 2 files, last one is the destination directory.

Inner command of cpclib-bndbuild 0.11.0
```

### File management: mkdir (mkdir)

```
Create directories.

Usage: mkdir [OPTIONS] [arguments]...

Arguments:
  [arguments]...
          Folders to create.

Options:
  -p, --parents  Set to specify if missing parent directories must be created
  -i, --ignore   Set to specify we ignore existing folders

Inner command of cpclib-bndbuild 0.11.0
```

### File management: Amsdos header management (hideur)

**Standalone:** Available as `hideur` binary. For complete documentation, see [Hideur Documentation](../../hideur).

### File management: Orgams to text conversion (borgams)

**Standalone:** Available as `borgams` binary. For complete documentation, see [Borgams Documentation](../../borgams).

### File management: SNA/Snapshot management (sna,snpashot)

**Standalone:** Available as `snapshot` binary. Create and manipulate snapshot files. For complete documentation, see [Snapshot Documentation](../../snapshot).

## Image Processing

### Image management: Benediction transfer tool (img2cpc,imgconverter)

**Standalone:** Available as `img2cpc` binary. For complete documentation, see [img2cpc Documentation](../../img2cpc).

### Image management: CPC to image (cpc2img)

**Standalone:** Available as `cpc2img` binary. For complete documentation, see [CPC2IMG Documentation](../../cpc2img).

### Image management: Impact transfer tool (martine)

For complete documentation and download, see <https://github.com/jeromelesaux/martine>.

### Image management: Grafx2 (grafx2,grafx)

Advanced graphics editor. For complete documentation, see <https://gitlab.com/GrafX2/grafX2>.

### Image management: Conversion (convgeneric)

Generic file format conversion utility. For complete documentation and download, see <https://github.com/EdouardBERGE/convgeneric>.

### Image management: HSP Compiler (hspcompiler,hspc)

HxC Sound Player compiler for CPC music. For complete documentation, see <https://github.com/EdouardBERGE/hspcompiler>.

### Image management: Fade (fade)

**Standalone:** Available as `fade` binary. For complete documentation, see [Fade Documentation](../../fade).

## Disc Management

### Disc management: Benediction dsk manager (dsk,disc)

Embedded-only command for low-level DSK disc image manipulation (format, add files, catalog operations).

### Disc management: Catalog listing (catalog,cat)

**Standalone:** Available as `catalog` binary. List contents of DSK files. For complete documentation, see [Catalog Documentation](../../catalog).

### Disc management: Impact dsk manager (impdsk,impdisc)

For complete documentation and download, see <https://github.com/jeromelesaux/dsk>.

## Cartridge Management

### Cartridge management: CPR analysis (cpr)

**Standalone:** Available as `cpr` binary. Analyze and compare CPR cartridge files. For complete documentation, see [CPR CLI Documentation](../../cprcli).

## Assemblers

### Assembler: BASM (basm,assemble)

**Standalone:** Available as `basm` binary. For complete documentation, see [BASM Documentation](../../basm).

### Assembler: RASM (rasm)

For complete documentation, see <http://rasm.wikidot.com/>.

### Assembler: Orgams (orgams)

**Standalone:** Available as `orgams` binary. For complete documentation, see [Orgams Documentation](../../orgams).

Orgams is a native assembler. So, an emulator is used to assemble source.
WARNING: it currently does not work properly under windows.

### Assembler: Sjasmplus (sjasmplus)

For complete documentation, see <https://z00m128.github.io/sjasmplus/documentation.html>.

### Assembler: UZ80AS (uz80)

Universal Z80 assembler. For complete documentation, see <http://cngsoft.no-ip.org/uz80.htm>.

### Assembler: Vasm z80 oldstyle (vasm)

<http://sun.hasenbraten.de/vasm/release/vasm_6.html>

## Disassemblers

### Disassembler: BDASM (bdasm,dz80)

**Standalone:** Available as `bdasm` binary. Z80 disassembler. For complete documentation, see [BDASM Documentation](../../bdasm).

### Disassembler: DISARK (disark)

Arkos Tracker disassembler utility. For complete documentation, see <https://julien-nevo.com/disark/>.

## Emulators

### Emulator: Emulator-agnostic emulation (cpc,emu,emuctrl,emucontrol)

**Standalone:** Available as `cpcrunner` binary. For complete documentation, see [CPC Runner Documentation](../../runner).

### Emulator: ACE DL (ace,acedl)

ACE emulator with direct loading support. For complete documentation, see <https://roudoudou.com/ACE-DL/>.

### Emulator: AMSpiriT (amspirit)

For complete documentation, see <https://www.amspirit.fr/>.

### Emulator: Caprice Forever (caprice)

Classic CPC emulator. For more information, see <http://www.caprice32.org/>.

### Emulator: CPC Emu Power (cpcemupower)

Advanced CPC emulator with debugging features. For complete documentation, see <https://www.cpc-power.com/cpcarchives/index.php?page=articles&num=446>.

### Emulator: CPCEC (cpcec)

For complete documentation, see <http://cngsoft.no-ip.org/cpcec.htm>.

### Emulator: SugarboxV2 (sugarbox)

For complete documentation and download, see <https://github.com/Tom1975/SugarboxV2>.

### Emulator: Winape (winape)

For complete documentation, see <https://www.winape.net/help/>.

## Hardware Transfer

### Transfer: M4 support (xfer,cpcwifi,m4)

**Standalone:** Available as `cpclib-xfertool` binary. For complete documentation, see [XferTool Documentation](../../xfertool).

## Build Management

### Build: BndBuild (bndbuild,build)

Recursive bndbuild invocation for nested build configurations. For complete documentation, see [BndBuild Documentation](../../bndbuild).

### Build: BASIC/Locomotive (locomotive,basic)

**Standalone:** Library-only (no standalone binary). BASIC tokenizer and manager for Locomotive BASIC programs. For complete documentation, see [Locomotive Documentation](../../locomotive).

## Data Compression

### Compression: Crunch (crunch,compress)

**Standalone:** Available as `crunch` binary. Data compression utility for Z80 programs. For complete documentation, see [Crunch Documentation](../../crunch).

## CDT Tools

### CDT: RTZX (rtzx)

Real-time ZX data compression. For complete documentation, see <https://github.com/LichP/rtzx>.

## Audio Tools

### Audio: Arkos Tracker 3 (at3,ArkosTracker3)

Music tracker for CPC sound chip programming. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: AY Test (ayt)

AY-3-8912 sound chip test utility. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Chip and SFX (chipnsfx)

Sound effects and chip music utility. For complete documentation, see <http://cngsoft.no-ip.org/chipnsfx.htm>.

### Audio: Minimiser (miny)

Music data minimization tool. For complete documentation, see <https://github.com/tattlemuss/minymiser>.

### Audio: Song to AKG (SongToAkg)

Convert song data to AKG format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to AKM (SongToAkm)

Convert song data to AKM format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to AKY (SongToAky)

Convert song data to AKY format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to Events (SongToEvents)

Convert song data to event stream format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to Raw (SongToRaw)

Convert song data to raw binary format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to Sound Effects (SongToSoundEffects)

Convert song data to sound effects format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to VGM (SongToVgm)

Convert song data to Video Game Music format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to WAV (SongToWav)

Convert song data to WAV audio format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

### Audio: Song to YM (SongToYm)

Convert song data to YM audio format. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.

## Development Tools

### Development: BASMDOC (basmdoc,doc)

**Standalone:** Available as `basmdoc` binary. Generate documentation from BASM source code. For complete documentation, see [BASMDOC Documentation](../../basmdoc).

### Development: Z80 Profiler (Z80Profiler)

Z80 code profiler for performance analysis. For complete documentation, see <https://www.julien-nevo.com/arkostracker/>.
