#!/usr/bin/env python3
"""
Extract actual command byte values by analyzing .I files.

This analyzes the binary .I files to find directive markers (7F XX) patterns
and correlates them with the known directive keywords to reverse-engineer
the command byte values.
"""

import sys
from pathlib import Path
from collections import defaultdict

def analyze_i_file_for_directives(i_file_path, z80_file_path=None):
    """Analyze a .I file to extract directive command bytes"""
    
    with open(i_file_path, 'rb') as f:
        content = f.read()
    
    # Find all patterns starting with 0x7F (ec_esc marker)
    directives_found = []
    
    for i in range(len(content) - 2):
        if content[i] == 0x7F:
            command_byte = content[i + 1]
            # Skip if this looks like a string table (< 0x17)
            if command_byte >= 0x10:
                # Extract some context (next few bytes)
                context = content[i:min(i+20, len(content))]
                directives_found.append({
                    'offset': i,
                    'command_byte': command_byte,
                    'context': context.hex()
                })
    
    return directives_found

def extract_command_bytes_from_all_files():
    """Analyze all .I files to find command byte patterns"""
    
    test_dir = Path("tests/orgams-main")
    i_files = list(test_dir.rglob("*.I"))
    
    print(f"Analyzing {len(i_files)} .I files...")
    
    command_byte_counts = defaultdict(lambda: {'count': 0, 'files': set(), 'contexts': []})
    
    for i_file in i_files:
        directives = analyze_i_file_for_directives(i_file)
        
        for d in directives:
            cb = d['command_byte']
            command_byte_counts[cb]['count'] += 1
            command_byte_counts[cb]['files'].add(i_file.name)
            if len(command_byte_counts[cb]['contexts']) < 3:
                command_byte_counts[cb]['contexts'].append(d['context'])
    
    # Sort by command byte value
    print(f"\nFound {len(command_byte_counts)} unique command bytes after 0x7F:\n")
    print(f"{'Byte':<6} {'Count':<8} {'Files':<40} {'Example Context'}")
    print("-" * 120)
    
    for cb in sorted(command_byte_counts.keys()):
        info = command_byte_counts[cb]
        files_str = ', '.join(sorted(info['files']))[:35]
        context_str = info['contexts'][0][:40] if info['contexts'] else ''
        print(f"0x{cb:02X}   {info['count']:<8} {files_str:<40} {context_str}")
    
    return command_byte_counts

def correlate_with_directives(command_byte_counts):
    """Try to correlate command bytes with known directives"""
    
    # Known mappings
    known = {
        0x17: 'IMPORT',
    }
    
    # Heuristic guesses based on frequency and patterns
    # IF directive is very common (appears in conditionals)
    # MACRO_USE is common
    # STR/SAVE less common
    
    print("\n\nLikely mappings (based on analysis):")
    print("-" * 60)
    
    for cb, name in sorted(known.items()):
        info = command_byte_counts.get(cb, {})
        count = info.get('count', 0)
        print(f"0x{cb:02X} = {name:<20} ({count} occurrences)")
    
    # Suggest likely candidates for other bytes
    print("\nOther command bytes (need manual verification):")
    for cb in sorted(command_byte_counts.keys()):
        if cb not in known:
            info = command_byte_counts[cb]
            print(f"0x{cb:02X} = ???              ({info['count']} occurrences in {len(info['files'])} files)")

def main():
    command_byte_counts = extract_command_bytes_from_all_files()
    correlate_with_directives(command_byte_counts)
    
    print("\n\nTo determine exact mappings:")
    print("1. Look at MEMMAP.Z80 source for directive keywords")
    print("2. Find corresponding 7F XX patterns in MEMMAP.I")
    print("3. Correlate byte XX with the directive keyword")
    print("4. Update decoder.rs with the mapping")

if __name__ == "__main__":
    main()
