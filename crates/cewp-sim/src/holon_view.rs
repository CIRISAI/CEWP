//! Flat-buffer views of the holon demonstration for the globe (HOOKS.md §2), in the layout
//! `twin-globe` asked for: plain functions returning `Vec<f32>` / `Vec<u32>`, deterministic per
//! seed, strides and fields documented here, a class per field
//! (0 MEASURED, 1 DERIVED, 2 MODEL, 3 PROJECTED, 4 FRONTIER).
//!
//! The scene: the coarse cohort tier (R4c), certified on the Expected regime, is handed one live
//! coarse state from a run where access network 0 is down. The refinement trigger decides, per
//! coarse cell, whether that state is inside the certificate's envelope; a refused cell is sent
//! back to the finer (R5) tier. Every number is a twin output (DERIVED from the R5 micro tier's
//! own events, under MODEL parameters), never a production measurement.
//!
//! **Cells are not places.** A coarse cell is (host kind × access network); its devices are spread
//! over the whole generated patch. The patch is one square around a public city centre (coarse
//! grain; every street, household and device in it is generated). The page's 1.5° population grid
//! and `tiling.rs` `HolonId` cells are far larger than the patch (at most 1.3 km across), so the cells
//! do not align with them; instead the view lays the cells out as a grid of sub-squares INSIDE the
//! patch square (row = host kind, column = access network, last column "no access network"). That
//! layout is a drawing choice (class MODEL), not a location.

use crate::holon_demo::{demonstration_seeded, DemoReport, ACCESS_NETWORKS, DEMO_LAT, DEMO_LON, DEMO_SEED};
use crate::holon_dist::{EnvelopeBound, RefinementVerdict, FP_ONE, TAIL_LEVELS};

/// Horizon of the view scenario: 2 days per regime (5,760 cadences), the shortest at which p99
/// holds ≥ 30 order statistics beyond it (`holon_demo::DEMO_MIN_TAIL_SAMPLES`), so p99 is judged
/// and only p99.9 shows as unresolved. MODEL choice, for drawing only; the regression test runs 4
/// days.
pub const VIEW_HORIZON_S: f64 = 2.0 * 86_400.0;

/// The seed whose numbers FSD/twin/holon.md §6 reports (other seeds draw other generated patches;
/// nothing is asserted on them).
pub const VIEW_DEFAULT_SEED: u64 = DEMO_SEED;

/// Bound index → name, the `binding bound index` of `holon_view_cells`:
///
/// | idx | name | what broke |
/// |---|---|---|
/// | 0 | population | devices in the tier outside the certified range |
/// | 1 | workload | rows per device-hour outside the certified range |
/// | 2 | topology | another density class / seat count / homing k |
/// | 3 | failure regime | a regime the certificate was not measured in |
/// | 4 | horizon | longer than the certified horizon |
/// | 5 | cell state | a cell's live occupancy outside the range seen at certification |
/// | 6 | unknown cell | a live cell the certificate never saw |
/// | 7 | observable not certified | the observable is not listed |
/// | 8 | W1 budget | the observable's W1 broke its budget when measured |
/// | 9 | quantile budget | a resolved quantile broke its budget when measured |
pub const BOUND_NAMES: [&str; 10] = [
    "population",
    "workload",
    "topology",
    "failure regime",
    "horizon",
    "cell state",
    "unknown cell",
    "observable not certified",
    "W1 budget",
    "quantile budget",
];

/// Bound index of an envelope bound (row of `BOUND_NAMES`).
#[must_use]
pub fn bound_code(b: &EnvelopeBound) -> u32 {
    match b {
        EnvelopeBound::Population { .. } => 0,
        EnvelopeBound::Workload { .. } => 1,
        EnvelopeBound::Topology { .. } => 2,
        EnvelopeBound::FailureRegime { .. } => 3,
        EnvelopeBound::Horizon { .. } => 4,
        EnvelopeBound::Cell { .. } => 5,
        EnvelopeBound::UnknownCell { .. } => 6,
        EnvelopeBound::ObservableNotCertified { .. } => 7,
        EnvelopeBound::W1 { .. } => 8,
        EnvelopeBound::Quantile { .. } => 9,
    }
}

/// The bound table as `[idx, name_offset, name_len]` triples into the UTF-8 bytes of
/// `BOUND_NAMES.concat()` (for a caller that wants it as numbers; the doc table above is the same).
#[must_use]
pub fn holon_view_bounds() -> Vec<u32> {
    let mut out = Vec::with_capacity(3 * BOUND_NAMES.len());
    let mut off = 0u32;
    for (i, n) in BOUND_NAMES.iter().enumerate() {
        out.extend_from_slice(&[i as u32, off, n.len() as u32]);
        off += n.len() as u32;
    }
    out
}

/// Stride of `holon_view_cells`.
pub const CELL_STRIDE: usize = 8;

/// Class of each field of a `holon_view_cells` record (0 MEASURED .. 4 FRONTIER).
pub const CELL_CLASS: [u32; CELL_STRIDE] = [2, 2, 2, 1, 1, 1, 1, 1];

/// Per-cell certificate state, stride `CELL_STRIDE` = 8, one record per row:
///
/// | field | meaning | class |
/// |---|---|---|
/// | 0 `lat` | centre latitude, degrees | MODEL (layout) |
/// | 1 `lon` | centre longitude, degrees | MODEL (layout) |
/// | 2 `half` | half-size, degrees of latitude (the square's angular half-side; longitude half-extent is `half / cos(lat)`) | MODEL (layout) |
/// | 3 `level` | 0 = the coarse tier as a whole (the patch square, record 0); 1 = one coarse cell | DERIVED |
/// | 4 `state` | 0 certified, inside the envelope; 1 refused (sent back to the finer tier); 2 not evaluated (no live state) | DERIVED |
/// | 5 `bound` | bound index (`BOUND_NAMES`) that refused, or for an inside cell the bound that binds; −1 none | DERIVED |
/// | 6 `margin` | signed distance to that bound, in online share of the cell (0..1): positive inside, 0 at the bound, negative outside. Record 0: the smallest over the cells. NaN if the bound has no distance (whole-tier bound, unknown cell) | DERIVED |
/// | 7 `defect` | record 0: the coarse tier's measured worst normalised distributional defect in the outage regime, max over observables of W1/budget and resolved quantile defect/budget (> 1 = over budget); cells: NaN (the certificate's defects are per tier, not per cell) | DERIVED |
///
/// Record 0 is the patch; records 1.. are the cells in `DemoReport::cells` order.
/// Deterministic for a given `seed`; `VIEW_DEFAULT_SEED` is the one the spec reports. Runs the
/// R5 tier for `VIEW_HORIZON_S` twice (normal and outage): call once, not per frame.
#[must_use]
pub fn holon_view_cells(seed: u64) -> Vec<f32> {
    cells_of(&demonstration_seeded(VIEW_HORIZON_S, seed))
}

/// `holon_view_cells` for an already-run demonstration.
#[must_use]
pub fn cells_of(r: &DemoReport) -> Vec<f32> {
    let deg_per_m = 1.0 / 111_320.0;
    let patch_half = r.patch_half_m * deg_per_m;
    let coslat = DEMO_LAT.to_radians().cos();
    // grid: rows = host kinds present (in cell order), cols = networks + one "none" column
    let mut kinds: Vec<_> = r.cells.iter().map(|c| c.kind).collect();
    kinds.dedup();
    let none_col = r.cells.iter().any(|c| c.net.is_none());
    let cols = usize::from(ACCESS_NETWORKS) + usize::from(none_col);
    let g = kinds.len().max(cols).max(1) as f64;
    let slot = patch_half / g;

    let verdict = r.trigger.as_ref();
    let (broken, refine): (&[EnvelopeBound], &[_]) = match verdict {
        None | Some(RefinementVerdict::Inside) => (&[], &[]),
        Some(RefinementVerdict::Refine { broken, refine_cells }) => (broken, refine_cells),
    };
    let global = broken.iter().find(|b| b.is_global());

    let mut out = Vec::with_capacity(CELL_STRIDE * (1 + r.cells.len()));
    let mut cell_recs = Vec::with_capacity(CELL_STRIDE * r.cells.len());
    let mut min_margin = f32::NAN;
    for c in &r.cells {
        let id = c.id();
        let row = kinds.iter().position(|&k| k == c.kind).unwrap_or(0) as f64;
        let col = c.net.map_or(cols - 1, usize::from) as f64;
        let lat = DEMO_LAT + patch_half - (2.0 * row + 1.0) * slot;
        let lon = DEMO_LON + (-patch_half + (2.0 * col + 1.0) * slot) / coslat;
        let live = r.live.as_ref().and_then(|l| l.cells.iter().find(|(k, _)| *k == id)).map(|(_, v)| *v as f64 / FP_ONE as f64);
        let range = r.expected.envelope.cells.iter().find(|e| e.cell == id).map(|e| (e.occupancy_fp.lo as f64 / FP_ONE as f64, e.occupancy_fp.hi as f64 / FP_ONE as f64));
        let margin = match (live, range) {
            (Some(v), Some((lo, hi))) => (v - lo).min(hi - v) as f32,
            _ => f32::NAN,
        };
        let own = broken.iter().find(|b| matches!(b, EnvelopeBound::Cell { cell, .. } | EnvelopeBound::UnknownCell { cell } if *cell == id));
        let (state, bound, margin) = if verdict.is_none() || live.is_none() {
            (2.0, -1.0, f32::NAN)
        } else if let Some(g) = global {
            (1.0, bound_code(g) as f32, f32::NAN)
        } else if let Some(b) = own {
            debug_assert!(refine.contains(&id));
            (1.0, bound_code(b) as f32, margin)
        } else if range.is_some() {
            (0.0, 5.0, margin)
        } else {
            (0.0, -1.0, f32::NAN)
        };
        if !margin.is_nan() {
            min_margin = if min_margin.is_nan() { margin } else { min_margin.min(margin) };
        }
        cell_recs.extend_from_slice(&[lat as f32, lon as f32, slot as f32, 1.0, state, bound, margin, f32::NAN]);
    }
    let worst = r
        .outage_mean_field
        .observables
        .iter()
        .flat_map(|o| {
            let w1 = o.w1.as_f64() / (o.spec.w1_budget_fp as f64 / FP_ONE as f64);
            std::iter::once(w1).chain(o.quantiles.iter().filter(|q| q.resolved).map(|q| q.defect_fp as f64 / q.budget_fp.max(1) as f64))
        })
        .fold(f64::NAN, f64::max);
    let (pstate, pbound) = match verdict {
        None => (2.0, -1.0),
        Some(RefinementVerdict::Inside) => (0.0, if min_margin.is_nan() { -1.0 } else { 5.0 }),
        Some(RefinementVerdict::Refine { broken, .. }) => (1.0, global.or(broken.first()).map_or(-1.0, |b| bound_code(b) as f32)),
    };
    let pmargin = if global.is_some() { f32::NAN } else { min_margin };
    out.extend_from_slice(&[DEMO_LAT as f32, DEMO_LON as f32, patch_half as f32, 0.0, pstate, pbound, pmargin, worst as f32]);
    out.extend_from_slice(&cell_recs);
    out
}

/// Stride of `holon_view_certificates` (u32 part).
pub const CERT_U32: usize = 5 + TAIL_LEVELS.len();

/// Stride of `holon_view_certificates` (f32 part).
pub const CERT_F32: usize = 4 + 4 * TAIL_LEVELS.len();

/// The certificate table of the three tiers the demonstration measured (all fields DERIVED).
/// One row per (tier, observable).
///
/// u32, stride `CERT_U32`: `[tier, observable, distributional_certified, mean_only_certified,
/// w1_within, level_state × 4]`. `tier`: 0 Expected regime / mean-field cohort tier, 1 network-0
/// outage / mean-field, 2 network-0 outage / gated (refined) tier. `observable`: 0
/// `seat_round_arrivals`, 1 `devices_off_seat`. Booleans 0/1. `level_state` for p50, p95, p99,
/// p99.9: 0 within budget, 1 breach, 2 unresolved (too few order statistics beyond the level; not
/// certified, not refused).
///
/// f32, stride `CERT_F32`: `[w1, w1_budget, mean_fine, mean_coarse, (fine, coarse, defect,
/// budget) × 4]`, devices per 30 s cadence.
#[must_use]
pub fn certificates_of(r: &DemoReport) -> (Vec<u32>, Vec<f32>) {
    let (mut u, mut f) = (Vec::new(), Vec::new());
    let x = |v: i64| v as f32 / FP_ONE as f32;
    for (t, c) in [&r.expected, &r.outage_mean_field, &r.outage_gated].into_iter().enumerate() {
        for (k, o) in c.observables.iter().enumerate() {
            u.extend_from_slice(&[t as u32, k as u32, u32::from(o.within()), u32::from(o.mean.certified), u32::from(o.w1_within())]);
            f.extend_from_slice(&[o.w1.as_f64() as f32, x(o.spec.w1_budget_fp), x(o.mean_fine_fp), x(o.mean_coarse_fp)]);
            for q in &o.quantiles {
                u.push(if !q.resolved {
                    2
                } else if q.within() {
                    0
                } else {
                    1
                });
                f.extend_from_slice(&[x(q.fine_fp), x(q.coarse_fp), x(q.defect_fp), x(q.budget_fp)]);
            }
        }
    }
    (u, f)
}

/// `certificates_of` the seeded view scenario.
#[must_use]
pub fn holon_view_certificates(seed: u64) -> (Vec<u32>, Vec<f32>) {
    certificates_of(&demonstration_seeded(VIEW_HORIZON_S, seed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holon_demo::CellObs;
    use crate::micro::HostKind;

    #[test]
    fn view_buffers_have_the_documented_layout_and_show_the_outage() {
        let r = demonstration_seeded(VIEW_HORIZON_S, VIEW_DEFAULT_SEED);
        let v = cells_of(&r);
        assert_eq!(v.len(), CELL_STRIDE * (1 + r.cells.len()));
        let rec: Vec<&[f32]> = v.chunks(CELL_STRIDE).collect();
        // record 0: the patch, refused on a cell bound, its measured outage defect over budget
        let p = rec[0];
        assert_eq!((p[3], p[4], p[5]), (0.0, 1.0, 5.0), "{p:?}");
        assert!(p[6] < 0.0 && p[7] > 1.0, "{p:?}");
        let cells: Vec<(&CellObs, &[f32])> = r.cells.iter().zip(&rec[1..]).map(|(c, x)| (c, *x)).collect();
        for (c, x) in &cells {
            assert_eq!(x[3], 1.0);
            // refused exactly on network 0, on the cell bound, outside it
            if x[4] == 1.0 {
                assert_eq!(c.net, Some(0), "{c:?} {x:?}");
                assert_eq!(x[5], 5.0);
                assert!(x[6] < 0.0, "{c:?} {x:?}");
            } else {
                assert_eq!(x[4], 0.0, "{c:?} {x:?}");
                assert!(x[6].is_nan() || x[6] >= 0.0, "{c:?} {x:?}");
            }
            // inside the patch square
            assert!((x[0] - p[0]).abs() <= p[2] && x[2] > 0.0 && x[2] <= p[2]);
        }
        assert!(cells.iter().any(|(c, x)| c.kind == HostKind::HomeNode && c.net == Some(0) && x[4] == 1.0));
        // the bound table names every code
        let b = holon_view_bounds();
        let names = BOUND_NAMES.concat();
        assert_eq!(&names[b[3 * 5 + 1] as usize..(b[3 * 5 + 1] + b[3 * 5 + 2]) as usize], "cell state");
        // deterministic
        assert_eq!(v.iter().map(|x| x.to_bits()).collect::<Vec<_>>(), cells_of(&r).iter().map(|x| x.to_bits()).collect::<Vec<_>>());

        let (cu, cf) = certificates_of(&r);
        let rows = cu.len() / CERT_U32;
        assert_eq!((rows, cf.len()), (6, rows * CERT_F32));
        let row = |t: u32, k: u32| cu.chunks(CERT_U32).find(|c| c[0] == t && c[1] == k).unwrap();
        // outage / mean-field on devices_off_seat: mean-only passes, distributional refuses
        assert_eq!(&row(1, 1)[2..4], &[0, 1]);
        // p99.9 is unresolved at this horizon in every row
        assert!(cu.chunks(CERT_U32).all(|c| c[5 + 3] == 2), "{cu:?}");
    }
}
