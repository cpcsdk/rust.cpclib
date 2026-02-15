"""
Simple script to examine a PDF and find BASIC listings
"""
import pdfplumber
from pathlib import Path
import re

def looks_like_basic_line(line):
    """Check if line looks like BASIC code (starts with line number + keyword)."""
    match = re.match(r'^\s*(\d+)\s+([A-Z]+)', line)
    if match:
        keywords = ['REM', 'PRINT', 'FOR', 'NEXT', 'IF', 'THEN', 'GOTO', 'GOSUB', 
                   'INPUT', 'LET', 'DIM', 'READ', 'DATA', 'RESTORE', 'CLS', 'PLOT',
                   'DRAW', 'MODE', 'INK', 'BORDER', 'PEN', 'PAPER', 'WINDOW', 'LOCATE']
        keyword = match.group(2)
        return keyword in keywords
    return False

pdf_path = Path(__file__).parent / 'books' / 'AMSOFT_AMSTRAD_BASIC_Initiation_au_Basic_AMSTRAD_Partie1_SOFT411[OCR].pdf'

with pdfplumber.open(pdf_path) as pdf:
    print(f"PDF has {len(pdf.pages)} pages\n")
    
    # Scan all pages looking for BASIC code
    pages_with_code = []
    for i in range(len(pdf.pages)):
        try:
            text = pdf.pages[i].extract_text(layout=False)
            if text:
                lines = text.split('\n')
                basic_lines = [line for line in lines if looks_like_basic_line(line)]
                if len(basic_lines) >= 2:  # At least 2 BASIC lines
                    pages_with_code.append(i)
        except Exception as e:
            pass
    
    print(f"Found pages with BASIC code: {pages_with_code[:20]}")
    print()
    
    # Show first few pages with code
    for page_idx in pages_with_code[:3]:
        print(f"\n{'='*80}")
        print(f"PAGE {page_idx+1}")
        print('='*80)
        try:
            text = pdf.pages[page_idx].extract_text(layout=False)
            if text:
                lines = text.split('\n')
                for line in lines:
                    if line.strip() and (line.strip()[0].isdigit() or looks_like_basic_line(line)):
                        print(line)
        except:
            print("Error")
