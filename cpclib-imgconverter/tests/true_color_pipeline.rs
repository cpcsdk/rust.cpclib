//! The true-color conversion pipeline (resize, automatic palette selection
//! and dithering) is opt-in: any of `--dither`/`--resize-filter`/`--colors`
//! (top-level) or `--out-width`/`--out-height` (on `sprite`/`tile`) switches
//! a run from the default exact-transfer pipeline to this one. These tests
//! drive the real CLI end to end against a genuine true-color fixture
//! (`gradient_truecolor.png`: 64x64, 4096 distinct pixel colors, far more
//! than any CPC mode's pen budget), following the in-process pattern
//! `plus_snapshot.rs` already uses.

use cpclib_common::event::DiscardObserver;
use cpclib_image::ink::Ink;
use cpclib_imgconverter::{build_img2cpc_args_parser, process_img2cpc};

const FIXTURE: &str = "tests/gradient_truecolor.png";

fn run(argv: &[&str]) -> anyhow::Result<()> {
    let args = build_img2cpc_args_parser();
    let matches = args.clone().get_matches_from(argv);
    process_img2cpc(&matches, args, &DiscardObserver)
}

/// Runs a `sprite` conversion of the fixture and returns the inks the
/// resulting palette actually uses, read back through `--inks`.
fn sprite_inks(extra: &[&str], tag: &str) -> Vec<Ink> {
    let sprite_out = std::env::temp_dir().join(format!("cpclib_tc_{tag}.spr"));
    let inks_out = std::env::temp_dir().join(format!("cpclib_tc_{tag}.inks"));
    let _ = std::fs::remove_file(&sprite_out);
    let _ = std::fs::remove_file(&inks_out);

    let mut argv = vec!["img2cpc"];
    argv.extend_from_slice(extra);
    argv.push(FIXTURE);
    argv.extend_from_slice(&[
        "sprite",
        "-o",
        sprite_out.to_str().unwrap(),
        "-i",
        inks_out.to_str().unwrap()
    ]);

    run(&argv).expect("the conversion must succeed");

    let bytes = std::fs::read(&inks_out).expect("the inks file must have been written");
    let _ = std::fs::remove_file(&sprite_out);
    let _ = std::fs::remove_file(&inks_out);
    bytes.into_iter().map(Ink::from).collect()
}

#[test]
fn a_single_true_color_flag_alone_engages_the_pipeline_and_fits_the_mode_budget() {
    // Mode 0's budget is 16 colors; the fixture has 4096 distinct ones, so
    // without the true-color pipeline this would fail today exactly as it
    // does for any other over-budget image.
    let inks = sprite_inks(&["--dither", "floyd-steinberg"], "single_flag");
    assert!(inks.len() <= 16, "got {} inks", inks.len());
}

#[test]
fn every_named_dither_algorithm_runs_successfully() {
    for algo in [
        "ordered",
        "floyd-steinberg",
        "false-floyd-steinberg",
        "jarvis-judice-ninke",
        "stucki",
        "atkinson",
        "burkes",
        "sierra-3",
        "sierra-2",
        "sierra-lite"
    ] {
        let inks = sprite_inks(&["--dither", algo], &format!("dither_{algo}"));
        assert!(!inks.is_empty(), "{algo} produced no inks");
        assert!(inks.len() <= 16, "{algo} produced {} inks", inks.len());
    }
}

#[test]
fn every_named_resize_filter_runs_successfully() {
    for filter in ["nearest", "triangle", "catmullrom", "gaussian", "lanczos3"] {
        let inks = sprite_inks(
            &["--dither", "ordered", "--resize-filter", filter],
            &format!("filter_{filter}")
        );
        assert!(!inks.is_empty(), "{filter} produced no inks");
    }
}

#[test]
fn colors_flag_caps_the_automatically_selected_palette_size() {
    for n in [2u8, 5, 9] {
        let inks = sprite_inks(
            &["--dither", "ordered", "--colors", &n.to_string()],
            &format!("colors_{n}")
        );
        assert!(inks.len() <= n as usize, "asked for <= {n}, got {}", inks.len());
    }
}

#[test]
fn a_fully_specified_fixed_palette_runs_no_automatic_selection() {
    // Mode 1's budget is exactly 4 colors, matching the 4 given here, so
    // there is no room left to auto-fill - the palette must come through
    // unchanged.
    let inks = sprite_inks(
        &["--mode", "1", "--pens", "0,1,2,3", "--dither", "atkinson"],
        "fixed_palette"
    );
    let mut inks = inks;
    inks.sort();
    assert_eq!(inks, vec![Ink::from(0u8), Ink::from(1u8), Ink::from(2u8), Ink::from(3u8)]);
}

#[test]
fn a_partial_palette_keeps_the_pinned_pen_and_auto_fills_the_rest() {
    let inks = sprite_inks(
        &["--mode", "1", "--pen0", "0", "--unlock-pens", "--dither", "ordered"],
        "partial_palette"
    );
    assert_eq!(inks.len(), 4, "mode 1's full budget should have been used");
    assert_eq!(inks[0], Ink::from(0u8), "pen 0 must stay exactly the pinned ink");
    assert!(
        inks[1..].iter().all(|&i| i != Ink::from(0u8)),
        "the auto-filled pens must not duplicate the pinned one: {inks:?}"
    );
    let distinct: std::collections::HashSet<_> = inks.iter().collect();
    assert_eq!(distinct.len(), 4, "all 4 pens must end up distinct: {inks:?}");
}

#[test]
fn out_width_and_out_height_resize_before_conversion() {
    let conf_out = std::env::temp_dir().join("cpclib_tc_out_dims.asm");
    let sprite_out = std::env::temp_dir().join("cpclib_tc_out_dims.spr");
    let _ = std::fs::remove_file(&conf_out);
    let _ = std::fs::remove_file(&sprite_out);

    // Mode 0 packs 2 pixels per byte, so a requested width of 8 pixels must
    // come back as a 4-byte-wide sprite; height is in raster lines and is
    // not mode-dependent.
    run(&[
        "img2cpc",
        "--mode",
        "0",
        FIXTURE,
        "sprite",
        "--out-width",
        "8",
        "--out-height",
        "6",
        "-o",
        sprite_out.to_str().unwrap(),
        "-c",
        conf_out.to_str().unwrap()
    ])
    .expect("the conversion must succeed");

    let conf = std::fs::read_to_string(&conf_out).expect("the configuration file must exist");
    let _ = std::fs::remove_file(&conf_out);
    let _ = std::fs::remove_file(&sprite_out);

    assert!(conf.contains("_WIDTH equ 4"), "expected a 4-byte-wide sprite, got: {conf}");
    assert!(conf.contains("_HEIGHT equ 6"), "expected a 6-line-tall sprite, got: {conf}");
}
