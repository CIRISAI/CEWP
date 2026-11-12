//! A street's day as CIRISHolon's rhythm Series (`holon_scene::rhythm::Series`): where each person of a
//! street leaf is in each hour, from PRESENCEGEN-2's chain (`holon_scene::presence::day`), drawn as it is.
//!
//! Cells: one per household (its home), then three away cells (work or school; elsewhere, awake or
//! asleep; travelling). Bins: the 24 slots of the generator's day, slot `s` being clock hour
//! `(4 + s) mod 24`, so 25 counts would need tomorrow; the series holds the 24 counts of one day and the
//! 23 bins between them. Flows: every person whose cell changes between two slots, summed by (from, to).
//! Source: zero (nobody appears or vanishes), and `check_continuity` must find nothing.
//!
//! Who gets a day (MODEL, stated on the page): the household's persons as DEVICEGEN-1 drew them (adults
//! 18 to 64, adults over 64, children). Diaries start at 15 and the device generator's children are
//! under 18, so children get no day and are not in the series (counted apart). Employment is drawn per
//! adult from `presence::employed_ppm`: adults 18 to 64 at the 25-to-64 band (the 18-to-24 share is not
//! separable), adults over 64 at the 65-and-over band. Under the legacy device draw there is no
//! composition and so no series.

use crate::micro::MicroWorld;
use holon_scene::presence::{self, Generator};
use holon_scene::rhythm::{check_continuity, Series};
use holon_scene::seed::{child_seed, Stream};

/// Away cells after the homes.
pub const AWAY_WORK: usize = 0;
pub const AWAY_ELSEWHERE: usize = 1;
pub const AWAY_TRAVEL: usize = 2;

/// What [`street_day`] returns.
#[derive(Clone, Debug)]
pub struct StreetDay {
    /// Homes first (household order), then the three away cells.
    pub series: Series,
    /// Households (home cells).
    pub homes: usize,
    /// Persons with a generated day.
    pub persons: u32,
    /// Children: no diary, not in the series.
    pub children: u32,
    /// Persons drawn employed.
    pub employed: u32,
    /// The generator's verdict for this day type and place (0 certified, 1 refused, 2 unresolved) and why.
    pub verdict: (u8, &'static str),
    /// Places the continuity law failed (must be 0).
    pub violations: usize,
    /// Per slot, persons at home awake (state 1), for the tint of a home; not part of the series.
    pub home_awake: Vec<Vec<u16>>,
    /// Per home, its persons with a day (its adults); the home's count is at most this in every slot.
    pub residents: Vec<u16>,
}

/// The cell a PRESENCE-0 state puts a person in (0 home asleep, 1 home awake, 2 work or school, 3
/// elsewhere awake, 4 elsewhere asleep, 5 travelling).
fn cell_of(state: u8, home: usize, homes: usize) -> usize {
    match state {
        0 | 1 => home,
        2 => homes + AWAY_WORK,
        3 | 4 => homes + AWAY_ELSEWHERE,
        _ => homes + AWAY_TRAVEL,
    }
}

/// One adult's generated day: their household, their index among its adults (18 to 64 first, then over 64),
/// whether they were drawn employed, the 24 states, and the generator's verdict.
#[derive(Clone, Debug)]
pub struct PersonDay {
    pub household: usize,
    pub index: u8,
    pub employed: bool,
    pub states: [u8; 24],
    pub verdict: (u8, &'static str),
}

/// Every adult's day on the street (the seeds `street_day` uses): None under the legacy device draw.
#[must_use]
pub fn person_days(w: &MicroWorld, day: u64, day_type: u8, in_country: bool) -> Option<Vec<PersonDay>> {
    if w.household_groups.is_empty() || w.household_groups.iter().any(Option::is_none) {
        return None;
    }
    let (p_adult, p_senior) = (u64::from(presence::employed_ppm(1)), u64::from(presence::employed_ppm(2)));
    let mut out = Vec::new();
    for (hi, g) in w.household_groups.iter().enumerate() {
        let [adults, seniors, _] = g.unwrap_or([0, 0, 0]);
        for k in 0..u64::from(adults) + u64::from(seniors) {
            let key = (hi as u64) << 8 | k;
            let person = child_seed(w.seed ^ 0x9E5E_4CE0_0000_0003, key);
            let ppm = if k < u64::from(adults) { p_adult } else { p_senior };
            let emp = Stream::new(child_seed(person, 0xE4)).below(1_000_000) < ppm;
            let (d, call) = presence::day(Generator::Chain, child_seed(person, day), emp, day_type, person, false, in_country);
            out.push(PersonDay { household: hi, index: k as u8, employed: emp, states: d.states, verdict: (crate::popchain::households::verdict_code(call.verdict), call.reason) });
        }
    }
    Some(out)
}

/// Awake in PRESENCE-0's sense (states 1, 2, 3 and 5: not asleep at home or elsewhere).
#[must_use]
pub const fn is_awake(state: u8) -> bool {
    matches!(state, 1 | 2 | 3 | 5)
}

/// The street's day number `day` (any integer; it seeds the day) of `day_type` (0 Monday to Thursday,
/// 1 Friday, 2 Saturday, 3 Sunday). `in_country`: inside the United States (the diaries' country).
/// None under the legacy device draw (no composition).
#[must_use]
pub fn street_day(w: &MicroWorld, day: u64, day_type: u8, in_country: bool) -> Option<StreetDay> {
    let days = person_days(w, day, day_type, in_country)?;
    let homes = w.household_groups.len();
    let cells = homes + 3;
    let mut counts = vec![vec![0i64; cells]; 24];
    let mut home_awake = vec![vec![0u16; homes]; 24];
    let mut moves: Vec<std::collections::BTreeMap<(usize, usize), i64>> = vec![std::collections::BTreeMap::new(); 23];
    let children: u32 = w.household_groups.iter().map(|g| u32::from(g.unwrap_or([0, 0, 0])[2])).sum();
    let (persons, employed) = (days.len() as u32, days.iter().filter(|d| d.employed).count() as u32);
    let verdict = days.last().map_or((2u8, "no person with a day"), |d| d.verdict);
    let mut residents = vec![0u16; homes];
    for d in &days {
        let hi = d.household;
        residents[hi] += 1;
        for s in 0..24 {
            counts[s][cell_of(d.states[s], hi, homes)] += 1;
            if d.states[s] == 1 {
                home_awake[s][hi] += 1;
            }
            if s < 23 {
                let (a, b) = (cell_of(d.states[s], hi, homes), cell_of(d.states[s + 1], hi, homes));
                if a != b {
                    *moves[s].entry((a, b)).or_insert(0) += 1;
                }
            }
        }
    }
    let flows = moves.into_iter().map(|m| m.into_iter().map(|((a, b), n)| (a, b, n)).collect()).collect();
    let series = Series { cells, counts, flows, source: vec![vec![0; cells]; 23] };
    let violations = check_continuity(&series).len();
    Some(StreetDay { series, homes, persons, children, employed, verdict, violations, home_awake, residents })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::micro::{DeviceRule, MicroParams, HOME_ON_GATEWAY};
    use crate::tiling::{HolonId, LeafContext, LeafLayout};

    fn world(la: f64, lo: f64, rule: Option<DeviceRule>) -> MicroWorld {
        let p = MicroParams::default();
        let leaf = LeafLayout::build(HolonId::at(8, la, lo), &LeafContext { budget: 40, chain: [3_000; 11], ..LeafContext::default() }, &p);
        let dwellings: usize = leaf.buildings.iter().map(|b| usize::from(b.units)).sum();
        let sizes: Vec<u8> = (0..dwellings).map(|i| 1 + (i % 4) as u8).collect();
        MicroWorld::from_leaf_with(&leaf, p, sizes, rule)
    }

    #[test]
    fn devices_sleep_with_their_people_inside_the_country_only() {
        use crate::micro::{Activity, HostKind, P};
        let asleep_at = |hour: f64, in_country: bool| {
            let p = { let mut p = MicroParams::default(); p.set(P::StartLocalHour, hour); p };
            let leaf = LeafLayout::build(HolonId::at(8, 40.06, -83.05), &LeafContext { budget: 40, chain: [3_000; 11], ..LeafContext::default() }, &p);
            let dwellings: usize = leaf.buildings.iter().map(|b| usize::from(b.units)).sum();
            let sizes: Vec<u8> = (0..dwellings).map(|i| 1 + (i % 4) as u8).collect();
            let mut w = MicroWorld::from_leaf_with(&leaf, p, sizes, Some(DeviceRule { home_node_where: HOME_ON_GATEWAY, in_country }));
            w.set_presence(7, 1);
            assert_eq!(w.presence.is_some(), in_country);
            w.step(4.0);
            let phones: Vec<_> = w.nodes.iter().filter(|n| n.kind == HostKind::Phone && n.household.is_some() && n.awake).collect();
            (phones.iter().filter(|n| n.activity == Activity::Asleep).count(), phones.len())
        };
        let (night, n) = asleep_at(3.0, true);
        let (noon, _) = asleep_at(13.0, true);
        assert!(n > 10, "{n} phones");
        assert!(night * 10 >= n * 7, "3 a.m.: {night} of {n} asleep");
        assert!(noon * 10 <= n * 2, "1 p.m.: {noon} of {n} asleep");
        // outside the country nothing changes: no presence, the stand-in alone
        assert_eq!(asleep_at(3.0, false).0, 0);
    }

    /// CIRISHolon published PRESENCEGEN-2's digest (`PRESENCEGEN2_RESULTS.md`: one value native and wasm32);
    /// docs/geo/holon/presencegen_digest.txt carries it and presencegen.test.js checks the page's wasm against it.
    #[test]
    fn the_presence_digest_is_holons() {
        let want: u64 = include_str!("../../../docs/geo/holon/presencegen_digest.txt").trim().parse().unwrap();
        assert_eq!(holon_scene::presence::digest(), want);
    }

    #[test]
    fn a_streets_day_keeps_every_person_and_nobody_appears() {
        let w = world(40.06, -83.05, Some(DeviceRule { home_node_where: HOME_ON_GATEWAY, in_country: true }));
        for (day, dt) in [(0u64, 0u8), (1, 1), (5, 2), (6, 3)] {
            let d = street_day(&w, day, dt, true).expect("composition drawn");
            assert_eq!(d.violations, 0, "continuity with source zero");
            assert!(holon_scene::rhythm::totals(&d.series).iter().all(|&t| t == i64::from(d.persons)), "every slot holds every person");
            let people: u32 = w.households.iter().map(|h| u32::from(h.persons)).sum();
            assert_eq!(d.persons + d.children, people, "persons with a day and children are the street's people");
            assert_eq!(d.verdict.0, 0, "the chain is certified inside the United States");
            assert_eq!(d.residents.iter().map(|&r| u32::from(r)).sum::<u32>(), d.persons);
            for s in 0..24 {
                for hi in 0..d.homes {
                    assert!(d.series.counts[s][hi] <= i64::from(d.residents[hi]) && i64::from(d.home_awake[s][hi]) <= d.series.counts[s][hi]);
                }
            }
            // at 4 a.m. most are home; by 1 p.m. on a working day some are at work
            let home = |s: usize| d.series.counts[s][..d.homes].iter().sum::<i64>();
            assert!(home(0) * 10 >= i64::from(d.persons) * 8, "slot 0 (4 a.m.): {} of {} home", home(0), d.persons);
            if dt == 0 {
                assert!(d.series.counts[9][d.homes + AWAY_WORK] > 0, "1 p.m. on a weekday: someone at work");
            }
        }
        // the same day twice is the same series; another day is another draw
        assert_eq!(street_day(&w, 3, 0, true).unwrap().series, street_day(&w, 3, 0, true).unwrap().series);
        assert_ne!(street_day(&w, 3, 0, true).unwrap().series, street_day(&w, 4, 0, true).unwrap().series);
        // outside the country: generated, unresolved
        assert_eq!(street_day(&w, 0, 0, false).unwrap().verdict.0, 2);
        // the legacy device draw has no composition, so no series
        assert!(street_day(&world(40.06, -83.05, None), 0, 0, true).is_none());
    }
}
