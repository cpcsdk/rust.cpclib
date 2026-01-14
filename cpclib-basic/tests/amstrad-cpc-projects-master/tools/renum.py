import sys
import re

def renumber_basic_file(input_file, output_file=None, start_line=10, old_start=0, increment=10):
    if not output_file:
        output_file = input_file
    # Read input file
    try:
        with open(input_file, 'r') as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"Error: File '{input_file}' not found")
        sys.exit(1)

    # Parse lines and collect current line numbers
    line_numbers = []
    new_lines = []
    for line in lines:
        match = re.match(r'^\s*(\d+)\s*(.*)$', line.strip())
        if match:
            num = int(match.group(1))
            if num >= old_start:
                line_numbers.append(num)
            new_lines.append((num, match.group(2)))
        else:
            new_lines.append((None, line.strip()))  # Non-numbered line (e.g., comment)

    # Create mapping of old to new line numbers
    number_map = {}
    new_num = start_line
    for old_num in sorted([n for n in line_numbers if n >= old_start]):
        number_map[old_num] = new_num
        new_num += increment

    # Process lines, updating references
    output_lines = []
    for num, code in new_lines:
        if num is None or num < old_start:
            output_lines.append(code)
            continue
        # Update line number
        new_num = number_map.get(num, num)
        # Update references in code (GOTO, GOSUB, THEN, etc.)
        new_code = code
        # Regex for all line-referencing commands
        ref_commands = r'\b(GOTO|GOSUB|THEN|ELSE|RESTORE)\s+(\d+)\b|\bON\b\s+[^:]+?\s+(GOTO|GOSUB)\s+(\d+(?:\s*,\s*\d+)*)'
        matches = re.findall(ref_commands, code, re.IGNORECASE)
        for match in matches:
            if match[0]:  # Single-number commands (GOTO, GOSUB, THEN, ELSE, RESTORE)
                ref_num = int(match[1])
                if ref_num in number_map:
                    new_code = re.sub(
                        r'\b(' + str(ref_num) + r')\b(?=(?!\w))',
                        str(number_map[ref_num]),
                        new_code
                    )
            elif match[2]:  # ON ... GOTO/GOSUB with number list
                command = match[2]
                number_list = match[3]
                # Get numbers and map to new numbers
                numbers = re.findall(r'\d+', number_list)
                new_numbers = [str(number_map[int(num)]) if int(num) in number_map else num for num in numbers]
                # Replace entire ON statement
                pattern = re.escape(match[2] + ' ' + number_list)
                new_code = re.sub(
                    pattern,
                    f"{command} {','.join(new_numbers)}",
                    new_code
                )
        output_lines.append(f"{new_num} {new_code}")

    # Write to output file
    try:
        with open(output_file, 'w') as f:
            for line in output_lines:
                f.write(line + '\n')
        print(f"Renumbered file saved as '{output_file}'")
    except IOError:
        print(f"Error: Could not write to '{output_file}'")
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python renum.py input.bas [output.bas] [start_line] [old_start] [increment]")
        sys.exit(1)
    input_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else None
    start_line = int(sys.argv[3]) if len(sys.argv) > 3 else 10
    old_start = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    increment = int(sys.argv[5]) if len(sys.argv) > 5 else 10
    renumber_basic_file(input_file, output_file, start_line, old_start, increment)