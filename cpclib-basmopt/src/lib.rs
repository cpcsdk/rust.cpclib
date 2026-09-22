//! `basmopt`'s core logic: parse a real `.asm` file, assemble it (dry-run, so
//! nothing on disk changes), match it against `cpclib-asmoptim`'s peephole
//! rules, and either report what it found or rewrite the source in place.
//!
//! Kept separate from [`cli`] (which is feature-gated behind `cmdline`) so
//! this crate is a plain library with no `clap` dependency for any consumer
//! that only wants `analyze_file`/`apply_fixes` directly - the LSP, in
//! particular, once it wires this engine into diagnostics.

#[cfg(feature = "cmdline")]
pub mod cli;

use std::collections::HashSet;

use camino::{Utf8Path, Utf8PathBuf};
use cpclib_asm::assembler::{Env, visit_tokens_all_passes_with_options};
use cpclib_asm::flatten::flatten_for_analysis;
use cpclib_asm::parser::context::ParserOptions;
use cpclib_asm::parser::{LocatedListing, LocatedToken, parse_z80_with_context_builder};
use cpclib_asm::{AssemblerError, AssemblingOptions, EnvOptions};
use cpclib_asmoptim::dsl::RuleSet;
use cpclib_asmoptim::engine::find_matches_with_resolver;
use cpclib_tokens::ListingElement;
pub use cpclib_asmoptim::{EnvAddressResolver, OptimizationGoal, ProjectAddressResolver};

/// What to check for, and which rules to check with.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct Options {
    /// Which built-in rule set to use as the base - see
    /// [`cpclib_asmoptim::OptimizationGoal`]. Ignored entirely when
    /// [`Self::no_builtin`] is set.
    pub goal: OptimizationGoal,
    /// Extra pattern files (mdlz80optimizer format, same as the vendored
    /// built-ins) to load on top of the built-in set. Each may itself
    /// `include "..."` a sibling file, resolved relative to its own
    /// directory.
    pub extra_rule_files: Vec<Utf8PathBuf>,
    /// Rule names (a rule's `name:` line) to skip even if the built-in set or
    /// an extra rule file defines them. An unnamed rule can't be targeted
    /// this way - name your own custom rules if you want them individually
    /// toggleable.
    pub disabled_rules: Vec<String>,
    /// Skip the built-in rule set entirely - only [`Self::extra_rule_files`]
    /// are used. For a user who wants full control over what basmopt
    /// suggests, without upstream's curated set mixed in.
    pub no_builtin: bool,
    /// Extra directories to search when resolving `INCLUDE`d files, same
    /// role as `basm`'s own `-I`/`--include`. Only ever consulted when a
    /// real assemble actually happens - see [`analyze_file`]'s own doc
    /// comment for when that is.
    pub include_dirs: Vec<Utf8PathBuf>,
    /// Symbols to define before assembling, `NAME` (= 1) or `NAME=VALUE`,
    /// same role as `basm`'s own `-D`/`--define`. Needed whenever the code
    /// under analysis is conditional on a symbol the build passes on its
    /// command line (`if LINKED_VERSION`), otherwise the real assemble the
    /// address-aware rules depend on fails on an unknown symbol.
    pub defines: Vec<String>,
    /// Show progress while analyzing - same flag, same underlying
    /// `cpclib_asm::progress` machinery `basm` itself uses (see
    /// `analyze_source`'s own doc comment for exactly what gets reported
    /// and why). Off by default, matching `basm`'s own CLI convention: an
    /// opt-in for a real, possibly slow run, not noise for a quick check.
    pub show_progress: bool
}


/// Everything that can go wrong turning a real file into suggestions.
#[derive(Debug, thiserror::Error)]
pub enum BasmOptError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error
    },
    // `AssemblerError` does not implement `std::error::Error`, so this is
    // formatted straight into the message rather than chained via `#[source]`
    // (a field literally named `source` gets that treatment implicitly,
    // hence `cause` here).
    #[error("{path}: {cause}")]
    Parse {
        path: Utf8PathBuf,
        cause: Box<AssemblerError>
    },
    #[error("{path}: {source}")]
    Rules {
        path: Utf8PathBuf,
        #[source]
        source: cpclib_asmoptim::dsl::RuleParseError
    }
}

/// One optimization opportunity found in the source.
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// 1-based line of the first matched instruction.
    pub line: u32,
    /// 1-based column of the first matched instruction.
    pub column: u32,
    /// The matched rule's `name:`, when it had one.
    pub rule_name: Option<String>,
    /// [`cpclib_asmoptim::engine::PeepholeMatch::bulk_unsafe`] - whether
    /// this suggestion must not be applied unreviewed. `--in-place` skips
    /// these; the plain report still lists them for a human to judge
    /// individually.
    pub bulk_unsafe: bool,
    /// The rule's `pattern:` description with `?variables` substituted -
    /// what to show the user.
    pub message: String,
    /// The suggested replacement, one entry per instruction. Empty means the
    /// matched instructions should simply be removed.
    pub replacement: Vec<String>,
    /// The source edit that applies this suggestion, computed by
    /// [`cpclib_asmoptim::edit`]. `None` when the match had no span to anchor
    /// an edit to.
    ///
    /// Kept private - a `Suggestion` a caller builds by hand (rather than
    /// getting one from [`analyze_file`]) has no meaningful edit to give it.
    fix: Option<cpclib_asmoptim::edit::SourceEdit>,
    /// Why this suggestion is safe - see [`SuggestionReason`]. Empty when the
    /// rule rests only on the shape of the instructions and there is nothing
    /// to explain beyond what is already visible.
    pub reasons: Vec<SuggestionReason>
}

/// One reason a suggestion is safe, with the source position of the
/// instruction that proves it.
///
/// The point of this is auditability: "Remove unused `ld b, c`" alone gives a
/// reader no way to tell whether B is clobbered two instructions later, inside
/// a routine three calls deep, or not at all.
#[derive(Debug, Clone)]
pub struct SuggestionReason {
    pub text: String,
    /// 1-based position of the instruction that proves it, when the reason
    /// rests on one. `None` for reasons about a distance or about execution
    /// ending, which have no single location.
    pub line: Option<u32>,
    pub column: Option<u32>
}

/// [`analyze_file`]'s result: the source text (so [`apply_fixes`] can be
/// called without re-reading the file), the suggestions found, and whether
/// an address-aware rule had to sit out because the file couldn't actually
/// be assembled.
#[derive(Debug, Clone)]
pub struct AnalyzeOutcome {
    pub source: String,
    pub suggestions: Vec<Suggestion>,
    /// Set when a real assemble was attempted (some active rule needed real
    /// addresses - e.g. `jp2jr`) but failed, most commonly an unresolvable
    /// `INCLUDE`. Not a hard error: every rule that doesn't need addresses
    /// was still checked normally and its findings are in `suggestions` -
    /// only address-aware rules could not be evaluated this run. See
    /// [`Options::include_dirs`] to fix an unresolvable `INCLUDE`.
    pub assemble_warning: Option<String>
}

/// Parse `path` and match it against `options`' rules.
///
/// Assembles it too (dry run, so nothing on disk changes) whenever the
/// active rule set contains anything that needs real addresses to decide
/// (currently `jp2jr`, in every goal - see `reachableByJr`). If that
/// assemble fails - most commonly an `INCLUDE` this call can't resolve, e.g.
/// because the process's working directory isn't the file's own project
/// root - this does **not** abort: it falls back to matching against the
/// parsed token stream alone. Address-aware rules simply report nothing in
/// that case (the same safe "unknown means don't suggest" behavior
/// `cpclib-asmoptim`'s engine already has for a missing resolver), rather
/// than the whole command refusing to report anything. The failure is
/// still surfaced via [`AnalyzeOutcome::assemble_warning`] rather than
/// silently swallowed. [`Options::include_dirs`] (`-I`/`--include` on the
/// CLI, matching `basm`'s own flag) is how to give it the real search path
/// instead of falling back.
pub fn analyze_file(path: &Utf8Path, options: &Options) -> Result<AnalyzeOutcome, BasmOptError> {
    let source = fs_err::read_to_string(path).map_err(|source| {
        BasmOptError::Io {
            path: path.to_owned(),
            source
        }
    })?;
    analyze_source(source, path, options)
}

/// Parses `entry` and every file it (transitively) `INCLUDE`s, and analyzes
/// each one on its own - `analyze_file` alone only ever sees `entry`'s own
/// top-level lines, which on a project shaped the common way (a thin
/// top-level file that mostly `INCLUDE`s everything else, the real code
/// living in the included files) means almost nothing gets analyzed at all.
///
/// `entry` is assembled once, for real (dry run), and every file's
/// address-aware rules are answered against that one whole-project address
/// space - an included file is not a complete program on its own, so
/// [`analyze_file`] called on it directly cannot give those rules real
/// addresses at all (see [`ProjectAddressResolver`]).
///
/// Each returned `(path, AnalyzeOutcome)` is scoped to exactly that one
/// file and applies with [`apply_fixes`]/[`apply_fixes_in_place`] exactly
/// as [`analyze_file`]'s own result would, against that file's own path -
/// nothing here changes what a suggestion means, only how many files get
/// looked at.
///
/// Best-effort in two ways, both matching [`analyze_file`]'s own philosophy
/// for an unresolvable `INCLUDE`: a file that fails to resolve, read or
/// parse is skipped (that one `INCLUDE` line just isn't followed further,
/// everything else still gets analyzed), and if the whole-project assemble
/// itself fails, this falls back to analyzing `entry` alone, address-aware
/// rules sitting out - it has no way to resolve any `INCLUDE` target that
/// needs symbol interpolation without a real `Env` to evaluate it with.
pub fn analyze_project(
    entry: &Utf8Path,
    options: &Options
) -> Result<Vec<(Utf8PathBuf, AnalyzeOutcome)>, BasmOptError> {
    let entry_source = fs_err::read_to_string(entry).map_err(|source| {
        BasmOptError::Io {
            path: entry.to_owned(),
            source
        }
    })?;

    let mut parser_options = ParserOptions::default();
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(cwd) = Utf8PathBuf::from_path_buf(cwd)
    {
        let _ = parser_options.add_search_path(cwd);
    }
    let _ = parser_options.add_search_path_from_file(entry.as_str());
    for dir in &options.include_dirs {
        let _ = parser_options.add_search_path(dir.as_str());
    }

    let builder = parser_options.clone().context_builder().set_current_filename(entry.as_str());
    let entry_listing = parse_z80_with_context_builder(&entry_source, builder).map_err(|cause| {
        BasmOptError::Parse {
            path: entry.to_owned(),
            cause: Box::new(cause)
        }
    })?;

    let rules = build_rule_set(options, entry)?;
    let needs_addresses = cpclib_asmoptim::rules_need_addresses(&rules);

    // One real assemble of the whole project - every file below is matched
    // against this same `env`, so their addresses agree with the actual
    // final program rather than each being assembled (wrongly) as if it
    // were the whole program on its own.
    let mut env = assemble_dry_run(&entry_listing, parser_options.clone(), &options.defines).ok();
    let fallback_to_entry_only = needs_addresses && env.is_none();

    let entry_canonical = fs_err::canonicalize(entry)
        .ok()
        .and_then(|p| Utf8PathBuf::try_from(p).ok())
        .unwrap_or_else(|| entry.to_owned());
    let mut seen = HashSet::new();
    seen.insert(entry_canonical.clone());
    let mut queue: Vec<(Utf8PathBuf, String, LocatedListing)> =
        vec![(entry_canonical, entry_source, entry_listing)];

    let mut outcomes = Vec::new();
    while let Some((path, source, listing)) = queue.pop() {
        let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();

        let (matches, assemble_warning) = match &env {
            Some(env) if needs_addresses => {
                let resolver = ProjectAddressResolver::new(env, path.clone().into_std_path_buf());
                (find_matches_with_resolver(&tokens, &rules, &resolver, options.goal), None)
            },
            None if needs_addresses => {
                (
                    cpclib_asmoptim::engine::find_matches(&tokens, &rules, options.goal),
                    Some(
                        "the project could not be fully assembled - address-aware rules were \
                         skipped for every file"
                            .to_string()
                    )
                )
            },
            _ => (cpclib_asmoptim::engine::find_matches(&tokens, &rules, options.goal), None)
        };
        let suggestions = matches.iter().map(|m| to_suggestion(&source, &tokens, m)).collect();

        // Queue this file's own further INCLUDEs before its listing (which
        // `tokens` borrows from) is dropped at the end of this iteration.
        // Needs a real `Env` to resolve a target that interpolates a symbol
        // into its filename - a plain string literal would not, but there
        // is no cheap way to tell which case an `INCLUDE` is without one.
        if !fallback_to_entry_only && let Some(env) = env.as_mut() {
            for token in &tokens {
                if !token.is_include() {
                    continue;
                }
                let Ok(fname) = env.build_fname(token.include_fname()) else { continue };
                let Ok(resolved) = cpclib_asm::assembler::file::get_filename_to_read(
                    &fname,
                    &parser_options,
                    Some(&*env)
                )
                else {
                    continue;
                };
                let Some(canonical) = fs_err::canonicalize(&resolved)
                    .ok()
                    .and_then(|p| Utf8PathBuf::try_from(p).ok())
                else {
                    continue;
                };
                if !seen.insert(canonical.clone()) {
                    continue; // already queued or being processed - mutual/repeated INCLUDE
                }
                let Ok(text) = fs_err::read_to_string(&canonical) else { continue };
                let builder = parser_options.clone().context_builder().set_current_filename(canonical.as_str());
                let Ok(nested) = parse_z80_with_context_builder(&text, builder) else { continue };
                queue.push((canonical, text, nested));
            }
        }

        outcomes.push((
            path,
            AnalyzeOutcome {
                source,
                suggestions,
                assemble_warning
            }
        ));
    }

    Ok(outcomes)
}

/// [`analyze_file`]'s real work, taking the source text directly instead of
/// reading it - what [`apply_fixes_in_place`]'s multi-pass loop needs, since
/// a later pass must analyze the *rewritten* in-memory text, not whatever is
/// still on disk. `path` is still needed (not just for error messages): it
/// anchors `INCLUDE` resolution and `assemble_dry_run`'s per-token address
/// recording to a real location on every pass, even though only the first
/// pass's text came from that location.
///
/// When [`Options::show_progress`] is set, this reports through the exact
/// same `cpclib_asm::progress::Progress` singleton `basm`'s own CLI uses
/// (a real assemble - the only thing a plain parse-and-match doesn't need
/// address-aware rules for - reports its own Parse/Pass/... phases from
/// *inside* the assembler once `ParserOptions::show_progress` is set, same
/// as `basm` itself; nothing here has to drive that part manually), plus a
/// spinner around the peephole matching pass, which has no such
/// infrastructure of its own and, on a real match-dense file, can be the
/// slower half - see `cpclib-asmoptim::engine`'s own module doc comment for
/// what makes a candidate's own cost genuinely expensive to compute.
fn analyze_source(
    source: String,
    path: &Utf8Path,
    options: &Options
) -> Result<AnalyzeOutcome, BasmOptError> {
    let mut parser_options = ParserOptions::default();
    parser_options.show_progress = options.show_progress;
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(cwd) = Utf8PathBuf::from_path_buf(cwd)
    {
        let _ = parser_options.add_search_path(cwd);
    }
    let _ = parser_options.add_search_path_from_file(path.as_str());
    for dir in &options.include_dirs {
        let _ = parser_options.add_search_path(dir.as_str());
    }

    let builder = parser_options
        .clone()
        .context_builder()
        .set_current_filename(path.as_str());

    // Mirrors `basm`'s own `parse()`: the assembler reports Pass/
    // PassProgress on its own once `show_progress` is set (below, on the
    // assemble), but *which file* is being parsed is something only the
    // caller knows, so it wraps the parse call itself.
    let show_progress =
        options.show_progress || cpclib_asm::progress::has_progress_sink();
    let progress_name = cpclib_asm::progress::normalize(path).to_string();
    if show_progress {
        cpclib_asm::progress::Progress::instance().add_parse(&progress_name);
    }
    let listing = parse_z80_with_context_builder(&source, builder).map_err(|cause| {
        BasmOptError::Parse {
            path: path.to_owned(),
            cause: Box::new(cause)
        }
    });
    if show_progress {
        cpclib_asm::progress::Progress::instance().remove_parse(&progress_name);
    }
    let listing = listing?;

    let rules = build_rule_set(options, path)?;
    let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();

    // The matching pass has no progress reporting of its own (unlike the
    // parse/assemble above) - a plain spinner is enough to say "still
    // working" rather than nothing at all, without needing engine-level
    // instrumentation for what is, even on a real match-dense file,
    // seconds rather than minutes.
    let matching_bar = start_matching_bar(show_progress);
    let (matches, assemble_warning) = if cpclib_asmoptim::rules_need_addresses(&rules) {
        match assemble_dry_run(&listing, parser_options, &options.defines) {
            Ok(env) => {
                let resolver = EnvAddressResolver::new(&env);
                (
                    find_matches_with_resolver(&tokens, &rules, &resolver, options.goal),
                    None
                )
            },
            Err(message) => {
                (
                    cpclib_asmoptim::engine::find_matches(&tokens, &rules, options.goal),
                    Some(message)
                )
            }
        }
    }
    else {
        (
            cpclib_asmoptim::engine::find_matches(&tokens, &rules, options.goal),
            None
        )
    };
    finish_matching_bar(matching_bar);

    let suggestions = matches
        .into_iter()
        .map(|m| to_suggestion(&source, &tokens, &m))
        .collect();

    Ok(AnalyzeOutcome {
        source,
        suggestions,
        assemble_warning
    })
}

/// Assemble `listing` as a dry run, with addresses recorded so
/// address-aware constraints can be evaluated - mirrors
/// `cpclib-lsp`'s own `dry_run_env` exactly, so the two tools never disagree
/// about what's reachable.
/// Splits a `-D` style definition into its name and value: `NAME` alone
/// means 1; a value is an integer (decimal, `0x..`, `&..`, `#..`, `0b..`,
/// optionally negative) or otherwise taken as a string, with one pair of
/// surrounding quotes removed.
pub fn parse_define(definition: &str) -> (String, cpclib_tokens::ExprResult) {
    use cpclib_tokens::ExprResult;
    let (name, raw) = definition.split_once('=').unwrap_or((definition, "1"));
    let raw = raw.trim();
    let (negative, digits) = raw.strip_prefix('-').map_or((false, raw), |d| (true, d));
    let number = if let Some(hex) = digits.strip_prefix("0x").or_else(|| digits.strip_prefix('&')).or_else(|| digits.strip_prefix('#')) {
        i32::from_str_radix(hex, 16).ok()
    }
    else if let Some(bin) = digits.strip_prefix("0b") {
        i32::from_str_radix(bin, 2).ok()
    }
    else {
        digits.parse::<i32>().ok()
    };
    let value = match number {
        Some(n) => ExprResult::from(if negative { -n } else { n }),
        None => {
            let unquoted = raw
                .strip_prefix('"')
                .and_then(|r| r.strip_suffix('"'))
                .unwrap_or(raw);
            ExprResult::from(unquoted.to_string())
        }
    };
    (name.trim().to_string(), value)
}

/// Defines every `NAME[=VALUE]` of `defines` in `assemble`'s symbol table.
pub fn apply_defines(assemble: &mut AssemblingOptions, defines: &[String]) -> Result<(), String> {
    use cpclib_tokens::symbols::SymbolsTableTrait;
    for definition in defines {
        let (name, value) = parse_define(definition);
        assemble
            .symbols_mut()
            .assign_symbol_to_value(name.as_str(), value)
            .map_err(|e| format!("cannot define `{definition}`: {e:?}"))?;
    }
    Ok(())
}

fn assemble_dry_run(
    listing: &LocatedListing,
    parse: ParserOptions,
    defines: &[String]
) -> Result<Env, String> {
    let mut assemble = AssemblingOptions::default();
    apply_defines(&mut assemble, defines)?;
    assemble.set_dry_run(true);
    assemble.set_record_token_addresses(true);
    let options = EnvOptions::new(parse, assemble, std::sync::Arc::new(()));

    match visit_tokens_all_passes_with_options(listing, options) {
        Ok((_, env)) => Ok(env),
        Err((_, _, e)) => Err(e.to_string())
    }
}

/// A spinner around the peephole matching pass - see [`analyze_source`]'s own
/// doc comment for why that pass needs one of its own. Mirrors `cpclib-basm`'s
/// own `#[cfg(feature = "indicatif")]`-gated bar-around-a-slow-step pattern
/// (`cpclib-basm/src/lib.rs`, around its `TO_M4` transfers): with the feature
/// off, `Progress::add_bar` doesn't exist at all, so there is nothing to show
/// and nothing to gate - only the `show_progress` flag itself survives.
#[cfg(feature = "indicatif")]
fn start_matching_bar(show_progress: bool) -> Option<indicatif::ProgressBar> {
    show_progress.then(|| cpclib_asm::progress::Progress::instance().add_bar("Matching"))
}

#[cfg(not(feature = "indicatif"))]
fn start_matching_bar(_show_progress: bool) {}

#[cfg(feature = "indicatif")]
fn finish_matching_bar(bar: Option<indicatif::ProgressBar>) {
    if let Some(bar) = bar {
        cpclib_asm::progress::Progress::instance().remove_bar_ok(&bar);
    }
}

#[cfg(not(feature = "indicatif"))]
fn finish_matching_bar(_bar: ()) {}

/// Build the working rule set: the built-in goal set (unless
/// [`Options::no_builtin`]), plus every extra rule file, minus every
/// disabled name.
fn build_rule_set(options: &Options, source_path: &Utf8Path) -> Result<RuleSet, BasmOptError> {
    let mut rules = if options.no_builtin {
        RuleSet::default()
    }
    else {
        cpclib_asmoptim::builtin_rules(options.goal).clone()
    };

    for path in &options.extra_rule_files {
        let text = fs_err::read_to_string(path).map_err(|source| {
            BasmOptError::Io {
                path: path.clone(),
                source
            }
        })?;
        let dir = path.parent().map(|p| p.to_owned());
        let extra = RuleSet::parse_with_includes(&text, |include| {
            let dir = dir.as_deref()?;
            fs_err::read_to_string(dir.join(include)).ok()
        })
        .map_err(|source| {
            BasmOptError::Rules {
                path: path.clone(),
                source
            }
        })?;
        rules.rules.extend(extra.rules);
    }

    if !options.disabled_rules.is_empty() {
        let disabled: HashSet<&str> = options.disabled_rules.iter().map(String::as_str).collect();
        rules
            .rules
            .retain(|r| !r.name.as_deref().is_some_and(|n| disabled.contains(n)));
    }

    let _ = source_path; // reserved: per-file rule overrides are a natural future extension
    Ok(rules)
}

/// Turn one engine match into a user-facing [`Suggestion`].
///
/// The byte range comes from [`cpclib_asmoptim::edit`], shared with the LSP's
/// quickfix - see that module for why computing it is more delicate than it
/// looks.
fn to_suggestion(
    source: &str,
    tokens: &[&LocatedToken],
    m: &cpclib_asmoptim::engine::PeepholeMatch
) -> Suggestion {
    use cpclib_asm::parser::MayHaveSpan;

    let anchor_span = tokens[m.anchor].span();
    let (line, column) = anchor_span.relative_line_and_column();

    let edit = cpclib_asmoptim::edit::edit_for_match(source, tokens, m);

    // A reason's witness is a token index; turn it into a source position, so
    // the reason can name a line the reader can go and look at.
    let reasons = m
        .reasons
        .iter()
        .map(|r| {
            let at = r
                .witness
                .and_then(|i| tokens.get(i))
                .map(|t| t.span().relative_line_and_column());
            SuggestionReason {
                text: r.text.clone(),
                line: at.map(|(l, _)| l as u32),
                column: at.map(|(_, c)| c as u32)
            }
        })
        .collect();

    Suggestion {
        line: line as u32,
        column: column as u32,
        rule_name: m.rule_name.clone(),
        bulk_unsafe: m.bulk_unsafe,
        message: m.message.clone(),
        replacement: m.replacement.clone(),
        fix: edit,
        reasons
    }
}

/// Rewrite `source`, applying every suggestion, highest byte offset first so
/// earlier offsets stay valid as the string is edited.
///
/// A suggestion whose edit could not be computed (no span to anchor it) is
/// skipped rather than guessed at.
pub fn apply_fixes(source: &str, suggestions: &[Suggestion]) -> String {
    let mut ordered: Vec<&Suggestion> = suggestions.iter().filter(|s| s.fix.is_some()).collect();
    ordered.sort_by_key(|s| std::cmp::Reverse(s.fix.as_ref().unwrap().range.start));

    let mut out = source.to_owned();
    for s in ordered {
        let edit = s.fix.as_ref().unwrap();
        out.replace_range(edit.range.clone(), &edit.text);
    }
    out
}

/// [`apply_fixes_in_place`]'s result.
#[derive(Debug, Clone)]
pub struct MultiPassOutcome {
    /// The rewritten source, after every pass that found something to fix.
    pub final_source: String,
    /// How many passes actually ran a real analysis - `1` in the common
    /// case (nothing left a second pass anything new to do), up to
    /// [`apply_fixes_in_place`]'s fixed cap.
    pub passes_run: usize,
    /// Bulk-safe suggestions applied, summed across every pass.
    pub total_applied: usize,
    /// Bulk-unsafe suggestions still standing after the last pass run - not
    /// applied, and not going to be by another pass either.
    pub remaining_skipped: usize,
    /// [`AnalyzeOutcome::assemble_warning`] from the *first* pass only - an
    /// `INCLUDE` this run can't resolve won't resolve itself on a later pass
    /// over the same text, so there's nothing more later passes could add.
    pub assemble_warning: Option<String>
}

/// Repeatedly analyze-and-apply `path`'s bulk-safe suggestions, up to a
/// fixed 2 passes (mirrors upstream `mdlz80optimizer`'s own default
/// `nPasses`) - so a fix pass 1 applies that exposes a genuinely new
/// opportunity (e.g. a dead load only becomes provably dead once an earlier
/// instruction reading it is itself removed) gets caught within the same
/// `--in-place` run, rather than needing the user to notice and re-run.
///
/// Stops early once a pass finds nothing bulk-safe to apply - the common
/// single-pass case costs exactly what it did before this existed. Each
/// pass re-analyzes the *rewritten* in-memory text (`analyze_source`), never
/// re-reading the file - the file itself is only written once, after the
/// loop ends.
pub fn apply_fixes_in_place(
    path: &Utf8Path,
    options: &Options
) -> Result<MultiPassOutcome, BasmOptError> {
    const MAX_PASSES: usize = 2;

    let mut source = fs_err::read_to_string(path).map_err(|source| {
        BasmOptError::Io {
            path: path.to_owned(),
            source
        }
    })?;
    let mut passes_run = 0;
    let mut total_applied = 0;
    let mut remaining_skipped = 0;
    let mut assemble_warning = None;

    for pass in 0..MAX_PASSES {
        let outcome = analyze_source(source, path, options)?;
        passes_run += 1;
        if pass == 0 {
            assemble_warning = outcome.assemble_warning;
        }

        let (safe, skipped): (Vec<Suggestion>, Vec<Suggestion>) =
            outcome.suggestions.into_iter().partition(|s| !s.bulk_unsafe);
        remaining_skipped = skipped.len();

        if safe.is_empty() {
            source = outcome.source;
            break;
        }
        total_applied += safe.len();
        source = apply_fixes(&outcome.source, &safe);
    }

    Ok(MultiPassOutcome {
        final_source: source,
        passes_run,
        total_applied,
        remaining_skipped,
        assemble_warning
    })
}

/// [`apply_fixes_in_place_project`]'s result, summed across every file in a
/// project-wide run.
#[derive(Debug, Clone, Default)]
pub struct ProjectApplyOutcome {
    pub files_touched: usize,
    pub files_with_errors: usize,
    pub total_applied: usize,
    pub total_skipped_for_review: usize,
    /// Address-aware suggestions (e.g. `jp2jr`) found but never applied in
    /// this mode - a real count, not an estimate. See
    /// [`apply_fixes_in_place_project`]'s own doc comment for why
    /// project-wide apply always excludes them.
    pub total_address_aware_skipped: usize
}

/// Every rule name (`Options::disabled_rules`'s own doc comment notes an
/// unnamed rule can't be individually targeted - the same limitation applies
/// here) that `cpclib_asmoptim::constraints::rule_needs_addresses` flags in
/// `rules`.
fn address_aware_rule_names(rules: &RuleSet) -> HashSet<String> {
    rules
        .rules
        .iter()
        .filter(|r| cpclib_asmoptim::constraints::rule_needs_addresses(r))
        .filter_map(|r| r.name.clone())
        .collect()
}

/// Every `.asm` file under `root`, gitignore-respecting (a project's own
/// generated output stays unmodified) and walked in parallel, sorted by
/// path for reproducible results - every `.asm` regardless of include-
/// reachability, matching a human's `find . -name '*.asm' -exec basmopt -i
/// {}'` mental model and this crate's own single-file simplicity.
///
/// A small, self-contained copy of `cpclib_project::walk`'s own walk
/// (same `ignore::WalkBuilder` settings) rather than a dependency on that
/// crate: `cpclib-project` itself depends on `cpclib-bndbuild`, which
/// depends on `cpclib-basmopt` (it wires this crate in as a build-rule
/// runner) - a direct dependency the other way would be a real cycle, not
/// just an inconvenience.
fn asm_files_under(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut builder = ignore::WalkBuilder::new(root.as_std_path());
    builder
        .require_git(false)
        .hidden(false)
        .filter_entry(|entry| {
            !entry.file_type().is_some_and(|t| t.is_dir())
                || !entry
                    .file_name()
                    .to_str()
                    .is_some_and(|n| matches!(n, ".git" | ".hg" | ".svn" | "target" | "node_modules"))
        });

    let found = std::sync::Mutex::new(Vec::new());
    builder.build_parallel().run(|| {
        Box::new(|entry| {
            if let Ok(entry) = entry
                && entry.file_type().is_some_and(|t| t.is_file())
                && entry.path().extension().is_some_and(|e| e == "asm")
                && let Ok(path) = Utf8PathBuf::from_path_buf(entry.into_path())
            {
                found.lock().unwrap().push(path);
            }
            ignore::WalkState::Continue
        })
    });

    let mut found = found.into_inner().unwrap();
    found.sort();
    found
}

/// Apply bulk-safe, non-address-aware fixes to every `.asm` file under
/// `root` (via [`asm_files_under`]), one file at a time via
/// [`apply_fixes_in_place`]'s own unchanged 2-pass loop.
///
/// Deliberately never applies an address-aware rule's suggestion (today,
/// only `jp2jr`'s `reachableByJr`), regardless of `options.goal`:
/// [`cpclib_asmoptim::ProjectAddressResolver`] resolves every file's
/// addresses from *one* `Env`, assembled once from the project's entry
/// point - rewriting one file in this run can shift another file's real
/// addresses (via a shared `INCLUDE`), invalidating an address-aware
/// match found before that rewrite happened. Bulk-safe, non-address-aware
/// rules never consult addresses at all and are unaffected by rewrite
/// order. Composes with a `; noopt`-marked instruction automatically: the
/// engine vetoes it before a match is ever constructed
/// (`cpclib_asmoptim::noopt`), so it never reaches this function's own
/// apply/skip logic in the first place.
///
/// Not implemented by filtering the rule set before matching (which would
/// need a second, unfiltered analysis per file just to count what got
/// excluded): each file is analyzed once, with the full rule set, and each
/// resulting [`Suggestion`] is classified by name against
/// [`address_aware_rule_names`] - cheap, computed once for the whole run,
/// not per file. Known limitation, documented rather than solved: this is
/// name-based, so an *unnamed* custom rule (via `--rules FILE`) that
/// happens to need addresses cannot be excluded this way and would be
/// treated as safe - name your own custom rules if they need addresses,
/// same advice `Options::disabled_rules` already gives for the same reason.
pub fn apply_fixes_in_place_project(root: &Utf8Path, options: &Options) -> ProjectApplyOutcome {
    const MAX_PASSES: usize = 2;

    let address_aware = build_rule_set(options, root)
        .map(|rules| address_aware_rule_names(&rules))
        .unwrap_or_default();

    let mut result = ProjectApplyOutcome::default();
    for path in asm_files_under(root) {
        let Ok(initial_source) = fs_err::read_to_string(&path)
        else {
            result.files_with_errors += 1;
            continue;
        };

        let mut source = initial_source.clone();
        let mut file_applied = 0;
        let mut file_skipped_for_review = 0;
        let mut file_address_aware_skipped = 0;
        let mut file_had_error = false;

        for _pass in 0..MAX_PASSES {
            // Cloned rather than moved (unlike `apply_fixes_in_place`'s own
            // loop): that function propagates a failed `analyze_source` via
            // `?` and never touches `source` again, but this one has to
            // fall back to the last known-good `source` on a per-file error
            // and keep going with the next file, so `source` must stay
            // usable either way.
            let analysis = match analyze_source(source.clone(), &path, options) {
                Ok(a) => a,
                Err(_) => {
                    file_had_error = true;
                    break;
                }
            };

            let mut safe = Vec::new();
            file_skipped_for_review = 0;
            file_address_aware_skipped = 0;
            for suggestion in analysis.suggestions {
                if suggestion.bulk_unsafe {
                    file_skipped_for_review += 1;
                }
                else if suggestion
                    .rule_name
                    .as_deref()
                    .is_some_and(|n| address_aware.contains(n))
                {
                    file_address_aware_skipped += 1;
                }
                else {
                    safe.push(suggestion);
                }
            }

            if safe.is_empty() {
                source = analysis.source;
                break;
            }
            file_applied += safe.len();
            source = apply_fixes(&analysis.source, &safe);
        }

        if file_had_error {
            result.files_with_errors += 1;
            continue;
        }

        if source != initial_source {
            let _ = fs_err::write(&path, &source);
        }

        result.files_touched += 1;
        result.total_applied += file_applied;
        result.total_skipped_for_review += file_skipped_for_review;
        result.total_address_aware_skipped += file_address_aware_skipped;
    }

    result
}


#[cfg(test)]
mod analyze_project_tests {
    use super::*;

    /// The whole point: a top-level file that only `INCLUDE`s another one
    /// still gets that included file's own suggestions - which `analyze_file`
    /// on the entry alone cannot see (its `INCLUDE` is one opaque token).
    #[test]
    fn a_project_wide_analysis_finds_suggestions_in_an_included_file() {
        let dir = camino_tempfile::tempdir().unwrap();
        fs_err::write(dir.path().join("real_code.asm"), "\tld a,0\n\tld a,0\n\tret\n").unwrap();
        fs_err::write(dir.path().join("entry.asm"), "\torg 0x4000\n\tinclude \"real_code.asm\"\n").unwrap();
        let entry = camino::Utf8Path::from_path(dir.path().join("entry.asm").as_std_path()).unwrap().to_owned();

        let outcomes = analyze_project(&entry, &Options::default()).unwrap();
        assert_eq!(outcomes.len(), 2, "{outcomes:?}");

        let entry_alone = analyze_file(&entry, &Options::default()).unwrap();
        assert!(
            entry_alone.suggestions.is_empty(),
            "analyze_file on the entry alone cannot see inside its own INCLUDE: {:?}",
            entry_alone.suggestions
        );

        let included = outcomes.iter().find(|(p, _)| p.as_str().ends_with("real_code.asm")).expect("real_code.asm must be queued");
        assert!(!included.1.suggestions.is_empty(), "the duplicate `ld a,0` must be found inside the included file");
    }

    /// A chain of INCLUDEs (A includes B includes C) is walked fully, and a
    /// file INCLUDEd from two places is only queued once.
    #[test]
    fn included_files_are_walked_transitively_and_deduplicated() {
        let dir = camino_tempfile::tempdir().unwrap();
        fs_err::write(dir.path().join("c.asm"), "\tret\n").unwrap();
        fs_err::write(dir.path().join("b.asm"), "\tinclude \"c.asm\"\n\tinclude \"c.asm\"\n").unwrap();
        fs_err::write(dir.path().join("a.asm"), "\torg 0x4000\n\tinclude \"b.asm\"\n").unwrap();
        let entry = camino::Utf8Path::from_path(dir.path().join("a.asm").as_std_path()).unwrap().to_owned();

        let outcomes = analyze_project(&entry, &Options::default()).unwrap();
        let names: Vec<String> = outcomes.iter().map(|(p, _)| p.file_name().unwrap().to_string()).collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert!(names.contains(&"a.asm".to_string()));
        assert!(names.contains(&"b.asm".to_string()));
        assert!(names.contains(&"c.asm".to_string()), "c.asm queued once despite two INCLUDEs: {names:?}");
    }

    /// An unresolvable INCLUDE must not abort the whole walk - everything
    /// else discoverable still gets analyzed, matching `analyze_file`'s own
    /// "fall back gracefully" behavior for the same situation.
    #[test]
    fn an_unresolvable_include_does_not_abort_the_rest_of_the_walk() {
        let dir = camino_tempfile::tempdir().unwrap();
        fs_err::write(
            dir.path().join("entry.asm"),
            "\torg 0x4000\n\tinclude \"does-not-exist.asm\"\n\tld a,0\n\tld a,0\n\tret\n"
        )
        .unwrap();
        let entry = camino::Utf8Path::from_path(dir.path().join("entry.asm").as_std_path()).unwrap().to_owned();

        let outcomes = analyze_project(&entry, &Options::default()).unwrap();
        assert_eq!(outcomes.len(), 1, "only the entry itself, the bad INCLUDE just isn't followed: {outcomes:?}");
        assert!(!outcomes[0].1.suggestions.is_empty(), "the entry's own duplicate ld a,0 must still be found");
    }
}

#[cfg(test)]
mod define_tests {
    use cpclib_tokens::ExprResult;

    use super::*;

    #[test]
    fn a_define_parses_numbers_bare_names_and_strings() {
        assert_eq!(parse_define("LINKED_VERSION=1"), ("LINKED_VERSION".to_string(), ExprResult::from(1)));
        assert_eq!(parse_define("FLAG").1, ExprResult::from(1), "a bare name means 1");
        assert_eq!(parse_define("A=0x10").1, ExprResult::from(16));
        assert_eq!(parse_define("A=&ff").1, ExprResult::from(255));
        assert_eq!(parse_define("A=#20").1, ExprResult::from(32));
        assert_eq!(parse_define("A=0b101").1, ExprResult::from(5));
        assert_eq!(parse_define("A=-3").1, ExprResult::from(-3));
        assert_eq!(parse_define("A=\"hi\"").1, ExprResult::from("hi".to_string()));
        assert_eq!(parse_define("A=word").1, ExprResult::from("word".to_string()));
    }

    /// The reason the option exists: without the define, this file cannot
    /// even be assembled (unknown symbol), so the address-aware rules lose
    /// the real assemble they need; with it, `jp2jr` resolves and fires.
    #[test]
    fn a_define_lets_a_conditional_file_assemble_for_address_aware_rules() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("cond.asm");
        fs_err::write(&path, "if LINKED\nstart:\n jp target\ntarget:\n ret\nendif\n").unwrap();
        let path = camino::Utf8Path::from_path(path.as_std_path()).unwrap().to_owned();

        let goal = Options { goal: cpclib_asmoptim::OptimizationGoal::Size, ..Default::default() };
        let without = analyze_file(&path, &goal).unwrap();
        assert!(without.assemble_warning.is_some(), "the unknown symbol must be reported");

        let with = analyze_file(
            &path,
            &Options { defines: vec!["LINKED".to_string()], ..goal }
        )
        .unwrap();
        assert!(with.assemble_warning.is_none(), "{:?}", with.assemble_warning);
    }
}
