---
title: BASM documentation - WIP
authors:
 - Krusty/Benediction
---


## BASM

Benediction ASsembler (`BASM` in short) is a modern Z80 assembler.
He has taken its inspiration from various Z80 assembler (Maxam/[Winape](http://www.winape.net/help/), [sjasmplus](https://github.com/z00m128/sjasmplus), [rasm](https://github.com/EdouardBERGE/rasm), [vasm](http://sun.hasenbraten.de/vasm/), [BRASS](https://benryves.com/bin/brass/), [glass](https://grauw.nl/projects/glass/), [zasm](https://k1.spdns.de/Develop/Projects/zasm)) as well as assemblers from other platforms ([asm11](http://www.aspisys.com/asm11man.htm), [sarcasm](https://www.ecstaticlyrics.com/electronics/Z80/sarcasm/)).
It is tailored for Amstrad CPC demomaking and  has been successfully used to develop the Amstrad CPC demo [Can Robots Take Control?](https://www.pouet.net/prod.php?which=88554).
It has been still improved since and will serve for futur productions too.


The documentation is quite minimal at the moment, but included example code should be still valid and assembled propetly.
The user base being quite small, lots of bugs can remain. Do note hesitate to fill issues <https://github.com/cpcsdk/rust.cpclib/issues> or propose fixes.


## Features of Interest

- Possibility to assemble fake instructions (e.g. `ld hl, de`).
- Possibility to use standard directives (e.g. `incbin 'file.asm`).
- Rare directives and functions (e.g. `ld a, opcode(xor a)`).
- Macros definition and usage (e.g. `MY_MACRO_WITH_TWO_ARGS 1, "string"`).
- Function definition and usage (e.g. `db 5, my_function(3)`).
- Expressions able to handle numbers, strings, lists, matrices - see [expression types](expression-types.md).
- Handling of Amstrad CPC snapshots.
- Possibility to execute directly the assembled project in the Amstrad CPC thanks to the M4/CPC WIFI card.
- Multi-pass (in fact, `BASM` uses as many passes as needed).
- Multiplatform (mainly tested on Linux and Windows).
- Embedding of various ASM source files inside `BASM` that can be used by the users.
- Possibility to write LOCOMOTIVE BASIC for easily writting Amstrad CPC bootstrap loaders.


## Hello World

An hello world representative of the functionalities of `BASM` would be:
```z80
--8<-- "cpclib-basm/tests/asm/good_hello_world.asm"
```

## Download last version

Prefer to compile yourself `basm`. But you can still download latest versions here:

 - [Linux](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/basm) 
 - [Windows](https://github.com/cpcsdk/rust.cpclib/releases/download/latest/basm.exe) 


!!! failure Wrong files

    Continuous delivery system for Linux is broken. The executables are outdated of few years...


## Differences with RASM

- slower on the parsing side
- more buggy because not enough tested ;)
- `MODULE` directive must be closed by `ENDMODULE`
- `REPEAT` counter is not accessible by using the variable `counter` but `{counter}` as in a `MACRO`
- It is possible to name a `MACRO` using the label before the `MACRO` directive
- More data types (list, matrix, int, float, boolean)
- As `basm` can use an unlimited number of pass (warning there is not infinite loop check ATM), it can assemble  code that would not be assembled with `rasm` because labels have to be known at this moment
- Weak support of `DSK``, no support of `TAPE`` and `CPR``. `HFE` is usable on Linux with the appropriate compilation option. `AMSDOS` support is buggy `ATM`
- `SNA` should be ok
- Possibility to add some `BASIC` tokens to create loaders that do not clear the screen when launched
