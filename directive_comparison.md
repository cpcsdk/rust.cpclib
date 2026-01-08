# BASM Directive Comparison with Other Z80 Assemblers

## Current BASM Directives Inventory

### Data Definition Directives
- **DB/DEFB/DEFM/BYTE/TEXT** - Define byte(s)
- **DW/DEFW/WORD** - Define word(s)
- **DS/DEFS/FILL/RMEM** - Define space/reserve memory
- **STR** - Define string
- **ABYTE** - Define aligned byte

### Organization & Addressing
- **ORG** - Set origin address
- **ALIGN** - Align to boundary
- **MAP** - Set memory map
- **RORG** - Relocatable origin
- **SKIP** - Skip bytes (ORGAMS)
- **LIMIT** - Set assembly limit

### Banking & Paging
- **BANK** - Configure bank (GATE ARRAY value)
- **BANKSET** - Set bank
- **RANGE/DEFSECTION** - Define named section in page
- **SECTION** - Define section

### Conditional Assembly
- **IF** - Conditional assembly
- **IFNOT** - Negated condition
- **IFDEF** - If symbol defined
- **IFNDEF** - If symbol not defined
- **IFUSED/IFEXIST** - If symbol used
- **IFNUSED** - If symbol not used
- **ENDIF/END** - End conditional

### Loop & Iteration Directives
- **FOR** ... **ENDFOR/FEND/ENDF** - For loop with counter
- **REPEAT/REPT/REP** ... **ENDREPEAT/ENDREPT/ENDREP/ENDR/REND** - Repeat n times
- **REPEAT** ... **UNTIL** - Repeat until condition
- **WHILE** ... **ENDWHILE/ENDW/WEND** - While loop
- **ITERATE/ITER** ... **ENDITERATE/ENDITER/ENDI/IEND** - Iterate over list
- **SWITCH** ... **CASE** ... **DEFAULT** ... **ENDSWITCH/ENDS** - Switch/case statement
- **BREAK** - Break from switch case

### Macro & Function Directives
- **MACRO** ... **ENDMACRO/ENDM/MEND** - Define macro
- **FUNCTION** ... **ENDFUNCTION/ENDF** - Define function
- **RETURN** - Return value from function
- **STRUCT** ... **ENDSTRUCT/ENDS** - Define structure

### File Inclusion
- **INCLUDE/READ/IMPORT** - Include source file
- **INCBIN/BINCLUDE** - Include binary file
- **INCL48** - Include LZ48 compressed binary
- **INCL49** - Include LZ49 compressed binary
- **INCEXO** - Include LZ Exomizer compressed
- **INCLZ4** - Include LZ4 compressed
- **INCZX0** - Include ZX0 compressed
- **INCZX0_BACKWARD** - Include backward ZX0
- **INCZX7** - Include ZX7 compressed (alias)
- **INCAPU** - Include APU compressed
- **INCLZSA1** - Include LZSA1 compressed
- **INCLZSA2** - Include LZSA2 compressed
- **INCSHRINKLER** - Include Shrinkler compressed
- **INCUPKR** - Include UPKR compressed

### Crunching Directives
- **LZEXO/LZ4/LZ48/LZ49/LZX7/LZX0/LZX0_BACKWARD/LZAPU/LZSA1/LZSA2/LZSHRINKLER/LZUPKR** ... **LZCLOSE** - Crunch section

### Symbol Management
- **EQU** - Define constant
- **=** (Assign) - Define/redefine symbol
- **LET** - Define variable
- **UNDEF** - Undefine symbol
- **EXPORT** - Export symbols
- **NOEXPORT** - Don't export symbols

### Output Control
- **SAVE/WRITE** - Save binary file
- **BUILDSNA** - Build snapshot
- **BUILDCPR** - Build cartridge
- **RUN/ENT** - Set entry point

### CPC-Specific
- **SETCPC** - Set CPC model
- **SETCRTC** - Set CRTC type
- **CHARSET** - Define charset encoding
- **LOCOMOTIVE** ... **ENDLOCOMOTIVE** - Embed BASIC code
- **BASIC** - BASIC code inclusion
- **SNAINIT** - Initialize snapshot flags
- **SNASET** - Set snapshot flag value
- **PROTECT** - Protect memory range

### Debug & Development
- **ASSERT** - Runtime assertion
- **FAIL** - Fail assembly with message
- **BREAKPOINT/BRK** - Set breakpoint
- **PRINT** - Print at assembly time
- **LIST/NOLIST** - Control listing output
- **WAITNOPS** - Insert NOPs for timing
- **TICKER** - Stable ticker for cycle counting
- **PAUSE** - Pause assembly

### Structural Directives
- **MODULE** ... **ENDMODULE** - Define module/namespace
- **CONFINED** ... **ENDCONFINED/CEND/ENDC** - Confined scope
- **ASMCONTROL/ASMCONTROLENV** - Assembler control environment

### Special
- **NOP** - No operation (directive, not opcode)
- **END** - End of assembly
- **STARTINGINDEX** - Set starting index for local labels

---

## Comparison with Popular Z80 Assemblers

### RASM (Roudoudou's Assembler)

**RASM directives that BASM has:**
- ORG, ALIGN, DB, DW, DS
- IF/IFDEF/IFNDEF/ELSE/ENDIF
- REPEAT/REPT/ENDREPEAT
- MACRO/ENDMACRO
- INCLUDE, INCBIN
- SAVE, BUILDSNA, BUILDCPR
- BANK (but incompatible behavior)
- PROTECT

**RASM directives MISSING in BASM:**
1. **LOCAL** - Define local labels within macros (basm uses @ prefix instead)
2. **DEVICE** - Specify target device (AMSTRADCPC464, ZX128K, etc.) - BASM has SETCPC
3. **DISPLAY** - Alternative to PRINT
4. **PAGE** - Page directive for paging (different from BASM's BANK)
5. **SLOT** - Slot directive for banking
6. **PHASE/DEPHASE** - Assemble code for execution at different address (BASM has RORG)
7. **SAVEBIN** - Save binary with different parameters
8. **SETCPC** - In RASM this sets memory configuration (BASM has it but different meaning)
9. **CRUNCHEDBASIC** - Crunched BASIC (BASM has LOCOMOTIVE/BASIC)
10. **MAXI** - Maximum value directive
11. **PUSH/POP** - Save/restore assembly context (ORG, $)
12. **SOUND** - Sound generation directives

**Analysis:** RASM focuses more on multi-platform support and has different banking model. BASM is more CPC-specific.

### SJASMPLUS (Sjasm Plus)

**SjasmPlus directives that BASM has:**
- ORG, ALIGN, DB/DEFB, DW/DEFW, DS/DEFS
- IF/IFDEF/IFNDEF/ELSE/ENDIF
- REPT/ENDREPEAT
- MACRO/ENDM
- INCLUDE, INCBIN
- MODULE/ENDMODULE
- STRUCT/ENDSTRUCT

**SjasmPlus directives MISSING in BASM:**
1. **DEVICE** - Target device specification (ZX128, ZXSPECTRUM48, NOSLOT64K, etc.)
2. **PAGE** - Memory page selection
3. **SLOT** - Memory slot definition
4. **MMU** - Memory management unit control
5. **DISP/ENT** - Displacement/entry (similar to PHASE/DEPHASE)
6. **DEVICE ZXSPECTRUM48/128/NEXT** - Platform-specific device types
7. **SAVEDEV** - Save device-specific format
8. **SAVESNA, SAVETAP, SAVEHOB** - Format-specific save directives (BASM has BUILDSNA)
9. **TAPOUT, TAPEND** - TAP file generation
10. **FPOS** - Get file position
11. **INCZ80, INCHOB** - Include Z80/HOB compressed
12. **DEVICE AMSTRADCPC464/6128** - CPC device specs (BASM has SETCPC but different)
13. **OUTPUT** - Set output filename
14. **LABELSLIST** - Generate labels list
15. **LUA** - Embedded Lua scripting
16. **DUP/EDUP** - Duplicate block (BASM has REPEAT)
17. **ASSERT with formatting** - More advanced assertions
18. **UNDEFINE** - Undefine macro (BASM has UNDEF for symbols)

**Analysis:** SjasmPlus is very feature-rich with multi-platform support, Lua scripting, and advanced memory management. BASM focuses on CPC-specific features.

### VASM (Volker Barthelmann Assembler)

**VASM directives that BASM has:**
- ORG, ALIGN, DC.B/DB, DC.W/DW, DS
- IF/ELSE/ENDIF, IFDEF/IFNDEF
- REPT/ENDR
- MACRO/ENDM
- INCLUDE, INCBIN

**VASM directives MISSING in BASM:**
1. **SECTION** (different meaning - VASM uses it for code/data/bss sections)
2. **OFFSET** - Define offset section
3. **ORG with different syntax** - VASM supports `ORG addr,offset`
4. **EVEN** - Align to even address (BASM uses ALIGN 2)
5. **CNOP** - Conditional NOP alignment
6. **RSRESET, RSSET** - Reset/set structure offset counter (BASM has STRUCT)
7. **RS.B, RS.W, RS.L** - Reserve space in structure
8. **FO.B, FO.W, FO.L** - Fixed offset in structure
9. **FAIL** with condition - BASM has FAIL but different
10. **PRINTT, PRINTV** - Print text/value
11. **PUBLIC, XDEF, XREF** - Symbol visibility (BASM has EXPORT/NOEXPORT)
12. **WEAK** - Weak symbol definition
13. **NEAR, FAR** - Address mode hints
14. **IFC, IFNC** - String comparison conditionals
15. **IFEQ, IFNE, IFGT, IFGE, IFLT, IFLE** - Numeric comparison conditionals (BASM has IF with expressions)
16. **IFMACROD, IFMACROND** - Macro defined conditionals
17. **ELSEIF, ELIFC** - Else-if constructs (BASM doesn't have ELSEIF)

**Analysis:** VASM is very portable and standards-compliant. BASM has richer expression evaluation in IF.

### BRASS (Benryves Assembler)

**BRASS directives that BASM has:**
- ORG, ALIGN, DB/DEFB, DW/DEFW, DS/DEFS
- IF/IFDEF/IFNDEF/ELSE/ENDIF
- REPT/ENDREPEAT
- MACRO/ENDM
- INCLUDE, INCBIN

**BRASS directives MISSING in BASM:**
1. **ECHO** - Print message (BASM has PRINT)
2. **FILLTO** - Fill to address (BASM has ALIGN with fill)
3. **SEEK** - Seek in output file
4. **RELOCATE/ENDRELOCATE** - Relocatable section (BASM has RORG)
5. **DEFPAGE** - Define memory page
6. **ERROR/WARNING** - Custom error/warning messages (BASM has FAIL)
7. **INVOKE** - Invoke macro (BASM just uses macro name)

**Analysis:** BRASS is simpler and more focused on Z80. BASM has more advanced features.

---

## Recommended Additions for BASM

### Status Update (2026-01-07)

Based on current implementation review:

#### ✅ Already Implemented

1. **ELSE IF** - BASM already supports else-if (written as two words)

   ```asm
   IF condition1
       ; code
   ELSE IF condition2
       ; code
   ELSE
       ; code
   ENDIF
   ```

2. **@ prefix for local labels** - BASM has this built-in, no need for explicit LOCAL directive

3. **String comparison** - BASM already has comprehensive string comparison in expressions

4. **Include once** - BASM already supports including files only once

#### 🔧 To Implement

1. **WARNING** - Emit warning without stopping assembly

   ```asm
   WARNING "Using deprecated function"
   ```

   **Rationale:** Distinguish between fatal errors (FAIL) and warnings
   **Difficulty:** Easy (similar to PRINT/FAIL)
   **Status:** TODO - Need to implement

2. **OUTPUT** - Set output filename explicitly

   ```asm
   OUTPUT "mycode.bin"
   ```

   **Rationale:** Currently must use command line, inline control is useful
   **Difficulty:** Easy
   **Status:** TODO - Need to implement

3. **EVEN** - Align to even address (syntactic sugar)

   ```asm
   EVEN  ; equivalent to ALIGN 2
   ```

   **Rationale:** Very common idiom, readable shorthand
   **Difficulty:** Trivial (alias to ALIGN 2)
   **Status:** TODO - Need to implement

4. **PHASE/DEPHASE** - Verify if already implemented (may be aliases for RORG)

   ```asm
   PHASE 0x8000
       ; code assembled for 0x8000
   DEPHASE
   ```

   **Rationale:** Compatibility with RASM/SjasmPlus, clearer name than RORG
   **Difficulty:** Easy (may already exist or trivial alias)
   **Status:** TODO - Need to test and verify

#### ❌ Not Needed

1. **LOCAL** - Not needed, @ prefix works fine
2. **INCONCE** - Not needed, already have include once
3. **SAVEBIN** - Not needed, SAVE covers this
4. **IFC/IFNC** - Not needed, string comparison already exists
5. **Multi-platform device specs** - Not needed, CPC-focused assembler
6. **Embedded Lua** - Not needed for now (WASM engine could be considered for future)

### Future Considerations

1. **DISPLAY/ECHO** - Alternative to PRINT (compatibility alias)

   ```asm
   DISPLAY "Assembling main code..."
   ```

   **Rationale:** Compatibility with RASM
   **Difficulty:** Trivial (alias to PRINT)
   **Priority:** Low - nice to have but not essential

2. **DUP/EDUP** - Alternative to REPEAT (SjasmPlus compatibility)

   ```asm
   DUP 10
       db 0
   EDUP
   ```

   **Rationale:** Compatibility alias
   **Difficulty:** Trivial (alias to REPEAT/ENDREPEAT)
   **Priority:** Low - compatibility only

3. **UNDEFMACRO/PURGE** - Undefine macro

   ```asm
   UNDEFMACRO mymacro
   ```

   **Rationale:** Free up macro name for redefinition
   **Difficulty:** Medium
   **Priority:** Low

4. **LABELSLIST** - Export labels to file

   ```asm
   LABELSLIST "labels.txt"
   ```

   **Rationale:** Useful for debugging, external tools
   **Difficulty:** Medium
   **Priority:** Medium - useful for debugging

5. **WASM Engine** - Embedded WebAssembly scripting

   **Rationale:** Could be more useful than Lua for cross-platform scripting, lighter weight
   **Difficulty:** High
   **Priority:** Future consideration - interesting but complex

---

## Summary

**BASM Strengths:**

- Extensive cruncher support (10+ compression formats)
- Rich CPC-specific features (LOCOMOTIVE, SNAINIT, SNASET, SETCPC, SETCRTC)
- Advanced control flow (SWITCH/CASE, WHILE, ITERATE over lists)
- Functions with return values (unique feature)
- Expression-based conditionals (more flexible than IFEQ/IFNE)
- Comprehensive loop constructs
- Good symbol management
- ELSE IF construct (written as two words)
- @ prefix for local labels
- String comparison in expressions
- Include once capability

**Immediate TODO (2026-01-07):**

1. **WARNING** - Emit warnings without stopping assembly (high priority)
2. **OUTPUT** - Set output filename in code (high priority)
3. **EVEN** - Shorthand for ALIGN 2 (easy win)
4. **PHASE/DEPHASE** - Verify existence, test, implement if missing (may already exist)

**Future Considerations:**

- Compatibility aliases (DISPLAY, DUP/EDUP)
- LABELSLIST for debugging
- WASM engine for scripting (more practical than Lua)

**Not Needed:**

- ELSEIF (already have ELSE IF)
- LOCAL directive (@ prefix works)
- INCONCE (include once already works)
- SAVEBIN (SAVE covers it)
- IFC/IFNC (string comparison exists)
- Multi-platform device specs (CPC-focused)
- Embedded Lua (functions cover most needs)

BASM is already very complete for CPC development. The focus should be on the four immediate TODOs above, which are all straightforward implementations that will improve user experience.
