#!/usr/bin/env python3
"""
Extract CPC fonts from language-specific ROM files.
Each language folder contains ROMs with the firmware (OS) or BASIC ROM.
Font data is at offset 0x3880 (128-byte header + 0x3800).
"""

import os
import glob
import hashlib

def extract_font_from_rom(rom_path):
    """Extract 2KB font data from CPC ROM"""
    with open(rom_path, 'rb') as f:
        rom_data = f.read()
    
    font_size = 2048  # 256 characters * 8 bytes each
    
    # Lower ROM (16384 bytes): font at 0x3800
    # Full ROM with header: 128-byte header + font at 0x3800 = offset 0x3880
    if len(rom_data) == 16384:
        # Lower ROM without header
        font_offset = 0x3800
    else:
        # Full ROM with header
        font_offset = 0x3880
    
    if len(rom_data) < font_offset + font_size:
        return None
    
    return rom_data[font_offset:font_offset + font_size]

def main():
    fonts_dir = "/home/romain/Perso/CPC/rust.cpcdemotools/cpclib-catart/src/fonts"
    
    languages = {
        'english': 'font_english.bin',
        'french': 'font_french.bin',
        'spanish': 'font_spanish.bin',
        'danish': 'font_danish.bin'
    }
    
    print("Extracting CPC fonts from language-specific ROMs...\n")
    
    for lang, output_filename in languages.items():
        lang_dir = os.path.join(fonts_dir, lang)
        
        if not os.path.exists(lang_dir):
            print(f"❌ {lang.upper()}: Directory not found: {lang_dir}")
            continue
        
        # Find ROM files (prefer OS ROMs over BASIC ROMs, prefer corrected versions)
        rom_patterns = [
            f"{lang_dir}/*Lower*.rom",     # Danish corrected
            f"{lang_dir}/*OS*.ROM",         # English
            f"{lang_dir}/os6128*.rom",      # French/Spanish OS
            f"{lang_dir}/*AZERTY*.rom",     # French AZERTY (standard)
            f"{lang_dir}/*.rom",            # Any ROM
        ]
        
        rom_file = None
        for pattern in rom_patterns:
            matches = glob.glob(pattern)
            if matches:
                # For corrected versions, prefer "Corrected" in name
                corrected = [m for m in matches if "Corrected" in m or "corrected" in m]
                rom_file = corrected[0] if corrected else matches[0]
                break
        
        if not rom_file:
            print(f"❌ {lang.upper()}: No ROM file found in {lang_dir}")
            continue
        
        # Extract font
        font_data = extract_font_from_rom(rom_file)
        
        if font_data is None:
            print(f"❌ {lang.upper()}: ROM too small: {rom_file}")
            continue
        
        # Save font
        output_path = os.path.join(fonts_dir, output_filename)
        with open(output_path, 'wb') as f:
            f.write(font_data)
        
        # Calculate MD5
        md5 = hashlib.md5(font_data).hexdigest()
        
        # Check character 92 to identify language
        char_92 = font_data[92*8:92*8+8]
        char_92_hex = ' '.join(f'{b:02X}' for b in char_92)
        
        # Identify character 92 pattern
        if char_92[1:3] == bytes([0x40, 0x20]):
            char_92_desc = "\\ (backslash - English)"
        elif char_92[2:4] == bytes([0x3C, 0x66]):
            char_92_desc = "ù (u-grave - French)"
        else:
            char_92_desc = f"? (unknown: {char_92_hex})"
        
        rom_name = os.path.basename(rom_file)
        print(f"✅ {lang.upper():<8} → {output_filename}")
        print(f"   Source: {rom_name}")
        print(f"   MD5: {md5}")
        print(f"   Char 92: {char_92_desc}")
        print()
    
    print("\n" + "="*60)
    print("Verifying all fonts are different...")
    
    # Check all fonts have different MD5s
    font_hashes = {}
    for lang, filename in languages.items():
        path = os.path.join(fonts_dir, filename)
        if os.path.exists(path):
            with open(path, 'rb') as f:
                md5 = hashlib.md5(f.read()).hexdigest()
                if md5 in font_hashes:
                    print(f"⚠️  {lang} and {font_hashes[md5]} have IDENTICAL fonts!")
                else:
                    font_hashes[md5] = lang
    
    print(f"\n✅ Successfully extracted {len(font_hashes)} unique fonts")

if __name__ == "__main__":
    main()
