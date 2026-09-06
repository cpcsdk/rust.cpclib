#!/usr/bin/env python3
"""
Extract BASIC code listings from PDF file.
BASIC listings are identified by lines starting with line numbers.
"""

import pdfplumber
import re
from typing import List, Dict, Tuple
from dataclasses import dataclass

@dataclass
class BasicListing:
    """Represents a BASIC code listing."""
    page_numbers: List[int]
    code: str
    start_line_num: int
    end_line_num: int

def is_line_number_start(line: str) -> bool:
    """Check if a line starts with a BASIC line number."""
    line = line.strip()
    if not line:
        return False
    # Check if starts with digits followed by space or tab
    match = re.match(r'^(\d+)\s+', line)
    return match is not None

def extract_line_number(line: str) -> int:
    """Extract the line number from a BASIC line."""
    match = re.match(r'^(\d+)\s+', line.strip())
    if match:
        return int(match.group(1))
    return -1

def is_continuation_line(line: str) -> bool:
    """Check if a line is a continuation of the previous line (doesn't start with number)."""
    line = line.strip()
    if not line:
        return False
    # If it doesn't start with a digit, it might be a continuation
    return not re.match(r'^\d+\s+', line)

def extract_basic_listings(pdf_path: str) -> List[BasicListing]:
    """Extract all BASIC listings from a PDF file."""
    listings = []
    current_listing_lines = []
    current_listing_pages = []
    in_listing = False
    
    with pdfplumber.open(pdf_path) as pdf:
        for page_num, page in enumerate(pdf.pages, start=1):
            text = page.extract_text()
            if not text:
                continue
            
            lines = text.split('\n')
            
            for line in lines:
                stripped = line.strip()
                
                if is_line_number_start(stripped):
                    # This is a BASIC line
                    if not in_listing:
                        # Start a new listing
                        in_listing = True
                        current_listing_lines = [stripped]
                        current_listing_pages = [page_num]
                    else:
                        # Continue current listing
                        current_listing_lines.append(stripped)
                        if page_num not in current_listing_pages:
                            current_listing_pages.append(page_num)
                
                elif in_listing and is_continuation_line(stripped) and stripped:
                    # This might be a wrapped line - append to previous line
                    if current_listing_lines:
                        current_listing_lines[-1] += ' ' + stripped
                
                elif in_listing and not stripped:
                    # Empty line might indicate end of listing, but let's be lenient
                    continue
                
                elif in_listing and not is_line_number_start(stripped):
                    # Non-BASIC line encountered, end the current listing
                    if current_listing_lines:
                        # Save the listing
                        code = '\n'.join(current_listing_lines)
                        line_nums = [extract_line_number(l) for l in current_listing_lines]
                        line_nums = [n for n in line_nums if n > 0]
                        
                        if line_nums:  # Only save if we have valid line numbers
                            listings.append(BasicListing(
                                page_numbers=current_listing_pages.copy(),
                                code=code,
                                start_line_num=min(line_nums),
                                end_line_num=max(line_nums)
                            ))
                        
                        current_listing_lines = []
                        current_listing_pages = []
                    in_listing = False
        
        # Don't forget the last listing if we're still in one
        if in_listing and current_listing_lines:
            code = '\n'.join(current_listing_lines)
            line_nums = [extract_line_number(l) for l in current_listing_lines]
            line_nums = [n for n in line_nums if n > 0]
            
            if line_nums:
                listings.append(BasicListing(
                    page_numbers=current_listing_pages.copy(),
                    code=code,
                    start_line_num=min(line_nums),
                    end_line_num=max(line_nums)
                ))
    
    return listings

def main():
    pdf_path = r"c:\Users\giotr\Perso\CPC\rust.cpcdemotools\cpclib-basic\tests\books\AMSOFT_AMSTRAD_BASIC_Initiation_au_Basic_AMSTRAD_Partie1_SOFT411[OCR].pdf"
    
    print("Extracting BASIC listings from PDF...")
    listings = extract_basic_listings(pdf_path)
    
    print(f"\nFound {len(listings)} BASIC listings:\n")
    print("=" * 80)
    
    for i, listing in enumerate(listings, start=1):
        print(f"\n### Listing {i} ###")
        print(f"Pages: {', '.join(map(str, listing.page_numbers))}")
        print(f"Line numbers: {listing.start_line_num} - {listing.end_line_num}")
        print(f"Lines of code: {len(listing.code.split(chr(10)))}")
        print("\nCode:")
        print("-" * 80)
        print(listing.code)
        print("-" * 80)

if __name__ == "__main__":
    main()
