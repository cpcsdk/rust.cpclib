

# Directives

!!! failure Inacurate documentation

    Not all directives have their synopsis, explanation, and examples


## Listing related

### LIST, NOLIST
Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_list.asm"
```


## Memory related

### ALIGN

Example:

```z80
--8<-- "cpclib-basm/tests/asm/good_align.asm"
```


### CONFINED

Confine a memory area of 256 bytes maximum in such a way that it is always possible to navigate in the data by only modifying the low byte address (*i.e* INC L always works).

```
CONFINED
  LISTING
ENDCONFINED
```

```z80
--8<-- "cpclib-basm/tests/asm/good_confined.asm"
```


### ORG



### LIMIT

On the code space ($), not physical space ($$)

Example of code that assembles:
```z80
--8<-- "cpclib-basm/tests/asm/good_limit.asm"
```

Example of code that fails:
```z80
--8<-- "cpclib-basm/tests/asm/wrong_limit.asm"
```

### PHASE, DEPHASE

```z80
--8<-- "cpclib-basm/tests/asm/good_phase.asm"
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


### RANGE, SECTION

Description:
RANGE allows to define named portion of the memory, while SECTION allows to chose the portion of interest.

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

### MAP

`MAP VALUE` define a map counter to `the required value.
`#` is used to assign the value to a given label and increment it of the appropriate amount.


Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_map.asm"
```

### SETN, NEXT

`MAP` directive is probably easier to use

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_next.asm"
```

### UNDEF

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_undef.asm"
```

## Data related

### BYTE, TEXT, DB, DEFB, DM, DEFM

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_db.asm"
```

### WORD, DW, DEFW

### DEFS

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_defs.asm"
```

### STR

Description:
STR encodes string in AMSDOS format (i.e., adds 0x80 to the last char) and stores its bytes in memory.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_str.asm"
```

### CHARSET

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_charset.asm"
```


## Conditional directives

### IF, IFNOT

### IFDEF, IFNDEF

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_ifndef.asm"
```

### IFUSED

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_ifused.asm"
```

### Nested conditions

Conditions can be nested.

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_if.asm"
```


### SWITCH, ENDSWITCH

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_switch.asm"
```

## Code duplication directives

### FOR

```
FOR <variable> [, EXPRESSION]+
  LISTING
ENDFOR|FEND
```

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_for.asm"
```
Corresponds to
```z80
--8<-- "cpclib-basm/tests/asm/good_for.equiv"
```


### WHILE

```z80
--8<-- "cpclib-basm/tests/asm/good_while.asm"
```

### REPEAT

REPEAT AMOUNT [, COUNTER [, START]]
	INNER LISTING
REND


```z80
--8<-- "cpclib-basm/tests/asm/good_repeat_incbin2.asm"
```

### ITERATE

```
ITERATE COUNTER, EXPR...
	INNER LISTING
IEND
```

The expression $i$ is evaluated after having generated the code of expression $i-1$. Take that into account if expressions use $.


Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_iter.asm"
```
Corresponds to:
```z80
--8<-- "cpclib-basm/tests/asm/good_iter.equiv"
```

## Code and data generation directives

### MACRO

Example of standard macro:
```z80
--8<-- "cpclib-basm/tests/asm/good_macro_rasmdoc.asm"
```

Example of macro using raw arguments:
```z80
--8<-- "cpclib-basm/tests/asm/good_macro_raw_input.asm"
```


### STRUCT

Description:
Structures allow to defined data blocs with semantic.
In practice, they replace bunches of `DEFB`, `DEFW` directives and enforce checks at assembling (you cannot add more data than expected or forget some).
If a label is used before the use of a struct, it is necessary to postfix it by :.
Otherwise the assembler thinks the label is a macro or structure call.

Synopsis

```
STRUCT <name>
<filed1> DB|DW|STR|<other struct> [<value>]
...
<filedn> DB|DW|<other struct> [<value>]
ENDSTRUCT



[<label>:] <name> <arg1>, ... , <argn>
```

Standard example:
```z80
--8<-- "cpclib-basm/tests/asm/good_struct.asm"
```

Example using default values:
```z80
--8<-- "cpclib-basm/tests/asm/good_struct2.asm"
```

## Data loading and transformation directives

Filenames are stored in a string.
These string can do expansion of formulas embedded in {}.

basm embeds some files in its executable, they are access under the name "inner://" :

### LZAPU, LZ48, LZ49

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_aplib_decrunch.asm"
```


### INCBIN, BINCLUDE



`INCBIN|BINCLUDE "fname" [[, SKIP], AMOUNT]`


Fname can be build with variables.

Limitations:

- File is loaded fully in memory before being sliced depending on arguments.


Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_incbin3.asm"
```
with `AZERTY.TXT` containing the text `AZERTYUIOPQSDFGHJKLMWXCVBN`.


### INCLUDE, READ

`INCLUDE|READ [ONCE] "<fname>" [AS|MODULE|NAMESPACE "<module>"]`

Fname can be build with variables.

Example with once:
```z80
--8<-- "cpclib-basm/tests/asm/good_include2.asm"
```

Example with namespace:
```z80
--8<-- "cpclib-basm/tests/asm/good_include3.asm"
```

Files prefixed by `inner://` are embedded by `BASM`.
Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_embedd.asm"
```

In case of conditional assembling, inclusion are only done in the executed branch. This code always assemble as it never includes 'unknonw' file.

```z80
--8<-- "cpclib-basm/tests/asm/good_noinclude.asm"
```

## Data saving and export

### EXPORT, NOEXPORT

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_export.asm"
```


### SAVE, WRITE

- `SAVE "<fname>", [[[START], [SIZE]], AMSDOS|BASIC|TAPE]`
- `SAVE "<fname>", START, SIZE, DSK, "<fname.dsk>" [, SIDE]`
- `SAVE "<fname>", START, SIZE, HFE, "<fname.hfe>" [, SIDE]`
- `SAVE "<fname>", START, SIZE, DISC, "<fname.hfe>"|"<dname.dsk>" [, SIDE]`


!!! Unimplemented

    TAPE option is not coded.
    Other options are not intensively tested


Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_save.asm"
```

## Debug directives

### ASSERT

```
ASSERT BOOLEAN_EXPRESSION [, PRINTABLE_EXPRESSION]*
```

### PRINT

Example:
```z80
--8<-- "cpclib-basm/tests/asm/good_print.asm"
```


## Amstrad CPC related directives

### TICKER

Description:
Compute the execution duration of a block of code

Synopsys:
```
TICKER START variable
 instructions
TICKER STOP
```

Example 1:
```z80
--8<-- "cpclib-basm/tests/asm/good_cycle.asm"
```

Example 2:
```z80
--8<-- "cpclib-basm/tests/asm/good_timing.asm"
```

### WAITNOPS

Generate a list of instructions that do not modify any registers or memory but is executed with the expected amount of nops.
(Currently it is synonym of NOP, but as soon as someone wants to provide clever rules to use less bytes, I'll implement them)

### LOCOMOTIVE

```z80
--8<-- "cpclib-basm/tests/asm/good_basic.asm"
```



### SNASET