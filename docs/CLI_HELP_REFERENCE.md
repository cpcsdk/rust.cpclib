# Command Line Help Reference

This page provides the help output for all cpclib tools and their subcommands.

## Table of Contents

- [basm](#basm) - Z80 Assembler
- [bdasm](#bdasm) - Z80 Disassembler 
- [catalog](#catalog) - DSK File Manager
- [fade](#fade) - Color Fade Generator
- [img2cpc](#img2cpc) - Image to CPC Converter
- [cpc2img](#cpc2img) - CPC to Image Converter
- [xfertool](#xfertool) - M4 Transfer Tool
- [snapshot](#snapshot) - SNA File Tool

---

## basm

```
{{RUN: cargo run --release -p cpclib-asm -- --help}}
```

---

## bdasm

```  
{{RUN: cargo run --release -p cpclib-bdasm -- --help}}
```

---

## catalog

```
{{RUN: cargo run --release -p cpclib-catalog -- --help}}
```

### catalog Subcommands

#### build
```
{{RUN: cargo run --release -p cpclib-catalog -- build --help}}
```

#### cat
```
{{RUN: cargo run --release -p cpclib-catalog -- cat --help}}
```

#### dir
```
{{RUN: cargo run --release -p cpclib-catalog -- dir --help}}
```

---

## fade

```
{{RUN: cargo run --release -p cpclib-imgconverter --bin fade -- --help}}
```

### fade rgb
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin fade -- rgb --help}}
```

---

## img2cpc

```
{{RUN: cargo run --release -p cpclib-imgconverter --bin img2cpc -- --help}}
```

### img2cpc Subcommands

#### sna
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin img2cpc -- sna --help}}
```

#### dsk
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin img2cpc -- dsk --help}}
```

#### scr
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin img2cpc -- scr --help}}
```

#### exec
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin img2cpc -- exec --help}}
```

---

## cpc2img

```
{{RUN: cargo run --release -p cpclib-imgconverter --bin cpc2img -- --help}}
```

### cpc2img Subcommands

#### palette
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin cpc2img -- palette --help}}
```

#### screen
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin cpc2img -- screen --help}}
```

#### sprite
```
{{RUN: cargo run --release -p cpclib-imgconverter --bin cpc2img -- sprite --help}}
```

---

## xfertool

```
{{RUN: cargo run --release -p cpclib-xfertool -- --help}}
```

---

## snapshot

```
{{RUN: cargo run --release -p cpclib-sna --bin snapshot -- --help}}
```

### snapshot Subcommands

#### chunk
```
{{RUN: cargo run --release -p cpclib-sna --bin snapshot -- chunk --help}}
```

#### header
```
{{RUN: cargo run --release -p cpclib-sna --bin snapshot -- header --help}}
```
