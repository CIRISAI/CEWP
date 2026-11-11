//! Counterfactuals edge does not have: a UPnP / PCP port-mapping client and an IPv6 listener.
//! Run on exactly the worlds and seeds of the regime table in [`super::report`] (256 correlated
//! default-shape metros, 400 community pairs each; 32 metros = 256 streets for the per-street
//! spread). Every number this module prints is MODEL.
//!
//! Cases: (a) as shipped; (b) `port_mapping_client` = 1 and `port_mapping_punch` = 1 (the client
//! maps the listener and the punch socket); (b-l) the listener only, to split (b)'s gain; (c)
//! `ipv6_listener` = 1; (d) (b) and (c).

use super::hooks::{classes, NodeClass};
use super::{
    mix, neighbourhood_distribution, Correlation, Dist, Mechanism, ReachModel, ReachParams, V6Fw, World, WorldShape, METROS,
    PAIR_MIX, R,
};
use crate::workload::ParameterRegime;

/// One case's switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Case {
    /// Label.
    pub name: &'static str,
    /// `port_mapping_client`.
    pub client: bool,
    /// `port_mapping_punch` (meaningful with `client`).
    pub punch: bool,
    /// `ipv6_listener`.
    pub v6_listener: bool,
}

/// The cases, in report order.
pub const CASES: [Case; 5] = [
    Case { name: "(a) as shipped", client: false, punch: false, v6_listener: false },
    Case { name: "(b-l) UPnP/PCP, listener only", client: true, punch: false, v6_listener: false },
    Case { name: "(b) UPnP/PCP, listener + punch socket", client: true, punch: true, v6_listener: false },
    Case { name: "(c) IPv6 listener", client: false, punch: false, v6_listener: true },
    Case { name: "(d) (b) + (c)", client: true, punch: true, v6_listener: true },
];

/// Parameters of `case` on top of `base`.
#[must_use]
pub fn params(base: &ReachParams, case: Case) -> ReachParams {
    let mut p = base.clone();
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    p.set(R::PortMappingClient, b(case.client));
    p.set(R::PortMappingPunch, b(case.punch));
    p.set(R::Ipv6Listener, b(case.v6_listener));
    p
}

/// What one case gives on the regime table's worlds.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    /// Weighted relay share across metros.
    pub relay: Dist,
    /// Per-street community relay share.
    pub street: Dist,
    /// Weighted mechanism mix, [`Mechanism::ALL`] order (Relay = no mechanism at all; a punch
    /// mechanism still fails with `1 - punch_success_share`).
    pub mechanism: [f64; 7],
    /// Mean direct probability of room-peer pairs not on one LAN (family and community), by
    /// device pair: [home x home, phone x home, phone x phone]. Home = the household's fixed
    /// endpoint; phone = on Wi-Fi or a carrier.
    pub direct_by_pair: [f64; 3],
    /// Homes (fixed endpoints) with a public IPv4 WAN and no second NAT: the ceiling of any
    /// port-mapping gain.
    pub homes_v4_public: f64,
    /// Homes whose router would map a port (UPnP / PCP on, under that ceiling).
    pub homes_map_capable: f64,
    /// Homes with global IPv6.
    pub homes_v6: f64,
    /// Of those, with no inbound IPv6 firewall.
    pub v6_homes_open: f64,
}

/// Run one model over the regime table's worlds.
#[must_use]
pub fn stats(m: &ReachModel, seed: u64) -> Stats {
    let shape = WorldShape::default();
    let mut relay = Vec::with_capacity(METROS);
    let mut mechanism = [0.0; 7];
    let (mut dp, mut dn) = ([0.0; 3], [0u64; 3]);
    let (mut homes, mut v4p, mut cap, mut v6, mut open) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for i in 0..METROS {
        // the same world and pairs as `relay_distribution` / `world_relay`
        let ws = mix(seed, 0x5745_4C44, i as u64);
        let w = World::generate(m, ws, shape, Correlation::Correlated);
        let pr = w.pairs(ws, 400);
        let cls = classes(&w);
        let mut share = 0.0;
        for (k, pairs) in pr.iter().enumerate() {
            if pairs.is_empty() {
                continue;
            }
            let wk = PAIR_MIX[k] / pairs.len() as f64;
            for &(a, b, c) in pairs {
                let d = super::direct_path(&w.devices[a].ep, &w.devices[b].ep, c, &m.params);
                share += wk * (1.0 - d.p);
                mechanism[d.mechanism as usize] += wk / METROS as f64;
                if k < 2 && !c.same_lan {
                    let phones = usize::from(cls[a] != NodeClass::Fixed) + usize::from(cls[b] != NodeClass::Fixed);
                    dp[phones] += d.p;
                    dn[phones] += 1;
                }
            }
        }
        relay.push(share);
        for (d, c) in w.devices.iter().zip(&cls) {
            if *c != NodeClass::Fixed {
                continue;
            }
            homes += 1;
            v4p += u64::from(!d.ep.cgnat && !d.ep.double_nat);
            cap += u64::from(d.ep.port_map_capable);
            if let Some(f) = d.ep.v6 {
                v6 += 1;
                open += u64::from(f == V6Fw::Open);
            }
        }
    }
    let r = |a: u64, b: u64| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    Stats {
        relay: Dist::of(&relay),
        street: neighbourhood_distribution(m, seed, 32, Correlation::Correlated),
        mechanism,
        direct_by_pair: [0, 1, 2].map(|k| if dn[k] == 0 { 0.0 } else { dp[k] / dn[k] as f64 }),
        homes_v4_public: r(v4p, homes),
        homes_map_capable: r(cap, homes),
        homes_v6: r(v6, homes),
        v6_homes_open: r(open, v6),
    }
}

fn line(s: &mut String, name: &str, st: &Stats, base: f64) {
    use std::fmt::Write as _;
    let (d, t) = (&st.relay, &st.street);
    let _ = writeln!(
        s,
        "  {name:<40} {:.3} ({:.3}) [{:.3},{:.3}]  gain {:+.3}   street p10 {:.3} p50 {:.3} p90 {:.3}",
        d.mean, d.se, d.p10, d.p90, base - d.mean, t.p10, t.p50, t.p90
    );
}

/// The counterfactual report (`cewp-cli nat-cf`).
#[must_use]
pub fn report(seed: u64) -> String {
    use std::fmt::Write as _;
    use ParameterRegime as G;
    let mut s = String::new();
    let _ = writeln!(s, "CEWP twin: UPnP/PCP and IPv6-listener counterfactuals (reach::counterfactual), seed {seed}");
    let _ = writeln!(s, "Same {METROS} correlated metros and 256 streets as the `nat` regime table. Every number MODEL.");
    let _ = writeln!(s, "Relay share = weighted pairs needing a forwarder, mean (se) [p10,p90] across metros; gain = (a) - case.");
    let base_e = ReachParams::default();
    let mut expected = Vec::new();
    let _ = writeln!(s, "\nExpected regime:");
    let mut a_mean = 0.0;
    for c in CASES {
        let m = ReachModel::new(params(&base_e, c)).expect("the defaults calibrate");
        let st = stats(&m, seed);
        if c == CASES[0] {
            a_mean = st.relay.mean;
        }
        line(&mut s, c.name, &st, a_mean);
        expected.push((c, st));
    }
    let _ = writeln!(s, "\n  mechanism mix (weighted pairs):");
    let _ = write!(s, "  {:<40}", "case");
    for mm in Mechanism::ALL {
        let _ = write!(s, " {:>10}", mm.as_str());
    }
    let _ = writeln!(s);
    for (c, st) in &expected {
        let _ = write!(s, "  {:<40}", c.name);
        for x in st.mechanism {
            let _ = write!(s, " {x:>10.3}");
        }
        let _ = writeln!(s);
    }
    let _ = writeln!(s, "\n  direct share, room peers not on one LAN (family + community):");
    let _ = writeln!(s, "  {:<40} {:>12} {:>12} {:>12}", "case", "home x home", "phone x home", "phone x phone");
    for (c, st) in &expected {
        let d = st.direct_by_pair;
        let _ = writeln!(s, "  {:<40} {:>12.3} {:>12.3} {:>12.3}", c.name, d[0], d[1], d[2]);
    }
    let st = &expected[0].1;
    let _ = writeln!(s, "\n  ceilings (fixed endpoints of the same worlds):");
    let _ = writeln!(s, "    public IPv4 WAN, no CGN, no double NAT: {:.3} of homes (the most a mapping client can reach)", st.homes_v4_public);
    let _ = writeln!(s, "    of all homes, router would map (UPnP/PCP on): {:.3}", st.homes_map_capable);
    let _ = writeln!(s, "    homes with global IPv6: {:.3}; of those, no inbound IPv6 firewall: {:.3}", st.homes_v6, st.v6_homes_open);

    let _ = writeln!(s, "\nSweep of router_upnp_share (Expected regime; calibration does not involve it):");
    for u in [0.2, 0.4, 0.6, 0.8] {
        let mut b = base_e.clone();
        b.set(R::RouterUpnpShare, u);
        let a = stats(&ReachModel::new(params(&b, CASES[0])).expect("calibrates"), seed).relay.mean;
        for c in [CASES[1], CASES[2], CASES[4]] {
            let st = stats(&ReachModel::new(params(&b, c)).expect("calibrates"), seed);
            let _ = write!(s, "  upnp {u:.1} ");
            line(&mut s, c.name, &st, a);
            if c == CASES[2] {
                let d = st.direct_by_pair;
                let _ = writeln!(s, "             homes map-capable {:.3}; direct home x home {:.3} phone x home {:.3} phone x phone {:.3}", st.homes_map_capable, d[0], d[1], d[2]);
            }
        }
    }

    let _ = writeln!(s, "\nOther regimes (relay share and per-street spread):");
    for g in [G::Optimistic, G::Pessimistic, G::Adversarial] {
        let base = ReachParams::for_regime(g);
        let _ = writeln!(s, "  {g:?}:");
        let mut a = 0.0;
        for c in [CASES[0], CASES[2], CASES[3], CASES[4]] {
            match ReachModel::new(params(&base, c)) {
                Ok(m) => {
                    let st = stats(&m, seed);
                    if c == CASES[0] {
                        a = st.relay.mean;
                    }
                    line(&mut s, c.name, &st, a);
                }
                Err(e) => {
                    let _ = writeln!(s, "  {:<40} does not calibrate: {e}", c.name);
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reach::{direct_path, Endpoint, Filtering, Mapping, NatBox, PairCtx};

    fn home(mapping: Mapping, capable: bool, client_only: bool) -> Endpoint {
        let m = ReachModel::default_model();
        let nb = m.network(1, crate::reach::AccessNet { id: 1, tech: crate::reach::AccessTech::Cable, region: crate::reach::RegionClass::V4Rich });
        let mut e = m.home(1, &nb, 1);
        e.nat = Some(NatBox { mapping, filtering: Filtering::Apdf, hairpin: false });
        e.cgnat = false;
        e.double_nat = false;
        e.port_forward = false;
        e.port_map_capable = capable;
        e.client_only = client_only;
        e.v6 = None;
        e
    }

    #[test]
    fn the_punch_socket_mapping_only_helps_a_client_behind_a_capable_router() {
        let base = ReachParams::default();
        let phone = home(Mapping::Apdm, true, true);
        let other = home(Mapping::Apdm, false, true);
        let d = |c: Case| direct_path(&phone, &other, PairCtx::ROOM_PEERS, &params(&base, c)).mechanism;
        // two symmetric NATs: no listener (both client-only), so only a mapped punch socket helps
        assert_eq!(d(CASES[0]), Mechanism::Relay);
        assert_eq!(d(CASES[1]), Mechanism::Relay);
        assert_eq!(d(CASES[2]), Mechanism::PeerReflexivePunch);
        // a home node behind the same router listens on its mapped port in (b-l) already
        let node = home(Mapping::Apdm, true, false);
        assert_eq!(direct_path(&node, &other, PairCtx::ROOM_PEERS, &params(&base, CASES[1])).mechanism, Mechanism::PortMapped);
        // not capable (CGN or double NAT): nothing
        let behind = home(Mapping::Apdm, false, true);
        assert_eq!(direct_path(&behind, &other, PairCtx::ROOM_PEERS, &params(&base, CASES[2])).mechanism, Mechanism::Relay);
    }

    #[test]
    fn counterfactuals_never_relay_more_and_switches_off_reproduce_the_table() {
        let m = ReachModel::default_model();
        let small = |c: Case| {
            let mm = ReachModel::new(params(&m.params, c)).expect("calibrates");
            let ws = mix(3, 0x5745_4C44, 0);
            crate::reach::world_relay(&mm, ws, WorldShape::default(), Correlation::Correlated, 400)[3]
        };
        let a = small(CASES[0]);
        assert_eq!(a, crate::reach::world_relay(&m, mix(3, 0x5745_4C44, 0), WorldShape::default(), Correlation::Correlated, 400)[3]);
        for c in &CASES[1..] {
            assert!(small(*c) <= a + 1e-12, "{} relays more", c.name);
        }
        assert!(small(CASES[2]) <= small(CASES[1]) + 1e-12);
        assert!(small(CASES[4]) <= small(CASES[2]) + 1e-12 && small(CASES[4]) <= small(CASES[3]) + 1e-12);
    }
}
