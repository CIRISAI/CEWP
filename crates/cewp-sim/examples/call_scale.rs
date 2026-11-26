//! Call scale: `cargo run -p cewp-sim --example call_scale`.
//!
//! Prints the call-scale report (per-member cost of one live call at N = 4 to 1000, today and
//! with the 45.3 fixes plus per-stream subscription; what a room hubbed at the seat costs it),
//! then the seat's knee against `key_publish_set` growth (v45, one seat, MODEL with intervals).

use cewp_sim::capacity::CostParams;
use cewp_sim::{call_scale, seat_calibration};

fn main() {
    let p = CostParams::calibrated();
    print!("{}", call_scale::report(&p));
    println!("\n## Seat knee against key_publish_set growth (v45, one seat)\n");
    print!("{}", seat_calibration::growth_table(&p));
    // The point knee (calibrated inputs) and who holds the permits there.
    for f in [0.0, 1.0] {
        let q = seat_calibration::with_value(&p, cewp_sim::capacity::C::AnnouncedFraction, f);
        let sc = cewp_sim::capacity::Scenario::v45(1.0);
        let k = cewp_sim::capacity::knee(&q, &sc);
        let mut at = sc;
        at.devices = k;
        let o = cewp_sim::capacity::run(&q, &at);
        let l = &o.seat;
        let hold = q.get(cewp_sim::capacity::C::StoreReadS);
        let pull = l.pull_permits_per_s * hold;
        let key = l.ttl_reads_per_s * hold;
        let pages = (l.permits_per_s - l.key_plane_permits_per_s - l.pull_permits_per_s) * q.page_cpu_s();
        let probes = l.probe_reads_per_s * q.get(cewp_sim::capacity::C::ProbeReadS);
        let all = pull + key + pages + probes;
        println!(
            "\npoint knee at f = {f}: {k:.0} devices, binds {}; rho cpu {:.2} permits {:.2} lock {:.2}; permit-seconds/s: subject Pulls {:.3} ({:.0}%), key-plane refills {:.3}, pages {:.3}, probes {:.4}; Global subjects {:.0}, Key {:.0}, IO {:.0}",
            cewp_sim::capacity::binding(&q, &at).label(),
            o.seat_load.rho_cpu, o.seat_load.rho_permit, o.seat_load.rho_lock,
            pull, 100.0 * pull / all, key, pages, probes,
            l.key_planes[2].subjects, l.key_planes[0].subjects, l.key_planes[1].subjects
        );
    }
}
