# Command line arguments

Here is the help provided by `basm`.

```text
Profile release compiled: Tue, 6 Jan 2026 19:53:38 +0000

Benediction ASM -- z80 assembler that mainly targets Amstrad CPC

Usage: basm [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file to read.

Options:
      --inline <INLINE>                Z80 code is provided inline
      --db                             Write a db list on screen (usefull to get the value of an opcode)
      --lst <LISTING_OUTPUT>           Filename of the listing output.
      --remu <REMU_OUTPUT>             Filename to store the remu file used by Ace to import label and debug information
      --wabp <WABP_OUTPUT>             Filename to stare the WABP file use to provide Winape breakpoints
      --breakpoint-as-opcode           Breakpoints are stored as opcodes (mainly interesting for winape emulation)
      --sym <SYMBOLS_OUTPUT>           Filename of the output symbols file.
      --sym_kind <SYMBOLS_KIND>        Format of the output symbols file [default: basm] [possible values: winape, basm]
  -o, --output <OUTPUT>                Filename of the output.
      --basic                          Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive).
      --binary                         Request a binary header
      --cartridge                      Generate a CPR
      --snapshot                       Generate a snapshot
      --nochunk <CODE>                 Deactivate some snapshot chunks (comma separated) [possible values: BRKC, BRKS, REMU, SYMB]
  -i, --case-insensitive               Configure the assembler to be case insensitive.
      --disable-warnings               Do not generate warnings
  -d, --directives-prefixed-by-dot     Expect directives to by prefixed with a dot
  -I, --include <INCLUDE_DIRECTORIES>  Provide additional directories used to search files
  -D, --define <DEFINE_SYMBOL>         Provide a symbol with its value (default set to 1)
      --no-forced-void                 By default (void) is mandatory on macro without parameters. This option disable this behavior
      --debug                          Trace more information to help debug
      --override                       Override file when already stored in a disc
      --backup                         Backup an existing file when saved on disc
      --orgams                         Main source is at ORGAMS format
      --m4 <TO_M4>                     Provide the IP address of the M4
  -l <LOAD_SYMBOLS>                    Load symbols from the given file
      --Werror                         Warning are considered to be errors
      --progress                       Show a progress bar.
      --list-embedded                  List the embedded files
      --view-embedded <VIEW_EMBEDDED>  Display one specific embedded file [possible values: inner://crtc.asm, inner://deexo.asm, inner://deshrink.asm, inner://dzx0_fast.asm, inner://dzx0_standard.asm, inner://firmware/amsdos.asm, inner://firmware/casmng.asm, inner://firmware/gfxvdu.asm, inner://firmware/highkern.asm, inner://firmware/indirect.asm, inner://firmware/kernel.asm, inner://firmware/keymng.asm, inner://firmware/lowkern.asm, inner://firmware/machine.asm, inner://firmware/math6128.asm, inner://firmware/mathnot464.asm, inner://firmware/mathnot6xx.asm, inner://firmware/not464.asm, inner://firmware/scrpack.asm, inner://firmware/sound.asm, inner://firmware/txtvdu.asm, inner://ga.asm, inner://lz48decrunch.asm, inner://lz49decrunch.asm, inner://lz4_docent.asm, inner://opcodes_first_byte.asm, inner://pixels-routs.asm, inner://unaplib.asm, inner://unaplib_fast.asm, inner://uncrunch/dzx0_mega_back.asm, inner://uncrunch/dzx0_standard_back.asm, inner://uncrunch/dzx0_turbo_back.asm, inner://uncrunch/dzx7_turbo.asm, inner://uncrunch/upkr.asm, inner://unlzsa1_fast.asm, inner://unlzsa1_small.asm, inner://unlzsa2_fast.asm, inner://unlzsa2_small.asm]
  -h, --help                           Print help
  -V, --version                        Print version

Still a Work In Progress assembler
```
