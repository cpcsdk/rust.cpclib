"""
Generate Rust tests for BASIC listings from PDF books.
Each test parses a complete program with OCR error correction using a macro.
"""
import pdfplumber
from pathlib import Path
import re
from typing import List, Tuple

assert False, "This file is only for context and should not be run."

def looks_like_basic_line(line: str) -> bool:
    return bool(re.match(r'^\s*\d+\s+[A-Z]', line))

def extract_listings_from_pdf(pdf_path: Path) -> List[Tuple[str, List[int]]]:
    """Extract BASIC listings from PDF, handling line continuations."""
    listings = []
    current_listing = []
    current_pages = []
    in_listing = False
    last_line_num = 0
    max_line_num = 0
    empty_count = 0
    
    with pdfplumber.open(pdf_path) as pdf:
        for page_num in range(len(pdf.pages)):
            try:
                text = pdf.pages[page_num].extract_text(layout=False)
                if not text:
                    continue
                
                lines = text.split('\n')
                i = 0
                
                while i < len(lines):
                    line = lines[i].strip()
                    
                    match = re.match(r'^(\d+)\s+', line)
                    if match:
                        line_num = int(match.group(1))
                        
                        if in_listing and (line_num < last_line_num or line_num - last_line_num > 100):
                            # Save listing only if max line number >= 10
                            if len(current_listing) >= 2 and max_line_num >= 10:
                                listings.append(('\n'.join(current_listing), current_pages[:]))
                            current_listing = []
                            current_pages = []
                            max_line_num = 0
                        
                        # Collect the complete BASIC line (including continuations)
                        complete_line = line
                        j = i + 1
                        
                        # Check if next physical lines are continuations (no line number)
                        while j < len(lines):
                            next_line = lines[j].strip()
                            if not next_line:
                                # Empty line might be end or just spacing
                                j += 1
                                break
                            elif re.match(r'^\d+\s+', next_line):
                                # Next BASIC line starts, stop continuation
                                break
                            else:
                                # This is a continuation line
                                complete_line += ' ' + next_line
                                j += 1
                        
                        current_listing.append(complete_line)
                        if page_num not in current_pages:
                            current_pages.append(page_num)
                        last_line_num = line_num
                        max_line_num = max(max_line_num, line_num)
                        in_listing = True
                        empty_count = 0
                        i = j  # Skip the continuation lines we processed
                        continue
                    
                    elif not line:
                        if in_listing:
                            empty_count += 1
                            if empty_count > 3:
                                if len(current_listing) >= 2 and max_line_num >= 10:
                                    listings.append(('\n'.join(current_listing), current_pages[:]))
                                current_listing = []
                                current_pages = []
                                max_line_num = 0
                                in_listing = False
                                empty_count = 0
                    else:
                        empty_count = 0
                        if in_listing and not looks_like_basic_line(line):
                            if not re.match(r'^[A-Z]{2,}', line):
                                if len(current_listing) >= 2 and max_line_num >= 10:
                                    listings.append(('\n'.join(current_listing), current_pages[:]))
                                current_listing = []
                                current_pages = []
                                max_line_num = 0
                                in_listing = False
                    
                    i += 1
                    
            except Exception as e:
                pass
    
    # Save any remaining listing only if max line number >= 10
    if len(current_listing) >= 2 and max_line_num >= 10:
        listings.append(('\n'.join(current_listing), current_pages[:]))
    
    # Merge listings across consecutive pages if line numbers are close
    merged_listings = []
    i = 0
    while i < len(listings):
        current_text, current_pages = listings[i]
        
        # Get last line number of current listing
        current_lines = current_text.split('\n')
        last_match = None
        for line in reversed(current_lines):
            match = re.match(r'^(\d+)\s+', line)
            if match:
                last_match = match
                break
        
        if last_match and i + 1 < len(listings):
            last_line_num = int(last_match.group(1))
            next_text, next_pages = listings[i + 1]
            
            # Get first line number of next listing
            next_lines = next_text.split('\n')
            first_match = None
            for line in next_lines:
                match = re.match(r'^(\d+)\s+', line)
                if match:
                    first_match = match
                    break
            
            if first_match:
                first_line_num = int(first_match.group(1))
                
                # Check if pages are consecutive and line numbers are close
                pages_consecutive = (max(current_pages) + 1) == min(next_pages)
                line_diff = first_line_num - last_line_num
                
                if pages_consecutive and 0 < line_diff <= 10:
                    # Merge the two listings
                    merged_text = current_text + '\n' + next_text
                    merged_pages = current_pages + next_pages
                    merged_listings.append((merged_text, merged_pages))
                    i += 2  # Skip both listings
                    continue
        
        merged_listings.append((current_text, current_pages))
        i += 1
    
    return merged_listings

def sanitize_name(name: str) -> str:
    name = Path(name).stem
    name = re.sub(r'[^a-zA-Z0-9_]', '_', name)
    if name[0].isdigit():
        name = 'book_' + name
    return name.lower()

def generate_rust_test_file(book_name: str, listings: List[Tuple[str, List[int]]], output_path: Path):
    """Generate Rust test file with macro-based tests."""
    
    rust_code = f'''// Tests for BASIC listings from {book_name}
// Each test parses a complete program with OCR error correction

mod common_book_tests;
use common_book_tests::test_basic_program;

/// Fix common OCR errors specific to this book
fn fix_ocr_errors(code: &str) -> String {{
    code
        .replace("DRAM ", "DRAW ")
        .replace("DRAw ", "DRAW ")
        .replace("oRAw ", "DRAW ")
        .replace("DRAl›l ", "DRAW ")
        .replace("DRAH ", "DRAW ")
        .replace("DRAN ", "DRAW ")
        .replace("HOVE ", "MOVE ")
        .replace("Move ", "MOVE ")
        .replace("HODE ", "MODE ")
        .replace("REN ", "REM ")
        .replace("PR I NT", "PRINT")
        .replace("'", "'")
        .replace("■", "-")
        .replace("›", "")
        .replace("¿", "")
        .replace("l>", "D")
        .replace("t›", "D")
        .replace("MOVE100", "MOVE 100")
        .replace("PLOT×", "PLOT x")
        .replace("INPUT\\"", "INPUT \\"")
        .replace("PRINT\\"", "PRINT \\"")
}}

/// Macro to generate test functions for this book
macro_rules! basic_test {{
    ($name:ident, $program:expr, $desc:expr) => {{
        #[test]
        fn $name() {{
            let fixed = fix_ocr_errors($program);
            test_basic_program(&fixed, $desc, stringify!($name));
        }}
    }};
}}

'''
    
    for idx, (listing, pages) in enumerate(listings, 1):
        page_str = f"page{'s' if len(pages) > 1 else''} {', '.join(str(p+1) for p in pages)}"
        
        # Filter out lines with line numbers < 10
        filtered_lines = []
        for line in listing.split('\n'):
            match = re.match(r'^(\d+)\s+', line)
            if match:
                line_num = int(match.group(1))
                if line_num >= 10:
                    filtered_lines.append(line)
            else:
                # Keep non-numbered lines (shouldn't happen in listings, but be safe)
                if line.strip():
                    filtered_lines.append(line)
        
        if not filtered_lines:
            continue  # Skip if no lines remain
        
        program_text = '\n'.join(filtered_lines)
        
        # Always use 10 hashes for raw string delimiter
        hash_level = '##########'
        
        test_name = f"test_{sanitize_name(book_name)}_listing_{idx:03d}"
        
        rust_code += f'''basic_test!({test_name}, r{hash_level}"{program_text}"{hash_level}, "{page_str}");
'''
    
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(rust_code)

def main():
    books_dir = Path(__file__).parent / 'books'
    output_dir = Path(__file__).parent
    
    pdf_files = sorted(books_dir.glob('*.pdf'))
    print(f"Found {len(pdf_files)} PDF files\n")
    
    total_listings = 0
    
    for pdf_path in pdf_files:
        print(f"Processing: {pdf_path.name}")
        listings = extract_listings_from_pdf(pdf_path)
        print(f"  Found {len(listings)} listings")
        
        if listings:
            output_path = output_dir / f"test_book_{sanitize_name(pdf_path.name)}.rs"
            generate_rust_test_file(pdf_path.name, listings, output_path)
            print(f"  Generated: {output_path.name}\n")
            total_listings += len(listings)
    
    print(f"\nTotal: {total_listings} listings from {len(pdf_files)} books")

if __name__ == '__main__':
    main()
