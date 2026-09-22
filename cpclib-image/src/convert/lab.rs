//! Lab-space colour utilities for the true-color conversion pipeline.
//!
//! Nearest-color search, palette clustering and dithering all compare colors
//! through [`lab_distance`], which uses CIEDE2000 rather than a raw Euclidean
//! distance - the eye is far more sensitive to some regions of the Lab space
//! than others, and accounting for that is exactly what keeps quantization
//! and dithering artefacts less visible than a naive RGB-distance approach
//! would produce.

use std::sync::LazyLock;

use image as im;
use palette::color_difference::Ciede2000;
use palette::white_point::D65;
use palette::{FromColor, Lab, Srgb, Xyz};

use crate::asic::AsicColor;
use crate::color::AmstradColor;
use crate::ink::Ink;

/// CIE L*a*b*, D65 white point, `f32` components - the space every distance
/// comparison and every clustering/dithering operation in this pipeline
/// works in.
pub type LabF32 = Lab<D65, f32>;

pub fn rgb8_to_lab(rgb: im::Rgb<u8>) -> LabF32 {
    let srgb_u8 = Srgb::new(rgb[0], rgb[1], rgb[2]);
    let srgb_f32: Srgb<f32> = srgb_u8.into_format();
    let xyz: Xyz<D65, f32> = Xyz::from_color(srgb_f32);
    Lab::from_color(xyz)
}

pub fn lab_to_rgb8(lab: LabF32) -> im::Rgb<u8> {
    let xyz: Xyz<D65, f32> = Xyz::from_color(lab);
    let srgb_f32: Srgb<f32> = Srgb::from_color(xyz);
    let srgb_u8: Srgb<u8> = srgb_f32.into_format();
    im::Rgb([srgb_u8.red, srgb_u8.green, srgb_u8.blue])
}

/// Perceptual distance between two Lab colors (CIEDE2000 delta-E). Smaller
/// is closer; a difference below roughly 1.0 is not visually distinguishable.
pub fn lab_distance(a: LabF32, b: LabF32) -> f32 {
    a.difference(b)
}

/// Lab value of every distinct Gate Array ink. `Ink::INKS` has 32 entries -
/// 27 real inks at indices 0-26, plus 5 duplicate aliases at 27-31 mapping
/// back to 13/7/25/1/19 (see `Ink::duplicate`) - so only the first 27 are
/// needed here.
static INK_LAB: LazyLock<[(Ink, LabF32); 27]> =
    LazyLock::new(|| std::array::from_fn(|i| {
        let ink = Ink::INKS[i];
        (ink, rgb8_to_lab(ink.color()))
    }));

/// Nearest of the 27 Gate Array inks to `target`, by perceptual distance.
pub fn nearest_ink_lab(target: LabF32) -> Ink {
    INK_LAB
        .iter()
        .min_by(|(_, a), (_, b)| lab_distance(target, *a).total_cmp(&lab_distance(target, *b)))
        .map(|&(ink, _)| ink)
        .unwrap()
}

/// Snap an arbitrary Lab point to the nearest color the target hardware can
/// actually display - the last step of automatic palette selection, and the
/// only place the Gate Array and the Plus genuinely differ in this pipeline.
pub trait SnapToHardware: AmstradColor {
    /// Nearest representable hardware color to `lab`.
    fn snap_from_lab(lab: LabF32) -> Self;

    /// Nearest representable hardware color to `lab`, excluding anything
    /// already in `claimed` - used to resolve collisions when automatic
    /// palette selection snaps two different clusters onto the same color.
    fn snap_excluding_from_lab(lab: LabF32, claimed: &std::collections::HashSet<Self>) -> Self;
}

impl SnapToHardware for Ink {
    /// Exhaustive nearest-of-27 search - the Gate Array's inks are not a
    /// regular grid, so there is no shortcut.
    fn snap_from_lab(lab: LabF32) -> Self {
        nearest_ink_lab(lab)
    }

    fn snap_excluding_from_lab(lab: LabF32, claimed: &std::collections::HashSet<Self>) -> Self {
        INK_LAB
            .iter()
            .filter(|(ink, _)| !claimed.contains(ink))
            .min_by(|(_, a), (_, b)| lab_distance(lab, *a).total_cmp(&lab_distance(lab, *b)))
            .map(|&(ink, _)| ink)
            .expect("mode.max_colors() <= 16 < 27, so claimed can never exhaust the 27 inks")
    }
}

impl SnapToHardware for AsicColor {
    /// The ASIC's 4-bit-per-channel space is a regular grid, so the nearest
    /// representable color is exact arithmetic, not a search - reusing
    /// `AsicColor::from(Rgb<u8>)`, which already rounds each 8-bit channel to
    /// the nearest 1/16 step.
    fn snap_from_lab(lab: LabF32) -> Self {
        AsicColor::from(lab_to_rgb8(lab))
    }

    fn snap_excluding_from_lab(lab: LabF32, claimed: &std::collections::HashSet<Self>) -> Self {
        let base = Self::snap_from_lab(lab);
        if !claimed.contains(&base) {
            return base;
        }

        // Regular 16x16x16 grid: expand a search radius around `base` one
        // step at a time (Chebyshev distance) until a free grid point is
        // found. `claimed.len()` is always well under 4096 (mode.max_colors()
        // <= 16), so this always terminates quickly.
        fn shift(base: AsicColor, dr: i32, dg: i32, db: i32) -> AsicColor {
            let clamp = |v: u8, d: i32| (v as i32 + d).clamp(0, 15) as u8;
            AsicColor::new(
                clamp(base.get_red().value(), dr),
                clamp(base.get_green().value(), dg),
                clamp(base.get_blue().value(), db)
            )
        }

        for radius in 1..=15i32 {
            let mut best: Option<(AsicColor, i32)> = None;
            for dr in -radius..=radius {
                for dg in -radius..=radius {
                    for db in -radius..=radius {
                        if dr.abs().max(dg.abs()).max(db.abs()) != radius {
                            continue; // only the new shell at this radius
                        }
                        let candidate = shift(base, dr, dg, db);
                        if !claimed.contains(&candidate) {
                            let d2 = dr * dr + dg * dg + db * db;
                            if best.is_none_or(|(_, best_d2)| d2 < best_d2) {
                                best = Some((candidate, d2));
                            }
                        }
                    }
                }
            }
            if let Some((candidate, _)) = best {
                return candidate;
            }
        }

        panic!("AsicColor::snap_excluding_from_lab: claimed set exhausted the whole 16^3 grid")
    }
}

/// `(color, Lab)` pairs for a runtime palette (<=16 entries) - what the
/// dither stage searches against. Distinct from the fixed 27-ink `INK_LAB`
/// table above, which is only used during automatic palette selection's snap
/// step.
pub fn palette_lab<C: AmstradColor>(colors: &[C]) -> Vec<(C, LabF32)> {
    colors.iter().map(|&c| (c, rgb8_to_lab(c.color()))).collect()
}

/// Nearest palette entry to `target`, by perceptual distance.
pub fn nearest_in_palette<C: AmstradColor>(
    target: LabF32,
    palette: &[(C, LabF32)]
) -> (C, LabF32) {
    *palette
        .iter()
        .min_by(|(_, a), (_, b)| lab_distance(target, *a).total_cmp(&lab_distance(target, *b)))
        .expect("palette must not be empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_lab_round_trip_is_close() {
        for rgb in [
            im::Rgb([0, 0, 0]),
            im::Rgb([255, 255, 255]),
            im::Rgb([255, 0, 0]),
            im::Rgb([0, 255, 0]),
            im::Rgb([0, 0, 255]),
            im::Rgb([128, 64, 200])
        ] {
            let lab = rgb8_to_lab(rgb);
            let back = lab_to_rgb8(lab);
            for i in 0..3 {
                let diff = (rgb[i] as i32 - back[i] as i32).abs();
                assert!(diff <= 2, "component {i} of {rgb:?} round-tripped to {back:?}");
            }
        }
    }

    #[test]
    fn nearest_ink_lab_picks_black_and_white() {
        assert_eq!(nearest_ink_lab(rgb8_to_lab(im::Rgb([0, 0, 0]))), Ink::BLACK);
        assert_eq!(
            nearest_ink_lab(rgb8_to_lab(im::Rgb([255, 255, 255]))),
            Ink::BRIGHTWHITE
        );
    }

    #[test]
    fn asic_snap_from_lab_matches_direct_rgb_rounding() {
        for rgb in [
            im::Rgb([10, 200, 33]),
            im::Rgb([255, 128, 0]),
            im::Rgb([17, 17, 17])
        ] {
            let via_lab = AsicColor::snap_from_lab(rgb8_to_lab(rgb));
            let direct = AsicColor::from(rgb);
            assert_eq!(via_lab, direct);
        }
    }
}
