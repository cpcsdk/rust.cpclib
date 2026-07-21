#![deny(deprecated)]
#![recursion_limit = "256"]

/// Implementation of various behavior for the tokens of cpclib_tokens
pub mod implementation;

/// All the stuff to parse z80 code.
pub mod parser;

/// Production of the bytecodes from the tokens.
pub mod assembler;

pub mod disass;

pub mod preamble;

pub mod error;
pub use error::{AssemblerError, ExpressionError};

mod crunchers;

pub mod lsp;
pub mod orgams;
pub mod progress;

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
    dry_run: bool
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
            force_void: true,
            debug: false,
            forbid_memory_override: false,
            dry_run: false
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
