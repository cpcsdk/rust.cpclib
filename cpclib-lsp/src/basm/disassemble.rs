//! Shared "assemble a self-contained snippet, then disassemble the result"
//! helper: recovers the real Z80 instruction(s) a fake instruction (or an
//! overflow-truncated value) actually becomes - derived purely by
//! assembling then disassembling, never hardcoded per fake-instruction
//! shape, so it automatically stays correct for any current or future one.

use cpclib_asm::ListingExt;

/// Assemble `snippet` (a small, self-contained piece of Z80 source with no
/// external symbol dependencies - callers are responsible for substituting
/// any variable references with their resolved literal value first) and
/// disassemble the resulting bytes back into its individual real
/// instructions, one element per instruction, in source order. Returns
/// `None` if the snippet fails to assemble or produces no bytes - callers
/// should treat that as "nothing to show", not an error.
pub(super) fn disassemble_snippet_lines(snippet: &str) -> Option<Vec<String>> {
    let bytes = cpclib_asm::assemble(snippet).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let listing = cpclib_asm::disass::disassemble(&bytes);
    let text = listing.to_string();
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    if lines.is_empty() { None } else { Some(lines) }
}

/// Like `disassemble_snippet_lines`, but joins every recovered instruction
/// into one string - for callers that just want a compact one-line summary
/// (e.g. an overflow warning's "assembles as: ..." text) rather than a
/// full per-instruction breakdown.
pub(super) fn disassemble_snippet(snippet: &str) -> Option<String> {
    let lines = disassemble_snippet_lines(snippet)?;
    let joined = lines.join(" ; ");
    if joined.is_empty() {
        None
    }
    else {
        Some(joined)
    }
}
