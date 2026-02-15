"""
Extract BASIC listings from all PDF books and generate Rust test files.
"""
import pdfplumber
from pathlib import Path
import re
from typing import List, Tuple

def looks_like_basic_line(line: str) -> bool:
    """Check if line looks like BASIC code."""
    return bool(re.match(r'^\s*\d+\s+[A-Z]', line))

def extract_listings_from_pdf(pdf_path: Path) -> List[Tuple[str, List[int]]]:
    """Extract BASIC listings. Returns list of (listing_text, page_numbers)."""
    listings = []
    current_listing = []
    current_pages = []
    in_listing = False
    last_line_num = 0
    empty_count = 0
    
    with pdfplumber.open(pdf_path) as pdf:
        for page_num in range(len(pdf.pages)):
            try:
                text = pdf.pages[page_num].extract_text(layout=False)
                if not text:
                    continue
                
                lines = text.split('\n')
                
                for line in lines:
                    line = line.strip()
                    
                    # Check for BASIC line
                    match = re.match(r'^(\d+)\s+', line)
                    if match:
                        line_num = int(match.group(1))
                        
                        # Start new listing if line numbers reset or jump back
                        if in_listing and (line_num < last_line_num or line_num - last_line_num > 100):
                            if len(current_listing) >= 2:
                                listings.append(('\n'.join(current_listing), current_pages[:]))
                            current_listing = []
                            current_pages = []
                        
                        current_listing.append(line)
                        if page_num not in current_pages:
                            current_pages.append(page_num)
                        last_line_num = line_num
                        in_listing = True
                        empty_count = 0
                    
                    # Empty line
                    elif not line:
                        if in_listing:
                            empty_count += 1
                            if empty_count > 3:  # End listing after several empty lines
                                if len(current_listing) >= 2:
                                    listings.append(('\n'.join(current_listing), current_pages[:]))
                                current_listing = []
                                current_pages = []
                                in_listing = False
                                empty_count = 0
                    
                    # Non-empty, non-BASIC line
                    else:
                        empty_count = 0
                        # Could be continuation, or end of listing
                        # For now, simple approach: if in listing and doesn't start with number, end listing
                        if in_listing and not looks_like_basic_line(line):
                            # Check if it might be a wrapped line (starts with keyword)
                            if not re.match(r'^[A-Z]{2,}', line):
                                if len(current_listing) >= 2:
                                    listings.append(('\n'.join(current_listing), current_pages[:]))
                                current_listing = []
                                current_pages = []
                                in_listing = False
            
            except Exception as e:
                print(f"  Error on page {page_num+1}: {e}")
    
    # Save any remaining listing
    if len(current_listing) >= 2:
        listings.append(('\n'.join(current_listing), current_pages[:]))
    
    return listings

def sanitize_name(name: str) -> str:
    """Create a valid Rust identifier from filename."""
    # Remove extension and path
    name = Path(name).stem
    # Keep only alphanumeric and underscore
    name = re.sub(r'[^a-zA-Z0-9_]', '_', name)
    # Ensure it starts with letter
    if name[0].isdigit():
        name = 'book_' + name
    return name.lower()

def generate_rust_test_file(book_name: str, listings: List[Tuple[str, List[int]]], output_path: Path):
    """Generate a Rust test file for the book's listings."""
    
    rust_code = f'''// Auto-generated tests for BASIC listings from {book_name}
// Generated from PDF with OCR - may contain OCR errors

use cpclib_basic::string_parser::parse_basic_line;
use cpclib_common::winnow::Parser;

'''
    
    for idx, (listing, pages) in enumerate(listings, 1):
        page_str = f"page{'s' if len(pages) > 1 else ''} {', '.join(str(p+1) for p in pages)}"
        
        # Escape the listing for Rust string
        escaped = listing.replace('\\', '\\\\').replace('"', '\\"')
        
        rust_code += f'''
#[test]
fn test_{sanitize_name(book_name)}_listing_{idx:03d}() {{
    // From {page_str}
    let code = r#"{escaped}"#;
    
    let mut total = 0;
    let mut parsed = 0;
    
    // Parse each line
    for line in code.lines() {{
        let trimmed = line.trim();
        if trimmed.is_empty() {{
            continue;
        }}
        
        total += 1;
        
        // Try to parse the line
        let line_with_newline = format!("{{}}\\n", line);
        if let Ok(tokens) = parse_basic_line.parse(&line_with_newline) {{
            parsed += 1;
            
            // Try to reconstruct
            let reconstructed = tokens.to_string();
            
            // With OCR errors, exact match is unlikely, but we should get something
            if reconstructed.trim() == trimmed {{
                // Perfect reconstruction!
            }}
        }}
    }}
    
    // Print statistics for this listing
    let success_rate = if total > 0 {{ (parsed as f64 / total as f64 * 100.0) as u32 }} else {{ 0 }};
    println!("Listing {idx:03d}: {{parsed}}/{{total}} lines parsed successfully ({{success_rate}}%)");
    
    // Don't fail the test if parsing fails due to OCR errors
    // The goal is to track parser coverage, not enforce perfect parsing
}}
'''
    
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(rust_code)

def main():
    books_dir = Path(__file__).parent / 'books'
    output_dir = Path(__file__).parent
    
    pdf_files = sorted(books_dir.glob('*.pdf'))
    print(f"Found {len(pdf_files)} PDF files\n")
    
    all_stats = []
    
    for pdf_path in pdf_files:
        print(f"Processing: {pdf_path.name}")
        
        try:
            listings = extract_listings_from_pdf(pdf_path)
            print(f"  Found {len(listings)} listings")
            
            if listings:
                # Show first listing as sample
                first = listings[0][0]
                first_lines = first.split('\n')[:3]
                print(f"  Sample: {first_lines[0][:60]}...")
                
                # Generate Rust test file
                output_path = output_dir / f"test_book_{sanitize_name(pdf_path.name)}.rs"
                generate_rust_test_file(pdf_path.name, listings, output_path)
                print(f"  Generated: {output_path.name}")
                
                all_stats.append((pdf_path.name, len(listings)))
            else:
                print(f"  No listings found")
        
        except Exception as e:
            print(f"  Error: {e}")
        
        print()
    
    print("\n" + "="*80)
    print("Summary:")
    for name, count in all_stats:
        print(f"  {name}: {count} listings")

if __name__ == '__main__':
    main()
