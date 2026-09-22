//! Resizing a true-color source image to the exact target CPC pixel
//! resolution, before any color quantization/dithering runs.

use image::imageops::FilterType;
use image::RgbImage;

/// `--resize-filter` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3
}

impl ResizeFilter {
    pub fn to_image_filter(self) -> FilterType {
        match self {
            Self::Nearest => FilterType::Nearest,
            Self::Triangle => FilterType::Triangle,
            Self::CatmullRom => FilterType::CatmullRom,
            Self::Gaussian => FilterType::Gaussian,
            Self::Lanczos3 => FilterType::Lanczos3
        }
    }
}

/// Resize to exactly `(target_width, target_height)`, ignoring aspect ratio -
/// the CPC target resolution *is* the aspect ratio to hit, there is nothing
/// to preserve. A no-op when the source already matches, to avoid a lossy
/// resample-to-same-size round trip.
pub fn resize_to_target(
    img: &RgbImage,
    target_width: u32,
    target_height: u32,
    filter: ResizeFilter
) -> RgbImage {
    if img.width() == target_width && img.height() == target_height {
        img.clone()
    }
    else {
        image::imageops::resize(img, target_width, target_height, filter.to_image_filter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_produces_exact_target_dimensions() {
        let img = RgbImage::from_fn(10, 20, |x, y| image::Rgb([x as u8, y as u8, 0]));
        let resized = resize_to_target(&img, 37, 41, ResizeFilter::Lanczos3);
        assert_eq!(resized.width(), 37);
        assert_eq!(resized.height(), 41);
    }

    #[test]
    fn resize_is_noop_when_already_target_size() {
        let img = RgbImage::from_fn(10, 20, |x, y| image::Rgb([x as u8, y as u8, 0]));
        let resized = resize_to_target(&img, 10, 20, ResizeFilter::Nearest);
        assert_eq!(resized, img);
    }
}
