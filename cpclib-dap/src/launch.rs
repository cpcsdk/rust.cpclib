//! Getting from "debug this file" to a running emulator with a source map.
//!
//! Four steps, in this order because each needs the one before: assemble the
//! program for real (a dry run produces no snapshot), write the snapshot, put
//! the patched emulator on a loopback port with that snapshot to hand, and tell
//! the editor where to look.

use std::path::{Path, PathBuf};

use cpclib_asm::assembler::listing_output::SourceMapFile;
use cpclib_project::config::AsmConfig;
use cpclib_project::srcmap::SourceMap;
use cpclib_tokens::symbols::SymbolsTableTrait;

/// What a launch produced.
pub struct Launched {
    pub source_map: SourceMap,
    pub snapshot: Vec<u8>,
    /// The program's memory, flat, indexed by physical offset - the base 64K
    /// first. Held so the debugger can answer "are these three bytes a `CALL`?"
    /// without a round trip to the emulator for every candidate on the stack.
    pub image: Vec<u8>,
    /// Where the snapshot starts executing, for `stopOnEntry`.
    pub entry_point: Option<u16>,
    pub entry: PathBuf,
    /// The breakpoints the program itself asked for, via `BREAKPOINT`.
    pub breakpoints: Vec<cpclib_asm::assembler::delayed_command::AssembledBreakpoint>
}

/// Assemble `entry` and collect everything a debug session needs from it.
///
/// A **real** assemble, not the dry run the language server uses for analysis:
/// `save_sna` is a no-op under `dry_run`, and a debugger with no snapshot has
/// nothing to run. The source map comes from the same pass, so the addresses in
/// it are by construction the addresses in the snapshot.
pub fn assemble_for_debug(entry: &Path, config: &AsmConfig) -> Result<Launched, String> {
    let text = fs_err::read_to_string(entry).map_err(|e| format!("{}: {e}", entry.display()))?;

    let mut parse = cpclib_asm::parser::context::ParserOptions::default();
    parse.set_quiet(true);
    parse.set_disabled_warning_categories(config.warnings.disabled_parser_categories());
    for directory in cpclib_project::root::ancestor_directories(entry) {
        let _ = parse.add_search_path(directory);
    }
    let builder = parse
        .clone()
        .context_builder()
        .set_current_filename(entry.to_str().ok_or("the entry path is not utf-8")?);
    let listing = cpclib_asm::parser::parse_z80_with_context_builder(&text, builder)
        .map_err(|e| format!("{e}"))?;

    let mut assemble = cpclib_asm::AssemblingOptions::default();
    assemble.set_case_sensitive(config.case_sensitive);
    assemble.record_source_map();
    for category in config.warnings.disabled_assembling_categories().iter() {
        assemble.disable_warning_category(category);
    }
    // The `-D` symbols the project's build rule passes; without them an entry
    // that `include`s a configured file does not assemble at all.
    let definitions = cpclib_project::build_defs::definitions_for_entry(entry);
    for (name, value) in &definitions.values {
        let value = match value.parse::<i32>() {
            Ok(number) => cpclib_tokens::ExprResult::from(number),
            Err(_) => cpclib_tokens::ExprResult::String(value.as_str().into())
        };
        assemble
            .symbols_mut()
            .assign_symbol_to_value(name.as_str(), value)
            .ok();
    }

    let (_processed, mut env) = cpclib_asm::assembler::visit_tokens_all_passes_with_options(
        &listing,
        cpclib_asm::EnvOptions::new(
            parse,
            assemble,
            std::sync::Arc::new(cpclib_common::event::DiscardObserver)
        )
    )
    .map_err(|(_, _, e)| format!("{e}"))?;

    // The post actions are what run the listing pass, which is what fills the
    // source map - and what writes any `SAVE` the program asked for.
    env.handle_post_actions(&listing)
        .map_err(|e| format!("{e}"))?;

    let raw = env
        .source_map()
        .ok_or("no source map was produced; the listing pass did not run")?;

    // The snapshot travels in memory: it is served to the emulator over
    // loopback, so there is no reason to leave a file behind.
    let temporary = std::env::temp_dir().join(format!("cpclib-dap-{}.sna", std::process::id()));
    let utf8 = cpclib_common::camino::Utf8PathBuf::from_path_buf(temporary.clone())
        .map_err(|_| "the temporary directory is not utf-8".to_string())?;
    env.save_sna(&utf8)
        .map_err(|e| format!("cannot write the snapshot: {e}"))?;
    let snapshot = fs_err::read(&temporary).map_err(|e| format!("cannot read it back: {e}"))?;
    let _ = fs_err::remove_file(&temporary);

    // The labels come from the same assemble as the addresses; taking them
    // from anywhere else would risk describing a different build.
    //
    // Both kinds count. A plain label is an `Address`, but the
    // self-modifying-code idiom - `ld a,0 : .activated equ $-1` - defines an
    // *expression*, and those are exactly the ones worth watching. Taking only
    // addresses silently loses them.
    let mut symbols = std::collections::HashMap::new();
    // Which of them are a *place* rather than a value that equals one: only a
    // label should ever name a call frame.
    let mut address_symbols = std::collections::HashSet::new();
    for (name, _) in env.symbols().expression_symbol() {
        let symbol = name.value().to_string();
        let address = match env.symbols().address_value(name.value()) {
            Ok(Some(address)) => {
                address_symbols.insert(symbol.clone());
                Some(u32::from(address.address()))
            },
            _ => {
                env.symbols()
                    .int_value(name.value())
                    .ok()
                    .flatten()
                    .and_then(|value| u32::try_from(value).ok())
            },
        };
        if let Some(address) = address {
            symbols.insert(symbol, address);
        }
    }

    let breakpoints = env.assembled_breakpoints();
    let image = env.sna().memory_dump()?;
    let entry_point = match env.sna().get_value(&cpclib_sna::SnapshotFlag::Z80_PC) {
        cpclib_sna::FlagValue::Word(pc) => Some(pc),
        cpclib_sna::FlagValue::Byte(pc) => Some(pc as u16),
        _ => None
    };

    // A direct-file launch has no build behind it, so nothing else will ever
    // write a map or a snapshot for it - unlike the rule-based launch, whose
    // cache is the project's own build output. Written here, best-effort, so
    // the *next* unmodified launch of this same entry can skip this assemble
    // entirely (see `cached_program_for_debug`).
    let wanted_definitions: std::collections::BTreeMap<String, String> = definitions
        .values
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    write_program_cache(
        entry,
        &raw,
        &symbols,
        &wanted_definitions,
        &breakpoints,
        &image,
        entry_point,
        &address_symbols,
        &snapshot
    );

    Ok(Launched {
        breakpoints,
        image,
        entry_point,
        source_map: SourceMap::from_raw(&raw)
            .resolved_against(entry)
            .with_symbols(symbols)
            .with_address_symbols(address_symbols),
        snapshot,
        entry: entry.to_path_buf()
    })
}

/// [`assemble_for_debug`], with basm's own internal (parse/load/pass/save)
/// progress reported over `progress` while it runs - the direct-file launch
/// path's own counterpart to `build_rule_for_debug`'s progress support. No
/// `Rule`/`Task` updates are possible here (there is no `BndBuilder`/rule at
/// all on this path), only `ProgressUpdate::Asm` ones.
pub fn assemble_for_debug_with_progress(
    entry: &Path,
    config: &AsmConfig,
    progress: Option<std::sync::mpsc::Sender<cpclib_bndbuild::progress::ProgressUpdate>>
) -> Result<Launched, String> {
    let _sink_guard = progress.map(DapProgressSinkGuard::install);
    assemble_for_debug(entry, config)
}

/// Where cpclib-dap's own cache of a direct-file launch lives, keyed by the
/// entry's canonical path so two projects with the same file name never
/// collide, and named `.map.json`/`.sna` to match `cached_for_debug`'s
/// terminology even though nothing here comes from a project build.
fn program_cache_paths(entry: &Path) -> (PathBuf, PathBuf) {
    use std::hash::{Hash, Hasher};
    let canonical = fs_err::canonicalize(entry).unwrap_or_else(|_| entry.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical.hash(&mut hasher);
    let key = hasher.finish();
    let dir = std::env::temp_dir();
    (
        dir.join(format!("cpclib-dap-cache-{key:016x}.map.json")),
        dir.join(format!("cpclib-dap-cache-{key:016x}.sna"))
    )
}

/// Best-effort: a write failure here must never fail the launch that just
/// succeeded, so every error is silently swallowed.
#[allow(clippy::too_many_arguments)]
fn write_program_cache(
    entry: &Path,
    raw: &cpclib_asm::assembler::listing_output::RawSourceMap,
    symbols: &std::collections::HashMap<String, u32>,
    definitions: &std::collections::BTreeMap<String, String>,
    breakpoints: &[cpclib_asm::assembler::delayed_command::AssembledBreakpoint],
    image: &[u8],
    entry_point: Option<u16>,
    address_symbols: &std::collections::HashSet<String>,
    snapshot: &[u8]
) {
    let (map_path, snapshot_path) = program_cache_paths(entry);
    let file = SourceMapFile::new(raw.clone(), symbols.clone(), definitions.clone())
        .with_program(breakpoints.to_vec(), image, entry_point)
        .with_address_symbols(address_symbols.iter().cloned().collect());
    if let Ok(json) = serde_json::to_string(&file) {
        let _ = fs_err::write(&map_path, json);
        let _ = fs_err::write(&snapshot_path, snapshot);
    }
}

/// Everything a *direct-file* debug launch needs, read from cpclib-dap's own
/// cache of a previous launch of this exact entry - instead of assembled
/// again.
///
/// Unlike `cached_for_debug` (which reads a map the project's own build
/// wrote), a `program` launch has no build behind it at all: nothing else
/// ever produces a source map for it, so nobody else can have cached one.
/// This is written and read by cpclib-dap itself (see `write_program_cache`),
/// keyed by the entry's canonical path, and invalidated the same way
/// `cached_for_debug` is: version, `-D` definitions, and mtime.
pub fn cached_program_for_debug(
    entry: &Path,
    _config: &AsmConfig,
    why: &mut Vec<String>
) -> Option<Launched> {
    let (map_path, snapshot_path) = program_cache_paths(entry);
    let text = fs_err::read_to_string(&map_path).ok()?;
    let file = SourceMapFile::from_json(&text)?;

    let build = cpclib_project::build_defs::definitions_for_entry(entry);
    let wanted: std::collections::BTreeMap<String, String> = build
        .values
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    if !file.assembled_with(&wanted) {
        why.push(
            "the cached debug launch was assembled with different -D definitions, \
             so it is ignored and the program is assembled again."
                .to_string()
        );
        return None;
    }

    let written = fs_err::metadata(&map_path).ok()?.modified().ok()?;
    let entry_is_older = fs_err::metadata(entry)
        .ok()?
        .modified()
        .ok()
        .is_some_and(|source| source <= written);
    if !entry_is_older {
        why.push(
            "the source has changed since the last debug launch, so it is assembled again."
                .to_string()
        );
        return None;
    }
    for name in &file.map.files {
        let source = fs_err::metadata(name).ok().and_then(|m| m.modified().ok())?;
        if source > written {
            why.push(format!(
                "{name} changed since the last debug launch, so it is assembled again."
            ));
            return None;
        }
    }

    let snapshot = fs_err::read(&snapshot_path).ok()?;

    Some(Launched {
        breakpoints: file.breakpoints.clone(),
        image: file.image_bytes(),
        entry_point: file.entry_point,
        source_map: SourceMap::from_raw(&file.map)
            .resolved_against(entry)
            .with_symbols(file.symbols.clone().into_iter().collect())
            .with_address_symbols(file.address_symbols.iter().cloned().collect()),
        snapshot,
        entry: entry.to_path_buf()
    })
}

/// Everything a debug session needs, read from `basm --sourcemap` instead of
/// assembled again.
///
/// The whole reason a debug launch is slow: the project's build already
/// assembled this program, and the adapter assembled it a second time to learn
/// where the lines went. If the build wrote the map, that second assemble is
/// pure waste - 32 seconds of it, on a real demo, against sixteen milliseconds
/// to read the file.
///
/// `None` whenever anything is not certainly right. A map that does not match
/// the program is far worse than a slow launch: every line it reports looks
/// plausible and is wrong, and nothing says so.
///
/// The snapshot is **not** in it, so this is only for the caller that has the
/// snapshot already - a rule-based launch, where the bytes come from the rule's
/// own output.
pub fn cached_for_debug(
    entry: &Path,
    config: &AsmConfig,
    why: &mut Vec<String>
) -> Option<Launched> {
    // Where the build says it writes the map, not where we would have guessed.
    // The name is the user's - `sna.map`, `birthtro.map`, `build/sna.map` - and
    // it is written in the same command as the `-D` values, so both are read
    // from there. Falling back to the entry's own name only helps someone who
    // happened to pick it.
    let build = cpclib_project::build_defs::definitions_for_entry(entry);
    let path = build
        .source_map
        .clone()
        .unwrap_or_else(|| entry.with_extension("map"));
    let text = match fs_err::read_to_string(&path) {
        Ok(text) => text,
        Err(problem) => {
            why.push(format!(
                "no source map at {} ({problem}), so the program is assembled again. \
                 Add `--sourcemap {}` to the basm command in your build file to skip that.",
                path.display(),
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            return None;
        }
    };
    let Some(file) = SourceMapFile::from_json(&text)
    else {
        why.push(format!(
            "{} was written by another version of basm, so it is ignored and the \
             program is assembled again.",
            path.display()
        ));
        return None;
    };

    // Assembled from other `-D` values is another program under the same file
    // names - the one difference nothing else here could notice.
    let wanted: std::collections::BTreeMap<String, String> = build
        .values
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    if !file.assembled_with(&wanted) {
        why.push(format!(
            "{} was assembled with different -D definitions ({:?}) than this build passes \
             ({:?}), so it describes another program and is ignored.",
            path.display(),
            file.definitions,
            wanted
        ));
        return None;
    }

    // Older than any source it describes means the sources moved on.
    let written = fs_err::metadata(&path).ok()?.modified().ok()?;
    let entry_is_older = fs_err::metadata(entry)
        .ok()?
        .modified()
        .ok()
        .is_some_and(|source| source <= written);
    if !entry_is_older {
        why.push(format!(
            "{} is older than {}, so it is ignored.",
            path.display(),
            entry.display()
        ));
        return None;
    }
    for name in &file.map.files {
        let Some(source) = fs_err::metadata(name).ok().and_then(|m| m.modified().ok())
        else {
            why.push(format!(
                "{name} is named by the source map but cannot be read."
            ));
            return None;
        };
        if source > written {
            why.push(format!(
                "{name} is newer than {}, so the map is stale and is ignored.",
                path.display()
            ));
            return None;
        }
    }

    let _ = config;
    Some(Launched {
        breakpoints: file.breakpoints.clone(),
        image: file.image_bytes(),
        entry_point: file.entry_point,
        source_map: SourceMap::from_raw(&file.map)
            .resolved_against(entry)
            .with_symbols(file.symbols.clone().into_iter().collect())
            .with_address_symbols(file.address_symbols.iter().cloned().collect()),
        // The caller brings its own; this path exists for the launch that
        // already has one.
        snapshot: Vec::new(),
        entry: entry.to_path_buf()
    })
}

/// Everything a rule-based launch needs, read out of the build file.
pub struct RuleLaunch {
    /// The snapshot the rule's emulator command names.
    pub snapshot: PathBuf,
    /// The source the snapshot is assembled from, when the build says so.
    pub entry: Option<PathBuf>,
    /// The rewritten command, for reporting what will be run.
    pub command: String
}

/// Build what `target` needs, and work out what it would have launched.
///
/// **Only the dependencies are built, never the rule itself.** A rule like
///
/// ```yaml
/// - tgt: test_sna
///   dep: {{ SNA }}
///   cmd: -emu --emulator ace --snapshot {{SNA}} run
/// ```
///
/// *is* the emulator launch: executing it starts Winape, which is exactly what
/// debugging must not do - the debuggable emulator is the one we serve. So its
/// dependencies are built (which is what actually produces the snapshot, with
/// the project's own assembler flags) and its command is *read* rather than
/// run. The `run` -> `debug` rewrite is still applied, because it is the
/// canonical description of "the same thing, for debugging" - here only its
/// arguments are used, to find which snapshot to serve.
/// The debuggable targets one embedded block declares.
fn embedded_block_targets(
    build_file: &cpclib_common::camino::Utf8Path,
    yaml_text: &str
) -> Vec<String> {
    let working_directory = build_file.parent().map(|d| d.to_owned());
    let synthetic_name =
        cpclib_common::camino::Utf8PathBuf::from(format!("{build_file} (embedded)"));
    let Ok(rendered) = cpclib_bndbuild::BndBuilder::decode_from_reader(
        std::io::Cursor::new(yaml_text.as_bytes()),
        working_directory.as_ref(),
        &Vec::<(String, String)>::new(),
        &synthetic_name
    )
    else {
        return Vec::new();
    };
    let Ok(builder) =
        cpclib_bndbuild::BndBuilder::from_string(rendered, Some(&synthetic_name), false)
    else {
        return Vec::new();
    };
    debuggable_targets_of(&builder)
}

/// Load the rules of a build file - standalone, or embedded in a `.asm`.
///
/// A project small enough not to want a separate build file can keep its rules
/// in its source's own comments, behind a `#!bndbuild` marker. Those are still
/// rules, and a rule that launches an emulator is still debuggable, so the
/// difference is confined to how the YAML is found: after that both go through
/// the same builder.
fn builder_for(
    build_file: &cpclib_common::camino::Utf8Path,
    target: &str
) -> Result<cpclib_bndbuild::BndBuilder, String> {
    if !is_embedded_host(build_file) {
        return cpclib_bndbuild::BndBuilder::from_path(build_file, true)
            .map(|(_, builder)| builder)
            .map_err(|e| format!("cannot read {build_file}: {e}"));
    }

    let text = fs_err::read_to_string(build_file)
        .map_err(|e| format!("cannot read {build_file}: {e}"))?;
    let blocks = cpclib_project::embedded_build::blocks_in_source(&text);
    if blocks.is_empty() {
        return Err(format!(
            "{build_file} contains no `#!bndbuild` block, so there is no rule \
             '{target}' to debug"
        ));
    }

    // Relative paths in an embedded rule are relative to the file it lives in,
    // exactly as they would be in a build file sitting there.
    let working_directory = build_file.parent().map(|d| d.to_owned());
    let synthetic_name =
        cpclib_common::camino::Utf8PathBuf::from(format!("{build_file}#{target} (embedded)"));

    // The block that actually declares this target. Several blocks in one file
    // are allowed, and picking the first would debug the wrong rule.
    let mut last_error = None;
    for block in &blocks {
        let rendered = cpclib_bndbuild::BndBuilder::decode_from_reader(
            std::io::Cursor::new(block.yaml_text.as_bytes()),
            working_directory.as_ref(),
            &Vec::<(String, String)>::new(),
            &synthetic_name
        );
        let rendered = match rendered {
            Ok(rendered) => rendered,
            Err(problem) => {
                last_error = Some(problem.to_string());
                continue;
            }
        };
        match cpclib_bndbuild::BndBuilder::from_string(rendered, Some(&synthetic_name), false) {
            Ok(builder) => {
                if builder
                    .rules()
                    .iter()
                    .any(|rule| rule.targets().iter().any(|t| t.as_str() == target))
                {
                    return Ok(builder);
                }
            },
            Err(problem) => last_error = Some(problem.to_string())
        }
    }

    Err(match last_error {
        Some(problem) => {
            format!("the `#!bndbuild` block in {build_file} could not be read: {problem}")
        },
        None => format!("no `#!bndbuild` block in {build_file} builds '{target}'")
    })
}

/// Whether this path is a source file carrying its rules in comments rather
/// than a build file of its own.
fn is_embedded_host(path: &cpclib_common::camino::Utf8Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("asm"))
}

/// Reacts to `StartRule`/`StartTask`, forwarding them as
/// `ProgressUpdate::Rule`/`ProgressUpdate::Task` over a plain
/// `std::sync::mpsc::Sender` - `build_rule_for_debug` runs synchronously on
/// whatever thread `run_stdio`'s "launch" handler spawns for it (see that
/// module's own doc), so there is no tokio runtime here to hand an
/// `UnboundedSender` to, unlike the LSP server's equivalent
/// `RuleProgressObserver`.
#[derive(Debug)]
struct DapRuleProgressObserver(std::sync::mpsc::Sender<cpclib_bndbuild::progress::ProgressUpdate>);

impl cpclib_common::event::EventObserver for DapRuleProgressObserver {
    fn emit_stdout(&self, _s: &str) {}

    fn emit_stderr(&self, _s: &str) {}
}

impl cpclib_bndbuild::event::BndBuilderObserver for DapRuleProgressObserver {
    fn update(&self, event: cpclib_bndbuild::event::BndBuilderEvent) {
        let update = match event {
            cpclib_bndbuild::event::BndBuilderEvent::StartRule { rule, nb, out_of } => {
                Some(cpclib_bndbuild::progress::ProgressUpdate::Rule {
                    rule: rule.to_string(),
                    nb,
                    out_of
                })
            },
            cpclib_bndbuild::event::BndBuilderEvent::StartTask(_rule, task) => {
                Some(cpclib_bndbuild::progress::ProgressUpdate::Task {
                    command: task.to_string()
                })
            },
            _ => None
        };
        if let Some(update) = update {
            let _ = self.0.send(update);
        }
    }
}

/// Forwards basm's internal [`cpclib_asm::progress::AsmProgressEvent`]s to
/// the same channel `DapRuleProgressObserver` uses, so a single consumer
/// sees rule-level and basm-internal progress interleaved in real time.
struct DapAsmProgressForwarder(std::sync::mpsc::Sender<cpclib_bndbuild::progress::ProgressUpdate>);

impl cpclib_asm::progress::AsmProgressSink for DapAsmProgressForwarder {
    fn on_progress(&self, event: cpclib_asm::progress::AsmProgressEvent) {
        let _ = self.0.send(cpclib_bndbuild::progress::ProgressUpdate::Asm(event));
    }
}

/// Installs a [`DapAsmProgressForwarder`] on this thread and broadcast onto
/// the process's rayon global pool for the guard's lifetime, restoring
/// `None` everywhere on drop (including on an early return, since
/// restoration happens in `Drop`, not after a fallible call).
struct DapProgressSinkGuard;

impl DapProgressSinkGuard {
    fn install(tx: std::sync::mpsc::Sender<cpclib_bndbuild::progress::ProgressUpdate>) -> Self {
        let sink: std::sync::Arc<dyn cpclib_asm::progress::AsmProgressSink> =
            std::sync::Arc::new(DapAsmProgressForwarder(tx));
        cpclib_asm::progress::install_progress_sink(Some(std::sync::Arc::clone(&sink)));
        cpclib_common::rayon::broadcast(|_| {
            cpclib_asm::progress::install_progress_sink(Some(std::sync::Arc::clone(&sink)));
        });
        Self
    }
}

impl Drop for DapProgressSinkGuard {
    fn drop(&mut self) {
        cpclib_asm::progress::install_progress_sink(None);
        cpclib_common::rayon::broadcast(|_| {
            cpclib_asm::progress::install_progress_sink(None);
        });
    }
}

pub fn build_rule_for_debug(
    build_file: &Path,
    target: &str,
    progress: Option<std::sync::mpsc::Sender<cpclib_bndbuild::progress::ProgressUpdate>>
) -> Result<RuleLaunch, String> {
    let build_file = cpclib_common::camino::Utf8Path::from_path(build_file)
        .ok_or("the build file path is not utf-8")?;
    let mut builder = builder_for(build_file, target)?;

    // Basm's own internal (parse/load/pass/save) progress, forwarded to the
    // same channel - see `cpclib_asm::progress::AsmProgressSink`'s own doc
    // for why a plain thread-local install alone would silently miss
    // `INCLUDE`-parsing work, which runs on rayon worker threads. Safe to
    // broadcast onto the process-wide rayon global pool here (unlike the
    // LSP server's own dedicated-pool `run_with_progress_sink`): a
    // `cpclib-dap` process handles one "launch" at a time, so there is no
    // second, concurrent build sharing it to contaminate.
    let _sink_guard = progress.clone().map(DapProgressSinkGuard::install);

    if let Some(tx) = progress {
        use cpclib_bndbuild::event::BndBuilderObserved;
        builder.add_observer(cpclib_bndbuild::event::BndBuilderObserverRc::new(
            DapRuleProgressObserver(tx)
        ));
    }

    let rule = builder
        .rules()
        .iter()
        .find(|rule| rule.targets().iter().any(|t| t.as_str() == target))
        .ok_or_else(|| format!("no rule builds '{target}' in {build_file}"))?;

    // Build everything the rule needs - the snapshot among them - without
    // running the rule, whose command is the emulator launch itself.
    let known: Vec<String> = builder.targets().iter().map(|t| t.to_string()).collect();
    for dependency in rule.dependencies() {
        // A dependency that is a plain source file has no rule to run; only
        // the ones something builds are worth asking for.
        if known.iter().any(|t| t == dependency.as_str()) {
            builder.execute(dependency).map_err(|e| {
                format!("building '{dependency}', needed by '{target}', failed: {e}")
            })?;
        }
    }

    for task in rule.commands() {
        let rendered = task.to_string();
        let Some((program, arguments)) = rendered.split_once(' ')
        else {
            continue;
        };
        // A tool may be prefixed with `-` to ignore its errors.
        let program = program.strip_prefix('-').unwrap_or(program);
        if !cpclib_bndbuild::task::EMUCTRL_CMDS.contains(&program) {
            continue;
        }
        let Some(rewritten) = cpclib_bndbuild::pipeline::debug::debug_arguments(arguments)
        else {
            continue;
        };
        let snapshot =
            cpclib_bndbuild::pipeline::debug::snapshot_of(&rewritten).ok_or_else(|| {
                format!("'{target}' launches an emulator but names no --snapshot to debug")
            })?;
        let snapshot = build_file
            .parent()
            .map(|dir| dir.join(&snapshot))
            .unwrap_or_else(|| cpclib_common::camino::Utf8PathBuf::from(&snapshot));
        // Which source produces that snapshot is stated by the build itself -
        // `basm --snapshot sna.asm -o birthtro.sna ...` - and reading it is
        // exact where guessing from `RUN` directives is not: a project with
        // tests has several of those and no way to choose between them.
        let entry =
            entry_building(&builder, snapshot.file_name().unwrap_or_default()).map(|source| {
                build_file
                    .parent()
                    .map(|dir| dir.join(&source))
                    .unwrap_or_else(|| cpclib_common::camino::Utf8PathBuf::from(&source))
                    .into_std_path_buf()
            });

        return Ok(RuleLaunch {
            snapshot: snapshot.into_std_path_buf(),
            entry,
            command: rewritten
        });
    }

    Err(format!(
        "'{target}' has no emulator command to debug - looked for one of {:?} with a `run`",
        cpclib_bndbuild::task::EMUCTRL_CMDS
    ))
}

/// The assembler source a rule builds `target` from.
///
/// Read out of the `basm` invocation rather than inferred: the build file is
/// the only thing that actually knows, and a project with a `tests/` directory
/// has many files carrying a `RUN` with no way to pick between them.
fn entry_building(
    builder: &cpclib_bndbuild::BndBuilder,
    target: &str
) -> Option<cpclib_common::camino::Utf8PathBuf> {
    let rule = builder
        .rules()
        .iter()
        .find(|rule| rule.targets().iter().any(|t| t.file_name() == Some(target)))?;

    for task in rule.commands() {
        let rendered = task.to_string();
        let mut words = rendered.split_whitespace();
        let program = words.next()?;
        let program = program.strip_prefix('-').unwrap_or(program);
        if program != "basm" {
            continue;
        }
        // The first bare `.asm`: options carry their values after a `-flag`,
        // and none of basm's take a source file as an option value.
        if let Some(source) =
            words.find(|word| !word.starts_with('-') && word.to_ascii_lowercase().ends_with(".asm"))
        {
            return Some(cpclib_common::camino::Utf8PathBuf::from(source));
        }
    }
    None
}

/// The rules of a build file that could be debugged.
///
/// Used to offer a list rather than making the user remember rule names.
pub fn debuggable_rules(build_file: &Path) -> Vec<String> {
    let Some(build_file) = cpclib_common::camino::Utf8Path::from_path(build_file)
    else {
        return Vec::new();
    };
    // `builder_for` needs a target to pick between several embedded blocks;
    // here every block counts, so each is asked in turn.
    if is_embedded_host(build_file) {
        let Ok(text) = fs_err::read_to_string(build_file)
        else {
            return Vec::new();
        };
        return cpclib_project::embedded_build::blocks_in_source(&text)
            .iter()
            .flat_map(|block| embedded_block_targets(build_file, &block.yaml_text))
            .collect();
    }
    let Ok((_, builder)) = cpclib_bndbuild::BndBuilder::from_path(build_file, true)
    else {
        return Vec::new();
    };

    debuggable_targets_of(&builder)
}

/// The nearby build file + rule that would build and launch `entry` in an
/// emulator, if one exists.
///
/// A direct assemble (`assemble_for_debug`) has no dependency resolution
/// behind it - an `incbin`/`include` of a generated asset a build rule would
/// normally produce first (a converted screen, a packed sprite sheet, ...)
/// fails outright, even though the project's own build file already knows
/// how to produce it. When a nearby build file names a rule reachable
/// (through [`entry_building`]'s "which source does the snapshot this rule
/// launches come from" lookup, the same one [`build_rule_for_debug`] itself
/// uses) from `entry`, running *that* rule is the more correct answer: it
/// builds every dependency the project's own build system knows about, not
/// just whatever happens to already be on disc.
///
/// Deliberately reuses "debuggable" (a rule ending its emulator command in a
/// bare `run` token, per [`debuggable_targets_of`]/[`pipeline::debug::
/// debug_arguments`](cpclib_bndbuild::pipeline::debug::debug_arguments)) as
/// the definition of "launches an emulator" here too, rather than a second,
/// looser notion of "runnable" - every rule shaped to be run is already
/// shaped to be debugged, and vice versa, so one concept covers both
/// callers.
///
/// `None` when no candidate build file (searched the same way
/// [`cpclib_project::build_defs::definitions_for_entry`] does: `entry`'s own
/// directory, then upwards to the project root) names such a rule - the
/// caller then falls back to a direct assemble, exactly today's behaviour.
pub fn find_debuggable_rule_for_entry(entry: &Path) -> Option<(PathBuf, String)> {
    let canonical_entry = fs_err::canonicalize(entry).ok()?;

    for build_file in cpclib_project::build_defs::candidate_build_files(entry) {
        let build_file_utf8 = cpclib_common::camino::Utf8Path::from_path(&build_file)?;
        let Ok((_, builder)) = cpclib_bndbuild::BndBuilder::from_path(build_file_utf8, true)
        else {
            continue;
        };

        for rule in builder.rules() {
            let Some(rewritten) = rule.commands().iter().find_map(|task| {
                let rendered = task.to_string();
                let (program, arguments) = rendered.split_once(' ')?;
                let program = program.strip_prefix('-').unwrap_or(program);
                if !cpclib_bndbuild::task::EMUCTRL_CMDS.contains(&program) {
                    return None;
                }
                cpclib_bndbuild::pipeline::debug::debug_arguments(arguments)
            })
            else {
                continue;
            };
            let Some(snapshot) = cpclib_bndbuild::pipeline::debug::snapshot_of(&rewritten)
            else {
                continue;
            };
            let snapshot_name =
                cpclib_common::camino::Utf8Path::new(&snapshot).file_name().unwrap_or_default();
            let Some(source) = entry_building(&builder, snapshot_name)
            else {
                continue;
            };
            let resolved = build_file_utf8
                .parent()
                .map(|dir| dir.join(&source))
                .unwrap_or(source);
            let matches = fs_err::canonicalize(resolved.as_std_path())
                .is_ok_and(|p| p == canonical_entry);
            if matches {
                let target = rule.targets().first()?.to_string();
                return Some((build_file, target));
            }
        }
    }
    None
}

/// The targets of every rule in `builder` that launches an emulator.
fn debuggable_targets_of(builder: &cpclib_bndbuild::BndBuilder) -> Vec<String> {
    builder
        .rules()
        .iter()
        .filter(|rule| {
            rule.commands().iter().any(|task| {
                let rendered = task.to_string();
                let Some((program, arguments)) = rendered.split_once(' ')
                else {
                    return false;
                };
                // A tool may be prefixed with `-` to ignore its errors.
                let program = program.strip_prefix('-').unwrap_or(program);
                cpclib_bndbuild::task::EMUCTRL_CMDS.contains(&program)
                    && cpclib_bndbuild::pipeline::debug::debug_arguments(arguments).is_some()
            })
        })
        .flat_map(|rule| rule.targets().iter().map(|t| t.to_string()))
        .collect()
}

#[cfg(test)]
mod progress_tests {
    use serial_test::serial;

    use super::*;

    /// Proves the actual point of wiring `build_rule_for_debug` into
    /// `cpclib_bndbuild::progress`: a real debug-session "launch" against a
    /// `rule`, exactly like the one a real project's `test_sna`/`debug_sna`
    /// launch drives, must report both the rule-level "step N of M" signal
    /// and basm's own internal (parse/load/pass/save) progress while the
    /// assemble it depends on runs - not leave the channel empty and make
    /// the caller fall back to an indeterminate spinner for however long
    /// that assemble takes.
    ///
    /// `#[serial(cwd)]`: `BndBuilder::from_path` (reached via
    /// `build_rule_for_debug`) changes the process's current directory as a
    /// side effect, which races against every other test elsewhere in this
    /// crate that resolves its own fixture through a relative path - shares
    /// the `cwd` key with all of them (`lib.rs`, `inspect.rs`).
    /// Restores the process's current directory on drop - `BndBuilder::
    /// from_path` (via `decode_from_reader`) sets it to the build file's own
    /// directory as a side effect and never puts it back (see that
    /// function's own "XXX here it is problematic to modify work dir"
    /// comment), which would otherwise permanently break every other test
    /// in this crate that resolves a fixture through a relative path
    /// (`lib.rs`'s and `inspect.rs`'s own `#[serial(cwd)]` tests) - not just
    /// while this one runs, but for the rest of the process, regardless of
    /// run order.
    struct RestoreCwd(std::path::PathBuf);
    impl Drop for RestoreCwd {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    #[test]
    #[serial(cwd)]
    fn build_rule_for_debug_reports_progress_for_its_own_dependency_build() {
        let _restore_cwd = RestoreCwd(std::env::current_dir().unwrap());

        let dir = std::env::temp_dir().join(format!(
            "cpclib-dap-progress-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("build.bnd"),
            "- tgt: main.bin\n  dep: main.asm\n  cmd: basm main.asm -o main.bin\n\n\
             - tgt: test_launch\n  dep: main.bin\n  cmd: emu --snapshot main.bin run\n"
        )
        .unwrap();
        std::fs::write(dir.join("main.asm"), "    org 0x4000\n    nop\n    ret\n").unwrap();

        let (progress_tx, progress_rx) = std::sync::mpsc::channel();
        let launched = build_rule_for_debug(
            &dir.join("build.bnd"),
            "test_launch",
            Some(progress_tx)
        )
        .unwrap_or_else(|e| panic!("build_rule_for_debug failed: {e}"));
        assert_eq!(launched.snapshot, dir.join("main.bin"));

        let updates: Vec<_> = progress_rx.try_iter().collect();
        assert!(
            updates
                .iter()
                .any(|u| matches!(u, cpclib_bndbuild::progress::ProgressUpdate::Rule { .. })),
            "expected the rule-level '[1/1]' signal, got: {updates:?}"
        );
        assert!(
            updates.iter().any(|u| matches!(
                u,
                cpclib_bndbuild::progress::ProgressUpdate::Asm(
                    cpclib_asm::progress::AsmProgressEvent::PassStarted { .. }
                )
            )),
            "expected basm's own internal pass progress while its dependency \
             build ran, got: {updates:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Same proof as `build_rule_for_debug_reports_progress_...` above, for
    /// the direct-file ("program", no `rule`) launch path -
    /// `assemble_for_debug_with_progress` has no `BndBuilder`/rule to fire
    /// `Rule`/`Task` updates from, only basm's own internal progress, but
    /// that must still flow.
    #[test]
    fn assemble_for_debug_with_progress_reports_basm_internal_progress() {
        let dir = std::env::temp_dir().join(format!(
            "cpclib-dap-progress-program-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("main.asm");
        std::fs::write(&entry, "    org 0x4000\n    run $\n    nop\n    ret\n").unwrap();

        let (progress_tx, progress_rx) = std::sync::mpsc::channel();
        let config = cpclib_project::config::AsmConfig::default();
        let result = assemble_for_debug_with_progress(&entry, &config, Some(progress_tx));
        assert!(result.is_ok(), "{:?}", result.err());

        let updates: Vec<_> = progress_rx.try_iter().collect();
        assert!(
            updates.iter().any(|u| matches!(
                u,
                cpclib_bndbuild::progress::ProgressUpdate::Asm(
                    cpclib_asm::progress::AsmProgressEvent::Parse { .. }
                )
            )),
            "expected basm's own internal Parse progress, got: {updates:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The actual bug this whole lookup exists to fix: "Run in emulator" on
    /// `sna.asm` failing because a direct assemble has no dependency
    /// resolution, so a generated asset (`face3_cpc.scr`, produced by a
    /// separate rule) is never built first. `test_launch` here mirrors that
    /// real project's shape exactly - a debuggable rule (`emu ... run`)
    /// depending on a snapshot built from `main.asm` by another rule.
    #[test]
    #[serial(cwd)]
    fn find_debuggable_rule_for_entry_matches_the_rule_that_builds_the_entry() {
        let _restore_cwd = RestoreCwd(std::env::current_dir().unwrap());

        let dir = std::env::temp_dir().join(format!(
            "cpclib-dap-find-rule-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("build.bnd"),
            "- tgt: main.bin\n  dep: main.asm\n  cmd: basm main.asm -o main.bin\n\n\
             - tgt: test_launch\n  dep: main.bin\n  cmd: emu --snapshot main.bin run\n"
        )
        .unwrap();
        std::fs::write(dir.join("main.asm"), "    org 0x4000\n    nop\n    ret\n").unwrap();

        let found = find_debuggable_rule_for_entry(&dir.join("main.asm"));
        assert_eq!(
            found,
            Some((dir.join("build.bnd"), "test_launch".to_string())),
            "{found:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A source no nearby build file mentions falls back to `None` - the
    /// caller then takes the direct-assemble path, exactly today's
    /// behaviour for a standalone file.
    #[test]
    #[serial(cwd)]
    fn find_debuggable_rule_for_entry_is_none_when_no_rule_builds_the_source() {
        let _restore_cwd = RestoreCwd(std::env::current_dir().unwrap());

        let dir = std::env::temp_dir().join(format!(
            "cpclib-dap-find-rule-none-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("standalone.asm"), "    org 0x4000\n    ret\n").unwrap();

        assert!(find_debuggable_rule_for_entry(&dir.join("standalone.asm")).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
