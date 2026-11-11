//! The two hooks every twin package exposes (HOOKS.md), for the seat-capacity model:
//!
//! 1. **Energy counters, counts only.** Per seat and per device: rounds an hour, store reads and
//!    bytes per round, and how many of a device's rounds carry nothing. A device's round
//!    schedule is its radio wake schedule, so it is also given as wake-ups an hour per device
//!    class. No watts, no joules: those belong to the `energy` package.
//! 2. **Flat-buffer views for the globe** (`twin-globe` wraps them in kernel-wasm): the seat load
//!    against fleet size with its interval, the knee interval, and the fit's in-sample and
//!    held-out counters, per `key_publish_set` growth scenario.
//!
//! Device classes: the `micro.rs` host kinds. The edge runtime at 8e46165 has no host-kind
//! branch in its round or Pull schedule (runtime.rs holds no phone, mobile, battery or doze
//! path), so every edge device class (Phone, Laptop, HomeNode, HostedServer, CiRunner) has the
//! same schedule; the classes this model can tell apart are claimed (with an owner, three
//! owner legs a Pull cycle) and unclaimed devices. The counters are DERIVED from the model's own
//! events (the round cadence, the Pull timer and its events); the event rates they rest on carry
//! their own class in [`crate::capacity::COST_SPECS`].

use crate::capacity::{
    self, edge_971, pull_event_legs_per_device_s, run, Binding, CostParams, Plane, Scenario, C,
};
use crate::micro::HostKind;
use crate::seat_calibration::{self, input_intervals, propagate, Term, GROWTH_AXIS};

// ---------------------------------------------------------------------------
// 1. Energy counters
// ---------------------------------------------------------------------------

/// One seat's round counters at a fleet size (DERIVED from the cost ledger).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeatCounters {
    /// Devices in the fleet.
    pub devices: u64,
    /// Summaries the seat receives an hour (responder rounds and reply Summaries).
    pub rounds_per_hour: u64,
    /// Subject Pull requests the seat answers an hour.
    pub pulls_per_hour: u64,
    /// Counted store calls an hour (what the store counter shows).
    pub store_reads_per_hour: u64,
    /// Uncounted holdings-index pages an hour (drains).
    pub index_pages_per_hour: u64,
    /// Sweep permits taken an hour.
    pub permits_per_hour: u64,
    /// Seat egress an hour, bytes.
    pub egress_bytes_per_hour: u64,
    /// Store reads per received round (counted calls over Summaries and Pulls), milli-reads.
    pub store_reads_per_round_milli: u64,
    /// Egress bytes per received round (Summaries and Pulls).
    pub bytes_per_round: u64,
}

/// One device class's wake and round counters (DERIVED from the edge schedule at 8e46165).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceCounters {
    /// The host kind.
    pub kind: HostKind,
    /// Whether the device has an owner (three owner legs a Pull cycle).
    pub claimed: bool,
    /// Radio wake-ups an hour: Revocation rounds (one each 30 s cadence) plus Pull cycles
    /// (timer, first contact, kicked writes, host kicks), each a separate wake. MODEL: no
    /// coalescing of a Pull cycle onto a round's wake (they run on separate timers).
    pub wakes_per_hour_milli: u64,
    /// Rounds an hour with the seat (Revocation Summaries).
    pub rounds_per_hour_milli: u64,
    /// Pull cycles an hour.
    pub pull_cycles_per_hour_milli: u64,
    /// Of the rounds, how many an hour carry no row (idle), milli.
    pub idle_rounds_per_hour_milli: u64,
    /// Of the Pull cycles, how many an hour return no row (idle), milli.
    pub idle_pull_cycles_per_hour_milli: u64,
    /// Bytes each direction a device spends with one seat an hour (rounds and Pulls).
    pub bytes_per_hour: u64,
    /// Store reads the device's rounds and Pulls cost the seat an hour, milli.
    pub seat_reads_per_hour_milli: u64,
}

fn u(x: f64) -> u64 {
    if x.is_finite() && x > 0.0 { x.round() as u64 } else { 0 }
}

/// The seat's counters for v45.0.0 at `devices` (growth as set in `p`).
#[must_use]
pub fn seat_counters(p: &CostParams, devices: f64) -> SeatCounters {
    let o = run(p, &Scenario::v45(devices));
    let l = &o.seat;
    let pulls = pull_requests_per_s(p, devices);
    let per_round = l.summaries_per_s + pulls;
    let reads = l.counted_per_s();
    SeatCounters {
        devices: u(devices),
        rounds_per_hour: u(l.summaries_per_s * 3600.0),
        pulls_per_hour: u(pulls * 3600.0),
        store_reads_per_hour: u(reads * 3600.0),
        index_pages_per_hour: u(l.index_pages_per_s * 3600.0),
        permits_per_hour: u(l.permits_per_s * 3600.0),
        egress_bytes_per_hour: u(o.seat_egress_bytes_per_s * 3600.0),
        store_reads_per_round_milli: if per_round > 0.0 { u(reads / per_round * 1000.0) } else { 0 },
        bytes_per_round: if per_round > 0.0 { u(o.seat_egress_bytes_per_s / per_round) } else { 0 },
    }
}

/// Pull requests one seat answers per second from `devices` devices (one request a leg pair,
/// `pull_calls_per_cycle` a timer cycle).
fn pull_requests_per_s(p: &CostParams, devices: f64) -> f64 {
    let timer = p.get(C::PullCallsPerCycle) / p.get(C::PullCycleS);
    let events = pull_event_legs_per_device_s(p) / 2.0;
    devices * (timer + events)
}

/// Pull cycles a device runs per second: the timer, first-contact cycles after each runtime
/// start, kicked write cycles and host kicks.
fn pull_cycles_per_s(p: &CostParams) -> (f64, f64) {
    let timer = 1.0 / p.get(C::PullCycleS);
    let first = p.get(C::FirstContactCycles).min(p.get(C::FirstContactTries)) * p.get(C::RuntimeStartsPerDeviceDay);
    let events = (first + p.get(C::PullPlaneWritesPerDeviceDay) + p.get(C::HostKicksPerDeviceDay)) / 86_400.0;
    (timer, events)
}

/// Every edge device class, claimed and unclaimed.
pub const DEVICE_KINDS: [HostKind; 5] = [HostKind::Phone, HostKind::Laptop, HostKind::HomeNode, HostKind::HostedServer, HostKind::CiRunner];

/// One device class's counters. The edge schedule does not depend on `kind`; it is carried so
/// the energy package can price the same counts per class.
#[must_use]
pub fn device_counters(p: &CostParams, kind: HostKind, claimed: bool) -> DeviceCounters {
    let cadence = p.get(C::CadenceS);
    let rounds = 3600.0 / cadence;
    let (timer, events) = pull_cycles_per_s(p);
    let cycles = (timer + events) * 3600.0;
    // A Revocation round carries a row only when a revocation of a key in own ∪ cohort lands.
    // No key:revocation row has ever landed at seat-1 (rev_non_empty_subjects = 0, MEASURED),
    // so the rate is 0 and every round today is idle; a fleet that revokes keys changes this.
    // The model has no revocation-rate parameter, so it books none rather than invent one.
    debug_assert!(p.get(C::RevNonEmptySubjects) == 0.0, "a revocation rate is needed once revocations exist");
    let rev_rows_per_s = 0.0_f64;
    let carry_round = 1.0 - (-rev_rows_per_s * cadence).exp();
    // A timer Pull cycle returns a row only when one of its subjects changed since the last:
    // the device's own Pull-plane writes and key:record rows. Event cycles carry by definition.
    let own_rows_per_s = (p.get(C::PullPlaneWritesPerDeviceDay) + p.get(C::KeyRecordWritesPerDeviceDay)) / 86_400.0;
    let carry_timer = 1.0 - (-own_rows_per_s * p.get(C::PullCycleS)).exp();
    let idle_cycles = timer * 3600.0 * (1.0 - carry_timer);
    let owner = if claimed { p.get(C::OwnerPlanes) } else { 0.0 };
    let legs_per_timer = 2.0 * p.get(C::PullPlanes) + owner;
    let rb = p.get(C::RefBytes);
    let fo = p.get(C::FrameOverheadBytes);
    let round_bytes = p.get(C::RevPlaneRefs) * rb + fo;
    let timer_bytes = p.get(C::PullReplyBytesPerCycle) * legs_per_timer / (2.0 * p.get(C::PullPlanes)) + p.get(C::PullCallsPerCycle) * fo;
    let event_bytes = pull_event_legs_per_device_s(p) * 3600.0 * (p.get(C::PullRefsPerLeg) * rb + fo);
    let bytes = rounds * round_bytes + timer * 3600.0 * timer_bytes + event_bytes;
    // seat reads: a v45 Revocation round is a listing-cache hit (no store read); a Pull leg is
    // one subject read, plus the reach reads the seat's ledger books per device
    let reach = p.get(C::ReachReadsPerUnownedSubject) * p.get(C::ReachUnownedShare);
    let reads = (timer * 3600.0) * legs_per_timer * (1.0 + reach) + pull_event_legs_per_device_s(p) * 3600.0;
    DeviceCounters {
        kind,
        claimed,
        wakes_per_hour_milli: u((rounds + cycles) * 1000.0),
        rounds_per_hour_milli: u(rounds * 1000.0),
        pull_cycles_per_hour_milli: u(cycles * 1000.0),
        idle_rounds_per_hour_milli: u(rounds * (1.0 - carry_round) * 1000.0),
        idle_pull_cycles_per_hour_milli: u(idle_cycles * 1000.0),
        bytes_per_hour: u(bytes),
        seat_reads_per_hour_milli: u(reads * 1000.0),
    }
}

/// Every device class, claimed then unclaimed, in [`DEVICE_KINDS`] order.
#[must_use]
pub fn all_device_counters(p: &CostParams) -> Vec<DeviceCounters> {
    let mut v = Vec::new();
    for claimed in [true, false] {
        for k in DEVICE_KINDS {
            v.push(device_counters(p, k, claimed));
        }
    }
    v
}

/// The energy counters, markdown.
#[must_use]
pub fn energy_report(p: &CostParams) -> String {
    let mut s = format!(
        "Counts only (DERIVED from the model's events; no power figure). Edge {} schedule: one Revocation round a {:.0} s cadence, a Pull cycle every {:.0} s mean (jittered 0.5x to 1.5x), first-contact, write and kick cycles on top. The runtime does not branch on host kind, so every class below has the same schedule; claimed devices add owner legs.\n\n",
        edge_971::COMMIT,
        p.get(C::CadenceS),
        p.get(C::PullCycleS)
    );
    s.push_str("| class | claimed | wake-ups / h | rounds / h | idle rounds / h | Pull cycles / h | idle Pull cycles / h | bytes / h each way | seat reads / h |\n|---|---|---|---|---|---|---|---|---|\n");
    for d in all_device_counters(p).iter().filter(|d| d.kind == HostKind::Phone) {
        s.push_str(&format!(
            "| any edge device (Phone shown) | {} | {:.2} | {:.1} | {:.1} | {:.2} | {:.2} | {} | {:.1} |\n",
            d.claimed,
            d.wakes_per_hour_milli as f64 / 1000.0,
            d.rounds_per_hour_milli as f64 / 1000.0,
            d.idle_rounds_per_hour_milli as f64 / 1000.0,
            d.pull_cycles_per_hour_milli as f64 / 1000.0,
            d.idle_pull_cycles_per_hour_milli as f64 / 1000.0,
            d.bytes_per_hour,
            d.seat_reads_per_hour_milli as f64 / 1000.0,
        ));
    }
    s.push_str("\n| devices | seat rounds / h | Pulls / h | store reads / h | index pages / h | permits / h | egress bytes / h | reads per round | bytes per round |\n|---|---|---|---|---|---|---|---|---|\n");
    for d in [1_349.0, 5_000.0, 10_000.0] {
        let c = seat_counters(p, d);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {:.3} | {} |\n",
            c.devices,
            c.rounds_per_hour,
            c.pulls_per_hour,
            c.store_reads_per_hour,
            c.index_pages_per_hour,
            c.permits_per_hour,
            c.egress_bytes_per_hour,
            c.store_reads_per_round_milli as f64 / 1000.0,
            c.bytes_per_round
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// 2. Globe views
// ---------------------------------------------------------------------------

/// Number classes, as `twin-globe` reads them.
pub mod class {
    /// MEASURED.
    pub const MEASURED: f32 = 0.0;
    /// DERIVED.
    pub const DERIVED: f32 = 1.0;
    /// MODEL.
    pub const MODEL: f32 = 2.0;
    /// PROJECTED.
    pub const PROJECTED: f32 = 3.0;
    /// FRONTIER.
    pub const FRONTIER: f32 = 4.0;
}

/// Binding term codes in the views: 0 CPU, 1 sweep permits, 2 + plane index for a key plane's
/// lock (2 Key, 3 IdentityOccurrence, 4 Revocation, 5 IdentityOccurrenceRevocation, others by
/// [`Plane`] order + 2), -1 for none.
#[must_use]
pub fn binding_code(b: Binding) -> f32 {
    match b {
        Binding::Cpu => 0.0,
        Binding::Permits => 1.0,
        Binding::Lock(k) => 2.0 + plane_index(k) as f32,
    }
}

fn plane_index(k: Plane) -> usize {
    capacity::KEY_PLANES.iter().position(|x| *x == k).unwrap_or(4)
}

/// The growth for a view scenario: the index into [`GROWTH_AXIS`], clamped.
#[must_use]
pub fn growth_of(scenario: u32) -> f64 {
    GROWTH_AXIS[(scenario as usize).min(GROWTH_AXIS.len() - 1)]
}

/// The fleet sizes the curve view samples: 24 points, geometric from 500 to 100,000 devices.
#[must_use]
pub fn curve_devices() -> Vec<f64> {
    let n = 24;
    (0..n).map(|i| 500.0 * (200.0_f64).powf(i as f64 / (n - 1) as f64)).map(f64::round).collect()
}

fn params_for(scenario: u32) -> CostParams {
    seat_calibration::with_value(&CostParams::calibrated(), C::KeyPublishSetGrowth, growth_of(scenario))
}

/// Seat load against fleet size for growth scenario `scenario` (index into [`GROWTH_AXIS`]:
/// 0 = g 0, today's publish set; 4 = g 1, grows with announced nodes).
///
/// Stride 6, one row per [`curve_devices`] point:
/// `[devices, rho central, rho lower, rho upper, class, binding code]`.
/// `rho` is the binding utilisation of one v45.0.0 seat (the larger of CPU, sweep permits and
/// the busiest key-plane lock); lower and upper are its bounds over every input interval
/// ([`input_intervals`]): each term's utilisation is propagated alone by the vertex method and
/// the band is `[max lo, max hi]` over the terms, since `rho` is their maximum and switches term
/// across the input box (outer bounds, not a probability). Class 3 (PROJECTED) at every point: past 1,349 devices (the measured fleet) it is an extrapolation. The binding code
/// ([`binding_code`]) is the central case's.
#[must_use]
pub fn calibration_view_curve(scenario: u32) -> Vec<f32> {
    let p = params_for(scenario);
    let inputs = input_intervals(&p);
    let mut v = Vec::new();
    for d in curve_devices() {
        let sc = Scenario::v45(d);
        let central = run(&p, &sc).seat_load.rho;
        let (mut lo, mut hi) = (0.0_f64, 0.0_f64);
        for term in [Term::Cpu, Term::Permits, Term::Lock] {
            let pr = propagate(&p, &inputs, |q| {
                let l = run(q, &sc).seat_load;
                match term {
                    Term::Cpu => l.rho_cpu,
                    Term::Permits => l.rho_permit,
                    Term::Lock => l.rho_lock,
                }
            });
            lo = lo.max(pr.interval.lo);
            hi = hi.max(pr.interval.hi);
        }
        v.extend_from_slice(&[
            d as f32,
            central as f32,
            lo as f32,
            hi as f32,
            class::PROJECTED,
            binding_code(capacity::binding(&p, &sc)),
        ]);
    }
    v
}

/// The knee for growth scenario `scenario`.
///
/// Seven fields: `[knee lower, knee central, knee upper, interval kind, binds at lower,
/// binds at upper, class]`. Devices; the knee is the largest fleet whose seat stays under the
/// `cpu_ceiling` on every term. Interval kind 0 = outer bounds over the input intervals
/// (`seat_calibration::knee_interval`, per-term), not a confidence level. Class 3 (PROJECTED). A knee field is -1 when no knee is
/// claimed (the seat is over the ceiling at one device, or under it past 10^9 devices).
#[must_use]
pub fn calibration_view_knee(scenario: u32) -> Vec<f32> {
    let p = params_for(scenario);
    let sc = Scenario::v45(1.0);
    let k = seat_calibration::knee_interval(&p, &sc);
    let central = capacity::knee(&p, &sc);
    let claim = |x: f64| if x >= 1.0 && x < 1.0e9 { x as f32 } else { -1.0 };
    vec![
        claim(k.devices.lo),
        claim(central),
        claim(k.devices.hi),
        0.0,
        binding_code(k.binds_low),
        binding_code(k.binds_high),
        class::PROJECTED,
    ]
}

/// The fit's counters, in-sample and held out.
///
/// Stride 4, one row per counter: `[counter id, observed, predicted, out of sample]`; out of
/// sample is 0 for a counter the fit read (in-sample: it reproduces it by construction or by a
/// ratio) and 1 for a held-out counter ([`seat_calibration::HELD_OUT`]). Units are each
/// counter's own (per minute for counts, cores for CPU), so the globe should plot
/// predicted / observed. Counter ids:
/// 0 window B CPU (in), 1 window A composer (in), 2 window A attestation queries (in),
/// 3 window B attestation queries (out), 4 window B composer (out), 5 window B permits (out),
/// 6 window A' permits (out), 7 window C permits (out), 8 window C attestation queries (out),
/// 9 busy-hour CPU (out).
#[must_use]
pub fn calibration_view_residuals() -> Vec<f32> {
    use seat_calibration::{Counter as K, WindowId as W};
    let obs = seat_calibration::observations();
    let f = seat_calibration::fit(&seat_calibration::FitView::new(&obs));
    let p = seat_calibration::fitted_params(&CostParams::default(), &f);
    let c = capacity::calibrate(&p);
    let a = capacity::ledger(
        &p,
        &capacity::measured_mix(&p, capacity::window_a::ROUNDS, 0.0, capacity::window_a::WITHHOLDS),
        capacity::ReadSet::AsShipped,
    );
    let mut v: Vec<f32> = vec![
        0.0,
        capacity::window_b::CPU_CORES as f32,
        (c.b_cpu_index + c.b_cpu_query + c.b_cpu_rest) as f32,
        0.0,
        1.0,
        capacity::window_a::COMPOSER as f32,
        (a.composer_per_s * 60.0) as f32,
        0.0,
        2.0,
        capacity::window_a::ATT_QUERIES as f32,
        (a.att_queries_per_s * 60.0) as f32,
        0.0,
    ];
    let ids = [
        ((W::B, K::AttQueries), 3.0),
        ((W::B, K::Composer), 4.0),
        ((W::B, K::Permits), 5.0),
        ((W::APrime, K::Permits), 6.0),
        ((W::C, K::Permits), 7.0),
        ((W::C, K::AttQueries), 8.0),
        ((W::BusyHour, K::Cpu), 9.0),
    ];
    for h in seat_calibration::held_out(&p, &obs) {
        let id = ids.iter().find(|x| x.0 == (h.window, h.counter)).map_or(-1.0, |x| x.1);
        v.extend_from_slice(&[id, h.measured as f32, h.predicted as f32, 1.0]);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_schedule_is_the_edge_schedule() {
        let p = CostParams::calibrated();
        let d = device_counters(&p, HostKind::Phone, false);
        // 120 Revocation rounds an hour at the 30 s cadence
        assert_eq!(d.rounds_per_hour_milli, 120_000);
        // every Revocation round is idle while the seat holds no revocation
        assert_eq!(d.idle_rounds_per_hour_milli, d.rounds_per_hour_milli);
        // six timer Pull cycles an hour, plus the event cycles
        assert!(d.pull_cycles_per_hour_milli >= 6_000);
        assert_eq!(d.wakes_per_hour_milli, d.rounds_per_hour_milli + d.pull_cycles_per_hour_milli);
        assert!(d.idle_pull_cycles_per_hour_milli < 6_000);
        // the class does not change the schedule; an owner adds legs
        for k in DEVICE_KINDS {
            let e = device_counters(&p, k, false);
            assert_eq!((e.wakes_per_hour_milli, e.bytes_per_hour), (d.wakes_per_hour_milli, d.bytes_per_hour));
        }
        let c = device_counters(&p, HostKind::Phone, true);
        assert!(c.bytes_per_hour > d.bytes_per_hour && c.seat_reads_per_hour_milli > d.seat_reads_per_hour_milli);
        assert_eq!(all_device_counters(&p).len(), 10);
    }

    #[test]
    fn seat_counters_grow_with_the_fleet() {
        let p = CostParams::calibrated();
        let a = seat_counters(&p, 1_349.0);
        let b = seat_counters(&p, 10_000.0);
        assert!(b.rounds_per_hour > a.rounds_per_hour && b.store_reads_per_hour > a.store_reads_per_hour);
        assert!(a.store_reads_per_round_milli > 0 && a.bytes_per_round > 0);
        assert!(energy_report(&p).contains("wake-ups / h"));
    }

    #[test]
    fn the_curve_view_is_an_interval_at_every_point() {
        let v = calibration_view_curve(4);
        assert_eq!(v.len(), 6 * curve_devices().len());
        for r in v.chunks(6) {
            assert!(r[2] <= r[1] && r[1] <= r[3], "{r:?}");
            assert!(r[2] < r[3], "a point at {r:?}");
            assert!((r[4] - class::PROJECTED).abs() < f32::EPSILON);
        }
        // load rises with devices
        assert!(v[6 * 23 + 1] > v[1]);
        assert_eq!(calibration_view_curve(4), v, "deterministic");
    }

    #[test]
    fn the_knee_view_is_an_interval_or_no_claim() {
        for s in 0..GROWTH_AXIS.len() as u32 {
            let k = calibration_view_knee(s);
            assert_eq!(k.len(), 7);
            if k[0] >= 0.0 && k[2] >= 0.0 {
                assert!(k[0] < k[2], "scenario {s}: {k:?}");
                assert!(k[1] < 0.0 || (k[0] <= k[1] && k[1] <= k[2]), "{k:?}");
            }
        }
        // an out-of-range scenario clamps to g = 1
        assert_eq!(calibration_view_knee(99), calibration_view_knee(4));
    }

    #[test]
    fn the_residual_view_flags_held_out_counters() {
        let v = calibration_view_residuals();
        assert_eq!(v.len() % 4, 0);
        let rows: Vec<&[f32]> = v.chunks(4).collect();
        assert_eq!(rows.len(), 3 + seat_calibration::HELD_OUT.len());
        assert_eq!(rows.iter().filter(|r| r[3] == 1.0).count(), seat_calibration::HELD_OUT.len());
        for r in &rows {
            assert!(r[0] >= 0.0 && r[1] > 0.0 && r[2] > 0.0, "{r:?}");
        }
        // window B's CPU is fitted exactly
        assert!((rows[0][2] / rows[0][1] - 1.0).abs() < 1e-4);
    }
}
