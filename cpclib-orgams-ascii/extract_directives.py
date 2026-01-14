#!/usr/bin/env python3
"""
Extract directive patterns from PARSE.Z80 to systematize decoding.

This script parses the t_XXX directive definitions in PARSE.Z80 to extract:
- Directive keywords (from ps_litt patterns)
- Command codes (ec2_XXX values)
- Parameter types (expression, string, etc.)
- Generate Rust code for systematic decoding
"""

import re
import sys
from pathlib import Path

def extract_directive_patterns(parse_z80_content):
    """Extract all t_XXX directive patterns from PARSE.Z80"""
    
    directives = []
    lines = parse_z80_content.split('\n')
    
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        
        # Look for t_XXX labels (directive definitions)
        if re.match(r'^t_[a-z_]+\s*$', line):
            directive_name = line.strip()
            directive_data = {
                'name': directive_name,
                'keyword': None,
                'command_code': None,
                'has_expression': False,
                'line_start': i
            }
            
            # Scan ahead to collect the directive pattern
            i += 1
            pattern_lines = []
            while i < len(lines) and not re.match(r'^t_[a-z_]+\s*$', lines[i]):
                pattern_lines.append(lines[i])
                i += 1
                # Stop at next directive or after reasonable distance
                if len(pattern_lines) > 50:
                    break
            
            # Parse pattern lines
            for pline in pattern_lines:
                # Extract keyword from ps_litt
                # Example: BYTE ps_litt,6,"import"
                match = re.search(r'ps_litt\s*,\s*(\d+)\s*,\s*"([^"]+)"', pline)
                if match:
                    directive_data['keyword'] = match.group(2)
                
                # Extract command code from f_emit_inc,ec2_XXX
                # Example: BYTE f_emit_inc,ec2_import
                match = re.search(r'f_emit_inc\s*,\s*ec2_([a-z_]+)', pline)
                if match:
                    directive_data['command_code'] = f'ec2_{match.group(1)}'
                
                # Extract from ps_littenum_emit list (multi-directive format)
                # Example: BYTE 6,"import",ec2_import
                match = re.search(r'BYTE\s+\d+\s*,\s*"([^"]+)"\s*,\s*ec2_([a-z_]+)', pline)
                if match:
                    # This is a list entry - create separate directive
                    list_directive = {
                        'name': directive_name,
                        'keyword': match.group(1),
                        'command_code': f'ec2_{match.group(2)}',
                        'has_expression': 't_exp' in '\n'.join(pattern_lines),
                        'line_start': i
                    }
                    directives.append(list_directive)
                
                # Check if it has expression encoding
                if 'f_exp_init' in pline or 'f_exp_encode' in pline or 't_exp' in pline:
                    directive_data['has_expression'] = True
            
            # Only add if we found a keyword and command code (for main directive)
            if directive_data['keyword'] and directive_data['command_code']:
                directives.append(directive_data)
            
            continue
        
        i += 1
    
    return directives

def extract_test_command_codes(parse_z80_content):
    """Extract command byte values from test expectations"""
    
    # Look for patterns like: BYTE ec_esc,ec2_XXX
    # followed by expected encoded bytes in tests
    
    command_codes = {}
    lines = parse_z80_content.split('\n')
    
    for i, line in enumerate(lines):
        # Look for test patterns with ec2_ references
        match = re.search(r'BYTE\s+ec_esc\s*,\s*ec2_([a-z_]+)', line)
        if match:
            cmd_name = f'ec2_{match.group(1)}'
            # Try to find the actual byte value in nearby lines
            # This is tricky - values may be defined elsewhere
            # For now, just note we found this command
            if cmd_name not in command_codes:
                command_codes[cmd_name] = None
    
    # Known values from manual inspection
    known_values = {
        'ec2_import': 0x17,
        # Add more as we discover them
    }
    
    command_codes.update(known_values)
    return command_codes

def generate_rust_directive_decoder(directives, command_codes):
    """Generate Rust code for systematic directive decoding"""
    
    rust_code = """
// Auto-generated directive decoder mappings
// Extracted from PARSE.Z80 t_XXX directive patterns

pub struct DirectiveInfo {
    pub keyword: &'static str,
    pub command_code: u8,
    pub has_expression: bool,
}

pub const DIRECTIVE_MAP: &[(u8, DirectiveInfo)] = &[
"""
    
    for directive in directives:
        cmd_code = directive['command_code']
        code_value = command_codes.get(cmd_code, 0)
        
        if code_value:
            rust_code += f"""    (0x{code_value:02X}, DirectiveInfo {{
        keyword: "{directive['keyword'].upper()}",
        command_code: 0x{code_value:02X},
        has_expression: {str(directive['has_expression']).lower()},
    }}),
"""
    
    rust_code += """
];

pub fn get_directive_info(command_byte: u8) -> Option<&'static DirectiveInfo> {
    DIRECTIVE_MAP.iter()
        .find(|(code, _)| *code == command_byte)
        .map(|(_, info)| info)
}
"""
    
    return rust_code

def main():
    # Find PARSE.Z80
    parse_z80_path = Path(__file__).parent / "tests/orgams-main/orgext/PARSE.Z80"
    
    if not parse_z80_path.exists():
        print(f"Error: PARSE.Z80 not found at {parse_z80_path}", file=sys.stderr)
        sys.exit(1)
    
    print(f"Reading {parse_z80_path}...")
    parse_z80_content = parse_z80_path.read_text(encoding='latin-1')
    
    print("Extracting directive patterns...")
    directives = extract_directive_patterns(parse_z80_content)
    
    print(f"\nFound {len(directives)} directives with patterns:")
    for d in directives:
        print(f"  {d['name']:20} → {d['keyword']:10} ({d['command_code']})")
    
    print("\nExtracting command codes from tests...")
    command_codes = extract_test_command_codes(parse_z80_content)
    
    print(f"\nFound {len(command_codes)} command codes:")
    for name, value in sorted(command_codes.items()):
        if value:
            print(f"  {name:20} = 0x{value:02X}")
        else:
            print(f"  {name:20} = <unknown>")
    
    print("\nGenerating Rust directive decoder...")
    rust_code = generate_rust_directive_decoder(directives, command_codes)
    
    output_path = Path(__file__).parent / "src/directive_map.rs"
    output_path.write_text(rust_code)
    
    print(f"\nGenerated {output_path}")
    print("\nNext steps:")
    print("1. Review the generated directive_map.rs")
    print("2. Extract actual command byte values from .O/.I files or assembly")
    print("3. Integrate into decoder.rs for systematic directive handling")

if __name__ == "__main__":
    main()
