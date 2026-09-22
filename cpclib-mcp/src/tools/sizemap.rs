//! `size_map` - where do the bytes go? Assembles a source and reports, per
//! global label, how many bytes it spans, and (optionally) what each region
//! costs *after crunching*.
//!
//! Symbols come from the assembler's own table (only real addresses, so an
//! `equ` constant never pollutes the map) rather than from a `.sym` file,
//! which a project may export only in part (etchy exports 18 symbols). The
//! per-line detail comes from the assembler's source map - the structured
//! form of the listing, which knows far more than a symbol table: for every
//! emitted run its file, line, columns, byte count and whether it is code or
//! data, with macro expansions recorded as contexts of their own. That is
//! what lets bytes be attributed to a source file, to a macro (the cost of
//! its call sites), or to one source line. The
//! crunched cost of a region is measured, not estimated: the whole image is
//! crunched, then again with that region removed, and the difference is the
//! region's contribution. It is what a size-limited intro actually needs -
//! raw size and crunched size can rank routines very differently.

use std::sync::Arc;

use camino::Utf8Path;
use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};
use cpclib_asm::assembler::{CrunchedSectionInfo, visit_tokens_all_passes_with_options};
use cpclib_asm::implementation::instructions::Compressor;
use cpclib_asm::parser::parse_z80_with_context_builder;
use cpclib_asm::parser::context::ParserOptions;
use cpclib_asm::{AssemblingOptions, EnvOptions};
use cpclib_tokens::symbols::{SymbolsTableTrait, Value};
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde_json::{Value as Json2, json};

use crate::McpServer;
use crate::error::{ToolError, ToolErrorKind, ToolResult};
use crate::tools::crunch::{CRUNCHER_TIMEOUT, compress_with_timeout, resolve_cruncher};

/// A local label (`routine.sub`) belongs to the global label before the dot.
fn is_local(name: &str) -> bool {
    name.contains('.')
}

/// One global label and the bytes up to the next one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Region {
    pub name: String,
    pub address: u32,
    pub bytes: u32,
    /// Local (`name.sub`) labels inside the span.
    pub locals: usize,
    /// Other global labels at exactly the same address.
    pub aliases: Vec<String>
}

/// Turns `(label, address)` pairs into regions covering `[start, end)`:
/// each distinct global-label address runs to the next one (or to `end`).
/// Labels outside the range are ignored. When a file has no global labels
/// in range at all, every label is treated as global.
pub(crate) fn build_regions(symbols: &[(String, u32)], start: u32, end: u32) -> Vec<Region> {
    let in_range: Vec<&(String, u32)> = symbols.iter().filter(|(_, a)| *a >= start && *a < end).collect();
    let has_globals = in_range.iter().any(|(n, _)| !is_local(n));
    let is_boundary = |n: &str| !has_globals || !is_local(n);

    let mut boundaries: Vec<&(String, u32)> = in_range.iter().copied().filter(|(n, _)| is_boundary(n)).collect();
    boundaries.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let mut regions: Vec<Region> = Vec::new();
    for (name, address) in boundaries {
        match regions.last_mut() {
            Some(prev) if prev.address == *address => prev.aliases.push(name.clone()),
            _ => {
                regions.push(Region {
                    name: name.clone(),
                    address: *address,
                    bytes: 0,
                    locals: 0,
                    aliases: Vec::new()
                })
            }
        }
    }
    for i in 0..regions.len() {
        let next = regions.get(i + 1).map_or(end, |r| r.address);
        regions[i].bytes = next - regions[i].address;
        let (lo, hi) = (regions[i].address, next);
        regions[i].locals = in_range
            .iter()
            .filter(|(n, a)| has_globals && is_local(n) && *a >= lo && *a < hi)
            .count();
    }
    regions
}

/// Crunched size with `range` cut out of `image` (`None` = nothing removed).
fn crunched_size(cruncher: &str, image: &[u8], range: Option<std::ops::Range<usize>>) -> Result<i64, String> {
    let data: Vec<u8> = match range {
        Some(r) => image[..r.start].iter().chain(&image[r.end..]).copied().collect(),
        None => image.to_vec()
    };
    compress_with_timeout(cruncher.to_string(), data, CRUNCHER_TIMEOUT).map(|c| c.stream.len() as i64)
}

/// `(baseline, per-region contribution)` - each region's contribution is
/// `baseline - crunched(image without the region)`, computed concurrently.
fn crunched_costs(
    cruncher: &str,
    image: &[u8],
    image_start: u32,
    regions: &[Region]
) -> Result<(i64, Vec<Option<i64>>), String> {
    let baseline = crunched_size(cruncher, image, None)?;
    let handles: Vec<_> = regions
        .iter()
        .map(|r| {
            let lo = r.address.saturating_sub(image_start) as usize;
            let hi = (lo + r.bytes as usize).min(image.len());
            let (cruncher, image) = (cruncher.to_string(), image.to_vec());
            std::thread::spawn(move || {
                if lo >= hi {
                    return None;
                }
                crunched_size(&cruncher, &image, Some(lo..hi)).ok().map(|without| baseline - without)
            })
        })
        .collect();
    Ok((baseline, handles.into_iter().map(|h| h.join().unwrap_or(None)).collect()))
}

/// `main.asm:121:1 > MACRO DRAW:` -> (`main.asm`, Some(`MACRO DRAW`)); a plain
/// file context -> (that file, None). Nested expansions report the innermost
/// one. (The assembler has the same split privately; a context name is a
/// stable, documented shape - `path:LINE:COL > KIND NAME:`.)
pub(crate) fn split_context(name: &str) -> (String, Option<String>) {
    let Some(marker) = name.find(" > ") else {
        return (name.to_string(), None);
    };
    let mut head = &name[..marker];
    // Walk back over `:COL` then `:LINE` when both are numbers.
    for _ in 0..2 {
        match head.rsplit_once(':') {
            Some((rest, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => head = rest,
            _ => break
        }
    }
    let expansion = name.rsplit(" > ").next().unwrap_or("").trim().trim_end_matches(':').trim();
    (head.to_string(), (!expansion.is_empty()).then(|| expansion.to_string()))
}

/// Bytes attributed to one key (a file, a macro, a source line).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Tally {
    pub bytes: u64,
    pub code: u64,
    pub data: u64,
    /// Emitted runs - for a macro or a line, the number of places it was
    /// instantiated.
    pub runs: u64
}

impl Tally {
    fn add(&mut self, row: &SourceMapRow, len: u64) {
        self.bytes += len;
        if row.is_data {
            self.data += len;
        }
        else {
            self.code += len;
        }
        self.runs += 1;
    }
}

/// Everything the source map says about who emitted the bytes.
#[derive(Debug, Default)]
pub(crate) struct Attribution {
    /// Code/data bytes inside each region, parallel to the region list.
    pub per_region: Vec<Tally>,
    pub by_file: std::collections::BTreeMap<String, Tally>,
    pub by_macro: std::collections::BTreeMap<String, Tally>,
    pub by_line: std::collections::BTreeMap<(String, u32), Tally>,
    /// Image bytes no placeable row accounts for.
    pub unowned_image_bytes: u64,
    /// Bytes of decrunched-code rows (inside crunched sections), which are not
    /// in the image and so left out of every table.
    pub unplaced_row_bytes: u64
}

/// Where each source file defines its macros: `(first line, last line,
/// name)`, 1-based, body lines only.
///
/// The source map names a macro's expansion only sometimes; a row for a
/// macro body usually just points at the body's own line in the defining
/// file. Reading the definitions back is what lets those rows still be
/// charged to the macro.
#[derive(Debug, Default)]
pub(crate) struct MacroIndex(std::collections::HashMap<String, Vec<(u32, u32, String)>>);

impl MacroIndex {
    pub(crate) fn scan(files: &[String]) -> Self {
        let mut index = std::collections::HashMap::new();
        for file in files {
            let Ok(text) = fs_err::read_to_string(file) else { continue };
            let defs = Self::definitions(&text);
            if !defs.is_empty() {
                index.insert(file.clone(), defs);
            }
        }
        Self(index)
    }

    fn definitions(text: &str) -> Vec<(u32, u32, String)> {
        let mut defs = Vec::new();
        let mut open: Option<(u32, String)> = None;
        for (i, raw) in text.lines().enumerate() {
            let line = i as u32 + 1;
            let code = raw.split(';').next().unwrap_or("");
            let mut words = code.split_whitespace().map(|w| w.trim_end_matches(':'));
            let Some(first) = words.next() else { continue };
            let first_lower = first.to_ascii_lowercase();
            if open.is_none() && first_lower == "macro" {
                if let Some(name) = words.next() {
                    let name = name.split('(').next().unwrap_or(name);
                    open = Some((line + 1, name.to_string()));
                }
            }
            else if open.is_none() && words.clone().next().is_some_and(|w| w.eq_ignore_ascii_case("macro")) {
                // `NAME macro args`
                open = Some((line + 1, first.to_string()));
            }
            else if first_lower == "endm" || first_lower == "endmacro" {
                if let Some((start, name)) = open.take() {
                    if line > start {
                        defs.push((start, line - 1, name));
                    }
                }
            }
        }
        defs
    }

    fn macro_at(&self, file: &str, line: u32) -> Option<&str> {
        self.0
            .get(file)?
            .iter()
            .find(|(start, end, _)| (*start..=*end).contains(&line))
            .map(|(_, _, name)| name.as_str())
    }
}

/// Walks the map's rows once. Rows that emitted nothing (an `equ`, a
/// comment) are skipped.
///
/// Only bytes that survive into the final `image_range` count, the last row
/// to write an address owning it. That matters for crunched sections: the map
/// also holds the *uncrunched* rows that went in (an `incbin` of 12 KB), which
/// are overwritten by the much smaller crunched output - counting both would
/// make the sources look four times bigger than the file. The crunched output
/// itself is charged to the directive that produced it (e.g. `LZSHRINKLER`).
pub(crate) fn attribute(
    map: &RawSourceMap,
    regions: &[Region],
    macros: &MacroIndex,
    image_range: std::ops::Range<u32>
) -> Attribution {
    let mut out = Attribution {
        per_region: vec![Tally::default(); regions.len()],
        ..Default::default()
    };
    let contexts: Vec<(String, Option<String>)> = map.files.iter().map(|f| split_context(f)).collect();

    // Who owns each address of the image.
    let mut owner = vec![u32::MAX; image_range.len()];
    // Only rows that say where their bytes sit in the image. Inside a crunched
    // section the map describes the *decrunched* code: `logical` is where it
    // will run, `physical` an offset in the assembler's scratch buffer, so the
    // two disagree - those rows are counted apart, the section's real bytes
    // being the LZ directive's own (agreeing) rows.
    // The base 64K only: the same logical address in another bank is other memory.
    let placeable = |r: &&SourceMapRow| r.len > 0 && r.page == 0 && r.logical == r.physical;
    out.unplaced_row_bytes = map
        .rows
        .iter()
        .filter(|r| r.len > 0 && !placeable(r))
        .map(|r| r.len as u64)
        .sum();
    for (i, row) in map.rows.iter().enumerate().filter(|(_, r)| placeable(r)) {
        let from = row.logical.max(image_range.start);
        let to = (row.logical as u64 + row.len as u64).min(image_range.end as u64) as u32;
        for address in from..to {
            owner[(address - image_range.start) as usize] = i as u32;
        }
    }
    out.unowned_image_bytes = owner.iter().filter(|&&o| o == u32::MAX).count() as u64;
    let mut owned = vec![0u32; map.rows.len()];
    for &o in owner.iter().filter(|&&o| o != u32::MAX) {
        owned[o as usize] += 1;
    }

    for (row, &len) in map.rows.iter().zip(&owned).filter(|(_, l)| **l > 0) {
        let len = len as u64;
        let Some((file, macro_name)) = contexts.get(row.file as usize) else { continue };
        out.by_file.entry(file.clone()).or_default().add(row, len);
        let defined = macros.macro_at(file, row.line).map(|n| format!("MACRO {n}"));
        if let Some(m) = macro_name.clone().or(defined) {
            out.by_macro.entry(m).or_default().add(row, len);
        }
        out.by_line.entry((file.clone(), row.line)).or_default().add(row, len);
        // Regions are sorted by address: the last one starting at or before
        // this row holds it.
        let idx = regions.partition_point(|r| r.address <= row.logical);
        if idx > 0 && row.logical < regions[idx - 1].address + regions[idx - 1].bytes {
            out.per_region[idx - 1].add(row, len);
        }
    }
    out
}

/// A key/tally ranking as a markdown table, biggest first.
fn render_tally_table<K: std::fmt::Display>(title: &str, rows: Vec<(K, &Tally)>, top: usize, total: u64) -> String {
    let mut rows = rows;
    rows.sort_by(|a, b| b.1.bytes.cmp(&a.1.bytes).then_with(|| a.0.to_string().cmp(&b.0.to_string())));
    let mut out = format!("| # | {title} | Bytes | % of image | Code | Data | Instances |\n|--:|:--|--:|--:|--:|--:|--:|\n");
    for (i, (key, t)) in rows.into_iter().take(top).enumerate() {
        let pct = if total == 0 { 0.0 } else { t.bytes as f64 * 100.0 / total as f64 };
        out.push_str(&format!("| {} | {key} | {} | {pct:.1} | {} | {} | {} |\n", i + 1, t.bytes, t.code, t.data, t.runs));
    }
    out
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SizeMapInput {
    /// The `.asm` file to assemble (as a build would, so give the build's
    /// `include_dirs` / `defines` - see `project_context`).
    pub path: String,
    /// Extra `INCLUDE` search directories.
    pub include_dirs: Option<Vec<String>>,
    /// Symbols to define first, `NAME` or `NAME=VALUE`, like `basm -D`.
    pub defines: Option<Vec<String>>,
    /// Only regions at or above this address (default: where the produced
    /// bytes start).
    pub min_address: Option<u32>,
    /// Only regions below this address (default: end of the produced
    /// bytes).
    pub max_address: Option<u32>,
    /// Also measure each region's crunched cost with this cruncher (any
    /// `compare_crunchers` format name, e.g. `shrinkler`).
    pub cruncher: Option<String>,
    /// How many of the biggest regions to list (default 25).
    pub top: Option<usize>
}

struct Assembled {
    symbols: Vec<(String, u32)>,
    image: Vec<u8>,
    start: u32,
    map: Option<RawSourceMap>,
    sections: Vec<CrunchedSectionInfo>
}

fn assemble_for_map(input: &SizeMapInput) -> Result<Assembled, String> {
    let path = Utf8Path::new(&input.path);
    let source = fs_err::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;

    let mut parse = ParserOptions::default();
    parse.add_search_path_from_file(path.as_str()).map_err(|e| e.to_string())?;
    for dir in input.include_dirs.clone().unwrap_or_default() {
        parse.add_search_path(dir).map_err(|e| e.to_string())?;
    }
    let builder = parse.clone().context_builder().set_current_filename(path.as_str());
    let listing = parse_z80_with_context_builder(&source, builder).map_err(|e| e.to_string())?;

    let mut assemble = AssemblingOptions::default();
    cpclib_basmopt::apply_defines(&mut assemble, &input.defines.clone().unwrap_or_default())?;
    // Nothing must be written to disk (a source may `SAVE` its outputs).
    assemble.set_dry_run(true);
    // The structured listing: per emitted run, file/line/columns/len/is_data.
    assemble.record_source_map();
    let options = EnvOptions::new(parse, assemble, Arc::new(()));
    // The processed tokens own the included files' listings, which the spans
    // of the `PRINT`s and the like point into until the post actions have
    // formatted them: they must outlive `handle_post_actions` - dropping them
    // straight away (`Ok((_, env))`) is a use-after-free that shows up as a
    // nondeterministic panic.
    let (processed, env) = match visit_tokens_all_passes_with_options(&listing, options) {
        Ok(done) => done,
        Err((_, _, e)) => return Err(e.to_string())
    };
    // The listing pass - what fills the source map - runs as a post action
    // (under `dry_run` it writes nothing, `SAVE` included).
    let mut env = env;
    env.handle_post_actions(&listing).map_err(|e| e.to_string())?;

    let start = env.start_address().ok_or("the source produced no bytes")? as u32;
    let symbols = env
        .symbols()
        .expression_symbol()
        .into_iter()
        .filter_map(|(sym, v)| {
            match v.value() {
                Value::Address(a) => Some((sym.value().to_string(), a.address() as u32)),
                _ => None
            }
        })
        .filter(|(name, _)| !name.starts_with('$') && !name.starts_with(".__hidden__"))
        .collect();
    let assembled = Assembled {
        symbols,
        image: env.produced_bytes(),
        start,
        map: env.source_map(),
        sections: env.crunched_sections().to_vec()
    };
    drop(env);
    drop(processed);
    Ok(assembled)
}

/// Crunched size of `data` with the section's own cruncher.
fn section_crunched_len(section: &CrunchedSectionInfo, data: &[u8]) -> Option<i64> {
    if data.is_empty() {
        return Some(0);
    }
    section.kind.compress(data).ok().map(|c| c.compressed_len() as i64)
}

/// Runs `f` on every item, a few at a time.
fn parallel_map<T: Sync, R: Send>(items: &[T], workers: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<Option<R>>> = std::sync::Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..workers.max(1).min(items.len().max(1)) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    results.lock().unwrap()[i] = Some(r);
                }
            });
        }
    });
    results.into_inner().unwrap().into_iter().flatten().collect()
}

/// Each crunched section, measured from the inside.
///
/// The image only holds a section's *output*, so the regions of the code that
/// went in are invisible to the image-level table - and that code is where a
/// size-limited intro's bytes are. For every section this crunches the
/// decrunched bytes with the section's own cruncher, then again with each of
/// the biggest labelled regions removed: the difference is what the region
/// really costs in the output. The map's decrunched-code rows say which
/// source lines those bytes come from.
pub(crate) fn measure_sections(
    sections: &[CrunchedSectionInfo],
    symbols: &[(String, u32)],
    map: Option<&RawSourceMap>,
    top: usize
) -> Vec<Json2> {
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get()).min(4);
    let contexts: Vec<(String, Option<String>)> = map
        .map(|m| m.files.iter().map(|f| split_context(f)).collect())
        .unwrap_or_default();
    sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let start = section.address;
            let end = start + section.decrunched.len() as u32;
            let baseline = section_crunched_len(section, &section.decrunched);

            // The biggest regions only: each one costs a whole crunch.
            let mut regions = build_regions(symbols, start, end);
            regions.sort_by_key(|r| std::cmp::Reverse(r.bytes));
            regions.truncate(top);
            let costs = parallel_map(&regions, workers, |r| {
                let lo = (r.address - start) as usize;
                let hi = (lo + r.bytes as usize).min(section.decrunched.len());
                let rest: Vec<u8> = section.decrunched[..lo].iter().chain(&section.decrunched[hi..]).copied().collect();
                Some(baseline? - section_crunched_len(section, &rest)?)
            });

            // Decrunched-code rows: where they run once decrunched, but not
            // where they sit in the image (that is what tells them apart).
            let mut by_line: std::collections::BTreeMap<(String, u32), u64> = Default::default();
            if let Some(map) = map {
                for row in map.rows.iter().filter(|r| r.len > 0 && r.page == 0 && r.logical != r.physical) {
                    if row.logical >= start && row.logical < end {
                        if let Some((file, _)) = contexts.get(row.file as usize) {
                            *by_line.entry((file.clone(), row.line)).or_default() += row.len as u64;
                        }
                    }
                }
            }
            let mut lines: Vec<_> = by_line.into_iter().collect();
            lines.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

            let mut regions_table = String::from("| # | Label | Address | Decrunched bytes | Crunched cost |\n|--:|:--|--:|--:|--:|\n");
            let mut ranked: Vec<usize> = (0..regions.len()).collect();
            ranked.sort_by_key(|&i| std::cmp::Reverse(costs[i].unwrap_or(i64::MIN)));
            for (n, &i) in ranked.iter().enumerate() {
                regions_table.push_str(&format!(
                    "| {} | {} | {:#06x} | {} | {} |\n",
                    n + 1,
                    regions[i].name,
                    regions[i].address,
                    regions[i].bytes,
                    costs[i].map_or("-".to_string(), |c| c.to_string())
                ));
            }
            let mut lines_table = String::from("| # | Source line | Decrunched bytes |\n|--:|:--|--:|\n");
            for (n, ((file, line), bytes)) in lines.iter().take(top).enumerate() {
                lines_table.push_str(&format!("| {} | {file}:{line} | {bytes} |\n", n + 1));
            }
            json!({
                "index": index,
                "cruncher": format!("{:?}", section.kind),
                "address": start,
                "decrunched_bytes": section.decrunched.len(),
                "crunched_bytes": section.crunched_len,
                "remeasured_crunched_bytes": baseline,
                "regions_table": regions_table,
                "lines_table": lines_table,
                "regions": ranked.iter().map(|&i| json!({
                    "label": regions[i].name,
                    "address": regions[i].address,
                    "bytes": regions[i].bytes,
                    "crunched_cost": costs[i]
                })).collect::<Vec<_>>()
            })
        })
        .collect()
}

fn render_table(
    regions: &[&Region],
    indices: &[usize],
    total: u32,
    costs: Option<&[Option<i64>]>,
    tallies: Option<&[Tally]>
) -> String {
    let mut header = String::from("| # | Label | Address | Bytes | % of image |");
    let mut rule = String::from("|--:|:--|--:|--:|--:|");
    if tallies.is_some() {
        header.push_str(" Code | Data |");
        rule.push_str("--:|--:|");
    }
    if costs.is_some() {
        header.push_str(" Crunched cost |");
        rule.push_str("--:|");
    }
    header.push_str(" Locals |\n");
    rule.push_str("--:|\n");
    let mut out = format!("{header}{rule}");
    for (i, r) in regions.iter().enumerate() {
        let pct = if total == 0 { 0.0 } else { r.bytes as f64 * 100.0 / total as f64 };
        let name = if r.aliases.is_empty() {
            r.name.clone()
        }
        else {
            format!("{} (= {})", r.name, r.aliases.join(", "))
        };
        out.push_str(&format!("| {} | {name} | {:#06x} | {} | {pct:.1} |", i + 1, r.address, r.bytes));
        if let Some(t) = tallies {
            let t = &t[indices[i]];
            out.push_str(&format!(" {} | {} |", t.code, t.data));
        }
        if let Some(c) = costs {
            out.push_str(&format!(" {} |", c.get(i).copied().flatten().map_or("-".to_string(), |v| v.to_string())));
        }
        out.push_str(&format!(" {} |\n", r.locals));
    }
    out
}

pub(crate) fn size_map(input: SizeMapInput) -> ToolResult {
    if let Some(c) = &input.cruncher {
        resolve_cruncher(c)?;
    }
    let assembled =
        assemble_for_map(&input).map_err(|e| ToolError::new(ToolErrorKind::Assembler, e))?;
    let image_end = assembled.start + assembled.image.len() as u32;
    let start = input.min_address.unwrap_or(assembled.start).max(assembled.start);
    let end = input.max_address.unwrap_or(image_end).min(image_end);
    if start >= end {
        return Err(ToolError::invalid_input("the requested address range is empty"));
    }

    let mut regions = build_regions(&assembled.symbols, start, end);
    let costs = match &input.cruncher {
        Some(c) => {
            Some(
                crunched_costs(c, &assembled.image, assembled.start, &regions)
                    .map_err(|e| ToolError::new(ToolErrorKind::InvalidInput, e))?
            )
        },
        None => None
    };

    // Rank by crunched cost when measured, otherwise by raw size.
    let mut order: Vec<usize> = (0..regions.len()).collect();
    order.sort_by_key(|&i| {
        let cost = costs.as_ref().and_then(|(_, c)| c[i]).unwrap_or(i64::MIN);
        (std::cmp::Reverse(cost), std::cmp::Reverse(regions[i].bytes))
    });
    if costs.is_none() {
        order.sort_by_key(|&i| std::cmp::Reverse(regions[i].bytes));
    }
    let top = input.top.unwrap_or(25);
    let shown: Vec<&Region> = order.iter().take(top).map(|&i| &regions[i]).collect();
    let shown_costs: Option<Vec<Option<i64>>> = costs
        .as_ref()
        .map(|(_, c)| order.iter().take(top).map(|&i| c[i]).collect());

    let total = end - start;
    let attribution = assembled.map.as_ref().map(|m| attribute(m, &regions, &MacroIndex::scan(&m.files.iter().map(|f| split_context(f).0).collect::<Vec<_>>()), start..end));
    let shown_indices: Vec<usize> = order.iter().take(top).copied().collect();
    let table = render_table(
        &shown,
        &shown_indices,
        total,
        shown_costs.as_deref(),
        attribution.as_ref().map(|a| a.per_region.as_slice())
    );
    let rows: Vec<Json2> = order
        .iter()
        .map(|&i| {
            let r = &regions[i];
            json!({
                "label": r.name,
                "aliases": r.aliases,
                "address": r.address,
                "bytes": r.bytes,
                "locals": r.locals,
                "code_bytes": attribution.as_ref().map(|a| a.per_region[i].code),
                "data_bytes": attribution.as_ref().map(|a| a.per_region[i].data),
                "crunched_cost": costs.as_ref().and_then(|(_, c)| c[i])
            })
        })
        .collect();
    regions.clear();
    let emitted: u64 = attribution.as_ref().map_or(0, |a| a.by_file.values().map(|t| t.bytes).sum());
    let detail = attribution.as_ref().map(|a| {
        json!({
            "files_table": render_tally_table("Source file", a.by_file.iter().map(|(k, t)| (k.clone(), t)).collect(), 15, total as u64),
            "macros_table": render_tally_table("Macro / expansion", a.by_macro.iter().map(|(k, t)| (k.clone(), t)).collect(), 15, total as u64),
            "lines_table": render_tally_table(
                "Source line",
                a.by_line.iter().map(|((f, l), t)| (format!("{f}:{l}"), t)).collect(),
                20,
                total as u64
            ),
            "emitted_bytes": emitted,
            "unowned_image_bytes": a.unowned_image_bytes,
            "unplaced_row_bytes": a.unplaced_row_bytes,
            "note": "Tables count image bytes by the source line that produced them. A crunched \
                     section's bytes are charged to its LZ directive; the code inside it is \
                     not listed (those rows describe the decrunched code, not the image)."
        })
    });
    Ok(json!({
        "listing": detail,
        "crunched_sections": measure_sections(&assembled.sections, &assembled.symbols, assembled.map.as_ref(), top.min(16)),
        "path": input.path,
        "start": start,
        "end": end,
        "image_bytes": total,
        "cruncher": input.cruncher,
        "crunched_image": costs.as_ref().map(|(b, _)| *b),
        "table": table,
        "regions": rows
    }))
}

fn ok_or_tool_error(result: ToolResult) -> Result<Json<Json2>, Json<Json2>> {
    result.map(Json).map_err(|e| Json(e.to_json()))
}

#[tool_router(router = sizemap_router, vis = "pub(crate)")]
impl McpServer {
    #[tool(description = "Read-only: where do the bytes go? Assembles a source (like a build \
                           would - pass include_dirs/defines from project_context) and returns \
                           a markdown `table` of the biggest global labels: address, bytes to \
                           the next global label, share of the image, number of local labels. \
                           With `cruncher` (e.g. shrinkler) it also measures each region's \
                           *crunched* cost - the whole image crunched, minus the image without \
                           that region - which can rank routines very differently from raw size \
                           and is what a size-limited intro needs. Labels come from the \
                           assembler's own table (real addresses only, never `equ` constants), \
                           not a .sym file. From the assembler's source map (the structured \
                           listing) it adds a code/data split per region and three more tables: \
                           bytes by source file, by macro (what a macro's call sites cost \
                           together) and by individual source line, each with the number of \
                           places it was instantiated. Spans include any data placed between two \
                           labels; `SAVE` directives are not executed.")]
    async fn size_map(&self, Parameters(input): Parameters<SizeMapInput>) -> Result<Json<Json2>, Json<Json2>> {
        ok_or_tool_error(size_map(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syms(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(n, a)| (n.to_string(), *a)).collect()
    }

    #[test]
    fn regions_run_to_the_next_global_and_count_their_locals() {
        let regions = build_regions(
            &syms(&[("main", 0x4000), ("main.loop", 0x4004), ("draw", 0x4010), ("draw.x", 0x4012), ("draw.y", 0x4014), ("data", 0x4030)]),
            0x4000,
            0x4040
        );
        assert_eq!(regions.len(), 3);
        assert_eq!((regions[0].name.as_str(), regions[0].bytes, regions[0].locals), ("main", 0x10, 1));
        assert_eq!((regions[1].name.as_str(), regions[1].bytes, regions[1].locals), ("draw", 0x20, 2));
        assert_eq!((regions[2].name.as_str(), regions[2].bytes, regions[2].locals), ("data", 0x10, 0));
    }

    #[test]
    fn labels_at_the_same_address_are_aliases_and_out_of_range_ones_are_ignored() {
        let regions = build_regions(
            &syms(&[("a", 0x100), ("b", 0x100), ("c", 0x110), ("way_before", 0x10), ("beyond", 0x900)]),
            0x100,
            0x120
        );
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].name, "a");
        assert_eq!(regions[0].aliases, vec!["b"]);
        assert_eq!((regions[0].bytes, regions[1].bytes), (0x10, 0x10));
    }

    #[test]
    fn a_file_with_only_dotted_labels_still_gets_a_map() {
        let regions = build_regions(&syms(&[("m.a", 0x200), ("m.b", 0x208)]), 0x200, 0x210);
        assert_eq!(regions.iter().map(|r| r.bytes).collect::<Vec<_>>(), vec![8, 8]);
    }

    /// End to end on a real (tiny) assemble: constants must not appear, and
    /// crunched costs must be measured for the labels that hold real bytes.
    #[test]
    fn a_real_source_is_mapped_and_costed() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("m.asm");
        fs_err::write(
            &path,
            "SIZE equ 4\n org 0x4000\nstart:\n ld hl,0x1234\n ret\nfiller:\n defs 64,0xAA\ntable:\n db 1,2,3,4,5,6,7,8\n",
        )
        .unwrap();
        let out = size_map(SizeMapInput {
            path: path.to_string(),
            include_dirs: None,
            defines: None,
            min_address: None,
            max_address: None,
            cruncher: Some("zx0".to_string()),
            top: None
        })
        .expect("size_map should succeed");
        let regions = out["regions"].as_array().unwrap();
        let names: Vec<&str> = regions.iter().map(|r| r["label"].as_str().unwrap()).collect();
        assert!(!names.contains(&"SIZE"), "an equ constant is not a region: {names:?}");
        let filler = regions.iter().find(|r| r["label"] == "filler").unwrap();
        assert_eq!(filler["bytes"], 64);
        assert_eq!(filler["address"], 0x4004);
        assert!(out["table"].as_str().unwrap().contains("filler"));
        assert!(out["crunched_image"].as_i64().unwrap() > 0);
    }

    #[test]
    fn a_context_name_is_split_into_file_and_expansion() {
        assert_eq!(split_context("main.asm"), ("main.asm".to_string(), None));
        assert_eq!(
            split_context("/p/engine_macros.asm:121:1 > MACRO ENGINE_DRAW_SHADOW:"),
            ("/p/engine_macros.asm".to_string(), Some("MACRO ENGINE_DRAW_SHADOW".to_string()))
        );
        // A Windows drive letter must not be mistaken for line/column.
        assert_eq!(split_context("C:\\p\\a.asm").0, "C:\\p\\a.asm");
        assert_eq!(
            split_context("a.asm:3:1 > MACRO OUTER: > b.asm:9:2 > MACRO INNER:").1.as_deref(),
            Some("MACRO INNER")
        );
    }

    #[test]
    fn rows_are_attributed_to_files_macros_lines_and_regions() {
        let map = RawSourceMap {
            files: vec!["a.asm".to_string(), "a.asm:5:1 > MACRO M:".to_string()],
            rows: vec![
                SourceMapRow::flat(0, 1, 0x100, 2),
                SourceMapRow { is_data: true, ..SourceMapRow::flat(0, 2, 0x102, 4) },
                SourceMapRow::flat(1, 1, 0x106, 1),
                SourceMapRow::flat(1, 1, 0x107, 1),
                SourceMapRow::flat(0, 3, 0x108, 0)
            ]
        };
        let regions = build_regions(&syms(&[("first", 0x100), ("second", 0x106)]), 0x100, 0x110);
        let a = attribute(&map, &regions, &MacroIndex::default(), 0x100..0x110);
        assert_eq!((a.per_region[0].code, a.per_region[0].data), (2, 4));
        assert_eq!((a.per_region[1].code, a.per_region[1].data), (2, 0));
        assert_eq!(a.by_file["a.asm"].bytes, 8, "expansions count towards their real file");
        let m = &a.by_macro["MACRO M"];
        assert_eq!((m.bytes, m.runs), (2, 2), "a macro used twice is two instances");
        assert_eq!(a.by_line[&("a.asm".to_string(), 1)].bytes, 4, "same line via macro and directly");
        assert!(!a.by_line.contains_key(&("a.asm".to_string(), 3)), "an empty row emits nothing");
    }

    /// A crunched section's uncrunched inputs are rows too, but their bytes
    /// are overwritten by the (smaller) crunched output.
    #[test]
    fn decrunched_code_rows_are_counted_apart_from_the_image() {
        let map = RawSourceMap {
            files: vec!["a.asm".to_string()],
            rows: vec![
                SourceMapRow::flat(0, 1, 0x100, 2),
                // as seen inside a crunched section: runs at 0x4001 once
                // decrunched, but sits at a scratch-buffer offset
                SourceMapRow { physical: 0, ..SourceMapRow::flat(0, 2, 0x4001, 8) }
            ]
        };
        let regions = build_regions(&syms(&[("x", 0x100)]), 0x100, 0x10a);
        let a = attribute(&map, &regions, &MacroIndex::default(), 0x100..0x10a);
        assert_eq!(a.by_file["a.asm"].bytes, 2);
        assert_eq!((a.unplaced_row_bytes, a.unowned_image_bytes), (8, 8));
    }

    #[test]
    fn overwritten_rows_do_not_count() {
        let map = RawSourceMap {
            files: vec!["a.asm".to_string()],
            rows: vec![
                // 16 raw bytes that were fed to a cruncher...
                SourceMapRow { is_data: true, ..SourceMapRow::flat(0, 5, 0x100, 16) },
                // ...replaced by 4 crunched ones, charged to the directive.
                SourceMapRow { is_data: true, ..SourceMapRow::flat(0, 3, 0x100, 4) }
            ]
        };
        let regions = build_regions(&syms(&[("blob", 0x100)]), 0x100, 0x104);
        let a = attribute(&map, &regions, &MacroIndex::default(), 0x100..0x104);
        assert_eq!(a.by_file["a.asm"].bytes, 4);
        assert!(!a.by_line.contains_key(&("a.asm".to_string(), 5)));
        assert_eq!(a.by_line[&("a.asm".to_string(), 3)].bytes, 4);
        assert_eq!(a.per_region[0].data, 4);
    }

    /// Inside a crunched section the image only holds the output, so the
    /// cost of each part has to be measured on the bytes that went in: a big
    /// run of zeros crunches to almost nothing, noise does not.
    #[test]
    fn regions_inside_a_crunched_section_are_costed_on_what_went_in() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("lz.asm");
        fs_err::write(
            &path,
            "\torg 0x4000\n\tLZ48\nzeros:\n\tdefs 200\nnoise:\n\tdb %NOISE%\n\tLZCLOSE\n"
                .replace("%NOISE%", "121,66,189,242,33,6,240,132,119,98,240,243,203,77,118,77,199,7,32,81,21,154,15,137,242,198,218,202,227,68,187,49,18,69,253,111,132,223,154,215,197,179,208,118,172,14,143,83,167,53,108,136,145,63,32,246,247,45,176,34,210,77,10,150"),
        )
        .unwrap();
        let out = size_map(SizeMapInput {
            path: path.to_string(),
            include_dirs: None,
            defines: None,
            min_address: None,
            max_address: None,
            cruncher: None,
            top: None
        })
        .expect("size_map should succeed");
        let sections = out["crunched_sections"].as_array().unwrap();
        assert_eq!(sections.len(), 1, "{out}");
        let s = &sections[0];
        assert_eq!(s["address"], 0x4000);
        assert_eq!(s["decrunched_bytes"], 264);
        let cost = |label: &str| {
            s["regions"].as_array().unwrap().iter().find(|r| r["label"] == label).unwrap()["crunched_cost"].as_i64().unwrap()
        };
        assert!(cost("noise") > cost("zeros") + 30, "noise {} vs zeros {}: {s}", cost("noise"), cost("zeros"));
        assert!(s["regions_table"].as_str().unwrap().contains("noise"));
        assert!(s["lines_table"].as_str().unwrap().contains("lz.asm:4"), "{}", s["lines_table"]);
    }

    /// Real assemble: a macro called twice, plus data. The listing must say
    /// who emitted what.
    #[test]
    fn a_real_macro_and_data_are_attributed_from_the_source_map() {
        let dir = camino_tempfile::tempdir().unwrap();
        let path = dir.path().join("mm.asm");
        fs_err::write(
            &path,
            "macro CLEAR\n ld a,0\n ld (hl),a\nendm\n org 0x4000\nfirst:\n CLEAR(void)\n CLEAR(void)\n ret\ntable:\n db 1,2,3,4,5\n",
        )
        .unwrap();
        let out = size_map(SizeMapInput {
            path: path.to_string(),
            include_dirs: None,
            defines: None,
            min_address: None,
            max_address: None,
            cruncher: None,
            top: None
        })
        .expect("size_map should succeed");
        let regions = out["regions"].as_array().unwrap();
        let table = regions.iter().find(|r| r["label"] == "table").unwrap();
        assert_eq!((table["code_bytes"].as_u64(), table["data_bytes"].as_u64()), (Some(0), Some(5)));
        let first = regions.iter().find(|r| r["label"] == "first").unwrap();
        assert_eq!(first["code_bytes"], 7, "two CLEARs (3 bytes each) + ret");
        let macros = out["listing"]["macros_table"].as_str().unwrap();
        assert!(macros.contains("MACRO CLEAR") && macros.contains("| 6 |"), "{macros}");
    }
}
