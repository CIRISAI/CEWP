//! The globe's failures layer (HOOKS.md §2): one correlated outage stepping through time on the
//! failure package's generated world ([`crate::failure`]), as flat buffers for `kernel-wasm`.
//!
//! Everything here is MODEL: the world is `DomainWorld::build` (twelve generated countries,
//! Zipf metros, four ISPs each), the event is one of `failure`'s scenarios with its default
//! duration, and who is up and which network component each survivor is in are
//! [`failure::evaluate`]'s own outputs at each hour. This module adds no failure logic; it picks
//! three named scenarios, evaluates them at a grid of hours, and places the generated world on
//! the page's localities for display.
//!
//! # Placement (display only, MODEL, no meaning)
//!
//! The failure world's countries and metros are generated indices with no coordinates. To draw
//! them, generated country `c` is given region `(c + 6) mod 9` of the page's nine regions
//! ([`crate::netgraph::REGION_NAMES`]); its anchor is the heaviest locality of that region not
//! yet taken, and each further metro is the free locality of that region farthest from the ones
//! already placed (farthest-point spread). A generated country therefore spans a whole region,
//! across several real countries, and is none of them. Nodes sit on a sunflower spiral
//! (`NODE_RING_DEG` x sqrt(k) degrees) around their metro's locality. No position is an address.
//!
//! # Buffers
//!
//! * `meta` f32 x 10: `[nodes, steps, scenario, pre-onset steps, duration h, ramp start h,
//!   target, countries, metros, horizon h]`. `target` is the country (shutdown), access
//!   network (ISP) or power region (power) the event hits.
//! * `nodes` f32 x 4 per node: `[lat, lon, kind (failure::NodeKind: 0 phone, 1 home node,
//!   2 laptop, 3 hosted, 4 seat), country]`.
//! * `metros` f32 x 4 per metro: `[lat, lon, country, nodes]`.
//! * `steps` f32 x 7 per step: `[hour from onset, up, in the largest component, up but cut
//!   off, components among up nodes, down because of the event, withholding]`.
//! * `state` u32, `steps x nodes`: `0` down because of the event (up in the baseline),
//!   `1` up in the largest component, `2` up but cut off from it, `3` up and withholding,
//!   `4` not up in the baseline either (asleep, or failed to rejoin with nothing failed).
//! * `comp` u32, `steps x nodes`: [`failure::view_event_components`]'s dense component label,
//!   `u32::MAX` when the node is not up.
//! * `cables` u32 x 2 per international link: `[country, second country or u32::MAX]` (a link
//!   that lands in one country joins it to the global core).
//!
//! Steps: [`PRE_STEPS`] before onset (the baseline, nothing failed), then [`POST_STEPS`] from
//! onset to `horizon = 1.25 x duration`, at the same hours as
//! `failure::view_event_steps(.., POST_STEPS, horizon)` (the native test checks the two agree).

use crate::failure::{self, evaluate, AsnKind, Condition, DomainWorld, Event, FailureParams, ScenarioKind, F};
use crate::netgraph::LOCALITIES;
use std::collections::BTreeMap;

/// Steps shown before onset.
pub const PRE_STEPS: usize = 2;
/// Steps from onset to the horizon.
pub const POST_STEPS: usize = 41;
/// Spiral spacing of nodes around their metro, degrees (display only).
pub const NODE_RING_DEG: f64 = 0.09;
/// Nodes in the view's world (the failure package's own demo size).
pub const VIEW_NODES: f64 = 2_000.0;
/// World seed (the failure package's `view_demo` seed).
pub const VIEW_SEED: u64 = 2026;
/// Scenario names, index = the view's `scenario` argument.
pub const VIEW_SCENARIOS: [&str; 3] = ["national shutdown", "nationwide ISP outage", "regional power failure"];

/// The flat buffers of one scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureView {
    /// See the module doc.
    pub meta: Vec<f32>,
    /// See the module doc.
    pub nodes: Vec<f32>,
    /// See the module doc.
    pub metros: Vec<f32>,
    /// See the module doc.
    pub steps: Vec<f32>,
    /// See the module doc.
    pub state: Vec<u32>,
    /// See the module doc.
    pub comp: Vec<u32>,
    /// See the module doc.
    pub cables: Vec<u32>,
}

fn count_by<T: Ord + Copy>(it: impl Iterator<Item = T>) -> BTreeMap<T, u32> {
    let mut m = BTreeMap::new();
    for x in it {
        *m.entry(x).or_insert(0) += 1;
    }
    m
}

/// The world, parameters and event of scenario `scenario` (0 shutdown, 1 ISP, 2 power; others
/// clamp to 2), and the event's target.
///
/// * 0: a full national shutdown (international links and domestic cellular) of the second
///   most populous generated country, for `shutdown.median_h` (MODEL, 48 h), Step return.
///   The second, so the shut country is an island beside a mainland, not the mainland itself.
/// * 1: the most loaded fixed-line ISP of the most populous country down nationwide for
///   `isp.median_h` (MODEL, 4 h), Step return.
/// * 2: `failure::view_demo`: the most populous grid region dark for 12 h, linear return.
#[must_use]
pub fn failure_view_scenario(scenario: u32) -> (DomainWorld, FailureParams, Event, u32) {
    match scenario {
        0 | 1 => {
            let mut p = FailureParams::default();
            p.set(F::Nodes, VIEW_NODES);
            let w = DomainWorld::build(VIEW_SEED, &p);
            let by_country = count_by(w.nodes.iter().map(|n| n.country));
            let mut ranked: Vec<(u32, u16)> = by_country.iter().map(|(&c, &k)| (k, c)).collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            if scenario == 0 {
                let c = ranked.get(1).or(ranked.first()).map_or(0, |x| x.1);
                let mut e = Event::manual(ScenarioKind::NationalShutdown, VIEW_SEED, p.get(F::ShutMedianH));
                e.shutdown.push(c);
                e.shutdown_mobile = true;
                e.scope = format!("country {c}");
                (w, p, e, u32::from(c))
            } else {
                let c = ranked.first().map_or(0, |x| x.1);
                let a = (0..w.asns.len())
                    .filter(|&a| w.asns[a].kind == AsnKind::Fixed && w.asns[a].country == c)
                    .max_by_key(|&a| (w.asn_load[a], usize::MAX - a))
                    .unwrap_or(0) as u32;
                let mut e = Event::manual(ScenarioKind::IspOutage, VIEW_SEED, p.get(F::IspMedianH));
                e.asns.push(a);
                e.scope = format!("fixed asn {a} (country {c})");
                (w, p, e, a)
            }
        }
        _ => {
            let (w, p, e) = failure::view_demo();
            let r = e.power.first().copied().unwrap_or(0);
            (w, p, e, r)
        }
    }
}

/// Display positions of the generated metros (module doc, "Placement"): `(lat, lon)` per metro.
#[must_use]
pub fn metro_positions(w: &DomainWorld) -> Vec<(f64, f64)> {
    let loc: Vec<(f64, f64, u32, u8)> =
        LOCALITIES.iter().map(|&(la, lo, wt, r)| (f64::from(la) / 100.0, f64::from(lo) / 100.0, wt, r)).collect();
    let mut taken = vec![false; loc.len()];
    let mut pos = vec![(0.0, 0.0); w.metros.len()];
    for c in 0..w.countries {
        // region (c + 6) mod 9: display only; it puts the view's shut country (index 5) across Europe,
        // where a generated country plainly spans many real ones
        let region = ((c + 6) % 9) as u8;
        let mine: Vec<usize> = (0..w.metros.len()).filter(|&m| w.metros[m].country == c).collect();
        if mine.is_empty() {
            continue;
        }
        let pool = |taken: &[bool]| (0..loc.len()).filter(|&k| !taken[k] && loc[k].3 == region).collect::<Vec<_>>();
        let mut cand = pool(&taken);
        if cand.is_empty() {
            cand = (0..loc.len()).filter(|&k| !taken[k]).collect();
        }
        let Some(&anchor) = cand.iter().max_by_key(|&&k| (loc[k].2, usize::MAX - k)) else { continue };
        // squared degrees between two localities, longitude by cos lat
        let d2 = |a: usize, b: usize| {
            let dla = loc[a].0 - loc[b].0;
            let dlo = (((loc[a].1 - loc[b].1) + 540.0) % 360.0 - 180.0) * loc[a].0.to_radians().cos();
            dla * dla + dlo * dlo
        };
        let mut placed: Vec<usize> = Vec::new();
        for (j, &m) in mine.iter().enumerate() {
            let k = if j == 0 {
                anchor
            } else {
                // the free locality of the region farthest from every one already placed
                cand.iter()
                    .copied()
                    .filter(|&k| !taken[k])
                    .max_by(|&a, &b| {
                        let da = placed.iter().map(|&q| d2(a, q)).fold(f64::MAX, f64::min);
                        let db = placed.iter().map(|&q| d2(b, q)).fold(f64::MAX, f64::min);
                        da.total_cmp(&db).then(b.cmp(&a))
                    })
                    .unwrap_or(anchor)
            };
            taken[k] = true;
            placed.push(k);
            pos[m] = (loc[k].0, loc[k].1);
        }
    }
    pos
}

fn state_codes(base: &failure::State, st: &failure::State) -> Vec<u32> {
    (0..st.up.len())
        .map(|i| match (base.up[i], st.up[i], st.in_lcc[i], st.withholding[i]) {
            (false, false, _, _) => 4,
            (true, false, _, _) => 0,
            (_, true, _, true) => 3,
            (_, true, true, false) => 1,
            (_, true, false, false) => 2,
        })
        .collect()
}

fn dense_components(st: &failure::State) -> Vec<u32> {
    let mut dense: BTreeMap<u32, u32> = BTreeMap::new();
    (0..st.up.len())
        .map(|i| {
            if st.up[i] {
                let next = dense.len() as u32;
                *dense.entry(st.comp[i]).or_insert(next)
            } else {
                u32::MAX
            }
        })
        .collect()
}

/// The hours the view evaluates, relative to onset: [`PRE_STEPS`] before, [`POST_STEPS`] from
/// onset to `horizon` inclusive.
#[must_use]
pub fn view_hours(horizon_h: f64) -> Vec<f64> {
    let dt = horizon_h / (POST_STEPS - 1) as f64;
    let mut v: Vec<f64> = (0..PRE_STEPS).map(|k| -dt * (PRE_STEPS - k) as f64 * 2.0).collect();
    v.extend((0..POST_STEPS).map(|k| k as f64 * horizon_h / (POST_STEPS - 1) as f64));
    v
}

/// Every buffer of scenario `scenario` (module doc).
#[must_use]
pub fn failure_view(scenario: u32) -> FailureView {
    let (w, p, e, target) = failure_view_scenario(scenario);
    let n = w.nodes.len();
    let horizon = 1.25 * e.duration_h;
    let hours = view_hours(horizon);
    let evs = std::slice::from_ref(&e);
    let base = evaluate(&w, &p, &Condition { events: &[], t_h: 0.0, removed: None, rejoin: true });
    let pos = metro_positions(&w);
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    let mut seen = vec![0u32; w.metros.len()];
    let mut nodes = Vec::with_capacity(n * 4);
    for nd in &w.nodes {
        let m = nd.metro as usize;
        let k = f64::from(seen[m]);
        seen[m] += 1;
        let (la, lo) = pos[m];
        let r = NODE_RING_DEG * (k + 0.5).sqrt();
        let a = k * golden;
        let la2 = la + r * a.sin();
        let lo2 = lo + r * a.cos() / la.to_radians().cos().max(0.2);
        nodes.extend_from_slice(&[la2 as f32, lo2 as f32, nd.kind as u32 as f32, f32::from(nd.country)]);
    }
    let mut metros = Vec::with_capacity(w.metros.len() * 4);
    for (m, mt) in w.metros.iter().enumerate() {
        metros.extend_from_slice(&[pos[m].0 as f32, pos[m].1 as f32, f32::from(mt.country), seen[m] as f32]);
    }
    let mut steps = Vec::with_capacity(hours.len() * 7);
    let mut state = Vec::with_capacity(hours.len() * n);
    let mut comp = Vec::with_capacity(hours.len() * n);
    for &t in &hours {
        let st = if t < 0.0 { base.clone() } else { evaluate(&w, &p, &Condition { events: evs, t_h: t, removed: None, rejoin: true }) };
        let codes = state_codes(&base, &st);
        let labels = dense_components(&st);
        let up = st.up.iter().filter(|&&u| u).count();
        let cut = codes.iter().filter(|&&c| c == 2).count();
        let down = codes.iter().filter(|&&c| c == 0).count();
        let wh = codes.iter().filter(|&&c| c == 3).count();
        let ncomp = labels.iter().filter(|&&l| l != u32::MAX).max().map_or(0, |&m| m + 1);
        steps.extend_from_slice(&[t as f32, up as f32, st.lcc as f32, cut as f32, ncomp as f32, down as f32, wh as f32]);
        state.extend(codes);
        comp.extend(labels);
    }
    let meta = vec![
        n as f32,
        hours.len() as f32,
        scenario.min(2) as f32,
        PRE_STEPS as f32,
        e.duration_h as f32,
        e.ramp_start_h as f32,
        target as f32,
        f32::from(w.countries),
        w.metros.len() as f32,
        horizon as f32,
    ];
    let mut cables = Vec::with_capacity(w.cables.len() * 2);
    for c in &w.cables {
        cables.push(u32::from(c.landings.first().copied().unwrap_or(0)));
        cables.push(c.landings.get(1).map_or(u32::MAX, |&x| u32::from(x)));
    }
    FailureView { meta, nodes, metros, steps, state, comp, cables }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_view_post_onset_matches_the_package_functions() {
        for sc in 0..3u32 {
            let v = failure_view(sc);
            let (w, p, e, _) = failure_view_scenario(sc);
            let n = w.nodes.len();
            let horizon = 1.25 * e.duration_h;
            let comps = failure::view_event_components(&w, &p, &e, POST_STEPS, horizon);
            assert_eq!(v.comp[PRE_STEPS * n..].to_vec(), comps, "scenario {sc}: components");
            let steps = failure::view_event_steps(&w, &p, &e, POST_STEPS, horizon);
            for k in 0..POST_STEPS {
                let blk = &steps[k * (3 + n)..(k + 1) * (3 + n)];
                let mine = &v.steps[(PRE_STEPS + k) * 7..(PRE_STEPS + k + 1) * 7];
                assert_eq!(f32::from_bits(blk[0]), mine[0], "scenario {sc} step {k}: hour");
                assert_eq!((blk[1] as f32, blk[2] as f32), (mine[1], mine[2]), "scenario {sc} step {k}: up, lcc");
                for i in 0..n {
                    // the package's 0 (down) is this view's 0 or 4 (down in the baseline too)
                    let s = v.state[(PRE_STEPS + k) * n + i];
                    assert_eq!(blk[3 + i], if s == 4 { 0 } else { s }, "scenario {sc} step {k} node {i}");
                }
            }
        }
    }

    #[test]
    fn failure_view_scenarios_bite_and_recover() {
        for sc in 0..3u32 {
            let v = failure_view(sc);
            let n = v.meta[0] as usize;
            let s = v.meta[1] as usize;
            assert_eq!((v.nodes.len(), v.state.len(), v.comp.len(), v.steps.len()), (n * 4, n * s, n * s, s * 7));
            assert_eq!(v.metros.len(), v.meta[8] as usize * 4);
            // before onset nothing is down because of the event; one step after onset something is
            assert!(v.steps[5] == 0.0 && v.steps[7 + 5] == 0.0, "scenario {sc}: baseline");
            let after = &v.steps[(PRE_STEPS + 1) * 7..(PRE_STEPS + 2) * 7];
            assert!(after[5] > 0.0 || after[3] > 0.0, "scenario {sc}: the event bites: {after:?}");
            // past the duration everything is back
            let last = &v.steps[(s - 1) * 7..s * 7];
            assert_eq!(last[5], 0.0, "scenario {sc}: recovered");
            assert_eq!(last[1], v.steps[1], "scenario {sc}: as many up as before");
        }
        // the shutdown makes the shut country an island beside the mainland
        let v = failure_view(0);
        let k = PRE_STEPS + 1;
        assert!(v.steps[k * 7 + 3] > 0.0 && v.steps[k * 7 + 4] >= 2.0, "{:?}", &v.steps[k * 7..k * 7 + 7]);
    }

    #[test]
    fn failure_view_is_deterministic_and_placed_on_localities() {
        assert_eq!(failure_view(0), failure_view(0));
        let v = failure_view(2);
        for m in v.metros.chunks(4) {
            assert!(LOCALITIES.iter().any(|&(la, lo, _, _)| (f64::from(la) / 100.0) as f32 == m[0] && (f64::from(lo) / 100.0) as f32 == m[1]));
        }
        assert!(!v.cables.is_empty() && v.cables.len() % 2 == 0);
        // a generated country's metros never share a locality with another country's
        let mut at: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for m in v.metros.chunks(4) {
            let c = *at.entry((m[0].to_bits(), m[1].to_bits())).or_insert(m[2] as u32);
            assert_eq!(c, m[2] as u32);
        }
    }
}
