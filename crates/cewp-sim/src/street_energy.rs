//! The focal street's mesh energy delta from the status quo: twin-theory's energy pass
//! (`energy::delta`, FSD/twin/energy.md) with its household mix set from this street's own households as
//! CIRISHolon's DEVICEGEN-1 drew them (`energy::with_mix_from_draws`), under the home-node scenario the page
//! has chosen. Expected regime, short and long run, the point and the lowest and highest found over every
//! input's interval. In kWh per household-year (the pass's joules per household-hour x 8,766 / 3.6e6).
//!
//! The mix scenarios put a home node in EVERY household that can host it (the pass's rule), not the
//! street's drawn 60 %: the line says so. Nothing here is spatial beyond the street's own households, which
//! is why there is no Power map (holon-intake.md).

use crate::energy::{delta, host_reach, with_mix_from_draws, Class, Counters, EnergyParams, Run, Scenario};
use crate::workload::ParameterRegime;
use crate::micro::{MicroWorld, HOME_ON_COMPUTER, HOME_ON_GATEWAY, HOME_ON_NEW_BOX};

/// kWh per household-year from joules per household-hour.
pub const J_PH_TO_KWH_PY: f64 = 8766.0 / 3.6e6;

/// What [`street_energy`] returns.
#[derive(Clone, Debug, PartialEq)]
pub struct StreetEnergy {
    pub households: usize,
    pub scenario: Scenario,
    /// Share of the street's households the scenario's home node reaches.
    pub reach: f64,
    /// Short run: point, lowest, highest (kWh per household-year).
    pub short: [f64; 3],
    /// Long run.
    pub long: [f64; 3],
    /// The weakest class of any row priced.
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
    let one = |run| {
        let d = delta(&p, k, sc, ParameterRegime::Expected, run);
        ([d.net * J_PH_TO_KWH_PY, d.net_bounds.lo * J_PH_TO_KWH_PY, d.net_bounds.hi * J_PH_TO_KWH_PY], d.class)
    };
    let (short, c1) = one(Run::Short);
    let (long, c2) = one(Run::Long);
    Some(StreetEnergy { households: w.household_draws.len(), scenario: sc, reach: host_reach(&p, sc), short, long, class: c1.weaker(c2) })
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
            for v in [e.short, e.long] {
                assert!(v[1] <= v[0] && v[0] <= v[2], "{where_}: {v:?}");
            }
            // the gateway scenario reaches the street's fixed-line households
            if where_ == HOME_ON_GATEWAY {
                let fixed = w.household_draws.iter().flatten().filter(|d| d.connection == 0).count() as f64 / dwellings as f64;
                assert!((e.reach - fixed).abs() < 1e-9, "{} vs {fixed}", e.reach);
            }
            out.push(e.short[0]);
        }
        // a home node in every household that can host costs more than phones only
        assert!(out[2] > out[3], "{out:?}");
        // the legacy device draw: no draws, no line
        let w = MicroWorld::from_leaf_with(&leaf, p, sizes, None);
        assert!(street_energy(&w, &k).is_none());
    }
}
