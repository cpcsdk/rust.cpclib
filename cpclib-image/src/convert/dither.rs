//! Dithering algorithms for the true-color conversion pipeline: an
//! ordered/Bayer dither generalized to work with arbitrary/irregular
//! palettes (CPC hardware inks are not evenly spaced in perceptual space, so
//! naive per-channel Bayer thresholding, which assumes a regular color cube,
//! does not apply), and the classic error-diffusion family described in
//! Tanner Helland's "Dithering eleven algorithms" article
//! (<https://tannerhelland.com/2012/12/28/dithering-eleven-algorithms-source-code.html>).

use image::RgbImage;

use super::lab::{
    lab_distance, linear_to_lab, nearest_in_palette, rgb8_to_lab, rgb8_to_linear, LabF32, LinRgbF32
};
use crate::color::AmstradColor;
use crate::image::ColorMatrix;

/// Classic recursive Bayer matrix construction. `n` must be a power of two.
/// Values range over `0..n*n`, each exactly once.
pub fn bayer_matrix(n: usize) -> Vec<Vec<u32>> {
    assert!(
        n.is_power_of_two(),
        "Bayer matrix size must be a power of two, got {n}"
    );

    if n == 1 {
        return vec![vec![0]];
    }

    let half = n / 2;
    let m = bayer_matrix(half);
    let mut out = vec![vec![0u32; n]; n];
    for i in 0..half {
        for j in 0..half {
            let v = m[i][j];
            out[i][j] = 4 * v; // top-left
            out[i][j + half] = 4 * v + 2; // top-right
            out[i + half][j] = 4 * v + 3; // bottom-left
            out[i + half][j + half] = 4 * v + 1; // bottom-right
        }
    }
    out
}

/// Ordered dithering generalized for arbitrary/irregular palettes, following
/// Joel Yliluoma's "ordered dithering for arbitrary palettes" algorithm
/// (<https://bisqwit.iki.fi/story/howto/dither/jy/>). For each pixel:
///
/// - every pair of palette colors is a dithering candidate, not just pairs
///   involving the single nearest one - a target color can sit between two
///   colors that are each individually far from it;
/// - the mixing ratio is solved in closed form (a luma-weighted per-channel
///   least-squares fit, `best_linear_ratio`) rather than sampled at every
///   Bayer threshold level;
/// - the blend itself is computed in linear (gamma-decoded) RGB
///   (`LinRgbF32`), because that is the space two adjacent dithered pixels'
///   emitted light actually adds in - blending in Lab or gamma-encoded RGB
///   would not correspond to what a display physically produces, even
///   though Lab/CIEDE2000 is still what decides which candidate is closest;
/// - a candidate pair's own *chroma* dissimilarity (Euclidean distance in
///   the Lab a*/b* plane between its two colors - hue and saturation, with
///   lightness deliberately excluded) is added to its score - without this,
///   two colors that are nowhere near each other in hue can still average to
///   something numerically close to the target in linear RGB (e.g. red and
///   green metamerically averaging toward gray), but a real low-resolution
///   display does not optically blend adjacent pixels finely enough for that
///   to read as anything but colored speckle. Lightness is excluded from
///   this penalty on purpose: black and white are about as far apart as two
///   colors can be in full Lab distance, but blending them is the ordinary,
///   desired case for a grayscale ramp, not a metamerism problem - only a
///   *hue* mismatch between the two candidates is the failure mode this
///   guards against; and
/// - the winning pair is always mapped low-threshold -> darker,
///   high-threshold -> lighter, regardless of which of the two happened to
///   be nearest, so neighboring pixels' dither patterns stay tonally
///   consistent instead of fighting each other.
///
/// `edge_aware` (`--dither-edge-aware`) is an opt-in, non-standard extra on
/// top of Yliluoma's algorithm: the chroma-dissimilarity penalty above is a
/// purely per-pixel rule, so it cannot tell a flat region (where a
/// chroma-mismatched pair reads as unwanted colored speckle) from a fine
/// edge or line (where the same mismatched pair can actually help represent
/// genuine sub-pixel anti-aliasing detail from the source image). When
/// enabled, the penalty is relaxed in proportion to local lightness
/// contrast in the *source* image, so mismatched pairs stay suppressed in
/// flat areas but are allowed again near real detail. Off by default
/// because it is a deliberate trade-off, not a strict improvement: it can
/// reintroduce a little color fringing right at edges in exchange for
/// crisper-looking fine detail there.
pub fn ordered_arbitrary_dither<C: AmstradColor>(
    img: &RgbImage,
    palette: &[(C, LabF32, LinRgbF32)],
    bayer_size: usize,
    edge_aware: bool
) -> ColorMatrix<C> {
    /// Weight of a candidate pair's own chroma dissimilarity in its score,
    /// relative to how well the pair's blend fits the target pixel (the fit
    /// is a CIEDE2000 delta-E; the chroma term is a plain Euclidean a*/b*
    /// distance, a different scale - this weight is what reconciles them).
    /// Picked empirically by checking that a mid-gray photograph dithers
    /// into a neutral gray-black-white halftone rather than colored
    /// speckle, without also flattening out legitimate high-lightness-span
    /// pairs (black+white for a wide tonal range).
    const PAIR_DISSIMILARITY_WEIGHT: f32 = 0.05;

    /// Local lightness contrast (see `local_edge_strength`) at or above
    /// which the pair-dissimilarity penalty is fully relaxed, in `--dither
    /// -edge-aware` mode. Lab L units; picked empirically as "a real edge,
    /// not just quantization noise between two very similar tones."
    const EDGE_STRENGTH_FOR_FULL_RELAXATION: f32 = 15.0;

    let bayer = bayer_matrix(bayer_size);
    let levels = (bayer_size * bayer_size) as u32;
    let mut out = ColorMatrix::<C>::new(img.width() as usize, img.height() as usize);

    let edge_strength = edge_aware.then(|| local_edge_strength(img));

    for y in 0..img.height() {
        for x in 0..img.width() {
            let target_rgb = *img.get_pixel(x, y);
            let target = rgb8_to_lab(target_rgb);
            let target_lin = rgb8_to_linear(target_rgb);

            let pair_dissimilarity_weight = match &edge_strength {
                Some(strength) => {
                    let relaxation = (strength[(y * img.width() + x) as usize]
                        / EDGE_STRENGTH_FOR_FULL_RELAXATION)
                        .clamp(0.0, 1.0);
                    PAIR_DISSIMILARITY_WEIGHT * (1.0 - relaxation)
                },
                None => PAIR_DISSIMILARITY_WEIGHT
            };

            // (color_a, lab_a, color_b, lab_b, fraction toward b, distance).
            // a == b is the "no blend improves on the nearest single color"
            // case, always present as a starting candidate.
            let mut best: Option<(C, LabF32, C, LabF32, f32, f32)> = None;

            for &(ci, labi, _) in palette {
                let d = lab_distance(target, labi);
                if best.is_none_or(|(.., bd)| d < bd) {
                    best = Some((ci, labi, ci, labi, 0.0, d));
                }
            }

            for i in 0..palette.len() {
                let (ci, labi, lini) = palette[i];
                for &(cj, labj, linj) in &palette[i + 1..] {
                    let t = best_linear_ratio(target_lin, lini, linj);
                    let mixed_lin = LinRgbF32::new(
                        lini.red + (linj.red - lini.red) * t,
                        lini.green + (linj.green - lini.green) * t,
                        lini.blue + (linj.blue - lini.blue) * t
                    );
                    let fit = lab_distance(target, linear_to_lab(mixed_lin));

                    // Penalize dissimilar pairs directly (Yliluoma's
                    // ColorCompare(a,b) term), but only on chroma - a
                    // lightness-inclusive penalty would equally punish
                    // black+white, the ordinary case for a wide tonal
                    // range, along with the actual failure mode: two colors
                    // far apart in hue can still average to something
                    // numerically close to the target in linear RGB (e.g.
                    // red+green metamerically averaging to a target gray),
                    // but a real low-resolution display does not optically
                    // blend adjacent pixels finely enough for that to read
                    // as anything but colored speckle. Without this term
                    // the fit score alone would happily pick such a pair.
                    let score = fit + chroma_distance(labi, labj) * pair_dissimilarity_weight;

                    if best.is_none_or(|(.., bs)| score < bs) {
                        best = Some((ci, labi, cj, labj, t, score));
                    }
                }
            }

            let (a, lab_a, b, lab_b, frac_b, _) = best.expect("palette must not be empty");

            let chosen = if a == b {
                a
            }
            else {
                let (lo, hi, frac_hi) = if lab_a.l <= lab_b.l {
                    (a, b, frac_b)
                }
                else {
                    (b, a, 1.0 - frac_b)
                };
                let level = (frac_hi * levels as f32).round() as u32;
                let threshold = bayer[(y as usize) % bayer_size][(x as usize) % bayer_size];
                if threshold < level {
                    hi
                }
                else {
                    lo
                }
            };
            out.set_color(x as usize, y as usize, chosen);
        }
    }

    out
}

/// Distance between two colors in just the Lab a*/b* (chroma) plane,
/// deliberately ignoring lightness - see [`ordered_arbitrary_dither`]'s
/// pair-dissimilarity penalty for why: it must catch a hue mismatch (red vs
/// green) without also catching an ordinary wide lightness span (black vs
/// white), and full Lab/CIEDE2000 distance conflates the two.
fn chroma_distance(a: LabF32, b: LabF32) -> f32 {
    let da = a.a - b.a;
    let db = a.b - b.b;
    (da * da + db * db).sqrt()
}

/// Per-pixel local detail measure for `--dither-edge-aware`: the largest
/// absolute Lab-lightness difference between a pixel and its four direct
/// neighbors in the *source* image (missing neighbors at the image's edges
/// are just skipped). Near zero in a flat region; larger wherever the
/// source has a real edge or fine line.
fn local_edge_strength(img: &RgbImage) -> Vec<f32> {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let lightness: Vec<f32> = img.pixels().map(|p| rgb8_to_lab(*p).l).collect();
    let idx = |x: i32, y: i32| (y * w + x) as usize;

    let mut strength = vec![0.0f32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let center = lightness[idx(x, y)];
            let mut max_diff = 0.0f32;
            for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    max_diff = max_diff.max((lightness[idx(nx, ny)] - center).abs());
                }
            }
            strength[idx(x, y)] = max_diff;
        }
    }
    strength
}

/// Closed-form mixing ratio (fraction toward `b`) that best approximates
/// `target` as a blend of `a` and `b` in linear RGB: a luma-weighted (CCIR
/// 601 coefficients) per-channel least-squares fit, solved directly instead
/// of sampled - the same "solve mathematically" optimization Yliluoma's
/// article uses in place of brute-force ratio scanning. Clamped to `[0, 1]`:
/// extrapolating past either endpoint is never a valid mix.
fn best_linear_ratio(target: LinRgbF32, a: LinRgbF32, b: LinRgbF32) -> f32 {
    const WEIGHTS: [f32; 3] = [0.299, 0.587, 0.114];
    let a = [a.red, a.green, a.blue];
    let b = [b.red, b.green, b.blue];
    let t = [target.red, target.green, target.blue];

    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for k in 0..3 {
        let diff = b[k] - a[k];
        num += WEIGHTS[k] * diff * (t[k] - a[k]);
        den += WEIGHTS[k] * diff * diff;
    }

    if den <= f32::EPSILON {
        0.0
    }
    else {
        (num / den).clamp(0.0, 1.0)
    }
}

/// A weighted error-diffusion kernel: each `(dx, dy, weight)` tap propagates
/// `weight` of the current pixel's quantization error to the neighbor at
/// `(x + dx, y + dy)`.
pub struct DiffusionKernel {
    pub name: &'static str,
    pub taps: &'static [(i32, i32, f32)]
}

pub static FLOYD_STEINBERG: DiffusionKernel = DiffusionKernel {
    name: "floyd-steinberg",
    taps: &[
        (1, 0, 7.0 / 16.0),
        (-1, 1, 3.0 / 16.0),
        (0, 1, 5.0 / 16.0),
        (1, 1, 1.0 / 16.0)
    ]
};

pub static FALSE_FLOYD_STEINBERG: DiffusionKernel = DiffusionKernel {
    name: "false-floyd-steinberg",
    taps: &[(1, 0, 3.0 / 8.0), (0, 1, 3.0 / 8.0), (1, 1, 2.0 / 8.0)]
};

pub static JARVIS_JUDICE_NINKE: DiffusionKernel = DiffusionKernel {
    name: "jarvis-judice-ninke",
    taps: &[
        (1, 0, 7.0 / 48.0),
        (2, 0, 5.0 / 48.0),
        (-2, 1, 3.0 / 48.0),
        (-1, 1, 5.0 / 48.0),
        (0, 1, 7.0 / 48.0),
        (1, 1, 5.0 / 48.0),
        (2, 1, 3.0 / 48.0),
        (-2, 2, 1.0 / 48.0),
        (-1, 2, 3.0 / 48.0),
        (0, 2, 5.0 / 48.0),
        (1, 2, 3.0 / 48.0),
        (2, 2, 1.0 / 48.0)
    ]
};

pub static STUCKI: DiffusionKernel = DiffusionKernel {
    name: "stucki",
    taps: &[
        (1, 0, 8.0 / 42.0),
        (2, 0, 4.0 / 42.0),
        (-2, 1, 2.0 / 42.0),
        (-1, 1, 4.0 / 42.0),
        (0, 1, 8.0 / 42.0),
        (1, 1, 4.0 / 42.0),
        (2, 1, 2.0 / 42.0),
        (-2, 2, 1.0 / 42.0),
        (-1, 2, 2.0 / 42.0),
        (0, 2, 4.0 / 42.0),
        (1, 2, 2.0 / 42.0),
        (2, 2, 1.0 / 42.0)
    ]
};

pub static ATKINSON: DiffusionKernel = DiffusionKernel {
    // Deliberately propagates only 6/8 of the error - Bill Atkinson's
    // original design, which trades some banding for less noise.
    name: "atkinson",
    taps: &[
        (1, 0, 1.0 / 8.0),
        (2, 0, 1.0 / 8.0),
        (-1, 1, 1.0 / 8.0),
        (0, 1, 1.0 / 8.0),
        (1, 1, 1.0 / 8.0),
        (0, 2, 1.0 / 8.0)
    ]
};

pub static BURKES: DiffusionKernel = DiffusionKernel {
    name: "burkes",
    taps: &[
        (1, 0, 8.0 / 32.0),
        (2, 0, 4.0 / 32.0),
        (-2, 1, 2.0 / 32.0),
        (-1, 1, 4.0 / 32.0),
        (0, 1, 8.0 / 32.0),
        (1, 1, 4.0 / 32.0),
        (2, 1, 2.0 / 32.0)
    ]
};

pub static SIERRA3: DiffusionKernel = DiffusionKernel {
    name: "sierra-3",
    taps: &[
        (1, 0, 5.0 / 32.0),
        (2, 0, 3.0 / 32.0),
        (-2, 1, 2.0 / 32.0),
        (-1, 1, 4.0 / 32.0),
        (0, 1, 5.0 / 32.0),
        (1, 1, 4.0 / 32.0),
        (2, 1, 2.0 / 32.0),
        (-1, 2, 2.0 / 32.0),
        (0, 2, 3.0 / 32.0),
        (1, 2, 2.0 / 32.0)
    ]
};

pub static SIERRA2: DiffusionKernel = DiffusionKernel {
    name: "sierra-2",
    taps: &[
        (1, 0, 4.0 / 16.0),
        (2, 0, 3.0 / 16.0),
        (-2, 1, 1.0 / 16.0),
        (-1, 1, 2.0 / 16.0),
        (0, 1, 3.0 / 16.0),
        (1, 1, 2.0 / 16.0),
        (2, 1, 1.0 / 16.0)
    ]
};

pub static SIERRA_LITE: DiffusionKernel = DiffusionKernel {
    name: "sierra-lite",
    taps: &[(1, 0, 2.0 / 4.0), (-1, 1, 1.0 / 4.0), (0, 1, 1.0 / 4.0)]
};

/// Error-diffusion dithering: one generic driver serving every named kernel
/// above by simply swapping which one is passed in. Error is accumulated and
/// matched entirely in Lab space - never round-tripped back through RGB
/// mid-pass, which would mix two different notions of distance - converting
/// to `C`'s hardware representation only once per pixel, at the moment it is
/// finalized.
pub fn error_diffuse<C: AmstradColor>(
    img: &RgbImage,
    palette: &[(C, LabF32)],
    kernel: &DiffusionKernel
) -> ColorMatrix<C> {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let mut error = vec![(0.0f32, 0.0f32, 0.0f32); (w * h) as usize];
    let mut out = ColorMatrix::<C>::new(w as usize, h as usize);

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let true_lab = rgb8_to_lab(*img.get_pixel(x as u32, y as u32));
            let (el, ea, eb) = error[idx];
            let target = LabF32::new(true_lab.l + el, true_lab.a + ea, true_lab.b + eb);
            let (chosen, chosen_lab) = nearest_in_palette(target, palette);
            out.set_color(x as usize, y as usize, chosen);

            let (rl, ra, rb) = (
                target.l - chosen_lab.l,
                target.a - chosen_lab.a,
                target.b - chosen_lab.b
            );
            for &(dx, dy, weight) in kernel.taps {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    let e = &mut error[(ny * w + nx) as usize];
                    e.0 += rl * weight;
                    e.1 += ra * weight;
                    e.2 += rb * weight;
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;
    use crate::ink::Ink;

    #[test]
    fn bayer_matrix_4x4_matches_known_reference() {
        let m = bayer_matrix(4);
        assert_eq!(
            m,
            vec![
                vec![0, 8, 2, 10],
                vec![12, 4, 14, 6],
                vec![3, 11, 1, 9],
                vec![15, 7, 13, 5]
            ]
        );
    }

    fn black_white_palette() -> Vec<(Ink, LabF32)> {
        super::super::lab::palette_lab(&[Ink::BLACK, Ink::BRIGHTWHITE])
    }

    fn black_white_palette_linear() -> Vec<(Ink, LabF32, LinRgbF32)> {
        super::super::lab::palette_lab_and_linear(&[Ink::BLACK, Ink::BRIGHTWHITE])
    }

    fn mid_gray_gradient(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |_, _| Rgb([128, 128, 128]))
    }

    fn assert_only_palette_colors(matrix: &ColorMatrix<Ink>, palette: &[(Ink, LabF32)]) {
        let allowed: std::collections::HashSet<Ink> = palette.iter().map(|&(c, _)| c).collect();
        for y in 0..matrix.height() as usize {
            for x in 0..matrix.width() as usize {
                assert!(allowed.contains(matrix.get_color(x, y)));
            }
        }
    }

    #[test]
    fn local_edge_strength_is_zero_in_flat_regions_and_high_at_edges() {
        // Left half dark, right half bright - a sharp vertical edge down
        // the middle of an otherwise flat image.
        let img = RgbImage::from_fn(8, 4, |x, _| {
            if x < 4 {
                Rgb([0, 0, 0])
            }
            else {
                Rgb([255, 255, 255])
            }
        });
        let strength = local_edge_strength(&img);
        let idx = |x: usize, y: usize| y * 8 + x;

        assert_eq!(
            strength[idx(1, 2)],
            0.0,
            "deep in the dark half, no local contrast"
        );
        assert_eq!(
            strength[idx(6, 2)],
            0.0,
            "deep in the bright half, no local contrast"
        );
        assert!(
            strength[idx(3, 2)] > 50.0,
            "right at the edge, large local contrast"
        );
        assert!(
            strength[idx(4, 2)] > 50.0,
            "right at the edge, large local contrast"
        );
    }

    /// `edge_aware` must be a true no-op when there is no edge anywhere -
    /// the whole point is that it only relaxes the chroma penalty near real
    /// detail, so a fully flat image (its main use case: a photo's smooth
    /// background) must dither identically whether it's on or off.
    #[test]
    fn edge_aware_matches_default_on_a_fully_flat_image() {
        let img = mid_gray_gradient(8, 8);
        let palette = super::super::lab::palette_lab_and_linear(&[
            Ink::BLACK,
            Ink::BRIGHTWHITE,
            Ink::RED,
            Ink::BRIGHTGREEN
        ]);
        let without = ordered_arbitrary_dither(&img, &palette, 8, false);
        let with = ordered_arbitrary_dither(&img, &palette, 8, true);
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(
                    without.get_color(x, y),
                    with.get_color(x, y),
                    "pixel ({x},{y}) differed with no edges anywhere in the source"
                );
            }
        }
    }

    /// A real regression: with only a lightness-based guard, a candidate
    /// pair whose two colors are nowhere near each other in hue (e.g. red
    /// and green) can still average to something numerically close to a
    /// gray target in linear RGB, purely by metameric cancellation - and
    /// get picked over the much more sensible black/white pair. On an
    /// actual low-resolution display that reads as colored speckle, not
    /// gray, since adjacent pixels aren't optically blended finely enough.
    #[test]
    fn ordered_dither_prefers_black_white_over_a_metameric_red_green_pair_for_gray() {
        let img = mid_gray_gradient(16, 16);
        let palette = super::super::lab::palette_lab_and_linear(&[
            Ink::BLACK,
            Ink::BRIGHTWHITE,
            Ink::RED,
            Ink::BRIGHTGREEN
        ]);
        let out = ordered_arbitrary_dither(&img, &palette, 8, false);

        for y in 0..out.height() as usize {
            for x in 0..out.width() as usize {
                let color = *out.get_color(x, y);
                assert!(
                    color == Ink::BLACK || color == Ink::BRIGHTWHITE,
                    "pixel ({x},{y}) used {color:?} instead of black/white for a neutral gray \
                     target - a red/green metameric pair was picked instead"
                );
            }
        }
    }

    #[test]
    fn ordered_dither_uses_only_palette_colors() {
        let img = mid_gray_gradient(16, 16);
        let palette = black_white_palette_linear();
        let out = ordered_arbitrary_dither(&img, &palette, 8, false);
        assert_only_palette_colors(&out, &black_white_palette());
    }

    #[test]
    fn every_error_diffusion_kernel_uses_only_palette_colors() {
        let img = mid_gray_gradient(16, 16);
        let palette = black_white_palette();
        for kernel in [
            &FLOYD_STEINBERG,
            &FALSE_FLOYD_STEINBERG,
            &JARVIS_JUDICE_NINKE,
            &STUCKI,
            &ATKINSON,
            &BURKES,
            &SIERRA3,
            &SIERRA2,
            &SIERRA_LITE
        ] {
            let out = error_diffuse(&img, &palette, kernel);
            assert_only_palette_colors(&out, &palette);
        }
    }

    #[test]
    fn mid_gray_dithers_to_roughly_half_each_color() {
        let img = mid_gray_gradient(32, 32);
        let palette = black_white_palette();
        let out = error_diffuse(&img, &palette, &FLOYD_STEINBERG);
        let total = 32 * 32;
        let white_count = (0..32)
            .flat_map(|y| (0..32).map(move |x| (x, y)))
            .filter(|&(x, y)| *out.get_color(x, y) == Ink::BRIGHTWHITE)
            .count();
        let ratio = white_count as f32 / total as f32;
        assert!((0.3..0.7).contains(&ratio), "white ratio was {ratio}");
    }
}
