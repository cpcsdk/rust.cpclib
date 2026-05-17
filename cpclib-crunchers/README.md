# cpclib-crunchers

A collection of compression/crunching algorithms optimized for Amstrad CPC development. This crate embeds C/C++ implementations of various compression methods commonly used in demo coding.

## Available Crunchers

The following compression algorithms are available (when their respective features are enabled):

- **Apultra** (BSD 3-Clause) - Modern compression algorithm
- **ZX0** (BSD 3-Clause) - Optimal compressor for 8-bit systems  
- **ZX7** (BSD 3-Clause) - Fast compressor variant
- **LZ4** (BSD 2-Clause) - High-speed compression
- **LZSA** (Zlib/CC0) - Optimized LZ algorithm with dual licensing
- **PuCrunch** (LGPL) - General-purpose cruncher
- **Shrinkler** (Custom) - Advanced compressor for executables
- **Exomizer** (Non-commercial) - Professional-grade compressor
- **UPKR** (Public Domain) - Lightweight packer
- **BZPack** (BSD 2-Clause) - Multiple compression formats (LZM, EF8, BX0, BX2)
- **LZ48/LZ49** (No explicit license) - Rust adaptations

## License Information

**⚠️ Important**: Each embedded cruncher has its own license terms. Before using this crate in a commercial product, review the license file:

📄 **[CRUNCHER_LICENSES.md](./CRUNCHER_LICENSES.md)** - Complete license details for all algorithms

### Quick License Summary

| Category | Crunchers |
|----------|-----------|
| **Commercial OK** | Apultra, ZX0, ZX7, LZ4, LZSA, BZPack, UPKR |
| **Restricted** | Exomizer (non-commercial only), PuCrunch (LGPL) |
| **Special** | Shrinkler (proprietary), LZ48/LZ49 (unknown license) |

### Critical Restrictions

- **Exomizer**: Non-commercial use only. Do not use in commercially distributed software without explicit permission.
- **PuCrunch**: LGPL - modifications must be disclosed.
- **Shrinkler**: Proprietary - verify usage rights with author.

## Usage in cpclib-bndbuild

The crunchers are primarily accessed through `cpclib-bndbuild` with the `--cruncher` option:

```bash
cpclib-bndbuild --cruncher apultra input.bin output.crunched
```

For all available crunchers, run:

```bash
cpclib-crunch --help
```

## Features

The following features can be enabled/disabled:

- `apultra` (default) - Apultra compressor
- `exomizer` (default) - Exomizer compressor
- `lz4` (default) - LZ4 compressor
- `lz48` (default) - LZ48 compressor
- `lz49` (default) - LZ49 compressor
- `lzsa` (default) - LZSA compressor
- `shrinkler` (default) - Shrinkler compressor
- `pucrunch` (default) - PuCrunch compressor
- `upkr` (default) - UPKR packer
- `zx0` (default) - ZX0 compressor
- `bzpack` (default) - BZPack compressor family
- `zx7` (optional) - ZX7 compressor (not in default)

## Z80 Decompression Routines

For most crunchers, Z80 assembly decompression stubs are available. These can be generated using:

```bash
cpclib-crunch --cruncher <algorithm> --z80
```

Note: Some BZPack forward formats (Bx0, Bx2, EF8, Lzm) do not have Z80 decompression routines available in forward mode. Use the backward variants instead.

## References

- **CRUNCHER_LICENSES.md** - Detailed licensing and attribution for all embedded algorithms
- Individual source files in `extra/` directory contain license headers and copyright notices
