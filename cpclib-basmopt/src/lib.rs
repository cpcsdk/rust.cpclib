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
pub use cpclib_asmoptim::{EnvAddressResolver, OptimizationGoal};

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
    pub include_dirs: Vec<Utf8PathBuf>
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
    Parse { path: Utf8PathBuf, cause: AssemblerError },
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

    let mut parser_options = ParserOptions::default();
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
    let listing = parse_z80_with_context_builder(&source, builder).map_err(|cause| {
        BasmOptError::Parse {
            path: path.to_owned(),
            cause
        }
    })?;

    let rules = build_rule_set(options, path)?;
    let tokens: Vec<&LocatedToken> = flatten_for_analysis(listing.iter()).collect();

    let (matches, assemble_warning) = if cpclib_asmoptim::rules_need_addresses(&rules) {
        match assemble_dry_run(&listing, parser_options) {
            Ok(env) => {
                let resolver = EnvAddressResolver::new(&env);
                (find_matches_with_resolver(&tokens, &rules, &resolver), None)
            },
            Err(message) => {
                (cpclib_asmoptim::engine::find_matches(&tokens, &rules), Some(message))
            }
        }
    }
    else {
        (cpclib_asmoptim::engine::find_matches(&tokens, &rules), None)
    };

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
fn assemble_dry_run(listing: &LocatedListing, parse: ParserOptions) -> Result<Env, String> {
    let mut assemble = AssemblingOptions::default();
    assemble.set_dry_run(true);
    assemble.set_record_token_addresses(true);
    let options = EnvOptions::new(parse, assemble, std::sync::Arc::new(()));

    match visit_tokens_all_passes_with_options(listing, options) {
        Ok((_, env)) => Ok(env),
        Err((_, _, e)) => Err(e.to_string())
    }
}

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

