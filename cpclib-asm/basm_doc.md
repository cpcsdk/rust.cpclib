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

### LOCOMOTIVEBASIC

### SNASET

# Expression handling

## Special variables

 - $: get the current code address
 - $$: get the current output address

## Special functions