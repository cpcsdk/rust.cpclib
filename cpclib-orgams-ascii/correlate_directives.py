#!/usr/bin/env python3
"""
Correlate directive keywords in .Z80 source with command bytes in .I binary.
"""

from pathlib import Path
import re

def find_directives_in_source(z80_path):
    """Extract directive lines with their line numbers"""
    with open(z80_path, 'r', encoding='latin-1') as f:
        lines = f.readlines()
    
    directives = []
    for i, line in enumerate(lines, 1):
        # Match directive keywords at start of line (after whitespace)
        match = re.match(r'^\s+(IF|ELSE|END|SKIP|IMPORT|SAVE|LOAD|STR|BANK|REPEAT|ENT|DEFS|DS)\s', line, re.IGNORECASE)
        if match:
            keyword = match.group(1).upper()
            directives.append({
                'line': i,
                'keyword': keyword,
                'text': line.strip()
            })
    
    return directives

def analyze_i_file_directives(i_path):
    """Extract all 7F XX patterns with their offsets"""
    with open(i_path, 'rb') as f:
        content = f.read()
    
    patterns = []
    for i in range(len(content) - 2):
        if content[i] == 0x7F:
            cmd_byte = content[i + 1]
            # Get context
            ctx = content[i:min(i+30, len(content))]
            patterns.append({
                'offset': i,
                'cmd_byte': cmd_byte,
                'context': ctx
            })
    
    return patterns

def main():
    # Analyze MEMMAP files
    z80_path = Path("tests/orgams-main/MEMMAP.Z80")
    i_path = Path("tests/orgams-main/MEMMAP.I")
    
    print("Extracting directives from MEMMAP.Z80...")
    directives = find_directives_in_source(z80_path)
    
    print(f"Found {len(directives)} directives\n")
    
    # Show first few
    print("Sample directives:")
    for d in directives[:15]:
        print(f"  Line {d['line']:3d}: {d['keyword']:<8} {d['text'][:60]}")
    
    print("\n" + "="*80)
    print("\nAnalyzing MEMMAP.I for directive markers...")
    patterns = analyze_i_file_directives(i_path)
    
    # Group by command byte
    by_cmd = {}
    for p in patterns:
        cb = p['cmd_byte']
        if cb not in by_cmd:
            by_cmd[cb] = []
        by_cmd[cb].append(p)
    
    print(f"\nCommand bytes found (after 0x7F):\n")
    for cb in sorted(by_cmd.keys()):
        occurrences = by_cmd[cb]
        print(f"0x{cb:02X}: {len(occurrences):3d} times - First at offset 0x{occurrences[0]['offset']:06x}")
        # Show first context
        ctx = occurrences[0]['context']
        hex_str = ' '.join(f'{b:02x}' for b in ctx[:16])
        ascii_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in ctx[:16])
        print(f"      Context: {hex_str}  {ascii_str}")
    
    print("\n" + "="*80)
    print("\nManual correlation needed:")
    print("1. IMPORT at line 21 → we know it's 0x17")
    print("2. IF directives at lines 97, 100, 130, etc.")
    print("3. SKIP directives at lines 206, 345, 348")
    print("\nLet's check specific line correlations...")
    
    # We know from the test that:
    # - Line 21 is IMPORT → offset 0x02B7 → 7F 17
    # - So we can estimate: ~200 bytes per 10 lines (rough)
    # - Line 97 would be around offset ~(97/21) * 0x2B7 = ~0xC00
    
    print(f"\nFirst few 7F patterns:")
    for p in patterns[:10]:
        ctx = p['context']
        hex_str = ' '.join(f'{b:02x}' for b in ctx[:20])
        ascii_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in ctx[:20])
        print(f"0x{p['offset']:06x}: 7F {p['cmd_byte']:02X} - {hex_str}")
        print(f"           {ascii_str}")
    
    print("\n" + "="*80)
    print("\nCommand byte hypotheses:")
    print("0x17 = IMPORT (confirmed)")
    print("0x15 = IF (likely - appears multiple times)")  
    print("0x03 = ? (very common)")
    print("0x01 = ? (common - possibly comment/asis)")
    print("0x0C = ? (appears often)")
    print("0x0F = ? (appears often)")
    print("0x04 = ? (appears often)")
    print("0x08 = SKIP (likely)")
    print("0x09 = ? (common)")
    print("0x43 = inline comment marker")

if __name__ == "__main__":
    main()
