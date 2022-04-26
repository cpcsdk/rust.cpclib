---
title: BASM documentation - WIP
---

# Command line arguments

- --snapshot: save output in an Amstrad CPC snapshot instead of a single file
- --lst [fname]: also generate the listing of the assembled program. Standard output is used if fname is -

# Z80 Syntax

## General syntax

## Labels handling

basm is quite lax on the z80 format: it does not impose to start a label at the very first char of a line and does not force an instruction or directive to not start at the very first line of a char.
As a consequence there can be ambiguities between labels and macros.
If it fails in properly recognizing macros or label, you can guide it by suffixing label declaration by : or by using (void) for macros with no arguments. 

## Fake instructions

## Comments

## Expressions

## Provided functions

## User defined functions

Function, endf

# Directives

## Memory related


### CONFINED

Confine a memory area of 256 bytes maximum in such a way that it is always possible to navigate in the data by only modifying the low byte address (*i.e* INC L always works).

CONFINED
  ...
ENDCONFINED

### ORG

### ALIGN

### LIMIT

On the code space ($), not physical space ($$)

### PROTECT

On the code space ($), not physical space ($$)

### BANK

When used with no argument, a bank corresponds to a memory area outside of the snapshot. All things read&write in memory are thus uncorrelated to the snapshot.
Sections do not apply in a bank.

## Labels related

## =

## EQU

## Data related

### BYTE, TEXT, DB, DEFB, DM, DEFM

### WORD, DW, DEFW

### STR

### CHARSET

## Conditional directives

### IF, IFNOT

### IFDEF, IFNDEF


## Code duplication directives

### WHILE

### REPEAT

REPEAT AMOUNT [, COUNTER [, START]]
	INNER LISTING
REND
### ITERATE

ITERATE COUNTER, EXPR...
	INNER LISTING
IEND

The expression $i$ is evaluated after having generated the code of expression $i-1$. Take that into account if expressions use $.

    iterate value, 1, 2, 10
        add {value}
        jr nz, @no_inc
            inc c
@no_inc
		call do_stuff
    iend

do_stuff
	ret

## Code and data generation directives

### MACRO

### STRUCT

## Data loading and transformation directives

Filenames are stored in a string.
These string can do expansion of formulas embedded in {}.

basm embeds some files in its executable, they are access under the name "inner://" :
### LZAPU, LZ48, LZ49

### INCBIN, BINCLUDE

INCBIN|BINCLUDE "fname" [[, SKIP], AMOUNT]


Fname can be build with variables.

Limitations:

- File is loaded fully in memory before being sliced depending on arguments.

### INCLUDE, READ

`INCLUDE|READ [ONCE] "fname" [AS|MODULE|NAMESPACE "module"]`

Fname can be build with variables.

## Data saving and export

### EXPORT, NOEXPORT

### WRITE


## Debug directives

### ASSERT

### PRINT

## Amstrad CPC related directives

### TICKERSTART

### WAITNOPS

Generate a list of instructions that do not modify any registers or memory but is executed with the expected amount of nops.
(Currently it is synonym of NOP, but as soon as someone wants to provide clever rules to use less bytes, I'll implement them)

### LOCOMOTIVEBASIC

### SNASET

# Expression handling

## Special variables

 - $: get the current code address
 - $$: get the current output address

## Special functions