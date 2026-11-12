//! A finer locality (twin package `theory`, `FSD/twin/locality.md`): three measures that are
//! never merged into one headline.
//!
//! * **(a) Audience locality**: where the recipients of an act sit, by act and by delivery, per
//!   kind of group, in nested member-distance bands (household, building, tile ~1 km,
//!   locality, country, abroad in the same world region, abroad elsewhere). This is what the
//!   page's "% local" measures today, made finer; it says nothing about bytes.
//! * **(b) Path locality**: where the delivered bytes go, from the netgraph routes and the
//!   reach outcomes: the highest network segment a delivery reaches (home LAN, one access
//!   network, the metro, an IXP, long-haul), the relayed share and where the relay sits.
//! * **(c) Presence-weighted locality**: (a) and (b) weighted by who is awake, at home and on
//!   which network at the hour of the act, through one seam ([`Presence`]) shaped to
//!   CIRISHolon's `presence.people` and `attach.phone.network` rows (168 hour-of-week bins).
//!
//! The room placement that sets (a) is explicit ([`SocialParams`] placement rows and
//! [`LOC2_SPECS`]); [`sweep`] shows the headline as a function of them. Every measure returns
//! the verdict of its inputs with its number ([`Verdict`]): a figure computed from the
//! generated, uncertified tie table says so.

#![allow(clippy::needless_range_loop, clippy::field_reassign_with_default)]

use std::collections::BTreeMap;

use crate::activity::{Act, ActivityParams, Generator};
use crate::energy::Class;
use crate::micro::{Rng, Tag};
use crate::netgraph::{self, NetGraph, NetParams};
use crate::reach::hooks::{energy_counters, NodeClass};
use crate::reach::{Correlation, ReachModel};
use crate::social::{Geography, Placement, RoomKind, SocialParams, SocialWorld, NO_ROOM, SOCIAL_PARAM_SPECS};
use crate::workload::{diurnal_multiplier, ParameterRegime, RegimeParameters};

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// One row of [`LOC2_SPECS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loc2Spec {
    /// Name.
    pub name: &'static str,
    /// Default.
    pub default: f64,
    /// Class.
    pub class: Class,
    /// The CIRISHolon registry row that would replace it (`docs/geo/couplings.json`), or `""`.
    pub registry: &'static str,
    /// Reasoning.
    pub why: &'static str,
}

macro_rules! loc2_params {
    ($( $idx:ident = $name:literal, $def:expr, $cls:ident, $reg:literal, $why:literal; )*) => {
        /// Index into [`LOC2_SPECS`].
        #[allow(non_camel_case_types)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum L { $( $idx, )* }
        /// The measure's own inputs.
        pub const LOC2_SPECS: &[Loc2Spec] = &[ $( Loc2Spec { name: $name, default: $def, class: Class::$cls, registry: $reg, why: $why }, )* ];
    };
}

loc2_params! {
    HouseholdsPerBuilding = "band.households_per_building", 4.0, Frontier, "building.dwellings",
        "households sharing a building: 1 for detached houses, tens for apartment blocks; one global mean until the per-cell settlement class is read";
    HouseholdsPerTile = "band.households_per_tile", 400.0, Frontier, "cell.households",
        "households in a ~1 km tile: of the order of 400 at suburban density; GHS-POP per cell replaces it";
    LocalitiesPerCountry = "band.localities_per_country", 3.0, Model, "",
        "the twin has no country layer: localities of one world region are chunked in index order into countries of this many";
    CloseFriendsPerPerson = "close_friends.per_person", 4.0, Frontier, "person.tie",
        "a small inner circle (of the order of the 3 to 5 of the support-clique literature, quoted from memory)";
    CloseFriendsTile = "close_friends.tile_share", 0.149, Model, "person.tie",
        "CIRISHolon TIES-1 (US 2008, closest ties, non-kin): same house 0.036 + same block 0.052 + under 1 mile 0.0605; the one measured sub-district anchor, for close ties only; swept";
    CloseFriendsLocality = "close_friends.locality_share", 0.678, Model, "person.tie", "TIES-1 non-kin 1 to 100 miles (0.167 + 0.4585 + 0.0524): inside the twin's locality (half-width ~300 km)";
    CloseFriendsCountry = "close_friends.country_share", 0.0715, Model, "person.tie", "TIES-1 non-kin 100 to 500 miles";
    CloseFriendsRegion = "close_friends.world_region_share", 0.0741, Model, "person.tie", "TIES-1 non-kin 500 to 3,000 miles; the rest (0.028, 3,000 miles or more) abroad elsewhere";
    CloseFriendsActsPerDay = "close_friends.acts_per_person_day", 15.0, Frontier, "activity.message.direct.weekly", "acts a person sends the close-friends group a day";
    OwnDevicesPerPerson = "own_devices.other_devices_per_person", 1.25, Model, "household.devices",
        "11.25 B devices over 5 B people (FSD) less the phone: the person's other devices (home node, laptop) sit in the household";
    OwnDevicesActsPerDay = "own_devices.acts_per_person_day", 24.0, Frontier, "activity.presence.change.events", "own-state sync acts a person's devices exchange a day";
    OwnAgentsPerPerson = "own_agents.per_person", 1.0, Frontier, "", "AI agents a person runs";
    AgentHostedShare = "own_agents.hosted_share", 0.6, Frontier, "", "share of agents hosted in a data centre; the rest run on the home node";
    AgentHostCountry = "own_agents.host_in_country_share", 0.5, Frontier, "", "of hosted agents, the share hosted in the person's country";
    AgentHostRegion = "own_agents.host_in_world_region_share", 0.3, Frontier, "", "abroad in the same world region; the rest abroad elsewhere";
    OwnAgentsActsPerDay = "own_agents.acts_per_person_day", 10.0, Frontier, "activity.agent.follow.weekly", "acts between a person and their agents a day";
    CdnInIspShare = "baseline.cdn_inside_isp_share", 0.21, Model, "",
        "status quo: share of traffic to customers served from CDN caches inside the ISP's own network (Arcep 2024 via CIRISHolon: 21%, 14 to 48% by ISP); applied to public-room deliveries (cacheable broadcast), not to messaging, which goes to the platform's data centre";
    PublicIxpInboundShare = "baseline.public_ixp_inbound_share", 0.015, Model, "",
        "status quo check: public IXP peering carries 1.5% of inbound traffic (Arcep via CIRISHolon); the netgraph stand-in's paths to a data centre cross an IXP far more often, so the stand-in overstates the IXP segment";
}

/// The measure's input values.
#[derive(Debug, Clone, PartialEq)]
pub struct Loc2Params {
    v: Vec<f64>,
}

impl Default for Loc2Params {
    fn default() -> Self {
        Self { v: LOC2_SPECS.iter().map(|s| s.default).collect() }
    }
}

impl Loc2Params {
    /// Value.
    #[must_use]
    pub fn get(&self, l: L) -> f64 {
        self.v[l as usize]
    }
    /// Set.
    pub fn set(&mut self, l: L, x: f64) {
        self.v[l as usize] = x;
    }
}

// ---------------------------------------------------------------------------
// Kinds and bands
// ---------------------------------------------------------------------------

/// Kinds of group: social.rs's seven room kinds and three added here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupKind {
    /// Co-resident household.
    Household = 0,
    /// Extended family.
    Kin = 1,
    /// Work team.
    Work = 2,
    /// School class.
    School = 3,
    /// Neighbourhood room.
    Neighbourhood = 4,
    /// Interest group.
    Interest = 5,
    /// Public room.
    Public = 6,
    /// A person's close friends (FRONTIER placement).
    CloseFriends = 7,
    /// A person's own other devices (home node, laptop).
    OwnDevices = 8,
    /// A person's AI agents (hosted or on the home node).
    OwnAgents = 9,
}

impl GroupKind {
    /// Every kind.
    pub const ALL: [Self; 10] = [Self::Household, Self::Kin, Self::Work, Self::School, Self::Neighbourhood, Self::Interest, Self::Public, Self::CloseFriends, Self::OwnDevices, Self::OwnAgents];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Household => "household",
            Self::Kin => "kin",
            Self::Work => "work",
            Self::School => "school",
            Self::Neighbourhood => "neighbourhood",
            Self::Interest => "interest",
            Self::Public => "public",
            Self::CloseFriends => "close friends",
            Self::OwnDevices => "own devices",
            Self::OwnAgents => "own agents",
        }
    }
    fn of(k: RoomKind) -> Self {
        Self::ALL[k as usize]
    }
}

/// Member-distance bands, nested.
pub const BANDS: [&str; 7] = ["household", "building", "tile ~1 km", "locality", "country", "abroad, same world region", "abroad, other world region"];
/// Index of the locality band (the page's "local").
pub const LOCALITY_BAND: usize = 3;

/// Network segments, the highest a delivery reaches.
pub const SEGMENTS: [&str; 5] = ["home LAN", "one access network", "metro (cross-network)", "IXP", "long-haul"];

/// Where each person sits in the bands.
#[derive(Debug, Clone)]
struct Places {
    building: Vec<u32>,
    tile: Vec<u32>,
    country: Vec<u16>,
}

impl Places {
    fn of(w: &SocialWorld, lp: &Loc2Params) -> Self {
        // households by cell, in index order: buildings and tiles are chunks of them
        let mut cell_hh: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut first = vec![u32::MAX; w.households];
        for (i, q) in w.persons.iter().enumerate() {
            if first[q.household as usize] == u32::MAX {
                first[q.household as usize] = i as u32;
                cell_hh.entry(q.cell).or_default().push(q.household);
            }
        }
        let b = lp.get(L::HouseholdsPerBuilding).max(1.0) as usize;
        let t = lp.get(L::HouseholdsPerTile).max(1.0) as usize;
        let (mut hb, mut ht) = (vec![0u32; w.households], vec![0u32; w.households]);
        let (mut nb, mut nt) = (0u32, 0u32);
        for hs in cell_hh.values() {
            for (k, &h) in hs.iter().enumerate() {
                if k % b == 0 {
                    nb += 1;
                }
                if k % t == 0 {
                    nt += 1;
                }
                hb[h as usize] = nb;
                ht[h as usize] = nt;
            }
        }
        // countries: localities of one region chunked in index order
        let per = lp.get(L::LocalitiesPerCountry).max(1.0) as usize;
        let mut seen: BTreeMap<u8, usize> = BTreeMap::new();
        let mut country_of_loc = vec![0u16; w.region_of_locality.len()];
        for (l, &reg) in w.region_of_locality.iter().enumerate() {
            let k = seen.entry(reg).or_insert(0);
            country_of_loc[l] = u16::from(reg) * 1000 + (*k / per) as u16;
            *k += 1;
        }
        Self {
            building: w.persons.iter().map(|q| hb[q.household as usize]).collect(),
            tile: w.persons.iter().map(|q| ht[q.household as usize]).collect(),
            country: w.persons.iter().map(|q| country_of_loc[q.locality as usize]).collect(),
        }
    }

    fn band(&self, w: &SocialWorld, a: u32, b: u32) -> usize {
        let (pa, pb) = (&w.persons[a as usize], &w.persons[b as usize]);
        let (ai, bi) = (a as usize, b as usize);
        if pa.household == pb.household {
            0
        } else if self.building[ai] == self.building[bi] {
            1
        } else if self.tile[ai] == self.tile[bi] {
            2
        } else if pa.locality == pb.locality {
            3
        } else if self.country[ai] == self.country[bi] {
            4
        } else if pa.region == pb.region {
            5
        } else {
            6
        }
    }
}

// ---------------------------------------------------------------------------
// Presence seam
// ---------------------------------------------------------------------------

/// Hour-of-week bins (bin 0 = Monday 00:00-01:00 local, as `docs/geo/couplings.json`).
pub const HOW_BINS: usize = 168;

/// Who is awake, home and on which network, by local hour of week. CIRISHolon's
/// `presence.people` (awake, at home) and `attach.phone.network` (on Wi-Fi) rows land here, 168
/// bins each.
pub trait Presence {
    /// Share of persons awake (a delivery to a sleeping person waits in a mailbox).
    fn awake(&self, how: usize) -> f64;
    /// Share of persons at home.
    fn at_home(&self, how: usize) -> f64;
    /// Share of persons whose phone is on Wi-Fi (home or elsewhere).
    fn on_wifi(&self, how: usize) -> f64;
    /// The verdict of the source.
    fn verdict(&self) -> (Class, &'static str);
}

/// The stand-in: awake from the twin's diurnal curve (generator `model.diurnal.v1`), at home
/// and on Wi-Fi from a stated weekday/weekend day shape. All MODEL, no reference.
#[derive(Debug, Clone, Copy, Default)]
pub struct StandInPresence;

impl Presence for StandInPresence {
    fn awake(&self, how: usize) -> f64 {
        // the diurnal multiplier peaks at 1.6 and troughs at 0.5: scale so the trough hour is
        // about 0.3 awake and the peak 1.0
        let h = (how % 24) as f64 + 0.5;
        ((diurnal_multiplier(h * 3600.0) - 0.3) / 1.3).clamp(0.05, 1.0)
    }
    fn at_home(&self, how: usize) -> f64 {
        let (day, h) = (how / 24, how % 24);
        let weekend = day >= 5;
        match h {
            0..=6 | 22..=23 => 0.95,
            9..=16 if !weekend => 0.40,
            7 | 8 | 17 | 18 => if weekend { 0.75 } else { 0.55 },
            _ => if weekend { 0.65 } else { 0.75 },
        }
    }
    fn on_wifi(&self, how: usize) -> f64 {
        // at home almost always on Wi-Fi; away a fifth of the time (work, cafe)
        let a = self.at_home(how);
        a * 0.95 + (1.0 - a) * 0.2
    }
    fn verdict(&self) -> (Class, &'static str) {
        (Class::Model, "stand-in presence (diurnal model.diurnal.v1, stated day shape): unresolved, no reference; CIRISHolon presence.people and attach.phone.network replace it")
    }
}

/// CIRISHolon PRESENCE-0 (US time diaries 2022-2024, 15+) as `presence.people`: awake and at
/// home by day type and clock hour. Networks are not in the table: Wi-Fi at a workplace or
/// school and elsewhere are FRONTIER inputs.
#[derive(Debug, Clone, Copy)]
pub struct Presence0Presence {
    /// Share of time at a workplace or school on Wi-Fi (FRONTIER).
    pub work_wifi: f64,
    /// Share of time elsewhere on Wi-Fi (FRONTIER).
    pub away_wifi: f64,
}

impl Default for Presence0Presence {
    fn default() -> Self {
        Self { work_wifi: crate::energy::WORK_WIFI, away_wifi: crate::energy::AWAY_WIFI }
    }
}

impl Presence for Presence0Presence {
    fn awake(&self, how: usize) -> f64 {
        crate::presence0::awake(how)
    }
    fn at_home(&self, how: usize) -> f64 {
        crate::presence0::at_home(how)
    }
    fn on_wifi(&self, how: usize) -> f64 {
        crate::presence0::on_wifi(how, self.work_wifi, self.away_wifi)
    }
    fn verdict(&self) -> (Class, &'static str) {
        (Class::Model, "PRESENCE-0 as presence.people (US time diaries 2022-2024, ages 15+; awake 0.623 of the week, the twin's 0.85 refused); outside the US it is a stand-in of another country's week; Wi-Fi at work 0.8 and elsewhere 0.2 are FRONTIER; nothing for under-15s or for whether anyone is home in a multi-person household")
    }
}

// ---------------------------------------------------------------------------
// Verdict
// ---------------------------------------------------------------------------

/// The verdict of a measure's inputs, carried with its number.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    /// Weakest class among the inputs read.
    pub class: Class,
    /// The CIRISHolon registry rows the measure will read when they exist.
    pub reads: &'static [&'static str],
    /// Plain statement.
    pub note: String,
}

const AUDIENCE_READS: &[&str] = &["household.kin", "person.work_team", "person.school_class", "household.neighbourhood_room", "person.interest_group", "person.public_follow", "person.tie", "act.recipient", "act.scope", "cell.households", "building.dwellings", "locality.region"];
const PATH_READS: &[&str] = &["household.access_network", "phone.carrier", "as.population", "as.upstream", "as.peering", "ixp.membership", "metro.nearest_ixp", "cable.links", "household.reach_marginals", "household.lan"];
const PRESENCE_READS: &[&str] = &["presence.people", "attach.phone.network", "joint.presence_attach_activity", "locality.time_zone", "region.week_class", "person.commute"];

fn social_class(names: &[&str]) -> Class {
    let mut c = Class::Model;
    for s in SOCIAL_PARAM_SPECS {
        if names.contains(&s.name) && s.tag == Tag::Frontier {
            c = Class::Frontier;
        }
    }
    c
}

const PLACEMENT_ROWS: &[&str] = &["diaspora_share", "diaspora_abroad_share", "work_cross_locality_share", "work_cross_region_share", "community_join_share", "interest_local_share", "interest_regional_share", "public_local_share", "public_regional_share", "interest_groups_per_person", "public_follows_per_person"];

fn audience_verdict() -> Verdict {
    let frontier: Vec<&str> = SOCIAL_PARAM_SPECS.iter().filter(|s| PLACEMENT_ROWS.contains(&s.name) && s.tag == Tag::Frontier).map(|s| s.name).collect();
    Verdict {
        class: social_class(PLACEMENT_ROWS).weaker(Class::Frontier),
        reads: AUDIENCE_READS,
        note: format!("computed from a generated, uncertified tie table: room placement rows {} are FRONTIER, and close friends, own agents and the building and tile bands are FRONTIER here; no CIRISHolon tie row is certified yet", frontier.join(", ")),
    }
}

// ---------------------------------------------------------------------------
// The measures
// ---------------------------------------------------------------------------

/// One group kind's three localities.
#[derive(Debug, Clone, PartialEq)]
pub struct KindLocality {
    /// The kind.
    pub kind: GroupKind,
    /// Acts with an audience.
    pub acts: f64,
    /// Deliveries.
    pub deliveries: f64,
    /// (a) Share of acts whose every recipient lies within band `b` (cumulative).
    pub act_within: [f64; 7],
    /// (a) Share of deliveries per band (sums to 1).
    pub delivery_band: [f64; 7],
    /// (b) Share of deliveries per highest segment (sums to 1).
    pub path_segment: [f64; 5],
    /// (b) Share of deliveries relayed; of those, share relayed by a neighbour's home node.
    pub relayed: f64,
    /// Of relayed deliveries, the share on a neighbour's home node (the rest on a hosted relay).
    pub relayed_by_neighbour: f64,
    /// (b) The status quo's segment shares for the same deliveries (platform data centre; CDN
    /// inside the ISP for public rooms). Path locality is stated as `path_segment - this`.
    pub path_baseline: [f64; 5],
    /// (c) Presence-weighted share of deliveries per band (weights: recipient awake).
    pub presence_band: [f64; 7],
    /// (c) Presence-weighted share per segment (at home and on Wi-Fi by hour).
    pub presence_segment: [f64; 5],
}

/// All three measures for one world.
#[derive(Debug, Clone, PartialEq)]
pub struct Localities {
    /// Persons.
    pub persons: usize,
    /// Per kind.
    pub kinds: Vec<KindLocality>,
    /// All kinds together, weighted by acts.
    pub all: KindLocality,
    /// Verdict of (a).
    pub audience: Verdict,
    /// Verdict of (b).
    pub path: Verdict,
    /// Verdict of (c).
    pub presence: Verdict,
}

/// Segment distributions the path measure reads, built once per world from netgraph and reach.
#[derive(Debug, Clone, PartialEq)]
pub struct PathModel {
    /// Same-locality, different networks: share per segment (metro, IXP, long-haul via a
    /// trombone), from routed pairs in each metro.
    pub same_metro: [f64; 5],
    /// Share of same-locality fixed pairs on one access network (sum of market shares squared).
    pub same_network: f64,
    /// Cross-locality, same country / region / other: share per segment.
    pub cross: [[f64; 5]; 3],
    /// Relayed share of pairs: fixed, phone on Wi-Fi, phone on a carrier (reach hooks).
    pub relayed: [f64; 3],
    /// Segment of a path to the nearest data centre (hosted relay, hosted agent).
    pub to_dc: [f64; 5],
    /// Relay offering share (regime).
    pub relay_offer: f64,
}

fn seg_of(c: &netgraph::PathCounters, same_metro: bool) -> usize {
    let _ = same_metro;
    if c.long_haul_km > 0 {
        4
    } else if c.ixp_crossings > 0 {
        3
    } else if c.as_hops == 0 {
        1
    } else {
        2
    }
}

fn norm<const N: usize>(mut v: [f64; N]) -> [f64; N] {
    let s: f64 = v.iter().sum();
    if s > 0.0 {
        for x in &mut v {
            *x /= s;
        }
    }
    v
}

fn pick(r: &mut Rng, m: &[(netgraph::Asn, u32)]) -> Option<netgraph::Asn> {
    let tot: u64 = m.iter().map(|x| u64::from(x.1)).sum();
    if tot == 0 {
        return None;
    }
    let mut u = r.next_u64() % tot;
    for &(a, s) in m {
        if u < u64::from(s) {
            return Some(a);
        }
        u -= u64::from(s);
    }
    m.last().map(|x| x.0)
}

impl PathModel {
    /// Build from the netgraph stand-in and the reach model at `seed`, regime `r`.
    #[must_use]
    pub fn build(seed: u64, r: ParameterRegime) -> Self {
        let g: NetGraph = netgraph::generate(seed, &NetParams::default());
        let mut rng = Rng::new(seed ^ 0x10CA_17E2);
        let (mut same, mut same_net, mut n_m) = ([0.0; 5], 0.0, 0.0);
        for m in 0..g.metros.len() {
            let m = m as netgraph::MetroId;
            let mk = g.fixed_market(m);
            let tot: f64 = mk.iter().map(|x| f64::from(x.1)).sum();
            if tot <= 0.0 {
                continue;
            }
            same_net += mk.iter().map(|x| (f64::from(x.1) / tot).powi(2)).sum::<f64>();
            n_m += 1.0;
            for _ in 0..6 {
                let (Some(a), Some(b)) = (pick(&mut rng, mk), pick(&mut rng, mk)) else { continue };
                if a == b {
                    continue;
                }
                let (Some(la), Some(lb)) = (g.label(a, m), g.label(b, m)) else { continue };
                if let Some(p) = g.path(la, lb) {
                    same[seg_of(&p.counters, true)] += 1.0;
                }
            }
        }
        let mut cross = [[0.0; 5]; 3];
        for p in netgraph::sample_fixed_pairs(&g, seed, 600) {
            if p.same_metro {
                continue;
            }
            let (ma, mb) = (p.path.metro_trail[0], *p.path.metro_trail.last().unwrap_or(&0));
            let (ra, rb) = (g.metros[ma as usize].region, g.metros[mb as usize].region);
            // the netgraph has no country layer either: a pair under 1,500 km in one region
            // stands for the same country
            let k = if ra != rb { 2 } else if p.km < 1500 { 0 } else { 1 };
            cross[k][seg_of(&p.path.counters, false)] += 1.0;
        }
        let mut to_dc = [0.0; 5];
        for m in 0..g.metros.len().min(181) {
            let m = m as netgraph::MetroId;
            let Some(a) = pick(&mut rng, g.fixed_market(m)) else { continue };
            let mut best: Option<(u32, netgraph::Asn, netgraph::MetroId)> = None;
            for mc in 0..g.metros.len() {
                let mc = mc as netgraph::MetroId;
                if let Some(&(c, _)) = g.content_market(mc).first() {
                    let km = g.backbone_km(m, mc);
                    if best.is_none_or(|b| km < b.0) {
                        best = Some((km, c, mc));
                    }
                }
            }
            let Some((_, c, mc)) = best else { continue };
            let (Some(la), Some(lc)) = (g.label(a, m), g.label(c, mc)) else { continue };
            if let Some(p) = g.path(la, lc) {
                to_dc[seg_of(&p.counters, m == mc)] += 1.0;
            }
        }
        let rc = energy_counters(&ReachModel::default_model(), seed, Correlation::Correlated, 120);
        let share = |k: NodeClass| {
            let c = rc.per_class[k as usize];
            if c.pairs == 0 { 0.0 } else { f64::from(c.relayed) / f64::from(c.pairs) }
        };
        Self {
            same_metro: norm(same),
            same_network: if n_m > 0.0 { same_net / n_m } else { 0.0 },
            cross: [norm(cross[0]), norm(cross[1]), norm(cross[2])],
            relayed: [share(NodeClass::Fixed), share(NodeClass::PhoneWifi), share(NodeClass::PhoneCellular)],
            to_dc: norm(to_dc),
            relay_offer: RegimeParameters::for_regime(r).relay_offering_share,
        }
    }

    /// Segment distribution of one delivery at band `b`, given the chance `home_wifi` that each
    /// end is at home on Wi-Fi and `wifi` that its phone is on Wi-Fi at all; returns the
    /// distribution and the relayed share.
    fn segments(&self, b: usize, kind: GroupKind, home_wifi: f64, wifi: f64, agent_hosted: f64) -> ([f64; 5], f64) {
        let mut s = [0.0; 5];
        if kind == GroupKind::OwnAgents {
            // a hosted agent sits behind the data-centre path; a home agent is on the LAN when
            // the person is at home on Wi-Fi
            for (i, x) in self.to_dc.iter().enumerate() {
                s[i] += agent_hosted * x;
            }
            s[0] += (1.0 - agent_hosted) * home_wifi;
            s[2] += (1.0 - agent_hosted) * (1.0 - home_wifi);
            return (s, 0.0);
        }
        let cell = 1.0 - wifi;
        let relayed = (self.relayed[1] * wifi + self.relayed[2] * cell).min(1.0);
        match b {
            0 => {
                // both at home on the LAN, else a phone away reaches home through its carrier
                let lan = if kind == GroupKind::OwnDevices { home_wifi } else { home_wifi * home_wifi };
                s[0] = lan;
                for (i, x) in self.same_metro.iter().enumerate() {
                    s[i] += (1.0 - lan) * x;
                }
            }
            1..=3 => {
                // same locality: one access network with the market's chance when both ends are
                // on fixed lines, else a cross-network metro path
                let fixed_both = wifi * wifi;
                let one = fixed_both * self.same_network;
                s[1] = one;
                for (i, x) in self.same_metro.iter().enumerate() {
                    s[i] += (1.0 - one) * x;
                }
            }
            _ => {
                let k = (b - 4).min(2);
                s = self.cross[k];
            }
        }
        // a relayed delivery goes through a neighbour (metro at least) or a hosted relay; a
        // delivery on the home LAN needs no relay
        let mut out = [0.0; 5];
        out[0] = s[0];
        for (i, x) in s.iter().enumerate().skip(1) {
            out[i] += (1.0 - relayed) * x;
            let via_n = i.max(2);
            out[via_n] += relayed * self.relay_offer * x;
            for (j, y) in self.to_dc.iter().enumerate() {
                out[i.max(j)] += relayed * (1.0 - self.relay_offer) * x * y;
            }
        }
        (out, relayed * (1.0 - s[0]))
    }

    /// The status quo's segment distribution for one delivery of `kind`: up to the platform's
    /// data centre and down again (the path to the nearest data centre), with public-room
    /// deliveries served from a CDN inside the ISP `cdn` of the time.
    #[must_use]
    pub fn baseline(&self, kind: GroupKind, cdn: f64) -> [f64; 5] {
        let mut b = self.to_dc;
        if kind == GroupKind::Public {
            b = b.map(|x| x * (1.0 - cdn));
            b[1] += cdn;
        }
        b
    }
}

#[derive(Default, Clone)]
struct Acc {
    acts: f64,
    deliveries: f64,
    act_within: [f64; 7],
    band: [f64; 7],
    seg: [f64; 5],
    relayed: f64,
    pres_w: f64,
    pres_band: [f64; 7],
    pres_seg: [f64; 5],
}

impl Acc {
    /// One act with `bands[b]` recipients per band, at local hour of week `how`.
    fn act(&mut self, kind: GroupKind, bands: &[f64; 7], how: usize, pm: &PathModel, pr: &dyn Presence, hosted: f64) {
        let n: f64 = bands.iter().sum();
        if n <= 0.0 {
            return;
        }
        self.acts += 1.0;
        self.deliveries += n;
        let widest = bands.iter().rposition(|&x| x > 0.0).unwrap_or(0);
        for b in widest..7 {
            self.act_within[b] += 1.0;
        }
        // average presence and network over the day for (b); the hour's for (c)
        let (dh, dw) = day_means(pr);
        // at home on Wi-Fi: no more than either share
        let (hh, hw) = (pr.at_home(how).min(pr.on_wifi(how)), pr.on_wifi(how));
        let aw = pr.awake(how);
        for (b, &c) in bands.iter().enumerate() {
            if c <= 0.0 {
                continue;
            }
            self.band[b] += c;
            let (s, rel) = pm.segments(b, kind, dh, dw, hosted);
            self.relayed += c * rel;
            for i in 0..5 {
                self.seg[i] += c * s[i];
            }
            self.pres_w += c * aw;
            self.pres_band[b] += c * aw;
            let (sp, _) = pm.segments(b, kind, hh, hw, hosted);
            for i in 0..5 {
                self.pres_seg[i] += c * aw * sp[i];
            }
        }
    }

    fn add(&mut self, o: &Self) {
        self.acts += o.acts;
        self.deliveries += o.deliveries;
        self.relayed += o.relayed;
        self.pres_w += o.pres_w;
        for i in 0..7 {
            self.act_within[i] += o.act_within[i];
            self.band[i] += o.band[i];
            self.pres_band[i] += o.pres_band[i];
        }
        for i in 0..5 {
            self.seg[i] += o.seg[i];
            self.pres_seg[i] += o.pres_seg[i];
        }
    }

    fn finish(&self, kind: GroupKind, relay_offer: f64) -> KindLocality {
        let d = self.deliveries.max(1e-12);
        let a = self.acts.max(1e-12);
        let pw = self.pres_w.max(1e-12);
        KindLocality {
            kind,
            acts: self.acts,
            deliveries: self.deliveries,
            act_within: self.act_within.map(|x| x / a),
            delivery_band: self.band.map(|x| x / d),
            path_segment: self.seg.map(|x| x / d),
            relayed: self.relayed / d,
            relayed_by_neighbour: if self.relayed > 0.0 { relay_offer } else { 0.0 },
            path_baseline: [0.0; 5],
            presence_band: self.pres_band.map(|x| x / pw),
            presence_segment: self.pres_seg.map(|x| x / pw),
        }
    }
}

fn day_means(pr: &dyn Presence) -> (f64, f64) {
    let (mut h, mut w) = (0.0, 0.0);
    for b in 0..HOW_BINS {
        h += pr.at_home(b).min(pr.on_wifi(b));
        w += pr.on_wifi(b);
    }
    (h / HOW_BINS as f64, w / HOW_BINS as f64)
}

fn how_of(t_utc: f64, lon: f32, start_weekday: u32) -> usize {
    let local = t_utc + f64::from(lon) / 360.0 * 86_400.0;
    let h = (local / 3600.0).floor() as i64 + i64::from(start_weekday) * 24;
    h.rem_euclid(HOW_BINS as i64) as usize
}

/// Measure all three localities on `w` driven by `ap` for `horizon_s`, seeded.
#[must_use]
pub fn measure(w: &SocialWorld, ap: &ActivityParams, lp: &Loc2Params, pm: &PathModel, pr: &dyn Presence, horizon_s: f64, seed: u64) -> Localities {
    let places = Places::of(w, lp);
    let mut acc: Vec<Acc> = vec![Acc::default(); GroupKind::ALL.len()];
    let hosted = lp.get(L::AgentHostedShare);
    // social.rs rooms, through the activity generator
    let g = Generator::new(w, ap, seed);
    let mut sink = |a: &Act| {
        if a.room == NO_ROOM {
            return;
        }
        let kind = GroupKind::of(w.room_kind[a.room as usize]);
        let mut bands = [0.0; 7];
        for &m in w.members(a.room) {
            if m != a.signer {
                bands[places.band(w, a.signer, m)] += 1.0;
            }
        }
        let s = &w.persons[a.signer as usize];
        acc[kind as usize].act(kind, &bands, how_of(a.t, s.lon, ap.start_weekday), pm, pr, hosted);
    };
    g.run(horizon_s, seed, &[], &mut sink);
    // the three added kinds: expected acts per local hour (the diurnal shape), so only the
    // friend draw is random
    let mut r = Rng::new(seed ^ 0x0F12_E7D5);
    let mut by_tile: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut by_loc: BTreeMap<u16, Vec<u32>> = BTreeMap::new();
    let mut by_country: BTreeMap<u16, Vec<u32>> = BTreeMap::new();
    let mut by_region: BTreeMap<u8, Vec<u32>> = BTreeMap::new();
    for (i, q) in w.persons.iter().enumerate() {
        by_tile.entry(places.tile[i]).or_default().push(i as u32);
        by_loc.entry(q.locality).or_default().push(i as u32);
        by_country.entry(places.country[i]).or_default().push(i as u32);
        by_region.entry(q.region).or_default().push(i as u32);
    }
    let n = w.persons.len() as u32;
    let days = horizon_s / 86_400.0;
    let hour_w: Vec<f64> = (0..24).map(|h| diurnal_multiplier((f64::from(h) + 0.5) * 3600.0)).collect();
    let hw_sum: f64 = hour_w.iter().sum();
    let cf = [lp.get(L::CloseFriendsTile), lp.get(L::CloseFriendsLocality), lp.get(L::CloseFriendsCountry), lp.get(L::CloseFriendsRegion)];
    for (i, q) in w.persons.iter().enumerate() {
        let i32_ = i as u32;
        // close friends
        let k = lp.get(L::CloseFriendsPerPerson).round().max(0.0) as usize;
        let mut fb = [0.0; 7];
        for _ in 0..k {
            let u = r.f();
            let pool: Option<&Vec<u32>> = if u < cf[0] {
                by_tile.get(&places.tile[i])
            } else if u < cf[0] + cf[1] {
                by_loc.get(&q.locality)
            } else if u < cf[0] + cf[1] + cf[2] {
                by_country.get(&places.country[i])
            } else if u < cf[0] + cf[1] + cf[2] + cf[3] {
                by_region.get(&q.region)
            } else {
                None
            };
            let mut f = match pool {
                Some(v) if v.len() > 1 => v[r.below(v.len())],
                _ => r.below(n as usize) as u32,
            };
            if f == i32_ {
                f = (f + 1) % n;
            }
            fb[places.band(w, i32_, f)] += 1.0;
        }
        // own devices: other devices in the household
        let od = [lp.get(L::OwnDevicesPerPerson), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        // own agents: hosted ones by the host's distance, home ones in the household
        let ag = lp.get(L::OwnAgentsPerPerson);
        let (hc, hr) = (lp.get(L::AgentHostCountry), lp.get(L::AgentHostRegion));
        let oa = [ag * (1.0 - hosted), 0.0, 0.0, 0.0, ag * hosted * hc, ag * hosted * hr, ag * hosted * (1.0 - hc - hr).max(0.0)];
        for (kind, bands, rate) in [(GroupKind::CloseFriends, fb, lp.get(L::CloseFriendsActsPerDay)), (GroupKind::OwnDevices, od, lp.get(L::OwnDevicesActsPerDay)), (GroupKind::OwnAgents, oa, lp.get(L::OwnAgentsActsPerDay))] {
            if bands.iter().sum::<f64>() <= 0.0 {
                continue;
            }
            // one representative act per local hour, weighted by the expected count
            for h in 0..24usize {
                let expected = rate * days * hour_w[h] / hw_sum;
                let mut a1 = Acc::default();
                let how = (h + 24 * ap.start_weekday as usize) % HOW_BINS;
                a1.act(kind, &bands, how, pm, pr, hosted);
                scale(&mut a1, expected);
                acc[kind as usize].add(&a1);
            }
        }
    }
    let relay_offer = pm.relay_offer;
    let cdn = lp.get(L::CdnInIspShare);
    let kinds: Vec<KindLocality> = GroupKind::ALL.iter().map(|&k| {
        let mut x = acc[k as usize].finish(k, relay_offer);
        x.path_baseline = pm.baseline(k, cdn);
        x
    }).collect();
    let mut all = Acc::default();
    let mut base_all = [0.0; 5];
    let mut d_all = 0.0;
    for (a, k) in acc.iter().zip(&kinds) {
        all.add(a);
        for i in 0..5 {
            base_all[i] += a.deliveries * k.path_baseline[i];
        }
        d_all += a.deliveries;
    }
    let pv = pr.verdict();
    Localities {
        persons: w.persons.len(),
        kinds,
        all: {
            let mut x = all.finish(GroupKind::Household, relay_offer);
            x.path_baseline = base_all.map(|v| v / d_all.max(1e-12));
            x
        },
        audience: audience_verdict(),
        path: Verdict { class: Class::Frontier, reads: PATH_READS, note: "netgraph stand-in AS graph (MODEL) and reach outcomes (MODEL marginals, FRONTIER CGNAT and carrier punching); no country layer, so 'same country' is a same-region pair under 1,500 km; equal bytes per delivery".to_string() },
        presence: Verdict { class: pv.0.weaker(Class::Frontier), reads: PRESENCE_READS, note: format!("{}; on top of (a) and (b)", pv.1) },
    }
}

fn scale(a: &mut Acc, k: f64) {
    a.acts *= k;
    a.deliveries *= k;
    a.relayed *= k;
    a.pres_w *= k;
    for i in 0..7 {
        a.act_within[i] *= k;
        a.band[i] *= k;
        a.pres_band[i] *= k;
    }
    for i in 0..5 {
        a.seg[i] *= k;
        a.pres_seg[i] *= k;
    }
}

// ---------------------------------------------------------------------------
// The reference at district grain: CIRISHolon TIES-0 (Germany)
// ---------------------------------------------------------------------------

/// Where TIES-0 was read.
pub const TIES0_PIN: &str = "CIRISHolon 8477c00a7d1f4d92f086a44ade4d1c8889109932 conformance/scene/tables/ties0_de/ties_table.json v1 sha256 3ca880e83577204bfd9ede906381b4307f1bb980bed7b16dac0289f67d77a06f";
/// TIES-0 bands (friendship links of one platform between German districts).
pub const TIES0_BANDS: [&str; 5] = ["own district", "within 100 km", "100 to 300 km", "farther in Germany", "foreign Europe"];
/// Population-weighted share of a district's links per band, all districts (users in
/// proportion to residents; the other convention differs by at most 0.02).
pub const TIES0_ALL: [f64; 5] = [0.3456, 0.2738, 0.1292, 0.0999, 0.1515];
/// The same by density fifth, sparsest first.
pub const TIES0_FIFTHS: [[f64; 5]; 5] = [
    [0.4347, 0.2580, 0.1294, 0.0874, 0.0905],
    [0.4069, 0.2746, 0.1191, 0.0797, 0.1196],
    [0.3761, 0.2700, 0.1234, 0.0814, 0.1491],
    [0.2842, 0.3229, 0.1322, 0.0979, 0.1628],
    [0.2300, 0.2433, 0.1416, 0.1518, 0.2333],
];
/// TIES-0 `below_the_area`: the share of own-district links within 1 km, median district,
/// is anywhere in this bracket (no licence below a district; a bracket, not a measurement).
pub const TIES0_WITHIN_1KM_BRACKET: (f64, f64) = (0.0038, 0.4824);

/// How ties become messages: a member pair of a room of `n` weighs `n^-k` (k = 0 flat: every
/// tie talks as much; k = 1 strong ties talk more: a pair in a small room outweighs one in a
/// large room). MODEL: TIES-0 counts links, not messages.
#[must_use]
pub fn tie_weight(n: usize, k: f64) -> f64 {
    (n.max(2) as f64).powf(-k)
}

fn gc_km(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f64 {
    let r = std::f64::consts::PI / 180.0;
    let (p1, p2) = (f64::from(lat1) * r, f64::from(lat2) * r);
    let dl = (f64::from(lon2) - f64::from(lon1)) * r;
    let c = (p1.sin() * p2.sin() + p1.cos() * p2.cos() * dl.cos()).clamp(-1.0, 1.0);
    6371.0 * c.acos()
}

/// The twin's acquaintance and kin ties (member pairs of kin, work, school, neighbourhood and
/// interest rooms; public follows make no tie), in TIES-0's bands at the twin's grain: "own
/// district" = the same grid cell (the twin's finest place); "farther in Germany" = over 300 km
/// in the same world region; "foreign" = another world region. Weighted by [`tie_weight`].
/// Rooms over 40 members contribute 40 sampled pairs scaled to their pair count.
#[must_use]
pub fn tie_bands(w: &SocialWorld, geo: &Geography, k: f64, seed: u64) -> [f64; 5] {
    let mut r = Rng::new(seed ^ 0x7135);
    let mut b = [0.0; 5];
    for room in 0..w.rooms() as u32 {
        let kind = w.room_kind[room as usize];
        if matches!(kind, RoomKind::Household | RoomKind::Public) {
            continue;
        }
        let m = w.members(room);
        let n = m.len();
        if n < 2 {
            continue;
        }
        let pairs = n * (n - 1) / 2;
        let wt = tie_weight(n, k);
        let mut one = |a: u32, c: u32, scale: f64| {
            let (pa, pc) = (&w.persons[a as usize], &w.persons[c as usize]);
            let band = if pa.cell == pc.cell {
                0
            } else {
                let (ca, cc) = (&geo.cells[pa.cell as usize], &geo.cells[pc.cell as usize]);
                let d = gc_km(ca.lat, ca.lon, cc.lat, cc.lon);
                if pa.region != pc.region { 4 } else if d < 100.0 { 1 } else if d < 300.0 { 2 } else { 3 }
            };
            b[band] += wt * scale;
        };
        if n <= 40 {
            for i in 0..n {
                for j in i + 1..n {
                    one(m[i], m[j], 1.0);
                }
            }
        } else {
            for _ in 0..40 {
                let i = r.below(n);
                let mut j = r.below(n);
                if j == i {
                    j = (j + 1) % n;
                }
                one(m[i], m[j], pairs as f64 / 40.0);
            }
        }
    }
    norm(b)
}

// ---------------------------------------------------------------------------
// Closest ties by kind: CIRISHolon TIES-1 (United States, 2008)
// ---------------------------------------------------------------------------

/// Where TIES-1 was read.
pub const TIES1_PIN: &str = "CIRISHolon 1ba48eac69aae8126d1200f1b0b4371fa341db3e conformance/scene/tables/ties1_us/ties_by_kind_table.json v1 sha256 64aeb4ceefffa317a7833cca3bb21caab8d8b91974ca3bc83f6f86479267b376 (2,512 respondents, 7,059 closest ties)";
/// TIES-1 distance bands.
pub const TIES1_BANDS: [&str; 9] = ["same house", "same block", "under 1 mile", "1-5 miles", "5-50 miles", "50-100 miles", "100-500 miles", "500-3,000 miles", "3,000+ miles"];
/// All closest ties by band.
pub const TIES1_ALL: [f64; 9] = [0.2367, 0.0330, 0.0396, 0.1233, 0.3166, 0.0483, 0.0890, 0.0867, 0.0266];
/// By relation (partner, child, parent, sibling, other family, non-kin): (share of ties, bands).
pub const TIES1_RELATIONS: [(&str, f64, [f64; 9]); 6] = [
    ("partner", 0.1517, [0.8215, 0.0039, 0.0103, 0.0240, 0.0792, 0.0069, 0.0166, 0.0207, 0.0169]),
    ("child", 0.1864, [0.2878, 0.0217, 0.0359, 0.1081, 0.2709, 0.0463, 0.1154, 0.0906, 0.0235]),
    ("parent", 0.1081, [0.1932, 0.0504, 0.0419, 0.1060, 0.2614, 0.0512, 0.1173, 0.1380, 0.0407]),
    ("sibling", 0.1088, [0.0716, 0.0352, 0.0297, 0.1413, 0.3086, 0.0836, 0.1403, 0.1625, 0.0272]),
    ("other family", 0.1240, [0.1480, 0.0196, 0.0339, 0.1533, 0.3638, 0.0579, 0.1136, 0.0831, 0.0268]),
    ("non-kin", 0.3211, [0.0356, 0.0520, 0.0605, 0.1673, 0.4585, 0.0524, 0.0715, 0.0741, 0.0281]),
];
/// Overlapping flags among closest ties (floors for the twin's kinds): co-worker, neighbour,
/// group member.
pub const TIES1_FLAGS: [(&str, [f64; 9]); 3] = [
    ("co-worker", [0.0903, 0.0078, 0.0370, 0.1524, 0.5614, 0.0541, 0.0610, 0.0224, 0.0137]),
    ("neighbour", [0.0278, 0.5506, 0.2090, 0.0669, 0.0859, 0.0061, 0.0124, 0.0379, 0.0034]),
    ("group member", [0.2339, 0.0311, 0.0555, 0.1792, 0.3696, 0.0188, 0.0461, 0.0551, 0.0107]),
];
/// The household's share of electronic contact with closest ties: floor as named, central
/// (correcting for unnamed co-residents), ceiling (top answer 30 a day). A bracket, not a point.
pub const TIES1_HOUSEHOLD_ELECTRONIC: (f64, f64, f64) = (0.29, 0.52, 0.61);
/// Of closest ties outside the house and within 50 miles, the share on the block or under a
/// mile (se 0.0074); an evenly spread distance law gives 0.02.
pub const TIES1_SUBDISTRICT_ANCHOR: f64 = 0.142;

/// TIES-1's reference for a twin kind: `(label, bands)`; `None` where the survey has none.
/// Household = same house; Kin = family elsewhere (child, parent, sibling, other family
/// outside the house, tie-weighted); CloseFriends = non-kin; Work, Neighbourhood and Interest
/// get floors (their closest members: co-worker, neighbour, group member flags).
#[must_use]
pub fn ties1_reference(k: GroupKind) -> Option<(&'static str, [f64; 9])> {
    match k {
        GroupKind::Household => Some(("same house by definition", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])),
        GroupKind::Kin => {
            let mut b = [0.0; 9];
            for (_, w, v) in &TIES1_RELATIONS[1..5] {
                for i in 1..9 {
                    b[i] += w * v[i];
                }
            }
            Some(("family elsewhere (outside the house)", norm(b)))
        }
        GroupKind::CloseFriends => Some(("non-kin closest ties", TIES1_RELATIONS[5].2)),
        GroupKind::Work => Some(("floor: co-worker closest ties", TIES1_FLAGS[0].1)),
        GroupKind::Neighbourhood => Some(("floor: neighbour closest ties", TIES1_FLAGS[1].1)),
        GroupKind::Interest => Some(("floor: group-member closest ties", TIES1_FLAGS[2].1)),
        _ => None,
    }
}

/// Cumulative share of a TIES-1 band vector within 100 miles (bands 0..=5) and within 500
/// miles (0..=6): the twin's locality half-width (~300 km, 190 miles) lies between them.
#[must_use]
pub fn ties1_within(v: &[f64; 9]) -> (f64, f64) {
    (v[..6].iter().sum(), v[..7].iter().sum())
}

// ---------------------------------------------------------------------------
// The sweep over room placement
// ---------------------------------------------------------------------------

/// One sweep point.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepRow {
    /// What was set.
    pub setting: String,
    /// Act locality at the locality band (the page's definition), the seven social.rs kinds.
    pub act_local: f64,
    /// Delivery locality at the locality band.
    pub delivery_local: f64,
    /// Same, all ten kinds.
    pub act_local_all10: f64,
    /// Same, all ten kinds.
    pub delivery_local_all10: f64,
    /// Share of delivered bytes leaving the metro (IXP or long-haul), all ten kinds.
    pub bytes_leave_metro: f64,
    /// Delivery share within the tile (~1 km), all ten kinds: the sub-kilometre bands.
    pub delivery_within_tile: f64,
}

fn headline(l: &Localities) -> SweepRow {
    let mut a7 = Acc::default();
    for k in &l.kinds[..7] {
        let mut x = Acc::default();
        x.acts = k.acts;
        x.deliveries = k.deliveries;
        x.act_within = k.act_within.map(|v| v * k.acts);
        x.band = k.delivery_band.map(|v| v * k.deliveries);
        a7.add(&x);
    }
    let local_d = |band: &[f64; 7]| band[..=LOCALITY_BAND].iter().sum::<f64>();
    SweepRow {
        setting: String::new(),
        act_local: a7.act_within[LOCALITY_BAND] / a7.acts.max(1e-12),
        delivery_local: local_d(&a7.band) / a7.deliveries.max(1e-12),
        act_local_all10: l.all.act_within[LOCALITY_BAND],
        delivery_local_all10: local_d(&l.all.delivery_band),
        bytes_leave_metro: l.all.path_segment[3] + l.all.path_segment[4],
        delivery_within_tile: l.all.delivery_band[..3].iter().sum(),
    }
}

/// The room-placement inputs swept, one at a time from the defaults, then jointly, then the
/// placeless null model.
#[must_use]
pub fn sweep(geo: &Geography, persons: usize, horizon_s: f64, seed: u64) -> Vec<SweepRow> {
    let pm = PathModel::build(seed, ParameterRegime::Expected);
    let lp = Loc2Params::default();
    let ap = ActivityParams::poisson();
    let run = |sp: &SocialParams, pl: Placement, lp: &Loc2Params| {
        let w = SocialWorld::build(geo, sp, persons, seed, pl);
        headline(&measure(&w, &ap, lp, &pm, &StandInPresence, horizon_s, seed))
    };
    let mut rows = Vec::new();
    let base = SocialParams::default();
    let mut push = |setting: String, mut r: SweepRow| {
        r.setting = setting;
        rows.push(r);
    };
    push("defaults".to_string(), run(&base, Placement::Placed, &lp));
    type Setter = fn(&mut SocialParams, f64);
    let one: [(&str, Setter, &[f64]); 5] = [
        ("interest_local_share", |p, x| p.interest_local_share = x, &[0.1, 0.3, 0.7, 0.9]),
        ("public_local_share", |p, x| p.public_local_share = x, &[0.1, 0.7]),
        ("diaspora_share", |p, x| p.diaspora_share = x, &[0.02, 0.3]),
        ("work_cross_locality_share", |p, x| p.work_cross_locality_share = x, &[0.0, 0.2]),
        ("public_follows_per_person", |p, x| p.public_follows_per_person = x, &[0.0, 10.0]),
    ];
    for (name, set, vals) in one {
        for &x in vals {
            let mut p = base.clone();
            set(&mut p, x);
            push(format!("{name} = {x}"), run(&p, Placement::Placed, &lp));
        }
    }
    for x in [0.1, 0.5, 0.9] {
        let mut p = base.clone();
        p.interest_local_share = x;
        p.public_local_share = x;
        p.interest_regional_share = (1.0 - x) * 0.6;
        p.public_regional_share = (1.0 - x) * 0.5;
        push(format!("joint: interest and public local share = {x}"), run(&p, Placement::Placed, &lp));
    }
    let mut l2 = lp.clone();
    l2.set(L::CloseFriendsTile, 0.1);
    l2.set(L::CloseFriendsLocality, 0.2);
    push("close friends 30% local (tile + locality)".to_string(), run(&base, Placement::Placed, &l2));
    for (name, row, vals) in [("band.households_per_building", L::HouseholdsPerBuilding, [1.0, 20.0]), ("band.households_per_tile", L::HouseholdsPerTile, [100.0, 2000.0])] {
        for x in vals {
            let mut l3 = lp.clone();
            l3.set(row, x);
            push(format!("{name} = {x} (sub-kilometre, FRONTIER)"), run(&base, Placement::Placed, &l3));
        }
    }
    // close friends: TIES-1's anchor is the default; the share within the tile is swept over
    // TIES-0's no-licence bracket (tile + locality held at the default's sum)
    let own = lp.get(L::CloseFriendsTile) + lp.get(L::CloseFriendsLocality);
    for x in [own * TIES0_WITHIN_1KM_BRACKET.0, own * TIES0_WITHIN_1KM_BRACKET.1] {
        let mut l4 = lp.clone();
        l4.set(L::CloseFriendsTile, x);
        l4.set(L::CloseFriendsLocality, own - x);
        push(format!("close_friends.tile_share = {x:.4} (TIES-0 below-district bracket, FRONTIER)"), run(&base, Placement::Placed, &l4));
    }
    push("placeless null model (every non-household room ignores place)".to_string(), run(&base, Placement::Placeless, &lp));
    rows
}

// ---------------------------------------------------------------------------
// Report, CLI
// ---------------------------------------------------------------------------

fn pct(x: f64) -> String {
    format!("{:.1}%", 100.0 * x)
}

/// The markdown report.
#[must_use]
pub fn report(geo: &Geography, geo_name: &str, persons: usize, days: f64, seed: u64, with_sweep: bool) -> String {
    let horizon = days * 86_400.0;
    let pm = PathModel::build(seed, ParameterRegime::Expected);
    let w = SocialWorld::build(geo, &SocialParams::default(), persons, seed, Placement::Placed);
    let l = measure(&w, &ActivityParams::poisson(), &Loc2Params::default(), &pm, &StandInPresence, horizon, seed);
    let l0 = measure(&w, &ActivityParams::poisson(), &Loc2Params::default(), &pm, &Presence0Presence::default(), horizon, seed);
    let mut s = format!("# Three localities ({persons} persons, {days} day(s), seed {seed}, geography: {geo_name})\n\nNever merged into one number. Each carries the verdict of its inputs.\n\n");
    s.push_str(&format!("## (a) Audience locality: where recipients sit\n\nVerdict: **{}**. {}. Will read: {}.\n\n", l.audience.class.as_str(), l.audience.note, l.audience.reads.join(", ")));
    s.push_str("Share of deliveries per band, and act locality (every recipient within the band) at the locality band.\n\n| kind | acts | deliveries |");
    for b in BANDS {
        s.push_str(&format!(" {b} |"));
    }
    s.push_str(" acts all within locality |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for k in l.kinds.iter().chain(core::iter::once(&l.all)) {
        let name = if core::ptr::eq(k, &l.all) { "all kinds" } else { k.kind.as_str() };
        s.push_str(&format!("| {name} | {:.0} | {:.0} |", k.acts, k.deliveries));
        for x in k.delivery_band {
            s.push_str(&format!(" {} |", pct(x)));
        }
        s.push_str(&format!(" {} |\n", pct(k.act_within[LOCALITY_BAND])));
    }
    s.push_str(&format!("\n## (b) Path locality: where the bytes go\n\nVerdict: **{}**. {}. Will read: {}.\n\nPath model: same-locality cross-network segments {:?}, one-network share {:.2}; cross-locality (same country / same world region / other) {:?}; relayed share of pairs fixed / Wi-Fi / cellular {:?}; path to the nearest data centre {:?}; relay offering {:.2} (expected regime). Path locality is stated against the status quo (every delivery up to the platform's data centre and down, public rooms served from a CDN inside the ISP {:.0}% of the time, Arcep 2024); Arcep also reports public IXP peering at {:.1}% of inbound, far below the stand-in's IXP share on data-centre paths, so the IXP column is the stand-in's.\n\n| kind |", l.path.class.as_str(), l.path.note, l.path.reads.join(", "), pm.same_metro.map(|x| (x * 1000.0).round() / 1000.0), pm.same_network, pm.cross.map(|v| v.map(|x| (x * 1000.0).round() / 1000.0)), pm.relayed.map(|x| (x * 1000.0).round() / 1000.0), pm.to_dc.map(|x| (x * 1000.0).round() / 1000.0), pm.relay_offer, 100.0 * Loc2Params::default().get(L::CdnInIspShare), 100.0 * Loc2Params::default().get(L::PublicIxpInboundShare)));
    for g in SEGMENTS {
        s.push_str(&format!(" {g} |"));
    }
    s.push_str(" relayed | relayed by a neighbour | status quo: in one network / leaves the metro | mesh minus status quo: in one network or LAN / leaves the metro |\n|---|---|---|---|---|---|---|---|---|---|\n");
    for k in l.kinds.iter().chain(core::iter::once(&l.all)) {
        let name = if core::ptr::eq(k, &l.all) { "all kinds" } else { k.kind.as_str() };
        s.push_str(&format!("| {name} |"));
        for x in k.path_segment {
            s.push_str(&format!(" {} |", pct(x)));
        }
        let b = &k.path_baseline;
        let near = |v: &[f64; 5]| v[0] + v[1];
        let leave = |v: &[f64; 5]| v[3] + v[4];
        s.push_str(&format!(" {} | {} | {} / {} | {:+.1} pp / {:+.1} pp |\n", pct(k.relayed), pct(k.relayed_by_neighbour), pct(near(b)), pct(leave(b)), 100.0 * (near(&k.path_segment) - near(b)), 100.0 * (leave(&k.path_segment) - leave(b))));
    }
    s.push_str(&format!("\n## (c) Presence-weighted locality\n\nVerdict: **{}**. {}. Will read: {}.\n\nBeside it, PRESENCE-0: verdict **{}**. {}.\n\nDeliveries weighted by the recipient being awake at the act's local hour; the path by who is at home and on Wi-Fi at that hour. (b) uses the presence's daily means; (c) the hour's.\n\n| kind | within locality (a) | within locality (c, stand-in) | within locality (c, PRESENCE-0) | home LAN (b, stand-in) | home LAN (c, stand-in) | home LAN (c, PRESENCE-0) | leaves the metro (b, stand-in) | leaves the metro (c, stand-in) | leaves the metro (c, PRESENCE-0) |\n|---|---|---|---|---|---|---|---|---|---|\n", l.presence.class.as_str(), l.presence.note, l.presence.reads.join(", "), l0.presence.class.as_str(), l0.presence.note));
    let k0s: Vec<&KindLocality> = l0.kinds.iter().chain(core::iter::once(&l0.all)).collect();
    for (i, k) in l.kinds.iter().chain(core::iter::once(&l.all)).enumerate() {
        let name = if core::ptr::eq(k, &l.all) { "all kinds" } else { k.kind.as_str() };
        let k0 = k0s[i];
        let wl = |b: &[f64; 7]| b[..=LOCALITY_BAND].iter().sum::<f64>();
        let lv = |v: &[f64; 5]| v[3] + v[4];
        s.push_str(&format!("| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n", pct(wl(&k.delivery_band)), pct(wl(&k.presence_band)), pct(wl(&k0.presence_band)), pct(k.path_segment[0]), pct(k.presence_segment[0]), pct(k0.presence_segment[0]), pct(lv(&k.path_segment)), pct(lv(&k.presence_segment)), pct(lv(&k0.presence_segment))));
    }
    s.push_str(&format!("\n## (a) at district grain: the reference, CIRISHolon TIES-0 (Germany)\n\n{TIES0_PIN}. One undifferentiated count of friendship links between German districts on one platform: not kinds of tie, not households, not messages; links beyond Europe are not in the file. The twin's column is its acquaintance and kin ties (member pairs of kin, work, school, neighbourhood and interest rooms) in the same bands at its own grain (own area = one grid cell; 'farther' = over 300 km in the same world region; 'foreign' = another world region). **Not the same quantity**: TIES-0 is links on one platform in one country; the twin is room memberships drawn from FRONTIER placement inputs (the page's 'community rooms 90% local', social.rs diaspora 0.12, interest groups 0.5 in the signer's locality). TIES-0's 0.75 within 300 km is an outer envelope for acquaintance and kin links at the twin's locality grain (a locality's half-width is of the order of 300 km), not a target for message locality.\n\n| ties | own area | within 100 km | 100 to 300 km | farther, same country or region | foreign | within 300 km |\n|---|---|---|---|---|---|---|\n"));
    let row = |s: &mut String, name: &str, v: &[f64; 5]| {
        s.push_str(&format!("| {name} |"));
        for x in v {
            s.push_str(&format!(" {} |", pct(*x)));
        }
        s.push_str(&format!(" {} |\n", pct(v[0] + v[1] + v[2])));
    };
    row(&mut s, "TIES-0, all districts (population-weighted)", &TIES0_ALL);
    row(&mut s, "TIES-0, sparsest fifth", &TIES0_FIFTHS[0]);
    row(&mut s, "TIES-0, densest fifth", &TIES0_FIFTHS[4]);
    row(&mut s, "twin, ties weighted flat (k = 0)", &tie_bands(&w, geo, 0.0, seed));
    row(&mut s, "twin, strong ties talk more (k = 1, MODEL)", &tie_bands(&w, geo, 1.0, seed));
    s.push_str(&format!("\nBelow a district TIES-0 gives no licence: the share of own-district links within 1 km lies anywhere in [{:.1}%, {:.1}%] for the median district (a bracket, not a measurement). The household, building, tile and close-friends placements stay FRONTIER and are swept below; a distance law carried down one scale failed at Holon (2.8% of the population within 0.05, against 53.5% for a density-fifth mean), so no kernel is used.\n", 100.0 * TIES0_WITHIN_1KM_BRACKET.0, 100.0 * TIES0_WITHIN_1KM_BRACKET.1));
    s.push_str(&format!("\n## Audience locality three ways, side by side\n\n1. **Legacy inputs (by construction, MODEL)**: the page draws families 100% in the signer's locality and community rooms 90/9/1; act mix 65/30/4/1. Nothing measured.\n2. **Weak-tie outer envelope at district grain (TIES-0, Germany, one platform's links)**: 74.9% within 300 km; no licence below a district.\n3. **Closest ties by kind (TIES-1, United States 2008, adults, up to ten closest ties; {TIES1_PIN})**: shares within 100 miles and within 500 miles (the twin's locality half-width, ~300 km, lies between). Work, Neighbourhood and Interest get floors only (their closest members); School, Public, Own devices and Own agents have no reference and stay FRONTIER. Close-tie distance is not keyed on density (TIES-1 shows no rise with it).\n\nThe twin's own column is (a) above: the share of each kind's deliveries inside the locality, and, for kin and close friends, the same outside the household.\n\n| kind | legacy (page, by construction) | TIES-1 reference | TIES-1 within 100 mi / 500 mi | twin (a) within locality | twin (a) within locality, outside the household | verdict |\n|---|---|---|---|---|---|---|\n"));
    for k in &l.kinds {
        let legacy = match k.kind {
            GroupKind::Household | GroupKind::Kin => "100% (family)",
            GroupKind::Work | GroupKind::School | GroupKind::Neighbourhood | GroupKind::Interest | GroupKind::CloseFriends => "90% (community)",
            GroupKind::Public => "1% public rooms are world rooms",
            _ => "not on the page",
        };
        let within = k.delivery_band[..=LOCALITY_BAND].iter().sum::<f64>();
        let outside_hh = if k.delivery_band[0] < 1.0 { (within - k.delivery_band[0]) / (1.0 - k.delivery_band[0]) } else { f64::NAN };
        let (lab, w100, w500, verdict) = match ties1_reference(k.kind) {
            Some((lab, v)) => {
                let (a, b) = ties1_within(&v);
                (lab, pct(a), pct(b), if lab.starts_with("floor") { "MODEL floor (US 2008 close members only)" } else { "MODEL reference (US 2008, closest ties)" })
            }
            None => ("none", "-".to_string(), "-".to_string(), "FRONTIER: no reference"),
        };
        s.push_str(&format!("| {} | {legacy} | {lab} | {w100} / {w500} | {} | {} | {verdict} |\n", k.kind.as_str(), pct(within), if outside_hh.is_nan() { "-".to_string() } else { pct(outside_hh) }));
    }
    let close = [GroupKind::Household, GroupKind::Kin, GroupKind::CloseFriends];
    let d: f64 = close.iter().map(|&g| l.kinds[g as usize].deliveries).sum();
    let hh = l.kinds[GroupKind::Household as usize].deliveries;
    let (f, c, h) = TIES1_HOUSEHOLD_ELECTRONIC;
    s.push_str(&format!("\n**Audience locality at household grain is a bracket, not a point.** TIES-1: the household carries at least {:.0}% of electronic contact with closest ties, {:.0}% correcting for unnamed co-residents, up to {:.0}% (a 2008 price; written contact follows ties band for band within 0.03, so ties stand in for written messages with no extra weight on near ties; voice leans to the house at 0.325, in person 0.437). The twin's household share of its close-tie kinds' deliveries (household, kin, close friends) is {} ({}).\n\nSub-district: of closest ties outside the house and within 50 miles, {:.1}% are on the block or under a mile (an evenly spread distance law gives 2%); the close-friends tile share takes TIES-1's non-kin value ({:.1}%) as its central value, swept below. Weak ties below a district still have no licence.\n", 100.0 * f, 100.0 * c, 100.0 * h, pct(hh / d.max(1e-12)), if hh / d > h { "above the bracket" } else if hh / d < f { "below the bracket" } else { "inside the bracket" }, 100.0 * TIES1_SUBDISTRICT_ANCHOR, 100.0 * Loc2Params::default().get(L::CloseFriendsTile)));
    if with_sweep {
        s.push_str("\n## Sweep: the headline as a function of the room-placement inputs\n\nAct and delivery locality at the locality band (the page's definition) over social.rs's seven kinds, then over all ten, and the share of delivered bytes that leave the metro. Inputs one at a time from the defaults, then jointly, then the placeless null.\n");
        let rows = sweep(geo, persons, horizon, seed);
        let sub: Vec<f64> = rows.iter().filter(|r| r.setting == "defaults" || r.setting.contains("FRONTIER")).map(|r| r.delivery_within_tile).collect();
        let (lo, hi) = sub.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| (a.0.min(x), a.1.max(x)));
        let loc: Vec<f64> = rows.iter().filter(|r| r.setting == "defaults" || r.setting.contains("FRONTIER")).map(|r| r.delivery_local_all10).collect();
        let (llo, lhi) = loc.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, &x| (a.0.min(x), a.1.max(x)));
        let bands_moved = rows.iter().filter(|r| r.setting.starts_with("band.")).any(|r| (r.delivery_within_tile - rows[0].delivery_within_tile).abs() > 1e-3);
        let verdict = if hi - lo > 0.05 {
            "it swings by more than 5 points: **the model cannot state a sub-district locality yet**"
        } else if !bands_moved {
            "the building and tile inputs do not move it at all: at this world size a grid cell holds fewer households than one building or tile chunk, so those bands are not resolved and what lies 'within the tile' is the household and same-cell placement. **The model cannot state a sub-district locality yet**; it needs a world placed at street grain (the micro tier); for close ties TIES-1 now gives one anchor (14.2% of close ties outside the house within 50 miles are on the block or under a mile), for weak ties below a district there is still no reference"
        } else {
            "it moves by under 5 points at this world's placement"
        };
        s.push_str(&format!("\nAcross the sub-kilometre FRONTIER rows the delivery share within the tile runs {} to {} ({verdict}); delivery locality at the locality band runs {} to {} over the same rows.\n\n", pct(lo), pct(hi), pct(llo), pct(lhi)));
        s.push_str("| setting | act locality (7 kinds) | delivery locality (7) | act locality (10) | delivery locality (10) | delivery within the tile (10) | bytes leaving the metro |\n|---|---|---|---|---|---|---|\n");
        for r in rows {
            s.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} |\n", r.setting, pct(r.act_local), pct(r.delivery_local), pct(r.act_local_all10), pct(r.delivery_local_all10), pct(r.delivery_within_tile), pct(r.bytes_leave_metro)));
        }
    }
    s
}

/// `cewp-cli locality [--persons N] [--days D] [--seed S] [--geo docs/globe.js|stand-in] [--sweep]`.
pub fn cli(args: &[String]) -> Result<String, String> {
    let (mut persons, mut days, mut seed, mut geo_path, mut with_sweep) = (5000usize, 1.0f64, 1u64, "docs/globe.js".to_string(), false);
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--sweep" {
            with_sweep = true;
            i += 1;
            continue;
        }
        let val = args.get(i + 1).ok_or_else(|| format!("{} needs a value", args[i]))?;
        let num = || val.parse::<f64>().map_err(|e| format!("{}: {e}", args[i]));
        match args[i].as_str() {
            "--persons" => persons = num()? as usize,
            "--days" => days = num()?,
            "--seed" => seed = num()? as u64,
            "--geo" => geo_path = val.clone(),
            other => return Err(format!("locality: unknown option {other}")),
        }
        i += 2;
    }
    if geo_path == "stand-in" {
        return Ok(report(&Geography::stand_in(seed, 181, 16, 9), "generated stand-in (MODEL)", persons, days, seed, with_sweep));
    }
    let src = std::fs::read_to_string(&geo_path).map_err(|e| format!("{geo_path}: {e} (run from the repo root, or pass --geo stand-in)"))?;
    let geo = Geography::from_globe_js(&src)?;
    Ok(report(&geo, &format!("the page's population grid ({geo_path})"), persons, days, seed, with_sweep))
}

// ---------------------------------------------------------------------------
// Globe view
// ---------------------------------------------------------------------------

/// Floats per row of [`view_localities`].
pub const VIEW_STRIDE: usize = 1 + 7 + 5 + 5 + 2 + 7 + 5 + 3;

/// The three localities per group kind, `VIEW_STRIDE` f32 per row, kinds in
/// [`GroupKind::ALL`] order then one "all kinds" row (kind = 10):
/// `[kind, delivery_band[7], path_segment[5], path_baseline[5], relayed, relayed_by_neighbour,
///   presence_band[7], presence_segment[5], class_a, class_b, class_c]`; bands in [`BANDS`]
/// order, segments in [`SEGMENTS`] order, classes as `seat_hooks::class` codes.
#[must_use]
pub fn view_localities(seed: u64) -> Vec<f32> {
    let geo = Geography::stand_in(seed, 30, 4, 5);
    let w = SocialWorld::build(&geo, &SocialParams::default(), 2000, seed, Placement::Placed);
    let pm = PathModel::build(seed, ParameterRegime::Expected);
    let l = measure(&w, &ActivityParams::poisson(), &Loc2Params::default(), &pm, &StandInPresence, 86_400.0, seed);
    let mut v = Vec::with_capacity(11 * VIEW_STRIDE);
    for (i, k) in l.kinds.iter().chain(core::iter::once(&l.all)).enumerate() {
        v.push(i as f32);
        v.extend(k.delivery_band.iter().map(|&x| x as f32));
        v.extend(k.path_segment.iter().map(|&x| x as f32));
        v.extend(k.path_baseline.iter().map(|&x| x as f32));
        v.extend_from_slice(&[k.relayed as f32, k.relayed_by_neighbour as f32]);
        v.extend(k.presence_band.iter().map(|&x| x as f32));
        v.extend(k.presence_segment.iter().map(|&x| x as f32));
        v.extend_from_slice(&[l.audience.class.code(), l.path.class.code(), l.presence.class.code()]);
    }
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn small(seed: u64, pl: Placement, sp: &SocialParams) -> (SocialWorld, PathModel) {
        let geo = Geography::stand_in(seed, 15, 3, 3);
        (SocialWorld::build(&geo, sp, 1500, seed, pl), PathModel::build(seed, ParameterRegime::Expected))
    }

    fn run(w: &SocialWorld, pm: &PathModel, lp: &Loc2Params, seed: u64) -> Localities {
        measure(w, &ActivityParams::poisson(), lp, pm, &StandInPresence, 86_400.0 / 4.0, seed)
    }

    #[test]
    fn shares_sum_to_one_and_acts_nest() {
        let (w, pm) = small(1, Placement::Placed, &SocialParams::default());
        let l = run(&w, &pm, &Loc2Params::default(), 1);
        for k in l.kinds.iter().chain(core::iter::once(&l.all)) {
            if k.deliveries <= 0.0 {
                continue;
            }
            for v in [k.path_baseline.iter().sum::<f64>(), k.delivery_band.iter().sum::<f64>(), k.path_segment.iter().sum::<f64>(), k.presence_band.iter().sum::<f64>(), k.presence_segment.iter().sum::<f64>()] {
                assert!((v - 1.0).abs() < 1e-9, "{:?} {v}", k.kind);
            }
            for b in 1..7 {
                assert!(k.act_within[b] + 1e-12 >= k.act_within[b - 1], "within-band act shares nest");
            }
            assert!((k.act_within[6] - 1.0).abs() < 1e-9);
        }
        // a household room's recipients are all in the household
        assert!((l.kinds[0].delivery_band[0] - 1.0).abs() < 1e-12);
        assert!((l.kinds[GroupKind::OwnDevices as usize].delivery_band[0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn placement_moves_the_headline_monotonically() {
        let lp = Loc2Params::default();
        let mut last = -1.0;
        for x in [0.1, 0.5, 0.9] {
            let mut sp = SocialParams::default();
            sp.interest_local_share = x;
            sp.interest_regional_share = (1.0 - x) * 0.6;
            let (w, pm) = small(2, Placement::Placed, &sp);
            let h = headline(&run(&w, &pm, &lp, 2));
            assert!(h.delivery_local > last - 0.01, "more local interest groups do not lower delivery locality: {x} {}", h.delivery_local);
            last = h.delivery_local;
        }
        let (w0, pm) = small(2, Placement::Placeless, &SocialParams::default());
        let (w1, _) = small(2, Placement::Placed, &SocialParams::default());
        let a = headline(&run(&w0, &pm, &lp, 2));
        let b = headline(&run(&w1, &pm, &lp, 2));
        assert!(b.act_local > a.act_local, "placed rooms are more local than the placeless null");
    }

    #[test]
    fn the_three_measures_are_different_numbers() {
        let (w, pm) = small(3, Placement::Placed, &SocialParams::default());
        let l = run(&w, &pm, &Loc2Params::default(), 3);
        let a = l.all.delivery_band[..=LOCALITY_BAND].iter().sum::<f64>();
        let b = l.all.path_segment[0] + l.all.path_segment[1] + l.all.path_segment[2];
        assert!(l.all.relayed > 0.0 && l.all.relayed < 1.0);
        // bytes of local audiences can still leave the metro (tromboning, relays): b is not a
        assert!((a - b).abs() > 1e-6);
        assert_eq!(l.audience.class, Class::Frontier, "an uncertified tie table says so");
        assert!(l.audience.note.contains("uncertified"));
        assert!(!l.presence.reads.is_empty() && l.presence.reads.contains(&"presence.people"));
    }

    #[test]
    fn ties1_tables_are_shares() {
        assert!((TIES1_ALL.iter().sum::<f64>() - 1.0).abs() < 2e-3);
        for (_, _, v) in &TIES1_RELATIONS {
            assert!((v.iter().sum::<f64>() - 1.0).abs() < 2e-3);
        }
        let (k100, k500) = ties1_within(&ties1_reference(GroupKind::Kin).unwrap().1);
        // TIES-1 T4: kin ties elsewhere 0.68 within 100 miles, 0.82 within 500
        assert!((k100 - 0.68).abs() < 0.03 && (k500 - 0.82).abs() < 0.03, "{k100} {k500}");
        let lp = Loc2Params::default();
        let cf: f64 = [L::CloseFriendsTile, L::CloseFriendsLocality, L::CloseFriendsCountry, L::CloseFriendsRegion].iter().map(|&x| lp.get(x)).sum();
        assert!(cf <= 1.0 && cf > 0.95);
        assert!(ties1_reference(GroupKind::School).is_none() && ties1_reference(GroupKind::OwnAgents).is_none());
    }

    #[test]
    fn presence_seam_has_168_bins_in_range() {
        let q = Presence0Presence::default();
        for b in 0..HOW_BINS {
            for x in [q.awake(b), q.at_home(b), q.on_wifi(b)] {
                assert!((-1e-9..=1.0 + 1e-9).contains(&x));
            }
        }
        assert!(q.awake(3) < 0.1 && q.awake(19) > 0.9, "PRESENCE-0: 0.04 at 03:00, 0.98 early evening");
        let p = StandInPresence;
        for b in 0..HOW_BINS {
            for x in [p.awake(b), p.at_home(b), p.on_wifi(b)] {
                assert!((0.0..=1.0).contains(&x));
            }
        }
        assert!(p.awake(3) < p.awake(19), "night below evening");
    }

    #[test]
    fn deterministic_by_seed() {
        let (w, pm) = small(4, Placement::Placed, &SocialParams::default());
        let a = run(&w, &pm, &Loc2Params::default(), 4);
        let b = run(&w, &pm, &Loc2Params::default(), 4);
        assert_eq!(a, b);
        assert_eq!(view_localities(4), view_localities(4));
        assert_eq!(view_localities(4).len(), 11 * VIEW_STRIDE);
    }

    #[test]
    #[ignore = "prints the report and sweep; run with --ignored --nocapture"]
    fn print_report() {
        println!("{}", report(&Geography::stand_in(1, 181, 16, 9), "generated stand-in (MODEL)", 5000, 1.0, 1, true));
    }
}
