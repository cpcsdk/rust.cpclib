"""
Extract BASIC listings from PDF books.

A BASIC listing consists of lines starting with line numbers.
Long lines may be split across physical lines.
Listings may span multiple pages.
"""

import pdfplumber
import re
from pathlib import Path
from typing import List, Dict

def is_line_number_start(text: str) -> bool:
    """Check if text starts with a line number (digits followed by space)."""
    return bool(re.match(r'^\s*\d+\s+\w', text))

def is_continuation_line(text: str) -> bool:
    """Check if line is a continuation (no line number, looks like BASIC code)."""
    # Check for BASIC keywords or operators at start
    basic_patterns = [
        r'^[A-Z]+\s*[\(\$:]',  # Keyword followed by ( or $ or :
        r'^[A-Z]+\s*,',         # Keyword followed by comma
        r'^\s*:',               # Colon continuation
        r'^THEN\s',             # THEN clause
        r'^ELSE\s',             # ELSE clause
        r'^GOTO\s',             # GOTO
        r'^TO\s',               # TO in FOR loop
        r'^STEP\s',             # STEP in FOR loop
    ]
    return any(re.match(pattern, text, re.IGNORECASE) for pattern in basic_patterns)

def clean_line(line: str) -> str:
    """Clean up OCR artifacts in a line."""
    # Remove common OCR mistakes
    line = line.replace('~', '-')
    line = line.replace('_', ' ')
    # Normalize whitespace
    line = re.sub(r'\s+', ' ', line)
    return line.strip()

def extract_listings_from_pdf(pdf_path: Path) -> List[str]:
    """Extract all BASIC listings from a PDF."""
    listings = []
    current_listing = []
    current_line = ""
    in_listing = False
    empty_line_count = 0
    
    with pdfplumber.open(pdf_path) as pdf:
        for page_num, page in enumerate(pdf.pages, 1):
            text = page.extract_text()
            if not text:
                continue
            
            lines = text.split('\n')
            
            for line in lines:
                line_stripped = line.strip()
                
                # Check if this line starts a new BASIC line (has line number)
                if is_line_number_start(line_stripped):
                    # Save previous line if any
                    if current_line:
                        current_listing.append(clean_line(current_line))
                        current_line = ""
                    
                    # Start new line
                    current_line = line_stripped
                    in_listing = True
                    empty_line_count = 0
                
                # Continuation of current line (no line number, but looks like BASIC)
                elif in_listing and line_stripped and is_continuation_line(line_stripped):
                    current_line += " " + line_stripped
                    empty_line_count = 0
                
                # Empty line
                elif in_listing and not line_stripped:
                    empty_line_count += 1
                    # Two consecutive empty lines end a listing
                    if empty_line_count >= 2:
                        if current_line:
                            current_listing.append(clean_line(current_line))
                        
                        if len(current_listing) >= 2:  # Minimum 2 lines to be a listing
                            listings.append('\n'.join(current_listing))
                        
                        current_listing = []
                        current_line = ""
                        in_listing = False
                        empty_line_count = 0
                
                # Some other text while in listing - might be end
                elif in_listing and line_stripped:
                    # Check if it looks like regular text (starts with capital followed by lowercase)
                    if re.match(r'^[A-Z][a-z]{3,}', line_stripped):
                        # Probably end of listing
                        if current_line:
                            current_listing.append(clean_line(current_line))
                        
                        if len(current_listing) >= 2:
                            listings.append('\n'.join(current_listing))
                        
                        current_listing = []
                        current_line = ""
                        in_listing = False
                        empty_line_count = 0
                    
    # Handle any remaining content
    if current_line:
        current_listing.append(clean_line(current_line))
    
    if len(current_listing) >= 2:
        listings.append('\n'.join(current_listing))
    
    return listings

def main():
    books_dir = Path(__file__).parent / 'books'
    output_dir = Path(__file__).parent
    
    print(f"Looking for PDFs in: {books_dir}")
    
    pdf_files = sorted(books_dir.glob('*.pdf'))
    
    for pdf_path in pdf_files:
        print(f"\nProcessing: {pdf_path.name}")
        
        try:
            listings = extract_listings_from_pdf(pdf_path)
            print(f"  Found {len(listings)} potential listings")
            
            # Show first few characters of each listing
            for i, listing in enumerate(listings[:5], 1):
                first_line = listing.split('\n')[0][:60]
                print(f"    {i}. {first_line}...")
            
            if len(listings) > 5:
                print(f"    ... and {len(listings) - 5} more")
                
        except Exception as e:
            print(f"  Error: {e}")

if __name__ == '__main__':
    main()
