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
            // a phone whose person sleeps is offline (it follows its person's day), so count offline or asleep
            let phones: Vec<_> = w.nodes.iter().filter(|n| n.kind == HostKind::Phone && n.household.is_some()).collect();
            let resting = phones.iter().filter(|n| !n.awake || n.activity == Activity::Asleep).count();
            let awake_asleep = phones.iter().filter(|n| n.awake && n.activity == Activity::Asleep).count();
            (resting, phones.len(), awake_asleep)
        };
        let (night, n, _) = asleep_at(3.0, true);
        let (noon, _, _) = asleep_at(13.0, true);
        assert!(n > 10, "{n} phones");
        assert!(night * 10 >= n * 7, "3 a.m.: {night} of {n} resting");
        assert!(noon * 10 <= n * 2, "1 p.m.: {noon} of {n} resting");
        // outside the country nothing changes: no presence, the stand-in alone, no online phone asleep
        assert_eq!(asleep_at(3.0, false).2, 0);
    }

    /// One generated day on a Columbus street (4 a.m. to 4 a.m., Tuesday), minute by minute: rows phones and laptops
    /// sign per local hour, their awake device-seconds, rows signed while the sender's person was asleep at both ends of
    /// the minute, the seat's queue, and the late counts. `curve`: the second setting (send_by_curve 1); `presence`:
    /// false runs the stand-in alone (no generated days).
    struct DayRun { sends: [u64; 24], awake_s: [f64; 24], asleep_sends: u64, seat_peak: usize, seat_2330: usize, total: u64, held: u64, late: u64, under: Vec<f64> }
    fn run_day(curve: bool, presence: bool) -> DayRun {
        use crate::micro::{HostKind, Route, P};
        let p = { let mut p = MicroParams::default(); p.set(P::StartLocalHour, 4.0); p.set(P::SendByCurve, if curve { 1.0 } else { 0.0 }); p };
        let leaf = LeafLayout::build(HolonId::at(8, 40.06, -83.05), &LeafContext { budget: 40, chain: [3_000; 11], ..LeafContext::default() }, &p);
        let dwellings: usize = leaf.buildings.iter().map(|b| usize::from(b.units)).sum();
        let sizes: Vec<u8> = (0..dwellings).map(|i| 1 + (i % 4) as u8).collect();
        let mut w = MicroWorld::from_leaf_with(&leaf, p, sizes, Some(DeviceRule { home_node_where: HOME_ON_GATEWAY, in_country: true }));
        if presence {
            w.set_presence(7, 1);
        }
        let person_dev = |w: &MicroWorld, i: usize| matches!(w.nodes[i].kind, HostKind::Phone | HostKind::Laptop);
        let mut r = DayRun { sends: [0; 24], awake_s: [0.0; 24], asleep_sends: 0, seat_peak: 0, seat_2330: 0, total: 0, held: 0, late: 0, under: vec![] };
        for minute in 0..24 * 60 {
            let hour = w.local_hour() as usize % 24;
            let asleep0: Vec<bool> = (0..w.nodes.len()).map(|i| w.person_awake(i).is_some_and(|p| !p.0)).collect();
            for i in 0..w.nodes.len() {
                let n = &w.nodes[i];
                if person_dev(&w, i) && n.online(w.t) && !n.pinned && w.person_awake(i).is_none_or(|p| p.0) {
                    r.awake_s[hour] += 60.0;
                }
            }
            w.events.clear();
            w.step(60.0);
            for e in &w.events {
                if e.route == Route::MailDelivered || !person_dev(&w, e.src) {
                    continue;
                }
                r.sends[hour] += 1;
                r.asleep_sends += u64::from(asleep0[e.src] && w.person_awake(e.src).is_some_and(|p| !p.0));
            }
            let q = w.held_at_seats();
            r.seat_peak = r.seat_peak.max(q);
            if minute == (23 * 60 + 30) - 4 * 60 {
                r.seat_2330 = q;
            }
        }
        r.total = r.sends.iter().sum();
        r.held = w.stats.mail_delivered;
        r.late = w.stats.late;
        r.under = w.hold_shares_under(&[1.0, 4.0, 8.0, 24.0]);
        r
    }
    /// Hours whose count is off the day's pooled rate per awake device-second by more than four standard deviations
    /// (Poisson), among hours expecting at least 30 rows.
    fn off_rate_hours(r: &DayRun) -> Vec<usize> {
        let rate = r.total as f64 / r.awake_s.iter().sum::<f64>();
        (0..24).filter(|&h| { let lam = rate * r.awake_s[h]; lam >= 30.0 && (r.sends[h] as f64 - lam).abs() > 4.0 * lam.sqrt() + 2.0 }).collect()
    }

    /// Where people have generated days, each awake device sends at one flat rate: no row from a device whose person is
    /// asleep, and the rows per awake device-hour are the same every hour within sampling error. The second setting
    /// (the day curve's total poured onto the awake) is not flat, which is what makes the first a check at all.
    #[test]
    fn each_persons_day_leads_sending_at_a_flat_rate_per_awake_hour() {
        let day = run_day(false, true);
        assert_eq!(day.asleep_sends, 0, "no row from a device whose person is asleep");
        assert!(day.total > 500, "{} rows", day.total);
        assert_eq!(off_rate_hours(&day), Vec::<usize>::new(), "the rate per awake device-hour is flat: {:?} / {:?}", day.sends, day.awake_s);
        let curve = run_day(true, true);
        assert_eq!(curve.asleep_sends, 0);
        assert!(!off_rate_hours(&curve).is_empty(), "the curve setting is not flat per awake hour");
    }

    /// The switch moves the late evening the way the numbers say: the curve stays near or above 1 from 22:00 while the
    /// awake share falls below a half, so the curve setting sends more late in the evening and the seat holds more then.
    #[test]
    fn the_curve_setting_sends_more_late_in_the_evening_and_the_seat_holds_more() {
        let (day, curve) = (run_day(false, true), run_day(true, true));
        let late = |r: &DayRun| r.sends[21] + r.sends[22] + r.sends[23];
        assert!(late(&curve) > late(&day), "late-evening rows: curve {} against each person's day {}", late(&curve), late(&day));
        assert!(curve.seat_2330 >= day.seat_2330, "the seat's queue at 23:30: curve {} against each person's day {}", curve.seat_2330, day.seat_2330);
    }

    /// The two-setting table for the record (cargo test -p cewp-sim send_rule_table -- --ignored --nocapture).
    #[test]
    #[ignore = "a report, not a check"]
    fn send_rule_table() {
        for (name, curve, presence) in [("each person's day (default)", false, true), ("the assumed curve on people's days", true, true), ("the stand-in alone (no days)", false, false)] {
            let r = run_day(curve, presence);
            println!("{name}: rows a day {}, late evening (21-24 h) {}, seat queue peak {} (23:30: {}), held rows delivered {}, late {} ({:.3}), under 1/4/8/24 h {:?}, by hour {:?}",
                r.total, r.sends[21] + r.sends[22] + r.sends[23], r.seat_peak, r.seat_2330, r.held, r.late, r.late as f64 / r.held.max(1) as f64,
                r.under.iter().map(|x| (x * 1000.0).round() / 1000.0).collect::<Vec<_>>(), r.sends);
        }
    }

    /// One assumption of the street, or MESH-0's in its place, for the decomposition against MESH-0's hold times.
    #[derive(Clone, Copy, Default)]
    struct M0 { follow_h: f64, people_only: bool, phones: bool, other_households: bool, independent: bool }
    /// Messages people's devices send over one generated day on the Columbus street (each person's day leads sending),
    /// followed `follow_h` more hours: [sent, taken at once, held, held and delivered], the delivered waits (hours), and
    /// the share of the street's adults with no phone.
    fn mesh0_read(c: M0) -> ([u64; 4], Vec<f64>, f64) {
        use crate::micro::{HostKind, Route, P};
        use std::collections::{HashMap, VecDeque};
        let p = {
            let mut p = MicroParams::default();
            p.set(P::StartLocalHour, 4.0);
            if c.other_households {
                p.set(P::FamilyShare, 0.0);
                p.set(P::CommunityShare, 0.95);
            }
            p
        };
        let leaf = LeafLayout::build(HolonId::at(8, 40.06, -83.05), &LeafContext { budget: 40, chain: [3_000; 11], ..LeafContext::default() }, &p);
        let dwellings: usize = leaf.buildings.iter().map(|b| usize::from(b.units)).sum();
        // the page's composition: household sizes from CIRISHolon's generator (US 2024 rows, the street's density class)
        let class = crate::popchain::households::holon_class(3_000.0);
        let drawn = holon_scene::household::generate(0x00C0_1B05, (dwellings as u64) * 3, class, holon_scene::household::Reference::UnitedStates2024, true).sizes;
        let sizes: Vec<u8> = (0..dwellings).map(|i| drawn[i % drawn.len().max(1)]).collect();
        let mut w = MicroWorld::from_leaf_with(&leaf, p, sizes, Some(DeviceRule { home_node_where: HOME_ON_GATEWAY, in_country: true }));
        w.set_presence(7, 1);
        w.address_phones = c.phones;
        if c.independent {
            // each adult takes the day of an adult in another household (a fixed derangement by a seeded shuffle), so
            // members of one household no longer share their hours
            let pr = w.presence.as_mut().expect("presence");
            let mut all: Vec<[u8; 24]> = pr.days.iter().flatten().copied().collect();
            let n = all.len();
            let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
            for i in (1..n).rev() {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                all.swap(i, (x % (i as u64 + 1)) as usize);
            }
            let mut k = 0;
            for d in &mut pr.days { for s in d.iter_mut() { *s = all[k]; k += 1; } }
        }
        // adults with no phone, among the households that run CIRIS (the others have no devices on the street at all)
        let ad = |h: usize| w.households[h].adopting && !w.households[h].nodes.is_empty();
        let adults: usize = w.presence.as_ref().map_or(0, |p| p.days.iter().enumerate().filter(|&(h, _)| ad(h)).map(|(_, d)| d.len()).sum());
        let with_phone: usize = w.presence.as_ref().map_or(0, |p| p.days.iter().enumerate().filter(|&(h, _)| ad(h)).map(|(h, d)| (0..d.len()).filter(|&o| w.households[h].nodes.iter().any(|&j| w.nodes[j].kind == HostKind::Phone && usize::from(w.nodes[j].owner) == o)).count()).sum());
        let person = |w: &MicroWorld, i: usize| matches!(w.nodes[i].kind, HostKind::Phone | HostKind::Laptop);
        let (mut n, mut waits, mut open): ([u64; 4], Vec<f64>, HashMap<usize, VecDeque<f64>>) = ([0; 4], Vec::new(), HashMap::new());
        let minutes = ((24.0 + c.follow_h) * 60.0) as usize;
        for m in 0..minutes {
            w.events.clear();
            w.step(60.0);
            let sending = m < 24 * 60;
            for e in &w.events {
                if e.route == Route::MailDelivered {
                    if let Some(q) = open.get_mut(&e.dst) {
                        if let Some(t0) = q.pop_front() { waits.push((e.t - t0) / 3600.0); n[3] += 1; }
                    }
                    continue;
                }
                if !sending || !person(&w, e.src) || (c.people_only && !person(&w, e.dst)) {
                    continue;
                }
                n[0] += 1;
                if e.route == Route::Mailboxed { n[2] += 1; open.entry(e.dst).or_default().push_back(e.t); } else { n[1] += 1; }
            }
        }
        (n, waits, 1.0 - with_phone as f64 / adults.max(1) as f64)
    }
    fn m0_summary(r: &([u64; 4], Vec<f64>, f64)) -> [f64; 5] {
        let (n, w, _) = r; let mut v = w.clone(); v.sort_by(f64::total_cmp);
        let q = |f: f64| if v.is_empty() { 0.0 } else { v[((f * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1] };
        // shares of the delivered held rows (the page's base and MESH-0's, which followed every message to delivery)
        let over8 = v.iter().filter(|&&x| x > 8.0).count() as f64 / v.len().max(1) as f64;
        [n[1] as f64 / n[0].max(1) as f64, q(0.25), q(0.5), q(0.75), over8]
    }

    /// The decomposition of the street's hold times against MESH-0's, one assumption at a time (a report; cargo test -p
    /// cewp-sim mesh0_decomposition -- --ignored --nocapture).
    #[test]
    #[ignore = "a report, not a check"]
    fn mesh0_decomposition() {
        // every switch after the first is measured on the followed day, so that truncation does not confound it
        let f = M0 { follow_h: 24.0, ..M0::default() };
        let rows: [(&str, M0); 8] = [
            ("the page: one day, held rows counted if delivered within it", M0::default()),
            ("follow the day's rows 24 h more", f),
            ("+ addressed to people only (phones, laptops)", M0 { people_only: true, ..f }),
            ("+ a person, reached at their phone", M0 { phones: true, ..f }),
            ("+ addressees outside the sender's household", M0 { other_households: true, ..f }),
            ("+ household members' days independent", M0 { independent: true, ..f }),
            ("all of MESH-0's assumptions together", M0 { people_only: true, phones: true, other_households: true, independent: true, ..f }),
            ("all but 'outside the household' (MESH-0's ties may include it)", M0 { people_only: true, phones: true, independent: true, ..f }),
        ];
        println!("MESH-0: at once 0.858, held waits 2 / 4 / 8 h, over 8 h 0.23");
        for (name, c) in rows {
            let r = mesh0_read(c); let s = m0_summary(&r);
            println!("{name}: sent {}, at once {:.3}, held {} (delivered {}), waits {:.1} / {:.1} / {:.1} h, over 8 h {:.3}; adults with no phone {:.3}",
                r.0[0], s[0], r.0[2], r.0[3], s[1], s[2], s[3], s[4], r.2);
        }
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
