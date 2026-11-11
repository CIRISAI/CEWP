//! Distributional closure certificates, validity envelopes, uncertainty propagation and the
//! out-of-envelope refinement trigger (FSD/twin/holon.md; extends `holon.rs`).
//!
//! `holon::ClosureCertificate` admits a coarse tier on one number: the mean-square closure defect
//! D_v² ≤ β plus the Lipschitz gate. A cohort reduction can pass that and still lose the tail
//! (correlated outages, hubs, rare partitions). This module adds:
//!
//! - `wasserstein1` and `Samples::quantile_ppm`: exact 1-D distances and quantiles on integer
//!   fixed-point samples, so a certificate's numbers do not depend on summation order or float
//!   rounding;
//! - `ObservableDefect`: per observable, W1 and quantile defects (p50/p95/p99/p99.9 by default),
//!   each against its own budget, alongside the old mean test kept for comparison;
//! - `ValidityEnvelope`: population, workload, topology class, failure regime, horizon and the
//!   per-cell coarse state seen when the certificate was measured. `SecondSquareFence::
//!   use_certificate` (in `holon.rs`) refuses any use outside it, naming the bound;
//! - `DistributionalCertificate::compose`: the triangle-inequality composition rule (proved in
//!   the comment on it);
//! - `UncertainInput` and the cohort map's interval / sample propagation;
//! - `refinement_trigger`: a live coarse state inside or outside the envelope, and which cells
//!   must be re-run at the finer tier.
//!
//! Status (exact; see FSD/twin/holon.md §7): nothing in this file is machine-checked in Lean.
//! CIRISHolon proves exact closure (`Object.Closed`, `nonfactoring_iff_not_closed`), exact
//! stacking (`Tiers.closed_comp`, `Object.Tier.stack`), the RMS horizon budget
//! (`StatClosure.iterate_dist_le_horizonBudget`) and the transport fence
//! (`Transport.claim_transport_does_not_grant_certificate`). It has no Wasserstein, quantile,
//! envelope or interval statement. The W1 / quantile composition bound below is proved in the
//! comment, by hand, and checked numerically by the tests.

use crate::holon::ClosureCertificate;
use crate::micro::{DensityClass, Rng};
use crate::workload::ParameterRegime;

/// Fixed-point scale: one observable unit is `FP_ONE` fixed-point units.
pub const FP_ONE: i64 = 1_000_000;

/// Parts-per-million denominator for quantile levels and shares.
pub const PPM: u64 = 1_000_000;

/// Median.
pub const P50: u32 = 500_000;
/// 95th percentile.
pub const P95: u32 = 950_000;
/// 99th percentile.
pub const P99: u32 = 990_000;
/// 99.9th percentile.
pub const P999: u32 = 999_000;
/// The four levels the review asks for (REVIEW.md 1.A).
pub const TAIL_LEVELS: [u32; 4] = [P50, P95, P99, P999];

/// A float to fixed point (round half away from zero).
#[must_use]
pub fn fp(x: f64) -> i64 {
    (x * FP_ONE as f64).round() as i64
}

/// A count to fixed point.
#[must_use]
pub fn fp_count(n: u64) -> i64 {
    n as i64 * FP_ONE
}

/// Fixed point to float (for printing only; no decision reads it).
#[must_use]
pub fn fp_f64(v: i64) -> f64 {
    v as f64 / FP_ONE as f64
}

// ------------------------------------------------------------------ samples, W1, quantiles

/// An empirical distribution: fixed-point samples, kept sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Samples {
    sorted: Vec<i64>,
}

impl Samples {
    /// From fixed-point values (any order).
    #[must_use]
    pub fn from_fp(mut v: Vec<i64>) -> Self {
        v.sort_unstable();
        Self { sorted: v }
    }

    /// From integer counts (devices, rows, ...).
    #[must_use]
    pub fn from_counts(v: &[u64]) -> Self {
        Self::from_fp(v.iter().map(|&c| fp_count(c)).collect())
    }

    /// Number of samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sorted.len()
    }

    /// No samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sorted.is_empty()
    }

    /// The sorted values.
    #[must_use]
    pub fn values(&self) -> &[i64] {
        &self.sorted
    }

    /// Exact sum.
    #[must_use]
    pub fn sum(&self) -> i128 {
        self.sorted.iter().map(|&x| i128::from(x)).sum()
    }

    /// Mean, floored to a fixed-point unit (exact up to one unit, order independent).
    #[must_use]
    pub fn mean_fp(&self) -> i64 {
        assert!(!self.is_empty(), "mean of no samples");
        self.sum().div_euclid(self.len() as i128) as i64
    }

    /// The lower empirical quantile Q(p) = min{x : F(x) ≥ p}, p in parts per million
    /// (`P99` = 990_000). Q(0) is the minimum. Exact: the k-th order statistic with
    /// k = ⌈p·n / 10⁶⌉ (at least 1).
    #[must_use]
    pub fn quantile_ppm(&self, p: u32) -> i64 {
        assert!(!self.is_empty(), "quantile of no samples");
        assert!(u64::from(p) <= PPM, "quantile level above 1");
        let n = self.len() as u64;
        let k = (u64::from(p) * n).div_ceil(PPM).max(1) as usize;
        self.sorted[k - 1]
    }
}

/// An exact non-negative rational number of fixed-point units: `num / den`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactFp {
    /// Numerator (fixed-point units × `den`).
    pub num: u128,
    /// Denominator (> 0).
    pub den: u128,
}

impl ExactFp {
    /// Zero.
    pub const ZERO: Self = Self { num: 0, den: 1 };

    /// A whole number of fixed-point units.
    #[must_use]
    pub fn from_fp(v: i64) -> Self {
        assert!(v >= 0, "ExactFp is non-negative");
        Self { num: v as u128, den: 1 }
    }

    /// `self ≤ budget` exactly.
    #[must_use]
    pub fn le_fp(&self, budget_fp: i64) -> bool {
        budget_fp >= 0 && self.num <= budget_fp as u128 * self.den
    }

    /// `self ≤ other` exactly.
    #[must_use]
    pub fn le(&self, other: &Self) -> bool {
        self.num * other.den <= other.num * self.den
    }

    /// Exact sum (reduced by the gcd so repeated sums stay small).
    #[must_use]
    pub fn add(&self, o: &Self) -> Self {
        let num = self.num * o.den + o.num * self.den;
        let den = self.den * o.den;
        let g = gcd(num, den).max(1);
        Self { num: num / g, den: den / g }
    }

    /// Rounded up to a whole fixed-point unit.
    #[must_use]
    pub fn ceil_fp(&self) -> i64 {
        self.num.div_ceil(self.den) as i64
    }

    /// For printing only.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.num as f64 / self.den as f64 / FP_ONE as f64
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The 1-Wasserstein distance between two empirical distributions on the line, exactly.
///
/// In one dimension W1(μ, ν) = ∫ |F_μ(z) − F_ν(z)| dz (Vallender 1973). Both empirical CDFs are
/// step functions that change only at sample values, so walking the two sorted lists once gives
/// the integral as a finite sum: between consecutive distinct values z_k < z_{k+1} the CDFs are
/// i/n and j/m, and the piece contributes |i·m − j·n| · (z_{k+1} − z_k) / (n·m). The numerator is
/// an integer, so the result is an exact rational in fixed-point units. For equal sizes this is
/// the familiar (1/n) Σ |x_(i) − y_(i)|.
#[must_use]
pub fn wasserstein1(a: &Samples, b: &Samples) -> ExactFp {
    assert!(!a.is_empty() && !b.is_empty(), "W1 of an empty sample");
    let (xa, xb) = (a.values(), b.values());
    let (n, m) = (xa.len() as u128, xb.len() as u128);
    let (mut i, mut j) = (0usize, 0usize);
    let mut z = xa[0].min(xb[0]);
    let mut acc: u128 = 0;
    loop {
        while i < xa.len() && xa[i] == z {
            i += 1;
        }
        while j < xb.len() && xb[j] == z {
            j += 1;
        }
        let next = match (xa.get(i), xb.get(j)) {
            (None, None) => break,
            (Some(&p), None) => p,
            (None, Some(&q)) => q,
            (Some(&p), Some(&q)) => p.min(q),
        };
        let fa = i as u128 * m;
        let fb = j as u128 * n;
        acc += fa.abs_diff(fb) * (next - z) as u128;
        z = next;
    }
    let g = gcd(acc, n * m).max(1);
    ExactFp { num: acc / g, den: n * m / g }
}

// ------------------------------------------------------------------ observables and defects

/// A quantile level and its own budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantileBudget {
    /// Level, parts per million.
    pub p_ppm: u32,
    /// Largest |Q_fine(p) − Q_coarse(p)| allowed, fixed point.
    pub budget_fp: i64,
}

/// What a certificate checks for one observable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservableSpec {
    /// Name (a certificate covers exactly the names it lists).
    pub name: String,
    /// Unit, for the report.
    pub unit: String,
    /// W1 budget, fixed point.
    pub w1_budget_fp: i64,
    /// Quantile levels and budgets.
    pub quantiles: Vec<QuantileBudget>,
    /// Fewest order statistics each sample must hold beyond a level, `n·(1 − p)`, before that level
    /// is compared at all. A level with fewer is UNRESOLVED: neither passed nor certified, and
    /// listed as such (the twin says it cannot predict that quantile from this run). 0 = always
    /// resolved.
    pub min_tail_samples: u64,
}

/// One quantile's measured defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantileDefect {
    /// Level, ppm.
    pub p_ppm: u32,
    /// Fine-tier quantile.
    pub fine_fp: i64,
    /// Coarse-tier quantile.
    pub coarse_fp: i64,
    /// |fine − coarse|.
    pub defect_fp: i64,
    /// Budget.
    pub budget_fp: i64,
    /// Both samples hold at least `min_tail_samples` order statistics beyond the level.
    pub resolved: bool,
}

impl QuantileDefect {
    /// Defect within budget (an unresolved level is not judged here; see `resolved`).
    #[must_use]
    pub fn within(&self) -> bool {
        self.defect_fp <= self.budget_fp
    }

    /// The level is resolved and within budget: the only case a certificate may claim it.
    #[must_use]
    pub fn certifies(&self) -> bool {
        self.resolved && self.within()
    }
}

/// `n·(1 − p) ≥ m`, exactly, in integers.
#[must_use]
pub fn tail_resolved(n: usize, p_ppm: u32, min_tail_samples: u64) -> bool {
    (n as u128) * u128::from(PPM - u64::from(p_ppm).min(PPM)) >= u128::from(min_tail_samples) * u128::from(PPM)
}

/// The measured fine-vs-coarse comparison for one observable.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservableDefect {
    /// The spec it was measured against.
    pub spec: ObservableSpec,
    /// Fine-tier sample count.
    pub n_fine: usize,
    /// Coarse-tier sample count.
    pub n_coarse: usize,
    /// Exact W1.
    pub w1: ExactFp,
    /// Quantile defects, in the spec's order.
    pub quantiles: Vec<QuantileDefect>,
    /// Fine mean.
    pub mean_fine_fp: i64,
    /// Coarse mean.
    pub mean_coarse_fp: i64,
    /// The mean-only test, as `holon.rs` already does it (kept so the two can be compared).
    pub mean: ClosureCertificate,
}

impl ObservableDefect {
    /// Measure fine against coarse.
    ///
    /// The mean test is `ClosureCertificate::compute` with ‖diff‖² = (μ_fine − μ_coarse)² and
    /// ‖motion‖² = μ_fine²: a relative squared mean defect. That normalisation is the twin's
    /// choice (MODEL): the observables here are stationary, so the FSD's ‖v(T x) − v(x)‖² would be
    /// near zero. `lipschitz_k` is the coarse map's declared constant.
    #[must_use]
    pub fn measure(
        reference_tier: &str,
        coarse_tier: &str,
        spec: &ObservableSpec,
        fine: &Samples,
        coarse: &Samples,
        mean_beta: f64,
        lipschitz_k: f64,
    ) -> Self {
        let w1 = wasserstein1(fine, coarse);
        let quantiles = spec
            .quantiles
            .iter()
            .map(|q| {
                let f = fine.quantile_ppm(q.p_ppm);
                let c = coarse.quantile_ppm(q.p_ppm);
                let resolved = tail_resolved(fine.len(), q.p_ppm, spec.min_tail_samples)
                    && tail_resolved(coarse.len(), q.p_ppm, spec.min_tail_samples);
                QuantileDefect { p_ppm: q.p_ppm, fine_fp: f, coarse_fp: c, defect_fp: (f - c).abs(), budget_fp: q.budget_fp, resolved }
            })
            .collect();
        let (mf, mc) = (fine.mean_fp(), coarse.mean_fp());
        let d = fp_f64(mf) - fp_f64(mc);
        let mean = ClosureCertificate::compute(reference_tier, coarse_tier, d * d, fp_f64(mf) * fp_f64(mf), mean_beta, lipschitz_k);
        Self {
            spec: spec.clone(),
            n_fine: fine.len(),
            n_coarse: coarse.len(),
            w1,
            quantiles,
            mean_fine_fp: mf,
            mean_coarse_fp: mc,
            mean,
        }
    }

    /// W1 within budget.
    #[must_use]
    pub fn w1_within(&self) -> bool {
        self.w1.le_fp(self.spec.w1_budget_fp)
    }

    /// Distributional test: W1 and every RESOLVED quantile within budget. Unresolved levels are
    /// not certified (see `unresolved_levels`); they do not refuse the observable either, because
    /// the run cannot tell either way.
    #[must_use]
    pub fn within(&self) -> bool {
        self.w1_within() && self.quantiles.iter().filter(|q| q.resolved).all(QuantileDefect::within)
    }

    /// Levels this run could not resolve (too few order statistics beyond them).
    #[must_use]
    pub fn unresolved_levels(&self) -> Vec<u32> {
        self.quantiles.iter().filter(|q| !q.resolved).map(|q| q.p_ppm).collect()
    }

    /// Every distributional bound that broke, by name.
    #[must_use]
    pub fn breaches(&self) -> Vec<EnvelopeBound> {
        let mut out = Vec::new();
        if !self.w1_within() {
            out.push(EnvelopeBound::W1 { observable: self.spec.name.clone(), w1_ceil_fp: self.w1.ceil_fp(), budget_fp: self.spec.w1_budget_fp });
        }
        for q in &self.quantiles {
            if q.resolved && !q.within() {
                out.push(EnvelopeBound::Quantile { observable: self.spec.name.clone(), p_ppm: q.p_ppm, defect_fp: q.defect_fp, budget_fp: q.budget_fp });
            }
        }
        out
    }
}

// ------------------------------------------------------------------ validity envelope

/// A closed integer range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Low end.
    pub lo: i64,
    /// High end.
    pub hi: i64,
}

impl Range {
    /// A range.
    #[must_use]
    pub fn new(lo: i64, hi: i64) -> Self {
        assert!(lo <= hi, "empty range");
        Self { lo, hi }
    }
    /// One value.
    #[must_use]
    pub fn point(v: i64) -> Self {
        Self { lo: v, hi: v }
    }
    /// Contains a value.
    #[must_use]
    pub fn contains(&self, v: i64) -> bool {
        self.lo <= v && v <= self.hi
    }
    /// Contains another range.
    #[must_use]
    pub fn contains_range(&self, o: &Range) -> bool {
        self.lo <= o.lo && o.hi <= self.hi
    }
}

/// Topology class a certificate was measured on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopologyClass {
    /// Street layout class of the focal patch (`micro::DensityClass`).
    pub density: DensityClass,
    /// Serve seats M.
    pub serve_seats: u32,
    /// Homing k.
    pub homing_k: u32,
}

/// The failure regime a certificate was measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureRegime {
    /// One of the FSD §4.4 regimes with independent device churn only.
    Standard(ParameterRegime),
    /// A §4.4 regime plus a correlated loss of one access network among `networks`
    /// (`holon_demo`'s minimal local model; the failure library is package `failures`).
    CorrelatedAccessLoss {
        /// Base regime.
        base: ParameterRegime,
        /// Access networks the households are split over.
        networks: u8,
    },
}

/// A coarse cell (cohort) name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellId(pub String);

/// The range of one cell's coarse state seen while the certificate was measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellRange {
    /// Cell.
    pub cell: CellId,
    /// Occupancy (online share of the cell), fixed point.
    pub occupancy_fp: Range,
}

/// Where a certificate is valid: the operating region it was actually checked on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidityEnvelope {
    /// Devices in the tier.
    pub population: Range,
    /// Workload, rows per device-hour, fixed point.
    pub workload_fp: Range,
    /// Topology class.
    pub topology: TopologyClass,
    /// Regimes it was measured in.
    pub regimes: Vec<FailureRegime>,
    /// Longest run checked, seconds.
    pub horizon_s: i64,
    /// Per-cell coarse state ranges seen.
    pub cells: Vec<CellRange>,
}

/// A point the twin wants to use a certificate at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatingPoint {
    /// Devices.
    pub population: i64,
    /// Rows per device-hour, fixed point.
    pub workload_fp: i64,
    /// Topology class.
    pub topology: TopologyClass,
    /// Failure regime.
    pub regime: FailureRegime,
    /// Run length, seconds.
    pub horizon_s: i64,
}

/// A bound that broke: why a certificate may not be used, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeBound {
    /// Population outside the certified range.
    Population { value: i64, certified: Range },
    /// Workload outside the certified range.
    Workload { value_fp: i64, certified_fp: Range },
    /// Another topology class.
    Topology { value: TopologyClass, certified: TopologyClass },
    /// A failure regime the certificate was not measured in.
    FailureRegime { value: FailureRegime, certified: Vec<FailureRegime> },
    /// Longer than the certified horizon.
    Horizon { value_s: i64, certified_s: i64 },
    /// A cell's live coarse state outside the range seen at certification.
    Cell { cell: CellId, value_fp: i64, certified_fp: Range },
    /// A live cell the certificate never saw.
    UnknownCell { cell: CellId },
    /// The observable is not one the certificate lists.
    ObservableNotCertified { observable: String },
    /// The observable's W1 broke its budget when measured.
    W1 { observable: String, w1_ceil_fp: i64, budget_fp: i64 },
    /// One quantile broke its budget when measured.
    Quantile { observable: String, p_ppm: u32, defect_fp: i64, budget_fp: i64 },
}

impl EnvelopeBound {
    /// Short name of the bound.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Population { .. } => "population",
            Self::Workload { .. } => "workload",
            Self::Topology { .. } => "topology",
            Self::FailureRegime { .. } => "failure regime",
            Self::Horizon { .. } => "horizon",
            Self::Cell { .. } => "cell state",
            Self::UnknownCell { .. } => "unknown cell",
            Self::ObservableNotCertified { .. } => "observable not certified",
            Self::W1 { .. } => "W1 budget",
            Self::Quantile { .. } => "quantile budget",
        }
    }
    /// A bound of the whole tier (not of one cell): breaking it puts every cell in question.
    #[must_use]
    pub fn is_global(&self) -> bool {
        !matches!(self, Self::Cell { .. } | Self::UnknownCell { .. })
    }
}

impl ValidityEnvelope {
    /// Every bound of the operating point that lies outside this envelope (empty = inside).
    #[must_use]
    pub fn check(&self, at: &OperatingPoint) -> Vec<EnvelopeBound> {
        let mut out = Vec::new();
        if !self.population.contains(at.population) {
            out.push(EnvelopeBound::Population { value: at.population, certified: self.population });
        }
        if !self.workload_fp.contains(at.workload_fp) {
            out.push(EnvelopeBound::Workload { value_fp: at.workload_fp, certified_fp: self.workload_fp });
        }
        if self.topology != at.topology {
            out.push(EnvelopeBound::Topology { value: at.topology, certified: self.topology });
        }
        if !self.regimes.contains(&at.regime) {
            out.push(EnvelopeBound::FailureRegime { value: at.regime, certified: self.regimes.clone() });
        }
        if at.horizon_s > self.horizon_s {
            out.push(EnvelopeBound::Horizon { value_s: at.horizon_s, certified_s: self.horizon_s });
        }
        out
    }

    /// Every bound of `inner` not contained in this envelope (empty = `inner` ⊆ `self`). Cells
    /// are compared by name: a cell `inner` lists must be listed here with a containing range.
    #[must_use]
    pub fn contains(&self, inner: &ValidityEnvelope) -> Vec<EnvelopeBound> {
        let mut out = Vec::new();
        if !self.population.contains_range(&inner.population) {
            out.push(EnvelopeBound::Population { value: inner.population.hi, certified: self.population });
        }
        if !self.workload_fp.contains_range(&inner.workload_fp) {
            out.push(EnvelopeBound::Workload { value_fp: inner.workload_fp.hi, certified_fp: self.workload_fp });
        }
        if self.topology != inner.topology {
            out.push(EnvelopeBound::Topology { value: inner.topology, certified: self.topology });
        }
        for r in &inner.regimes {
            if !self.regimes.contains(r) {
                out.push(EnvelopeBound::FailureRegime { value: *r, certified: self.regimes.clone() });
            }
        }
        if inner.horizon_s > self.horizon_s {
            out.push(EnvelopeBound::Horizon { value_s: inner.horizon_s, certified_s: self.horizon_s });
        }
        for c in &inner.cells {
            match self.cells.iter().find(|x| x.cell == c.cell) {
                None => out.push(EnvelopeBound::UnknownCell { cell: c.cell.clone() }),
                Some(x) if !x.occupancy_fp.contains_range(&c.occupancy_fp) => out.push(EnvelopeBound::Cell {
                    cell: c.cell.clone(),
                    value_fp: c.occupancy_fp.hi,
                    certified_fp: x.occupancy_fp,
                }),
                Some(_) => {}
            }
        }
        out
    }
}

// ------------------------------------------------------------------ the certificate

/// A propagated output interval recorded on a certificate (REVIEW 1.C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputInterval {
    /// Observable.
    pub observable: String,
    /// Quantile level, ppm.
    pub p_ppm: u32,
    /// Low end, fixed point.
    pub lo_fp: i64,
    /// High end, fixed point.
    pub hi_fp: i64,
}

/// A coarse tier certified against a finer one, distributionally, for the observables it lists,
/// inside the envelope it records.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionalCertificate {
    /// Fine tier.
    pub reference_tier: String,
    /// Coarse tier.
    pub coarse_tier: String,
    /// Where it was measured.
    pub envelope: ValidityEnvelope,
    /// The observables it covers, with their measured defects.
    pub observables: Vec<ObservableDefect>,
    /// Output intervals propagated from uncertain inputs.
    pub output_intervals: Vec<OutputInterval>,
}

/// The bound the composition rule gives for one observable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedObservable {
    /// Observable.
    pub name: String,
    /// Bound on W1(fine-most, coarse-most).
    pub w1_bound: ExactFp,
    /// Bounds on each quantile defect (levels both certificates measured).
    pub quantile_bounds: Vec<(u32, i64)>,
}

/// The composed (stacked) bound of two distributional certificates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedBound {
    /// Fine-most tier.
    pub reference_tier: String,
    /// Coarse-most tier.
    pub coarse_tier: String,
    /// Per-observable bounds (the intersection of the two lists).
    pub observables: Vec<ComposedObservable>,
    /// Horizon: the shorter of the two.
    pub horizon_s: i64,
}

impl DistributionalCertificate {
    /// Certified: lists at least one observable and every listed one is within all its budgets.
    #[must_use]
    pub fn certified(&self) -> bool {
        !self.observables.is_empty() && self.observables.iter().all(ObservableDefect::within)
    }

    /// What the old mean-only test would say about the same comparison.
    #[must_use]
    pub fn mean_only_certified(&self) -> bool {
        !self.observables.is_empty() && self.observables.iter().all(|o| o.mean.certified)
    }

    /// The measured defect for an observable, if the certificate lists it.
    #[must_use]
    pub fn covers(&self, observable: &str) -> Option<&ObservableDefect> {
        self.observables.iter().find(|o| o.spec.name == observable)
    }

    /// The certificate claims quantile level `p_ppm` of `observable`: certified as a whole, the
    /// observable listed, and that level resolved and within budget.
    #[must_use]
    pub fn covers_level(&self, observable: &str, p_ppm: u32) -> bool {
        self.certified() && self.covers(observable).is_some_and(|o| o.quantiles.iter().any(|q| q.p_ppm == p_ppm && q.certifies()))
    }

    /// Compose X→C (`self`) with C→D (`next`) into a bound for X→D.
    ///
    /// Preconditions, refused by name if they fail: `next.reference_tier == self.coarse_tier`;
    /// both certified; `next`'s envelope contains `self`'s (so C→D was measured on at least the
    /// operating region X→C admits). Only observables both list are carried.
    ///
    /// Rule: defects add. W1(P_X, P_D) ≤ W1(P_X, P_C) + W1(P_C, P_D) and
    /// |Q_X(p) − Q_D(p)| ≤ |Q_X(p) − Q_C(p)| + |Q_C(p) − Q_D(p)|.
    ///
    /// Proof. Quantiles: the triangle inequality on ℝ, applied to three numbers. W1 on the line:
    /// W1(μ, ν) = ∫ |F_μ − F_ν| dz; pointwise |F_X − F_D| ≤ |F_X − F_C| + |F_C − F_D|, and the
    /// integral is monotone and additive, so the inequality integrates. (In general dimension W1
    /// is a metric by the gluing lemma; one dimension is all this module uses.) ∎
    ///
    /// What the proof does NOT give: that the C→D certificate's measured defect applies to the
    /// C-tier law that X→C actually feeds it. The envelope-containment precondition is the
    /// engineering stand-in for that hypothesis; it is not a theorem (MODEL), the same gap
    /// CIRISHolon's `StatClosure` names for locality of the coarse law.
    pub fn compose(&self, next: &Self) -> Result<ComposedBound, Vec<EnvelopeBound>> {
        let mut broken = Vec::new();
        if next.reference_tier != self.coarse_tier {
            broken.push(EnvelopeBound::Topology { value: next.envelope.topology, certified: self.envelope.topology });
        }
        for c in [self, next] {
            for o in &c.observables {
                broken.extend(o.breaches());
            }
        }
        broken.extend(next.envelope.contains(&self.envelope));
        if !broken.is_empty() {
            return Err(broken);
        }
        let mut observables = Vec::new();
        for a in &self.observables {
            let Some(b) = next.covers(&a.spec.name) else { continue };
            let quantile_bounds = a
                .quantiles
                .iter()
                .filter_map(|qa| b.quantiles.iter().find(|qb| qb.p_ppm == qa.p_ppm).map(|qb| (qa.p_ppm, qa.defect_fp + qb.defect_fp)))
                .collect();
            observables.push(ComposedObservable { name: a.spec.name.clone(), w1_bound: a.w1.add(&b.w1), quantile_bounds });
        }
        Ok(ComposedBound {
            reference_tier: self.reference_tier.clone(),
            coarse_tier: next.coarse_tier.clone(),
            observables,
            horizon_s: self.envelope.horizon_s.min(next.envelope.horizon_s),
        })
    }
}

// ------------------------------------------------------------------ the cohort map

/// How a cohort cell's devices are online in the coarse map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellLaw {
    /// Each device online independently with probability `a_ppm` (the mean-field cohort holon).
    Independent {
        /// Online share, ppm.
        a_ppm: u32,
    },
    /// The whole cell rides a shared factor (an access network): when factor `factor` is up,
    /// each device is online independently with `a_up_ppm`; when it is down, none is.
    Gated {
        /// Index into `CohortModel::factors_up_ppm`.
        factor: usize,
        /// Online share while the factor is up, ppm.
        a_up_ppm: u32,
    },
    /// A per-draw online share shared by the whole cell, drawn uniformly from `shares_ppm`
    /// (an input given as a sample set: REVIEW 1.C).
    SharedDraw {
        /// The input sample set, ppm.
        shares_ppm: Vec<u32>,
    },
}

/// One cohort cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortCell {
    /// Name.
    pub id: CellId,
    /// Devices.
    pub n: u32,
    /// Law.
    pub law: CellLaw,
}

/// The coarse cohort map: online devices = Σ over cells of the cell's online count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortModel {
    /// Cells.
    pub cells: Vec<CohortCell>,
    /// Shared factors' up probabilities, ppm.
    pub factors_up_ppm: Vec<u32>,
}

fn bernoulli_ppm(rng: &mut Rng, p_ppm: u32) -> bool {
    rng.next_u64() % PPM < u64::from(p_ppm)
}

impl CohortModel {
    /// Total devices.
    #[must_use]
    pub fn population(&self) -> u64 {
        self.cells.iter().map(|c| u64::from(c.n)).sum()
    }

    /// `count` independent draws of (online, offline) device totals, seeded.
    ///
    /// Every draw consumes the same random numbers in the same order whatever the laws and shares
    /// are (one u64 per factor, one per cell, one per device), so two models over the same cells
    /// are coupled draw by draw (common random numbers). Device j is online iff its uniform is
    /// below the share in force, so the online total is nondecreasing in every share, draw by
    /// draw. That makes interval propagation exact on the sampled map (`propagate_interval`).
    #[must_use]
    pub fn sample(&self, count: usize, seed: u64) -> (Vec<u64>, Vec<u64>) {
        let mut rng = Rng::new(seed);
        let n = self.population();
        let mut online = Vec::with_capacity(count);
        let mut offline = Vec::with_capacity(count);
        let mut up = vec![false; self.factors_up_ppm.len()];
        for _ in 0..count {
            for (k, p) in self.factors_up_ppm.iter().enumerate() {
                up[k] = bernoulli_ppm(&mut rng, *p);
            }
            let mut on = 0u64;
            for c in &self.cells {
                // one u64 per cell whatever its law, so every law stays on the same stream
                let pick = rng.next_u64();
                let a = match &c.law {
                    CellLaw::Independent { a_ppm } => *a_ppm,
                    CellLaw::Gated { factor, a_up_ppm } => if up[*factor] { *a_up_ppm } else { 0 },
                    CellLaw::SharedDraw { shares_ppm } => shares_ppm[(pick % shares_ppm.len() as u64) as usize],
                };
                for _ in 0..c.n {
                    on += u64::from(bernoulli_ppm(&mut rng, a));
                }
            }
            online.push(on);
            offline.push(n - on);
        }
        (online, offline)
    }
}

// ------------------------------------------------------------------ uncertain inputs

/// An input share given with its uncertainty (REVIEW 1.C).
#[derive(Debug, Clone, PartialEq)]
pub enum UncertainInput {
    /// A single value.
    Point(f64),
    /// Somewhere in [lo, hi], nothing more known.
    Interval {
        /// Low end.
        lo: f64,
        /// High end.
        hi: f64,
    },
    /// An empirical sample set of the input.
    Samples(Vec<f64>),
}

impl UncertainInput {
    /// The interval it spans.
    #[must_use]
    pub fn span(&self) -> (f64, f64) {
        match self {
            Self::Point(x) => (*x, *x),
            Self::Interval { lo, hi } => (*lo, *hi),
            Self::Samples(v) => v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| (a.min(x), b.max(x))),
        }
    }
    /// The midpoint collapse (what a point model does with an interval).
    #[must_use]
    pub fn midpoint(&self) -> f64 {
        let (lo, hi) = self.span();
        (lo + hi) / 2.0
    }
}

fn share_ppm(x: f64) -> u32 {
    (x.clamp(0.0, 1.0) * PPM as f64).round() as u32
}

/// Interval propagation through the cohort map, which is monotone: the online total is
/// nondecreasing in every cell's share and the offline total nonincreasing (draw by draw, under
/// the coupling `CohortModel::sample` keeps). So the image of a box of shares is spanned by its
/// two corners, and so is every quantile of the image: for each level p, Q_p ranges over
/// [Q_p(all lo), Q_p(all hi)] for online and the reverse for offline. `inputs[i]` is the
/// share interval of `cells[i]` (only `Independent` cells are re-parameterised; a cell's other
/// law is kept). Returns (online intervals, offline intervals) per level.
#[must_use]
pub fn propagate_interval(
    model: &CohortModel,
    inputs: &[UncertainInput],
    levels: &[u32],
    count: usize,
    seed: u64,
    observables: (&str, &str),
) -> Vec<OutputInterval> {
    assert_eq!(inputs.len(), model.cells.len(), "one input per cell");
    let corner = |pick_hi: bool| {
        let mut m = model.clone();
        for (c, u) in m.cells.iter_mut().zip(inputs) {
            if let CellLaw::Independent { a_ppm } = &mut c.law {
                let (lo, hi) = u.span();
                *a_ppm = share_ppm(if pick_hi { hi } else { lo });
            }
        }
        m.sample(count, seed)
    };
    let (on_lo, off_lo) = corner(false);
    let (on_hi, off_hi) = corner(true);
    let (on_lo, off_lo, on_hi, off_hi) =
        (Samples::from_counts(&on_lo), Samples::from_counts(&off_lo), Samples::from_counts(&on_hi), Samples::from_counts(&off_hi));
    let mut out = Vec::new();
    for &p in levels {
        let (a, b) = (on_lo.quantile_ppm(p), on_hi.quantile_ppm(p));
        out.push(OutputInterval { observable: observables.0.to_string(), p_ppm: p, lo_fp: a.min(b), hi_fp: a.max(b) });
        let (a, b) = (off_lo.quantile_ppm(p), off_hi.quantile_ppm(p));
        out.push(OutputInterval { observable: observables.1.to_string(), p_ppm: p, lo_fp: a.min(b), hi_fp: a.max(b) });
    }
    out
}

/// Sample propagation, for inputs given as sample sets or for maps that are not monotone: each
/// draw takes the cell's share from its input set (one shared draw per cell per sample; a
/// `Point` stays a point, an `Interval` is read as uniform on a 1% grid, which is a MODEL
/// choice). Returns the cohort model whose `sample` carries the input uncertainty.
#[must_use]
pub fn propagate_samples(model: &CohortModel, inputs: &[UncertainInput]) -> CohortModel {
    assert_eq!(inputs.len(), model.cells.len(), "one input per cell");
    let mut m = model.clone();
    for (c, u) in m.cells.iter_mut().zip(inputs) {
        c.law = match u {
            UncertainInput::Point(x) => CellLaw::Independent { a_ppm: share_ppm(*x) },
            UncertainInput::Interval { lo, hi } => {
                let steps = ((hi - lo) * 100.0).round().max(0.0) as u32;
                CellLaw::SharedDraw { shares_ppm: (0..=steps).map(|k| share_ppm(lo + f64::from(k) * (hi - lo) / f64::from(steps.max(1)))).collect() }
            }
            UncertainInput::Samples(v) => CellLaw::SharedDraw { shares_ppm: v.iter().map(|&x| share_ppm(x)).collect() },
        };
    }
    m
}

// ------------------------------------------------------------------ out-of-envelope detection

/// A live coarse state: where the twin is running the coarse tier now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCoarseState {
    /// Operating point.
    pub at: OperatingPoint,
    /// Each cell's current occupancy, fixed point.
    pub cells: Vec<(CellId, i64)>,
}

/// The refinement trigger's verdict (REVIEW 1.D).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefinementVerdict {
    /// Inside the envelope: the coarse tier may answer.
    Inside,
    /// "I cannot predict this: the coarse model has left its certified envelope." The bounds
    /// that broke, and the cells to re-run at the finer tier (every cell when a whole-tier bound
    /// broke).
    Refine {
        /// What broke.
        broken: Vec<EnvelopeBound>,
        /// Cells to refine, sorted.
        refine_cells: Vec<CellId>,
    },
}

/// Decide whether a live coarse state is inside the envelope.
#[must_use]
pub fn refinement_trigger(envelope: &ValidityEnvelope, live: &LiveCoarseState) -> RefinementVerdict {
    let mut broken = envelope.check(&live.at);
    let mut cells = Vec::new();
    for (id, v) in &live.cells {
        match envelope.cells.iter().find(|c| &c.cell == id) {
            None => {
                broken.push(EnvelopeBound::UnknownCell { cell: id.clone() });
                cells.push(id.clone());
            }
            Some(c) if !c.occupancy_fp.contains(*v) => {
                broken.push(EnvelopeBound::Cell { cell: id.clone(), value_fp: *v, certified_fp: c.occupancy_fp });
                cells.push(id.clone());
            }
            Some(_) => {}
        }
    }
    if broken.is_empty() {
        return RefinementVerdict::Inside;
    }
    if broken.iter().any(EnvelopeBound::is_global) {
        cells = live.cells.iter().map(|(id, _)| id.clone()).collect();
    }
    cells.sort();
    cells.dedup();
    RefinementVerdict::Refine { broken, refine_cells: cells }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[i64]) -> Samples {
        Samples::from_fp(v.iter().map(|&x| x * FP_ONE).collect())
    }

    #[test]
    fn w1_hand_cases() {
        // identical: 0
        assert_eq!(wasserstein1(&s(&[1, 2, 3]), &s(&[3, 1, 2])), ExactFp::ZERO);
        // a shift by 2 moves all mass 2
        let w = wasserstein1(&s(&[0, 1, 5]), &s(&[2, 3, 7]));
        assert_eq!((w.num, w.den), (2 * FP_ONE as u128, 1));
        // {0} vs {0, 1}: half the mass moves 1 => 1/2
        let w = wasserstein1(&s(&[0]), &s(&[0, 1]));
        assert_eq!(w.num * 2, w.den * FP_ONE as u128);
        // {0, 0, 3} vs {1, 2}: CDFs 2/3 vs 0 on [0,1), 2/3 vs 1/2 on [1,2), 2/3 vs 1 on [2,3)
        // => 2/3 + 1/6 + 1/3 = 7/6
        let w = wasserstein1(&s(&[0, 0, 3]), &s(&[1, 2]));
        assert_eq!(w.num * 6, w.den * 7 * FP_ONE as u128);
        // symmetric
        assert_eq!(wasserstein1(&s(&[1, 2]), &s(&[0, 0, 3])), w);
        // equal sizes: mean |x_(i) - y_(i)| = (|1-0| + |4-2| + |4-9|) / 3 = 8/3
        let w = wasserstein1(&s(&[4, 1, 4]), &s(&[9, 0, 2]));
        assert_eq!(w.num * 3, w.den * 8 * FP_ONE as u128);
        assert!(w.le_fp(fp(8.0 / 3.0) + 1) && !w.le_fp(fp(8.0 / 3.0) - 1));
    }

    #[test]
    fn w1_is_order_independent_and_matches_sorted_coupling() {
        let mut rng = Rng::new(9);
        let a: Vec<i64> = (0..257).map(|_| (rng.next_u64() % 10_000) as i64).collect();
        let b: Vec<i64> = (0..257).map(|_| (rng.next_u64() % 12_000) as i64).collect();
        let mut ar = a.clone();
        ar.reverse();
        let w = wasserstein1(&Samples::from_fp(a.clone()), &Samples::from_fp(b.clone()));
        assert_eq!(w, wasserstein1(&Samples::from_fp(ar), &Samples::from_fp(b.clone())));
        let (sa, sb) = (Samples::from_fp(a), Samples::from_fp(b));
        let direct: u128 = sa.values().iter().zip(sb.values()).map(|(x, y)| x.abs_diff(*y) as u128).sum();
        assert_eq!(w.num * 257, direct * w.den);
    }

    #[test]
    fn tail_resolution_is_exact_and_unresolved_levels_are_not_claimed() {
        // n·(1 − p) ≥ m, exact at the boundary: 1000 samples hold 10 beyond p99, 1 beyond p99.9
        assert!(tail_resolved(1000, P99, 10));
        assert!(!tail_resolved(999, P99, 10));
        assert!(tail_resolved(1000, P999, 1));
        assert!(!tail_resolved(1000, P999, 2));
        assert!(tail_resolved(0, P50, 0));
        // a p99.9 defect far over budget on 1000 samples: refused with no floor, unresolved with one
        let fine: Vec<u64> = (0..1000).map(|i| if i >= 998 { 50 } else { i % 10 }).collect();
        let coarse: Vec<u64> = (0..1000).map(|i| i % 10).collect();
        let mut sp = spec("x");
        sp.w1_budget_fp = fp(1.0);
        let strict = ObservableDefect::measure("f", "c", &sp, &Samples::from_counts(&fine), &Samples::from_counts(&coarse), 0.02, 1.0);
        assert!(!strict.within() && strict.unresolved_levels().is_empty(), "{strict:?}");
        sp.min_tail_samples = 2;
        let floored = ObservableDefect::measure("f", "c", &sp, &Samples::from_counts(&fine), &Samples::from_counts(&coarse), 0.02, 1.0);
        assert!(floored.within(), "{floored:?}");
        assert_eq!(floored.unresolved_levels(), vec![P999]);
        assert!(floored.breaches().is_empty());
    }

    #[test]
    fn quantiles_are_lower_order_statistics() {
        let x = s(&(1..=1000).collect::<Vec<_>>());
        assert_eq!(x.quantile_ppm(P50), 500 * FP_ONE);
        assert_eq!(x.quantile_ppm(P95), 950 * FP_ONE);
        assert_eq!(x.quantile_ppm(P99), 990 * FP_ONE);
        assert_eq!(x.quantile_ppm(P999), 999 * FP_ONE);
        assert_eq!(x.quantile_ppm(0), FP_ONE);
        assert_eq!(x.quantile_ppm(1_000_000), 1000 * FP_ONE);
        let y = s(&[5, 1, 3]);
        assert_eq!(y.quantile_ppm(P50), 3 * FP_ONE); // ceil(1.5) = 2nd
        assert_eq!(y.quantile_ppm(P999), 5 * FP_ONE);
        assert_eq!(y.mean_fp(), 3 * FP_ONE);
    }

    fn spec(name: &str) -> ObservableSpec {
        ObservableSpec {
            name: name.into(),
            unit: "devices".into(),
            w1_budget_fp: fp(1.0),
            quantiles: TAIL_LEVELS.iter().map(|&p| QuantileBudget { p_ppm: p, budget_fp: fp(2.0) }).collect(),
            min_tail_samples: 0,
        }
    }

    fn topo() -> TopologyClass {
        TopologyClass { density: DensityClass::Urban, serve_seats: 1, homing_k: 1 }
    }

    fn envelope() -> ValidityEnvelope {
        ValidityEnvelope {
            population: Range::new(100, 200),
            workload_fp: Range::new(fp(5.0), fp(10.0)),
            topology: topo(),
            regimes: vec![FailureRegime::Standard(ParameterRegime::Expected)],
            horizon_s: 86_400,
            cells: vec![CellRange { cell: CellId("phone".into()), occupancy_fp: Range::new(fp(0.7), fp(0.95)) }],
        }
    }

    fn inside() -> OperatingPoint {
        OperatingPoint {
            population: 150,
            workload_fp: fp(7.5),
            topology: topo(),
            regime: FailureRegime::Standard(ParameterRegime::Expected),
            horizon_s: 3_600,
        }
    }

    fn cert(defect_shift: i64) -> DistributionalCertificate {
        let fine = s(&(0..200).collect::<Vec<_>>());
        let coarse = s(&(0..200).map(|x| x + defect_shift).collect::<Vec<_>>());
        DistributionalCertificate {
            reference_tier: "R5".into(),
            coarse_tier: "R4c".into(),
            envelope: envelope(),
            observables: vec![ObservableDefect::measure("R5", "R4c", &spec("arrivals"), &fine, &coarse, 0.02, 1.0)],
            output_intervals: vec![],
        }
    }

    #[test]
    fn refused_outside_each_envelope_dimension() {
        use crate::holon::{SecondSquareFence, TransportRefusal};
        let c = cert(0);
        assert!(c.certified());
        assert!(SecondSquareFence::use_certificate(&c, "arrivals", &inside()).is_ok());
        let cases: Vec<(OperatingPoint, &str)> = vec![
            (OperatingPoint { population: 250, ..inside() }, "population"),
            (OperatingPoint { workload_fp: fp(12.0), ..inside() }, "workload"),
            (OperatingPoint { topology: TopologyClass { serve_seats: 3, homing_k: 2, ..topo() }, ..inside() }, "topology"),
            (OperatingPoint { topology: TopologyClass { density: DensityClass::Village, ..topo() }, ..inside() }, "topology"),
            (OperatingPoint { regime: FailureRegime::Standard(ParameterRegime::Adversarial), ..inside() }, "failure regime"),
            (
                OperatingPoint { regime: FailureRegime::CorrelatedAccessLoss { base: ParameterRegime::Expected, networks: 3 }, ..inside() },
                "failure regime",
            ),
            (OperatingPoint { horizon_s: 2 * 86_400, ..inside() }, "horizon"),
        ];
        for (at, name) in cases {
            match SecondSquareFence::use_certificate(&c, "arrivals", &at) {
                Err(TransportRefusal::OutsideEnvelope { broken, .. }) => {
                    assert_eq!(broken.len(), 1, "{broken:?}");
                    assert_eq!(broken[0].name(), name);
                }
                other => panic!("{name}: {other:?}"),
            }
        }
        // an observable the certificate does not list is not certified
        match SecondSquareFence::use_certificate(&c, "per_device_reachable_peers", &inside()) {
            Err(TransportRefusal::NotCertified { breaches, .. }) => assert_eq!(breaches[0].name(), "observable not certified"),
            other => panic!("{other:?}"),
        }
        // a certificate whose tail broke is refused by the bound that broke
        let bad = cert(3);
        assert!(!bad.certified() && bad.mean_only_certified());
        match SecondSquareFence::use_certificate(&bad, "arrivals", &inside()) {
            Err(TransportRefusal::NotCertified { breaches, .. }) => {
                assert!(breaches.iter().any(|b| b.name() == "W1 budget"));
                assert!(breaches.iter().any(|b| b.name() == "quantile budget"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn composition_bounds_the_composed_defect() {
        // Three tiers' samples of one observable; the composed bound must hold for the direct
        // X-vs-D defect (triangle inequality, proved on `compose`).
        let mut rng = Rng::new(31);
        let draw = |rng: &mut Rng, n: usize, lo: u64, span: u64| -> Samples {
            Samples::from_fp((0..n).map(|_| ((lo + rng.next_u64() % span) as i64) * FP_ONE / 10).collect())
        };
        let x = draw(&mut rng, 400, 1000, 200);
        let c = draw(&mut rng, 300, 1005, 200);
        let d = draw(&mut rng, 500, 998, 210);
        let mut sp = spec("arrivals");
        sp.w1_budget_fp = fp(2.0);
        for q in &mut sp.quantiles {
            q.budget_fp = fp(3.0);
        }
        let xc = ObservableDefect::measure("X", "C", &sp, &x, &c, 0.02, 1.0);
        let cd = ObservableDefect::measure("C", "D", &sp, &c, &d, 0.02, 1.0);
        let xd = ObservableDefect::measure("X", "D", &sp, &x, &d, 0.02, 1.0);
        let mk = |r: &str, k: &str, o: ObservableDefect, horizon_s: i64| DistributionalCertificate {
            reference_tier: r.into(),
            coarse_tier: k.into(),
            envelope: ValidityEnvelope { horizon_s, ..envelope() },
            observables: vec![o],
            output_intervals: vec![],
        };
        let a = mk("X", "C", xc, 86_400);
        let b = mk("C", "D", cd, 2 * 86_400);
        assert!(a.certified() && b.certified(), "{a:?} {b:?}");
        let bound = a.compose(&b).expect("composes");
        assert_eq!(bound.horizon_s, 86_400);
        let ob = &bound.observables[0];
        assert!(xd.w1.le(&ob.w1_bound), "W1 {} > bound {}", xd.w1.as_f64(), ob.w1_bound.as_f64());
        for (q, (p, bnd)) in xd.quantiles.iter().zip(&ob.quantile_bounds) {
            assert_eq!(q.p_ppm, *p);
            assert!(q.defect_fp <= *bnd);
        }
        // composition refuses when the second envelope does not contain the first
        let narrow = mk("C", "D", b.observables[0].clone(), 3_600);
        let err = a.compose(&narrow).unwrap_err();
        assert!(err.iter().any(|e| e.name() == "horizon"));
    }

    #[test]
    fn interval_propagation_is_monotone_and_brackets_the_samples() {
        let model = CohortModel {
            cells: vec![
                CohortCell { id: CellId("phone".into()), n: 100, law: CellLaw::Independent { a_ppm: 825_000 } },
                CohortCell { id: CellId("home".into()), n: 20, law: CellLaw::Independent { a_ppm: 1_000_000 } },
            ],
            factors_up_ppm: vec![],
        };
        let inputs = [UncertainInput::Interval { lo: 0.70, hi: 0.95 }, UncertainInput::Point(1.0)];
        let iv = propagate_interval(&model, &inputs, &TAIL_LEVELS, 4_000, 5, ("on", "off"));
        // the spread input on the same stream: each draw's share lies in [0.70, 0.95], so under
        // the coupling each draw's total lies between the corners' totals, and so does every
        // empirical quantile (exactly, no margin)
        let spread = propagate_samples(&model, &inputs);
        let (on, off) = spread.sample(4_000, 5);
        let (on, off) = (Samples::from_counts(&on), Samples::from_counts(&off));
        for o in &iv {
            assert!(o.lo_fp <= o.hi_fp);
            let q = if o.observable == "on" { on.quantile_ppm(o.p_ppm) } else { off.quantile_ppm(o.p_ppm) };
            assert!(q >= o.lo_fp && q <= o.hi_fp, "{o:?} q={q}");
        }
        assert!(iv.iter().any(|o| o.hi_fp - o.lo_fp >= 10 * FP_ONE), "the interval is carried, not collapsed: {iv:?}");
        // coupling: draw by draw, more share never means fewer online
        let mut lo = model.clone();
        lo.cells[0].law = CellLaw::Independent { a_ppm: 700_000 };
        let mut hi = model.clone();
        hi.cells[0].law = CellLaw::Independent { a_ppm: 950_000 };
        let (a, _) = lo.sample(500, 77);
        let (b, _) = hi.sample(500, 77);
        assert!(a.iter().zip(&b).all(|(x, y)| x <= y));
    }

    #[test]
    fn midpoint_collapse_passes_the_mean_and_fails_the_tail() {
        // 100 phones whose online share is known only to lie in [0.70, 0.95] (the FSD §4.4
        // p_offline spread, 5%..30%, read as an input interval) and moves for all of them
        // together (an evening, a carrier's bad hour). The midpoint model draws every phone at
        // 0.825. Means agree; the tail of devices off the seat does not.
        let model = CohortModel {
            cells: vec![CohortCell { id: CellId("phone".into()), n: 100, law: CellLaw::Independent { a_ppm: 825_000 } }],
            factors_up_ppm: vec![],
        };
        let input = [UncertainInput::Interval { lo: 0.70, hi: 0.95 }];
        let truth = propagate_samples(&model, &input);
        let (_, off_t) = truth.sample(20_000, 11);
        let mid = propagate_samples(&model, &[UncertainInput::Point(input[0].midpoint())]);
        let (_, off_m) = mid.sample(20_000, 12);
        let sp = ObservableSpec {
            name: "devices_off_seat".into(),
            unit: "devices".into(),
            w1_budget_fp: fp(1.0),
            quantiles: TAIL_LEVELS.iter().map(|&p| QuantileBudget { p_ppm: p, budget_fp: fp(3.0) }).collect(),
            min_tail_samples: 0,
        };
        let d = ObservableDefect::measure("input-spread", "midpoint", &sp, &Samples::from_counts(&off_t), &Samples::from_counts(&off_m), 0.02, 1.0);
        assert!(d.mean.certified, "mean test passes: {d:?}");
        assert!(!d.within(), "tail test fails: {d:?}");
        let p99 = d.quantiles.iter().find(|q| q.p_ppm == P99).unwrap();
        assert!(!p99.within(), "{p99:?}");
        // the propagated (sample) model is the truth's own law: it passes
        let (_, off_p) = truth.sample(20_000, 13);
        let ok = ObservableDefect::measure("input-spread", "propagated", &sp, &Samples::from_counts(&off_t), &Samples::from_counts(&off_p), 0.02, 1.0);
        assert!(ok.within(), "{ok:?}");
        // the numbers FSD/twin/holon.md §4 quotes (shown with --nocapture)
        for (name, x) in [("midpoint", &d), ("propagated", &ok)] {
            let qs: Vec<String> = x.quantiles.iter().map(|q| format!("p{}={}/{}", q.p_ppm, q.fine_fp / FP_ONE, q.coarse_fp / FP_ONE)).collect();
            println!("{name}: mean {}/{} D2 {:.6} W1 {:.3} {}", fp_f64(x.mean_fine_fp), fp_f64(x.mean_coarse_fp), x.mean.closure_defect_d2, x.w1.as_f64(), qs.join(" "));
        }
    }

    #[test]
    fn trigger_names_the_cells_that_left() {
        let env = envelope();
        let live = LiveCoarseState { at: inside(), cells: vec![(CellId("phone".into()), fp(0.8))] };
        assert_eq!(refinement_trigger(&env, &live), RefinementVerdict::Inside);
        let live = LiveCoarseState { at: inside(), cells: vec![(CellId("phone".into()), fp(0.4)), (CellId("laptop/net0".into()), 0)] };
        match refinement_trigger(&env, &live) {
            RefinementVerdict::Refine { broken, refine_cells } => {
                assert_eq!(broken.len(), 2);
                assert_eq!(refine_cells, vec![CellId("laptop/net0".into()), CellId("phone".into())]);
            }
            v => panic!("{v:?}"),
        }
        // a whole-tier bound puts every cell in question
        let live = LiveCoarseState {
            at: OperatingPoint { horizon_s: 10 * 86_400, ..inside() },
            cells: vec![(CellId("phone".into()), fp(0.8))],
        };
        match refinement_trigger(&env, &live) {
            RefinementVerdict::Refine { broken, refine_cells } => {
                assert_eq!(broken[0].name(), "horizon");
                assert_eq!(refine_cells, vec![CellId("phone".into())]);
            }
            v => panic!("{v:?}"),
        }
    }
}
