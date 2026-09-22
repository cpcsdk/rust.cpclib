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

use super::lab::{lab_distance, rgb8_to_lab, LabF32, SnapToHardware};
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
///
/// `prefer_neutral` (`--prefer-neutral-palette`) makes the final
/// hardware-snapping step avoid introducing more chroma than each cluster
/// centroid already has, rather than accepting whatever unclaimed ink is
/// nearest overall. It's opt-in because it's a genuine trade-off, not a
/// strict improvement: it helps a lot for a photo that's mostly grayscale
/// or pastel (fewer of the 16 pens end up wasted on fully-saturated
/// substitutes once the few genuinely neutral inks are taken), but for a
/// vividly colorful source it will make some clusters fit their target hue
/// less precisely than the unbiased nearest-match would have.
///
/// `prefer_salient` (`--prefer-salient-palette`) boosts a color's influence
/// on clustering by how far it is from the image's own dominant tone - see
/// [`SALIENCE_BOOST`]. Without it, plain frequency weighting can let a
/// small but strikingly different region (a handful of bright stars against
/// a huge dark sky) get diluted away entirely once the color budget is
/// small, even though it's the most visually important part of the source.
/// Opt-in because it's also a trade-off: for a source with no real outlier
/// colors, it does nothing useful and can very slightly skew clustering
/// toward whatever mild outliers do exist.
pub fn auto_select_palette<C: AmstradColor + SnapToHardware>(
    img: &RgbImage,
    max_colors: usize,
    pinned: &[C],
    prefer_neutral: bool,
    prefer_salient: bool
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
    let mut uniques: Vec<(LabF32, u32)> = counts
        .into_iter()
        .map(|(rgb, w)| (rgb8_to_lab(rgb), w))
        .collect();
    uniques.sort_by(|(a, _), (b, _)| {
        a.l.total_cmp(&b.l)
            .then(a.a.total_cmp(&b.a))
            .then(a.b.total_cmp(&b.b))
    });

    let pinned_lab: Vec<LabF32> = pinned.iter().map(|&c| rgb8_to_lab(c.color())).collect();

    let free_centroids = if uniques.len() <= k {
        // Few enough residual colors that every one gets its own cluster -
        // no need to iterate.
        uniques.iter().map(|&(lab, _)| lab).collect()
    }
    else {
        weighted_kmeans(&uniques, &pinned_lab, k, prefer_salient)
    };

    let mut claimed: HashSet<C> = pinned.iter().copied().collect();
    let resolved = snap_and_dedup::<C>(
        &free_centroids,
        &uniques,
        &pinned_lab,
        &mut claimed,
        prefer_neutral
    );

    let mut result = pinned.to_vec();
    result.extend(resolved);
    result
}

/// Nearest of `pinned_lab`/`free`, as a squared-away distance - used by both
/// the k-means loop and the dedup weighting below, which need the same
/// "what does this pixel actually get served by" notion.
fn nearest_distance(lab: LabF32, pinned_lab: &[LabF32], free: &[LabF32]) -> f32 {
    let d_pinned = pinned_lab
        .iter()
        .map(|&c| lab_distance(lab, c))
        .fold(f32::MAX, f32::min);
    let d_free = free
        .iter()
        .map(|&c| lab_distance(lab, c))
        .fold(f32::MAX, f32::min);
    d_pinned.min(d_free)
}

/// Index of the free centroid nearest `lab`, or `None` if a pinned anchor is
/// closer.
fn nearest_free(lab: LabF32, pinned_lab: &[LabF32], free: &[LabF32]) -> Option<usize> {
    let mut best_idx = None;
    let mut best_d = pinned_lab
        .iter()
        .map(|&c| lab_distance(lab, c))
        .fold(f32::MAX, f32::min);
    for (j, &c) in free.iter().enumerate() {
        let d = lab_distance(lab, c);
        if d < best_d {
            best_d = d;
            best_idx = Some(j);
        }
    }
    best_idx
}

/// How much extra weight a color gets in [`weighted_kmeans`]'s clustering
/// (not its seeding, which is already frequency-independent) in proportion
/// to its Lab distance from the image's own weighted-average tone, when
/// `prefer_salient` is set. A pixel far from the image's dominant color is
/// disproportionately noticeable to a human regardless of how little area
/// it covers - a handful of bright stars against a huge dark sky - but
/// plain frequency weighting treats it as negligible and lets it be diluted
/// away by everything competing to pull a cluster toward the majority tone.
/// Picked empirically: strong enough that a real accent color (Starry
/// Night's stars, invisible in mode 1 without this) survives to the final
/// palette, without inflating genuinely unremarkable outliers so much that
/// they crowd out the image's real structure.
const SALIENCE_BOOST: f32 = 1.0;

fn weighted_kmeans(
    uniques: &[(LabF32, u32)],
    pinned_lab: &[LabF32],
    k: usize,
    prefer_salient: bool
) -> Vec<LabF32> {
    // Frequency weight, optionally log-compressed and boosted by how much
    // of an outlier a color is relative to the image's own weighted-average
    // tone. Only used for the Lloyd-iteration mean update below -
    // farthest-point seeding already looks for outliers directly, and this
    // weighting would only ever reinforce, never override, the seeding.
    //
    // A distance-proportional boost alone is nowhere near enough on its
    // own: a real photo's frequency counts can span 3-4 orders of
    // magnitude (a few hundred star pixels against tens of thousands of
    // sky pixels), and no bounded per-color multiplier can claw back that
    // much of a raw-count disadvantage. Taking the *log* of the count
    // first compresses that dynamic range down to something the distance
    // boost can actually compete against, before the boost itself gives
    // genuine outliers the rest of the edge they need.
    let cluster_weight: Vec<f32> = if prefer_salient {
        let total_weight: f32 = uniques.iter().map(|&(_, w)| w as f32).sum();
        let (mut ml, mut ma, mut mb) = (0.0f32, 0.0f32, 0.0f32);
        for &(lab, w) in uniques {
            let wf = w as f32;
            ml += lab.l * wf;
            ma += lab.a * wf;
            mb += lab.b * wf;
        }
        let mean_lab = LabF32::new(ml / total_weight, ma / total_weight, mb / total_weight);
        uniques
            .iter()
            .map(|&(lab, w)| {
                (1.0 + w as f32).ln() * (1.0 + SALIENCE_BOOST * lab_distance(lab, mean_lab))
            })
            .collect()
    }
    else {
        uniques.iter().map(|&(_, w)| w as f32).collect()
    };

    let mut free: Vec<LabF32> = Vec::with_capacity(k);

    if pinned_lab.is_empty() {
        // Seed 0: highest-weight color (first occurrence wins ties, and
        // `uniques` is pre-sorted, so this is reproducible).
        let mut best_i = 0;
        let mut best_w = -1.0f32;
        for (i, &w) in cluster_weight.iter().enumerate() {
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
        for (i, &(lab, _)) in uniques.iter().enumerate() {
            if let Some(j) = nearest_free(lab, pinned_lab, &free) {
                let s = &mut sums[j];
                let wf = cluster_weight[i];
                s.0 += lab.l * wf;
                s.1 += lab.a * wf;
                s.2 += lab.b * wf;
                s.3 += wf;
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
    claimed: &mut HashSet<C>,
    prefer_neutral: bool
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
        let direct = C::snap_from_lab_biased(free_centroids[j], prefer_neutral);
        let candidate = if claimed.contains(&direct) {
            C::snap_excluding_from_lab_biased(free_centroids[j], claimed, prefer_neutral)
        }
        else {
            direct
        };
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
        let palette = auto_select_palette::<Ink>(&img, 4, &[], false, false);
        assert_eq!(palette.len(), 2);
        assert!(palette.contains(&Ink::BLACK));
        assert!(palette.contains(&Ink::BRIGHTWHITE));
    }

    /// A real complaint from converting an actual grayscale photo: the Gate
    /// Array has only 3 truly neutral inks (black, medium grey, white), so
    /// once those are claimed, unbiased selection for the remaining gray
    /// levels of a photo like this one falls back to whatever fully-
    /// saturated ink is nearest overall - wasting most of the budget on
    /// colors a human would never call "gray". `prefer_neutral_palette`
    /// exists specifically to fix this.
    #[test]
    fn prefer_neutral_reduces_average_chroma_for_a_grayscale_source() {
        // A dozen distinct near-neutral gray levels spanning the tonal
        // range - deliberately more shades than the Gate Array has real
        // grays for for, so budget has to be spent on approximations.
        let grays: Vec<(u8, u8, u8, u32)> = (0..12)
            .map(|i| {
                let v = (i * 23) as u8;
                (v, v, v, 10)
            })
            .collect();
        let img = image_with_colors(&grays);

        let unbiased = auto_select_palette::<Ink>(&img, 8, &[], false, false);
        let biased = auto_select_palette::<Ink>(&img, 8, &[], true, false);

        let avg_chroma = |palette: &[Ink]| -> f32 {
            let total: f32 = palette
                .iter()
                .map(|&c| super::super::lab::chroma(rgb8_to_lab(c.color())))
                .sum();
            total / palette.len() as f32
        };

        let unbiased_chroma = avg_chroma(&unbiased);
        let biased_chroma = avg_chroma(&biased);
        assert!(
            biased_chroma < unbiased_chroma,
            "prefer_neutral_palette should lower average chroma for a grayscale source: \
             unbiased={unbiased_chroma}, biased={biased_chroma}"
        );
    }

    /// The real Starry Night case, at unit-test scale: a vast dark sky, a
    /// much larger population of transitional pixels that sit closer to the
    /// accent than to the sky, and a tiny handful of pure bright-yellow
    /// "star" pixels. With a 2-color budget, plain frequency weighting lets
    /// the far more numerous transitional pixels drag the second cluster's
    /// centroid away from the rare but visually critical yellow.
    ///
    /// Goes through `weighted_kmeans` directly rather than the full
    /// `auto_select_palette` pipeline: the final hardware-snapping step
    /// rounds to one of only 27 fixed inks, which can hide a real but small
    /// shift in the underlying continuous centroid - this checks the
    /// centroid itself, where the effect is unambiguous.
    #[test]
    fn prefer_salient_pulls_the_centroid_closer_to_a_rare_but_highly_distinct_accent() {
        // (200, 180, 100) khaki sits much closer (in Lab) to the yellow
        // accent than to the dominant blue, so it gets assigned to the
        // yellow-seeded cluster during Lloyd iteration - and, being 60x
        // more numerous than the true yellow pixels, is exactly what would
        // drag that cluster's centroid away from yellow without the
        // saliency correction.
        let uniques: Vec<(LabF32, u32)> = [
            (Rgb([0u8, 0, 80]), 1000u32),
            (Rgb([200, 180, 100]), 300),
            (Rgb([255, 255, 0]), 5)
        ]
        .into_iter()
        .map(|(rgb, w)| (rgb8_to_lab(rgb), w))
        .collect();

        let unbiased = weighted_kmeans(&uniques, &[], 2, false);
        let salient = weighted_kmeans(&uniques, &[], 2, true);

        let yellow_lab = rgb8_to_lab(Rgb([255, 255, 0]));
        let closest_to_yellow = |centroids: &[LabF32]| -> f32 {
            centroids
                .iter()
                .map(|&c| lab_distance(c, yellow_lab))
                .fold(f32::MAX, f32::min)
        };

        assert!(
            closest_to_yellow(&salient) < closest_to_yellow(&unbiased),
            "prefer_salient should pull a centroid closer to the rare yellow accent: \
             unbiased dist={}, salient dist={}",
            closest_to_yellow(&unbiased),
            closest_to_yellow(&salient)
        );
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
        let palette = auto_select_palette::<Ink>(&img, 3, &[], false, false);
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
        let a = auto_select_palette::<Ink>(&img, 4, &[], false, false);
        let b = auto_select_palette::<Ink>(&img, 4, &[], false, false);
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
        let palette = auto_select_palette::<Ink>(&img, 3, &pinned, false, false);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette[0], Ink::RED);
        // The other 2 pens are free and must not duplicate the pin.
        assert!(!palette[1..].contains(&Ink::RED));
    }

    #[test]
    fn fully_pinned_budget_runs_no_clustering() {
        let img = image_with_colors(&[(0, 0, 0, 10), (255, 255, 255, 10), (255, 0, 0, 10)]);
        let pinned = [Ink::BLACK, Ink::BRIGHTWHITE];
        let palette = auto_select_palette::<Ink>(&img, 2, &pinned, false, false);
        assert_eq!(palette, vec![Ink::BLACK, Ink::BRIGHTWHITE]);
    }
}
