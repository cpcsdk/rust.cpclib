//! Shared "assemble a self-contained snippet, then disassemble the result"
//! helper. Used to enrich warning messages (fake-instruction, overflow)
//! with the actual real Z80 instruction(s) involved - derived purely by
//! assembling then disassembling, never hardcoded per fake-instruction
//! shape, so it automatically stays correct for any current or future one,
//! and for overflow it shows the real *truncated* value's resulting
//! instruction rather than just its numeric magnitude.

use cpclib_asm::ListingExt;

/// Assemble `snippet` (a small, self-contained piece of Z80 source with no
/// external symbol dependencies - callers are responsible for substituting
/// any variable references with their resolved literal value first) and
/// disassemble the resulting bytes back into human-readable instruction
/// text. Returns `None` if the snippet fails to assemble or produces no
/// bytes - callers should treat that as "nothing to add", not an error.
pub(super) fn disassemble_snippet(snippet: &str) -> Option<String> {
    let bytes = cpclib_asm::assemble(snippet).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let listing = cpclib_asm::disass::disassemble(&bytes);
    let text = listing.to_string();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        None
    }
    else {
        Some(lines.join(" ; "))
    }
}
