//! The focal street's share of the full-replacement test (FSD/twin/energy.md, TEST 1; `energy_full`), in
//! kWh per household-year, in two parts that sum to the street's net:
//!
//! - **added on the street's devices**: the household pass's terms A + B (`energy::delta`, expected regime,
//!   long run, as energy_full's `Additions` reads them) with the mix set from this street's own households
//!   as CIRISHolon's DEVICEGEN-1 drew them (`energy::with_mix_from_draws`), under the chosen home-node
//!   scenario; the pass's term C (the adoption path's avoided upstream) is NOT used: under full replacement
//!   that work is inside energy_full's "removed";
//! - **the data-centre side**: energy_full's net with the additions set to zero (removed, less the work that
//!   relocates to members' devices, the AI kept hosted, the signed records, Proof of Benefit and
//!   self-observation), Case A (the central case), the tiered AI mix at 10 interactions per person-day, per
//!   person-year (`FULL_REST`, baked from the run below and checked by a test), times the street's persons.
//!
//! The mix scenarios put a home node in EVERY household that can host it (the pass's rule), not the
//! street's drawn 60 %: the line says so. Nothing here is spatial beyond the street's own households, which
//! is why there is no Power map (holon-intake.md).

use crate::energy::{delta, host_reach, with_mix_from_draws, Class, Counters, EnergyParams, Run, Scenario};

/// energy_full's net with zero additions, kWh per person-year, [low, point, high]: Case A, `AiCase::Mix`,
/// 10 interactions per person-day, new-box share 0; the point from `energy_full::eval`, the range from
/// `energy_full::corners_at` (the headline row of `cewp-cli energy-full 1`, less its additions). Baked so a
/// street does not search the corners; `the_baked_data_centre_side_is_energy_fulls` recomputes it.
pub const FULL_REST: [f64; 3] = [-43.32818257044865, -15.404024153034381, 909.2970850317707];
use crate::workload::ParameterRegime;
use crate::micro::{MicroWorld, HOME_ON_COMPUTER, HOME_ON_GATEWAY, HOME_ON_NEW_BOX};

/// kWh per household-year from joules per household-hour.
pub const J_PH_TO_KWH_PY: f64 = 8766.0 / 3.6e6;

/// The energy pass's counters at seed 1 (the report's, `energy::Counters::from_hooks(1)`), baked: reading them
/// runs a 2,000-person stand-in day, 1.2 s on a phone at 4x CPU throttle, a frozen page; the test
/// `the_baked_counters_are_seed_ones` recomputes them.
#[must_use]
#[allow(clippy::unreadable_literal, clippy::excessive_precision)]
pub fn counters_seed1() -> Counters {
    crate::energy::Counters { sync_wakes_per_device_hour: 126.75, sync_rounds_per_device_hour: 120.0, sync_pulls_per_device_hour: 6.75, sync_bytes_per_device_hour: 76049.0, seat_reads_per_device_hour: 86.417, acts_sent_per_person_hour: 7.507541666666667, acts_received_per_person_hour: 158.86458333333334, content_wakes_per_phone_hour: 93.18291666666666, relayed_share_fixed: 0.25412293853073464, relayed_share_wifi: 0.21273230525899564, relayed_share_cell: 0.5170020818875781, held_mappings_per_phone: 0.7669421487603306, path_same_metro: crate::energy::MeanPath { n: 20, ases_crossed: 3.0, ixp_crossings: 0.35, long_haul_km: 0.0 }, path_any_peer: crate::energy::MeanPath { n: 400, ases_crossed: 3.935, ixp_crossings: 0.165, long_haul_km: 9781.1875 }, path_to_dc: crate::energy::MeanPath { n: 400, ases_crossed: 2.8325, ixp_crossings: 0.725, long_haul_km: 1711.3225 }, call: [crate::energy::CallClass { hours: 0.01595486611111111, own_tx_mb: 1680.0148502238553, fwd_tx_mb: 1892.0691524309946, rx_mb: 2137.432728203889, sig_ops: 5530350.388747647 }, crate::energy::CallClass { hours: 0.03256904638888889, own_tx_mb: 1273.5651054907987, fwd_tx_mb: 788.1955060482743, rx_mb: 2266.0720893927855, sig_ops: 5802902.478117281 }, crate::energy::CallClass { hours: 0.016426273055555555, own_tx_mb: 1361.6237185607615, fwd_tx_mb: 0.0, rx_mb: 2257.359589396276, sig_ops: 5805455.667117836 }], call_participants: 4.0 }
}

/// What [`street_energy`] returns (kWh per household-year, each as [low, point, high]).
#[derive(Clone, Debug, PartialEq)]
pub struct StreetEnergy {
    pub households: usize,
    /// Persons on the street (every person in its households).
    pub persons: u32,
    pub scenario: Scenario,
    /// Share of the street's households the scenario's home node reaches.
    pub reach: f64,
    /// Added on the street's devices (A + B, long run).
    pub devices: [f64; 3],
    /// The data-centre side for the street's persons ([`FULL_REST`] x persons per household).
    pub data_centre: [f64; 3],
    /// Their sum (the range is the sum of the two ranges).
    pub net: [f64; 3],
    /// The weakest class of any row priced on the device side (the data-centre side is FRONTIER).
    pub class: Class,
}

/// The pass's mix scenario for the page's home-node scenario.
#[must_use]
pub const fn scenario_for(home_node_where: u8) -> Scenario {
    match home_node_where {
        HOME_ON_GATEWAY => Scenario::MixGateway,
        HOME_ON_COMPUTER => Scenario::MixComputer,
        HOME_ON_NEW_BOX => Scenario::MixBox,
        _ => Scenario::MixNone,
    }
}

/// The street's delta, with `k` the pass's counters (the page uses seed 1, the report's). None under the
/// legacy device draw.
#[must_use]
pub fn street_energy(w: &MicroWorld, k: &Counters) -> Option<StreetEnergy> {
    let rule = w.device_rule?;
    if w.household_draws.is_empty() || w.household_draws.iter().any(Option::is_none) {
        return None;
    }
    let p = with_mix_from_draws(&EnergyParams::default(), w.household_draws.iter().flatten().copied());
    let sc = scenario_for(rule.home_node_where);
    let d = delta(&p, k, sc, ParameterRegime::Expected, Run::Long);
    let devices = [(d.bounds[0].lo + d.bounds[1].lo) * J_PH_TO_KWH_PY, (d.point[0] + d.point[1]) * J_PH_TO_KWH_PY, (d.bounds[0].hi + d.bounds[1].hi) * J_PH_TO_KWH_PY];
    let households = w.household_draws.len();
    let persons: u32 = w.household_draws.iter().flatten().map(|h| h.persons).sum();
    let pph = f64::from(persons) / households as f64;
    let data_centre = FULL_REST.map(|x| x * pph);
    let net = [devices[0] + data_centre[0], devices[1] + data_centre[1], devices[2] + data_centre[2]];
    Some(StreetEnergy { households, persons, scenario: sc, reach: host_reach(&p, sc), devices, data_centre, net, class: d.class })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::micro::{DeviceRule, MicroParams, HOME_NONE};
    use crate::tiling::{HolonId, LeafContext, LeafLayout};

    #[test]
    fn a_streets_delta_reads_its_own_households() {
        let p = MicroParams::default();
        let leaf = LeafLayout::build(HolonId::at(8, 40.06, -83.05), &LeafContext { budget: 40, chain: [3_000; 11], ..LeafContext::default() }, &p);
        let dwellings: usize = leaf.buildings.iter().map(|b| usize::from(b.units)).sum();
        let sizes: Vec<u8> = (0..dwellings).map(|i| 1 + (i % 4) as u8).collect();
        let k = Counters::from_hooks(1);
        let mut out = Vec::new();
        for where_ in [HOME_ON_GATEWAY, HOME_ON_COMPUTER, HOME_ON_NEW_BOX, HOME_NONE] {
            let w = MicroWorld::from_leaf_with(&leaf, p.clone(), sizes.clone(), Some(DeviceRule { home_node_where: where_, in_country: true }));
            let e = street_energy(&w, &k).expect("draws");
            assert_eq!(e.households, dwellings);
            for v in [e.devices, e.data_centre, e.net] {
                assert!(v[0] <= v[1] && v[1] <= v[2], "{where_}: {v:?}");
            }
            assert!((e.net[1] - e.devices[1] - e.data_centre[1]).abs() < 1e-9);
            // the gateway scenario reaches the street's fixed-line households
            if where_ == HOME_ON_GATEWAY {
                let fixed = w.household_draws.iter().flatten().filter(|d| d.connection == 0).count() as f64 / dwellings as f64;
                assert!((e.reach - fixed).abs() < 1e-9, "{} vs {fixed}", e.reach);
            }
            out.push(e.devices[1]);
        }
        // a home node in every household that can host adds more on the devices than phones only
        assert!(out[2] > out[3], "{out:?}");
        // the legacy device draw: no draws, no line
        let w = MicroWorld::from_leaf_with(&leaf, p, sizes, None);
        assert!(street_energy(&w, &k).is_none());
    }

    #[test]
    fn the_baked_data_centre_side_is_energy_fulls() {
        use crate::energy_full::{corners_at, eval, AiCase, Additions, FullParams, F};
        let z = Additions { gateway: [0.0; 3], new_box: [0.0; 3] };
        let mut p = FullParams::defaults(&z, AiCase::Mix);
        p.set(F::InteractionsPerPersonDay, 10.0);
        let (lo, hi, _) = corners_at(&z, AiCase::Mix, 0.0, 10.0);
        assert_eq!([lo, eval(&p).net, hi], FULL_REST);
        // and the headline it is part of: energy_full's net = its additions + this side
        let k = Counters::from_hooks(1);
        let a = Additions::from_household_pass(&k);
        let mut q = FullParams::defaults(&a, AiCase::Mix);
        q.set(F::InteractionsPerPersonDay, 10.0);
        let r = eval(&q);
        assert!((r.net - (r.additions + FULL_REST[1])).abs() < 1e-9, "{} vs {} + {}", r.net, r.additions, FULL_REST[1]);
    }

    #[test]
    fn the_baked_counters_are_seed_ones() {
        assert_eq!(counters_seed1(), Counters::from_hooks(1));
    }
}
