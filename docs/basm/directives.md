

# Directives

!!! failure Inacurate documentation

    Most content is IA generated and not yet proofreaded. However all examples are unit-tested


## Listing related

### LIST, NOLIST

Synopsis:

```
LIST
NOLIST
```

Description:
Control assembly listing output. LIST enables listing generation, NOLIST disables it. Useful for hiding macro expansions or repetitive code from listings.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_list.asm"
```


## Memory related

### ALIGN

Synopsis:

```
ALIGN boundary [, FILL]
```

Description:
Align the assembly address to the specified boundary. Pads with zeros until the address is a multiple of the boundary value.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_align.asm"
```

### EVEN

Synopsis:

```
EVEN
```

Description:
Align to an even address. Shorthand for ALIGN 2. Ensures the next byte is at an even memory address.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_even.asm"
```


### CONFINED

Synopsis:

```
CONFINED
  ... code/data ...
ENDCONFINED
```

Description:
Confine a memory area of 256 bytes maximum in such a way that it is always possible to navigate in the data by only modifying the low byte address (*i.e* INC L always works).

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_confined.asm"
```


### ORG

Synopsis:

```
ORG address
```

Description:
Set the assembly address. Code will be placed at this address in memory.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_org.asm"
```

### LIMIT

Synopsis:

```
LIMIT address
```

Description:
Set an upper limit for the assembly address. Assembly will fail if code exceeds this address. Works on the code space ($), not physical space ($$).

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_limit.asm"
```

### PHASE, DEPHASE

Synopsis:

```
PHASE address
  ... code ...
DEPHASE
```

Description:
Assemble code as if it were at a different address (relocation). Code is written to current $ but assembled as if at the PHASE address.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_phase.asm"
```

### RORG

Synopsis:

```
RORG address
...
REND
```

Description:
Relocatable ORG. Similar to PHASE but for relocatable code. Sets both physical and logical address. **RORG must be closed with REND to return to normal addressing mode.**

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_rorg.asm"
```

### PROTECT

Synopsis:
```
PROTECT START, STOP
```
Description:
Mark the memory between START and STOP as protected against write. Assembler fails when writting there.

On the code space ($), not physical space ($$)

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_protect.asm"
```


### RANGE/DEFSECTION

Synopsis:

```
RANGE start, stop, name
DEFSECTION start, stop, name    ; alias for RANGE
```

Description:
RANGE (and its alias DEFSECTION) allows to define named portions of the memory. Takes start address, stop address, and section name.

Example with DEFSECTION:
```z80
--8<-- "cpclib-basm/tests/asm/good_document_defsection.asm"
```

### SECTION

Synopsis:

```
SECTION name
```

Description:
SECTION allows to choose which portion of memory to use. The section must have been previously defined with RANGE or DEFSECTION.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_section.asm"
```

### BANK

Description:

When used with no argument, a bank corresponds to a memory area outside of the snapshot. All things read&write in memory are thus uncorrelated to the snapshot.
Sections do not apply in a bank.

`BANK page` is similar to `WRITE DIRECT -1 -1 page`


Synopsis:

```
BANK [EXPRESSION]
```

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_bank.asm"
```

#### BANKSET

Synopsis:

```
BANKSET EXPRESSION
```

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_bankset.asm"
```

#### WRITE DIRECT

Description:
WRITE DIRECT is a directive from Winape that we have not fully reproduced. It's two first arguments need to be -1.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_write_direct.asm"
```

## Labels related

### =, SET

Description:

Assign an expression to a label. Assignement can be repeated  several times.

Synopsis:
```
LABEL = EXPRESSION
```

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_assign.asm"
```


### EQU

Description:
Assign an expression to a label. Assignement cannot be repeated  several times.


Synopsis:
```
LABEL = EXPRESSION
```

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_assign.asm"
```

### LET

Synopsis:

```
LET label = expression
```

Description:
Explicit variable assignment. Equivalent to `label = expression` but with explicit LET keyword (BASIC-style).

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_let.asm"
```

### MAP

Synopsis:

```
MAP VALUE
label #increment
```

Description:
`MAP VALUE` defines a map counter to the required value. `#` is used to assign the value to a given label and increment it of the appropriate amount.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_map.asm"
```

### SETN, NEXT

Synopsis:

```
SETN value
label NEXT increment
```

Description:
Legacy directive for sequential label assignment. `MAP` directive is probably easier to use.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_next.asm"
```

### UNDEF

Synopsis:

```
UNDEF label
```

Description:
Undefine a previously defined label, allowing it to be redefined.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_undef.asm"
```

## Data related

### BYTE, TEXT, DB, DEFB, DM, DEFM

Synopsis:

```
DB|DEFB|BYTE|TEXT expression [, expression]*
DM|DEFM string [, string]*
```

Description:
Define byte(s) in memory. Can accept integers, characters, strings, or expressions.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_db.asm"
```

### WORD, DW, DEFW

Synopsis:

```
DW|DEFW|WORD expression [, expression]*
```

Description:
Define 16-bit word(s) in memory (little-endian format).

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_word.asm"
```

### DS, DEFS, FILL, RMEM

Synopsis:

```
DS|DEFS|FILL|RMEM size [, value]
```

Description:
Reserve space in memory. If value is specified, fills the space with that value.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_defs.asm"
```

### ABYTE

Synopsis:

```
ABYTE expression
```

Description:
Define a byte and align it at the specified address or boundary.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_abyte.asm"
```

### STR

Description:
STR encodes string in AMSDOS format (i.e., adds 0x80 to the last char) and stores its bytes in memory.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_str.asm"
```

### CHARSET

Synopsis:

```
CHARSET "filename"
```

Description:
Load a character set definition from a file for use in subsequent character encoding.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_charset.asm"
```

### STARTINGINDEX

Synopsis:

```
STARTINGINDEX value
```

Description:
Set the starting index for character encoding when using custom charsets.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_startingindex.asm"
```

## Module and Namespace Directives

### MODULE, ENDMODULE

Synopsis:

```
MODULE name
  ... code ...
ENDMODULE
```

Description:
Define a namespace module. Labels inside are prefixed with the module name.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_module.asm"
```

## Conditional directives

### IF, ELSE, ENDIF

Synopsis:

```
IF condition
  ... code if true ...
[ELSE
  ... code if false ...]
ENDIF
```

Description:
Conditional assembly based on expression evaluation at assembly time.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_if.asm"
```

### IFNOT

Synopsis:

```
IFNOT condition
  ... code if false ...
ENDIF
```

Description:
Opposite of IF - executes block if condition is false.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ifnot.asm"
```

### IFDEF, IFNDEF

Synopsis:

```
IFDEF label
  ... code if defined ...
ENDIF

IFNDEF label
  ... code if not defined ...
ENDIF
```

Description:
Check if a label has ALREADY been defined before reading this directive.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ifdef.asm"
```

### IFUSED/IFEXIST

Synopsis:

```
IFUSED label
IFEXIST label    ; alias for IFUSED
  ... code if label is used ...
ENDIF
```

Description:
Check if a label is referenced anywhere in the code. IFEXIST is an alias for IFUSED. Useful for conditional inclusion of code.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_ifused.asm"
```

### IFNUSED

Synopsis:

```
IFNUSED label
  ... code if label is NOT used ...
ENDIF
```

Description:
Opposite of IFUSED. Executes block if label is never referenced.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ifnused.asm"
```

### ELSEIF, ELSEIFDEF, ELSEIFNDEF, ELSEIFNOT, ELSEIFUSED/ELSEIFEXIST

Synopsis:

```
IF condition1
  ... code ...
ELSE IF condition2        ; or ELSEIF
  ... code ...
ELSE IFDEF label          ; or ELSEIFDEF
  ... code ...
ELSE IFNDEF label         ; or ELSEIFNDEF
  ... code ...
ELSE IFUSED label         ; or ELSEIFUSED
  ... code ...
ELSE IFNOT condition      ; or ELSEIFNOT
  ... code ...
ELSE
  ... code ...
ENDIF
```

Description:
Chained conditional directives. Allow multiple conditions without nesting. Can be written as two words (ELSE IF) or one word (ELSEIF). Available variants:

- **ELSEIF** - else + if combined
- **ELSEIFDEF** - else + ifdef combined  
- **ELSEIFNDEF** - else + ifndef combined
- **ELSEIFNOT** - else + ifnot combined
- **ELSEIFUSED/ELSEIFEXIST** - else + ifused combined (ELSEIFEXIST is an alias)

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_elseif.asm"
```

### Nested conditions

Conditions can be nested.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_if.asm"
```

### BREAKPOINT

Synopsis:

```
BREAKPOINT [expression]
```

Description:
Insert a breakpoint in the generated snapshot for debugging. Optional expression can specify a condition.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_breakpoint.asm"
```

### FAIL

Synopsis:

```
FAIL [message]
```

Description:
Force assembly to fail with an optional error message. Useful for compile-time validation.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_fail.asm"
```

### WARNING

Synopsis:

```
WARNING [message [, expressions...]]
```

Description:
Emit a warning message without stopping assembly. Unlike FAIL, assembly continues after a WARNING. Useful for highlighting potential issues or deprecated features while still producing output.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_warning.asm"
```

### STOP, END

Synopsis:

```
STOP
END
```

Description:
Stop assembly processing at this point. Useful for conditional assembly or debugging.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_stop.asm"
```


### SWITCH, ENDSWITCH

Synopsis:

```
SWITCH expression
CASE value1
  ... code for value1 ...
  [BREAK]
CASE value2
  ... code for value2 ...
  [BREAK]
DEFAULT
  ... code if no match ...
ENDSWITCH
```

Description:
Multi-way conditional based on expression value. CASE defines match values, DEFAULT handles unmatched cases. BREAK exits the switch early.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_switch.asm"
```

## Code duplication directives

### FOR

Synopsis:

```
FOR <variable>, EXPRESSION [, EXPRESSION]*
  ... LISTING ...
ENDFOR|FEND
```

Description:
Iterate over a list of values, executing the block for each value. The variable takes on each value in the list.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_for.asm"
```


### WHILE

Synopsis:

```
WHILE condition
  ... LISTING ...
ENDWHILE|WEND
```

Description:
Repeat a block of code while the condition is true. Condition is evaluated before each iteration.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_while.asm"
```

### REPEAT, REP, REPT

Synopsis:

```
REPEAT count [, counter [, start]]
  ... inner listing ...
REND|ENDR|ENDREPEAT
```

Description:
Repeat a block of code a fixed number of times. Optional counter variable tracks iteration (0-based by default, or from start value).

Aliases: REP, REPT (same as REPEAT)

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_repeat_incbin2.asm"
```

### ITERATE

Synopsis:

```
ITERATE COUNTER, EXPR [, EXPR]*
  ... INNER LISTING ...
IEND|ENDITERATE
```

Description:
Iterate over expressions, evaluating each expression after having generated the code of the previous one. Take that into account if expressions use $.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_iterate.asm"
```

## Code and data generation directives

### MACRO

Synopsis:

```
MACRO name [param1, param2, ...]
  ... macro body ...
ENDMACRO|ENDM

; Call macro
name [arg1, arg2, ...]
```

Description:
Define a reusable block of code that can be called with parameters. Macros are expanded inline at each call site.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_macro.asm"
```


### STRUCT

Synopsis:

```
STRUCT <name>
  <field1> DB|DW|STR|<other struct> [<value>]
  ...
  <fieldn> DB|DW|<other struct> [<value>]
ENDSTRUCT

; Create instance
[<label>:] <name> <arg1>, ... , <argn>
```

Description:
Structures allow to define data blocks with semantics. In practice, they replace bunches of `DEFB`, `DEFW` directives and enforce checks at assembling (you cannot add more data than expected or forget some). If a label is used before the use of a struct, it is necessary to postfix it by `:`. Otherwise the assembler thinks the label is a macro or structure call.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_struct.asm"
```

## Data loading and transformation directives

Filenames are stored in a string.
These string can do expansion of formulas embedded in {}.

basm embeds some files in its executable, they are access under the name "inner://" :

### LZAPU, LZ4, LZ48, LZ49, LZEXO, LZSA1, LZSA2, LZUPKR, LZSHRINKLER, LZX0, LZX0_BACKWARD, LZX7, INCSHRINKLER, INCUPKR, INCZX0, INCZX0_BACKWARD

Synopsis (as block directives):

```
LZ4|LZ48|LZ49|LZAPU|LZEXO|LZX0|LZX7|LZSHRINKLER
  ... data to crunch ...
LZCLOSE
```

Synopsis (as include directives):

```
INCAPU|INCLZ4|INCL48|INCL49|INCEXO|INCLZSA1|INCLZSA2|INCUPKR|INCSHRINKLER|INCZX0 "filename" [[, SKIP], AMOUNT]
INCZX0_BACKWARD "filename" [[, SKIP], AMOUNT]
```

Description:
Crunch (compress) data using various compression algorithms. Can be used as block directives (crunch inline data) or include directives (load and crunch file).

Supported crunchers:

- **LZ4** - LZ4 compression
- **LZ48** - LZ4 variant optimized for Z80
- **LZ49** - Another LZ4 variant
- **LZAPU** / **INCAPU** - Aplib compression
- **LZEXO** / **INCEXO** - Exomizer compression  
- **LZSA1** / **INCLZSA1** - LZSA1 compression
- **LZSA2** / **INCLZSA2** - LZSA2 compression
- **LZUPKR** / **INCUPKR** - Upkr compression
- **LZSHRINKLER** / **INCSHRINKLER** - Shrinkler compression
- **LZX0** / **INCZX0** - ZX0 compression (forward)
- **LZX0_BACKWARD** / **INCZX0_BACKWARD** - ZX0 compression (backward)
- **LZX7** - ZX7 compression

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_aplib_decrunch.asm"
```

`basm` automatically replaces the content of some automatic variables after crunching data. They can help in the uncrunch process for some uncrunchers (mainly the backward ones):

- `BASM_LATEST_CRUNCH_INPUT_DATA_SIZE` contains the size of the data BEFORE crunching.
- `BASM_LATEST_CRUNCH_OUTPUT_DATA_SIZE` contains the size of the data AFTER crunching.
- `BASM_LATEST_CRUNCH_DELTA` contains the delta value of the compressor. -1 if does not exist





### INCBIN, BINCLUDE

Synopsis:

```
INCBIN|BINCLUDE "fname" [[, SKIP], AMOUNT]
```

Description:
Include binary file content. Fname can be built with variables. File is loaded fully in memory before being sliced depending on arguments.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_incbin.asm"
```


### INCLUDE, READ

Synopsis:

```
INCLUDE|READ [ONCE] "<fname>" [AS|MODULE|NAMESPACE "<module>"]
```

Description:
Include another source file. Fname can be built with variables. Files prefixed by `inner://` are embedded by `BASM`. In case of conditional assembling, inclusion are only done in the executed branch.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_include.asm"
```

## Data saving and export

### OUTPUT

Synopsis:

```
OUTPUT "filename"
```

Description:
Set the output filename for the assembled code. This directive allows you to specify where the assembled binary should be written.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_output.asm"
```

### EXPORT, NOEXPORT

Synopsis:

```
EXPORT [label]
NOEXPORT
```

Description:
Control which symbols are exported to external files. EXPORT makes labels visible externally, NOEXPORT hides them.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_export.asm"
```


### SAVE, WRITE

Synopsis:

```
SAVE "<fname>", [[[START], [SIZE]], AMSDOS|BASIC|TAPE]
SAVE "<fname>", START, SIZE, DSK, "<fname.dsk>" [, SIDE]
SAVE "<fname>", START, SIZE, HFE, "<fname.hfe>" [, SIDE]
SAVE "<fname>", START, SIZE, DISC, "<fname.hfe>"|"<fname.dsk>" [, SIDE]
```

Description:
Save assembled data to a file in various formats (AMSDOS, DSK, HFE). TAPE option is not coded. Other options are not intensively tested.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_save.asm"
```

## Debug directives

### PAUSE

Synopsis:

```
PAUSE
```

Description:
Insert a pause during assembly. Can be useful for interactive debugging or inspection during multi-pass assembly.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_pause.asm"
```

### ASSERT

Synopsis:

```
ASSERT BOOLEAN_EXPRESSION [, PRINTABLE_EXPRESSION]*
```

Description:
Validate a condition at assembly time. If the condition is false, assembly fails with an optional message.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_assert.asm"
```

### PRINT

Synopsis:

```
PRINT expression [, expression]*
```

Description:
Print expressions to the console during assembly. Useful for debugging and displaying assembly-time values.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_print.asm"
```


## Amstrad CPC related directives

### TICKER

Synopsis:

```
TICKER START variable
  ... instructions ...
TICKER STOP
```

Description:
Compute the execution duration of a block of code. The variable will contain the number of T-states (cycles) required to execute the code.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ticker.asm"
```

### WAITNOPS

Synopsis:

```
WAITNOPS count
```

Description:
Generate instructions that wait for the specified number of NOPs without modifying registers or memory. Currently generates NOP instructions, but may use optimized instruction sequences in the future.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_waitnops.asm"
```

### LOCOMOTIVE, ENDLOCOMOTIVE

Synopsis:

```
LOCOMOTIVE
  BASIC code lines
ENDLOCOMOTIVE
```

Description:
Embed Locomotive BASIC code in the assembly. Useful for creating loaders or bootstrap code.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_basic.asm"
```

### SNASET

Synopsis:

```
SNASET FLAG, VALUE
```

Description:
Set CPU register or hardware state values in the generated snapshot. **All register names must be prefixed with `Z80_`.**

#### Z80 CPU Registers

**Main registers:**
- `Z80_AF`, `Z80_BC`, `Z80_DE`, `Z80_HL` (16-bit register pairs)
- `Z80_A`, `Z80_F`, `Z80_B`, `Z80_C`, `Z80_D`, `Z80_E`, `Z80_H`, `Z80_L` (8-bit registers)

**Alternate registers (shadow registers):**
- `Z80_AFX`, `Z80_BCX`, `Z80_DEX`, `Z80_HLX` (16-bit alternate pairs)
- `Z80_AX`, `Z80_FX`, `Z80_BX`, `Z80_CX`, `Z80_DX`, `Z80_EX`, `Z80_HX`, `Z80_LX` (8-bit alternates)

**Index registers:**
- `Z80_IX`, `Z80_IY` (16-bit index registers)
- `Z80_IXL`, `Z80_IXH`, `Z80_IYL`, `Z80_IYH` (8-bit index register halves)

**Special registers:**
- `Z80_SP` (Stack Pointer)
- `Z80_PC` (Program Counter)
- `Z80_I` (Interrupt Vector)
- `Z80_R` (Memory Refresh)
- `Z80_IFF0`, `Z80_IFF1` (Interrupt Flip-Flops)
- `Z80_IM` (Interrupt Mode: 0, 1, or 2)

#### Gate Array Registers

- `GA_PEN` - Selected pen number
- `GA_PAL:n` - Palette color n (0-16), requires index
- `GA_ROMCFG` - ROM/screen mode configuration
- `GA_RAMCFG` - RAM configuration
- `GA_MULTIMODE:n` - Multi-mode register n, requires index
- `GA_VSC` - Vertical sync counter
- `GA_ISC` - Interrupt sync counter

#### CRTC Registers

- `CRTC_SEL` - Selected CRTC register
- `CRTC_REG:n` - CRTC register n (0-17), requires index
- `CRTC_TYPE` - CRTC type (0=HD6845S/UM6845, 1=UM6845R, 2=MC6845, 3=AMS40489, 4=Pre-ASIC)
- `CRTC_HCC` - Horizontal character counter
- `CRTC_CLC` - Character line counter
- `CRTC_RLC` - Raster line counter
- `CRTC_VAC` - Vertical adjustment counter
- `CRTC_VSWC` - Vertical sync width counter
- `CRTC_HSWC` - Horizontal sync width counter
- `CRTC_STATE` - CRTC state flags

#### PPI (8255) Registers

- `PPI_A` - Port A
- `PPI_B` - Port B
- `PPI_C` - Port C
- `PPI_CTL` - Control register

#### PSG (AY-3-8912) Registers

- `PSG_SEL` - Selected PSG register
- `PSG_REG:n` - PSG register n (0-15), requires index

#### Other Hardware

- `ROM_UP` - Upper ROM number
- `CPC_TYPE` - CPC type (0=464, 1=664, 2=6128, 3=464+, 4=6128+, 5=KC Compact, 6=Unknown)
- `INT_NUM` - Interrupt number
- `INT_REQ` - Interrupt request flag
- `FDD_MOTOR` - Floppy disk motor state
- `FDD_TRACK` - Floppy disk current track
- `PRNT_DATA` - Printer data port

**Note:** Flags with `:n` suffix (like `GA_PAL:0`, `CRTC_REG:1`, `PSG_REG:7`) require an index to specify which register.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_snaset.asm"
```

### SNAINIT, SNAPINIT

Synopsis:

```
SNAINIT "template.sna"
```

Description:
Initialize snapshot generation from a template snapshot file. The template provides the initial memory and register state. Must be called before using SNASET or other snapshot directives.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_snainit.asm"
```

### BUILDSNA

Synopsis:

```
BUILDSNA "filename.sna"
```

Description:
Generate an Amstrad CPC snapshot file with the assembled code and configured registers. See SNASET example for complete usage.

### BUILDCPR

Synopsis:

```
BUILDCPR "filename.cpr"
```

Description:
Generate a cartridge (CPR) file for the CPC Plus/GX4000.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_buildcpr.asm"
```

### RUN

Synopsis:

```
RUN address
```

Description:
Set the execution address for the assembled program in a snapshot.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_run.asm"
```

### ENT

Synopsis:

```
ENT address
```

Description:
Set the entry point for AMSDOS files.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_ent.asm"
```

## Assembler Control Directives

### ASMCONTROL

Synopsis:

```
ASMCONTROL PRINT_PARSE, expression [, expression]*
ASMCONTROL PRINT_ANY_PASS, expression [, expression]*
```

Description:
Control assembler behavior during assembly. PRINT_PARSE prints expressions during the parsing pass. PRINT_ANY_PASS prints expressions during any assembly pass.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_asmcontrol.asm"
```

### ASMCONTROLENV, ENDASMCONTROLENV

Synopsis:

```
ASMCONTROLENV SET_MAX_NB_OF_PASSES = expression
  ... code with limited assembly passes ...
ENDASMCONTROLENV
```

Description:
Create a block with a maximum number of assembly passes. This restricts the assembler to complete the enclosed code within the specified number of passes.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_asmcontrolenv.asm"
```

### IMPORT

Synopsis:

```
IMPORT "filename"
```

Description:
Import symbols from an external symbol file.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_import.asm"
```

### FUNCTION, ENDFUNCTION

Synopsis:

```
FUNCTION name(param1, param2, ...)
  ... function body ...
  RETURN result
ENDFUNCTION
```

Description:
Define a custom function that can be called in expressions. Similar to macros but returns a value.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_function.asm"
```

### RETURN

Synopsis:

```
RETURN expression
```

Description:
Return a value from a function definition. Can only be used within FUNCTION blocks, not in macros. The expression is evaluated and becomes the return value of the function.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_document_return.asm"
```
