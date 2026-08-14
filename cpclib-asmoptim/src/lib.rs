//! Z80 peephole optimization pattern engine.
//!
//! Reads optimization rules written in
//! [mdlz80optimizer](https://github.com/santiontanon/mdlz80optimizer)'s pattern
//! format, matches them against a Z80 instruction stream, and reports where a
//! sequence could be replaced by a better one.
//!
//! This crate deliberately lives outside `cpclib-asm`: it *consumes* the
//! assembler's AST (`ListingElement`) and, for address-aware rules, a real
//! assembled `Env`, but nothing in `cpclib-asm` depends on it. That keeps the
//! door open for a later session to plug the same engine into the real
//! assembler pipeline as an actual optimization pass (which would need
//! `cpclib-asm` to be split so its top layer can depend on this crate) without
//! any rewrite of the engine itself.
//!
//! Current consumers: `cpclib-lsp`, which surfaces matches as advisory editor
//! diagnostics and quickfixes. A real `basm` build is never silently altered by
//! anything here.

pub mod builtin_rules;
pub mod constraints;
pub mod dsl;
pub mod edit;
pub mod engine;
pub mod env_resolver;
pub mod smc;

// Re-exported so existing consumers keep one import path for "the optimizer's
// analysis vocabulary", now that the analysis half lives a crate below.
pub use cpclib_z80flow::{analysis_op, dependency, effects, liveness, regflag, stream};

pub use builtin_rules::{OptimizationGoal, builtin_rules};
pub use constraints::rules_need_addresses;
pub use cpclib_asm::flatten::{flatten_for_analysis, flatten_listing};
pub use env_resolver::{EnvAddressResolver, ProjectAddressResolver};
