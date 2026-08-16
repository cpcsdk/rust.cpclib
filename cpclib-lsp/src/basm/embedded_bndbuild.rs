//! Detection of `#!bndbuild`-marked bndbuild rules embedded in a `.asm`
//! file's own `;`/`//` comments, e.g.:
//! ```text
//! ; #!bndbuild
//! ; - tgt: test
//! ;   cmd:
//! ;    - basm --snapshot foo.asm -o foo.sna
//! ```
//! (`/* */` block comments are not supported as a container: basm's real
//! grammar treats them as pure discarded whitespace - `parse_multiline_comment
//! .value(())` in `cpclib-asm/src/parser/common.rs` - never emitting a
//! `Token::Comment` for them, so there is no AST node to attach a block to;
//! only a raw-text re-scan could find one, which was deliberately not built
//! given the added false-positive surface for no token-based safety net.)
//! Detection walks the already-tokenized `Token::Comment` nodes from the
//! document's own parsed listing (via the existing `flatten_listing`) rather
//! than re-lexing raw text - this reuses basm's own comment recognition and
//! gets real source positions for free. The extracted YAML is then handed to
//! the `bndbuild` module (`code_lens`/`hover`/`goto_definition`/
//! `prepare_rename`/`rename`/`semantic_tokens`/`prepare_call_hierarchy`
//! delegate to the matching `BuildFileAnalyzer` method against a synthetic
//! `Document` wrapping just the block's own text; execution delegates to
//! `BuildFileAnalyzer::run_embedded_rule` from `server/backend.rs`) - a
//! one-directional `basm -> bndbuild` dependency, mirroring the existing
//! `basm -> locomotive` one (see `embedded_basic.rs`).

use cpclib_asm::parser::obtained::{LocatedListing, MayHaveSpan};
// The blocks themselves are found by `cpclib_project::embedded_build`, which
// the debug adapter uses too: two scanners could disagree about where a block
// starts, which would be two answers to "which rules does this file have".
// What stays here is the *editor* half - mapping positions in and out of a
// block, and the lenses.
pub(crate) use cpclib_project::embedded_build::{EmbeddedBndbuildBlock, extract_embedded_blocks};
use cpclib_tokens::ListingElement;
use tower_lsp::lsp_types::{CodeLens, Command, Location, Position, Range, Url};

use super::AssemblyAnalyzer;
use crate::common::document::Document;

/// Picks the block that actually declares `rule` as a target; falls back to
/// the first block if none matches (a small, accepted staleness window
/// between code-lens render and click - the same implicit assumption every
/// other code-lens command in this codebase already makes about the file not
/// having changed shape in between).
pub(crate) fn find_block_for_rule<'a>(
    blocks: &'a [EmbeddedBndbuildBlock],
    rule: &str,
    file_dir: Option<&std::path::Path>
) -> Option<&'a EmbeddedBndbuildBlock> {
    let analyzer = crate::bndbuild::BuildFileAnalyzer::new();
    blocks
        .iter()
        .find(|b| {
            analyzer
                .target_names_for_embedded_block(&b.yaml_text, file_dir)
                .iter()
                .any(|n| n == rule)
        })
        .or_else(|| blocks.first())
}

/// The block containing `line_idx` (outer-document, 0-based), if any - the
/// bndbuild counterpart of `embedded_basic::block_and_text_at`'s "find"
/// half (no "reconstruct" half is needed here: `yaml_text` is already
/// reconstructed at extraction time).
pub(crate) fn block_at(
    blocks: &[EmbeddedBndbuildBlock],
    line_idx: usize
) -> Option<&EmbeddedBndbuildBlock> {
    blocks.iter().find(|b| {
        let end = b.yaml_start_line + b.yaml_text.lines().count();
        (b.yaml_start_line..end).contains(&line_idx)
    })
}

/// Outer-document `position` -> block-local position (0-based within
/// `block.yaml_text`). `None` if outside the block's own line range, or if
/// the column falls left of where this line's real content starts (over
/// the stripped comment prefix).
pub(crate) fn position_into_block(
    block: &EmbeddedBndbuildBlock,
    position: Position
) -> Option<Position> {
    let rel_line = (position.line as usize).checked_sub(block.yaml_start_line)?;
    let content_col = *block.content_start_cols.get(rel_line)?;
    Some(Position {
        line: rel_line as u32,
        character: position.character.checked_sub(content_col)?
    })
}

/// Block-local position -> outer-document position.
pub(crate) fn position_out_of_block(block: &EmbeddedBndbuildBlock, position: Position) -> Position {
    let content_col = block
        .content_start_cols
        .get(position.line as usize)
        .copied()
        .unwrap_or(0);
    Position {
        line: block.yaml_start_line as u32 + position.line,
        character: content_col + position.character
    }
}

pub(crate) fn range_out_of_block(block: &EmbeddedBndbuildBlock, range: Range) -> Range {
    Range {
        start: position_out_of_block(block, range.start),
        end: position_out_of_block(block, range.end)
    }
}

/// As `range_out_of_block`, but for a `Location`: only shifts when
/// `loc.uri` is still the synthetic block-document's own uri (== the host
/// `.asm` file, since the synthetic `Document` is always built with
/// `document.uri.clone()`) - i.e. still inside the block. A `Location`
/// pointing at a genuinely different file (an `{% include %}`d file, or a
/// `cmd:`-referenced on-disk path - both real, already-tested cases in
/// `bndbuild::definition`'s own test suite) must pass through untouched,
/// never have its real coordinates corrupted by the shift.
pub(crate) fn location_out_of_block(
    block: &EmbeddedBndbuildBlock,
    host_uri: &Url,
    loc: Location
) -> Location {
    if &loc.uri == host_uri {
        Location {
            uri: loc.uri,
            range: range_out_of_block(block, loc.range)
        }
    }
    else {
        loc
    }
}

/// Emit semantic tokens for a `#!bndbuild` block's own YAML content by
/// running the real bndbuild scanner against a synthetic block-only
/// `Document`, decoding its delta-encoded output back to absolute
/// `RawSemanticToken`s, then shifting each one's line by
/// `block.yaml_start_line` and column by that line's own
/// `content_start_cols` entry. Appended into `raw`, mirroring
/// `embedded_basic::push_locomotive_basic_tokens`'s role - merged into the
/// same accumulator the ASM tokenizer fills, sorted and delta-re-encoded
/// together in one final pass by the caller.
pub(super) fn push_embedded_bndbuild_tokens(
    document: &Document,
    block: &EmbeddedBndbuildBlock,
    raw: &mut Vec<super::token::RawSemanticToken>
) {
    let block_doc = Document::new(document.uri.clone(), block.yaml_text.clone(), 0);
    let tokens = crate::bndbuild::BuildFileAnalyzer::new().semantic_tokens(&block_doc);
    let (mut line, mut col) = (0u32, 0u32);
    for t in tokens {
        if t.delta_line == 0 {
            col += t.delta_start;
        }
        else {
            line += t.delta_line;
            col = t.delta_start;
        }
        let content_col = block
            .content_start_cols
            .get(line as usize)
            .copied()
            .unwrap_or(0);
        raw.push(super::token::RawSemanticToken {
            line: block.yaml_start_line as u32 + line,
            col: content_col + col,
            len: t.length,
            token_type: t.token_type,
            modifiers: t.token_modifiers_bitset
        });
    }
}

impl AssemblyAnalyzer {
    /// Every `#!bndbuild` block found in `document`. Empty when there's no
    /// block, or the document doesn't currently parse at all - a parse
    /// failure is treated as "nothing found" here, matching this crate's
    /// other best-effort callers (e.g. `call_hierarchy.rs`'s `.ok()?`
    /// convention), not `remove_parameter.rs`'s stricter all-or-nothing one
    /// (that module needs a fully-valid parse for its own safety property;
    /// this feature doesn't).
    pub fn embedded_bndbuild_blocks(&self, document: &Document) -> Vec<EmbeddedBndbuildBlock> {
        match self.parse_document(document) {
            Ok(listing) => extract_embedded_blocks(&listing),
            Err(_) => Vec::new()
        }
    }

    /// Every CodeLens this analyzer offers for `.asm` files: "▶ Run" for
    /// each rule inside every `#!bndbuild` block, plus the peephole-
    /// optimizer's own "⚡ Fix All" summary lens
    /// (`peephole_code_lenses`, `peephole.rs`). Empty `Vec` (not `None`)
    /// when there's none - matches `BuildFileAnalyzer::code_lens`'s own
    /// shape; `Backend::code_lens` turns an empty result into `None`.
    pub fn code_lens(&self, document: &Document) -> Vec<CodeLens> {
        let file_path = document
            .uri
            .to_file_path()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_dir = document
            .uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        // "Run this program" at the top of the file. Built exactly as F5
        // builds it - same assemble, same `-D` definitions, same snapshot - and
        // then handed to the emulator the project names rather than to the one
        // that speaks the Debug Adapter Protocol: running and debugging differ
        // in what you do afterwards, not in what you build.
        let mut lenses: Vec<CodeLens> = Vec::new();
        if self.config().code_lens && !document.text().trim().is_empty() {
            let top = Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0)
            };
            lenses.push(CodeLens {
                range: top,
                command: Some(Command {
                    title: "▶ Run in emulator".to_string(),
                    command: "cpclib.runAssembly".to_string(),
                    arguments: Some(vec![serde_json::json!(file_path)])
                }),
                data: None
            });
            // The same build, stopped where you asked instead of run to
            // completion. Beside the Run button because the choice between them
            // is made at the moment you press one, not when you set the project
            // up - and going to the debug panel to start a session on the file
            // already open in front of you is a detour.
            //
            // A client-side command, like `cpclib.debugRule`: only the editor
            // can start a debug session, so this is deliberately not in
            // `executeCommandProvider.commands`.
            lenses.push(CodeLens {
                range: top,
                command: Some(Command {
                    title: "🐞 Debug".to_string(),
                    command: "cpclib.debugAssembly".to_string(),
                    arguments: Some(vec![serde_json::json!(file_path)])
                }),
                data: None
            });
        }

        let build_analyzer = crate::bndbuild::BuildFileAnalyzer::new();
        lenses.extend::<Vec<CodeLens>>(
            self.embedded_bndbuild_blocks(document)
                .into_iter()
                .flat_map(|block| {
                    build_analyzer.code_lens_for_embedded_block(
                        &block.yaml_text,
                        block.yaml_start_line as u32,
                        &file_path,
                        file_dir.as_deref()
                    )
                })
                .collect()
        );
        lenses.extend(self.peephole_code_lenses(document));
        lenses
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::*;

    fn doc(code: &str) -> Document {
        Document::new(Url::parse("file:///t.asm").unwrap(), code.to_string(), 1)
    }

    fn blocks_for(code: &str) -> Vec<EmbeddedBndbuildBlock> {
        let d = doc(code);
        let analyzer = AssemblyAnalyzer::new();
        analyzer.embedded_bndbuild_blocks(&d)
    }

    const SHADEBOBS_EXAMPLE: &str = "; #!bndbuild\n\
; - tgt: test\n\
;   phony: true\n\
;   cmd:\n\
;    - basm --snapshot shadebobs.asm -o shadebobs.sna --lst shadebobs.lst\n\
;    - -ace shadebobs.sna\n\
ORG 0x8000\n";

    #[test]
    fn extracts_the_real_shadebobs_example() {
        let blocks = blocks_for(SHADEBOBS_EXAMPLE);
        assert_eq!(blocks.len(), 1);
        let block = &blocks[0];
        assert_eq!(block.marker_line, 0);
        assert_eq!(block.yaml_start_line, 1);
        // Only the fixed comment prefix + one following space is stripped -
        // indentation beyond that (here: the extra 2 spaces under "- tgt:",
        // 3 under "cmd:") is preserved verbatim, since it's meaningful YAML
        // nesting, not part of the comment syntax.
        assert_eq!(
            block.yaml_text,
            "- tgt: test\n\
             \u{20}\u{20}phony: true\n\
             \u{20}\u{20}cmd:\n\
             \u{20}\u{20}\u{20}- basm --snapshot shadebobs.asm -o shadebobs.sna --lst shadebobs.lst\n\
             \u{20}\u{20}\u{20}- -ace shadebobs.sna"
        );
        // "; - tgt: test" -> content starts right after "; " at column 2.
        assert_eq!(block.content_start_cols[0], 2);
        // ";   phony: true" -> "; " (2) then this line's own extra indent
        // is part of the *content*, not the stripped prefix - column stays 2.
        assert_eq!(block.content_start_cols[1], 2);
    }

    #[test]
    fn no_marker_present_yields_no_blocks() {
        let blocks = blocks_for("; a plain comment\nORG 0x8000\n");
        assert!(blocks.is_empty());
    }

    #[test]
    fn marker_with_no_following_comment_lines_yields_no_block() {
        let blocks = blocks_for("; #!bndbuild\n");
        assert!(blocks.is_empty());
    }

    #[test]
    fn marker_immediately_followed_by_non_comment_content_yields_no_block() {
        let blocks = blocks_for("; #!bndbuild\nORG 0x8000\n");
        assert!(blocks.is_empty());
    }

    #[test]
    fn a_bare_spacer_comment_line_does_not_end_the_block() {
        let code = "; #!bndbuild\n; - tgt: a\n;\n; - tgt: b\n";
        let blocks = blocks_for(code);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].yaml_text, "- tgt: a\n\n- tgt: b");
    }

    #[test]
    fn a_genuinely_blank_line_ends_the_block() {
        let code = "; #!bndbuild\n; - tgt: a\n\n; - tgt: b\n";
        let blocks = blocks_for(code);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
    }

    #[test]
    fn block_can_end_at_eof_without_panicking() {
        let blocks = blocks_for("; #!bndbuild\n; - tgt: a");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
    }

    #[test]
    fn supports_both_semicolon_and_double_slash_prefixes() {
        let blocks = blocks_for("// #!bndbuild\n// - tgt: a\n");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
    }

    #[test]
    fn multi_target_block_is_dedented_line_for_line() {
        let blocks = blocks_for(SHADEBOBS_EXAMPLE);
        assert_eq!(blocks[0].yaml_text.lines().count(), 5);
        assert_eq!(blocks[0].content_start_cols.len(), 5);
    }

    #[test]
    fn multiple_independent_blocks_in_one_file_are_all_found() {
        let code = "; #!bndbuild\n; - tgt: a\nORG 0x8000\n; #!bndbuild\n; - tgt: b\n";
        let blocks = blocks_for(code);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
        assert_eq!(blocks[1].yaml_text, "- tgt: b");
        assert_eq!(blocks[1].marker_line, 3);
    }

    #[test]
    fn a_block_nested_inside_an_if_body_is_still_found() {
        let code = "IF 1\n; #!bndbuild\n; - tgt: a\nENDIF\n";
        let blocks = blocks_for(code);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].yaml_text, "- tgt: a");
    }

    #[test]
    fn a_document_that_fails_to_parse_yields_no_blocks() {
        let blocks = blocks_for("@#$ garbage @#$\n");
        assert!(blocks.is_empty());
    }

    #[test]
    fn find_block_for_rule_matches_by_target_name_and_falls_back_to_first() {
        let blocks = extract_embedded_blocks(
            &AssemblyAnalyzer::parse_source(
                "; #!bndbuild\n; - tgt: a\n;   cmd: echo a\nORG 1\n; #!bndbuild\n; - tgt: b\n;   cmd: echo b\n",
                None,
                Default::default()
            )
            .expect("should parse")
        );
        assert_eq!(blocks.len(), 2);

        let matched = find_block_for_rule(&blocks, "b", None).expect("should find block for b");
        assert!(matched.yaml_text.contains("tgt: b"));

        let fallback =
            find_block_for_rule(&blocks, "does-not-exist", None).expect("should fall back");
        assert!(fallback.yaml_text.contains("tgt: a"));
    }

    // ── position/location translation ──────────────────────────────────────

    #[test]
    fn block_at_finds_the_containing_block_and_returns_none_outside_any_block() {
        let blocks = blocks_for(SHADEBOBS_EXAMPLE);
        assert!(block_at(&blocks, 0).is_none()); // the marker line itself
        assert!(block_at(&blocks, 1).is_some()); // "- tgt: test"
        assert!(block_at(&blocks, 5).is_some()); // "- -ace shadebobs.sna"
        assert!(block_at(&blocks, 6).is_none()); // "ORG 0x8000", past the block
    }

    #[test]
    fn position_into_block_and_back_round_trips() {
        let blocks = blocks_for(SHADEBOBS_EXAMPLE);
        let block = &blocks[0];
        // Outer-doc line 1, column 4 -> "- tgt: test"[2..], i.e. block-local
        // line 0, column 2 (content starts at column 2, after "; ").
        let outer = Position {
            line: 1,
            character: 4
        };
        let local = position_into_block(block, outer).expect("inside the block");
        assert_eq!(
            local,
            Position {
                line: 0,
                character: 2
            }
        );
        assert_eq!(position_out_of_block(block, local), outer);
    }

    #[test]
    fn position_into_block_is_none_over_the_stripped_prefix_or_outside_the_block() {
        let blocks = blocks_for(SHADEBOBS_EXAMPLE);
        let block = &blocks[0];
        // Column 1 is inside "; " itself (content starts at column 2).
        assert!(
            position_into_block(
                block,
                Position {
                    line: 1,
                    character: 1
                }
            )
            .is_none()
        );
        // Line 0 is the marker line, not part of the block's content.
        assert!(
            position_into_block(
                block,
                Position {
                    line: 0,
                    character: 2
                }
            )
            .is_none()
        );
    }

    #[test]
    fn location_out_of_block_shifts_a_same_uri_location_but_passes_a_different_one_through() {
        let d = doc(SHADEBOBS_EXAMPLE);
        let blocks = AssemblyAnalyzer::new().embedded_bndbuild_blocks(&d);
        let block = &blocks[0];

        let same_uri_loc = Location {
            uri: d.uri.clone(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 2
                },
                end: Position {
                    line: 0,
                    character: 5
                }
            }
        };
        let shifted = location_out_of_block(block, &d.uri, same_uri_loc);
        assert_eq!(shifted.range.start.line, block.yaml_start_line as u32);

        let other_uri = Url::parse("file:///elsewhere.asm").unwrap();
        let other_loc = Location {
            uri: other_uri.clone(),
            range: Range::default()
        };
        let untouched = location_out_of_block(block, &d.uri, other_loc.clone());
        assert_eq!(untouched.uri, other_uri);
        assert_eq!(untouched.range, other_loc.range);
    }
}

#[cfg(test)]
mod embedded_include_tests {
    use super::*;

    /// End-to-end regression test for a real report: an included rule's
    /// code lens *appeared* but at a nonsensical line (the raw
    /// Jinja-expanded-text line index, since spliced-in content has no
    /// mapping of its own back to the block's real text - see
    /// `SourceMap::to_original_or_nearest_following`'s own doc comment).
    /// Exercises the *full* pipeline (`AssemblyAnalyzer::code_lens` on a
    /// real on-disk `.asm` file, not `code_lens_for_embedded_block` called
    /// directly), since the bug only manifests once the block's own
    /// `yaml_start_line` offset compounds with the wrong source-mapped line.
    #[test]
    fn an_included_rule_s_code_lens_lands_on_the_include_line_not_a_meaningless_index() {
        let tmp = camino_tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("build.bnd"),
            "- tgt: imported\n  cmd: echo hi\n"
        )
        .unwrap();
        let asm_path = tmp.path().join("host.asm");
        let text =
            ";#!bndbuild\n; {% include \"build.bnd\" %}\n; - tgt: local\n;   cmd: echo local\n";
        std::fs::write(&asm_path, text).unwrap();
        let uri = Url::from_file_path(&asm_path).unwrap();
        let d = Document::new(uri, text.to_string(), 1);
        let analyzer = AssemblyAnalyzer::new();
        let lenses = analyzer.code_lens(&d);

        let imported = lenses
            .iter()
            .find(|l| l.command.as_ref().unwrap().title == "▶ Run: imported")
            .expect("expected a code lens for the included rule");
        // Outer-doc line 1 is "; {% include \"build.bnd\" %}" - the included
        // rule's lens must land there, not at some unrelated expanded-index
        // line (block-local expanded line 0/1 shifted by yaml_start_line=1
        // would wrongly be line 1/2 by coincidence here, so the meaningful
        // assertion is against the *local* rule's own lens, proving the fix
        // isn't accidentally right for the wrong reason).
        let local = lenses
            .iter()
            .find(|l| l.command.as_ref().unwrap().title == "▶ Run: local")
            .expect("expected a code lens for the local rule");
        assert_eq!(imported.range.start.line, 1, "{imported:?}");
        assert_eq!(local.range.start.line, 2, "{local:?}");
    }
}

#[cfg(test)]
mod run_lens_tests {
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::common::document::Document;

    fn doc(text: &str) -> Document {
        Document::new(
            Url::parse("file:///demo/main.asm").unwrap(),
            text.to_string(),
            1
        )
    }

    /// Every assembly file offers to run itself, the way a `.bas` file does.
    ///
    /// Built exactly as F5 builds it - the difference is the emulator it is
    /// handed to afterwards, not the build.
    #[test]
    fn an_assembly_file_offers_to_run_itself() {
        let lenses = AssemblyAnalyzer::new().code_lens(&doc("\torg 0x8000\n\tnop\n"));
        let run: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.runAssembly")
            .collect();
        assert_eq!(run.len(), 1, "{lenses:?}");
        assert_eq!(run[0].command.as_ref().unwrap().title, "▶ Run in emulator");
        assert_eq!(run[0].range.start.line, 0, "at the top of the file");
        assert_eq!(
            run[0].command.as_ref().unwrap().arguments.as_ref().unwrap()[0],
            serde_json::json!("/demo/main.asm")
        );
    }

    /// ...and to debug itself, beside it: the choice between running and
    /// debugging is made when you press one, not when you set the project up.
    #[test]
    fn an_assembly_file_offers_to_debug_itself_too() {
        let lenses = AssemblyAnalyzer::new().code_lens(&doc("\torg 0x8000\n\tnop\n"));
        let debug: Vec<_> = lenses
            .iter()
            .filter(|l| l.command.as_ref().unwrap().command == "cpclib.debugAssembly")
            .collect();
        assert_eq!(debug.len(), 1, "{lenses:?}");
        assert_eq!(debug[0].command.as_ref().unwrap().title, "🐞 Debug");
        assert_eq!(debug[0].range.start.line, 0);
        // It names the file it sits in, so clicking it debugs *that* file
        // whatever the editor is focused on.
        assert_eq!(
            debug[0]
                .command
                .as_ref()
                .unwrap()
                .arguments
                .as_ref()
                .unwrap()[0],
            serde_json::json!("/demo/main.asm")
        );
    }

    /// Run comes first: it is the commoner of the two.
    #[test]
    fn run_is_offered_before_debug() {
        let lenses = AssemblyAnalyzer::new().code_lens(&doc("\torg 0x8000\n\tnop\n"));
        let commands: Vec<&str> = lenses
            .iter()
            .map(|l| l.command.as_ref().unwrap().command.as_str())
            .collect();
        let run = commands.iter().position(|c| *c == "cpclib.runAssembly");
        let debug = commands.iter().position(|c| *c == "cpclib.debugAssembly");
        assert!(run < debug, "{commands:?}");
    }

    /// An empty file has nothing to run.
    #[test]
    fn an_empty_file_offers_nothing() {
        let lenses = AssemblyAnalyzer::new().code_lens(&doc("\n  \n"));
        assert!(
            lenses.iter().all(|l| {
                let command = &l.command.as_ref().unwrap().command;
                command != "cpclib.runAssembly" && command != "cpclib.debugAssembly"
            }),
            "{lenses:?}"
        );
    }

    /// The Run lens sits alongside the embedded-rule lenses rather than
    /// replacing them.
    #[test]
    fn the_run_lens_does_not_displace_the_embedded_rule_lenses() {
        let lenses = AssemblyAnalyzer::new().code_lens(&doc(
            "; #!bndbuild\n; - tgt: run\n;   cmd: -emu --snapshot demo.sna run\n\torg 0x8000\n"
        ));
        let titles: Vec<&str> = lenses
            .iter()
            .filter_map(|l| l.command.as_ref().map(|c| c.title.as_str()))
            .collect();
        assert!(titles.contains(&"▶ Run in emulator"), "{titles:?}");
        assert!(
            titles.iter().any(|t| t.starts_with("🐞 Debug")),
            "{titles:?}"
        );
    }
}
