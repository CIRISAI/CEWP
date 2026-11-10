//! Seat capacity sweep: `cargo run -p cewp-sim --example capacity_sweep`.
//!
//! Prints the calibration against seat-1, the knee table, the one-seat sweep
//! (10 to 100,000 devices), the referral + proxy offload table, idle bytes and
//! the 1M-device v45 baseline, then every parameter with its tag. Override any
//! parameter with `name=value` arguments (the CPU fit is redone after them).

use cewp_sim::capacity::{calibrate, report_markdown, CostParams, C, COST_SPECS};

fn main() {
    let mut p = CostParams::default();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else {
            eprintln!("ignored argument {arg}: expected name=value");
            continue;
        };
        let Ok(x) = v.parse::<f64>() else {
            eprintln!("ignored {arg}: value is not a number");
            continue;
        };
        match COST_SPECS.iter().position(|s| s.name == k) {
            Some(i) => p.v[i] = x,
            None => eprintln!("ignored {arg}: no parameter named {k}"),
        }
    }
    let c = calibrate(&p).cpu_per_byte_s;
    p.set(C::CpuPerByte, c);
    print!("{}", report_markdown(&p));
}
