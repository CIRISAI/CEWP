//! Shares the home-node scenarios produce under CIRISHolon's device generator (MODEL): US households drawn
//! by HOUSEGEN-1 (US 2024 rows, class 2, in country) in 200 tiles of 2,000 people, each household through
//! DEVICEGEN-1 in country, building type cycling house / apartment / unknown in equal thirds. A home node
//! is drawn at the legacy 0.6 share among the households the scenario lets host.
//! `cargo run --release -p cewp-sim --example devicegen_shares`
use cewp_sim::holon_scene::{devices, household};
fn main() {
    let (mut n, mut nothing, mut gw, mut desk, mut net) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for t in 0..200u64 {
        let hh = household::generate(t, 2_000, 2, household::Reference::UnitedStates2024, true);
        for (i, &s) in hh.sizes.iter().enumerate() {
            let (d, _, _) = devices::generate(t * 100_003 + i as u64, u32::from(s), (i % 3) as u8, true);
            n += 1;
            nothing += u64::from(d.connection != 0 && d.counts[0] == 0 && d.counts[1] == 0 && d.counts[2] == 0);
            gw += u64::from(d.gateway_declared());
            desk += u64::from(d.has_desktop());
            net += u64::from(d.connection != 2);
        }
    }
    let pc = |x: u64| 100.0 * x as f64 / n as f64;
    println!("households {n}");
    println!("nothing to host on (no computer, no fixed line): {:.1} %", pc(nothing));
    println!("can host, on the gateway (fixed line): {:.1} % -> home node at 0.6: {:.1} %", pc(gw), 0.6 * pc(gw));
    println!("can host, on an existing computer (desktop): {:.1} % -> {:.1} %", pc(desk), 0.6 * pc(desk));
    println!("can host, on a new box (any internet): {:.1} % -> {:.1} %", pc(net), 0.6 * pc(net));
    println!("none (phones only): 0 %; legacy rule: 60 % of all households");
}
