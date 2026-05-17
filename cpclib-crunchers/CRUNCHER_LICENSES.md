# Cruncher Library Licenses

This document lists the licenses for all compression/crunching algorithms embedded in `cpclib-crunchers`.

## Overview

| Cruncher | License | Author(s) | Year(s) | Commercial Use |
|----------|---------|-----------|---------|-----------------|
| Apultra | BSD 3-Clause | Emmanuel Marty | 2019 | ✅ Yes |
| ZX0 | BSD 3-Clause | Einar Saukas | 2021 | ✅ Yes |
| ZX7 | BSD 3-Clause | Einar Saukas | 2012 | ✅ Yes |
| Exomizer | Custom Non-commercial | Magnus Lind | 2005 | ❌ No (non-commercial only) |
| LZ4 | BSD 2-Clause | Yann Collet | 2011-2017 | ✅ Yes |
| LZSA (v1 & v2) | Zlib / CC0 Dual | Emmanuel Marty | - | ✅ Yes |
| LZ48 | No explicit license | Roudoudou (Rust adaptation) | - | ⚠️ Check with author |
| LZ49 | No explicit license | Roudoudou (Rust adaptation) | - | ⚠️ Check with author |
| PuCrunch | LGPL | Pasi "Albert" Ojala | 1997-2008 | ⚠️ LGPL restrictions |
| Shrinkler | Custom/Proprietary | Aske Simon Christensen | 1999-2020 | ⚠️ Check terms |
| UPKR | Public Domain / Unlicensed | exoticorn | - | ✅ Yes |
| BZPack (LZM/EF8/BX0/BX2) | BSD 2-Clause | Milos "baze" Bazelides | 2021 | ✅ Yes |

## Detailed License Information

### Permissive Licenses (OK for commercial use)
- **Apultra** - BSD 3-Clause License
- **ZX0** - BSD 3-Clause License  
- **ZX7** - BSD 3-Clause License
- **LZ4** - BSD 2-Clause License
- **LZSA** - Zlib License (with CC0 for matchfinder)
- **BZPack** - BSD 2-Clause License
- **UPKR** - Public Domain

### Restrictive/Special Licenses
- **Exomizer** - Non-commercial license only. Explicit restriction on commercial use.
- **PuCrunch** - LGPL. Requires source code disclosure if modifications are made.
- **Shrinkler** - Custom/Proprietary license. Aske Simon Christensen retains copyright. Check usage terms.
- **LZ48/LZ49** - Roudoudou's Rust adaptations have no explicit license statement. Check with Roudoudou for terms.

## License File Locations

- **Apultra**: `cpclib-crunchers/extra/apultra.c` (header comments)
- **ZX0/ZX7**: `cpclib-crunchers/extra/zx7/zx0_compress.c` and `zx7.c` (header comments)
- **Exomizer**: `cpclib-crunchers/extra/exomizer.c` (header comments)
- **LZ4**: `cpclib-crunchers/extra/lz4_embedded.c` (header comments)
- **LZSA**: `cpclib-crunchers/extra/lzsa/lzsa-master/LICENSE` and related files
- **PuCrunch**: `cpclib-crunchers/extra/pucrunch.c` (header comments)
- **Shrinkler**: `cpclib-crunchers/extra/Shrinkler4.6NoParityContext/Shrinkler.cpp` (header comments)
- **BZPack**: `cpclib-crunchers/extra/bzpack/Compressor.cpp` (header comments)

## Important Notes

⚠️ **Exomizer**: The non-commercial restriction means this cruncher cannot be used in commercially distributed software without explicit permission from Magnus Lind.

⚠️ **PuCrunch**: The LGPL license requires that if modifications are made to the PuCrunch code, those modifications must be made available. However, using it as-is without modification should be fine for any use.

⚠️ **Shrinkler**: Aske Simon Christensen retains copyright. While available, verify you have rights to use it in your project.

⚠️ **LZ48/LZ49**: These are Rust adaptations with unclear original license status. Before using commercially, verify licensing with Roudoudou.

## Recommendations

1. **For commercial use**: Use only Apultra, ZX0, ZX7, LZ4, LZSA, BZPack, or UPKR.
2. **For GPL-compatible projects**: Can use PuCrunch and LZ4 (both GPL-compatible).
3. **For maximum permissiveness**: Prefer ZX0/ZX7 and LZSA (all highly permissive).
4. **Avoid**: Exomizer for commercial use unless you have explicit permission.

## Updates

This license information was compiled on 2026-05-17. Please check individual source files and license documents for the most current information.
