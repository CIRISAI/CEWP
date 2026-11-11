//! Correlated failure domains (REVIEW.md §3 item 6; spec `FSD/twin/failures.md`).
//!
//! The twin's churn elsewhere is per node: a share of devices is offline, drawn independently
//! (`micro.rs` `mobile_offline_share`, FSD §4.4 `p_offline`). The FSD §4.4 adversarial regime
//! names a "correlated regional ASN cut", but no code modelled one before this module. Removing
//! 10% of nodes at random is not the event of losing one ISP, one grid region or one country's
//! international links: those take out nodes that share a fate, and a partition keeps the far
//! side alive but unreachable.
//!
//! This module gives every node failure-domain labels (household, operator, access network,
//! metro, power-grid region, country, cloud region), a seeded library of nine correlated
//! scenarios with tagged parameters, a state evaluator (who is up, who can reach whom), and a
//! harness that compares a domain event with the same number of nodes lost at random on the
//! same world and seed.
//!
//! # What is generated
//!
//! [`DomainWorld`] is a generated stand-in: countries, metros, grid regions, access ISPs,
//! cellular carriers, cables and cloud regions with MODEL shares. Real AS relationships and
//! cable landings belong to the `topology` package and to datasets that need Eric's approval
//! (see the spec); [`FailureDomainSource`] is the seam they plug in behind. No real network,
//! place or person appears here: ids are indices.
//!
//! # Reachability rule (MODEL)
//!
//! Two up nodes reach each other when their access networks are connected in the network graph
//! (access AS to its country core, country core to the global core over that country's cables),
//! no route leak separates them, and either end has a public listener or a non-withholding relay
//! sits in their component (CC 5.4.3: without a direct path a row crosses a forwarder). Phones
//! are client-only and never public (CIRISEdge `DeviceState`; `micro.rs`).

use crate::micro::Rng;
use std::collections::BTreeMap;

// ------------------------------------------------------------------ classes and parameters

/// FSD §8.2 epistemic class of a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Benchmarked on real code or real hardware.
    Measured,
    /// Computed from other tagged numbers by a stated rule.
    Derived,
    /// A stated modelling choice, or the output of the twin under stated inputs.
    Model,
    /// Extrapolated beyond the tiers the twin runs.
    Projected,
    /// Nothing supports the value yet; a placeholder until field data exists.
    Frontier,
}

impl Class {
    /// The tag as written in the FSD.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "MEASURED",
            Self::Derived => "DERIVED",
            Self::Model => "MODEL",
            Self::Projected => "PROJECTED",
            Self::Frontier => "FRONTIER",
        }
    }
}

/// One named failure-model parameter.
#[derive(Debug, Clone, Copy)]
pub struct FSpec {
    /// Stable name.
    pub name: &'static str,
    /// Default value.
    pub default: f64,
    /// FSD §8.2 class.
    pub class: Class,
    /// Where the default comes from. "Anchor" names a public post-incident figure recalled from
    /// memory and NOT re-read for this work: it bounds the order of magnitude, it does not make
    /// the parameter measured.
    pub source: &'static str,
}

macro_rules! fparams {
    ($( $idx:ident = $name:literal, $def:expr, $class:ident, $src:literal; )*) => {
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        /// Index of each failure parameter in [`FailureParams::v`].
        pub enum F { $( $idx, )* }
        /// Every failure-model parameter, in index order.
        pub const FAILURE_SPECS: &[FSpec] = &[
            $( FSpec { name: $name, default: $def, class: Class::$class, source: $src }, )*
        ];
    };
}

fparams! {
    // ---- the generated world (stand-in for the topology package and for approved datasets)
    Nodes = "world.nodes", 10_000.0, Model, "REVIEW 'concrete first milestone': a 10,000-node benchmark";
    Countries = "world.countries", 12.0, Model, "enough jurisdictions for national events to be a minority of the world";
    MetrosMax = "world.metros_per_country_max", 5.0, Model, "1..=max metros per country, Zipf population weights";
    MetroZipf = "world.metro_zipf", 1.0, Model, "metro population ~ 1/rank^s inside a country (city-size law shape, not fitted)";
    PowerMetrosMax = "world.power_region_metros_max", 2.0, Model, "a grid region spans 1..=max consecutive metros of one country";
    FixedIsps = "world.fixed_isps_per_country", 4.0, Model, "access ISPs per country; share ~ 1/rank^s";
    IspZipf = "world.isp_zipf", 1.0, Model, "market concentration of access ISPs (s = 1: the largest of four holds 48%)";
    CellCarriers = "world.cell_carriers_per_country", 3.0, Model, "cellular carriers per country; share ~ 1/rank^s with the ISP exponent";
    CablesMax = "world.cables_per_country_max", 4.0, Model, "1..=max international links (cables) per country; a country with one is an island";
    CableShared = "world.cable_shared_share", 0.5, Model, "share of cables that also land in a second country (one cut, two countries)";
    CloudProviders = "world.cloud_providers", 3.0, Model, "hosting providers, share ~ 1/rank";
    CloudRegions = "world.cloud_regions_per_provider", 4.0, Model, "regions per provider, placed in metros by population";
    PersonsMax = "world.persons_per_household_max", 4.0, Model, "1..=max persons per household";
    HomeNodeShare = "world.home_node_share", 0.35, Model, "households with an always-on home node (NAS / mini-PC)";
    LaptopShare = "world.laptop_share", 0.6, Model, "persons with a laptop running CIRIS";
    HostedShare = "world.hosted_share", 0.03, Model, "persons who rent a hosted server";
    PhoneWifi = "world.phone_wifi_share", 0.6, Model, "phones on home Wi-Fi at the instant (the rest on cellular)";
    ReachHome = "world.reach_home_share", 0.40, Frontier, "FSD 4.4 Expected regime; micro.rs reach_home_share; real NAT mix awaits leviculum#100";
    PublicListener = "world.public_listener_share", 0.30, Model, "micro.rs public_listener_share: of reachable home routers, the share with an inbound listener";
    RelayPref = "world.relay_pref", 0.90, Model, "FSD 4.4 Expected relay offer opt-in";
    MobileOffline = "world.mobile_offline_share", 0.15, Model, "FSD 4.4 Expected p_offline; micro.rs mobile_offline_share";
    LaptopAwake = "world.laptop_awake_share", 0.5, Model, "laptops with the lid open at the instant";
    UpsShare = "world.ups_share", 0.05, Model, "home nodes (and their router) on a UPS that outlasts the evaluation hour";
    Seats = "world.seats", 1.0, Model, "serve seats; production shape is one (micro.rs default M = 1)";
    RoomsPerPerson = "world.rooms_per_person", 0.3, Model, "calls/rooms in the instant per person";
    RoomSizeMax = "world.room_size_max", 8.0, Model, "room sizes 2..=max, uniform";
    RoomLocality = "world.room_locality", 0.65, Model, "FSD 4.4 Expected locality share: a member after the first is from the first member's metro";
    // ---- evaluation
    EvalHour = "eval.hour", 1.0, Model, "hours after onset at which the state is measured";
    RejoinShare = "eval.rejoin_share", 0.3, Model, "share of nodes that must re-establish a session during an event (wake, network change)";
    PeerCache = "eval.peer_cache_share", 0.5, Model, "rejoining nodes that hold a cached public peer and need no bootstrap seat";
    // ---- regional power failure
    PowerMedianH = "power.median_h", 6.0, Model, "lognormal median; anchor: 2003 Northeast blackout, most load back within ~2 days (US-Canada Task Force final report, 2004)";
    PowerSigma = "power.sigma", 1.0, Model, "lognormal shape (heavy tail: storm outages run days)";
    PowerRamp = "power.ramp_share", 0.6, Model, "last share of the duration over which supply returns linearly (rolling restoration)";
    TowerBackupH = "power.tower_backup_h", 4.0, Frontier, "cell-site battery autonomy during a grid outage; no figure trusted here";
    DcFail = "power.dc_fail", 0.02, Model, "a data-centre node in the region whose generator transfer fails";
    // ---- single-ISP outage
    IspMedianH = "isp.median_h", 4.0, Model, "lognormal median; anchor for the tail: Rogers, Canada, 2022-07-08, about a day nationwide (CRTC inquiry)";
    IspSigma = "isp.sigma", 1.2, Model, "lognormal shape";
    // ---- submarine cable cut
    CablesCut = "cable.cuts", 2.0, Model, "cables cut on the route into one country (uniform over countries); anchor: 2008 Mediterranean cuts (SEA-ME-WE 4 and FLAG, two cables). Connectivity only: capacity loss on surviving cables is not modelled";
    CableMedianH = "cable.median_h", 336.0, Model, "lognormal median (14 d); anchor: Tonga 2022 single cable, ~5 weeks to repair";
    CableSigma = "cable.sigma", 0.7, Model, "lognormal shape";
    // ---- cloud region outage
    CloudMedianH = "cloud.median_h", 5.0, Model, "lognormal median; anchor: AWS us-east-1, 2021-12-07, ~7 h (AWS post-event summary)";
    CloudSigma = "cloud.sigma", 0.8, Model, "lognormal shape";
    CloudRamp = "cloud.ramp_share", 0.5, Model, "share of the duration over which services return";
    // ---- DNS / bootstrap-seat loss
    DnsMedianH = "dns.median_h", 3.0, Model, "lognormal median; anchors: Dyn DDoS 2016-10-21 (hours, in waves); Facebook 2021-10-04 ~6 h (Meta engineering post)";
    DnsSigma = "dns.sigma", 0.8, Model, "lognormal shape";
    // ---- BGP leak / hijack
    BgpAcceptors = "bgp.acceptor_share", 0.3, Model, "share of other ASes that accept the leaked route and lose the victims";
    BgpMedianH = "bgp.median_h", 1.5, Model, "lognormal median; anchor: Pakistan Telecom / YouTube 2008-02-24, ~2 h (RIPE NCC analysis)";
    BgpSigma = "bgp.sigma", 0.7, Model, "lognormal shape";
    // ---- cellular outage
    CellMedianH = "cell.median_h", 6.0, Model, "lognormal median; anchors: AT&T 2024-02-22 ~12 h (FCC report); Optus 2023-11-08 ~12 h";
    CellSigma = "cell.sigma", 0.8, Model, "lognormal shape";
    // ---- national shutdown
    ShutMedianH = "shutdown.median_h", 48.0, Model, "lognormal median; anchor: Egypt 2011-01-28, ~5 days";
    ShutSigma = "shutdown.sigma", 1.0, Model, "lognormal shape";
    ShutMobile = "shutdown.mobile_also", 1.0, Model, "probability the shutdown also cuts domestic cellular (1 = full shutdown)";
    // ---- coordinated relay withholding
    WithholdShare = "withhold.relay_share", 0.2, Model, "share of relays whose operators collude to withhold (forward nothing, serve no chunk)";
    WithholdMedianH = "withhold.median_h", 24.0, Frontier, "time until withholding is noticed and routed around; no instrument exists";
    WithholdSigma = "withhold.sigma", 1.0, Frontier, "lognormal shape";
}

/// Number of failure parameters.
pub const FPARAM_COUNT: usize = FAILURE_SPECS.len();

/// Parameter values, indexed by [`F`].
#[derive(Debug, Clone, PartialEq)]
pub struct FailureParams {
    /// Values in [`FAILURE_SPECS`] order.
    pub v: Vec<f64>,
}

impl Default for FailureParams {
    fn default() -> Self {
        Self { v: FAILURE_SPECS.iter().map(|s| s.default).collect() }
    }
}

impl FailureParams {
    /// Value of one parameter.
    #[must_use]
    pub fn get(&self, p: F) -> f64 {
        self.v[p as usize]
    }
    /// Set one parameter.
    pub fn set(&mut self, p: F, x: f64) {
        self.v[p as usize] = x;
    }
    /// Set by name; false when no parameter has that name.
    pub fn set_by_name(&mut self, name: &str, x: f64) -> bool {
        match FAILURE_SPECS.iter().position(|s| s.name == name) {
            Some(i) => {
                self.v[i] = x;
                true
            }
            None => false,
        }
    }
    fn n(&self, p: F) -> usize {
        self.get(p).max(0.0) as usize
    }
}

// ------------------------------------------------------------------ labels and the seam

/// A failure-domain axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Axis {
    /// One dwelling: one power circuit, one router, one access line.
    Household,
    /// One person: their devices leave, are wiped, or are coerced together.
    Operator,
    /// The access network (AS) the node rides now.
    Asn,
    /// Metro area.
    Metro,
    /// Power-grid region.
    Power,
    /// Jurisdiction.
    Country,
    /// Cloud provider region (hosted nodes only).
    CloudRegion,
    /// Cloud provider (hosted nodes only).
    CloudProvider,
}

/// Every axis, in order.
pub const AXES: [Axis; 8] = [
    Axis::Household,
    Axis::Operator,
    Axis::Asn,
    Axis::Metro,
    Axis::Power,
    Axis::Country,
    Axis::CloudRegion,
    Axis::CloudProvider,
];

impl Axis {
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Household => "household",
            Self::Operator => "operator",
            Self::Asn => "asn",
            Self::Metro => "metro",
            Self::Power => "power",
            Self::Country => "country",
            Self::CloudRegion => "cloud_region",
            Self::CloudProvider => "cloud_provider",
        }
    }
}

/// The seam the topology package (and, once approved, real AS / cable datasets) plugs in
/// behind: a node's label on each failure-domain axis. `None` means the node has no member of
/// that axis (a home device has no cloud region). Labels are opaque ids.
pub trait FailureDomainSource {
    /// Nodes.
    fn node_count(&self) -> usize;
    /// The node's domain on one axis.
    fn domain(&self, node: usize, axis: Axis) -> Option<u64>;
}

// ------------------------------------------------------------------ the generated world

/// What a node runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
    /// Phone: Wi-Fi at home or cellular, client-only, never public.
    Phone,
    /// Always-on home node (`home` class).
    HomeNode,
    /// Laptop (`home` class, sleeps with its lid).
    Laptop,
    /// Rented server in a cloud region (`server` class).
    Hosted,
    /// Serve seat: hosted, the bootstrap and mailbox of last resort.
    Seat,
}

impl NodeKind {
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::HomeNode => "home node",
            Self::Laptop => "laptop",
            Self::Hosted => "hosted",
            Self::Seat => "seat",
        }
    }
    /// Runs as `home` class on a household's line (the review's "home nodes").
    #[must_use]
    pub const fn home_class(self) -> bool {
        matches!(self, Self::HomeNode | Self::Laptop)
    }
    /// Can hold durable chunks in this model (MODEL: phones do not; a seat holds the commons,
    /// not a cohort's durable items).
    #[must_use]
    pub const fn holder_eligible(self) -> bool {
        matches!(self, Self::HomeNode | Self::Laptop | Self::Hosted)
    }
}

/// Kind of an access network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsnKind {
    /// Fixed-line access ISP.
    Fixed,
    /// Cellular carrier.
    Cellular,
    /// Cloud provider region.
    Cloud,
}

/// One access network.
#[derive(Debug, Clone, PartialEq)]
pub struct Asn {
    /// Kind.
    pub kind: AsnKind,
    /// Country it serves (cloud: the region's country).
    pub country: u16,
    /// Rank inside its country (ISPs, carriers) or provider index (cloud).
    pub provider: u16,
    /// Metro of a cloud region.
    pub metro: u32,
}

/// One international link (a submarine or land cable).
#[derive(Debug, Clone, PartialEq)]
pub struct Cable {
    /// Countries it lands in.
    pub landings: Vec<u16>,
}

/// One metro.
#[derive(Debug, Clone, PartialEq)]
pub struct Metro {
    /// Country.
    pub country: u16,
    /// Power-grid region.
    pub power: u32,
    /// Population weight.
    pub weight: f64,
}

/// One node.
#[derive(Debug, Clone, PartialEq)]
pub struct DNode {
    /// Kind.
    pub kind: NodeKind,
    /// Operating person.
    pub person: u32,
    /// Household (hosted nodes: the operator's household id, offset so it never matches a
    /// dwelling; see [`DomainWorld::domain`]).
    pub household: u32,
    /// Country.
    pub country: u16,
    /// Metro.
    pub metro: u32,
    /// Power region.
    pub power: u32,
    /// Fixed-line ISP of the household, or the cloud region's network for hosted nodes.
    pub fixed_asn: u32,
    /// Cellular carrier (phones), else `u32::MAX`.
    pub cell_asn: u32,
    /// Phone currently on home Wi-Fi.
    pub on_wifi: bool,
    /// Inbound listener.
    pub public: bool,
    /// Offers relay.
    pub relay: bool,
    /// Awake at the instant (background, not a failure).
    pub awake: bool,
    /// Home node on a UPS.
    pub ups: bool,
}

/// One person.
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    /// Household.
    pub household: u32,
    /// Metro.
    pub metro: u32,
    /// Their phone (the device they take a call on).
    pub phone: u32,
}

/// A generated world with failure-domain labels (MODEL stand-in; see the module doc).
#[derive(Debug, Clone)]
pub struct DomainWorld {
    /// Seed.
    pub seed: u64,
    /// Countries.
    pub countries: u16,
    /// Metros.
    pub metros: Vec<Metro>,
    /// Power regions.
    pub power_regions: u32,
    /// Access networks.
    pub asns: Vec<Asn>,
    /// International links.
    pub cables: Vec<Cable>,
    /// Nodes.
    pub nodes: Vec<DNode>,
    /// Persons.
    pub persons: Vec<Person>,
    /// Households (dwellings).
    pub households: u32,
    /// Rooms in the instant: member persons.
    pub rooms: Vec<Vec<u32>>,
    /// Cables landing in each country.
    pub country_cables: Vec<Vec<u32>>,
    /// Number of people (by attachment) on each access network.
    pub asn_load: Vec<u32>,
}

/// Hash two words to 64 bits (splitmix finaliser).
#[must_use]
pub fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17);
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A uniform number in [0, 1) from two words.
#[must_use]
pub fn unit(a: u64, b: u64) -> f64 {
    (mix(a, b) >> 11) as f64 / (1u64 << 53) as f64
}

/// Index drawn with probability proportional to `w` (all zero: uniform).
pub fn pick_weighted(r: &mut Rng, w: &[f64]) -> usize {
    let total: f64 = w.iter().sum();
    if total <= 0.0 {
        return r.below(w.len());
    }
    let mut x = r.f() * total;
    for (i, &wi) in w.iter().enumerate() {
        if x < wi {
            return i;
        }
        x -= wi;
    }
    w.len() - 1
}

fn zipf_weights(n: usize, s: f64) -> Vec<f64> {
    (0..n).map(|k| 1.0 / ((k + 1) as f64).powf(s)).collect()
}

impl DomainWorld {
    /// Generate a world.
    #[must_use]
    pub fn build(seed: u64, p: &FailureParams) -> Self {
        let mut r = Rng::new(seed ^ 0xFA11_D0A1);
        let countries = p.n(F::Countries).max(1) as u16;
        let mut metros = Vec::new();
        let mut power_regions = 0u32;
        for c in 0..countries {
            let n_metros = 1 + r.below(p.n(F::MetrosMax).max(1));
            let cw = r.range(0.5, 1.5);
            let zw = zipf_weights(n_metros, p.get(F::MetroZipf));
            let mut k = 0;
            while k < n_metros {
                let span = 1 + r.below(p.n(F::PowerMetrosMax).max(1));
                for j in k..(k + span).min(n_metros) {
                    metros.push(Metro { country: c, power: power_regions, weight: cw * zw[j] });
                }
                power_regions += 1;
                k += span;
            }
        }
        let mut asns = Vec::new();
        let n_fixed = p.n(F::FixedIsps).max(1);
        let n_cell = p.n(F::CellCarriers).max(1);
        for c in 0..countries {
            for k in 0..n_fixed {
                asns.push(Asn { kind: AsnKind::Fixed, country: c, provider: k as u16, metro: u32::MAX });
            }
            for k in 0..n_cell {
                asns.push(Asn { kind: AsnKind::Cellular, country: c, provider: k as u16, metro: u32::MAX });
            }
        }
        let fixed_of = |c: u16, k: usize| (c as usize * (n_fixed + n_cell) + k) as u32;
        let cell_of = |c: u16, k: usize| (c as usize * (n_fixed + n_cell) + n_fixed + k) as u32;
        let metro_w: Vec<f64> = metros.iter().map(|m| m.weight).collect();
        let n_prov = p.n(F::CloudProviders).max(1);
        let n_reg = p.n(F::CloudRegions).max(1);
        let mut provider_regions: Vec<Vec<u32>> = vec![Vec::new(); n_prov];
        for (prov, regs) in provider_regions.iter_mut().enumerate() {
            for _ in 0..n_reg {
                let m = pick_weighted(&mut r, &metro_w);
                regs.push(asns.len() as u32);
                asns.push(Asn { kind: AsnKind::Cloud, country: metros[m].country, provider: prov as u16, metro: m as u32 });
            }
        }
        let mut cables: Vec<Cable> = Vec::new();
        for c in 0..countries {
            let n = 1 + r.below(p.n(F::CablesMax).max(1));
            for _ in 0..n {
                let mut landings = vec![c];
                if countries > 1 && r.chance(p.get(F::CableShared)) {
                    let mut o = r.below(countries as usize - 1) as u16;
                    if o >= c {
                        o += 1;
                    }
                    landings.push(o);
                }
                cables.push(Cable { landings });
            }
        }
        let mut country_cables = vec![Vec::new(); countries as usize];
        for (i, cb) in cables.iter().enumerate() {
            for &l in &cb.landings {
                country_cables[l as usize].push(i as u32);
            }
        }
        let isp_w = zipf_weights(n_fixed, p.get(F::IspZipf));
        let cell_w = zipf_weights(n_cell, p.get(F::IspZipf));
        let prov_w = zipf_weights(n_prov, 1.0);
        let public_share = p.get(F::ReachHome) * p.get(F::PublicListener);
        let mut nodes = Vec::new();
        let mut persons: Vec<Person> = Vec::new();
        let mut households = 0u32;
        let target = p.n(F::Nodes).max(1);
        let add_hosted = |nodes: &mut Vec<DNode>, r: &mut Rng, kind: NodeKind, person: u32, region: u32, asns: &[Asn]| {
            let a = &asns[region as usize];
            let m = &metros[a.metro as usize];
            nodes.push(DNode {
                kind,
                person,
                household: u32::MAX - person,
                country: a.country,
                metro: a.metro,
                power: m.power,
                fixed_asn: region,
                cell_asn: u32::MAX,
                on_wifi: false,
                public: true,
                relay: kind == NodeKind::Seat || r.chance(p.get(F::RelayPref)),
                awake: true,
                ups: true,
            });
        };
        // Seats first, in the first provider's regions (production: one seat).
        for s in 0..p.n(F::Seats) {
            let region = provider_regions[0][s % provider_regions[0].len()];
            add_hosted(&mut nodes, &mut r, NodeKind::Seat, u32::MAX, region, &asns);
        }
        while nodes.len() < target {
            let m = pick_weighted(&mut r, &metro_w) as u32;
            let mt = metros[m as usize].clone();
            let isp = fixed_of(mt.country, pick_weighted(&mut r, &isp_w));
            let hh = households;
            households += 1;
            let n_persons = 1 + r.below(p.n(F::PersonsMax).max(1));
            let first_person = persons.len() as u32;
            let base = DNode {
                kind: NodeKind::HomeNode,
                person: first_person,
                household: hh,
                country: mt.country,
                metro: m,
                power: mt.power,
                fixed_asn: isp,
                cell_asn: u32::MAX,
                on_wifi: true,
                public: false,
                relay: false,
                awake: true,
                ups: false,
            };
            if r.chance(p.get(F::HomeNodeShare)) {
                let public = r.chance(public_share);
                nodes.push(DNode {
                    public,
                    relay: public && r.chance(p.get(F::RelayPref)),
                    ups: r.chance(p.get(F::UpsShare)),
                    ..base.clone()
                });
            }
            for k in 0..n_persons {
                let person = first_person + k as u32;
                let phone = nodes.len() as u32;
                persons.push(Person { household: hh, metro: m, phone });
                nodes.push(DNode {
                    kind: NodeKind::Phone,
                    person,
                    cell_asn: cell_of(mt.country, pick_weighted(&mut r, &cell_w)),
                    on_wifi: r.chance(p.get(F::PhoneWifi)),
                    awake: !r.chance(p.get(F::MobileOffline)),
                    ..base.clone()
                });
                if r.chance(p.get(F::LaptopShare)) {
                    let public = r.chance(public_share);
                    nodes.push(DNode {
                        kind: NodeKind::Laptop,
                        person,
                        public,
                        relay: public && r.chance(p.get(F::RelayPref)),
                        awake: r.chance(p.get(F::LaptopAwake)),
                        ..base.clone()
                    });
                }
                if r.chance(p.get(F::HostedShare)) {
                    let prov = pick_weighted(&mut r, &prov_w);
                    let region = provider_regions[prov][r.below(provider_regions[prov].len())];
                    add_hosted(&mut nodes, &mut r, NodeKind::Hosted, person, region, &asns);
                }
            }
        }
        // Rooms: the first member anywhere, the rest local with the locality share.
        let mut by_metro: Vec<Vec<u32>> = vec![Vec::new(); metros.len()];
        for (i, pp) in persons.iter().enumerate() {
            by_metro[pp.metro as usize].push(i as u32);
        }
        let n_rooms = (persons.len() as f64 * p.get(F::RoomsPerPerson)) as usize;
        let mut rooms = Vec::with_capacity(n_rooms);
        let size_max = p.n(F::RoomSizeMax).max(2);
        for _ in 0..n_rooms {
            let size = 2 + r.below(size_max - 1);
            let first = r.below(persons.len()) as u32;
            let mut members = vec![first];
            let mut tries = 0;
            while members.len() < size && tries < size * 8 {
                tries += 1;
                let local = &by_metro[persons[first as usize].metro as usize];
                let cand = if r.chance(p.get(F::RoomLocality)) {
                    local[r.below(local.len())]
                } else {
                    r.below(persons.len()) as u32
                };
                if !members.contains(&cand) {
                    members.push(cand);
                }
            }
            rooms.push(members);
        }
        let mut w = Self {
            seed,
            countries,
            metros,
            power_regions,
            asns,
            cables,
            nodes,
            persons,
            households,
            rooms,
            country_cables,
            asn_load: Vec::new(),
        };
        let mut load = vec![0u32; w.asns.len()];
        for n in &w.nodes {
            load[w.normal_asn(n) as usize] += 1;
        }
        w.asn_load = load;
        w
    }

    /// The access network a node rides with nothing failed.
    #[must_use]
    pub fn normal_asn(&self, n: &DNode) -> u32 {
        if n.kind == NodeKind::Phone && !n.on_wifi {
            n.cell_asn
        } else {
            n.fixed_asn
        }
    }

    /// Fixed-line ISPs, largest first by attached nodes.
    #[must_use]
    pub fn fixed_isps_by_size(&self) -> Vec<u32> {
        let mut v: Vec<u32> = (0..self.asns.len() as u32)
            .filter(|&a| self.asns[a as usize].kind == AsnKind::Fixed)
            .collect();
        v.sort_by(|&a, &b| self.asn_load[b as usize].cmp(&self.asn_load[a as usize]).then(a.cmp(&b)));
        v
    }

    /// Count of nodes per kind.
    #[must_use]
    pub fn kind_counts(&self) -> BTreeMap<NodeKind, usize> {
        let mut m = BTreeMap::new();
        for n in &self.nodes {
            *m.entry(n.kind).or_insert(0) += 1;
        }
        m
    }
}

impl FailureDomainSource for DomainWorld {
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn domain(&self, node: usize, axis: Axis) -> Option<u64> {
        let n = &self.nodes[node];
        let hosted = matches!(n.kind, NodeKind::Hosted | NodeKind::Seat);
        match axis {
            Axis::Household => (!hosted).then_some(u64::from(n.household)),
            Axis::Operator => Some(u64::from(n.person)),
            Axis::Asn => Some(u64::from(self.normal_asn(n))),
            Axis::Metro => Some(u64::from(n.metro)),
            Axis::Power => Some(u64::from(n.power)),
            Axis::Country => Some(u64::from(n.country)),
            Axis::CloudRegion => hosted.then_some(u64::from(n.fixed_asn)),
            Axis::CloudProvider => hosted.then(|| u64::from(self.asns[n.fixed_asn as usize].provider)),
        }
    }
}

// ------------------------------------------------------------------ scenarios

/// The nine correlated scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioKind {
    /// One power-grid region dark: homes and routers off, phones on battery fall to cellular,
    /// cell sites on battery until it runs out, data centres on generators.
    RegionalPower,
    /// One fixed-line access ISP down nationwide.
    IspOutage,
    /// International cables cut; a country whose every link is cut becomes an island.
    SubmarineCable,
    /// One cloud provider region down: hosted nodes and any seat in it.
    CloudRegion,
    /// The bootstrap name / seat is unresolvable: running sessions continue, rejoining nodes
    /// without a cached peer cannot join.
    DnsBootstrap,
    /// A route leak: one country's access networks unreachable from a share of other ASes.
    BgpLeak,
    /// One cellular carrier down nationwide.
    CellularOutage,
    /// A national shutdown: international links cut, and (by parameter) domestic cellular.
    NationalShutdown,
    /// Colluding operators of a share of relays withhold: forward nothing, serve no chunk.
    RelayWithholding,
}

/// Every scenario.
pub const SCENARIOS: [ScenarioKind; 9] = [
    ScenarioKind::RegionalPower,
    ScenarioKind::IspOutage,
    ScenarioKind::SubmarineCable,
    ScenarioKind::CloudRegion,
    ScenarioKind::DnsBootstrap,
    ScenarioKind::BgpLeak,
    ScenarioKind::CellularOutage,
    ScenarioKind::NationalShutdown,
    ScenarioKind::RelayWithholding,
];

impl ScenarioKind {
    /// Stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RegionalPower => "regional_power",
            Self::IspOutage => "isp_outage",
            Self::SubmarineCable => "submarine_cable",
            Self::CloudRegion => "cloud_region",
            Self::DnsBootstrap => "dns_bootstrap",
            Self::BgpLeak => "bgp_leak",
            Self::CellularOutage => "cellular_outage",
            Self::NationalShutdown => "national_shutdown",
            Self::RelayWithholding => "relay_withholding",
        }
    }
    /// Recovery shape (MODEL).
    #[must_use]
    pub const fn recovery(self) -> Recovery {
        match self {
            Self::RegionalPower | Self::CloudRegion => Recovery::Linear,
            Self::RelayWithholding => Recovery::Exponential,
            _ => Recovery::Step,
        }
    }
    fn duration_params(self) -> (F, F) {
        match self {
            Self::RegionalPower => (F::PowerMedianH, F::PowerSigma),
            Self::IspOutage => (F::IspMedianH, F::IspSigma),
            Self::SubmarineCable => (F::CableMedianH, F::CableSigma),
            Self::CloudRegion => (F::CloudMedianH, F::CloudSigma),
            Self::DnsBootstrap => (F::DnsMedianH, F::DnsSigma),
            Self::BgpLeak => (F::BgpMedianH, F::BgpSigma),
            Self::CellularOutage => (F::CellMedianH, F::CellSigma),
            Self::NationalShutdown => (F::ShutMedianH, F::ShutSigma),
            Self::RelayWithholding => (F::WithholdMedianH, F::WithholdSigma),
        }
    }
}

/// How the affected units come back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    /// All at the end of the duration.
    Step,
    /// Uniformly over the last `ramp_share` of the duration.
    Linear,
    /// From the ramp start, each unit after an exponential wait with mean a third of the ramp.
    Exponential,
}

/// One drawn correlated event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Scenario.
    pub kind: ScenarioKind,
    /// Seed for per-unit restore times.
    pub seed: u64,
    /// Hours from onset to full recovery (Step) or to the last unit's nominal return.
    pub duration_h: f64,
    /// Hours from onset at which recovery starts.
    pub ramp_start_h: f64,
    /// Access networks down.
    pub asns: Vec<u32>,
    /// Cables cut.
    pub cables: Vec<u32>,
    /// Power regions dark.
    pub power: Vec<u32>,
    /// Countries shut down.
    pub shutdown: Vec<u16>,
    /// Shutdown also cuts domestic cellular.
    pub shutdown_mobile: bool,
    /// Bootstrap seat unresolvable.
    pub bootstrap_lost: bool,
    /// Route-leak victims (access networks).
    pub leak_victims: Vec<u32>,
    /// ASes that accepted the leak.
    pub leak_acceptors: Vec<u32>,
    /// Withholding operators.
    pub withholding: Vec<u32>,
    /// Human-readable scope.
    pub scope: String,
}

impl Event {
    fn empty(kind: ScenarioKind, seed: u64) -> Self {
        Self {
            kind,
            seed,
            duration_h: 0.0,
            ramp_start_h: 0.0,
            asns: Vec::new(),
            cables: Vec::new(),
            power: Vec::new(),
            shutdown: Vec::new(),
            shutdown_mobile: false,
            bootstrap_lost: false,
            leak_victims: Vec::new(),
            leak_acceptors: Vec::new(),
            withholding: Vec::new(),
            scope: String::new(),
        }
    }

    /// An event of `kind` that is down for `duration_h` with a Step return, with no targets
    /// (callers fill them): for named, hand-built scenarios.
    #[must_use]
    pub fn manual(kind: ScenarioKind, seed: u64, duration_h: f64) -> Self {
        let mut e = Self::empty(kind, seed);
        e.duration_h = duration_h;
        e.ramp_start_h = duration_h;
        e
    }

    /// Hour at which unit `u` (a hashed id) returns.
    #[must_use]
    pub fn restore_h(&self, u: u64) -> f64 {
        let x = unit(self.seed, u);
        match self.kind.recovery() {
            Recovery::Step => self.duration_h,
            Recovery::Linear => self.ramp_start_h + (self.duration_h - self.ramp_start_h) * x,
            Recovery::Exponential => {
                let tau = ((self.duration_h - self.ramp_start_h) / 3.0).max(1e-9);
                self.ramp_start_h - tau * (1.0 - x).ln()
            }
        }
    }

    /// Unit `u` is still affected at hour `t`.
    #[must_use]
    pub fn active(&self, u: u64, t: f64) -> bool {
        t >= 0.0 && t < self.restore_h(u)
    }
}

// Unit-id namespaces so restore times of different unit kinds do not coincide.
const U_ASN: u64 = 1 << 40;
const U_CABLE: u64 = 2 << 40;
const U_HOUSE: u64 = 3 << 40;
const U_NODE: u64 = 4 << 40;
const U_COUNTRY: u64 = 5 << 40;
const U_GLOBAL: u64 = 6 << 40;

/// Draw one event of `kind` on `w`.
pub fn sample_event(w: &DomainWorld, p: &FailureParams, kind: ScenarioKind, r: &mut Rng) -> Event {
    let mut e = Event::empty(kind, r.next_u64());
    let (fm, fs) = kind.duration_params();
    e.duration_h = p.get(fm) * (p.get(fs) * r.gauss()).exp();
    let ramp = match kind {
        ScenarioKind::RegionalPower => p.get(F::PowerRamp),
        ScenarioKind::CloudRegion => p.get(F::CloudRamp),
        ScenarioKind::RelayWithholding => 1.0,
        _ => 0.0,
    };
    e.ramp_start_h = e.duration_h * (1.0 - ramp.clamp(0.0, 1.0));
    let load = |a: u32| f64::from(w.asn_load[a as usize]);
    match kind {
        ScenarioKind::RegionalPower => {
            let mut wt = vec![0.0; w.power_regions as usize];
            for n in &w.nodes {
                wt[n.power as usize] += 1.0;
            }
            let reg = pick_weighted(r, &wt) as u32;
            e.power.push(reg);
            e.scope = format!("power region {reg}");
        }
        ScenarioKind::IspOutage | ScenarioKind::CellularOutage => {
            let want = if kind == ScenarioKind::IspOutage { AsnKind::Fixed } else { AsnKind::Cellular };
            let ids: Vec<u32> = (0..w.asns.len() as u32).filter(|&a| w.asns[a as usize].kind == want).collect();
            let wt: Vec<f64> = ids.iter().map(|&a| load(a)).collect();
            let a = ids[pick_weighted(r, &wt)];
            e.asns.push(a);
            e.scope = format!("{} asn {a} (country {})", if want == AsnKind::Fixed { "fixed" } else { "cellular" }, w.asns[a as usize].country);
        }
        ScenarioKind::SubmarineCable => {
            // Cuts cluster on the route into one country (the anchors are a route's cables).
            let c = r.below(w.countries as usize);
            let mut pool = w.country_cables[c].clone();
            let n_cut = p.n(F::CablesCut).max(1);
            while e.cables.len() < n_cut && !pool.is_empty() {
                e.cables.push(pool.swap_remove(r.below(pool.len())));
            }
            e.cables.sort_unstable();
            e.scope = format!("cables {:?} into country {c} (of its {})", e.cables, w.country_cables[c].len());
        }
        ScenarioKind::CloudRegion => {
            let ids: Vec<u32> = (0..w.asns.len() as u32).filter(|&a| w.asns[a as usize].kind == AsnKind::Cloud).collect();
            let wt: Vec<f64> = ids.iter().map(|&a| 1.0 + load(a)).collect();
            let a = ids[pick_weighted(r, &wt)];
            e.asns.push(a);
            e.scope = format!("cloud region asn {a}");
        }
        ScenarioKind::DnsBootstrap => {
            e.bootstrap_lost = true;
            e.scope = "bootstrap seat name".into();
        }
        ScenarioKind::BgpLeak => {
            let mut wt = vec![0.0; w.countries as usize];
            for n in &w.nodes {
                wt[n.country as usize] += 1.0;
            }
            let c = pick_weighted(r, &wt) as u16;
            for (a, asn) in w.asns.iter().enumerate() {
                if asn.country == c && asn.kind != AsnKind::Cloud {
                    e.leak_victims.push(a as u32);
                } else if r.chance(p.get(F::BgpAcceptors)) {
                    e.leak_acceptors.push(a as u32);
                }
            }
            e.scope = format!("prefixes of country {c} leaked to {} ASes", e.leak_acceptors.len());
        }
        ScenarioKind::NationalShutdown => {
            let c = r.below(w.countries as usize) as u16;
            e.shutdown.push(c);
            e.shutdown_mobile = r.chance(p.get(F::ShutMobile));
            e.scope = format!("country {c}");
        }
        ScenarioKind::RelayWithholding => {
            let mut owners: Vec<u32> = w.nodes.iter().filter(|n| n.relay && n.kind != NodeKind::Seat).map(|n| n.person).collect();
            owners.sort_unstable();
            owners.dedup();
            let relays = w.nodes.iter().filter(|n| n.relay && n.kind != NodeKind::Seat).count();
            let want = (relays as f64 * p.get(F::WithholdShare)).ceil() as usize;
            let mut got = 0usize;
            while got < want && !owners.is_empty() {
                let o = owners.swap_remove(r.below(owners.len()));
                got += w.nodes.iter().filter(|n| n.relay && n.person == o && n.kind != NodeKind::Seat).count();
                e.withholding.push(o);
            }
            e.withholding.sort_unstable();
            e.scope = format!("{} operators withholding {got} relays", e.withholding.len());
        }
    }
    e
}

// ------------------------------------------------------------------ the state

/// What is failed, at what hour.
#[derive(Debug, Clone, Default)]
pub struct Condition<'a> {
    /// Correlated events in force.
    pub events: &'a [Event],
    /// Hours since onset.
    pub t_h: f64,
    /// Nodes removed outright (random arm; permanent loss).
    pub removed: Option<&'a [bool]>,
    /// Apply the rejoin rule (a share of nodes must re-establish a session now).
    pub rejoin: bool,
}

/// Who is up and who can reach whom.
#[derive(Debug, Clone)]
pub struct State {
    /// Up (powered, awake, attached to a live access network, not removed, joined).
    pub up: Vec<bool>,
    /// Network component (a vertex id) of an up node.
    pub comp: Vec<u32>,
    /// Route-leak class: 0 neutral, 1 victim, 2 acceptor.
    pub class: Vec<u8>,
    /// Withholding (relays forward nothing, holders serve nothing).
    pub withholding: Vec<bool>,
    /// Per component vertex: a usable relay exists in class 0 / 1 / 2.
    relay_in: Vec<[bool; 3]>,
    /// Up nodes in the largest reachability component.
    pub in_lcc: Vec<bool>,
    /// Size of the largest reachability component.
    pub lcc: usize,
}

struct Uf(Vec<u32>);
impl Uf {
    fn new(n: usize) -> Self {
        Self((0..n as u32).collect())
    }
    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            let p = self.0[self.0[x as usize] as usize];
            self.0[x as usize] = p;
            x = p;
        }
        x
    }
    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.0[hi as usize] = lo;
        }
    }
}

impl State {
    fn relay_ok(&self, comp: u32, k1: u8, k2: u8) -> bool {
        let r = self.relay_in[comp as usize];
        if r[0] {
            return true;
        }
        for k in 1..3u8 {
            if r[k as usize] && (k1 == 0 || k1 == k) && (k2 == 0 || k2 == k) {
                return true;
            }
        }
        false
    }

    fn cell_reach(&self, ca: (u32, u8, bool), cb: (u32, u8, bool)) -> bool {
        ca.0 == cb.0
            && !((ca.1 == 1 && cb.1 == 2) || (ca.1 == 2 && cb.1 == 1))
            && (ca.2 || cb.2 || self.relay_ok(ca.0, ca.1, cb.1))
    }

    /// Two nodes can exchange rows now.
    #[must_use]
    pub fn reach(&self, w: &DomainWorld, a: usize, b: usize) -> bool {
        if a == b {
            return self.up[a];
        }
        self.up[a]
            && self.up[b]
            && self.cell_reach(
                (self.comp[a], self.class[a], w.nodes[a].public),
                (self.comp[b], self.class[b], w.nodes[b].public),
            )
    }

    /// A NAT'd requester in the largest component can fetch from node `i`, and `i` serves.
    #[must_use]
    pub fn serves_mainland(&self, w: &DomainWorld, i: usize) -> bool {
        self.up[i]
            && self.in_lcc[i]
            && !self.withholding[i]
            && (w.nodes[i].public || self.relay_ok(self.comp[i], self.class[i], 0))
    }
}

/// Evaluate who is up and connected under `cond`.
#[must_use]
pub fn evaluate(w: &DomainWorld, p: &FailureParams, cond: &Condition<'_>) -> State {
    let t = cond.t_h;
    let na = w.asns.len();
    let nc = w.countries as usize;
    let mut asn_down = vec![false; na];
    let mut cable_down = vec![false; w.cables.len()];
    let mut power_dark = vec![None::<&Event>; w.power_regions as usize];
    let mut shut = vec![false; nc];
    let mut bootstrap_lost = false;
    let mut victim = vec![false; na];
    let mut acceptor = vec![false; na];
    let mut withholding_p: Vec<u32> = Vec::new();
    for e in cond.events {
        for &a in &e.asns {
            if e.active(U_ASN + u64::from(a), t) {
                asn_down[a as usize] = true;
            }
        }
        for &c in &e.cables {
            if e.active(U_CABLE + u64::from(c), t) {
                cable_down[c as usize] = true;
            }
        }
        for &pr in &e.power {
            power_dark[pr as usize] = Some(e);
        }
        for &c in &e.shutdown {
            if e.active(U_COUNTRY + u64::from(c), t) {
                shut[c as usize] = true;
                if e.shutdown_mobile {
                    for (a, asn) in w.asns.iter().enumerate() {
                        if asn.country == c && asn.kind == AsnKind::Cellular {
                            asn_down[a] = true;
                        }
                    }
                }
            }
        }
        if e.bootstrap_lost && e.active(U_GLOBAL, t) {
            bootstrap_lost = true;
        }
        if e.active(U_GLOBAL + 1, t) {
            for &a in &e.leak_victims {
                victim[a as usize] = true;
            }
            for &a in &e.leak_acceptors {
                acceptor[a as usize] = true;
            }
            for &o in &e.withholding {
                withholding_p.push(o);
            }
        }
    }
    withholding_p.sort_unstable();
    // Network graph: asns, country cores, the global core.
    let g = (na + nc) as u32;
    let mut uf = Uf::new(na + nc + 1);
    for (a, asn) in w.asns.iter().enumerate() {
        if !asn_down[a] {
            uf.union(a as u32, (na + asn.country as usize) as u32);
        }
    }
    for (c, cb) in w.cables.iter().enumerate() {
        if cable_down[c] {
            continue;
        }
        for &l in &cb.landings {
            if !shut[l as usize] {
                uf.union((na + l as usize) as u32, g);
            }
        }
    }
    let tower_ok = t < p.get(F::TowerBackupH);
    let n = w.nodes.len();
    let mut up = vec![false; n];
    let mut comp = vec![u32::MAX; n];
    let mut class = vec![0u8; n];
    let mut attach = vec![u32::MAX; n];
    for (i, nd) in w.nodes.iter().enumerate() {
        if !nd.awake || cond.removed.is_some_and(|m| m[i]) {
            continue;
        }
        let dark = power_dark[nd.power as usize]
            .is_some_and(|e| e.active(U_HOUSE + u64::from(nd.household), t));
        let a = match nd.kind {
            NodeKind::Phone => {
                if nd.on_wifi && !dark && !asn_down[nd.fixed_asn as usize] {
                    nd.fixed_asn
                } else if !asn_down[nd.cell_asn as usize] && (!dark || tower_ok) {
                    nd.cell_asn
                } else {
                    continue;
                }
            }
            NodeKind::HomeNode | NodeKind::Laptop => {
                if (dark && !nd.ups) || asn_down[nd.fixed_asn as usize] {
                    continue;
                }
                nd.fixed_asn
            }
            NodeKind::Hosted | NodeKind::Seat => {
                let dc_fail = power_dark[nd.power as usize].is_some_and(|e| {
                    e.active(U_NODE + i as u64, t) && unit(e.seed ^ 0xDC, i as u64) < p.get(F::DcFail)
                });
                if dc_fail || asn_down[nd.fixed_asn as usize] {
                    continue;
                }
                nd.fixed_asn
            }
        };
        up[i] = true;
        attach[i] = a;
        comp[i] = uf.find(a);
        class[i] = if victim[a as usize] {
            1
        } else if acceptor[a as usize] {
            2
        } else {
            0
        };
    }
    // Rejoin: a share of nodes must re-establish a session; without a cached peer they need
    // the bootstrap seat (name resolves, a seat is up in their component).
    if cond.rejoin {
        let mut seat_in = vec![false; na + nc + 1];
        let mut public_in = vec![0u32; na + nc + 1];
        for (i, nd) in w.nodes.iter().enumerate() {
            if up[i] && nd.kind == NodeKind::Seat {
                seat_in[comp[i] as usize] = true;
            }
            if up[i] && nd.public {
                public_in[comp[i] as usize] += 1;
            }
        }
        let salt = mix(w.seed, 0x5E10);
        for i in 0..n {
            if !up[i] || w.nodes[i].kind == NodeKind::Seat || unit(salt, i as u64) >= p.get(F::RejoinShare) {
                continue;
            }
            let cached = unit(salt ^ 1, i as u64) < p.get(F::PeerCache);
            let others = public_in[comp[i] as usize] - u32::from(w.nodes[i].public);
            let ok = if cached { others > 0 } else { !bootstrap_lost && seat_in[comp[i] as usize] };
            if !ok {
                up[i] = false;
            }
        }
    }
    let withholding: Vec<bool> = w.nodes.iter().map(|nd| withholding_p.binary_search(&nd.person).is_ok()).collect();
    let mut relay_in = vec![[false; 3]; na + nc + 1];
    for (i, nd) in w.nodes.iter().enumerate() {
        if up[i] && nd.relay && !withholding[i] {
            relay_in[comp[i] as usize][class[i] as usize] = true;
        }
    }
    let mut st = State { up, comp, class, withholding, relay_in, in_lcc: vec![false; n], lcc: 0 };
    // Largest reachability component over cells (component, class, public).
    let mut cells: BTreeMap<(u32, u8, bool), u64> = BTreeMap::new();
    for i in 0..n {
        if st.up[i] {
            *cells.entry((st.comp[i], st.class[i], w.nodes[i].public)).or_insert(0) += 1;
        }
    }
    let keys: Vec<(u32, u8, bool)> = cells.keys().copied().collect();
    let mut cu = Uf::new(keys.len());
    let mut linked = vec![false; keys.len()];
    for x in 0..keys.len() {
        for y in (x + 1)..keys.len() {
            if st.cell_reach(keys[x], keys[y]) {
                cu.union(x as u32, y as u32);
                linked[x] = true;
                linked[y] = true;
            }
        }
    }
    let mut size: BTreeMap<u32, u64> = BTreeMap::new();
    let mut best = (0u64, u32::MAX);
    for (x, k) in keys.iter().enumerate() {
        let internal = st.cell_reach(*k, *k);
        let cnt = cells[k];
        if !internal && !linked[x] {
            if best.0 < 1 {
                best = (1, u32::MAX - 1 - x as u32);
            }
            continue;
        }
        let root = cu.find(x as u32);
        let s = size.entry(root).or_insert(0);
        *s += cnt;
        if *s > best.0 {
            best = (*s, root);
        }
    }
    st.lcc = best.0 as usize;
    if best.1 < keys.len() as u32 {
        let lcc_cells: Vec<(u32, u8, bool)> = keys
            .iter()
            .enumerate()
            .filter(|(x, k)| (st.cell_reach(**k, **k) || linked[*x]) && cu.find(*x as u32) == best.1)
            .map(|(_, k)| *k)
            .collect();
        for i in 0..n {
            if st.up[i] && lcc_cells.binary_search(&(st.comp[i], st.class[i], w.nodes[i].public)).is_ok() {
                st.in_lcc[i] = true;
            }
        }
    }
    st
}

/// Reachable unordered pairs among up nodes.
#[must_use]
pub fn reachable_pairs(w: &DomainWorld, st: &State) -> f64 {
    let mut cells: BTreeMap<(u32, u8, bool), f64> = BTreeMap::new();
    for i in 0..w.nodes.len() {
        if st.up[i] {
            *cells.entry((st.comp[i], st.class[i], w.nodes[i].public)).or_insert(0.0) += 1.0;
        }
    }
    let keys: Vec<_> = cells.iter().map(|(k, v)| (*k, *v)).collect();
    let mut total = 0.0;
    for x in 0..keys.len() {
        let (kx, nx) = keys[x];
        if st.cell_reach(kx, kx) {
            total += nx * (nx - 1.0) / 2.0;
        }
        for &(ky, ny) in &keys[(x + 1)..] {
            if st.cell_reach(kx, ky) {
                total += nx * ny;
            }
        }
    }
    total
}

/// Room outcomes: (rooms whole at baseline, of those losing at least one member, of those
/// losing at least half). A member is lost when their phone cannot reach the room's
/// best-connected surviving member set (every pair must reach).
#[must_use]
pub fn room_losses(w: &DomainWorld, base: &State, st: &State) -> (usize, usize, usize) {
    let mut whole = 0;
    let mut any = 0;
    let mut half = 0;
    for room in &w.rooms {
        let phones: Vec<usize> = room.iter().map(|&pp| w.persons[pp as usize].phone as usize).collect();
        let ok_base = phones.iter().all(|&a| phones.iter().all(|&b| base.reach(w, a, b)));
        if !ok_base {
            continue;
        }
        whole += 1;
        // Largest mutually reaching subset, greedily from each member (rooms are <= 8).
        let mut kept = 0;
        for &a in &phones {
            let mut set = vec![a];
            if !st.up[a] {
                continue;
            }
            for &b in &phones {
                if b != a && set.iter().all(|&c| st.reach(w, b, c)) {
                    set.push(b);
                }
            }
            kept = kept.max(set.len());
        }
        let lost = phones.len() - kept;
        if lost > 0 {
            any += 1;
        }
        if lost * 2 >= phones.len() {
            half += 1;
        }
    }
    (whole, any, half)
}

// ------------------------------------------------------------------ distributions

/// p50 / p95 / p99 (and the low tail p01 / p05, for outcomes where low is bad) and mean of a
/// sample (nearest-rank quantiles).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dist {
    /// Samples.
    pub n: usize,
    /// Mean.
    pub mean: f64,
    /// 1st percentile.
    pub p01: f64,
    /// 5th percentile.
    pub p05: f64,
    /// Median.
    pub p50: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
}

impl Dist {
    /// From samples (empty: all zero).
    #[must_use]
    pub fn of(xs: &[f64]) -> Self {
        if xs.is_empty() {
            return Self { n: 0, mean: 0.0, p01: 0.0, p05: 0.0, p50: 0.0, p95: 0.0, p99: 0.0 };
        }
        let mut v = xs.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let q = |f: f64| v[((f * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1];
        Self {
            n: v.len(),
            mean: v.iter().sum::<f64>() / v.len() as f64,
            p01: q(0.01),
            p05: q(0.05),
            p50: q(0.50),
            p95: q(0.95),
            p99: q(0.99),
        }
    }
}

// ------------------------------------------------------------------ the comparison harness

/// One arm's outcomes, each normalised to the same world's baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmOutcome {
    /// Reachable pairs / baseline reachable pairs.
    pub reach_pairs: f64,
    /// Largest component / baseline up nodes.
    pub lcc: f64,
    /// Share of baseline-whole rooms that lose at least one member.
    pub rooms_any: f64,
    /// Share of baseline-whole rooms that lose at least half their members.
    pub rooms_half: f64,
    /// Durability: mean over the drawn publishers of P(the 10 GB object is NOT fully
    /// recoverable now).
    pub not_full_10gb: f64,
    /// Mean per-chunk loss probability.
    pub chunk_loss: f64,
}

/// One seed of a domain-vs-random comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct PairedRun {
    /// The event.
    pub event: Event,
    /// Baseline-up nodes no longer in the largest component.
    pub lost: usize,
    /// Domain arm.
    pub domain: ArmOutcome,
    /// Random arm (the same number of baseline-up nodes removed uniformly).
    pub random: ArmOutcome,
}

/// Durability probe settings used by the harness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Probe {
    /// Placement.
    pub placement: crate::durability::Placement,
    /// Holders per chunk.
    pub holders: u32,
    /// Object bytes.
    pub bytes: u64,
    /// Publishers drawn per seed.
    pub publishers: usize,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            placement: crate::durability::Placement::AudienceLocal { local_share: 0.65 },
            holders: 9,
            bytes: crate::durability::REVIEW_OBJECT_BYTES,
            publishers: 16,
        }
    }
}

/// Outcomes of one arm.
#[must_use]
pub fn arm_outcome(w: &DomainWorld, base: &State, st: &State, probe: &Probe, pub_seed: u64) -> ArmOutcome {
    let bp = reachable_pairs(w, base).max(1.0);
    let bu = base.up.iter().filter(|&&u| u).count().max(1);
    let (whole, any, half) = room_losses(w, base, st);
    let (d, c) = crate::durability::probe_publishers(w, base, st, probe, pub_seed);
    ArmOutcome {
        reach_pairs: reachable_pairs(w, st) / bp,
        lcc: st.lcc as f64 / bu as f64,
        rooms_any: any as f64 / whole.max(1) as f64,
        rooms_half: half as f64 / whole.max(1) as f64,
        not_full_10gb: d,
        chunk_loss: c,
    }
}

/// Run one paired seed: draw an event of `kind`, measure it, remove the same number of
/// baseline-up devices (never a seat) uniformly at random, measure that. Route leaks and
/// withholding remove pairs and service, not nodes, so their random arm removes nothing. Background (sleep, Wi-Fi, rejoiners)
/// is the world's and is identical in both arms.
#[must_use]
pub fn paired_run(w: &DomainWorld, p: &FailureParams, kind: ScenarioKind, seed: u64, probe: &Probe) -> PairedRun {
    let mut r = Rng::new(mix(mix(w.seed, kind as u64), seed));
    let event = sample_event(w, p, kind, &mut r);
    paired_with(w, p, event, seed, probe)
}

/// [`paired_run`] for a given event.
#[must_use]
pub fn paired_with(w: &DomainWorld, p: &FailureParams, event: Event, seed: u64, probe: &Probe) -> PairedRun {
    let base = evaluate(w, p, &Condition { events: &[], t_h: p.get(F::EvalHour), removed: None, rejoin: true });
    let evs = [event];
    let st = evaluate(w, p, &Condition { events: &evs, t_h: p.get(F::EvalHour), removed: None, rejoin: true });
    let [event] = evs;
    let base_up: Vec<usize> = (0..w.nodes.len()).filter(|&i| base.up[i]).collect();
    let lost = base_up.iter().filter(|&&i| !st.in_lcc[i]).count();
    let mut r = Rng::new(mix(mix(w.seed, 0x7A4D), seed));
    // Random arm: devices only. The seats are infrastructure; with one seat (production
    // shape) a uniform draw would sometimes take it and confound device loss with seat loss.
    let mut pool: Vec<usize> = base_up.iter().copied().filter(|&i| w.nodes[i].kind != NodeKind::Seat).collect();
    let mut removed = vec![false; w.nodes.len()];
    for _ in 0..lost.min(pool.len()) {
        let k = r.below(pool.len());
        removed[pool.swap_remove(k)] = true;
    }
    let rs = evaluate(w, p, &Condition { events: &[], t_h: p.get(F::EvalHour), removed: Some(&removed), rejoin: true });
    let pub_seed = mix(seed, 0x9B);
    PairedRun {
        lost,
        domain: arm_outcome(w, &base, &st, probe, pub_seed),
        random: arm_outcome(w, &base, &rs, probe, pub_seed),
        event,
    }
}

/// Distribution over seeds of each outcome, per arm.
#[derive(Debug, Clone, PartialEq)]
pub struct ComparisonReport {
    /// Scenario.
    pub kind: ScenarioKind,
    /// Seeds.
    pub seeds: usize,
    /// Nodes lost (both arms lose this many).
    pub lost: Dist,
    /// Event duration (hours).
    pub duration_h: Dist,
    /// Domain arm: reach pairs, lcc, rooms_any, rooms_half, durable, chunk_loss.
    pub domain: [Dist; 6],
    /// Random arm, same order.
    pub random: [Dist; 6],
}

/// Outcome names, in [`ComparisonReport`] order.
pub const OUTCOMES: [&str; 6] = ["reach_pairs", "lcc", "rooms_any", "rooms_half", "not_full_10gb", "chunk_loss"];

fn arm_vec(o: &ArmOutcome) -> [f64; 6] {
    [o.reach_pairs, o.lcc, o.rooms_any, o.rooms_half, o.not_full_10gb, o.chunk_loss]
}

/// Compare a domain event with random loss over `seeds` seeds.
#[must_use]
pub fn compare(w: &DomainWorld, p: &FailureParams, kind: ScenarioKind, seeds: usize, probe: &Probe) -> ComparisonReport {
    let runs: Vec<PairedRun> = (0..seeds as u64).map(|s| paired_run(w, p, kind, s, probe)).collect();
    let col = |f: &dyn Fn(&PairedRun) -> f64| Dist::of(&runs.iter().map(f).collect::<Vec<_>>());
    let mut domain = [Dist::of(&[]); 6];
    let mut random = [Dist::of(&[]); 6];
    for k in 0..6 {
        domain[k] = col(&|r| arm_vec(&r.domain)[k]);
        random[k] = col(&|r| arm_vec(&r.random)[k]);
    }
    ComparisonReport {
        kind,
        seeds,
        lost: col(&|r| r.lost as f64),
        duration_h: col(&|r| r.event.duration_h),
        domain,
        random,
    }
}

// ------------------------------------------------------------------ the globe view (HOOKS.md 2)

/// Per-node layout of [`view_nodes`]: 6 `u32` per node.
pub const VIEW_NODE_STRIDE: usize = 6;

/// Static per-node labels for the globe, flat: for node `i`, words `6i..6i+6` are
/// `[kind (NodeKind as u32: 0 phone, 1 home node, 2 laptop, 3 hosted, 4 seat), country, metro,
/// power region, access network (normal), public (0/1)]`. Ids are generated indices (no real
/// place); the topology package owns coordinates.
#[must_use]
pub fn view_nodes(w: &DomainWorld) -> Vec<u32> {
    let mut v = Vec::with_capacity(w.nodes.len() * VIEW_NODE_STRIDE);
    for n in &w.nodes {
        v.extend_from_slice(&[n.kind as u32, u32::from(n.country), n.metro, n.power, w.normal_asn(n), u32::from(n.public)]);
    }
    v
}

/// One event's timeline for the globe, flat. `steps` evaluations at hours
/// `t_k = k * horizon_h / (steps - 1)`, k = 0..steps. Output word layout, per step k:
/// `[t_k as f32 bits, up nodes, lcc size]` followed by one word per node:
/// `0` down, `1` up in the largest component, `2` up but cut off from it, `3` up and
/// withholding. Step block length is `3 + nodes`.
#[must_use]
pub fn view_event_steps(w: &DomainWorld, p: &FailureParams, event: &Event, steps: usize, horizon_h: f64) -> Vec<u32> {
    let n = w.nodes.len();
    let steps = steps.max(2);
    let mut v = Vec::with_capacity(steps * (3 + n));
    let evs = std::slice::from_ref(event);
    for k in 0..steps {
        let t = k as f64 * horizon_h / (steps - 1) as f64;
        let st = evaluate(w, p, &Condition { events: evs, t_h: t, removed: None, rejoin: true });
        v.push((t as f32).to_bits());
        v.push(st.up.iter().filter(|&&u| u).count() as u32);
        v.push(st.lcc as u32);
        for i in 0..n {
            v.push(match (st.up[i], st.in_lcc[i], st.withholding[i]) {
                (false, _, _) => 0,
                (true, _, true) => 3,
                (true, true, false) => 1,
                (true, false, false) => 2,
            });
        }
    }
    v
}

/// A small deterministic scenario worth drawing: a 2,000-node world (seed 2026) and a regional
/// power failure over its most populous grid region, with its default recovery ramp, from onset
/// to past full restoration.
#[must_use]
pub fn view_demo() -> (DomainWorld, FailureParams, Event) {
    let mut p = FailureParams::default();
    p.set(F::Nodes, 2_000.0);
    let w = DomainWorld::build(2026, &p);
    let mut wt = vec![0u32; w.power_regions as usize];
    for nd in &w.nodes {
        wt[nd.power as usize] += 1;
    }
    let reg = (0..wt.len()).max_by_key(|&i| (wt[i], usize::MAX - i)).unwrap_or(0) as u32;
    let mut e = Event::empty(ScenarioKind::RegionalPower, 2026);
    e.duration_h = 12.0;
    e.ramp_start_h = 12.0 * (1.0 - p.get(F::PowerRamp));
    e.power.push(reg);
    e.scope = format!("power region {reg}");
    (w, p, e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> (DomainWorld, FailureParams) {
        let mut p = FailureParams::default();
        p.set(F::Nodes, 3_000.0);
        (DomainWorld::build(7, &p), p)
    }

    #[test]
    fn specs_are_tagged_and_unique() {
        assert_eq!(FAILURE_SPECS.len(), FPARAM_COUNT);
        for (i, s) in FAILURE_SPECS.iter().enumerate() {
            assert!(!s.source.is_empty(), "{} has no source", s.name);
            assert!(FAILURE_SPECS[i + 1..].iter().all(|t| t.name != s.name), "duplicate {}", s.name);
            assert!(matches!(s.class, Class::Model | Class::Frontier), "{}: a failure input is never MEASURED here", s.name);
        }
        assert_eq!(FAILURE_SPECS[F::TowerBackupH as usize].class, Class::Frontier);
        assert_eq!(FAILURE_SPECS[F::ReachHome as usize].class, Class::Frontier);
    }

    #[test]
    fn world_and_events_are_deterministic_by_seed() {
        let (w, p) = small();
        let w2 = DomainWorld::build(7, &p);
        assert_eq!(w.nodes, w2.nodes);
        assert_eq!(w.rooms, w2.rooms);
        assert_ne!(DomainWorld::build(8, &p).nodes, w.nodes);
        let probe = Probe { publishers: 4, ..Probe::default() };
        for kind in SCENARIOS {
            let a = paired_run(&w, &p, kind, 3, &probe);
            let b = paired_run(&w2, &p, kind, 3, &probe);
            assert_eq!(a, b, "{}", kind.as_str());
        }
    }

    #[test]
    fn baseline_is_mostly_connected_and_every_scenario_bites() {
        let (w, p) = small();
        let base = evaluate(&w, &p, &Condition { events: &[], t_h: 1.0, removed: None, rejoin: true });
        let up = base.up.iter().filter(|&&u| u).count();
        assert!(up > w.nodes.len() / 2);
        assert_eq!(base.lcc, up, "with nothing failed, every up node is in one component");
        let probe = Probe { publishers: 2, ..Probe::default() };
        for kind in SCENARIOS {
            let lost: usize = (0..8).map(|s| paired_run(&w, &p, kind, s, &probe).lost).sum();
            // Withholding removes no node and, in one component with fungible forwarders, no
            // pair: it bites only as chunks that are not served.
            let worse = (0..8).any(|s| {
                let r = paired_run(&w, &p, kind, s, &probe);
                r.domain.reach_pairs < 1.0 || r.lost > 0 || r.domain.chunk_loss > r.random.chunk_loss
            });
            assert!(worse, "{} never removed a node, a pair or a chunk (lost {lost})", kind.as_str());
        }
    }

    /// A partition keeps the island's internal pairs; removing the same nodes does not
    /// (FSD/twin/failures.md, claim C1).
    #[test]
    fn island_keeps_internal_pairs_that_random_loss_removes() {
        let (w, p) = small();
        let probe = Probe { publishers: 1, ..Probe::default() };
        let mut checked = 0;
        for s in 0..40 {
            let run = paired_run(&w, &p, ScenarioKind::NationalShutdown, s, &probe);
            if run.lost < 50 || run.event.shutdown_mobile {
                continue;
            }
            assert!(
                run.domain.reach_pairs > run.random.reach_pairs,
                "seed {s}: island {} vs random {}",
                run.domain.reach_pairs,
                run.random.reach_pairs
            );
            assert!((run.domain.lcc - run.random.lcc).abs() < 0.02, "same nodes leave the mainland");
            checked += 1;
        }
        let mut p2 = p.clone();
        p2.set(F::ShutMobile, 0.0);
        for s in 0..20 {
            let run = paired_run(&w, &p2, ScenarioKind::NationalShutdown, s, &probe);
            if run.lost >= 50 {
                assert!(run.domain.reach_pairs > run.random.reach_pairs);
                checked += 1;
            }
        }
        assert!(checked >= 5, "too few island seeds ({checked})");
    }

    /// Claim C2 (FSD/twin/failures.md): for an event that takes out a domain's nodes, the mean
    /// per-chunk loss under audience-local placement is at least 100 times that of losing the
    /// same number of devices at random.
    #[test]
    fn domain_loss_costs_chunks_that_random_loss_does_not() {
        let (w, p) = small();
        let probe = Probe { publishers: 24, ..Probe::default() };
        for kind in [ScenarioKind::RegionalPower, ScenarioKind::IspOutage, ScenarioKind::NationalShutdown] {
            let r = compare(&w, &p, kind, 30, &probe);
            let (d, x) = (r.domain[5].mean, r.random[5].mean);
            assert!(d > 0.0 && d >= 100.0 * x, "{}: domain {d:e} vs random {x:e}", kind.as_str());
            assert!(r.domain[4].mean > r.random[4].mean, "{}: 10 GB loss", kind.as_str());
        }
    }

    /// Claim C3: one hour in, a grid or ISP outage loses fewer call members than random loss of
    /// the same count, because phones fall back to cellular on battery; once the cell sites'
    /// batteries are out the grid outage loses more half-rooms than random loss does.
    #[test]
    fn multihomed_phones_hide_an_outage_until_the_towers_fail() {
        let (w, mut p) = small();
        let probe = Probe { publishers: 1, ..Probe::default() };
        for kind in [ScenarioKind::RegionalPower, ScenarioKind::IspOutage] {
            let r = compare(&w, &p, kind, 30, &probe);
            assert!(r.domain[2].mean < r.random[2].mean, "{} hour 1: rooms_any", kind.as_str());
        }
        p.set(F::EvalHour, p.get(F::TowerBackupH) + 1.0);
        p.set(F::PowerMedianH, 24.0);
        let r = compare(&w, &p, ScenarioKind::RegionalPower, 30, &probe);
        assert!(r.domain[3].mean > r.random[3].mean, "after tower backup: rooms_half {} vs {}", r.domain[3].mean, r.random[3].mean);
    }

    #[test]
    fn rejoin_needs_bootstrap_or_cache() {
        let (w, p) = small();
        let mut e = Event::manual(ScenarioKind::DnsBootstrap, 1, 10.0);
        e.bootstrap_lost = true;
        let evs = [e];
        let st = evaluate(&w, &p, &Condition { events: &evs, t_h: 1.0, removed: None, rejoin: true });
        let base = evaluate(&w, &p, &Condition { events: &[], t_h: 1.0, removed: None, rejoin: true });
        let lost = (0..w.nodes.len()).filter(|&i| base.up[i] && !st.up[i]).count();
        let base_up = base.up.iter().filter(|&&u| u).count() as f64;
        let expect = base_up * p.get(F::RejoinShare) * (1.0 - p.get(F::PeerCache));
        assert!((lost as f64 - expect).abs() < 0.25 * expect, "lost {lost}, expected about {expect}");
        let after = evaluate(&w, &p, &Condition { events: &evs, t_h: 11.0, removed: None, rejoin: true });
        assert_eq!(after.up, base.up, "after the event everything rejoins");
    }

    #[test]
    fn view_buffers_have_their_documented_layout() {
        let (w, p, e) = view_demo();
        let nodes = view_nodes(&w);
        assert_eq!(nodes.len(), w.nodes.len() * VIEW_NODE_STRIDE);
        let v = view_event_steps(&w, &p, &e, 5, 14.0);
        let block = 3 + w.nodes.len();
        assert_eq!(v.len(), 5 * block);
        let up = |k: usize| v[k * block + 1];
        let base = evaluate(&w, &p, &Condition { events: &[], t_h: 0.0, removed: None, rejoin: true });
        let base_up = base.up.iter().filter(|&&u| u).count() as u32;
        assert!(up(0) < base_up, "the region is dark at onset");
        assert_eq!(up(4), base_up, "everything is back after the duration");
        assert_eq!(v, view_event_steps(&w, &p, &e, 5, 14.0));
    }

    #[test]
    fn dist_quantiles() {
        let xs: Vec<f64> = (1..=100).map(f64::from).collect();
        let d = Dist::of(&xs);
        assert_eq!((d.p01, d.p05, d.p50, d.p95, d.p99), (1.0, 5.0, 50.0, 95.0, 99.0));
        assert!((d.mean - 50.5).abs() < 1e-12);
    }
}
