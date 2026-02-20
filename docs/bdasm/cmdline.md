# BDASM Command Line Reference

## Synopsis

```bash
bdasm [OPTIONS] <INPUT>
```

## Description

BDASM (Benediction Disassembler) is a Z80 disassembler that converts machine code back into human-readable assembly language.

## Arguments

### `<INPUT>`
Input binary file to disassemble (required).

## Options

### `-o, --origin <ORIGIN>`
Disassembling origin address (memory location where code will be loaded).

**Example:**
```bash
bdasm -o 0x4000 program.bin
```

### `-d, --data <DATA_BLOC>`
Relative position that contains data (not code) for a given size.

**Format:** `RELATIVE_START(in hexadecimal)-SIZE(in decimal)`

**Description:**  
Tells the disassembler that a specific region contains data bytes, not instructions, so it won't try to disassemble them as code.

**Example:**
```bash
# Treat bytes at offset 0x100 for 64 bytes as data
bdasm -o 0x8000 -d 0x100-64 program.bin
```

### `-l, --label <LABEL>`
Set a label at the given address.

**Format:** `LABEL=ADDRESS`

**Description:**  
Define symbolic names for specific addresses in the disassembly output.

**Example:**
```bash
bdasm -l MAIN=0x4000 -l LOOP=0x4010 program.bin
```

You can specify multiple labels:
```bash
bdasm -l INIT=0x4000 -l GAME_LOOP=0x4100 -l SOUND=0x5000 game.bin
```

### `-s, --SKIP <SKIP>`
Skip the first `<SKIP>` bytes of the input file.

**Description:**  
Useful for skipping file headers or unwanted data at the beginning of the file.

**Example:**
```bash
# Skip 128-byte header
bdasm -s 128 -o 0x4000 program.bin
```

### `-c, --compressed`
Output a simple listing that only contains the opcodes (no addresses or hex dump).

**Description:**  
Produces minimal output showing only the disassembled instructions, useful for cleaner code review or when you only need the assembly mnemonics.

**Example:**
```bash
bdasm -c program.bin
```

Output:
```asm
LD A,0
LD BC,0x1234
JP 0x4000
```

Vs. normal output:
```asm
4000: 3E 00        LD A,0
4002: 01 34 12     LD BC,0x1234
4005: C3 00 40     JP 0x4000
```

### `-h, --help`
Print help information.

### `-V, --version`
Print version information.

## Environment Variables

None

## Exit Status

- `0` - Success
- `1` - Error during disassembly or file I/O

## Examples

See [Examples](examples.md) for detailed usage examples.
