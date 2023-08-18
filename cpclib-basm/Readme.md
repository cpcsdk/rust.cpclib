# BASM

## Synopsis

Benediction ASseMbler is a modern z80 assembler dedicated to Amstrad CPC plateform.
It has been developed and used mainly for the demo [CRTC](https://www.pouet.net/prod.php?which=88554).

No true documentation has been written yet... But some little parts can be found [here](../cpclib-asm/basm_doc.md)
What can be retained is that:
- It's parser is definitevely slower than rasm ones, but speed remains still competitive
- It contains all standard assembler directives (file inclusion, conditional assembling, variables, macros definition, loops, local labels)
- As well as uncommon ones (functions definition, list and matrices handling, embedding of file by the assembler)

Of course, there are still bugs due to the limited number of users ;) but it worked propelrly with my use cases


## Help

```
basm.exe --help`
Profile debug compiled: Fri, 18 Aug 2023 14:05:52 +0000

Benediction ASM -- z80 assembler that mainly targets Amstrad CPC

Usage: basm.exe [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file to read.

Options:
      --inline <INLINE>                Z80 code is provided inline
  -o, --output <OUTPUT>                Filename of the output.
      --db                             Write a db list on screen (usefull to get the value of an opcode)    
      --lst <LISTING_OUTPUT>           Filename of the listing output.
      --sym <SYMBOLS_OUTPUT>           Filename of the output symbols file.
      --sym_kind <SYMBOLS_KIND>        Format of the output symbols file [default: basm] [possible values: winape, basm]
      --basic                          Request a Basic header (the very first instruction has to be the LOCOMOTIVE directive).
      --binary                         Request a binary header
      --snapshot                       Generate a snapshot
  -i, --case-insensitive               Configure the assembler to be case insensitive.
  -d, --directives-prefixed-by-dot     Expect directives to by prefixed with a dot
  -I, --include <INCLUDE_DIRECTORIES>  Provide additional directories used to search files
  -D, --define <DEFINE_SYMBOL>         Provide a symbol with its value (default set to 1)
      --m4 <TO_M4>                     Provide the IP address of the M4
  -l <LOAD_SYMBOLS>                    Load symbols from the given file
      --Werror                         Warning are considered to be errors
      --progress                       Show a progress bar.
      --list-embedded                  List the embedded files
      --view-embedded <VIEW_EMBEDDED>  Display one specific embedded file [possible values: inner://crtc.asm, inner://deexo.asm, inner://dzx0_fast.asm, inner://dzx0_standard.asm, inner://firmware/amsdos.asm, inner://firmware/casmng.asm, inner://firmware/gfxvdu.asm, inner://firmware/highkern.asm, inner://firmware/indirect.asm, inner://firmware/kernel.asm, inner://firmware/keymng.asm, inner://firmware/lowkern.asm, inner://firmware/machine.asm, inner://firmware/math6128.asm, inner://firmware/mathnot464.asm, inner://firmware/mathnot6xx.asm, inner://firmware/not464.asm, inner://firmware/scrpack.asm, inner://firmware/sound.asm, inner://firmware/txtvdu.asm, inner://ga.asm, inner://lz48decrunch.asm, inner://lz49decrunch.asm, inner://lz4_docent.asm, inner://opcodes_first_byte.asm, inner://pixels-routs.asm, inner://unaplib.asm, inner://unaplib_fast.asm]
  -h, --help                           Print help
  -V, --version                        Print version

Still a Work In Progress assembler
```