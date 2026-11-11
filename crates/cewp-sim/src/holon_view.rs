//! Flat-buffer views of the holon demonstration for the globe (HOOKS.md §2).
//!
//! Two views, each filling caller-owned buffers (`clear()` then push, so a caller that keeps its
//! `Vec`s across calls allocates nothing after the first): the per-cell certificate state, and the
//! per-(tier, observable) certificate table. Both read one `DemoReport`; the scenario that makes
//! one worth drawing is `view_scenario`. Every number is a MODEL output of the twin (DERIVED from
//! the R5 micro tier's own events), never a production measurement.
//!
//! Positions: the cells are coarse (host kind × access network), not places. The view gives the
//! patch centre once (`header`); where the globe draws each cell around it is the globe's layout
//! choice. No device, household or street position leaves this module.

use crate::holon_demo::{demonstration, DemoReport, DEMO_LAT, DEMO_LON};
use crate::holon_dist::{EnvelopeBound, RefinementVerdict, FP_ONE, TAIL_LEVELS};

/// Horizon of the view scenario: 2 days per regime (5,760 cadences), the shortest at which p99
/// holds ≥ 30 order statistics beyond it (`holon_demo::DEMO_MIN_TAIL_SAMPLES`), so p99 is judged
/// and only p99.9 shows as unresolved. MODEL choice, for drawing only; the regression test runs 4
/// days.
pub const VIEW_HORIZON_S: f64 = 2.0 * 86_400.0;

/// The deterministic scenario behind the views (seeded; same buffers every call).
#[must_use]
pub fn view_scenario() -> DemoReport {
    demonstration(VIEW_HORIZON_S)
}

/// Bit index of each envelope bound in a `bound_mask`.
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

/// `header` f32 layout, length `HEADER_F32`:
/// `[patch_lat, patch_lon, horizon_s, down_share, n_cells, global_bound_mask_as_f32]`.
/// `down_share` = cadences with access network 0 down / cadences sampled, in the outage run.
pub const HEADER_F32: usize = 6;

/// Per-cell u32 stride. Record `i`:
/// `[cell_index, host_kind, network, state, bound_mask, devices]`
/// - `host_kind`: `micro::HostKind as u8` (0 phone, 1 home node, 2 laptop, 3 hosted server,
///   4 serve seat, 5 CI runner);
/// - `network`: access network 0..`ACCESS_NETWORKS`, or `u32::MAX` for a cell off the street;
/// - `state`: 0 = inside the Expected envelope; 1 = refused, this cell's own bound broke (the
///   refinement trigger sends it back to the finer tier); 2 = refused by a whole-tier bound (every
///   cell goes back);
/// - `bound_mask`: bit `bound_code(b)` set for each bound that broke for this cell (cell bounds)
///   or for the tier (global bounds, also in the header);
/// - `devices`: devices in the cell.
pub const CELL_U32: usize = 6;

/// Per-cell f32 stride. Record `i`: `[live_occupancy, certified_lo, certified_hi]`, online share
/// of the cell in [0, 1]: the live coarse state the trigger judged, and the occupancy range the
/// Expected certificate saw. `certified_lo = certified_hi = NaN` for a cell the certificate never
/// saw.
pub const CELL_F32: usize = 3;

/// The per-cell certificate state: what the refinement trigger said about a live coarse state
/// with access network 0 down, against the Expected-regime certificate.
pub fn fill_cells(r: &DemoReport, header: &mut Vec<f32>, u: &mut Vec<u32>, f: &mut Vec<f32>) {
    header.clear();
    u.clear();
    f.clear();
    let (broken, refine): (&[EnvelopeBound], &[_]) = match &r.trigger {
        RefinementVerdict::Inside => (&[], &[]),
        RefinementVerdict::Refine { broken, refine_cells } => (broken, refine_cells),
    };
    let global_mask = broken.iter().filter(|b| b.is_global()).fold(0u32, |m, b| m | (1 << bound_code(b)));
    let horizon = r.expected.envelope.horizon_s as f32;
    header.extend_from_slice(&[
        DEMO_LAT as f32,
        DEMO_LON as f32,
        horizon,
        r.down.0 as f32 / r.down.1.max(1) as f32,
        r.cells.len() as f32,
        global_mask as f32,
    ]);
    for (i, c) in r.cells.iter().enumerate() {
        let id = c.id();
        let own = broken.iter().filter(|b| matches!(b, EnvelopeBound::Cell { cell, .. } | EnvelopeBound::UnknownCell { cell } if *cell == id));
        let own_mask = own.fold(0u32, |m, b| m | (1 << bound_code(b)));
        let state = if global_mask != 0 {
            2
        } else if refine.contains(&id) {
            1
        } else {
            0
        };
        u.extend_from_slice(&[i as u32, c.kind as u32, c.net.map_or(u32::MAX, u32::from), state, own_mask | global_mask, c.n]);
        let live = r.live.cells.iter().find(|(k, _)| *k == id).map_or(f32::NAN, |(_, v)| *v as f32 / FP_ONE as f32);
        let (lo, hi) = r
            .expected
            .envelope
            .cells
            .iter()
            .find(|e| e.cell == id)
            .map_or((f32::NAN, f32::NAN), |e| (e.occupancy_fp.lo as f32 / FP_ONE as f32, e.occupancy_fp.hi as f32 / FP_ONE as f32));
        f.extend_from_slice(&[live, lo, hi]);
    }
}

/// Per-row u32 stride of the certificate table. Row = (tier, observable):
/// `[tier, observable, distributional_certified, mean_only_certified, w1_within, level_state × 4]`
/// - `tier`: 0 = Expected regime / mean-field cohort tier; 1 = network-0 outage / mean-field;
///   2 = network-0 outage / gated (refined) tier;
/// - `observable`: index in the certificate's list (0 `seat_round_arrivals`, 1 `devices_off_seat`);
/// - booleans as 0/1, of the observable (not of the whole certificate);
/// - `level_state` for p50, p95, p99, p99.9 (`TAIL_LEVELS` order): 0 within budget, 1 breach,
///   2 unresolved (too few order statistics beyond the level; not certified, not refused).
pub const CERT_U32: usize = 5 + TAIL_LEVELS.len();

/// Per-row f32 stride of the certificate table:
/// `[w1, w1_budget, mean_fine, mean_coarse, (fine, coarse, defect, budget) × 4]`, devices per 30 s
/// cadence.
pub const CERT_F32: usize = 4 + 4 * TAIL_LEVELS.len();

/// The certificate table of the three tiers the demonstration measured.
pub fn fill_certificates(r: &DemoReport, u: &mut Vec<u32>, f: &mut Vec<f32>) {
    u.clear();
    f.clear();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::micro::HostKind;

    #[test]
    fn view_buffers_have_the_documented_layout_and_show_the_outage() {
        let r = view_scenario();
        let (mut h, mut u, mut f) = (Vec::new(), Vec::new(), Vec::new());
        fill_cells(&r, &mut h, &mut u, &mut f);
        assert_eq!(h.len(), HEADER_F32);
        let n = h[4] as usize;
        assert_eq!((u.len(), f.len()), (n * CELL_U32, n * CELL_F32));
        // no whole-tier bound broke; network 0's home nodes went back to the finer tier, and only
        // network 0's cells did
        assert_eq!(h[5], 0.0);
        let rec: Vec<&[u32]> = u.chunks(CELL_U32).collect();
        assert!(rec.iter().any(|c| c[1] == HostKind::HomeNode as u32 && c[2] == 0 && c[3] == 1 && c[4] & (1 << 5) != 0), "{rec:?}");
        assert!(rec.iter().all(|c| c[3] == 0 || c[2] == 0), "{rec:?}");
        // a refused cell's live occupancy is outside its certified range
        for (c, v) in rec.iter().zip(f.chunks(CELL_F32)) {
            if c[3] == 1 && !v[1].is_nan() {
                assert!(v[0] < v[1] || v[0] > v[2], "{c:?} {v:?}");
            }
        }
        // the same call into kept buffers gives the same bytes (deterministic, reusable)
        let (h0, u0, f0) = (h.clone(), u.clone(), f.clone());
        fill_cells(&r, &mut h, &mut u, &mut f);
        assert_eq!((h0, u0), (h, u));
        assert_eq!(f0.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), f.iter().map(|v| v.to_bits()).collect::<Vec<_>>());

        let (mut cu, mut cf) = (Vec::new(), Vec::new());
        fill_certificates(&r, &mut cu, &mut cf);
        let rows = cu.len() / CERT_U32;
        assert_eq!((rows, cf.len()), (6, rows * CERT_F32));
        let row = |t: u32, k: u32| cu.chunks(CERT_U32).find(|c| c[0] == t && c[1] == k).unwrap();
        // outage / mean-field on devices_off_seat: mean-only passes, distributional refuses
        assert_eq!(&row(1, 1)[2..4], &[0, 1]);
        // p99.9 is unresolved at this horizon in every row
        assert!(cu.chunks(CERT_U32).all(|c| c[5 + 3] == 2), "{cu:?}");
    }
}
