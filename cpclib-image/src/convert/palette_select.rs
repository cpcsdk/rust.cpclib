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

use super::lab::{lab_distance, neutral_biased_score, rgb8_to_lab, LabF32, SnapToHardware};
use crate::color::AmstradColor;

/// `--palette-algorithm`: which strategy [`auto_select_palette`] uses to
/// fill whatever pens aren't already pinned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PaletteAlgorithm {
    /// Weighted k-means clustering - see [`weighted_kmeans`]. Fast: cost
    /// scales with the number of *distinct* colors in the source, not with
    /// the size of the hardware's native color space.
    #[default]
    KMeans,
    /// Exhaustive greedy forward selection - see [`greedy_exhaustive`].
    /// Directly optimizes the true objective at every step rather than
    /// approximating it through clustering, at the cost of trying every
    /// representable hardware color at every step - cheap for the Gate
    /// Array's 27 inks, much more expensive for the Plus's 4096-color ASIC
    /// grid.
    Greedy
}

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
/// `prefer_salient` (`--prefer-salient-palette`) reserves one free centroid
/// (when at least 2 are available) directly for the single color farthest
/// from the image's own dominant tone, and otherwise boosts a color's
/// influence on clustering by that same distance - see [`SALIENCE_BOOST`].
/// Without it, plain frequency weighting can let a small but strikingly
/// different region (a handful of bright stars against a huge dark sky) get
/// diluted away entirely once the color budget is small, even though it's
/// the most visually important part of the source: a weight boost alone,
/// however large, still gets averaged down once Lloyd iteration pulls in
/// every "transitional" pixel that sits closer to the accent than to the
/// background - only a centroid that never moves, the same way a pinned
/// color never moves, guarantees the accent survives. Opt-in because it's
/// also a trade-off: for a source with no real outlier colors, it spends a
/// whole centroid on whatever mild (or spurious, like a single compression
/// artifact pixel) outlier happens to be farthest from the mean, and at a
/// very small color budget that can cost more in lost background fidelity
/// than the accent is worth.
pub fn auto_select_palette<C: AmstradColor + SnapToHardware>(
    img: &RgbImage,
    max_colors: usize,
    pinned: &[C],
    prefer_neutral: bool,
    prefer_salient: bool,
    algorithm: PaletteAlgorithm
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

    let resolved: Vec<C> = if uniques.len() <= k {
        // Few enough residual colors that every one gets its own cluster -
        // no need to iterate, and no need to pick an algorithm either.
        let free_centroids: Vec<LabF32> = uniques.iter().map(|&(lab, _)| lab).collect();
        let mut claimed: HashSet<C> = pinned.iter().copied().collect();
        snap_and_dedup::<C>(
            &free_centroids,
            &uniques,
            &pinned_lab,
            &mut claimed,
            prefer_neutral
        )
    }
    else {
        match algorithm {
            PaletteAlgorithm::KMeans => {
                let free_centroids = weighted_kmeans::<C>(
                    &uniques,
                    pinned,
                    &pinned_lab,
                    k,
                    prefer_salient,
                    prefer_neutral
                );
                let mut claimed: HashSet<C> = pinned.iter().copied().collect();
                snap_and_dedup::<C>(
                    &free_centroids,
                    &uniques,
                    &pinned_lab,
                    &mut claimed,
                    prefer_neutral
                )
            },
            PaletteAlgorithm::Greedy => {
                greedy_exhaustive::<C>(
                    &uniques,
                    pinned,
                    &pinned_lab,
                    k,
                    prefer_salient,
                    prefer_neutral
                )
            },
        }
    };

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

/// Weighted average Lab position across every distinct color, used as the
/// image's own "dominant tone" reference point for [`prefer_salient`]'s
/// outlier reservation and cluster-weight boost.
fn weighted_mean_lab(uniques: &[(LabF32, u32)]) -> LabF32 {
    let total_weight: f32 = uniques.iter().map(|&(_, w)| w as f32).sum();
    let (mut ml, mut ma, mut mb) = (0.0f32, 0.0f32, 0.0f32);
    for &(lab, w) in uniques {
        let wf = w as f32;
        ml += lab.l * wf;
        ma += lab.a * wf;
        mb += lab.b * wf;
    }
    LabF32::new(ml / total_weight, ma / total_weight, mb / total_weight)
}

/// Frequency weight for each of `uniques`, optionally log-compressed and
/// boosted by how much of an outlier a color is relative to the image's own
/// weighted-average tone (`prefer_salient`). Shared between
/// [`weighted_kmeans`] (where it drives the Lloyd-iteration mean update -
/// farthest-point seeding already looks for outliers directly, so this
/// weighting only ever reinforces, never overrides, the seeding) and
/// [`greedy_exhaustive`] (where it directly weights each color's
/// contribution to the total error being minimized).
///
/// A distance-proportional boost alone is nowhere near enough on its own: a
/// real photo's frequency counts can span 3-4 orders of magnitude (a few
/// hundred star pixels against tens of thousands of sky pixels), and no
/// bounded per-color multiplier can claw back that much of a raw-count
/// disadvantage. Taking the *log* of the count first compresses that
/// dynamic range down to something the distance boost can actually compete
/// against, before the boost itself gives genuine outliers the rest of the
/// edge they need.
fn salience_weights(uniques: &[(LabF32, u32)], prefer_salient: bool) -> Vec<f32> {
    let mean_lab = (prefer_salient && !uniques.is_empty()).then(|| weighted_mean_lab(uniques));
    match mean_lab {
        Some(mean_lab) => {
            uniques
                .iter()
                .map(|&(lab, w)| {
                    (1.0 + w as f32).ln() * (1.0 + SALIENCE_BOOST * lab_distance(lab, mean_lab))
                })
                .collect()
        },
        None => uniques.iter().map(|&(_, w)| w as f32).collect()
    }
}

fn weighted_kmeans<C: AmstradColor + SnapToHardware>(
    uniques: &[(LabF32, u32)],
    pinned: &[C],
    pinned_lab: &[LabF32],
    k: usize,
    prefer_salient: bool,
    prefer_neutral: bool
) -> Vec<LabF32> {
    let cluster_weight = salience_weights(uniques, prefer_salient);
    let mean_lab = (prefer_salient && !uniques.is_empty()).then(|| weighted_mean_lab(uniques));

    // Reserve one free slot directly for the single color farthest (in
    // Lab, ignoring pixel count entirely) from the image's own dominant
    // tone - see this function's doc comment on why boosting alone isn't
    // enough. Needs at least 2 free slots: reserving the only one would
    // throw away the background entirely rather than fit it approximately.
    let reserved_lab = match mean_lab {
        Some(mean_lab) if k >= 2 => {
            uniques
                .iter()
                .map(|&(lab, _)| lab)
                .max_by(|&a, &b| lab_distance(a, mean_lab).total_cmp(&lab_distance(b, mean_lab)))
        },
        _ => None
    };
    let k = if reserved_lab.is_some() { k - 1 } else { k };
    let anchors: Vec<LabF32> = match reserved_lab {
        Some(r) => {
            pinned_lab
                .iter()
                .copied()
                .chain(std::iter::once(r))
                .collect()
        },
        None => pinned_lab.to_vec()
    };
    let pinned_lab = anchors.as_slice();

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

    // Each cluster's ideal (pre-snap) target given the current `free`
    // positions: the weighted mean of whatever pixels are currently closest
    // to it, or - if it ended up empty - the unique color currently
    // farthest from whichever centroid (pinned or another free one) it's
    // actually closest to. Also returns each cluster's accumulated weight,
    // used to order the discrete-polish phase's snapping below.
    let compute_ideal = |free: &[LabF32]| -> (Vec<LabF32>, Vec<f32>) {
        let mut sums = vec![(0.0f32, 0.0f32, 0.0f32, 0.0f32); k]; // l, a, b, weight
        for (i, &(lab, _)) in uniques.iter().enumerate() {
            if let Some(j) = nearest_free(lab, pinned_lab, free) {
                let s = &mut sums[j];
                let wf = cluster_weight[i];
                s.0 += lab.l * wf;
                s.1 += lab.a * wf;
                s.2 += lab.b * wf;
                s.3 += wf;
            }
        }
        let mut ideal: Vec<LabF32> = Vec::with_capacity(k);
        for (j, &s) in sums.iter().enumerate().take(k) {
            if s.3 > 0.0 {
                ideal.push(LabF32::new(s.0 / s.3, s.1 / s.3, s.2 / s.3));
            }
            else {
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
                ideal.push(uniques[worst_i].0);
            }
        }
        (ideal, sums.into_iter().map(|s| s.3).collect())
    };

    // Phase 1: plain continuous Lloyd's algorithm, exactly as if hardware
    // didn't exist - this is what actually gives k-means its guarantee of
    // monotonically decreasing total distance each round. Snapping to a
    // real ink is a discretization step, and discretizing on every round
    // (tried first, measured against real test images) trapped the result
    // in a worse local optimum than plain continuous Lloyd's would have
    // reached, on any source without one clearly dominant tone (a
    // synthetic full-hue color wheel got *worse*, not better, from
    // snapping too early) - so convergence happens in continuous space
    // first, and only the already-converged result gets discretized below.
    for _ in 0..32 {
        let (ideal, _) = compute_ideal(&free);
        let mut moved = false;
        for j in 0..k {
            if lab_distance(ideal[j], free[j]) > 1e-3 {
                moved = true;
            }
            free[j] = ideal[j];
        }
        if !moved {
            break;
        }
    }

    // Phase 2: short discrete-polish pass. The continuous optimum found
    // above may not be reachable exactly, so re-run a few more rounds that
    // reassign pixels against the *actual* nearest achievable hardware
    // color each round, heaviest cluster snapping first so a lighter one
    // yields its ideal target if two collide. This is what recovers real
    // photos' small real accents (a rare pure color a continuous mean
    // alone would average away) without the phase above ever losing its
    // proper convergence to premature discretization.
    for _ in 0..8 {
        let (ideal, weights) = compute_ideal(&free);
        let mut order: Vec<usize> = (0..k).collect();
        order.sort_by(|&a, &b| weights[b].total_cmp(&weights[a]));
        let mut claimed: HashSet<C> = pinned.iter().copied().collect();
        let mut snapped = vec![LabF32::new(0.0, 0.0, 0.0); k];
        for j in order {
            let color = C::snap_excluding_from_lab_biased(ideal[j], &claimed, prefer_neutral);
            claimed.insert(color);
            snapped[j] = rgb8_to_lab(color.color());
        }

        let mut moved = false;
        for j in 0..k {
            if lab_distance(snapped[j], free[j]) > 1e-3 {
                moved = true;
            }
            free[j] = snapped[j];
        }
        if !moved {
            break;
        }
    }

    if let Some(r) = reserved_lab {
        free.push(r);
    }
    free
}

/// Exhaustive greedy forward selection, alternative to [`weighted_kmeans`]:
/// for each remaining free slot in turn, tries *every* representable
/// hardware color ([`SnapToHardware::all_candidates`]) as the candidate to
/// add next, and keeps whichever one minimizes the total weighted
/// remaining error - the sum, over every distinct color in the image, of
/// the smaller of its current best distance to an already-chosen color and
/// its distance to the candidate. Once a candidate is chosen it's fixed
/// and never revisited, so this is greedy, not globally optimal - but each
/// individual choice is the true best possible addition given everything
/// decided so far, not an approximation of it.
///
/// This directly optimizes the real objective at every step rather than
/// approximating it through clustering, so on a *moderate* frequency
/// imbalance it tends to keep a rare accent color without any special
/// mechanism at all: a pixel that's currently badly served contributes to
/// the "current best distance" sum regardless of how few pixels share its
/// color, so a candidate near it can still win a slot on merit. But that
/// sum is still a sum over every pixel, so on the kind of extreme
/// imbalance a real photo can have (a handful of pixels against tens of
/// thousands), the same ceiling [`weighted_kmeans`] hits applies here too -
/// no finite per-color weight multiplier changes what a *sum* favors when
/// one side so vastly outnumbers the other. So `prefer_salient` reserves a
/// slot here exactly the way it does for `weighted_kmeans` - the single
/// color farthest from the image's mean tone, fixed before the remaining
/// slots are chosen - rather than relying on [`salience_weights`]'s
/// weighting alone to be enough.
///
/// Adapted from the "brute force best palette search" real Amiga/Atari
/// image converters use (see `arnaud-carre/abc` on GitHub) - those target
/// a 4096-color native gamut and need a GPU compute shader or many CPU
/// threads to search it every step; trying every one of the Gate Array's
/// 27 real inks at every step is cheap enough to do on a single thread,
/// which is why this stayed a separate opt-in engine rather than replacing
/// `weighted_kmeans` outright: it costs much more for the Plus's
/// 4096-entry native ASIC grid.
fn greedy_exhaustive<C: AmstradColor + SnapToHardware>(
    uniques: &[(LabF32, u32)],
    pinned: &[C],
    pinned_lab: &[LabF32],
    k: usize,
    prefer_salient: bool,
    prefer_neutral: bool
) -> Vec<C> {
    let weight = salience_weights(uniques, prefer_salient);
    let score = |a: LabF32, b: LabF32| -> f32 {
        if prefer_neutral {
            neutral_biased_score(a, b)
        }
        else {
            lab_distance(a, b)
        }
    };

    // Each unique color's current best (weighted) error given only
    // whatever's already chosen (the pinned colors, initially).
    let mut best_err: Vec<f32> = uniques
        .iter()
        .map(|&(lab, _)| {
            pinned_lab
                .iter()
                .map(|&p| score(lab, p))
                .fold(f32::MAX, f32::min)
        })
        .collect();

    let mut claimed: HashSet<C> = pinned.iter().copied().collect();
    let mut resolved: Vec<C> = Vec::with_capacity(k);

    // Reserve one slot outright for the single color farthest from the
    // image's own mean tone - see this function's doc comment on why
    // weighting the objective alone isn't always enough. Snapped and fixed
    // immediately, exactly like an extra pinned color, before the
    // remaining slots run through the ordinary greedy loop below.
    let remaining_k = if prefer_salient && k >= 2 && !uniques.is_empty() {
        let mean_lab = weighted_mean_lab(uniques);
        let outlier_lab = uniques
            .iter()
            .map(|&(lab, _)| lab)
            .max_by(|&a, &b| lab_distance(a, mean_lab).total_cmp(&lab_distance(b, mean_lab)))
            .unwrap();
        let reserved = C::snap_excluding_from_lab_biased(outlier_lab, &claimed, prefer_neutral);
        claimed.insert(reserved);
        resolved.push(reserved);
        let reserved_lab = rgb8_to_lab(reserved.color());
        for (i, &(lab, _)) in uniques.iter().enumerate() {
            best_err[i] = best_err[i].min(score(lab, reserved_lab));
        }
        k - 1
    }
    else {
        k
    };

    for _ in 0..remaining_k {
        let mut best_candidate = None;
        let mut best_total = f32::MAX;
        for candidate in C::all_candidates() {
            if claimed.contains(&candidate) {
                continue;
            }
            let candidate_lab = rgb8_to_lab(candidate.color());
            let mut total = 0.0f32;
            for (i, &(lab, _)) in uniques.iter().enumerate() {
                total += score(lab, candidate_lab).min(best_err[i]) * weight[i];
            }
            if total < best_total {
                best_total = total;
                best_candidate = Some(candidate);
            }
        }
        let chosen = best_candidate
            .expect("all_candidates() has far more entries than any mode's color budget");
        claimed.insert(chosen);
        resolved.push(chosen);

        let chosen_lab = rgb8_to_lab(chosen.color());
        for (i, &(lab, _)) in uniques.iter().enumerate() {
            best_err[i] = best_err[i].min(score(lab, chosen_lab));
        }
    }

    resolved
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
        let palette =
            auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::KMeans);
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

        let unbiased =
            auto_select_palette::<Ink>(&img, 8, &[], false, false, PaletteAlgorithm::KMeans);
        let biased =
            auto_select_palette::<Ink>(&img, 8, &[], true, false, PaletteAlgorithm::KMeans);

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

        let unbiased = weighted_kmeans::<Ink>(&uniques, &[], &[], 2, false, false);
        let salient = weighted_kmeans::<Ink>(&uniques, &[], &[], 2, true, false);

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
        // With 2 free slots, one is reserved outright for the farthest
        // outlier - the true yellow pixel itself, here - so it should be
        // recovered exactly, not just "closer than before".
        assert!(
            closest_to_yellow(&salient) < 1e-3,
            "prefer_salient with >=2 free slots should recover the rare accent's exact color: \
             dist={}",
            closest_to_yellow(&salient)
        );
    }

    /// End-to-end (through `auto_select_palette`, including the final
    /// hardware snap) version of the reservation test above, at a realistic
    /// mode 1 budget (4 colors): a dominant dark-blue sky, a large
    /// population of transitional greenish pixels, a mid-tone ground, and a
    /// tiny handful of pure-yellow "star" pixels. Before reservation, this
    /// was exactly the shape of case that `SALIENCE_BOOST` alone could not
    /// fix even at 20x its final value - the transitional pixels always
    /// outweighed the boost. With reservation, the accent must survive all
    /// the way through hardware snapping.
    #[test]
    fn prefer_salient_recovers_the_accent_through_hardware_snapping() {
        let img = image_with_colors(&[
            (0, 0, 80, 2000),
            (100, 140, 90, 600),
            (90, 70, 40, 400),
            (255, 255, 0, 6)
        ]);
        let palette =
            auto_select_palette::<Ink>(&img, 4, &[], false, true, PaletteAlgorithm::KMeans);
        let yellow_lab = rgb8_to_lab(image::Rgb([255, 255, 0]));
        let closest = palette
            .iter()
            .map(|&c| lab_distance(rgb8_to_lab(c.color()), yellow_lab))
            .fold(f32::MAX, f32::min);
        assert!(
            closest < 20.0,
            "prefer_salient should keep a hardware ink close to the rare yellow accent \
             even after snapping: closest CIEDE2000 distance={closest}, palette={palette:?}"
        );
    }

    /// The `Greedy` engine's whole premise: it should recover the same rare
    /// accent `KMeans` needs `prefer_salient`'s reservation mechanism for -
    /// but as a natural side effect of directly minimizing total error, not
    /// through any special-cased mechanism. `prefer_salient: false` here on
    /// purpose, to prove the engine itself is what does it.
    #[test]
    fn greedy_recovers_the_accent_without_needing_prefer_salient() {
        let img = image_with_colors(&[
            (0, 0, 80, 2000),
            (100, 140, 90, 600),
            (90, 70, 40, 400),
            (255, 255, 0, 6)
        ]);
        let palette =
            auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::Greedy);
        let yellow_lab = rgb8_to_lab(image::Rgb([255, 255, 0]));
        let closest = palette
            .iter()
            .map(|&c| lab_distance(rgb8_to_lab(c.color()), yellow_lab))
            .fold(f32::MAX, f32::min);
        assert!(
            closest < 20.0,
            "greedy exhaustive selection should keep a hardware ink close to the rare \
             yellow accent without needing prefer_salient at all: closest CIEDE2000 \
             distance={closest}, palette={palette:?}"
        );
    }

    /// A real conversion of Van Gogh's Starry Night in mode 1 (blue sky
    /// dominates almost every pixel, a moon and a few stars are a literal
    /// handful) found that plain greedy exhaustive selection chose a
    /// *second blue shade* over the accent: a real photo's sky isn't one
    /// exact repeated color, it's thousands of slightly different blue
    /// shades, so even after the first blue ink is picked there's real
    /// residual error a second blue can still reduce - and summed over
    /// tens of thousands of pixels, that outweighs perfectly fixing a
    /// handful of pixels no matter how large their individual error is.
    /// Reproducing that needs the same shape here: several *distinct*
    /// near-blue shades (not one flat repeated color, which a single ink
    /// already fits with zero residual error and so never creates a
    /// competing use for the last slot). `prefer_salient`'s reservation is
    /// what actually fixes this, exactly as it does for `weighted_kmeans`.
    #[test]
    fn greedy_prefer_salient_recovers_the_accent_under_an_extreme_imbalance() {
        let img = image_with_colors(&[
            (0, 0, 80, 4000),
            (0, 10, 85, 4000),
            (10, 0, 75, 4000),
            (5, 15, 80, 4000),
            (0, 5, 90, 4000),
            (100, 140, 90, 2000),
            (90, 70, 40, 1000),
            (255, 255, 0, 6)
        ]);
        let without_salient =
            auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::Greedy);
        let with_salient =
            auto_select_palette::<Ink>(&img, 4, &[], false, true, PaletteAlgorithm::Greedy);
        let yellow_lab = rgb8_to_lab(image::Rgb([255, 255, 0]));
        let closest_to_yellow = |palette: &[Ink]| -> f32 {
            palette
                .iter()
                .map(|&c| lab_distance(rgb8_to_lab(c.color()), yellow_lab))
                .fold(f32::MAX, f32::min)
        };
        assert!(
            closest_to_yellow(&without_salient) > 20.0,
            "this test should exercise the case plain greedy actually fails at - it should \
             NOT already recover the accent unaided: palette={without_salient:?}"
        );
        assert!(
            closest_to_yellow(&with_salient) < 20.0,
            "prefer_salient's reservation should still recover the accent under an extreme \
             imbalance where plain greedy selection does not: closest CIEDE2000 \
             distance={}, palette={with_salient:?}",
            closest_to_yellow(&with_salient)
        );
    }

    #[test]
    fn greedy_over_budget_yields_exactly_max_colors_distinct() {
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
        let palette =
            auto_select_palette::<Ink>(&img, 3, &[], false, false, PaletteAlgorithm::Greedy);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette.iter().collect::<HashSet<_>>().len(), 3);
    }

    #[test]
    fn greedy_pinned_colors_survive_unchanged_and_are_not_resnapped() {
        let img = image_with_colors(&[
            (0, 0, 255, 20),
            (0, 255, 0, 20),
            (10, 10, 200, 5),
            (10, 200, 10, 5)
        ]);
        let pinned = [Ink::RED];
        let palette =
            auto_select_palette::<Ink>(&img, 3, &pinned, false, false, PaletteAlgorithm::Greedy);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette[0], Ink::RED);
        assert!(!palette[1..].contains(&Ink::RED));
    }

    #[test]
    fn greedy_selection_is_deterministic() {
        let img = image_with_colors(&[
            (0, 0, 0, 20),
            (255, 255, 255, 20),
            (255, 0, 0, 10),
            (0, 255, 0, 10),
            (0, 0, 255, 10),
            (128, 128, 0, 5),
            (30, 60, 90, 7)
        ]);
        let a = auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::Greedy);
        let b = auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::Greedy);
        assert_eq!(a, b);
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
        let palette =
            auto_select_palette::<Ink>(&img, 3, &[], false, false, PaletteAlgorithm::KMeans);
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
        let a = auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::KMeans);
        let b = auto_select_palette::<Ink>(&img, 4, &[], false, false, PaletteAlgorithm::KMeans);
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
        let palette =
            auto_select_palette::<Ink>(&img, 3, &pinned, false, false, PaletteAlgorithm::KMeans);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette[0], Ink::RED);
        // The other 2 pens are free and must not duplicate the pin.
        assert!(!palette[1..].contains(&Ink::RED));
    }

    #[test]
    fn fully_pinned_budget_runs_no_clustering() {
        let img = image_with_colors(&[(0, 0, 0, 10), (255, 255, 255, 10), (255, 0, 0, 10)]);
        let pinned = [Ink::BLACK, Ink::BRIGHTWHITE];
        let palette =
            auto_select_palette::<Ink>(&img, 2, &pinned, false, false, PaletteAlgorithm::KMeans);
        assert_eq!(palette, vec![Ink::BLACK, Ink::BRIGHTWHITE]);
    }
}
