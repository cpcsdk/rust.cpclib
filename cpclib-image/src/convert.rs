//! True-color-to-CPC conversion.
//!
//! [`crate::transfer`] *transfers* images: it assumes the source is already
//! at the exact target CPC pixel resolution and already uses (near enough)
//! CPC hardware colors. This module does the real work a true-color source
//! needs first: resize an arbitrary-size image to the target resolution,
//! automatically select (or reuse a user-supplied) CPC palette, and
//! dither/quantize into it using perceptually-uniform color math - producing
//! a [`ColorMatrix<C>`] and [`Palette<C>`] that already satisfy every
//! assumption [`crate::transfer::ImageConverter`] makes about its input
//! (exact resolution, distinct-color count within the mode's budget, every
//! pixel a member of the returned palette), ready for
//! [`crate::transfer::ImageConverter::convert_from_matrix`].

pub mod dither;
pub mod lab;
pub mod palette_select;
pub mod resize;

use cpclib_common::camino::Utf8Path;
use image as im;
pub use resize::ResizeFilter;

use self::dither::DiffusionKernel;
use self::lab::SnapToHardware;
use crate::color::AmstradColor;
use crate::image::{ColorMatrix, Mode};
use crate::palette::{LockablePalette, Palette};

/// `--dither` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DitherAlgorithm {
    /// Ordered dithering generalized for arbitrary/irregular palettes -
    /// see [`dither::ordered_arbitrary_dither`].
    OrderedArbitrary,
    FloydSteinberg,
    FalseFloydSteinberg,
    JarvisJudiceNinke,
    Stucki,
    Atkinson,
    Burkes,
    Sierra3,
    Sierra2,
    SierraLite
}

impl DitherAlgorithm {
    fn kernel(self) -> Option<&'static DiffusionKernel> {
        match self {
            Self::OrderedArbitrary => None,
            Self::FloydSteinberg => Some(&dither::FLOYD_STEINBERG),
            Self::FalseFloydSteinberg => Some(&dither::FALSE_FLOYD_STEINBERG),
            Self::JarvisJudiceNinke => Some(&dither::JARVIS_JUDICE_NINKE),
            Self::Stucki => Some(&dither::STUCKI),
            Self::Atkinson => Some(&dither::ATKINSON),
            Self::Burkes => Some(&dither::BURKES),
            Self::Sierra3 => Some(&dither::SIERRA3),
            Self::Sierra2 => Some(&dither::SIERRA2),
            Self::SierraLite => Some(&dither::SIERRA_LITE)
        }
    }
}

/// Parameters for [`convert_true_color`].
pub struct TrueColorParams<C: AmstradColor> {
    pub resize_filter: ResizeFilter,
    pub dither: DitherAlgorithm,
    /// Caps automatic palette selection below the mode's own budget; `None`
    /// uses the full budget. Has no effect when `hint` is locked.
    pub max_colors: Option<usize>,
    /// The user's palette request, exactly as `get_requested_palette`
    /// produces it: locked means "use these colors exactly, no
    /// clustering"; unlocked (with zero or more pens already set) means
    /// "keep whatever's pinned, auto-fill the rest".
    pub hint: LockablePalette<C>,
    pub target_width: u32,
    pub target_height: u32,
    /// Bayer matrix size for [`DitherAlgorithm::OrderedArbitrary`] (must be
    /// a power of two - 2, 4 or 8). Unused by every other algorithm.
    pub bayer_size: usize
}

/// Resize -> resolve palette -> dither/quantize in Lab space. The returned
/// [`ColorMatrix<C>`]'s colors are already exactly members of the returned
/// [`Palette<C>`], and its distinct-color count is already within budget -
/// exactly what [`crate::transfer::ImageConverter::convert_from_matrix`]
/// needs to skip its own quantization.
pub fn convert_true_color<C, P>(
    input_file: P,
    mode: Mode,
    params: TrueColorParams<C>
) -> anyhow::Result<(ColorMatrix<C>, Palette<C>)>
where
    C: AmstradColor + SnapToHardware,
    P: AsRef<Utf8Path>
{
    let img = im::open(input_file.as_ref())
        .map_err(|e| anyhow::anyhow!("Unable to open {:?}: {e}", input_file.as_ref()))?
        .to_rgb8();
    let resized = resize::resize_to_target(
        &img,
        params.target_width,
        params.target_height,
        params.resize_filter
    );

    let palette = if params.hint.is_locked() {
        params.hint.as_palette().clone()
    }
    else {
        let hint_palette = params.hint.as_palette().clone();
        let pinned = hint_palette.colors();
        let max_colors = params
            .max_colors
            .unwrap_or_else(|| mode.max_colors())
            .min(mode.max_colors());

        let chosen = palette_select::auto_select_palette::<C>(&resized, max_colors, &pinned);

        let mut palette = hint_palette;
        for &color in &chosen[pinned.len()..] {
            let pen = palette.next_unused_pen_for_mode(mode).expect(
                "auto_select_palette never returns more colors than max_colors <= mode.max_colors()"
            );
            palette.set(pen, color);
        }
        palette
    };

    let pal_lab = lab::palette_lab(&palette.colors());

    let matrix = match params.dither.kernel() {
        None => dither::ordered_arbitrary_dither(&resized, &pal_lab, params.bayer_size),
        Some(kernel) => dither::error_diffuse(&resized, &pal_lab, kernel)
    };

    Ok((matrix, palette))
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;
    use crate::ink::Ink;

    #[test]
    fn end_to_end_over_budget_image_fits_mode_budget() {
        let path = std::env::temp_dir().join(format!(
            "cpclib_image_convert_test_{}.png",
            std::process::id()
        ));

        let img = image::RgbImage::from_fn(16, 16, |x, y| {
            Rgb([(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8])
        });
        img.save(&path).unwrap();

        let utf8_path: &cpclib_common::camino::Utf8Path =
            cpclib_common::camino::Utf8Path::from_path(&path).unwrap();
        let (matrix, palette) = convert_true_color::<Ink, _>(
            utf8_path,
            Mode::One,
            TrueColorParams {
                resize_filter: ResizeFilter::Nearest,
                dither: DitherAlgorithm::FloydSteinberg,
                max_colors: None,
                hint: LockablePalette::empty(),
                target_width: 16,
                target_height: 16,
                bayer_size: 8
            }
        )
        .unwrap();

        let _ = fs_err::remove_file(&path);

        assert!(matrix.nb_colors() <= Mode::One.max_colors());
        let palette_colors = palette.colors();
        for y in 0..matrix.height() as usize {
            for x in 0..matrix.width() as usize {
                assert!(palette_colors.contains(matrix.get_color(x, y)));
            }
        }
    }
}
