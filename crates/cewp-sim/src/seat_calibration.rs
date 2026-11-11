//! Seat-capacity calibration discipline: which measured windows the fit may read, what the
//! held-out windows say about the fit, an interval on every measured input, the propagation of
//! those intervals to the knee, and the permit residuals decomposed term by term.
//!
//! The production evidence is four 60-second windows of one seat's counters (seat-1,
//! 2026-11-10, edge v44.1.2) and one approximate busy-hour reading. That is thin, and this
//! module says so in every output: no knee is printed as one number, every knee is tagged
//! `PROJECTED` with its interval and the term that binds at each end of it.
//!
//! * **Split.** [`FIT`] names every (window, counter) a fitted or measured-ratio parameter may
//!   read; [`HELD_OUT`] names the counters the fit must predict. [`fit`] reads windows only
//!   through a [`FitView`], which refuses anything outside [`FIT`].
//! * **Intervals.** [`INPUT_INTERVALS`] gives each input an interval and how it was derived.
//!   The derivations that rest on the windows are computed here from the window constants
//!   ([`window_spreads`]), not typed in.
//! * **Propagation.** [`propagate`] is the vertex method on one monotone output: each input
//!   moved alone to each end of its interval, then all inputs together at the ends that lower
//!   (raise) it. [`knee_interval`] runs it on each term's knee alone (CPU, sweep permits, the
//!   busiest lock), where it is monotone, and takes `[min lo, min hi]` over the terms: the knee
//!   is a minimum that switches term across the box, so the vertex method on the knee itself
//!   is not a bound (a failing test found that). An outer bound, not a probability.
//! * **Growth axis.** How the server's `key_publish_set` grows with the fleet is not known to
//!   the twin (FRONTIER). [`knee_by_growth`] sweeps it from 0 (today's size) to 1 (with
//!   announced nodes) instead of choosing one.

use std::cell::RefCell;

use crate::capacity::{
    self, calibrate, edge_971, ledger, measured_mix, run, window_a, window_a_prime, window_b,
    window_c, busy_hour, Binding, CostParams, Plane, ReadSet, Scenario, C,
};

// ---------------------------------------------------------------------------
// Intervals
// ---------------------------------------------------------------------------

/// A closed interval. A knee, a core count or a utilisation computed from interval inputs is
/// one of these, never a bare number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// Lower end.
    pub lo: f64,
    /// Upper end.
    pub hi: f64,
}

impl Interval {
    /// The interval between two values, in either order.
    #[must_use]
    pub fn new(a: f64, b: f64) -> Self {
        Self { lo: a.min(b), hi: a.max(b) }
    }

    /// Widen to hold `x`.
    #[must_use]
    pub fn with(self, x: f64) -> Self {
        Self { lo: self.lo.min(x), hi: self.hi.max(x) }
    }

    /// Whether the interval is one value.
    #[must_use]
    pub fn is_point(&self) -> bool {
        self.lo == self.hi
    }

    /// Whether `x` lies in it.
    #[must_use]
    pub fn contains(&self, x: f64) -> bool {
        x >= self.lo && x <= self.hi
    }

    /// A device-count interval as printed: whole devices, thousands separated, an en dash.
    #[must_use]
    pub fn devices(&self) -> String {
        format!("{} to {}", fmt_count(self.lo), fmt_count(self.hi))
    }
}

fn fmt_count(x: f64) -> String {
    if !x.is_finite() || x >= 1.0e9 {
        return "beyond 1G".to_owned();
    }
    let n = x.max(0.0).floor() as u64;
    let d = n.to_string();
    let mut out = String::new();
    for (i, ch) in d.chars().enumerate() {
        if i > 0 && (d.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// One input's interval and how it was derived.
#[derive(Debug, Clone, Copy)]
pub struct InputInterval {
    /// The parameter.
    pub param: C,
    /// Lower end.
    pub lo: f64,
    /// Upper end.
    pub hi: f64,
    /// How the ends were derived.
    pub how: &'static str,
}

/// What the four windows say about spread, computed from the window constants.
#[derive(Debug, Clone, Copy)]
pub struct WindowSpreads {
    /// Composer lookups per Attestation round: A, B.
    pub composer_per_round: [f64; 2],
    /// Attestation queries per withheld row: A, B.
    pub queries_per_withheld: [f64; 2],
    /// Rows withheld per Attestation round: A, B.
    pub withheld_per_round: [f64; 2],
    /// Attestation queries per Attestation round: A, B, C.
    pub queries_per_round: [f64; 3],
    /// Relative half-range of `queries_per_withheld` (the borrowed band for a ratio one window
    /// counts).
    pub ratio_band: f64,
    /// Relative half-range of `queries_per_round` (the borrowed band for the one CPU reading and
    /// the one stale-route reading: counted queries are half of window B's CPU split and vary
    /// this much between windows).
    pub load_band: f64,
    /// The other windows' responder finals per plane against window B's, lowest and highest
    /// (C's 43 occurrence finals left out as unexplained).
    pub finals_ratio: [f64; 2],
}

fn half_range(v: &[f64]) -> f64 {
    let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (hi - lo) / (hi + lo)
}

/// The spreads the windows show.
#[must_use]
pub fn window_spreads() -> WindowSpreads {
    let a_rounds = window_a::ROUNDS[0];
    let b_rounds = window_b::ROUNDS[0];
    let composer_per_round = [window_a::COMPOSER / a_rounds, window_b::COMPOSER / b_rounds];
    let queries_per_withheld = [window_a::ATT_QUERIES / window_a::WITHHOLDS, window_b::ATT_QUERIES / window_b::WITHHOLDS];
    let withheld_per_round = [window_a::WITHHOLDS / a_rounds, window_b::WITHHOLDS / b_rounds];
    let queries_per_round =
        [window_a::ATT_QUERIES / a_rounds, window_b::ATT_QUERIES / b_rounds, window_c::ATT_QUERIES / window_c::ROUNDS[0]];
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for other in [window_a::ROUNDS, window_a_prime::ROUNDS, window_c::ROUNDS] {
        for plane in 0..4 {
            // C's occurrence finals (43 against 10 to 12 elsewhere) are unexplained: left out.
            if other == window_c::ROUNDS && plane == 1 {
                continue;
            }
            let r = other[plane] / window_b::ROUNDS[plane];
            lo = lo.min(r);
            hi = hi.max(r);
        }
    }
    WindowSpreads {
        composer_per_round,
        queries_per_withheld,
        withheld_per_round,
        queries_per_round,
        ratio_band: half_range(&queries_per_withheld),
        load_band: half_range(&queries_per_round),
        finals_ratio: [lo, hi],
    }
}

/// Every input with an interval. Measured inputs first (their ends computed from the windows
/// where two windows count them, else a borrowed band named in `how`), then the derived and
/// modelled inputs the knee rests on (bands stated, MODEL). `key_publish_set_growth` is not
/// here: it is the swept axis.
#[must_use]
pub fn input_intervals(p: &CostParams) -> Vec<InputInterval> {
    let w = window_spreads();
    let band = |c: C, r: f64, how: &'static str| InputInterval { param: c, lo: p.get(c) * (1.0 - r), hi: p.get(c) * (1.0 + r), how };
    let io = p.get(C::IoSubjectsAtSeat);
    vec![
        // ---- MEASURED: two windows count it ---------------------------------
        InputInterval { param: C::ComposerPerAttRound, lo: w.composer_per_round[0].min(w.composer_per_round[1]), hi: w.composer_per_round[0].max(w.composer_per_round[1]), how: "MEASURED: window A and window B composer lookups per Attestation round" },
        InputInterval { param: C::QueriesPerRefusedRow, lo: w.queries_per_withheld[0].min(w.queries_per_withheld[1]), hi: w.queries_per_withheld[0].max(w.queries_per_withheld[1]), how: "MEASURED: window A and window B attestation queries per withheld row" },
        InputInterval { param: C::RefusedRowsPerAttRound, lo: w.withheld_per_round[0].min(w.withheld_per_round[1]), hi: w.withheld_per_round[0].max(w.withheld_per_round[1]), how: "MEASURED: window A and window B rows withheld per Attestation round" },
        InputInterval { param: C::TodayRoundsScale, lo: w.finals_ratio[0], hi: w.finals_ratio[1], how: "MEASURED: the other windows' responder finals per plane against window B's (C's 43 occurrence finals left out)" },
        // ---- MEASURED once: a borrowed band ---------------------------------
        band(C::ConsentPerRefusedRow, w.ratio_band, "MEASURED in window A only: the relative half-range of queries per withheld row across A and B"),
        band(C::RowsPerAttQuery, w.ratio_band, "MEASURED in window A only: the relative half-range of queries per withheld row across A and B"),
        band(C::CpuWindowBCores, w.load_band, "MEASURED in window B only (the one CPU window): the relative half-range of attestation queries per Attestation round across A, B and C"),
        band(C::StaleRoutesPerS, w.load_band, "MEASURED once: the relative half-range of attestation queries per Attestation round across A, B and C"),
        band(C::ServerRefreshCores, 0.5, "MEASURED-CONTEXT, approximate (~10% of a core): +-50%"),
        band(C::AnnouncedNodes, 0.02, "MEASURED once (one snapshot; drift over the day not observed): +-2%, MODEL band"),
        band(C::AnnouncedOwners, 0.02, "MEASURED once: +-2%, MODEL band"),
        band(C::SeatAttRows, 0.02, "MEASURED once (window A): +-2%, MODEL band"),
        band(C::SeatTdRows, 0.02, "MEASURED once (window A): +-2%, MODEL band"),
        band(C::SeatKeyRows, 0.02, "MEASURED once (window A): +-2%, MODEL band"),
        InputInterval { param: C::PullCallsPerCycle, lo: 8.0, hi: 12.0, how: "MEASURED (debug) on one fixture store: one call per leg pair either way, MODEL band" },
        InputInterval { param: C::PullPermitsPerCycle, lo: 8.0, hi: 8.0, how: "MEASURED (debug), a point: a count of code-path legs (self and seat on four planes, runtime.rs cycle :1574), not a sample; a build does not change it" },
        band(C::PullReplyBytesPerCycle, 0.2, "MEASURED (debug) on one fixture: +-20%, MODEL band"),
        InputInterval { param: C::DrainPageKeyS, lo: 0.25 * p.get(C::DrainPageKeyS), hi: p.get(C::DrainPageKeyS), how: "MEASURED (debug wall time): a release build is faster, by up to 4x (MODEL)" },
        InputInterval { param: C::DrainPageIoS, lo: 0.25 * p.get(C::DrainPageIoS), hi: p.get(C::DrainPageIoS), how: "MEASURED (debug wall time): up to 4x faster in release (MODEL)" },
        InputInterval { param: C::DrainPageRevS, lo: 0.25 * p.get(C::DrainPageRevS), hi: p.get(C::DrainPageRevS), how: "MEASURED (debug wall time): up to 4x faster in release (MODEL)" },
        InputInterval { param: C::DrainPageIorS, lo: 0.25 * p.get(C::DrainPageIorS), hi: p.get(C::DrainPageIorS), how: "MEASURED (debug wall time): up to 4x faster in release (MODEL)" },
        // ---- DERIVED and MODEL inputs the knee rests on -----------------------
        InputInterval { param: C::GlobalSubjectsAtSeat, lo: p.get(C::KeySubjectsAtSeat), hi: p.get(C::SeatKeyRows), how: "DERIVED: own ∪ cohort lies between the Key publish set (8,211) and every key held (8,698)" },
        InputInterval { param: C::IoSubjectsAtSeat, lo: 5_600.0, hi: 5_700.0, how: "DERIVED: ~5.6k to 5.7k (withhold-rate's source answer)" },
        InputInterval { param: C::StoreReadS, lo: 1.0e-3, hi: 3.0e-3, how: "DERIVED ~2 ms from a key-plane refill, not measured on a Pull: 1 to 3 ms, MODEL band" },
        InputInterval { param: C::WarmListingS, lo: 0.1e-3, hi: 0.4e-3, how: "MODEL: a cache hit's set comparison at 8,211 subjects, 0.1 to 0.4 ms" },
        InputInterval { param: C::KeyPlaneWalkPerSubjectS, lo: 3.0e-3 / 8_211.0, hi: 6.0e-3 / 8_211.0, how: "DERIVED: the warm Key listing's 3 to 6 ms over 8,211 subjects" },
        InputInterval { param: C::SubjectSetBuildPerSubjectS, lo: 0.05e-6, hi: 0.3e-6, how: "MODEL: building own ∪ cohort as a HashSet<String>, 0.05 to 0.3 µs a subject" },
        InputInterval { param: C::ProbeReadS, lo: 0.2e-3, hi: 1.0e-3, how: "MODEL: one probe page, 0.2 to 1 ms" },
        InputInterval { param: C::ReachUnownedShare, lo: p.get(C::AnnouncedOwners) / io, hi: 1.0 - p.get(C::AnnouncedNodes) / io, how: "DERIVED bounds: owners / occurrence subjects (an identity key has no owner) to 1 - announced nodes / occurrence subjects (an announced node is owned)" },
        InputInterval { param: C::ReachReadsPerUnownedSubject, lo: 1.0, hi: 2.0, how: "DERIVED from unowned_reach: one read always, up to four when another identity holds the node (the own set is shared per identity)" },
        InputInterval { param: C::AnnounceMemoClearsPerHour, lo: 0.0, hi: 60.0, how: "MODEL: ownership events and host local-write doors at the seat, 0 to 60 an hour (server-side, not measured)" },
        InputInterval { param: C::KeyRecordWritesPerDeviceDay, lo: 0.01, hi: 1.0, how: "MODEL: enrolment to a daily re-publish" },
        InputInterval { param: C::ClaimedShare, lo: 0.25, hi: 0.75, how: "MODEL: share of bootstrap devices with an owner" },
        InputInterval { param: C::OwnerLegPermitShare, lo: 0.0, hi: 1.0, how: "MODEL: owner legs answered in the entitled arm (a mutual trust root) or refused" },
        InputInterval { param: C::HybridVerifyCpuS, lo: 1.0e-3, hi: 3.0e-3, how: "MODEL: not profiled" },
        InputInterval { param: C::RoundOverheadCpuS, lo: 0.1e-3, hi: 0.5e-3, how: "MODEL: frame handling per Summary" },
        InputInterval { param: C::IoQueriesPerSubject, lo: 1.0, hi: 2.0, how: "DERIVED: 1 to 2 reads per occurrence subject" },
        InputInterval { param: C::IorQueriesPerSubject, lo: 4.0, hi: 6.0, how: "MODEL: 4 to 6 reads per occurrence-withdrawal subject" },
        InputInterval { param: C::AttListingPermits, lo: 2.0, hi: 4.0, how: "DERIVED: 2 to 4 pages per Attestation listing" },
        InputInterval { param: C::TdListingPermits, lo: 1.0, hi: 2.0, how: "MODEL: 1 to 2 pages per route listing" },
        InputInterval { param: C::AttRequiredQueries, lo: 10.0, hi: 20.0, how: "DERIVED: 10 to 20 counted queries per required read" },
        InputInterval { param: C::RowBytes, lo: 675.0, hi: 954.0, how: "DERIVED: the seat-1 profile's 675 to 954 B a row" },
        InputInterval { param: C::HashRefBytes, lo: 64.0, hi: 80.0, how: "MODEL: 64 hex chars plus framing" },
        InputInterval { param: C::CallOverheadBytes, lo: 200.0, hi: 800.0, how: "MODEL: a 0-row call's cost" },
        InputInterval { param: C::NewRowsPerDeviceDay, lo: 10.0, hi: 40.0, how: "MODEL" },
        InputInterval { param: C::RuntimeStartsPerDeviceDay, lo: 1.0, hi: 12.0, how: "MODEL" },
        InputInterval { param: C::PullPlaneWritesPerDeviceDay, lo: 2.0, hi: 30.0, how: "MODEL" },
        InputInterval { param: C::FirstContactCycles, lo: 1.0, hi: 6.0, how: "MODEL, bounded by BUNDLE_FIRST_CONTACT_TRIES" },
    ]
}

/// `base` with `param` set to `x`, the CPU parameter refitted (window B).
#[must_use]
pub fn with_value(base: &CostParams, param: C, x: f64) -> CostParams {
    let mut q = base.clone();
    q.set(param, x);
    let c = calibrate(&q).cpu_per_byte_s;
    q.set(C::CpuPerByte, c);
    q
}

/// An output's interval over the input intervals, by the vertex method, and the parameter sets
/// at its two ends.
#[derive(Debug, Clone)]
pub struct Propagated {
    /// The output's interval.
    pub interval: Interval,
    /// Inputs at the ends that give `interval.lo`.
    pub low_corner: CostParams,
    /// Inputs at the ends that give `interval.hi`.
    pub high_corner: CostParams,
}

/// Propagate the input intervals of `p` (all but the growth axis) to `f`.
pub fn propagate(p: &CostParams, inputs: &[InputInterval], f: impl Fn(&CostParams) -> f64) -> Propagated {
    let mut low = p.clone();
    let mut high = p.clone();
    let mut interval = Interval::new(f(p), f(p));
    for i in inputs {
        let at_lo = f(&with_value(p, i.param, i.lo));
        let at_hi = f(&with_value(p, i.param, i.hi));
        interval = interval.with(at_lo).with(at_hi);
        // the end that lowers the output goes to the low corner (ties: lo to low)
        let (lo_end, hi_end) = if at_lo <= at_hi { (i.lo, i.hi) } else { (i.hi, i.lo) };
        low.set(i.param, lo_end);
        high.set(i.param, hi_end);
    }
    let refit = |mut q: CostParams| {
        let c = calibrate(&q).cpu_per_byte_s;
        q.set(C::CpuPerByte, c);
        q
    };
    let low = refit(low);
    let high = refit(high);
    interval = interval.with(f(&low)).with(f(&high));
    Propagated { interval, low_corner: low, high_corner: high }
}

/// A knee with its interval and what binds at each end.
#[derive(Debug, Clone)]
pub struct KneeInterval {
    /// Devices, PROJECTED.
    pub devices: Interval,
    /// What binds at the low end.
    pub binds_low: Binding,
    /// What binds at the high end.
    pub binds_high: Binding,
}

/// The three terms the knee is the minimum of: cores, sweep permits, the busiest key-plane lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Term {
    /// CPU utilisation.
    Cpu,
    /// Sweep-permit utilisation.
    Permits,
    /// The busiest key-plane lock's utilisation.
    Lock,
}

/// The largest fleet whose `term` alone stays under the ceiling (bisection, as
/// [`capacity::knee`]; `capacity::knee` is the minimum of the three).
#[must_use]
pub fn term_knee(p: &CostParams, base: &Scenario, term: Term) -> f64 {
    let ceiling = p.get(C::CpuCeiling);
    let ok = |d: f64| {
        let mut s = *base;
        s.devices = d;
        let l = run(p, &s).seat_load;
        let rho = match term {
            Term::Cpu => l.rho_cpu,
            Term::Permits => l.rho_permit,
            Term::Lock => l.rho_lock,
        };
        rho <= ceiling
    };
    if !ok(1.0) {
        return 0.0;
    }
    let (mut lo, mut hi) = (1.0_f64, 1.0e9_f64);
    if ok(hi) {
        return hi;
    }
    for _ in 0..80 {
        let mid = (lo * hi).sqrt();
        if ok(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi / lo < 1.001 {
            break;
        }
    }
    lo.floor()
}

/// The knee of `sc` over every input interval of `p` (growth held at `p`'s value).
///
/// The knee is the minimum of three term knees, and the minimum switches term across the input
/// box, so the vertex method run on the knee itself is not a bound (it put g = 1's upper end
/// above g = 0.75's). Each term's knee is propagated alone, where it is monotone in each input,
/// and the knee's interval is `[min_t lo_t, min_t hi_t]`: for every input point the knee is at
/// least the smallest term lower end and at most every term's upper end. An outer bound, not a
/// probability.
#[must_use]
pub fn knee_interval(p: &CostParams, sc: &Scenario) -> KneeInterval {
    let inputs = input_intervals(p);
    let mut lo: Option<(f64, Term, CostParams)> = None;
    let mut hi: Option<(f64, Term, CostParams)> = None;
    for t in [Term::Cpu, Term::Permits, Term::Lock] {
        let pr = propagate(p, &inputs, |q| term_knee(q, sc, t));
        if lo.as_ref().is_none_or(|x| pr.interval.lo < x.0) {
            lo = Some((pr.interval.lo, t, pr.low_corner.clone()));
        }
        if hi.as_ref().is_none_or(|x| pr.interval.hi < x.0) {
            hi = Some((pr.interval.hi, t, pr.high_corner.clone()));
        }
    }
    let (lo, hi) = (lo.expect("three terms"), hi.expect("three terms"));
    let bind = |t: Term, q: &CostParams, d: f64| match t {
        Term::Cpu => Binding::Cpu,
        Term::Permits => Binding::Permits,
        Term::Lock => {
            let mut s = *sc;
            s.devices = d.max(1.0);
            Binding::Lock(run(q, &s).seat.busiest_plane())
        }
    };
    KneeInterval {
        devices: Interval::new(lo.0, hi.0),
        binds_low: bind(lo.1, &lo.2, lo.0),
        binds_high: bind(hi.1, &hi.2, hi.0),
    }
}

/// The growth axis the knee is swept over (FRONTIER: the server's `key_publish_set` rule).
pub const GROWTH_AXIS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// One row of [`knee_by_growth`].
#[derive(Debug, Clone)]
pub struct GrowthRow {
    /// `key_publish_set_growth`.
    pub growth: f64,
    /// v45.0.0, no old peers, the seat names no device.
    pub knee: KneeInterval,
    /// The same with a quarter of the devices on v44.
    pub old_quarter: Interval,
    /// The same with the seat initiating to 1% of devices.
    pub initiator_1pct: Interval,
}

/// v45.0.0's knee (8e46165, one seat) against `key_publish_set` growth, PROJECTED.
#[must_use]
pub fn knee_by_growth(p: &CostParams) -> Vec<GrowthRow> {
    GROWTH_AXIS
        .iter()
        .map(|&g| {
            let q = with_value(p, C::KeyPublishSetGrowth, g);
            let sc = Scenario::v45(1.0);
            let old = with_value(&q, C::OldPeerShare, 0.25);
            let init = with_value(&q, C::SeatInitiatorDeviceShare, 0.01);
            GrowthRow {
                growth: g,
                knee: knee_interval(&q, &sc),
                old_quarter: knee_interval(&old, &sc).devices,
                initiator_1pct: knee_interval(&init, &sc).devices,
            }
        })
        .collect()
}

/// The growth table, markdown.
#[must_use]
pub fn growth_table(p: &CostParams) -> String {
    let mut s = String::from(
        "| key_publish_set growth g (FRONTIER) | v45.0.0 knee, devices (PROJECTED) | binds at the low end | binds at the high end | with 25% of devices on v44 (PROJECTED) | seat initiating to 1% of devices (PROJECTED) |\n|---|---|---|---|---|---|\n",
    );
    for r in knee_by_growth(p) {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.growth,
            r.knee.devices.devices(),
            r.knee.binds_low.label(),
            r.knee.binds_high.label(),
            r.old_quarter.devices(),
            r.initiator_1pct.devices(),
        ));
    }
    s
}

/// The input-interval table, markdown.
#[must_use]
pub fn input_table(p: &CostParams) -> String {
    let mut s = String::from("| input | point | interval | how |\n|---|---|---|---|\n");
    for i in input_intervals(p) {
        let name = capacity::COST_SPECS[i.param as usize].name;
        s.push_str(&format!("| `{name}` | {} | {} to {} | {} |\n", sig(p.get(i.param)), sig(i.lo), sig(i.hi), i.how));
    }
    s
}

fn sig(x: f64) -> String {
    let a = x.abs();
    if a == 0.0 {
        "0".to_owned()
    } else if a < 0.01 {
        format!("{x:.3e}")
    } else if a < 10.0 {
        format!("{x:.3}")
    } else {
        format!("{x:.1}")
    }
}

// ---------------------------------------------------------------------------
// The fit split
// ---------------------------------------------------------------------------

/// A measured window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowId {
    /// Window A (store doors).
    A,
    /// Window B (quiet, the one CPU window).
    B,
    /// Window A' (permits).
    APrime,
    /// Window C (permits, attestation queries).
    C,
    /// The 64-minute busy reading (approximate).
    BusyHour,
}

/// A counter in a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counter {
    /// Process CPU, cores.
    Cpu,
    /// attestation_query calls.
    AttQueries,
    /// Rows attestation_query returned.
    AttRows,
    /// list_composers_referencing_any calls.
    Composer,
    /// list_live_consent_grants_by calls.
    Consent,
    /// recipient_not_in_send_set withholds.
    Withholds,
    /// Sweep permits.
    Permits,
    /// Attestation responder finals.
    AttRounds,
    /// Occurrence responder finals.
    IoRounds,
    /// Key responder finals.
    KeyRounds,
    /// Route responder finals.
    RouteRounds,
}

/// One observation.
#[derive(Debug, Clone, Copy)]
pub struct Obs {
    /// Window.
    pub window: WindowId,
    /// Counter.
    pub counter: Counter,
    /// Value (per minute for counts, cores for CPU).
    pub value: f64,
}

/// Every aggregate the windows hold (from [`capacity`]'s window modules).
#[must_use]
pub fn observations() -> Vec<Obs> {
    use Counter as K;
    use WindowId as W;
    let o = |window, counter, value| Obs { window, counter, value };
    let rounds = |w, r: [f64; 4]| [o(w, K::AttRounds, r[0]), o(w, K::IoRounds, r[1]), o(w, K::KeyRounds, r[2]), o(w, K::RouteRounds, r[3])];
    let mut v = vec![
        o(W::A, K::AttQueries, window_a::ATT_QUERIES),
        o(W::A, K::AttRows, window_a::ATT_ROWS),
        o(W::A, K::Composer, window_a::COMPOSER),
        o(W::A, K::Consent, window_a::CONSENT),
        o(W::A, K::Withholds, window_a::WITHHOLDS),
        o(W::B, K::Cpu, window_b::CPU_CORES),
        o(W::B, K::AttQueries, window_b::ATT_QUERIES),
        o(W::B, K::Composer, window_b::COMPOSER),
        o(W::B, K::Permits, window_b::PERMITS),
        o(W::B, K::Withholds, window_b::WITHHOLDS),
        o(W::APrime, K::Permits, window_a_prime::PERMITS),
        o(W::C, K::Permits, window_c::PERMITS),
        o(W::C, K::AttQueries, window_c::ATT_QUERIES),
        o(W::BusyHour, K::Cpu, busy_hour::CPU_CORES),
    ];
    v.extend(rounds(W::A, window_a::ROUNDS));
    v.extend(rounds(W::B, window_b::ROUNDS));
    v.extend(rounds(W::APrime, window_a_prime::ROUNDS));
    v.extend(rounds(W::C, window_c::ROUNDS));
    v
}

/// What the fit may read: the door ratios from window A, the CPU (and the round mix it is
/// fitted at) from window B. Nothing else.
pub const FIT: &[(WindowId, Counter)] = &[
    (WindowId::A, Counter::AttQueries),
    (WindowId::A, Counter::AttRows),
    (WindowId::A, Counter::Composer),
    (WindowId::A, Counter::Consent),
    (WindowId::A, Counter::Withholds),
    (WindowId::A, Counter::AttRounds),
    (WindowId::B, Counter::Cpu),
    (WindowId::B, Counter::AttRounds),
    (WindowId::B, Counter::IoRounds),
    (WindowId::B, Counter::KeyRounds),
    (WindowId::B, Counter::RouteRounds),
    (WindowId::B, Counter::Withholds),
];

/// What the fit must predict and may not read.
pub const HELD_OUT: &[(WindowId, Counter)] = &[
    (WindowId::B, Counter::AttQueries),
    (WindowId::B, Counter::Composer),
    (WindowId::B, Counter::Permits),
    (WindowId::APrime, Counter::Permits),
    (WindowId::C, Counter::Permits),
    (WindowId::C, Counter::AttQueries),
    (WindowId::BusyHour, Counter::Cpu),
];

/// The only door the fit reads windows through: refuses anything outside [`FIT`] and records
/// every read.
#[derive(Debug)]
pub struct FitView<'a> {
    obs: &'a [Obs],
    /// Every (window, counter) read, in order.
    pub reads: RefCell<Vec<(WindowId, Counter)>>,
}

impl<'a> FitView<'a> {
    /// A view over `obs`.
    #[must_use]
    pub fn new(obs: &'a [Obs]) -> Self {
        Self { obs, reads: RefCell::new(Vec::new()) }
    }

    /// One fit observation.
    ///
    /// # Panics
    /// When `(window, counter)` is not in [`FIT`] (a held-out or undeclared read) or not
    /// observed.
    #[must_use]
    pub fn get(&self, window: WindowId, counter: Counter) -> f64 {
        assert!(FIT.contains(&(window, counter)), "the fit read {window:?} {counter:?}, which is not in FIT");
        self.reads.borrow_mut().push((window, counter));
        self.obs
            .iter()
            .find(|o| o.window == window && o.counter == counter)
            .unwrap_or_else(|| panic!("{window:?} {counter:?} not observed"))
            .value
    }
}

/// The parameters the windows fix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fitted {
    /// `queries_per_refused_row`.
    pub queries_per_refused_row: f64,
    /// `consent_per_refused_row`.
    pub consent_per_refused_row: f64,
    /// `rows_per_att_query`.
    pub rows_per_att_query: f64,
    /// `composer_per_att_round`.
    pub composer_per_att_round: f64,
    /// `refused_rows_per_att_round`.
    pub refused_rows_per_att_round: f64,
    /// `cpu_window_b_cores`, the input `cpu_per_byte_s` is fitted to.
    pub cpu_window_b_cores: f64,
}

/// Fit through the view.
#[must_use]
pub fn fit(v: &FitView) -> Fitted {
    use Counter as K;
    use WindowId as W;
    let withheld = v.get(W::A, K::Withholds);
    let att_rounds = v.get(W::A, K::AttRounds);
    Fitted {
        queries_per_refused_row: v.get(W::A, K::AttQueries) / withheld,
        consent_per_refused_row: v.get(W::A, K::Consent) / withheld,
        rows_per_att_query: v.get(W::A, K::AttRows) / v.get(W::A, K::AttQueries),
        composer_per_att_round: v.get(W::A, K::Composer) / att_rounds,
        refused_rows_per_att_round: withheld / att_rounds,
        cpu_window_b_cores: v.get(W::B, K::Cpu),
    }
}

/// `base` with the fitted values set and `cpu_per_byte_s` refitted at window B's round mix.
#[must_use]
pub fn fitted_params(base: &CostParams, f: &Fitted) -> CostParams {
    let mut p = base.clone();
    p.set(C::QueriesPerRefusedRow, f.queries_per_refused_row);
    p.set(C::ConsentPerRefusedRow, f.consent_per_refused_row);
    p.set(C::RowsPerAttQuery, f.rows_per_att_query);
    p.set(C::ComposerPerAttRound, f.composer_per_att_round);
    p.set(C::RefusedRowsPerAttRound, f.refused_rows_per_att_round);
    p.set(C::CpuWindowBCores, f.cpu_window_b_cores);
    let c = calibrate(&p).cpu_per_byte_s;
    p.set(C::CpuPerByte, c);
    p
}

/// One held-out prediction.
#[derive(Debug, Clone, Copy)]
pub struct HeldOut {
    /// Window.
    pub window: WindowId,
    /// Counter.
    pub counter: Counter,
    /// Predicted by the fit.
    pub predicted: f64,
    /// Measured.
    pub measured: f64,
}

impl HeldOut {
    /// (predicted - measured) / measured.
    #[must_use]
    pub fn error(&self) -> f64 {
        (self.predicted - self.measured) / self.measured
    }
}

/// Every held-out counter predicted by `p` (which should come from [`fitted_params`]).
#[must_use]
pub fn held_out(p: &CostParams, obs: &[Obs]) -> Vec<HeldOut> {
    use Counter as K;
    use WindowId as W;
    let measured = |w, k| obs.iter().find(|o| o.window == w && o.counter == k).map_or(f64::NAN, |o| o.value);
    let c = calibrate(p);
    let b = ledger(p, &measured_mix(p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS), ReadSet::AsShipped);
    // C has no withhold count: its rows withheld per round are the fit's (window A).
    let c_withheld = p.get(C::RefusedRowsPerAttRound) * window_c::ROUNDS[0];
    let cl = ledger(p, &measured_mix(p, window_c::ROUNDS, window_c::REV_ROUNDS, c_withheld), ReadSet::AsShipped);
    let mut v = Vec::new();
    for &(w, k) in HELD_OUT {
        let predicted = match (w, k) {
            (W::B, K::AttQueries) => b.att_queries_per_s * 60.0,
            (W::B, K::Composer) => b.composer_per_s * 60.0,
            (W::B, K::Permits) => c.permits_pred[0],
            (W::APrime, K::Permits) => c.permits_pred[1],
            (W::C, K::Permits) => c.permits_pred[2],
            (W::C, K::AttQueries) => cl.att_queries_per_s * 60.0,
            (W::BusyHour, K::Cpu) => c.busy_cpu_pred,
            _ => f64::NAN,
        };
        v.push(HeldOut { window: w, counter: k, predicted, measured: measured(w, k) });
    }
    v
}

/// The split and the held-out errors, markdown.
#[must_use]
pub fn held_out_table(p: &CostParams) -> String {
    let obs = observations();
    let mut s = String::from(
        "Fit on: window A's store doors (the door ratios) and window B's CPU at window B's round mix (`cpu_per_byte_s`). Held out: everything below. With four 60 s windows of one seat, a quiet one among them, this is a thin check: seven held-out counters, five of them permit or query counts the fit has no free parameter for, so a held-out error here says the model is missing a term, not that a parameter is mis-set.\n\n| window | counter | predicted | measured | held-out error |\n|---|---|---|---|---|\n",
    );
    for h in held_out(p, &obs) {
        s.push_str(&format!(
            "| {:?} | {:?} | {} | {} | {:+.1}% |\n",
            h.window,
            h.counter,
            sig(h.predicted),
            sig(h.measured),
            h.error() * 100.0
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// Residuals
// ---------------------------------------------------------------------------

/// The 12 planes v44.1.2 lists through `sweep_paged` beside the four the windows count finals
/// on ("the 12 planes whose per-row projection is pure", v44.1.2 bridge.rs :3860).
pub const SWEEP_PAGED_PLANES: f64 = 12.0;

/// Inbound links in each permit window (MEASURED): B, A', C.
pub const INBOUND_LINKS: [f64; 3] = [15.0, 18.0, 20.0];

/// One window's permits, term by term.
#[derive(Debug, Clone)]
pub struct Residual {
    /// Window label.
    pub window: &'static str,
    /// Measured permits per minute.
    pub measured: f64,
    /// Modelled terms per minute: (name, value).
    pub terms: Vec<(&'static str, f64)>,
    /// Of the terms, what the 2026-11-11 corrections added.
    pub closed: f64,
    /// Measured less every modelled term (UNEXPLAINED).
    pub left: f64,
    /// Rounds on the twelve uncounted planes at one or two permits each, if every inbound link
    /// rounded on every one of them every cadence: the most hypothesis H2 could supply.
    pub h2_max: f64,
    /// Whole index drains (42 pages) the residual is worth (H4, H7).
    pub drains_worth: f64,
}

/// The three permit windows decomposed.
#[must_use]
pub fn residuals(p: &CostParams) -> [Residual; 3] {
    let windows = [
        ("B", window_b::ROUNDS, window_b::PERMITS),
        ("A'", window_a_prime::ROUNDS, window_a_prime::PERMITS),
        ("C", window_c::ROUNDS, window_c::PERMITS),
    ];
    let pa = p.pages(p.get(C::SeatAttRows));
    let pt = p.pages(p.get(C::SeatTdRows));
    let per_min = 60.0 / p.get(C::CadenceS);
    let mut out = Vec::new();
    for (i, (name, r, measured)) in windows.into_iter().enumerate() {
        let l = ledger(p, &measured_mix(p, r, 0.0, 0.0), ReadSet::AsShipped);
        let att_drains = (r[0] + r[1] + r[2]) * pa;
        let td_drains = r[3] * pt;
        let att_listing = r[0] * (p.get(C::AttListingPermits) + p.get(C::RequiredRowsPermits));
        let td_listing = r[3] * p.get(C::TdListingPermits);
        let probes = l.probe_reads_per_s * 60.0;
        let send_set = l.send_set_permits_per_s * 60.0;
        let terms = vec![
            ("index drains on the Attestation index (Key, occurrence, Attestation finals x 42 pages)", att_drains),
            ("index drains on the route index (route finals x 18 pages)", td_drains),
            ("Attestation listing and required-rows pages", att_listing),
            ("route listing pages", td_listing),
            ("key-plane change probes (one per Key and occurrence listing)", probes),
            ("consent send-set reads (>= one per 10 s memo; ADDED 2026-11-11)", send_set),
        ];
        let model: f64 = terms.iter().map(|t| t.1).sum();
        debug_assert!((model - l.permits_per_s * 60.0).abs() < 1e-6);
        let left = measured - model;
        out.push(Residual {
            window: name,
            measured,
            terms,
            closed: send_set,
            left,
            h2_max: INBOUND_LINKS[i] * SWEEP_PAGED_PLANES * per_min * 2.0,
            drains_worth: left / pa,
        });
    }
    [out[0].clone(), out[1].clone(), out[2].clone()]
}

/// The residual decomposition and its hypotheses, markdown.
#[must_use]
pub fn residual_report(p: &CostParams) -> String {
    let r = residuals(p);
    let t = capacity::ttl_check(p);
    let mut s = String::from("| term (permits a minute) | B | A' | C |\n|---|---|---|---|\n");
    for k in 0..r[0].terms.len() {
        s.push_str(&format!("| {} | {:.1} | {:.1} | {:.1} |\n", r[0].terms[k].0, r[0].terms[k].1, r[1].terms[k].1, r[2].terms[k].1));
    }
    let model = |x: &Residual| x.terms.iter().map(|t| t.1).sum::<f64>();
    s.push_str(&format!("| **model** | {:.1} | {:.1} | {:.1} |\n", model(&r[0]), model(&r[1]), model(&r[2])));
    s.push_str(&format!("| **measured** | {:.0} | {:.0} | {:.0} |\n", r[0].measured, r[1].measured, r[2].measured));
    s.push_str(&format!("| **left, UNEXPLAINED** | {:.1} | {:.1} | {:.1} |\n", r[0].left, r[1].left, r[2].left));
    s.push_str(&format!(
        "| closed by the 2026-11-11 corrections | {:.1} ({:.1}%) | {:.1} ({:.1}%) | {:.1} ({:.1}%) |\n",
        r[0].closed,
        r[0].closed / (r[0].left + r[0].closed) * 100.0,
        r[1].closed,
        r[1].closed / (r[1].left + r[1].closed) * 100.0,
        r[2].closed,
        r[2].closed / (r[2].left + r[2].closed) * 100.0,
    ));
    s.push_str(&format!(
        "\nThe residual is the measured count less every term above; no constant absorbs it. Candidate causes, each a permit site v44.1.2 has that the model does not book, with the v45 counter that settles it (8e46165 observability.rs labels the sweep-permit counter by site):\n\n\
- **H1, a TTL refill burst** (v44.1.2 cleared the memo every 10 min and the next listing read every subject in one burst under the lock). A Key burst is {key:.0} permits, an occurrence burst {io0:.0} to {io1:.0}. A' ({a:.0}) and C ({c:.0}) are the size of part of a burst; B ({b:.0}) is not. One 60 s window in {win:.0} per listed plane holds one, so two of three is more than chance predicts. Settled by `sweep_permits.key_plane_subject` and `key_plane_sweep.rebuilds_ttl`. At 8e46165 the TTL revalidates at most {k:.0} reads a listing, so a v45 window cannot show this burst.\n\
- **H2, rounds on the planes the windows did not count.** v44.1.2 lists twelve more planes through `sweep_paged`, one or two permits a listing; the windows counted finals on four. If every inbound link rounded on all twelve every cadence: at most {h2b:.0} (B, {lb} links), {h2a:.0} (A', {la}), {h2c:.0} (C, {lc}) a minute. That could close B; it cannot close A' or C alone. Settled by `sweep_permits.plane_page` and responder finals on all planes.\n\
- **H3, subject Pulls answered for today's peers** (`subject_pull_answer`, one permit each). Settled by `sweep_permits.subject_pull` and `announced_testimony`.\n\
- **H4, the seat's own initiator rounds.** A reply Summary drains the whole index as shipped: the residual is worth {db:.1} (B), {da:.1} (A'), {dc:.1} (C) whole drains of {pages:.0} pages a minute. Settled by the `initiator_peers` gauges and initiator finals beside responder finals.\n\
- **H5, consent send-set reads** beyond the 6 a minute the memo floor gives (each invalidation reads again). Settled by `sweep_permits.send_set`.\n\
- **H6, accord-evidence pages** (`accord_evidence_since`, one permit). Settled by `sweep_permits.accord_evidence`.\n\
- **H7, Summaries that drain but never reach a final** (a round that times out or aborts after its Summary drained). Settled by received Summaries per plane beside `sweep_permits.holdings_index` / 42.\n\n\
None is fitted. The 43 occurrence finals in C (against 10 to 12 elsewhere) are unexplained too.\n",
        key = t.key_burst,
        io0 = t.io_burst[0],
        io1 = t.io_burst[1],
        a = r[1].left,
        c = r[2].left,
        b = r[0].left,
        win = 1.0 / t.window_share,
        k = p.get(C::ListingBudgetK),
        h2b = r[0].h2_max,
        h2a = r[1].h2_max,
        h2c = r[2].h2_max,
        lb = INBOUND_LINKS[0],
        la = INBOUND_LINKS[1],
        lc = INBOUND_LINKS[2],
        db = r[0].drains_worth,
        da = r[1].drains_worth,
        dc = r[2].drains_worth,
        pages = p.pages(p.get(C::SeatAttRows)),
    ));
    s
}

/// The aggregate measurements that would make the calibration sound, for the server session.
/// Aggregates only: no identifier, no per-device row.
pub const MEASUREMENT_REQUESTS: &[&str] = &[
    "Longer windows: at least one continuous hour of the seat's own counters at 60 s resolution (sweep permits by site label, store calls by caller label, responder AND initiator finals on every plane, received Summaries per plane, the key_plane_sweep counters, process utime+stime), so a 10-minute TTL cycle is seen whole six times.",
    "A busy hour and a quiet hour: the same counters in the roll hour and overnight, to separate load-proportional terms from fixed ones and to give the CPU fit a second point (one CPU window cannot split page cost from query cost).",
    "The release build: the seat runs v45 in release; with it, sweep_permit_wait_seconds sum and count per site, key_plane_sweep_ms, listing-cache hits, and the unread and stale gauges per plane (the Pull's permit hold and the cache hit's lock hold are MODEL until then).",
    "Per-term counters for the residuals: sweep_permits by site (holdings_index, plane_page, subject_pull, announced_testimony, key_plane_subject, feed_attestation, feed_route, send_set, advertise_page, advertise_required, accord_evidence), the initiator_peers gauges (host, bundle, homed), and the reach evaluations (announce_reads) per minute.",
    "Store composition at the start and end of each window: attestation, route and key row counts, announced nodes and owners, and the size of the host's key publish set and of own ∪ cohort (the Global set).",
    "The server's rule for key_publish_set: what it includes and how it grows as devices join (it decides the knee between its ends, see the growth table).",
    "Ownership events and host local-write doors per hour at the seat (each clears the reach memo), and key:record rows landing per hour.",
    "A second seat, if one is ever run: the same aggregates, to tell a seat property from a seat-1 property.",
];

/// The whole calibration section, markdown.
#[must_use]
pub fn report(p: &CostParams) -> String {
    let mut s = String::new();
    s.push_str("### Fit split and held-out error\n\n");
    s.push_str(&held_out_table(p));
    s.push_str("\n### Permit residuals, term by term\n\n");
    s.push_str(&residual_report(p));
    s.push_str("\n### v45.0.0 knee against key_publish_set growth (PROJECTED)\n\n");
    s.push_str(&format!(
        "One seat of {} cores, ceiling {:.0}%, edge {} (release/45.0.0, untagged when read). Each knee is an outer bound over every input interval below (each term's knee propagated alone by the vertex method, then the smallest lower and the smallest upper end), at one value of the growth axis; no single device count stands. 'Binds' names the term that reaches the ceiling first at each end.\n\n",
        p.get(C::SeatCores),
        p.get(C::CpuCeiling) * 100.0,
        edge_971::COMMIT
    ));
    s.push_str(&growth_table(p));
    s.push_str("\n### Input intervals\n\n");
    s.push_str(&input_table(p));
    s.push_str("\n### Measurements that would make it sound (aggregates only)\n\n");
    for m in MEASUREMENT_REQUESTS {
        s.push_str(&format!("- {m}\n"));
    }
    s
}

/// Seat CPU cores at `devices` for `sc`, as an interval over the inputs.
#[must_use]
pub fn cores_interval(p: &CostParams, sc: &Scenario, devices: f64) -> Interval {
    let mut s = *sc;
    s.devices = devices;
    propagate(p, &input_intervals(p), |q| run(q, &s).seat_load.cpu_cores).interval
}

/// Plane names, for tests and reports.
#[must_use]
pub const fn plane_label(p: Plane) -> &'static str {
    p.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capacity::knee;

    #[test]
    fn fit_and_held_out_do_not_overlap() {
        for h in HELD_OUT {
            assert!(!FIT.contains(h), "{h:?} is both fit and held out");
        }
    }

    /// The fit reads only FIT pairs (the view refuses the rest), and a held-out value changed
    /// to anything leaves every fitted parameter where it was.
    #[test]
    fn a_fitted_parameter_may_not_read_a_held_out_window() {
        let obs = observations();
        let view = FitView::new(&obs);
        let f = fit(&view);
        for r in view.reads.borrow().iter() {
            assert!(FIT.contains(r), "read {r:?}");
            assert!(!HELD_OUT.contains(r), "read held out {r:?}");
        }
        let mut moved = obs.clone();
        for o in &mut moved {
            if HELD_OUT.contains(&(o.window, o.counter)) {
                o.value *= 7.0;
            }
        }
        assert_eq!(fit(&FitView::new(&moved)), f);
        let base = CostParams::default();
        assert_eq!(fitted_params(&base, &f).v, fitted_params(&base, &fit(&FitView::new(&moved))).v);
    }

    #[test]
    #[should_panic(expected = "not in FIT")]
    fn the_view_refuses_a_held_out_read() {
        let obs = observations();
        let _ = FitView::new(&obs).get(WindowId::C, Counter::Permits);
    }

    /// The defaults the projection runs on are the fit's values, not a fit over every window.
    #[test]
    fn the_defaults_are_the_fit() {
        let obs = observations();
        let f = fit(&FitView::new(&obs));
        let d = CostParams::default();
        assert_eq!(d.get(C::QueriesPerRefusedRow), f.queries_per_refused_row);
        assert_eq!(d.get(C::ConsentPerRefusedRow), f.consent_per_refused_row);
        assert_eq!(d.get(C::RowsPerAttQuery), f.rows_per_att_query);
        assert!((d.get(C::ComposerPerAttRound) - f.composer_per_att_round).abs() < 1e-9);
        assert_eq!(d.get(C::RefusedRowsPerAttRound), f.refused_rows_per_att_round);
        assert_eq!(d.get(C::CpuWindowBCores), f.cpu_window_b_cores);
        let p = fitted_params(&d, &f);
        assert!((p.get(C::CpuPerByte) - CostParams::calibrated().get(C::CpuPerByte)).abs() < 1e-18);
    }

    /// The held-out errors, pinned: a change to the model moves them visibly. The pin is the
    /// value, not a tolerance band around a target.
    #[test]
    fn held_out_errors_are_reported_per_counter() {
        let obs = observations();
        let p = fitted_params(&CostParams::default(), &fit(&FitView::new(&obs)));
        let h = held_out(&p, &obs);
        assert_eq!(h.len(), HELD_OUT.len());
        let err = |w, k| h.iter().find(|x| x.window == w && x.counter == k).unwrap().error();
        for x in &h {
            assert!(x.predicted.is_finite() && x.measured.is_finite(), "{x:?}");
            eprintln!("held out {:?} {:?}: {:.1} vs {:.1} ({:+.1}%)", x.window, x.counter, x.predicted, x.measured, x.error() * 100.0);
        }
        // window B's composer is now held out: A's ratio predicts 3,867 of 4,512
        assert!((err(WindowId::B, Counter::Composer) - (3_867.0 / 4_512.0 - 1.0)).abs() < 1e-9);
        // the permit windows stay far under: the model is missing terms
        assert!(err(WindowId::B, Counter::Permits) < -0.25);
        assert!(err(WindowId::APrime, Counter::Permits) < -0.7);
        assert!(err(WindowId::C, Counter::Permits) < -0.55);
        // window C's queries, from A's rows withheld per round, over-predict
        assert!(err(WindowId::C, Counter::AttQueries) > 0.3);
        assert!(held_out_table(&p).contains("held-out error"));
    }

    #[test]
    fn interval_derivations_come_from_the_windows() {
        let w = window_spreads();
        assert!((w.composer_per_round[0] - 386.7).abs() < 1e-9 && (w.composer_per_round[1] - 451.2).abs() < 1e-9);
        assert!(w.ratio_band > 0.03 && w.ratio_band < 0.05, "{}", w.ratio_band);
        assert!(w.load_band > 0.15 && w.load_band < 0.2, "{}", w.load_band);
        assert!(w.finals_ratio[0] < 1.0 && w.finals_ratio[1] > 1.0, "{:?}", w.finals_ratio);
        let p = CostParams::calibrated();
        for i in input_intervals(&p) {
            assert!(i.lo <= i.hi, "{:?}", i.param);
            assert!(!i.how.is_empty());
            let spec = &capacity::COST_SPECS[i.param as usize];
            // every measured input listed has a derivation saying how
            if matches!(spec.tag, capacity::CTag::Measured) {
                assert!(i.how.starts_with("MEASURED"), "{}", spec.name);
            }
        }
        // every MEASURED parameter that the cost model reads has an interval
        let listed: Vec<C> = input_intervals(&p).iter().map(|i| i.param).collect();
        for (k, spec) in capacity::COST_SPECS.iter().enumerate() {
            if matches!(spec.tag, capacity::CTag::Measured | capacity::CTag::MeasuredDebug) {
                let named = listed.iter().any(|c| *c as usize == k);
                let exempt = matches!(
                    spec.name,
                    // a kept-for-the-table ratio and exact counts read at the seat
                    "composer_per_refused_row" | "rev_plane_clears_per_hour" | "ior_plane_clears_per_hour" | "rev_non_empty_subjects" | "ior_non_empty_subjects"
                );
                assert!(named || exempt, "{} is MEASURED and has no interval", spec.name);
            }
        }
    }

    /// An interval on an input never yields a point on an output: each knee row is an interval
    /// of non-zero width, and the store-read time alone moves the knee.
    #[test]
    fn an_interval_on_an_input_never_yields_a_point() {
        let p = CostParams::calibrated();
        let one = [InputInterval { param: C::StoreReadS, lo: 1.0e-3, hi: 3.0e-3, how: "test" }];
        let k = propagate(&p, &one, |q| knee(q, &Scenario::v45(1.0)));
        assert!(!k.interval.is_point() && k.interval.lo < k.interval.hi, "{:?}", k.interval);
        for r in knee_by_growth(&p) {
            assert!(r.knee.devices.lo < r.knee.devices.hi, "g {} {:?}", r.growth, r.knee.devices);
            assert!(r.old_quarter.lo < r.old_quarter.hi);
            assert!(r.initiator_1pct.lo < r.initiator_1pct.hi);
        }
        let c = cores_interval(&p, &Scenario::v45(1.0), 10_000.0);
        assert!(c.lo < c.hi);
    }

    /// The sweep is deterministic: the same inputs give the same table, byte for byte.
    #[test]
    fn the_sweep_is_deterministic() {
        let p = CostParams::calibrated();
        assert_eq!(growth_table(&p), growth_table(&p));
        assert_eq!(report(&p), report(&p));
    }

    /// Faster growth of the publish set never raises the knee: the knee's ends fall (or stay)
    /// as g rises, and a quarter of v44 peers never raises it.
    #[test]
    fn the_knee_falls_with_publish_set_growth() {
        let p = CostParams::calibrated();
        let rows = knee_by_growth(&p);
        for w in rows.windows(2) {
            assert!(w[1].knee.devices.hi <= w[0].knee.devices.hi * 1.001, "{} -> {}", w[0].growth, w[1].growth);
            assert!(w[1].knee.devices.lo <= w[0].knee.devices.lo * 1.001);
        }
        for r in &rows {
            assert!(r.old_quarter.hi <= r.knee.devices.hi * 1.001);
            // the outer bound holds the knee at the point inputs
            let q = with_value(&p, C::KeyPublishSetGrowth, r.growth);
            assert!(r.knee.devices.contains(knee(&q, &Scenario::v45(1.0))), "g {} {:?}", r.growth, r.knee.devices);
        }
    }

    #[test]
    fn the_residuals_are_decomposed_not_fitted() {
        let p = CostParams::calibrated();
        let r = residuals(&p);
        let c = calibrate(&p);
        for (i, x) in r.iter().enumerate() {
            let model: f64 = x.terms.iter().map(|t| t.1).sum();
            assert!((model - c.permits_pred[i]).abs() < 1e-6, "{} {} {}", x.window, model, c.permits_pred[i]);
            assert!((x.left - (x.measured - model)).abs() < 1e-9);
            assert!((x.closed - 6.0).abs() < 1e-9, "the send-set floor closes 6 a minute");
            assert!(x.left > 0.0, "still UNEXPLAINED");
        }
        // H2's ceiling covers B and covers neither A' nor C
        assert!(r[0].h2_max >= r[0].left && r[1].h2_max < r[1].left && r[2].h2_max < r[2].left);
        let s = residual_report(&p);
        assert!(s.contains("UNEXPLAINED") && s.contains("sweep_permits.plane_page"));
    }

    #[test]
    fn the_measurement_requests_name_no_identifier() {
        for m in MEASUREMENT_REQUESTS {
            assert!(!m.contains("key_id") && !m.contains("IP"), "{m}");
        }
    }
}
