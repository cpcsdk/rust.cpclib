//! Dithering algorithms for the true-color conversion pipeline: an
//! ordered/Bayer dither generalized to work with arbitrary/irregular
//! palettes (CPC hardware inks are not evenly spaced in perceptual space, so
//! naive per-channel Bayer thresholding, which assumes a regular color cube,
//! does not apply), and the classic error-diffusion family described in
//! Tanner Helland's "Dithering eleven algorithms" article
//! (<https://tannerhelland.com/2012/12/28/dithering-eleven-algorithms-source-code.html>).

use image::RgbImage;

use super::lab::{lab_distance, nearest_in_palette, rgb8_to_lab, LabF32};
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

/// Ordered dithering generalized for arbitrary/irregular palettes (Yliluoma
/// ordered dithering algorithm 3-style): for each pixel, find the nearest
/// palette color, then search blends of it with every other palette entry in
/// perceptual space to find the best-fitting pair and mixing ratio, then use
/// a Bayer threshold matrix value at that pixel's position to deterministically
/// choose between the pair according to the ratio. Restricted to pairs
/// involving the single nearest color (rather than every pair in the
/// palette) as an explicit complexity trade-off.
pub fn ordered_arbitrary_dither<C: AmstradColor>(
    img: &RgbImage,
    palette: &[(C, LabF32)],
    bayer_size: usize
) -> ColorMatrix<C> {
    let bayer = bayer_matrix(bayer_size);
    let levels = (bayer_size * bayer_size) as u32;
    let mut out = ColorMatrix::<C>::new(img.width() as usize, img.height() as usize);

    for y in 0..img.height() {
        for x in 0..img.width() {
            let target = rgb8_to_lab(*img.get_pixel(x, y));
            let (c0, lab0) = nearest_in_palette(target, palette);

            let mut best_color = c0;
            let mut best_level = 0u32;
            let mut best_dist = lab_distance(target, lab0);

            for &(cj, labj) in palette.iter() {
                if cj == c0 {
                    continue;
                }
                for level in 0..=levels {
                    let t = level as f32 / levels as f32;
                    let blend = LabF32::new(
                        lab0.l * (1.0 - t) + labj.l * t,
                        lab0.a * (1.0 - t) + labj.a * t,
                        lab0.b * (1.0 - t) + labj.b * t
                    );
                    let d = lab_distance(target, blend);
                    if d < best_dist {
                        best_dist = d;
                        best_color = cj;
                        best_level = level;
                    }
                }
            }

            let threshold = bayer[(y as usize) % bayer_size][(x as usize) % bayer_size];
            let chosen = if threshold < best_level {
                best_color
            }
            else {
                c0
            };
            out.set_color(x as usize, y as usize, chosen);
        }
    }

    out
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
    fn ordered_dither_uses_only_palette_colors() {
        let img = mid_gray_gradient(16, 16);
        let palette = black_white_palette();
        let out = ordered_arbitrary_dither(&img, &palette, 8);
        assert_only_palette_colors(&out, &palette);
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
