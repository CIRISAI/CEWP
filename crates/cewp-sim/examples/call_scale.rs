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
}
