//! The seat card's per-fleet rows for the globe, on the calibrated seat model
//! ([`crate::seat_calibration`], [`crate::seat_hooks`]): one v45.0.0 seat (`Scenario::v45`),
//! `CostParams::calibrated()` with `key_publish_set` growth set by scenario, every figure with its
//! outer bounds over the input intervals ([`seat_calibration::input_intervals`], vertex method,
//! [`seat_calibration::propagate`]). It replaces the page's use of kernel-wasm `cap_seat`, which
//! ran the same cost model at one point with growth silently fixed at 1 and no interval.
//!
//! # Row layout: f32 x [`SEAT_FLEET_STRIDE`]
//!
//! `[devices, g, cores central, cores lower, cores upper, counted store reads/s central, lower,
//! upper, rho central, rho lower, rho upper, binding code (central), busiest key-plane lock rho
//! central, lower, upper, busiest plane code, subjects that plane walks (central), class]`.
//!
//! `rho` is the binding utilisation (the largest of CPU, sweep permits and the busiest lock); its
//! band is `[max lo, max hi]` over the three terms, each propagated alone, exactly as
//! [`seat_hooks::calibration_view_curve`] builds its band (outer bounds, not a probability).
//! Binding and plane codes are [`seat_hooks::binding_code`]'s (0 CPU, 1 sweep permits, 2 + plane).
//! Class 2 (MODEL with its interval) on every row: past 1,349 devices it is an extrapolation of
//! one seat's model, and below it the model is still a model of the seat, not a measurement.

use crate::capacity::{self, run, CostParams, Scenario, C};
use crate::seat_calibration::{self, input_intervals, propagate, Term};
use crate::seat_hooks::{binding_code, class, growth_of};

/// f32 values per row.
pub const SEAT_FLEET_STRIDE: usize = 18;

/// One row for `devices` at growth scenario `scenario` (0 = g 0 .. 4 = g 1; see
/// [`seat_calibration::GROWTH_AXIS`]).
#[must_use]
pub fn seat_view_fleet(devices: f64, scenario: u32) -> Vec<f32> {
    let g = growth_of(scenario);
    let p = seat_calibration::with_value(&CostParams::calibrated(), C::KeyPublishSetGrowth, g);
    let inputs = input_intervals(&p);
    let sc = Scenario::v45(devices.max(0.0));
    let o = run(&p, &sc);
    let cores = propagate(&p, &inputs, |q| run(q, &sc).seat_load.cpu_cores).interval;
    let reads = propagate(&p, &inputs, |q| run(q, &sc).seat.counted_per_s()).interval;
    let (mut lo, mut hi) = (0.0_f64, 0.0_f64);
    let mut lock = None;
    for term in [Term::Cpu, Term::Permits, Term::Lock] {
        let pr = propagate(&p, &inputs, |q| {
            let l = run(q, &sc).seat_load;
            match term {
                Term::Cpu => l.rho_cpu,
                Term::Permits => l.rho_permit,
                Term::Lock => l.rho_lock,
            }
        })
        .interval;
        lo = lo.max(pr.lo);
        hi = hi.max(pr.hi);
        if term == Term::Lock {
            lock = Some(pr);
        }
    }
    let lock = lock.expect("the lock term ran");
    let plane = o.seat.busiest_plane();
    let subjects = o.seat.key_planes[o.seat.lock_plane].subjects;
    vec![
        devices as f32,
        g as f32,
        o.seat_load.cpu_cores as f32,
        cores.lo as f32,
        cores.hi as f32,
        o.seat.counted_per_s() as f32,
        reads.lo as f32,
        reads.hi as f32,
        o.seat_load.rho as f32,
        lo as f32,
        hi as f32,
        binding_code(capacity::binding(&p, &sc)),
        o.seat_load.rho_lock as f32,
        lock.lo as f32,
        lock.hi as f32,
        binding_code(capacity::Binding::Lock(plane)),
        subjects as f32,
        class::MODEL,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seat_hooks::{calibration_view_curve, curve_devices};

    #[test]
    fn seat_view_rows_bracket_their_central_value() {
        for sc in [0u32, 4] {
            for d in [59.0, 1_000.0, 10_000.0, 100_000.0] {
                let r = seat_view_fleet(d, sc);
                assert_eq!(r.len(), SEAT_FLEET_STRIDE);
                for k in [2usize, 5, 8, 12] {
                    assert!(r[k + 1] <= r[k] * 1.000_01 && r[k] <= r[k + 2] * 1.000_01, "sc {sc} d {d} field {k}: {:?}", &r[k..k + 3]);
                }
                assert_eq!(r[17], class::MODEL);
            }
        }
    }

    #[test]
    fn seat_view_rho_band_is_the_calibration_curves() {
        // at a fleet size the curve samples, the row's rho and band are the curve's
        for sc in [0u32, 4] {
            let c = calibration_view_curve(sc);
            let d = curve_devices()[10];
            let r = seat_view_fleet(d, sc);
            assert_eq!((r[8], r[9], r[10], r[11]), (c[10 * 6 + 1], c[10 * 6 + 2], c[10 * 6 + 3], c[10 * 6 + 5]));
        }
    }

    #[test]
    fn seat_view_load_grows_with_the_fleet() {
        let a = seat_view_fleet(1_000.0, 4);
        let b = seat_view_fleet(100_000.0, 4);
        assert!(b[2] > a[2] && b[4] > a[4] && b[10] > a[10]);
    }
}
