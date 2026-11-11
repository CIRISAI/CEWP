//! The two hooks of `HOOKS.md` for the reachability model: energy counters (counts only) and the
//! flat-buffer globe view (layout agreed with `twin-globe`).
//!
//! Both work on one REALISED world: [`super::direct_path`] gives each pair a mechanism and a
//! probability; [`realise`] draws the outcome once per pair from a seeded stream, so a punch that
//! the model gives 0.90 either comes up or fails, and the counts are integers.
//!
//! Node classes follow the `micro.rs` host kinds: the household's fixed endpoint is
//! `HostKind::HomeNode` (home node or laptop behind the router); every other device is a
//! `HostKind::Phone`, split by the link it is on (home Wi-Fi or a carrier). Hosted servers, seats
//! and CI runners are outside this model (public listeners; see `topo::Resolved::Hosted`).

use super::{mix, Correlation, Endpoint, Mechanism, PairCtx, ReachModel, V6Fw, World, WorldShape};
use crate::micro::Rng;

/// A device's class for the counters and the view (codes are the globe's `access class`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NodeClass {
    /// The household's fixed endpoint (micro `HostKind::HomeNode` / `Laptop`).
    Fixed = 0,
    /// A phone on its home Wi-Fi (micro `HostKind::Phone`, unmetered).
    PhoneWifi = 1,
    /// A phone on a carrier (micro `HostKind::Phone`, metered).
    PhoneCellular = 2,
}

impl NodeClass {
    /// Every class, in code order.
    pub const ALL: [Self; 3] = [Self::Fixed, Self::PhoneWifi, Self::PhoneCellular];
    /// Short name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::PhoneWifi => "phone-wifi",
            Self::PhoneCellular => "phone-cellular",
        }
    }
}

/// Class of every device of `w`, by index.
#[must_use]
pub fn classes(w: &World) -> Vec<NodeClass> {
    let mut out = vec![NodeClass::Fixed; w.devices.len()];
    for &(s, e) in &w.households {
        for (i, c) in out.iter_mut().enumerate().take(e).skip(s + 1) {
            *c = if w.devices[i].on_home_lan { NodeClass::PhoneWifi } else { NodeClass::PhoneCellular };
        }
    }
    out
}

/// A pair's realised outcome (codes are the globe's `mechanism`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Outcome {
    /// Direct over IPv6 (simultaneous send through stateful firewalls).
    DirectV6 = 0,
    /// Direct by dialling a public IPv4 listener or forwarded port (or a mapped port).
    DirectListener = 1,
    /// Direct by an IPv4 hole punch (reflexive or peer-reflexive candidate).
    Punched = 2,
    /// Relayed without a punch attempt (not room peers: `accepts_direct` refuses).
    Relayed = 3,
    /// A punch was attempted and failed; the pair is relayed.
    PunchFailed = 4,
    /// Same household LAN (AutoInterface).
    Lan = 5,
}

impl Outcome {
    /// Every outcome, in code order.
    pub const ALL: [Self; 6] = [Self::DirectV6, Self::DirectListener, Self::Punched, Self::Relayed, Self::PunchFailed, Self::Lan];
    /// The pair needs a forwarder.
    #[must_use]
    pub const fn relayed(self) -> bool {
        matches!(self, Self::Relayed | Self::PunchFailed)
    }
    /// Short name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectV6 => "direct-v6",
            Self::DirectListener => "direct-listener",
            Self::Punched => "punched",
            Self::Relayed => "relayed",
            Self::PunchFailed => "punch-failed",
            Self::Lan => "lan",
        }
    }
}

/// Draw each pair's outcome once. A room-peer pair with no LAN and no listener ATTEMPTS a punch
/// whatever its NATs (edge proposes direct links to room peers without knowing the NAT type,
/// `DirectLinksConfig::proposes`); the attempt fails where [`super::direct_path`] says `Relay`
/// and fails with probability `1 - p` where it says a punch mechanism.
#[must_use]
pub fn realise(w: &World, pairs: &[(usize, usize, PairCtx)], m: &ReachModel, seed: u64) -> Vec<Outcome> {
    let mut r = Rng::new(mix(seed, 0x5245_414C, pairs.len() as u64));
    pairs
        .iter()
        .map(|&(a, b, c)| {
            let d = super::direct_path(&w.devices[a].ep, &w.devices[b].ep, c, &m.params);
            let u = r.f(); // one draw per pair, whatever the branch
            match d.mechanism {
                Mechanism::Lan => Outcome::Lan,
                Mechanism::PublicListener | Mechanism::PortMapped => Outcome::DirectListener,
                Mechanism::Relay if c.may_punch => Outcome::PunchFailed,
                Mechanism::Relay => Outcome::Relayed,
                Mechanism::Ipv6Punch if u < d.p => Outcome::DirectV6,
                Mechanism::HolePunch | Mechanism::PeerReflexivePunch if u < d.p => Outcome::Punched,
                Mechanism::Ipv6Punch | Mechanism::HolePunch | Mechanism::PeerReflexivePunch => Outcome::PunchFailed,
            }
        })
        .collect()
}

/// Per-node path counters (all DERIVED: counts of this model's own pair events; no time, no
/// bytes, no watts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PathCounters {
    /// Pairs this node is an end of.
    pub pairs: u32,
    /// Of those, direct (LAN, listener, IPv6 or punched).
    pub direct: u32,
    /// Of those, carried by a forwarder because no direct path formed. WHICH node forwards is
    /// micro's routing (homing seat or a member's home node), not this model's: the forwarder's
    /// relay work is the sum of these over the pairs it is handed, halved (two ends per pair).
    pub relayed: u32,
    /// Punch attempts this node took part in (one per room-peer pair with no LAN and no
    /// listener; leviculum sends a probe every `PUNCH_INTERVAL` 100 ms for up to `PUNCH_WINDOW`
    /// 10 s per attempt, so at most 100 probes each; edge backs a failed pair off one hour).
    pub punch_attempts: u32,
    /// Of those, failed.
    pub punch_failures: u32,
    /// Direct paths for which this node holds a NAT mapping or firewall pinhole open (both ends
    /// of a punched path; the dialling end of a listener path when it sits behind a NAT). Each
    /// needs keepalives for as long as the path lives; the model has no time, so it counts the
    /// paths, not the keepalives.
    pub held_mappings: u32,
}

impl PathCounters {
    fn add(&mut self, o: &Self) {
        self.pairs += o.pairs;
        self.direct += o.direct;
        self.relayed += o.relayed;
        self.punch_attempts += o.punch_attempts;
        self.punch_failures += o.punch_failures;
        self.held_mappings += o.held_mappings;
    }
}

/// Energy counters of one realised world.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCounters {
    /// Class of each node.
    pub class: Vec<NodeClass>,
    /// Counters of each node.
    pub per_node: Vec<PathCounters>,
    /// Summed per class, in [`NodeClass::ALL`] order.
    pub per_class: [PathCounters; 3],
    /// Pairs per outcome, in [`Outcome::ALL`] order.
    pub per_outcome: [u32; 6],
}

fn holds(ep: &Endpoint) -> bool {
    ep.nat.is_some() || matches!(ep.v6, Some(V6Fw::Stateful))
}

/// Count path events of `w` over `pairs` with outcomes `out`.
#[must_use]
pub fn count(w: &World, pairs: &[(usize, usize, PairCtx)], out: &[Outcome]) -> EnergyCounters {
    let class = classes(w);
    let mut per_node = vec![PathCounters::default(); w.devices.len()];
    let mut per_outcome = [0u32; 6];
    for (&(a, b, _), &o) in pairs.iter().zip(out) {
        per_outcome[o as usize] += 1;
        let attempted = matches!(o, Outcome::DirectV6 | Outcome::Punched | Outcome::PunchFailed);
        let listener_side = |x: usize, y: usize| {
            let (ex, ey) = (&w.devices[x].ep, &w.devices[y].ep);
            // x listens if it binds a public listener and y cannot (or x is the first that can)
            ex.port_forward && !ex.client_only && !(ey.port_forward && !ey.client_only && y < x)
        };
        for (x, y) in [(a, b), (b, a)] {
            let c = &mut per_node[x];
            c.pairs += 1;
            if o.relayed() {
                c.relayed += 1;
            } else {
                c.direct += 1;
            }
            if attempted {
                c.punch_attempts += 1;
                if o == Outcome::PunchFailed {
                    c.punch_failures += 1;
                }
            }
            let hold = match o {
                Outcome::DirectV6 | Outcome::Punched => true,
                Outcome::DirectListener => !listener_side(x, y) && holds(&w.devices[x].ep),
                Outcome::Lan | Outcome::Relayed | Outcome::PunchFailed => false,
            };
            if hold {
                c.held_mappings += 1;
            }
        }
    }
    let mut per_class = [PathCounters::default(); 3];
    for (c, k) in per_node.iter().zip(&class) {
        per_class[*k as usize].add(c);
    }
    EnergyCounters { class, per_node, per_class, per_outcome }
}

/// Energy counters of a default-shape metro: every family pair, `community_pairs` community and
/// as many other pairs (the same pairs [`super::world_relay`] scores).
#[must_use]
pub fn energy_counters(m: &ReachModel, seed: u64, corr: Correlation, community_pairs: usize) -> EnergyCounters {
    let w = World::generate(m, seed, WorldShape::default(), corr);
    let pairs: Vec<_> = w.pairs(seed, community_pairs).into_iter().flatten().collect();
    let out = realise(&w, &pairs, m, seed);
    count(&w, &pairs, &out)
}

// ---------------------------------------------------------------- globe view

/// The view's small scenario: 3 streets of 12 households (about 130 devices), 4 ISPs (at most 2
/// per street), 3 carriers, correlated. Pairs: every family pair, 120 community pairs, 30 others.
#[must_use]
pub fn view_shape() -> WorldShape {
    WorldShape { neighbourhoods: 3, households: 12, metro_isps: 4, isps_per_neighbourhood: 2, carriers: 3 }
}

fn view_world(seed: u64) -> (ReachModel, World, Vec<(usize, usize, PairCtx)>) {
    let m = ReachModel::default_model();
    let w = World::generate(&m, seed, view_shape(), Correlation::Correlated);
    let pairs: Vec<_> = w.pairs(seed, 120).into_iter().enumerate().flat_map(|(k, v)| {
        let keep = if k == 2 { 30 } else { usize::MAX };
        v.into_iter().take(keep)
    }).collect();
    (m, w, pairs)
}

/// NAT behaviour code of an endpoint, as the globe draws it: 0 public (no NAT on its path, or a
/// home listener on a forwarded port); 1 / 2 / 3 endpoint-independent / address-dependent /
/// address-and-port-dependent ("symmetric") mapping, no CGN; 4 / 5 / 6 the same behind a CGN
/// (code - 3 is the mapping). A phone on Wi-Fi shows its router's mapping.
#[must_use]
pub fn behaviour_code(ep: &Endpoint) -> u32 {
    match ep.nat {
        None => 0,
        Some(_) if ep.port_forward && !ep.client_only => 0,
        Some(n) => 1 + n.mapping as u32 + if ep.cgnat { 3 } else { 0 },
    }
}

/// IPv6 code: 0 none, 1 open, 2 stateful firewall, 3 blocked (non-zero = has a global address).
#[must_use]
pub fn v6_code(ep: &Endpoint) -> u32 {
    match ep.v6 {
        None => 0,
        Some(V6Fw::Open) => 1,
        Some(V6Fw::Stateful) => 2,
        Some(V6Fw::Blocked) => 3,
    }
}

/// Nodes of the view scenario, stride 4 (u32), one row per device:
/// `[access class, NAT behaviour, IPv6, household]`.
/// - access class: 0 fixed (home node / laptop), 1 phone on home Wi-Fi, 2 phone on a carrier
///   ([`NodeClass`]); class MODEL.
/// - NAT behaviour: [`behaviour_code`], 0..=6; MODEL (drawn per network, calibrated so the micro
///   classes keep their marginals; maps to micro `RouterNat`: 0 with a listener = `Public`,
///   1 and 4 = `Punchable`, 2, 3, 5, 6 = `Hard`).
/// - IPv6: [`v6_code`], 0..=3; MODEL.
/// - household: global household index (devices of one household are adjacent); DERIVED.
#[must_use]
pub fn nat_view_nodes(seed: u64) -> Vec<u32> {
    let (_, w, _) = view_world(seed);
    let cls = classes(&w);
    let mut v = Vec::with_capacity(w.devices.len() * 4);
    for (d, c) in w.devices.iter().zip(&cls) {
        v.extend_from_slice(&[*c as u32, behaviour_code(&d.ep), v6_code(&d.ep), d.household as u32]);
    }
    v
}

/// Node positions, stride 2 (f32): `[x, y]` in metres in a GENERATED street frame (origin at the
/// first house of street 0; streets 500 m apart along x; houses on a 4-wide grid 40 m apart). A
/// home node sits on its house, a phone on Wi-Fi within 8 m of it, a phone on a carrier out and
/// about within 300 m. MODEL; no real place.
#[must_use]
pub fn nat_view_node_pos(seed: u64) -> Vec<f32> {
    let (_, w, _) = view_world(seed);
    let shape = view_shape();
    let cls = classes(&w);
    let mut r = Rng::new(mix(seed, 0x504F_5349, w.devices.len() as u64));
    let mut v = Vec::with_capacity(w.devices.len() * 2);
    for (d, c) in w.devices.iter().zip(&cls) {
        let k = d.household % shape.households;
        let (hx, hy) = (d.hood as f64 * 500.0 + (k % 4) as f64 * 40.0, (k / 4) as f64 * 40.0);
        let (ang, rad) = (r.range(0.0, std::f64::consts::TAU), r.f());
        let reach = match c {
            NodeClass::Fixed => 0.0,
            NodeClass::PhoneWifi => 8.0,
            NodeClass::PhoneCellular => 300.0,
        };
        v.push((hx + reach * rad * ang.cos()) as f32);
        v.push((hy + reach * rad * ang.sin()) as f32);
    }
    v
}

/// Pairs of the view scenario, stride 3 (u32): `[a, b, outcome]`, `a`/`b` row indices of
/// [`nat_view_nodes`], outcome the realised [`Outcome`] code: 0 direct IPv6, 1 direct listener,
/// 2 hole-punched, 3 relayed (no attempt), 4 punch failed then relayed, 5 same LAN. MODEL (one
/// seeded draw per pair of the probability [`super::direct_path`] gives).
#[must_use]
pub fn nat_view_pairs(seed: u64) -> Vec<u32> {
    let (m, w, pairs) = view_world(seed);
    let out = realise(&w, &pairs, &m, seed);
    let mut v = Vec::with_capacity(pairs.len() * 3);
    for (&(a, b, _), o) in pairs.iter().zip(out) {
        v.extend_from_slice(&[a as u32, b as u32, o as u32]);
    }
    v
}

/// Joint distribution of access class x NAT behaviour over 16 default-shape correlated metros:
/// `[rows, cols, then rows*cols shares row-major]` (f32), rows = [`NodeClass`] codes (3), cols =
/// [`behaviour_code`] codes (7). Shares of all devices (the matrix sums to 1). MODEL.
#[must_use]
pub fn nat_view_joint(seed: u64) -> Vec<f32> {
    let m = ReachModel::default_model();
    let mut acc = [[0u64; 7]; 3];
    let mut n = 0u64;
    for i in 0..16 {
        let w = World::generate(&m, mix(seed, 0x4A4F_494E, i), WorldShape::default(), Correlation::Correlated);
        for (d, c) in w.devices.iter().zip(classes(&w)) {
            acc[c as usize][behaviour_code(&d.ep) as usize] += 1;
            n += 1;
        }
    }
    let mut v = vec![3.0, 7.0];
    for row in &acc {
        v.extend(row.iter().map(|&x| (x as f64 / n.max(1) as f64) as f32));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_add_up_and_relays_match_the_outcomes() {
        let m = ReachModel::default_model();
        let e = energy_counters(&m, 5, Correlation::Correlated, 200);
        let pairs: u32 = e.per_outcome.iter().sum();
        let ends: u32 = e.per_class.iter().map(|c| c.pairs).sum();
        assert_eq!(ends, 2 * pairs);
        let relayed = e.per_outcome[Outcome::Relayed as usize] + e.per_outcome[Outcome::PunchFailed as usize];
        assert_eq!(e.per_class.iter().map(|c| c.relayed).sum::<u32>(), 2 * relayed);
        for c in &e.per_node {
            assert_eq!(c.direct + c.relayed, c.pairs);
            assert!(c.punch_failures <= c.punch_attempts && c.held_mappings <= c.direct);
        }
        // phones on a carrier are client-only: they never listen, so every direct path they have
        // holds a mapping (the carrier always NATs or firewalls)
        let cell = e.per_class[NodeClass::PhoneCellular as usize];
        assert_eq!(cell.held_mappings, cell.direct);
    }

    #[test]
    fn the_realised_relay_share_tracks_the_expected_one() {
        let m = ReachModel::default_model();
        let w = World::generate(&m, 9, WorldShape::default(), Correlation::Correlated);
        let pairs: Vec<_> = w.pairs(9, 400).into_iter().flatten().collect();
        let out = realise(&w, &pairs, &m, 9);
        let got = out.iter().filter(|o| o.relayed()).count() as f64 / pairs.len() as f64;
        let want = w.relay_share(&pairs, &m.params);
        // binomial: only the punch draws are random; sd <= sqrt(0.09 / n)
        let sd = (0.09 / pairs.len() as f64).sqrt();
        assert!((got - want).abs() < 4.0 * sd, "realised {got} vs expected {want}");
    }

    #[test]
    fn views_are_deterministic_and_consistent() {
        let nodes = nat_view_nodes(2);
        assert_eq!(nodes, nat_view_nodes(2));
        assert_eq!(nodes.len() % 4, 0);
        let n = nodes.len() / 4;
        assert_eq!(nat_view_node_pos(2).len(), 2 * n);
        let pairs = nat_view_pairs(2);
        assert_eq!(pairs, nat_view_pairs(2));
        for p in pairs.chunks(3) {
            assert!((p[0] as usize) < n && (p[1] as usize) < n && p[2] <= 5);
        }
        for row in nodes.chunks(4) {
            assert!(row[0] <= 2 && row[1] <= 6 && row[2] <= 3);
        }
        // every outcome the globe colours shows up at least once in the small scenario
        for o in [Outcome::DirectV6, Outcome::DirectListener, Outcome::PunchFailed, Outcome::Lan] {
            assert!(pairs.chunks(3).any(|p| p[2] == o as u32), "{} missing", o.as_str());
        }
        let j = nat_view_joint(2);
        assert_eq!((j[0], j[1], j.len()), (3.0, 7.0, 23));
        let total: f32 = j[2..].iter().sum();
        assert!((total - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_joint_reproduces_the_micro_router_classes() {
        // fixed endpoints: listener (code 0 with a forwarded port) ~ 0.12, EIM (1, 4) ~ 0.28 of
        // homes; the view's codes must keep the calibrated marginals
        let m = ReachModel::default_model();
        let (mut n, mut public, mut eim) = (0u32, 0u32, 0u32);
        for i in 0..16 {
            let w = World::generate(&m, mix(4, 0x4A4F_494E, i), WorldShape::default(), Correlation::Independent);
            for (d, c) in w.devices.iter().zip(classes(&w)) {
                if c == NodeClass::Fixed {
                    n += 1;
                    match behaviour_code(&d.ep) {
                        0 => public += 1,
                        1 | 4 => eim += 1,
                        _ => {}
                    }
                }
            }
        }
        let (fp, fe) = (f64::from(public) / f64::from(n), f64::from(eim) / f64::from(n));
        let sd = (0.25 / f64::from(n)).sqrt();
        assert!((fp - 0.12).abs() < 4.0 * sd, "public {fp}");
        assert!((fe - 0.28).abs() < 4.0 * sd, "eim {fe}");
    }
}
