//! `size_map` - where do the bytes go? Assembles a source and reports, per
//! global label, how many bytes it spans, and (optionally) what each region
//! costs *after crunching*.
//!
//! Symbols come from the assembler's own table (only real addresses, so an
//! `equ` constant never pollutes the map) rather than from a `.sym` file,
//! which a project may export only in part (etchy exports 18 symbols). The
//! crunched cost of a region is measured, not estimated: the whole image is
//! crunched, then again with that region removed, and the difference is the
//! region's contribution. It is what a size-limited intro actually needs -
//! raw size and crunched size can rank routines very differently.

use std::sync::Arc;

use camino::Utf8Path;
use cpclib_asm::assembler::visit_tokens_all_passes_with_options;
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
    start: u32
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
    let options = EnvOptions::new(parse, assemble, Arc::new(()));
    let env = match visit_tokens_all_passes_with_options(&listing, options) {
        Ok((_, env)) => env,
        Err((_, _, e)) => return Err(e.to_string())
    };

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
    Ok(Assembled {
        symbols,
        image: env.produced_bytes(),
        start
    })
}

fn render_table(regions: &[&Region], total: u32, costs: Option<&[Option<i64>]>) -> String {
    let mut out = String::from(if costs.is_some() {
        "| # | Label | Address | Bytes | % of image | Crunched cost | Locals |\n|--:|:--|--:|--:|--:|--:|--:|\n"
    }
    else {
        "| # | Label | Address | Bytes | % of image | Locals |\n|--:|:--|--:|--:|--:|--:|\n"
    });
    for (i, r) in regions.iter().enumerate() {
        let pct = if total == 0 { 0.0 } else { r.bytes as f64 * 100.0 / total as f64 };
        let name = if r.aliases.is_empty() {
            r.name.clone()
        }
        else {
            format!("{} (= {})", r.name, r.aliases.join(", "))
        };
        match costs {
            Some(c) => {
                let cost = c.get(i).copied().flatten().map_or("-".to_string(), |v| v.to_string());
                out.push_str(&format!(
                    "| {} | {name} | {:#06x} | {} | {pct:.1} | {cost} | {} |\n",
                    i + 1, r.address, r.bytes, r.locals
                ));
            },
            None => {
                out.push_str(&format!(
                    "| {} | {name} | {:#06x} | {} | {pct:.1} | {} |\n",
                    i + 1, r.address, r.bytes, r.locals
                ));
            }
        }
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
    let table = render_table(&shown, total, shown_costs.as_deref());
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
                "crunched_cost": costs.as_ref().and_then(|(_, c)| c[i])
            })
        })
        .collect();
    regions.clear();
    Ok(json!({
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
                           not a .sym file. Spans include any data placed between two labels; \
                           `SAVE` directives are not executed.")]
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
}
