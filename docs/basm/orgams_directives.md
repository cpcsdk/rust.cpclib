# ORGAMS-Only Directives

This document lists directives that are only available when ORGAMS compatibility mode is enabled. These directives are not supported in standard basm mode.

## SKIP

Synopsis:

```
SKIP <count>
```

Description:
Advances both the code address (`$`) and output address by the specified number of bytes without writing any data to memory. This directive reserves space in memory by skipping over it.

**Note: SKIP is only available in ORGAMS compatibility mode and is not supported in standard basm.**

For standard basm, use `DEFS` or `DS` to reserve memory, though these directives fill the space with a value (default 0) rather than just advancing addresses.

Example:

```z80
--8<-- "cpclib-basm/tests/asm/orgams_good_document_skip.asm"
```
