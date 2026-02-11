# CPC Character Fonts

This directory contains character fonts for different CPC language variants.

## Available Fonts

- `font_english.bin` - English (UK) character set (currently using French as placeholder)
- `font_french.bin` - French character set (extracted from fw-f3-u5.rom)

## Adding New Language Fonts

To add fonts for other languages (Spanish, German, Danish, etc.):

1. Download the appropriate firmware ROM from the [CPC Wiki ROM List](https://www.cpcwiki.eu/index.php/ROM_List#Lower_ROMs)

2. Extract the character font data using this Python script:

```python
#!/usr/bin/env python3
# Extract character font from CPC firmware ROM

import sys

rom_file = "path/to/firmware.rom"  # Your ROM file
output_file = "font_LANGUAGE.bin"   # Output file (e.g., font_spanish.bin)

with open(rom_file, 'rb') as f:
    rom_data = f.read()

# CPC ROM has 128-byte header, character set is at offset 0x3800
rom_header = 128
font_offset = rom_header + 0x3800  # 128 + 0x3800 = 0x3880
font_size = 0x800  # 2048 bytes = 256 characters * 8 bytes each

if len(rom_data) >= font_offset + font_size:
    font_data = rom_data[font_offset:font_offset + font_size]
    
    with open(output_file, 'wb') as f:
        f.write(font_data)
    
    print(f"✓ Extracted {len(font_data)} bytes to {output_file}")
else:
    print(f"✗ ROM file too small")
    sys.exit(1)
```

3. Copy the extracted `.bin` file to this directory

4. Update the `Locale::font_data()` method in `interpret.rs` to use your new font file

## Language Variants

Common CPC firmware language variants:

- **English (UK)**: Standard English character set
- **French**: Includes French accented characters (à, é, è, ç, etc.)
- **Spanish**: Includes Spanish characters (ñ, á, é, í, ó, ú, ¿, ¡, etc.)
- **German**: Includes German characters (ä, ö, ü, ß, etc.)
- **Danish**: Includes Danish characters (æ, ø, å, etc.)

## Character Set Layout

Each font file is 2048 bytes:
- 256 characters × 8 bytes per character
- Each character is an 8×8 pixel bitmap
- Characters 0-31: Control characters (some have graphics)
- Characters 32-127: ASCII characters
- Characters 128-255: Extended characters (accented letters, graphics, etc.)

The character differences between language variants are typically in the extended character range (128-255), where accented letters and special characters are located.
