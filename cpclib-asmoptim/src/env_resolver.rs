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
