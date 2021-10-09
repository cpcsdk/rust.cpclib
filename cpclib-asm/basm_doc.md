---
title: BASM documentation - WIP
---

# Command line arguments

- --snapshot: save output in an Amstrad CPC snapshot instead of a single file
- --lst [fname]: also generate the listing of the assembled program. Standard output is used if fname is -

# Z80 Syntax

## General syntax

## Labels handling

## Fake instructions

## Comments

# Directives

## Memory related

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

### LZAPU, LZ48, LZ49

### INCBIN, BINCLUDE

### INCLUDE, READ

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