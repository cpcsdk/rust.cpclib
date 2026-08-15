#![deny(deprecated)]
#![recursion_limit = "256"]

/// Implementation of various behavior for the tokens of cpclib_tokens
pub mod implementation;

/// All the stuff to parse z80 code.
pub mod parser;

/// Production of the bytecodes from the tokens.
pub mod assembler;

pub mod disass;

pub mod flatten;

pub mod preamble;

pub mod error;
pub use error::{AssemblerError, ExpressionError, WarningCategory};

mod crunchers;

pub mod lsp;
pub mod orgams;
pub mod progress;
pub mod unused_bindings;

use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, RwLock};

use cpclib_disc::amsdos::*;
use cpclib_sna::Snapshot;
use enumflags2::{BitFlags, bitflags};
use preamble::function::FunctionBuilder;
use preamble::processed_token::ProcessedToken;
pub use preamble::*;

use self::listing_output::{ListingOutput, ListingOutputFormat};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AssemblingOptionFlags {
    /// Set to consider that the assembler pay attention to the case of the labels
    CaseSensitive,
    // Set to include SYMB in sna chunks
    SnaSymb,
    // Set to include BRKS in sna chunks
    SnaBrks,
    // Set to include BRKC in sna chunks
    SnaBrkc,
    // Set to include REMU in sna chunks
    SnaRemu,
    // Save remu chunk in a file
    RemuInFile,
    // Save wabp chunck in a file
    WabpInFile,
    // generate breakpoint as code
    BreakpointAsOpcode
}

impl AssemblingOptionFlags {
    pub fn from_chunk(chunk: &str) -> Option<Self> {
        match chunk {
            "SYMB" => Some(Self::SnaSymb),
            "BRKS" => Some(Self::SnaBrks),
            "BRKC" => Some(Self::SnaBrkc),
            "REMU" => Some(Self::SnaRemu),
            _ => None
        }
    }
}

/// Configuration of the assembler. By default the assembler is case sensitive and has no symbol
#[derive(Debug, Clone)]
pub struct AssemblingOptions {
    flags: BitFlags<AssemblingOptionFlags>,

    /// Contains some symbols that could be used during assembling
    symbols: cpclib_tokens::symbols::SymbolsTable,
    output_builder: Option<Arc<RwLock<ListingOutput>>>,
    /// The snapshot may be prefiled with a dedicated snapshot
    snapshot_model: Option<Snapshot>,
    amsdos_behavior: AmsdosAddBehavior,
    enable_warnings: bool,
    /// Individually-disabled warning classes, on top of the blanket
    /// `enable_warnings` switch - see `WarningCategory`. Empty by default
    /// (nothing individually disabled, matching today's exact behavior).
    disabled_warning_categories: BitFlags<WarningCategory>,
    force_void: bool,
    debug: bool,
    forbid_memory_override: bool,
    /// When set, guarantees no real-world side effect occurs during
    /// assembling: no file is written to disk (`SAVE`/`WRITE`, `BUILDSNA`,
    /// `BUILDCPR`, listing output), and no blocking read of the real
    /// process stdin happens (`PAUSE`). Everything else (in particular
    /// computing the assembled bytes/symbol table) runs exactly as normal —
    /// this is not a "skip assembling" flag, only a "skip its disk/stdin
    /// effects" one. See each gated call site (`SaveCommand::execute_on`,
    /// `Env::save_sna`/`save_cpr`, `PauseCommand::execute`) for exactly
    /// what's suppressed.
    dry_run: bool,
    /// When set, `Env` records the real assembled address of every token it
    /// visits (`Env::visit_located_token`), overwritten pass over pass so it
    /// converges to the final pass's addresses. Default off: a real `basm`
    /// build should not pay for a `HashMap` insert per token it never reads
    /// back. Consumed by `Env::address_of_token_offset` — tooling such as the
    /// LSP's peephole-optimizer support (`cpclib-asmoptim`) turns this on to
    /// evaluate address-aware rule constraints (e.g. "is this JP in range for
    /// a JR").
    record_token_addresses: bool
}

impl Default for AssemblingOptions {
    fn default() -> Self {
        Self {
            flags: AssemblingOptionFlags::CaseSensitive
                | AssemblingOptionFlags::SnaBrkc
                | AssemblingOptionFlags::SnaBrks
                | AssemblingOptionFlags::SnaSymb
                | AssemblingOptionFlags::SnaRemu,
            symbols: cpclib_tokens::symbols::SymbolsTable::default(),
            output_builder: None,
            snapshot_model: None,
            amsdos_behavior: AmsdosAddBehavior::FailIfPresent,
            enable_warnings: true,
            disabled_warning_categories: BitFlags::empty(),
            force_void: true,
            debug: false,
            forbid_memory_override: false,
            dry_run: false,
            record_token_addresses: false
        }
    }
}

#[allow(missing_docs)]
impl AssemblingOptions {
    pub fn new_case_sensitive() -> Self {
        Self::default()
    }

    pub fn new_case_insensitive() -> Self {
        let mut options = Self::new_case_sensitive();
        options.set_case_sensitive(false);
        options
    }

    /// Creation an option object with the given symbol table
    pub fn new_with_table(symbols: &cpclib_tokens::symbols::SymbolsTable) -> Self {
        let mut options = Self::default();
        options.set_symbols(symbols);
        options
    }

    pub fn set_flag(&mut self, flag: AssemblingOptionFlags, val: bool) -> &mut Self {
        self.flags.set(flag, val);
        self
    }

    pub fn get_flag(&self, flag: AssemblingOptionFlags) -> bool {
        self.flags.contains(flag)
    }

    pub fn disable_warnings(&mut self) -> &mut Self {
        self.enable_warnings = false;
        self
    }

    pub fn enable_warnings_flag(&self) -> bool {
        self.enable_warnings
    }

    pub fn disable_warning_category(&mut self, category: WarningCategory) -> &mut Self {
        self.disabled_warning_categories.insert(category);
        self
    }

    pub fn enable_warning_category(&mut self, category: WarningCategory) -> &mut Self {
        self.disabled_warning_categories.remove(category);
        self
    }

    pub fn is_warning_category_enabled(&self, category: WarningCategory) -> bool {
        !self.disabled_warning_categories.contains(category)
    }

    /// Specify if the assembler must be case sensitive or not
    pub fn set_case_sensitive(&mut self, val: bool) -> &mut Self {
        self.set_flag(AssemblingOptionFlags::CaseSensitive, val);
        self
    }

    pub fn set_save_behavior(&mut self, behavior: AmsdosAddBehavior) -> &mut Self {
        self.amsdos_behavior = behavior;
        self
    }

    pub fn set_snapshot_model(&mut self, mut sna: Snapshot) -> &mut Self {
        sna.unwrap_memory_chunks();
        self.snapshot_model = Some(sna);
        self
    }

    /// Specify a symbol table to copy
    pub fn set_symbols(&mut self, val: &cpclib_tokens::symbols::SymbolsTable) -> &mut Self {
        self.symbols = val.clone();
        self
    }

    pub fn symbols(&self) -> &cpclib_tokens::symbols::SymbolsTable {
        &self.symbols
    }

    pub fn symbols_mut(&mut self) -> &mut cpclib_tokens::symbols::SymbolsTable {
        &mut self.symbols
    }

    pub fn case_sensitive(&self) -> bool {
        self.get_flag(AssemblingOptionFlags::CaseSensitive)
    }

    pub fn debug(&self) -> bool {
        self.debug
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn forbid_memory_override(&self) -> bool {
        self.forbid_memory_override
    }

    pub fn set_forbid_memory_override(&mut self, forbid: bool) -> &mut Self {
        self.forbid_memory_override = forbid;
        self
    }

    pub fn snapshot_model(&self) -> Option<&Snapshot> {
        self.snapshot_model.as_ref()
    }

    pub fn save_behavior(&self) -> AmsdosAddBehavior {
        self.amsdos_behavior
    }

    pub fn force_void(&self) -> bool {
        self.force_void
    }

    pub fn set_force_void(&mut self, force_void: bool) -> &mut Self {
        self.force_void = force_void;
        self
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    /// Also clears any listing-output writer already configured via
    /// `write_listing_output[_with_format]` — defense in depth against a
    /// caller that configures output *before* enabling `dry_run`, which
    /// the setters' own dry-run check (evaluated at the time *they're*
    /// called) can't otherwise catch.
    pub fn set_dry_run(&mut self, dry_run: bool) -> &mut Self {
        self.dry_run = dry_run;
        if dry_run {
            self.output_builder = None;
        }
        self
    }

    pub fn record_token_addresses(&self) -> bool {
        self.record_token_addresses
    }

    pub fn set_record_token_addresses(&mut self, record: bool) -> &mut Self {
        self.record_token_addresses = record;
        self
    }

    /// No-op under `dry_run` (defense in depth: a listing writer given to a
    /// dry-run environment is silently dropped rather than trusted not to
    /// point at a real file), otherwise as `write_listing_output`.
    pub fn write_listing_output<W: 'static + Write + Send + Sync>(
        &mut self,
        writer: W
    ) -> &mut Self {
        if self.dry_run {
            return self;
        }
        self.output_builder = Some(Arc::new(RwLock::new(ListingOutput::new(writer))));
        if let Some(b) = self.output_builder.as_mut() {
            b.write().unwrap().on()
        }
        self
    }

    /// Collect a source map: which address each source line ended up at, and
    /// how many bytes it emitted.
    ///
    /// Independent of the listing *text* - a caller can want the map, the
    /// listing, or both. Unlike `write_listing_output` this is **not** a no-op
    /// under `dry_run`, because nothing is written anywhere: the rows are read
    /// back out of the `Env` afterwards.
    pub fn record_source_map(&mut self) -> &mut Self {
        let builder = self.output_builder.get_or_insert_with(|| {
            Arc::new(RwLock::new(ListingOutput::new(std::io::sink())))
        });
        {
            let mut output = builder.write().unwrap();
            output.on();
            output.collect_source_map();
        }
        self
    }

    /// No-op under `dry_run` — see [`Self::write_listing_output`].
    pub fn write_listing_output_with_format<W: 'static + Write + Send + Sync>(
        &mut self,
        writer: W,
        format: ListingOutputFormat
    ) -> &mut Self {
        if self.dry_run {
            return self;
        }
        self.output_builder = Some(Arc::new(RwLock::new(ListingOutput::new_with_format(
            writer, format
        ))));
        if let Some(b) = self.output_builder.as_mut() {
            b.write().unwrap().on()
        }
        self
    }
}

/// Assemble a piece of code and returns the associated list of bytes.
pub fn assemble(code: &str) -> Result<Vec<u8>, Box<AssemblerError>> {
    let options = EnvOptions::default();
    // let options = AssemblingOptions::new_with_table(table);
    assemble_with_options(code, options).map(|(bytes, _symbols)| bytes)
}

/// Assemble a piece of code and returns the associates liste of bytes as well as the generated reference table.
pub fn assemble_with_options(
    code: &str,
    options: EnvOptions
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), Box<AssemblerError>> {
    let builder = options.parse_options().clone().context_builder();
    let tokens = parser::parse_z80_with_context_builder(code, builder)?;
    assemble_tokens_with_options(&tokens, options)
}

/// Assemble the predifined list of tokens
pub fn assemble_tokens_with_options<
    'tokens,
    T: 'static + Visited + ToSimpleToken + Clone + ListingElement + Sync + MayHaveSpan
>(
    tokens: &'tokens [T],
    options: EnvOptions
) -> Result<(Vec<u8>, cpclib_tokens::symbols::SymbolsTable), Box<AssemblerError>>
where
    <T as cpclib_tokens::ListingElement>::Expr: ExprEvaluationExt + Sync,
    <<T as cpclib_tokens::ListingElement>::TestKind as cpclib_tokens::TestKindElement>::Expr:
        implementation::expression::ExprEvaluationExt + Sync,
    ProcessedToken<'tokens, T>: FunctionBuilder
{
    let (_tok, env) = assembler::visit_tokens_all_passes_with_options(tokens, options)
        .map_err(|(_, _, e)| AssemblerError::AlreadyRenderedError(e.to_string()))?;
    Ok((env.produced_bytes(), env.symbols().clone()))
}

/// Build the code and store it inside a file supposed to be injected in a dsk
/// XXX probably crash if filename is not coherent
/// //
pub fn assemble_to_amsdos_file(
    code: &str,
    amsdos_filename: &str,
    options: EnvOptions
) -> Result<AmsdosFile, Box<AssemblerError>> {
    let amsdos_filename = AmsdosFileName::try_from(amsdos_filename)?;

    let tokens = parser::parse_z80_str(code)?;

    let (_, env) = assembler::visit_tokens_all_passes_with_options(&tokens, options)
        .map_err(|(_, _, e)| AssemblerError::AlreadyRenderedError(e.to_string()))?;

    Ok(AmsdosFile::binary_file_from_buffer(
        &amsdos_filename,
        env.loading_address().unwrap(),
        env.execution_address().unwrap(),
        &env.produced_bytes()
    )?)
}

#[cfg(test)]
mod test_super {

    use super::*;

    #[test]
    fn simple_test_assemble() {
        let code = "
		org 0
		db 1, 2
		db 3, 4
		";

        let bytes = assemble(code).unwrap_or_else(|e| panic!("Unable to assemble {}: {}", code, e));
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

    /// Regression test: `CP`'s parser representation was rewritten to match
    /// `ADD`/`ADC`'s own established two-slot shape (`arg1` = optional
    /// explicit `A,` prefix, `arg2` = the mandatory compared value) instead
    /// of silently discarding the `A,` prefix - `CP r` and `CP A,r` are the
    /// same real Z80 instruction and must assemble to byte-identical output
    /// regardless of which form was written. Also covers the non-register
    /// (`CP n`) and `(HL)` operand forms, since the byte-size table
    /// (`cpclib-asm/src/implementation/tokens.rs`) and codegen
    /// (`assembler/mod.rs::assemble_cp`) both had to be updated to read the
    /// compared value from its new slot.
    #[test]
    fn cp_with_and_without_the_explicit_accumulator_prefix_assemble_identically() {
        for (bare, explicit) in [
            ("cp c", "cp a, c"),
            ("cp 1", "cp a, 1"),
            ("cp (hl)", "cp a, (hl)")
        ] {
            let bare_bytes =
                assemble(bare).unwrap_or_else(|e| panic!("Unable to assemble {bare:?}: {e}"));
            let explicit_bytes = assemble(explicit)
                .unwrap_or_else(|e| panic!("Unable to assemble {explicit:?}: {e}"));
            assert_eq!(
                bare_bytes, explicit_bytes,
                "{bare:?} and {explicit:?} must assemble identically"
            );
            assert!(!bare_bytes.is_empty());
        }
    }

    /// Same regression as `cp_with_and_without_the_explicit_accumulator_prefix_assemble_identically`,
    /// for `SUB`/`AND`/`OR`/`XOR` - all four had the identical bug (`parse_sub`/
    /// `parse_logical_operator` in cpclib-asm silently discarded the optional
    /// `A,` prefix), found and fixed as a follow-up once `CP` was fixed.
    #[test]
    fn sub_and_or_xor_with_and_without_the_explicit_accumulator_prefix_assemble_identically() {
        for (bare, explicit) in [
            ("sub c", "sub a, c"),
            ("sub 1", "sub a, 1"),
            ("sub (hl)", "sub a, (hl)"),
            ("and c", "and a, c"),
            ("and 1", "and a, 1"),
            ("or c", "or a, c"),
            ("or 1", "or a, 1"),
            ("xor c", "xor a, c"),
            ("xor 1", "xor a, 1")
        ] {
            let bare_bytes =
                assemble(bare).unwrap_or_else(|e| panic!("Unable to assemble {bare:?}: {e}"));
            let explicit_bytes = assemble(explicit)
                .unwrap_or_else(|e| panic!("Unable to assemble {explicit:?}: {e}"));
            assert_eq!(
                bare_bytes, explicit_bytes,
                "{bare:?} and {explicit:?} must assemble identically"
            );
            assert!(!bare_bytes.is_empty());
        }
    }

    /// Regression test: `SUB`'s real (fake-instruction) 16-bit form
    /// (`SUB DE,rr`/`SUB HL,rr`) must keep working after `parse_sub` was
    /// restructured to also carry the optional 8-bit `A,` prefix in the same
    /// `arg1` slot - `assemble_sub` now has to distinguish "arg1 is DE/HL
    /// itself" (the fake 16-bit form) from "arg1 is the optional A prefix"
    /// (the normal 8-bit form) before deciding which path to take.
    #[test]
    fn sub_16bit_fake_instruction_still_assembles() {
        let bytes = assemble("sub hl, bc\n")
            .unwrap_or_else(|e| panic!("Unable to assemble the fake SUB HL,BC form: {e}"));
        assert!(!bytes.is_empty());

        let bytes = assemble("sub de, bc\n")
            .unwrap_or_else(|e| panic!("Unable to assemble the fake SUB DE,BC form: {e}"));
        assert!(!bytes.is_empty());
    }

    /// The explicit `A,` accumulator prefix on `ADD`/`ADC`/`SBC`/`CP`/`SUB`/
    /// `AND`/`OR`/`XOR` is redundant, real, valid Z80 syntax - the parser
    /// should warn about it (`LocatedToken::is_redundant_accumulator_prefix`)
    /// without treating it as a fake instruction, and the bare (implicit-`A`)
    /// form must never be flagged.
    #[test]
    fn explicit_accumulator_prefix_is_flagged_as_redundant_not_fake() {
        let code = "org 0x4000\ncp a, c\ncp c\nsub a, c\nadd a, c\nadd hl, de\nret\n";
        let listing = parser::parse_z80_str(code).unwrap();

        let mut warned = listing.iter().filter(|t| t.is_warning());

        let cp_explicit = warned.next().expect("cp a, c should be warning-wrapped");
        assert!(
            cp_explicit.is_redundant_accumulator_prefix(),
            "{cp_explicit:?}"
        );
        assert!(!cp_explicit.is_fake_instruction(), "{cp_explicit:?}");

        let sub_explicit = warned.next().expect("sub a, c should be warning-wrapped");
        assert!(
            sub_explicit.is_redundant_accumulator_prefix(),
            "{sub_explicit:?}"
        );

        let add_explicit = warned.next().expect("add a, c should be warning-wrapped");
        assert!(
            add_explicit.is_redundant_accumulator_prefix(),
            "{add_explicit:?}"
        );

        // `add hl, de` is a real, non-redundant, non-fake register-pair form -
        // never warning-wrapped at all.
        assert!(
            warned.next().is_none(),
            "no further warning-wrapped tokens expected"
        );

        let cp_bare = listing
            .iter()
            .find(|t| !t.is_warning() && t.mnemonic() == Some(&Mnemonic::Cp))
            .expect("cp c should be a plain, non-warning token");
        assert!(!cp_bare.is_redundant_accumulator_prefix(), "{cp_bare:?}");
    }

    #[test]
    fn fake_ld_instruction_warning_keeps_a_structured_location() {
        // Regression test: `ld hl, de` is a "fake" instruction (accepted by
        // basm, assembled using several real opcodes, but not a genuine Z80
        // instruction) and is reported as a warning. That warning used to be
        // flattened into a plain rendered string (`AlreadyRenderedError`)
        // before it ever reached `env.warnings()`, discarding its source
        // location entirely - callers (like the LSP, mapping this to a
        // `Diagnostic` range) had no way to recover *where* the warning
        // applies. It should now carry a structured line/column/len.
        //
        // Note: an earlier version of this fix kept the live `Z80Span`
        // around instead (as `RelocatedWarning`) for later rendering. That
        // caused real undefined behavior for warnings whose span pointed
        // into a macro-expansion scratch buffer (`str::from_utf8_unchecked`
        // on since-overwritten bytes, caught by nightly's UB checks) -
        // `AlreadyRenderedWarningWithLocation` avoids this by rendering
        // *and* capturing line/column/len eagerly, never holding the span
        // itself past this point. See `good_fake_instructions2.asm` in
        // `cpclib-basm`'s test fixtures for the macro case that surfaced it.
        let code = "
		org 0x4000
		ld hl, de
		ret
		";
        let tokens = parser::parse_z80_str(code).unwrap();
        let options = EnvOptions::default();
        let env = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };

        assert_eq!(env.warnings().len(), 1, "{:?}", env.warnings());
        match &*env.warnings()[0] {
            AssemblerError::AlreadyRenderedWarningWithLocation {
                msg,
                line,
                column,
                len
            } => {
                assert!(msg.contains("fake instruction"), "{msg}");
                assert_eq!(*line, 3);
                assert_eq!(*column, 3);
                assert_eq!(*len, "ld hl, de".len() as u32);
            },
            other => {
                panic!("expected an AlreadyRenderedWarningWithLocation, got: {other:?}")
            }
        }
    }

    /// `AssemblerError::warning_category` must classify each of the four
    /// individually-toggleable kinds correctly - the shared basis for
    /// `AssemblingOptions`/`ParserOptions`'s `disabled_warning_categories`
    /// and `basm --disable-warning`.
    #[test]
    fn warning_category_classifies_each_real_kind() {
        let code = "\
            org 0x4000\n\
            add de, bc\n\
            cp a, c\n\
            ld b, 300\n\
            db 1\n\
            org 0x4000\n\
            db 2\n\
            ret\n";
        let tokens = parser::parse_z80_str(code).unwrap();
        let options = EnvOptions::default();
        let env = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };

        let categories: std::collections::HashSet<_> = env
            .warnings()
            .iter()
            .map(|w| w.warning_category())
            .collect();
        assert!(
            categories.contains(&WarningCategory::FakeInstruction),
            "{:?}",
            env.warnings()
        );
        assert!(
            categories.contains(&WarningCategory::RedundantAccumulatorPrefix),
            "{:?}",
            env.warnings()
        );
        assert!(
            categories.contains(&WarningCategory::Overflow),
            "{:?}",
            env.warnings()
        );
        assert!(
            categories.contains(&WarningCategory::OverrideMemory),
            "{:?}",
            env.warnings()
        );
    }

    /// Regression test for the parser-level fix: disabling
    /// `RedundantAccumulatorPrefix`/`FakeInstruction` in `ParserOptions`
    /// must prevent `WarningWrapper` from being constructed at all - not
    /// just filter the resulting diagnostic out later. A token that's still
    /// wrapped (even if nothing ever renders the warning) is a real
    /// correctness gap for other consumers that key behavior off
    /// `is_warning()`/`is_fake_instruction()`.
    #[test]
    fn disabling_a_warning_category_in_parser_options_prevents_the_wrapper_from_ever_being_built() {
        let builder = crate::parser::context::ParserContextBuilder::default()
            .set_disabled_warning_categories(WarningCategory::RedundantAccumulatorPrefix.into());
        let tokens = crate::parser::parse_z80_with_context_builder("cp a, c\n", builder).unwrap();
        let token = tokens.iter().next().unwrap();
        assert!(!token.is_warning(), "{token:?}");

        // The bare form is never affected either way.
        let bare = parser::parse_z80_str("cp c\n").unwrap();
        assert!(!bare.iter().next().unwrap().is_warning());
    }

    /// Same fix, for `FakeInstruction` - and confirms disabling the warning
    /// never disables the accepted *syntax*: the fake-instruction expansion
    /// keys off the raw operand shape, not `is_warning()`, so the token
    /// must still assemble to the identical bytes either way.
    #[test]
    fn disabling_fake_instruction_warnings_does_not_disable_the_expansion() {
        let disabled_builder = crate::parser::context::ParserContextBuilder::default()
            .set_disabled_warning_categories(WarningCategory::FakeInstruction.into());
        let disabled_tokens =
            crate::parser::parse_z80_with_context_builder("add de, bc\n", disabled_builder)
                .unwrap();
        let disabled_token = disabled_tokens.iter().next().unwrap();
        assert!(!disabled_token.is_warning(), "{disabled_token:?}");

        let enabled_bytes = assemble("add de, bc\n").unwrap();
        let disabled_bytes = {
            let options = EnvOptions::default();
            match assembler::visit_tokens_all_passes_with_options(&disabled_tokens, options) {
                Ok((_tok, env)) => env.produced_bytes(),
                Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
            }
        };
        assert_eq!(enabled_bytes, disabled_bytes);
    }

    /// Regression test for the assembler-level gate (`override_memory`/
    /// `overflow`, only known at real assemble time): disabling one
    /// category via `AssemblingOptions` removes it from `env.warnings()`
    /// while the other real warnings present in the same file still fire -
    /// guards against a too-broad filter.
    #[test]
    fn disabling_a_warning_category_in_assembling_options_removes_only_that_category() {
        let code = "\
            org 0x4000\n\
            ld b, 300\n\
            db 1\n\
            org 0x4000\n\
            db 2\n\
            ret\n";
        let tokens = parser::parse_z80_str(code).unwrap();

        let mut assemble_opts = AssemblingOptions::default();
        assemble_opts.disable_warning_category(WarningCategory::OverrideMemory);
        let options = EnvOptions::new(Default::default(), assemble_opts, std::sync::Arc::new(()));
        let env = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };

        let categories: std::collections::HashSet<_> = env
            .warnings()
            .iter()
            .map(|w| w.warning_category())
            .collect();
        assert!(
            !categories.contains(&WarningCategory::OverrideMemory),
            "{:?}",
            env.warnings()
        );
        assert!(
            categories.contains(&WarningCategory::Overflow),
            "{:?}",
            env.warnings()
        );
    }

    #[test]
    fn is_fake_instruction_distinguishes_fake_instructions_from_other_warnings() {
        // Regression test: `is_warning()` alone just means "this token
        // produced *some* parse-time warning" - it can't distinguish a fake
        // instruction (`ld hl, de`) from any other kind of warning-wrapped
        // token (e.g. the unrelated `WRITE DIRECT` case in
        // `directives.rs`). `is_fake_instruction()` checks the actual
        // warning message against the shared `FAKE_INSTRUCTION_WARNING`
        // constant, so a caller (like the LSP) can ask the token directly
        // rather than approximating from `is_warning()`.
        let code = "org 0x4000\nld hl, de\nld a, 1\nret\n";
        let listing = parser::parse_z80_str(code).unwrap();

        let fake = listing
            .iter()
            .find(|t| t.is_warning())
            .expect("ld hl, de should be warning-wrapped");
        assert!(fake.is_fake_instruction(), "{fake:?}");

        let real = listing
            .iter()
            .find(|t| !t.is_warning() && t.mnemonic() == Some(&Mnemonic::Ld))
            .expect("ld a, 1 should be a plain, non-warning token");
        assert!(!real.is_fake_instruction(), "{real:?}");
    }

    #[test]
    fn overflow_warning_fires_for_plain_macro_and_repeat_bodies() {
        // Regression test: an out-of-range immediate (`ld b, 300`, 300
        // doesn't fit in a byte) now gets caught right where the assembler
        // already resolves the value and truncates it - `Env::checked_byte`
        // - rather than only in the LSP. This must keep working identically
        // when the offending line sits inside a MACRO body or a REPEAT
        // block (each REPEAT iteration re-visits the same source line) -
        // both are the exact "short source lifespan" scenario that caused
        // real undefined behavior for a *different* warning kind earlier in
        // this project's life (a warning holding onto a `Z80Span` whose
        // underlying buffer got reused by a later pass). These warnings are
        // safe from that: they're built as plain, unlocated
        // `AssemblingError{msg}` values from the moment they're created
        // (never holding a `Z80Span` themselves) - `visit_located_token`'s
        // existing auto-locate-then-promptly-render machinery does the
        // location/rendering entirely on its own, within the same pass.
        let code = "
		org 0x4000
		ld b, 300
		MACRO FOO
			ld c, 300
		ENDM
		FOO()
		repeat 2
			ld d, 300
		endrepeat
		ret
		";
        let tokens = parser::parse_z80_str(code).unwrap();
        let options = EnvOptions::default();
        let env = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };

        assert_eq!(env.warnings().len(), 4, "{:?}", env.warnings());
        for warning in env.warnings() {
            assert!(
                warning
                    .to_string()
                    .contains("value 300 does not fit in 8 bits"),
                "{warning}"
            );
        }
        let rendered: Vec<String> = env.warnings().iter().map(|w| w.to_string()).collect();
        assert!(rendered[0].contains("ld b, 300"), "{}", rendered[0]);
        assert!(rendered[1].contains("MACRO FOO"), "{}", rendered[1]);
        assert!(rendered[1].contains("ld c, 300"), "{}", rendered[1]);
        assert!(rendered[2].contains("ld d, 300"), "{}", rendered[2]);
        assert!(rendered[3].contains("ld d, 300"), "{}", rendered[3]);
    }

    #[test]
    fn overflow_warning_fires_for_16bit_immediates_and_defb_defw() {
        let code = "
		org 0x4000
		ld bc, 70000
		db 1, 2, 300
		dw 1, 70000
		ret
		";
        let tokens = parser::parse_z80_str(code).unwrap();
        let options = EnvOptions::default();
        let env = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };

        assert_eq!(env.warnings().len(), 3, "{:?}", env.warnings());
        let rendered: Vec<String> = env.warnings().iter().map(|w| w.to_string()).collect();
        assert!(
            rendered[0].contains("value 70000 does not fit in 16 bits"),
            "{}",
            rendered[0]
        );
        assert!(
            rendered[1].contains("value 300 does not fit in 8 bits"),
            "{}",
            rendered[1]
        );
        assert!(
            rendered[2].contains("value 70000 does not fit in 16 bits"),
            "{}",
            rendered[2]
        );
    }

    #[test]
    fn value_that_fits_produces_no_overflow_warning() {
        let code = "
		org 0x4000
		ld b, 200
		ld bc, 40000
		db 1, 2, 3
		ret
		";
        let tokens = parser::parse_z80_str(code).unwrap();
        let options = EnvOptions::default();
        let env = match assembler::visit_tokens_all_passes_with_options(&tokens, options) {
            Ok((_tok, env)) => env,
            Err((_tok, _env, e)) => panic!("assembling should not fail: {e}")
        };
        assert!(env.warnings().is_empty(), "{:?}", env.warnings());
    }

    #[test]
    fn located_test_assemble() {
        let code = "
		org 0x100
		db 1, 2
		db 3, 4
		";

        let bytes = assemble(code).unwrap_or_else(|e| panic!("Unable to assemble {}: {}", code, e));
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn dry_run_prevents_save_from_writing_a_file() {
        let target = std::env::temp_dir().join(format!(
            "cpclib_asm_dry_run_test_{}_should_not_exist.bin",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&target); // in case a previous failed run left it behind
        let code = format!("org 0\ndb 1,2,3,4\nsave \"{}\", 0, 4\n", target.display());

        let mut options = AssemblingOptions::default();
        options.set_dry_run(true);
        let options = EnvOptions::from(options);

        let tokens = parser::parse_z80_str(&code).unwrap();
        let (_toks, mut env) = assembler::visit_tokens_all_passes_with_options(&tokens, options)
            .unwrap_or_else(|(_, _, e)| panic!("assembling failed: {e}"));
        // `SAVE` only queues a command; a real build's post-processing step
        // is what actually writes it — exercise that step explicitly to
        // prove `dry_run` holds even when it's reached.
        env.handle_post_actions(&tokens)
            .unwrap_or_else(|e| panic!("handle_post_actions failed: {e}"));

        assert!(
            !target.exists(),
            "dry_run must prevent SAVE from writing a real file"
        );
    }

    #[test]
    fn dry_run_skips_pause_without_blocking_on_stdin() {
        let code = "org 0\ndb 1\npause\n";
        let mut options = AssemblingOptions::default();
        options.set_dry_run(true);
        // `PAUSE`'s queued command is only ever flushed from
        // `start_new_pass` when `debug` is on — enable it so this test
        // actually exercises the gated code path, not just a PAUSE that
        // was never reached regardless of dry_run.
        options.set_debug(true);
        let options = EnvOptions::from(options);
        // Would hang forever waiting on real stdin if dry_run didn't skip
        // PAUSE's blocking read — this test completing at all is the proof.
        let result = assemble_with_options(code, options);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn quiet_parser_option_is_off_by_default_and_round_trips_through_the_builder() {
        assert!(!crate::parser::context::ParserOptions::default().quiet);

        let ctx = crate::parser::context::ParserContextBuilder::default()
            .set_quiet(true)
            .build("");
        assert!(ctx.options.quiet);
    }

    /// Real proof that `quiet` suppresses `PRINT_PARSE`'s stdout write, using
    /// an actual OS-level fd redirection (`gag`) — `#[ignore]`d because it
    /// captures the real process stdout fd, which any *other* test running
    /// concurrently (incl. cargo's own "test ... ok" progress lines) would
    /// leak into, making this flaky under the default parallel test runner.
    /// Run in isolation (also needs `--nocapture`, or the test harness's
    /// own stdout capture intercepts the output before `gag` ever sees it):
    /// `cargo test -p cpclib-asm --lib -- --ignored --test-threads=1
    /// --nocapture quiet_parser_option_actually_suppresses_stdout`.
    #[test]
    #[ignore = "captures the real process stdout fd; must run alone, see doc comment"]
    fn quiet_parser_option_actually_suppresses_stdout() {
        let code = "ASMCONTROL PRINT_PARSE, \"hello\"\n";

        // Baseline: without `quiet`, PRINT_PARSE really does write to
        // stdout at parse time (this is the pre-existing, intentional CLI
        // behavior `quiet` must not break).
        {
            let stdout = gag::BufferRedirect::stdout().unwrap();
            let builder = crate::parser::context::ParserContextBuilder::default();
            let _ = crate::parser::obtained::LocatedListing::new_complete_source(code, builder);
            let mut buf = String::new();
            let mut stdout = stdout.into_inner();
            std::io::Read::read_to_string(&mut stdout, &mut buf).unwrap();
            assert!(
                buf.contains("[PARSE]"),
                "expected PRINT_PARSE to print without quiet, got: {buf:?}"
            );
        }

        // With `quiet`, nothing must reach the real stdout — this is what
        // an LSP server (whose real stdout carries JSON-RPC traffic) needs.
        {
            let stdout = gag::BufferRedirect::stdout().unwrap();
            let builder = crate::parser::context::ParserContextBuilder::default().set_quiet(true);
            let _ = crate::parser::obtained::LocatedListing::new_complete_source(code, builder);
            let mut buf = String::new();
            let mut stdout = stdout.into_inner();
            std::io::Read::read_to_string(&mut stdout, &mut buf).unwrap();
            assert!(
                buf.is_empty(),
                "quiet must suppress PRINT_PARSE's stdout output, got: {buf:?}"
            );
        }
    }

    #[test]
    fn case_verification() {
        let code = "
		ld hl, TruC
Truc
		";

        let options = AssemblingOptions::new_case_sensitive();
        let options = EnvOptions::from(options);
        println!("{:?}", assemble_with_options(code, options.clone()));
        assert!(assemble_with_options(code, options).is_err());

        let options = AssemblingOptions::new_case_insensitive();
        let options = EnvOptions::from(options);
        println!("{:?}", assemble_with_options(code, options.clone()));
        assert!(assemble_with_options(code, options).is_ok());
    }

    #[test]
    fn test_size() {
        let mut env: Env = Default::default();
        dbg!(
            env.assemble_call_jr_or_jp(Mnemonic::Jp, None, &DataAccess::Expression(Expr::Value(0)))
                .unwrap()
        );

        assert_eq!(
            Token::OpCode(Mnemonic::Nop, None, None, None).number_of_bytes(),
            Ok(1)
        );

        assert_eq!(
            Token::OpCode(
                Mnemonic::Dec,
                Some(DataAccess::Register8(Register8::A)),
                None,
                None
            )
            .number_of_bytes(),
            Ok(1)
        );
    }

    #[test]
    fn test_listing() {
        let mut listing = Listing::from_str("   nop").expect("unable to assemble");
        assert_eq!(listing.estimated_duration().unwrap(), 1);
        listing.set_duration(100);
        assert_eq!(listing.estimated_duration().unwrap(), 100);
    }

    fn code_test(code: &'static str) {
        let asm_options = AssemblingOptions::new_case_insensitive();
        let env_options = EnvOptions::new(ParserOptions::default(), asm_options, Arc::new(()));
        let res = assemble_with_options(code, env_options);
        res.map_err(|e| eprintln!("{e}")).unwrap();
    }

    /// Test stolen to rasm
    #[test]
    fn rasm_pagetag1() {
        let code = "  
        bankset 0
        org #5000
label1
        bankset 1
        org #9000
label2
        bankset 2
        assert {page}label1==0xC0
        assert {page}label2==0xC6 
        assert {pageset}label1==#C0
        assert {pageset}label2==#C2
        assert $ == 0x0000
        assert $$ == 0x0000
        nop";
        code_test(code);
    }
    // /// This test currently does not pass
    // #[test]
    // fn rasm_pagetag2() {
    // let code = "
    // bankset 0
    // call maroutine
    //
    // bank 4
    // org #C000
    // autreroutine
    // nop
    // ret
    //
    // bank 5
    // org #8000
    // maroutine
    // ldir
    // ret
    //
    // bankset 2
    // org #9000
    // troize
    // nop
    // assert {page}maroutine==#7FC5
    // assert {pageset}maroutine==#7FC2
    // assert {page}autreroutine==#7FC4
    // assert {pageset}autreroutine==#7FC2
    // assert {page}troize==#7FCE
    // assert {pageset}troize==#7FCA";
    // rasm_test(code);
    //
    // }
    // #define AUTOTEST_PAGETAG3	"buildsna:bank 2:assert {bank}$==2:assert {page}$==0x7FC0:assert {pageset}$==#7FC0:" \
    // "bankset 1:org #4000:assert {bank}$==5:assert {page}$==0x7FC5:assert {pageset}$==#7FC2"

    #[test]
    fn test_duration() {
        let listing = Listing::from_str(
            "
            pop de      ; 3
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 3);

        let listing = Listing::from_str(
            "
            inc l       ; 1
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 1);

        let listing = Listing::from_str(
            "
            ld (hl), e  ; 2
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            ld (hl), d  ; 2
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), 2);

        let listing = Listing::from_str(
            "
            pop de      ; 3
            inc l       ; 1
            ld (hl), e  ; 2
            inc l       ; 1
            ld (hl), d  ; 2
        "
        )
        .expect("Unable to assemble this code");
        println!("{}", listing.to_string());
        assert_eq!(listing.estimated_duration().unwrap(), (3 + 1 + 2 + 1 + 2));
    }

    #[test]
    fn test_real1() {
        let code = "RUN 0x50, 0xc0";
        code_test(code);

        let code = r"    if {bank}$ == 0
            RUN 0x50, 0xc0
        endif
        ";
        code_test(code);
    }
}
