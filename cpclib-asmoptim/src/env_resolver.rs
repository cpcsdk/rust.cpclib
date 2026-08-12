//! A real, assembled-`Env`-backed [`AddressResolver`].
//!
//! This is the piece that makes `reachableByJr` actually decidable: it reads
//! back the per-token addresses `Env` records when
//! `AssemblingOptions::set_record_token_addresses(true)` was set for the
//! assemble (the LSP's `dry_run_env` turns this on unconditionally, since one
//! extra `HashMap` insert per token during an already-cached assemble is
//! noise - see that option's doc comment in `cpclib-asm`).
//!
//! Lives in this crate (not e.g. `cpclib-asm` itself) because it is the glue
//! between two independent things - a real `Env` and this crate's constraint
//! engine - and `cpclib-asm` has no reason to know this crate exists.

use cpclib_asm::assembler::Env;
use cpclib_asm::parser::MayHaveSpan;
use cpclib_tokens::symbols::SymbolsTableTrait;

use crate::engine::AddressResolver;

/// Resolves addresses and label values against a real, already-assembled
/// `Env`.
///
/// Generic over any `T: MayHaveSpan` - `LocatedToken` (what the LSP has on
/// hand) works directly, and so would a wrapper type carrying the same span.
/// A token with no real span (e.g. plain `Token`, or anything synthesized
/// rather than parsed from the document) simply resolves to `None` - there
/// is no position for it to have been recorded under in the first place.
pub struct EnvAddressResolver<'e> {
    env: &'e Env
}

impl<'e> EnvAddressResolver<'e> {
    pub fn new(env: &'e Env) -> Self {
        Self { env }
    }
}

impl<T> AddressResolver<T> for EnvAddressResolver<'_>
where T: MayHaveSpan
{
    /// `None` for a spanless token, for a span from a different parse than
    /// the one this `Env` visited (see `Env::address_of_span`'s doc comment),
    /// and when addresses were never recorded for this assemble at all -
    /// every case a constraint needing this can only report "unknown".
    fn address_of(&self, token: &T) -> Option<u16> {
        let span = token.possible_span()?;
        self.env.address_of_span(span)
    }

    /// Only resolves a *bare* label to its address - the conservative,
    /// always-safe answer for the general case this format allows
    /// (`reachableByJr`'s target can, in principle, be any expression) is to
    /// report unknown rather than guess.
    fn value_of_label(&self, name: &str) -> Option<i64> {
        self.env
            .symbols()
            .address_value(name)
            .ok()
            .flatten()
            .map(|addr| i64::from(addr.address()))
    }
}

/// Resolves against an `Env` produced by assembling a *different* file - the
/// project's entry point - rather than the document being analysed.
///
/// This is what makes an address-aware rule usable in a file that is only ever
/// `include`d. Assembled on its own such a file is a different program: in a
/// real project the constants live in the entry file, conditional blocks
/// therefore vanish, and the memory map (`range 0x0300, ...`) that places the
/// code is not there at all. A `jp` measured against that standalone layout
/// reported 127 bytes where the real build measured 146, and the `jr` it
/// suggested did not assemble.
///
/// So addresses come from assembling the entry, and are looked up by
/// `(this document's path, token offset)` - see
/// `Env::address_of_file_offset`, which exists precisely because
/// `SpanIdentity` is parse-local and cannot cross that gap.
///
/// **The caller must guarantee the file has not changed since the assemble.**
/// Offsets are only meaningful for the text that was actually assembled; an
/// edited buffer shifts them and every answer silently becomes wrong. The LSP
/// enforces this by using this resolver only when the buffer matches disk.
pub struct ProjectAddressResolver<'e> {
    env: &'e Env,
    document: std::path::PathBuf
}

impl<'e> ProjectAddressResolver<'e> {
    pub fn new(env: &'e Env, document: impl Into<std::path::PathBuf>) -> Self {
        Self {
            env,
            document: document.into()
        }
    }
}

impl<T> AddressResolver<T> for ProjectAddressResolver<'_>
where T: MayHaveSpan
{
    fn address_of(&self, token: &T) -> Option<u16> {
        let span = token.possible_span()?;
        self.env
            .address_of_file_offset(&self.document, span.offset_from_start())
    }

    /// Reads the *project's* symbol table, which is the other half of why the
    /// standalone numbers were wrong: a label defined in another file has no
    /// value at all when this file is assembled alone.
    fn value_of_label(&self, name: &str) -> Option<i64> {
        self.env
            .symbols()
            .address_value(name)
            .ok()
            .flatten()
            .map(|addr| i64::from(addr.address()))
    }
}
