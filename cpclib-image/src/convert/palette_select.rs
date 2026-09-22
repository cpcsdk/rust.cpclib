//! Automatic palette selection for the true-color conversion pipeline.
//!
//! Generalizes the existing "hint palette" mechanism
//! (`Palette::next_unused_pen_for_mode`, `extract_palette_with_hint` in
//! `crate::image`, driven from the CLI by `--penN`/`--unlock-pens`): pens
//! the user already pinned stay exactly as given, and clustering only fills
//! whatever pens remain, using the pinned colors as anchors the free
//! clusters are built around rather than duplicated.

use std::collections::{HashMap, HashSet};

use image::RgbImage;

use super::lab::{LabF32, SnapToHardware, lab_distance, rgb8_to_lab};
use crate::color::AmstradColor;

/// Weighted, deterministic k-means (generalized Lloyd's algorithm) in Lab
/// space, followed by snapping each free centroid to an actual hardware
/// color and deduplicating collisions. `pinned` is whatever colors the user
/// already fixed (empty when no palette was given at all) - they are kept
/// exactly as given, even if a pure Lab-nearest search would have picked
/// something else, and act as anchors the free clusters are built around.
/// Deterministic end to end - no RNG anywhere, so the same image and the
/// same pins always yield the same result.
///
/// Returns `pinned` followed by the resolved free colors, all distinct,
/// length at most `max_colors`.
pub fn auto_select_palette<C: AmstradColor + SnapToHardware>(
    img: &RgbImage,
    max_colors: usize,
    pinned: &[C]
) -> Vec<C> {
    let k = max_colors.saturating_sub(pinned.len());
    if k == 0 {
        // Every available pen was already pinned - nothing left to fill.
        return pinned.to_vec();
    }

    // Weighted histogram, sorted so every tie-break below (farthest-point
    // seeding, empty-cluster reseeding, snap collision order) is
    // reproducible rather than dependent on hash-map iteration order.
    let mut counts: HashMap<image::Rgb<u8>, u32> = HashMap::new();
    for p in img.pixels() {
        *counts.entry(*p).or_insert(0) += 1;
    }
    let mut uniques: Vec<(LabF32, u32)> =
        counts.into_iter().map(|(rgb, w)| (rgb8_to_lab(rgb), w)).collect();
    uniques.sort_by(|(a, _), (b, _)| {
        a.l.total_cmp(&b.l).then(a.a.total_cmp(&b.a)).then(a.b.total_cmp(&b.b))
    });

    let pinned_lab: Vec<LabF32> = pinned.iter().map(|&c| rgb8_to_lab(c.color())).collect();

    let free_centroids = if uniques.len() <= k {
        // Few enough residual colors that every one gets its own cluster -
        // no need to iterate.
        uniques.iter().map(|&(lab, _)| lab).collect()
    }
    else {
        weighted_kmeans(&uniques, &pinned_lab, k)
    };

    let mut claimed: HashSet<C> = pinned.iter().copied().collect();
    let resolved = snap_and_dedup::<C>(&free_centroids, &uniques, &pinned_lab, &mut claimed);

    let mut result = pinned.to_vec();
    result.extend(resolved);
    result
}

/// Nearest of `pinned_lab`/`free`, as a squared-away distance - used by both
/// the k-means loop and the dedup weighting below, which need the same
/// "what does this pixel actually get served by" notion.
fn nearest_distance(lab: LabF32, pinned_lab: &[LabF32], free: &[LabF32]) -> f32 {
    let d_pinned = pinned_lab.iter().map(|&c| lab_distance(lab, c)).fold(f32::MAX, f32::min);
    let d_free = free.iter().map(|&c| lab_distance(lab, c)).fold(f32::MAX, f32::min);
    d_pinned.min(d_free)
}

/// Index of the free centroid nearest `lab`, or `None` if a pinned anchor is
/// closer.
fn nearest_free(lab: LabF32, pinned_lab: &[LabF32], free: &[LabF32]) -> Option<usize> {
    let mut best_idx = None;
    let mut best_d = pinned_lab.iter().map(|&c| lab_distance(lab, c)).fold(f32::MAX, f32::min);
    for (j, &c) in free.iter().enumerate() {
        let d = lab_distance(lab, c);
        if d < best_d {
            best_d = d;
            best_idx = Some(j);
        }
    }
    best_idx
}

fn weighted_kmeans(uniques: &[(LabF32, u32)], pinned_lab: &[LabF32], k: usize) -> Vec<LabF32> {
    let mut free: Vec<LabF32> = Vec::with_capacity(k);

    if pinned_lab.is_empty() {
        // Seed 0: highest-weight color (first occurrence wins ties, and
        // `uniques` is pre-sorted, so this is reproducible).
        let mut best_i = 0;
        let mut best_w = 0u32;
        for (i, &(_, w)) in uniques.iter().enumerate() {
            if w > best_w {
                best_w = w;
                best_i = i;
            }
        }
        free.push(uniques[best_i].0);
    }

    // Farthest-point seeding for the rest, relative to whatever anchors
    // (pinned colors + already-seeded free centroids) exist so far.
    while free.len() < k {
        let mut best_i = 0;
        let mut best_d = -1.0f32;
        for (i, &(lab, _)) in uniques.iter().enumerate() {
            let d = nearest_distance(lab, pinned_lab, &free);
            if d > best_d {
                best_d = d;
                best_i = i;
            }
        }
        free.push(uniques[best_i].0);
    }

    for _ in 0..32 {
        let mut sums = vec![(0.0f32, 0.0f32, 0.0f32, 0.0f32); k]; // l, a, b, weight
        for &(lab, w) in uniques {
            if let Some(j) = nearest_free(lab, pinned_lab, &free) {
                let s = &mut sums[j];
                s.0 += lab.l * w as f32;
                s.1 += lab.a * w as f32;
                s.2 += lab.b * w as f32;
                s.3 += w as f32;
            }
        }

        let mut moved = false;
        for j in 0..k {
            let s = sums[j];
            if s.3 > 0.0 {
                let new_c = LabF32::new(s.0 / s.3, s.1 / s.3, s.2 / s.3);
                if lab_distance(new_c, free[j]) > 1e-3 {
                    moved = true;
                }
                free[j] = new_c;
            }
            else {
                // Empty free cluster: reseed to the unique color currently
                // farthest from whichever centroid (pinned or another free
                // one) it's actually closest to.
                let mut worst_i = 0;
                let mut worst_d = -1.0f32;
                for (i, &(lab, _)) in uniques.iter().enumerate() {
                    let others: Vec<LabF32> = free
                        .iter()
                        .enumerate()
                        .filter(|&(fj, _)| fj != j)
                        .map(|(_, &c)| c)
                        .collect();
                    let d = nearest_distance(lab, pinned_lab, &others);
                    if d > worst_d {
                        worst_d = d;
                        worst_i = i;
                    }
                }
                free[j] = uniques[worst_i].0;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    free
}

/// Snaps every free centroid to hardware, processing the heaviest-weight
/// one first; on collision (two centroids - or a centroid and a pinned
/// color - snapping to the same hardware color), the lighter one re-snaps
/// to the nearest hardware color not already in `claimed`. Returns the
/// resolved colors in the same order as `free_centroids`.
fn snap_and_dedup<C: AmstradColor + SnapToHardware>(
    free_centroids: &[LabF32],
    uniques: &[(LabF32, u32)],
    pinned_lab: &[LabF32],
    claimed: &mut HashSet<C>
) -> Vec<C> {
    let weight_of = |j: usize| -> u32 {
        uniques
            .iter()
            .filter(|&&(lab, _)| nearest_free(lab, pinned_lab, free_centroids) == Some(j))
            .map(|&(_, w)| w)
            .sum()
    };

    let mut order: Vec<usize> = (0..free_centroids.len()).collect();
    let weights: Vec<u32> = (0..free_centroids.len()).map(weight_of).collect();
    order.sort_by(|&a, &b| weights[b].cmp(&weights[a]).then(a.cmp(&b)));

    let mut snapped: Vec<Option<C>> = vec![None; free_centroids.len()];
    for j in order {
        let direct = C::snap_from_lab(free_centroids[j]);
        let candidate =
            if claimed.contains(&direct) { C::snap_excluding_from_lab(free_centroids[j], claimed) } else { direct };
        claimed.insert(candidate);
        snapped[j] = Some(candidate);
    }

    snapped.into_iter().map(Option::unwrap).collect()
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;
    use crate::ink::Ink;

    fn image_with_colors(colors: &[(u8, u8, u8, u32)]) -> RgbImage {
        let total: u32 = colors.iter().map(|&(_, _, _, w)| w).sum();
        let mut img = RgbImage::new(total.max(1), 1);
        let mut x = 0u32;
        for &(r, g, b, w) in colors {
            for _ in 0..w {
                img.put_pixel(x, 0, Rgb([r, g, b]));
                x += 1;
            }
        }
        img
    }

    #[test]
    fn exact_fit_round_trips_unchanged() {
        let img = image_with_colors(&[(0, 0, 0, 5), (255, 255, 255, 5)]);
        let palette = auto_select_palette::<Ink>(&img, 4, &[]);
        assert_eq!(palette.len(), 2);
        assert!(palette.contains(&Ink::BLACK));
        assert!(palette.contains(&Ink::BRIGHTWHITE));
    }

    #[test]
    fn over_budget_yields_exactly_max_colors_distinct() {
        // 8 quite different colors squeezed into a 3-color budget.
        let img = image_with_colors(&[
            (0, 0, 0, 20),
            (30, 0, 0, 5),
            (255, 255, 255, 20),
            (200, 255, 255, 5),
            (255, 0, 0, 10),
            (0, 255, 0, 10),
            (0, 0, 255, 10),
            (128, 128, 0, 5)
        ]);
        let palette = auto_select_palette::<Ink>(&img, 3, &[]);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette.iter().collect::<HashSet<_>>().len(), 3);
    }

    #[test]
    fn selection_is_deterministic() {
        let img = image_with_colors(&[
            (0, 0, 0, 20),
            (255, 255, 255, 20),
            (255, 0, 0, 10),
            (0, 255, 0, 10),
            (0, 0, 255, 10),
            (128, 128, 0, 5),
            (30, 60, 90, 7)
        ]);
        let a = auto_select_palette::<Ink>(&img, 4, &[]);
        let b = auto_select_palette::<Ink>(&img, 4, &[]);
        assert_eq!(a, b);
    }

    #[test]
    fn pinned_colors_survive_unchanged_and_are_not_resnapped() {
        // Pin an ink that is a poor Lab-optimal choice for this image on
        // purpose (bright red, while the image is all blues/greens), to
        // prove it is kept exactly rather than re-optimized away.
        let img = image_with_colors(&[
            (0, 0, 255, 20),
            (0, 255, 0, 20),
            (10, 10, 200, 5),
            (10, 200, 10, 5)
        ]);
        let pinned = [Ink::RED];
        let palette = auto_select_palette::<Ink>(&img, 3, &pinned);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette[0], Ink::RED);
        // The other 2 pens are free and must not duplicate the pin.
        assert!(!palette[1..].contains(&Ink::RED));
    }

    #[test]
    fn fully_pinned_budget_runs_no_clustering() {
        let img = image_with_colors(&[(0, 0, 0, 10), (255, 255, 255, 10), (255, 0, 0, 10)]);
        let pinned = [Ink::BLACK, Ink::BRIGHTWHITE];
        let palette = auto_select_palette::<Ink>(&img, 2, &pinned);
        assert_eq!(palette, vec![Ink::BLACK, Ink::BRIGHTWHITE]);
    }
}
