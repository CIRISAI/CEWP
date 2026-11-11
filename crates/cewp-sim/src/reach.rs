//! Joint reachability model: who can reach whom without a forwarder (twin package `nat`).
//!
//! The micro tier (`micro.rs`) draws reachability as four independent shares: a home router is
//! public (`reach_home_share` × `public_listener_share` = 12 %), punchable (28 %) or hard (60 %);
//! a phone on cellular is punchable with `cellular_punchable_share` (15 %); two punchable ends
//! punch with `punch_success_share` (90 %). Every home and every carrier draws on its own.
//!
//! Here the same marginals come out of a structural model in which correlation is built in:
//!
//! 1. **Access networks first.** An access network (a fixed ISP or a cellular carrier) draws its
//!    behaviour once: carrier-grade NAT or not and the CGN's RFC 4787 mapping, filtering and
//!    hairpinning; the router the ISP ships (mapping, filtering, hairpinning, UPnP); IPv6
//!    deployment and the IPv6 firewall policy. Every home on that ISP and every phone on that
//!    carrier inherits it, so a street served by one ISP is correlated by construction.
//! 2. **Then the home.** A home may run its own router instead of the ISP's, sit behind two
//!    NATs, forward a port by hand, or leave IPv6 off.
//! 3. **Then the pair.** [`direct_path`] takes both endpoints (and whether a facilitator is
//!    reachable and the pair may propose) and returns the mechanism and the probability of a
//!    direct path. The punching rules are derived from the mapping/filtering combination and
//!    from how edge and leviculum actually traverse NAT (each rule cites its source below).
//!
//! The defaults are calibrated (closed form or bisection, [`ReachModel::new`]) so that the
//! per-endpoint class shares equal the micro tier's defaults exactly: existing results stay
//! comparable, and what changes is only what correlation and the pair rules add.
//!
//! Every number carries its class ([`Class`]). Shares set from general knowledge of published
//! NAT measurement work are `MODEL` and name the kind of source; nothing here is a measurement
//! of the CIRIS fleet, and no dataset was downloaded. `FSD/twin/nat.md` lists the aggregate
//! counters production would need to report to turn these into `MEASURED` values.
//!
//! **Mechanisms edge has (edge 8e46165, leviculum v0.34.0+ciris.3):**
//! - *LAN*: two devices of one household on its network (AutoInterface; `micro.rs`).
//! - *Public listener*: a node whose mode binds a listener (`NodeMode::binds_listener`,
//!   `src/edge.rs`) behind a public IPv4 address or a forwarded port is dialled; the link is one
//!   hop (`link_is_forwarded`, `src/transport/reticulum.rs:534`). The documented default
//!   `listen_addr` is `0.0.0.0:4242` (`src/ffi/pyo3.rs:7002`): IPv4, so an open IPv6 firewall
//!   alone is not a listener ([`R::Ipv6Listener`] keeps the counterfactual).
//! - *Hole punch*: both ends learn their reflexive address from one facilitator over one UDP
//!   socket, swap them on the relayed link, and punch (`DirectLinksConfig::leviculum_settings`,
//!   `ProbeProtocol::Rnsp`, `src/transport/direct_path.rs`; leviculum `direct_link::session`).
//!   Proposing needs a facilitator (`DirectLinksConfig::proposes`) and the peer accepts only a
//!   room peer or consented contact (`accepts_direct`, `DirectAccept::RoomPeers`).
//! - *Peer-reflexive punch*: leviculum acks every punch frame to the address it came from and
//!   punches that address too (`leviculum-std/src/direct_link.rs` `punch`, `PEER_REFLEXIVE_MAX`),
//!   so an endpoint-independent-mapping end whose filter admits the peer's new port learns it.
//! - *IPv6 punch*: the session socket is dual-stack (`bind_session_socket`) and the peer's global
//!   IPv6 host candidate is always punched (`same_nat_alternates`, `host_candidates`); a stateful
//!   IPv6 firewall opens on the simultaneous send.
//! - *Relay*: anything else crosses a forwarder.
//!
//! **Not in edge:** no UPnP, PCP or NAT-PMP client exists in edge `src/transport` or leviculum
//! `src` at those pins (`git grep`, no hits; `examples/scale_model.rs` cites leviculum#101), so
//! [`Mechanism::PortMapped`] is a counterfactual behind [`R::PortMappingClient`] (default 0).
//! Edge's word "one-sided punch" names a FAILURE (one side moved the link, leviculum#89, the
//! 2026-10-31 incident); the two-sided commit now turns it into a failed attempt, which sits
//! inside [`R::PunchSuccessShare`]. It is not a mechanism here.


pub mod counterfactual;
pub mod hooks;
pub mod topo;

use crate::micro::Rng;

// ---------------------------------------------------------------- classes and parameters

/// Epistemic class of a number (FSD §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Read from the field.
    Measured,
    /// Computed from sources or from other numbers by a stated rule.
    Derived,
    /// A stated modelling choice.
    Model,
    /// Extrapolated forward.
    Projected,
    /// An open empirical question: a placeholder until field data exists.
    Frontier,
}

impl Class {
    /// The class as written in the FSD.
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

/// One parameter of the reachability model.
#[derive(Debug, Clone, Copy)]
pub struct ReachParamSpec {
    /// Stable name.
    pub name: &'static str,
    /// Default value.
    pub default: f64,
    /// Class.
    pub class: Class,
    /// Where the default comes from, or why it is a placeholder.
    pub source: &'static str,
}

macro_rules! reach_params {
    ($( $idx:ident = $name:literal, $def:expr, $class:ident, $src:literal; )*) => {
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        /// Index of each reachability parameter in [`ReachParams::values`].
        pub enum R { $( $idx, )* }
        /// Every reachability parameter, in index order.
        pub const REACH_PARAM_SPECS: &[ReachParamSpec] = &[
            $( ReachParamSpec { name: $name, default: $def, class: Class::$class, source: $src }, )*
        ];
    };
}

reach_params! {
    ReachHomeShare = "reach_home_share", 0.40, Frontier, "micro P0 / FSD 4.4 Expected regime: home routers that are public or punchable over IPv4. Calibration target";
    PublicListenerShare = "public_listener_share", 0.30, Model, "micro P1: of reachable home routers, the share with an inbound IPv4 listener. Calibration target";
    CellularPunchableShare = "cellular_punchable_share", 0.15, Frontier, "micro P2: cellular attachments whose NAT maps endpoint-independently. Calibration target";
    PunchSuccessShare = "punch_success_share", 0.90, Frontier, "micro P3: per-attempt success of a punch the NAT pair allows (loss, timing, the 10 s PUNCH_WINDOW, the two-sided commit of leviculum#89); no fleet number";
    FixedCableShare = "fixed_cable_share", 0.30, Model, "home-weighted share of fixed access on cable; kind of source: national / OECD broadband subscription statistics by technology (varies widely by country)";
    FixedFibreShare = "fixed_fibre_share", 0.40, Model, "home-weighted share on fibre (DSL takes the rest); same kind of source";
    Cell5gShare = "cell_5g_share", 0.40, Model, "share of cellular attachments on 5G (rest 4G); kind of source: operator and industry subscription reports";
    RegionV4ScarceShare = "region_v4_scarce_share", 0.50, Model, "home-weighted share in regions whose ISPs got little IPv4 before registry exhaustion; reasoning, no dataset";
    FixedCgnatRich = "fixed_cgnat_rich", 0.10, Model, "fixed-line CGN share where IPv4 is plentiful; kind of source: academic CGN measurement studies (P2P DHT and Netalyzr-style vantage points) found fixed-line CGN in a minority of networks, growing";
    FixedCgnatScarce = "fixed_cgnat_scarce", 0.40, Model, "fixed-line CGN share where IPv4 is scarce; same kind of source, which found CGN far more common in late-allocation regions";
    CableCgnatMult = "cable_cgnat_mult", 0.5, Model, "cable incumbents hold older, larger IPv4 allocations (reasoning, not a measurement)";
    FibreCgnatMult = "fibre_cgnat_mult", 1.5, Model, "new fibre entrants hold little IPv4 (reasoning, not a measurement)";
    CgnEimShare = "cgn_eim_share", 0.70, Model, "fixed-line CGNs with endpoint-independent mapping: RFC 6888 REQ-1 requires RFC 4787 REQ-1; CGN measurement studies found most but not all comply";
    CgnFilterEif = "cgn_filter_eif", 0.10, Model, "CGN filtering endpoint-independent (RFC 4787 sec. 5)";
    CgnFilterAdf = "cgn_filter_adf", 0.30, Model, "CGN filtering address-dependent (the rest address-and-port-dependent)";
    CgnHairpinShare = "cgn_hairpin_share", 0.70, Model, "CGNs that hairpin (RFC 6888 REQ-9 requires it); compliance incomplete, no number";
    RouterFilterEif = "router_filter_eif", 0.15, Model, "home-router filtering endpoint-independent; kind of source: NAT-type classification in P2P / VoIP / game-console STUN surveys, where port-restricted cone is the most common";
    RouterFilterAdf = "router_filter_adf", 0.30, Model, "home-router filtering address-dependent (rest address-and-port-dependent); same kind of source";
    RouterHairpinShare = "router_hairpin_share", 0.60, Model, "home routers that hairpin (leviculum session.rs: 'many home routers do not')";
    RouterUpnpShare = "router_upnp_share", 0.60, Model, "home routers with UPnP-IGD or PCP on (moot while edge has no client)";
    OwnRouterShare = "own_router_share", 0.30, Model, "homes running their own router instead of the ISP's; reasoning";
    DoubleNatShare = "double_nat_share", 0.10, Model, "homes behind two NATs (ISP gateway in router mode plus their own); kind of source: home-network measurement (Netalyzr-era) reported a minority";
    FixedV6Cable = "fixed_v6_cable", 0.65, Model, "cable ISPs with IPv6 deployed; kind of source: public IPv6 adoption dashboards (search-engine and RIR measurement) put user IPv6 capability near 40-50 % globally, uneven by network";
    FixedV6Fibre = "fixed_v6_fibre", 0.55, Model, "fibre ISPs with IPv6 deployed; same kind of source";
    FixedV6Dsl = "fixed_v6_dsl", 0.30, Model, "DSL ISPs with IPv6 deployed; same kind of source";
    HomeV6EnabledShare = "home_v6_enabled_share", 0.85, Model, "on a v6 ISP, homes whose router hands out global IPv6; reasoning";
    FixedV6FwOpen = "fixed_v6_fw_open", 0.05, Frontier, "home IPv6 with no inbound firewall; RFC 6092 recommends a stateful default; no number";
    FixedV6FwBlocked = "fixed_v6_fw_blocked", 0.10, Frontier, "home IPv6 firewall that defeats a simultaneous UDP send; no number";
    CellCgnatShare = "cell_cgnat_share", 0.95, Model, "cellular attachments behind CGN; kind of source: CGN measurement studies found CGN in nearly every cellular network";
    CellFilterEif = "cell_filter_eif", 0.05, Model, "carrier CGN filtering endpoint-independent; reasoning";
    CellFilterAdf = "cell_filter_adf", 0.15, Model, "carrier CGN filtering address-dependent (rest address-and-port-dependent); reasoning";
    CellHairpinShare = "cell_hairpin_share", 0.50, Frontier, "carrier CGNs that hairpin between two subscribers; no number";
    CellV6_4g = "cell_v6_4g", 0.55, Model, "4G carriers with IPv6 deployed; kind of source: operator IPv6 deployment statistics, where large mobile networks are among the most IPv6-heavy";
    CellV6_5g = "cell_v6_5g", 0.75, Model, "5G carriers with IPv6 deployed; same kind of source";
    CellV6FwBlocked = "cell_v6_fw_blocked", 0.30, Frontier, "carriers whose IPv6 firewall drops subscriber-to-subscriber UDP even on a simultaneous send; no number";
    FamilySameCarrierShare = "family_same_carrier_share", 0.80, Model, "a household's phones on one carrier (family plans); reasoning";
    PhoneWifiShare = "phone_wifi_share", 0.65, Model, "micro P7: phone on Wi-Fi vs cellular";
    PortMappingClient = "port_mapping_client", 0.0, Derived, "0: no UPnP / PCP / NAT-PMP client in edge 8e46165 src/transport or leviculum v0.34.0+ciris.3 src (git grep: absent; leviculum#101). 1 = counterfactual";
    Ipv6Candidates = "ipv6_candidates", 1.0, Derived, "1: leviculum same_nat_alternates always punches the peer's global IPv6 host candidate on the dual-stack session socket (bind_session_socket, host_candidates). 0 = IPv4 only";
    PeerReflexive = "peer_reflexive", 1.0, Derived, "1: leviculum direct_link::punch acks and punches the address a punch came from (PEER_REFLEXIVE_MAX = 4). 0 = advertised addresses only";
    Ipv6Listener = "ipv6_listener", 0.0, Derived, "0: edge's documented listen_addr default is 0.0.0.0:4242 (src/ffi/pyo3.rs:7002), IPv4 only. 1 = counterfactual: a home with no IPv6 firewall is dialled over IPv6";
    PortMappingPunch = "port_mapping_punch", 0.0, Derived, "with port_mapping_client = 1 only: 1 = every device behind a mapping-capable router also maps the socket it punches from, so its punch socket is endpoint-independent (EIM, EIF); 0 = the client maps the listener only. Counterfactual, used to split the gain";
}

/// Number of reachability parameters.
pub const REACH_PARAM_COUNT: usize = REACH_PARAM_SPECS.len();

/// The parameters of one reachability model.
#[derive(Debug, Clone, PartialEq)]
pub struct ReachParams {
    /// Values in [`R`] order.
    pub values: [f64; REACH_PARAM_COUNT],
}

impl Default for ReachParams {
    fn default() -> Self {
        let mut values = [0.0; REACH_PARAM_COUNT];
        for (v, s) in values.iter_mut().zip(REACH_PARAM_SPECS) {
            *v = s.default;
        }
        Self { values }
    }
}

impl ReachParams {
    /// Read one parameter.
    #[must_use]
    pub fn get(&self, p: R) -> f64 {
        self.values[p as usize]
    }
    /// Set one parameter.
    pub fn set(&mut self, p: R, v: f64) {
        self.values[p as usize] = v;
    }
    /// A share, clamped to [0, 1].
    #[must_use]
    pub fn share(&self, p: R) -> f64 {
        self.get(p).clamp(0.0, 1.0)
    }
    fn on(&self, p: R) -> bool {
        self.get(p) >= 0.5
    }

    /// The parameters of one FSD §4.4 regime. Only `reach_home_share` is the FSD's; how the other
    /// shares follow it is this model's choice (MODEL): cellular punchability and IPv6 deployment
    /// scale with reach / 0.40, and the adversarial regime ("100 % hard CGNAT on homes/phones")
    /// puts every home and phone behind a CGN with no IPv6.
    #[must_use]
    pub fn for_regime(regime: crate::workload::ParameterRegime) -> Self {
        use crate::workload::ParameterRegime as G;
        let mut p = Self::default();
        let reach = crate::workload::RegimeParameters::for_regime(regime).reach_home_share;
        p.set(R::ReachHomeShare, reach);
        let k = reach / 0.40;
        let base = Self::default();
        p.set(R::CellularPunchableShare, (base.get(R::CellularPunchableShare) * k).min(1.0));
        for v6 in [R::FixedV6Cable, R::FixedV6Fibre, R::FixedV6Dsl, R::CellV6_4g, R::CellV6_5g] {
            p.set(v6, (base.get(v6) * k).min(0.95));
        }
        if regime == G::Adversarial {
            p.set(R::FixedCgnatRich, 1.0);
            p.set(R::FixedCgnatScarce, 1.0);
            p.set(R::CellCgnatShare, 1.0);
        }
        p
    }
}

// ---------------------------------------------------------------- network and endpoint types

/// Access technology of an access network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum AccessTech {
    /// DOCSIS cable.
    Cable = 0,
    /// Fibre to the home.
    Fibre = 1,
    /// DSL.
    Dsl = 2,
    /// 4G cellular.
    Cell4g = 3,
    /// 5G cellular.
    Cell5g = 4,
}

impl AccessTech {
    /// Every technology, in order.
    pub const ALL: [Self; 5] = [Self::Cable, Self::Fibre, Self::Dsl, Self::Cell4g, Self::Cell5g];
    /// Cellular (a carrier) rather than fixed (an ISP).
    #[must_use]
    pub const fn is_cellular(self) -> bool {
        matches!(self, Self::Cell4g | Self::Cell5g)
    }
    /// Short name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cable => "cable",
            Self::Fibre => "fibre",
            Self::Dsl => "dsl",
            Self::Cell4g => "4g",
            Self::Cell5g => "5g",
        }
    }
}

/// Coarse region class: how much IPv4 its networks hold (MODEL).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RegionClass {
    /// IPv4 plentiful (early allocation).
    V4Rich = 0,
    /// IPv4 scarce (late allocation).
    V4Scarce = 1,
}

/// An access network: the label the `topology` package assigns per node (its access AS). Local
/// stand-in until that package's type lands; only `id` must be stable, since it seeds the
/// network's behaviour draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessNet {
    /// Stable network index (seeds the per-network draw).
    pub id: u32,
    /// Its access technology.
    pub tech: AccessTech,
    /// Its region class.
    pub region: RegionClass,
}

/// RFC 4787 section 4.1 mapping behaviour, ordered from least to most restrictive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Mapping {
    /// Endpoint-independent mapping.
    Eim = 0,
    /// Address-dependent mapping.
    Adm = 1,
    /// Address-and-port-dependent mapping ("symmetric").
    Apdm = 2,
}

/// RFC 4787 section 5 filtering behaviour, ordered from least to most restrictive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Filtering {
    /// Endpoint-independent filtering.
    Eif = 0,
    /// Address-dependent filtering.
    Adf = 1,
    /// Address-and-port-dependent filtering.
    Apdf = 2,
}

/// IPv6 inbound policy in front of a host with a global IPv6 address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum V6Fw {
    /// No inbound firewall.
    Open = 0,
    /// Stateful: an outbound send opens the return path (a simultaneous send punches it).
    Stateful = 1,
    /// Defeats even a simultaneous send.
    Blocked = 2,
}

/// One NAT layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NatBox {
    /// Mapping.
    pub mapping: Mapping,
    /// Filtering.
    pub filtering: Filtering,
    /// Hairpins.
    pub hairpin: bool,
}

/// What one access network does, drawn once from its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetBehaviour {
    /// The network.
    pub net: AccessNet,
    /// The carrier-grade NAT in front of every subscriber, if any.
    pub cgn: Option<NatBox>,
    /// The router this ISP ships (fixed networks; a carrier has none).
    pub cpe: Option<NatBox>,
    /// The ISP router has UPnP / PCP on.
    pub cpe_upnp: bool,
    /// IPv6 deployed to subscribers.
    pub v6: bool,
    /// IPv6 firewall of the ISP router or the carrier.
    pub v6_fw: V6Fw,
}

/// The coarse reachability class the micro tier uses (IPv4 only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ReachClass {
    /// Inbound IPv4 listener (public address or forwarded port).
    Public = 0,
    /// Endpoint-independent mapping end to end: its reflexive address is the one peers reach.
    Punchable = 1,
    /// Neither.
    Hard = 2,
}

impl ReachClass {
    /// Every class, in order.
    pub const ALL: [Self; 3] = [Self::Public, Self::Punchable, Self::Hard];
    /// Short name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Punchable => "punchable",
            Self::Hard => "hard",
        }
    }
}

/// One endpoint as the outside sees it: a home router (and every device on its network) or a
/// phone on a carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    /// Its access network.
    pub net: AccessNet,
    /// Behind the network's CGN.
    pub cgnat: bool,
    /// Behind two home NATs.
    pub double_nat: bool,
    /// The NAT stack composed (most restrictive layer wins), or `None` with no NAT at all.
    pub nat: Option<NatBox>,
    /// The CGN hairpins (only meaningful with `cgnat`).
    pub cgn_hairpin: bool,
    /// A hand-forwarded port to a listener on a public IPv4 address.
    pub port_forward: bool,
    /// Its router would map a port on request (UPnP / PCP) to a public IPv4 address.
    pub port_map_capable: bool,
    /// Global IPv6 and the firewall in front of it.
    pub v6: Option<V6Fw>,
    /// Dials out only (every v44/v45 phone: `DeviceState::of(Phone)`).
    pub client_only: bool,
}

impl Endpoint {
    /// Mapping of the composed NAT stack (`Eim` with no NAT).
    #[must_use]
    pub fn mapping(&self) -> Mapping {
        self.nat.map_or(Mapping::Eim, |n| n.mapping)
    }
    /// Filtering of the composed NAT stack (`Eif` with no NAT).
    #[must_use]
    pub fn filtering(&self) -> Filtering {
        self.nat.map_or(Filtering::Eif, |n| n.filtering)
    }
    /// The micro tier's class (IPv4 only).
    #[must_use]
    pub fn class(&self) -> ReachClass {
        if self.port_forward && !self.client_only {
            ReachClass::Public
        } else if self.mapping() == Mapping::Eim {
            ReachClass::Punchable
        } else {
            ReachClass::Hard
        }
    }
}

/// Compose two NAT layers: the more restrictive mapping and filtering win (RFC 4787 behaviour of a
/// NAT behind a NAT, as seen from outside); hairpinning is the outer layer's.
#[must_use]
pub fn compose(inner: NatBox, outer: NatBox) -> NatBox {
    NatBox { mapping: inner.mapping.max(outer.mapping), filtering: inner.filtering.max(outer.filtering), hairpin: outer.hairpin }
}

// ---------------------------------------------------------------- pair rule

/// How a pair gets a direct path, or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Mechanism {
    /// Same household network (AutoInterface).
    Lan = 0,
    /// One end dials the other's public IPv4 listener (or forwarded port).
    PublicListener = 1,
    /// One end's router mapped a port on request (counterfactual: edge has no client).
    PortMapped = 2,
    /// Both global IPv6 behind stateful firewalls: the simultaneous send opens both.
    Ipv6Punch = 3,
    /// Both IPv4 NATs map endpoint-independently: reflexive addresses punch.
    HolePunch = 4,
    /// One end maps endpoint-independently and its filter admits the peer's new port, which it
    /// learns from the peer's punch (peer-reflexive candidate).
    PeerReflexivePunch = 5,
    /// No direct path: a forwarder carries the pair.
    Relay = 6,
}

impl Mechanism {
    /// Every mechanism, in order.
    pub const ALL: [Self; 7] =
        [Self::Lan, Self::PublicListener, Self::PortMapped, Self::Ipv6Punch, Self::HolePunch, Self::PeerReflexivePunch, Self::Relay];
    /// Short name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lan => "lan",
            Self::PublicListener => "public listener",
            Self::PortMapped => "port mapped",
            Self::Ipv6Punch => "ipv6 punch",
            Self::HolePunch => "hole punch",
            Self::PeerReflexivePunch => "peer-reflexive punch",
            Self::Relay => "relay",
        }
    }
    /// Needs a facilitated punch (and so a facilitator and a room-peer accept).
    #[must_use]
    pub const fn is_punch(self) -> bool {
        matches!(self, Self::Ipv6Punch | Self::HolePunch | Self::PeerReflexivePunch)
    }
}

/// What the pair has besides its two endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCtx {
    /// Both on one household network.
    pub same_lan: bool,
    /// A facilitator answers both, both run edge v44+, and the pair may propose (room peers or a
    /// consented contact: `accepts_direct`).
    pub may_punch: bool,
}

impl PairCtx {
    /// Room peers in different households, both on v44+, facilitator up.
    pub const ROOM_PEERS: Self = Self { same_lan: false, may_punch: true };
}

/// A pair's direct path: the mechanism the pair would use and the probability it comes up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direct {
    /// Mechanism (`Relay` when none).
    pub mechanism: Mechanism,
    /// Probability the direct path comes up (0 for `Relay`).
    pub p: f64,
}

/// Whether two endpoints share one CGN (same network, both behind it): a reflexive path between
/// them turns at the CGN and needs it to hairpin.
fn same_cgn(a: &Endpoint, b: &Endpoint) -> bool {
    a.cgnat && b.cgnat && a.net.id == b.net.id
}

/// Whether an IPv4 punch works between the pair's NATs, and by which mechanism. Derivation
/// (leviculum `direct_link::punch`, reflexive address from one facilitator over one socket):
/// - an EIM end's packets to the peer leave from its advertised (reflexive) address; a non-EIM
///   end's leave from a new mapping the peer was never told;
/// - both EIM: each end sends to the other's advertised address, which opens its own filter for
///   exactly that address, so repeated punch frames (every 100 ms for 10 s) pass any filtering;
/// - one EIM end E, one non-EIM end S: S's frames reach E's advertised address from a new port.
///   E admits them under EIF, and under ADF because E already sent to S's address (RFC 4787 IP
///   pooling "paired"); under APDF they are dropped. Once admitted, E acks and punches the new
///   port (peer-reflexive), which S's mapping toward E admits; both ends then hold an address
///   acked and heard (the two-sided check), so the path forms;
/// - neither EIM: neither end ever learns an address the other sends from. Never.
///   (No port prediction: leviculum punches only advertised and peer-reflexive addresses.)
fn v4_punch(a: &Endpoint, b: &Endpoint, p: &ReachParams) -> Option<Mechanism> {
    if same_cgn(a, b) && !(a.cgn_hairpin && b.cgn_hairpin) {
        return None;
    }
    // counterfactual: a port-mapping client that maps its punch socket makes that socket
    // endpoint-independent in mapping and filtering (a mapped port admits any source)
    let mapped = |e: &Endpoint| p.on(R::PortMappingClient) && p.on(R::PortMappingPunch) && e.port_map_capable;
    let (ea, eb) = (mapped(a) || a.mapping() == Mapping::Eim, mapped(b) || b.mapping() == Mapping::Eim);
    if ea && eb {
        return Some(Mechanism::HolePunch);
    }
    if !p.on(R::PeerReflexive) {
        return None;
    }
    let admits = |e: &Endpoint| mapped(e) || e.filtering() != Filtering::Apdf;
    if (ea && admits(a)) || (eb && admits(b)) {
        return Some(Mechanism::PeerReflexivePunch);
    }
    None
}

/// The pair rule: mechanism and probability of a direct path between `a` and `b`.
#[must_use]
pub fn direct_path(a: &Endpoint, b: &Endpoint, ctx: PairCtx, p: &ReachParams) -> Direct {
    let sure = |mechanism| Direct { mechanism, p: 1.0 };
    if ctx.same_lan {
        return sure(Mechanism::Lan);
    }
    // a node that binds a listener behind a public IPv4 address / forwarded port is dialled by
    // anyone (edge `NodeMode::binds_listener`; one hop: `link_is_forwarded`)
    if (a.port_forward && !a.client_only) || (b.port_forward && !b.client_only) {
        return sure(Mechanism::PublicListener);
    }
    if p.on(R::Ipv6Listener) {
        let open = |x: &Endpoint, y: &Endpoint| x.v6 == Some(V6Fw::Open) && !x.client_only && y.v6.is_some();
        if open(a, b) || open(b, a) {
            return sure(Mechanism::PublicListener);
        }
    }
    if p.on(R::PortMappingClient) && ((a.port_map_capable && !a.client_only) || (b.port_map_capable && !b.client_only)) {
        return sure(Mechanism::PortMapped);
    }
    if !ctx.may_punch {
        return Direct { mechanism: Mechanism::Relay, p: 0.0 };
    }
    let ok = p.share(R::PunchSuccessShare);
    // one session punches the IPv4 reflexive address and the IPv6 host candidate at once; the
    // first proven both ways is the path (leviculum `punch`)
    let v6 = p.on(R::Ipv6Candidates)
        && matches!(a.v6, Some(V6Fw::Open | V6Fw::Stateful))
        && matches!(b.v6, Some(V6Fw::Open | V6Fw::Stateful));
    if v6 {
        return Direct { mechanism: Mechanism::Ipv6Punch, p: ok };
    }
    match v4_punch(a, b, p) {
        Some(m) => Direct { mechanism: m, p: ok },
        None => Direct { mechanism: Mechanism::Relay, p: 0.0 },
    }
}

// ---------------------------------------------------------------- the model

/// Shares solved so the class marginals equal the targets (all DERIVED).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    /// Homes on a public IPv4 address (no CGN, no double NAT) that forward a port by hand.
    pub port_forward_share: f64,
    /// Home routers (the ISP's and people's own) with endpoint-independent mapping.
    pub router_eim_share: f64,
    /// Carrier CGNs with endpoint-independent mapping.
    pub cell_cgn_eim_share: f64,
}

/// A calibrated model: parameters plus the solved shares.
#[derive(Debug, Clone, PartialEq)]
pub struct ReachModel {
    /// Parameters.
    pub params: ReachParams,
    /// Solved shares.
    pub cal: Calibration,
}

fn mix(seed: u64, a: u64, b: u64) -> u64 {
    seed ^ a.wrapping_mul(0x9E6C_63D0_676A_9A99) ^ b.wrapping_mul(0xD1B5_4A32_D192_ED03).rotate_left(17)
}

fn pick_filter(r: &mut Rng, eif: f64, adf: f64) -> Filtering {
    let u = r.f();
    if u < eif {
        Filtering::Eif
    } else if u < eif + adf {
        Filtering::Adf
    } else {
        Filtering::Apdf
    }
}

/// Non-EIM mapping is address-dependent or address-and-port-dependent; the pair rule treats both
/// alike (MODEL split, 30 / 70, for the record only).
fn pick_mapping(r: &mut Rng, eim: f64) -> Mapping {
    let u = r.f();
    if u < eim {
        Mapping::Eim
    } else if u < eim + (1.0 - eim) * 0.3 {
        Mapping::Adm
    } else {
        Mapping::Apdm
    }
}

impl ReachModel {
    /// Calibrate `params`: solve the port-forward share, the router EIM share and the carrier CGN
    /// EIM share so the class marginals equal `reach_home_share`, `public_listener_share` and
    /// `cellular_punchable_share`. Fails when no share in [0, 1] reaches a target.
    pub fn new(params: ReachParams) -> Result<Self, String> {
        let p = &params;
        let public = p.share(R::ReachHomeShare) * p.share(R::PublicListenerShare);
        let punchable = p.share(R::ReachHomeShare) - public;
        let d = p.share(R::DoubleNatShare);
        let c = Self::fixed_cgnat_mean(p);
        let open = (1.0 - c) * (1.0 - d);
        if public > open + 1e-12 {
            return Err(format!("public share {public:.3} exceeds the homes with no CGN and no double NAT ({open:.3})"));
        }
        let pf = if open > 0.0 { (public / open).min(1.0) } else { 0.0 };
        let cgn_eim = p.share(R::CgnEimShare);
        // P(punchable) = sum over (cgn, double) of P(cgn) P(double) (1 - pf [no cgn, no double])
        //   s (cgn ? cgn_eim : 1) (double ? s : 1), increasing in s
        let punch_at = |s: f64| {
            let mut t = 0.0;
            for (pc, cg) in [(1.0 - c, false), (c, true)] {
                for (pd, dn) in [(1.0 - d, false), (d, true)] {
                    let not_public = if !cg && !dn { 1.0 - pf } else { 1.0 };
                    t += pc * pd * not_public * s * if cg { cgn_eim } else { 1.0 } * if dn { s } else { 1.0 };
                }
            }
            t
        };
        if punchable > punch_at(1.0) + 1e-12 {
            return Err(format!("punchable share {punchable:.3} exceeds what every router EIM gives ({:.3})", punch_at(1.0)));
        }
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if punch_at(mid) < punchable {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let s = 0.5 * (lo + hi);
        let cc = p.share(R::CellCgnatShare);
        let target = p.share(R::CellularPunchableShare);
        let e = if cc > 0.0 { (target - (1.0 - cc)) / cc } else { 0.0 };
        if !(-1e-12..=1.0 + 1e-12).contains(&e) || (cc == 0.0 && (target - 1.0).abs() > 1e-12) {
            return Err(format!("cellular punchable {target:.3} unreachable with {cc:.3} of carriers behind CGN"));
        }
        Ok(Self { params, cal: Calibration { port_forward_share: pf, router_eim_share: s, cell_cgn_eim_share: e.clamp(0.0, 1.0) } })
    }

    /// The default model (calibrated defaults).
    #[must_use]
    pub fn default_model() -> Self {
        Self::new(ReachParams::default()).expect("defaults calibrate")
    }

    fn tech_cgnat_mult(p: &ReachParams, t: AccessTech) -> f64 {
        match t {
            AccessTech::Cable => p.get(R::CableCgnatMult),
            AccessTech::Fibre => p.get(R::FibreCgnatMult),
            _ => 1.0,
        }
    }

    fn fixed_cgnat(p: &ReachParams, t: AccessTech, r: RegionClass) -> f64 {
        let base = match r {
            RegionClass::V4Rich => p.share(R::FixedCgnatRich),
            RegionClass::V4Scarce => p.share(R::FixedCgnatScarce),
        };
        (base * Self::tech_cgnat_mult(p, t)).clamp(0.0, 1.0)
    }

    /// Home-weighted fixed technology mix.
    fn fixed_mix(p: &ReachParams) -> [(AccessTech, f64); 3] {
        let cable = p.share(R::FixedCableShare);
        let fibre = p.share(R::FixedFibreShare).min(1.0 - cable);
        [(AccessTech::Cable, cable), (AccessTech::Fibre, fibre), (AccessTech::Dsl, (1.0 - cable - fibre).max(0.0))]
    }

    fn region_mix(p: &ReachParams) -> [(RegionClass, f64); 2] {
        let s = p.share(R::RegionV4ScarceShare);
        [(RegionClass::V4Rich, 1.0 - s), (RegionClass::V4Scarce, s)]
    }

    /// Home-weighted mean CGN share over technology and region.
    fn fixed_cgnat_mean(p: &ReachParams) -> f64 {
        let mut c = 0.0;
        for (t, wt) in Self::fixed_mix(p) {
            for (r, wr) in Self::region_mix(p) {
                c += wt * wr * Self::fixed_cgnat(p, t, r);
            }
        }
        c
    }

    fn v6_share(p: &ReachParams, t: AccessTech) -> f64 {
        p.share(match t {
            AccessTech::Cable => R::FixedV6Cable,
            AccessTech::Fibre => R::FixedV6Fibre,
            AccessTech::Dsl => R::FixedV6Dsl,
            AccessTech::Cell4g => R::CellV6_4g,
            AccessTech::Cell5g => R::CellV6_5g,
        })
    }

    /// Draw a technology for a new network of the given kind.
    pub fn draw_tech(&self, cellular: bool, r: &mut Rng) -> AccessTech {
        let p = &self.params;
        if cellular {
            return if r.chance(p.share(R::Cell5gShare)) { AccessTech::Cell5g } else { AccessTech::Cell4g };
        }
        let u = r.f();
        let mut acc = 0.0;
        for (t, w) in Self::fixed_mix(p) {
            acc += w;
            if u < acc {
                return t;
            }
        }
        AccessTech::Dsl
    }

    /// Draw a region class by its home weight.
    pub fn draw_region(&self, r: &mut Rng) -> RegionClass {
        if r.chance(self.params.share(R::RegionV4ScarceShare)) {
            RegionClass::V4Scarce
        } else {
            RegionClass::V4Rich
        }
    }

    /// The network's behaviour: a pure function of `seed` and the network (its id seeds it).
    #[must_use]
    pub fn network(&self, seed: u64, net: AccessNet) -> NetBehaviour {
        let p = &self.params;
        let mut r = Rng::new(mix(seed, 0x4E45_5457 ^ u64::from(net.id), net.tech as u64 + 8 * net.region as u64));
        let cell = net.tech.is_cellular();
        // a fixed draw order, whatever the branch, so one share moving reshuffles nothing else
        let (u_cgn, u_v6, u_fw, u_upnp) = (r.f(), r.f(), r.f(), r.f());
        let cgn_on = if cell { u_cgn < p.share(R::CellCgnatShare) } else { u_cgn < Self::fixed_cgnat(p, net.tech, net.region) };
        let cgn_box = if cell {
            NatBox {
                mapping: pick_mapping(&mut r, self.cal.cell_cgn_eim_share),
                filtering: pick_filter(&mut r, p.share(R::CellFilterEif), p.share(R::CellFilterAdf)),
                hairpin: r.chance(p.share(R::CellHairpinShare)),
            }
        } else {
            NatBox {
                mapping: pick_mapping(&mut r, p.share(R::CgnEimShare)),
                filtering: pick_filter(&mut r, p.share(R::CgnFilterEif), p.share(R::CgnFilterAdf)),
                hairpin: r.chance(p.share(R::CgnHairpinShare)),
            }
        };
        let cpe_box = NatBox {
            mapping: pick_mapping(&mut r, self.cal.router_eim_share),
            filtering: pick_filter(&mut r, p.share(R::RouterFilterEif), p.share(R::RouterFilterAdf)),
            hairpin: r.chance(p.share(R::RouterHairpinShare)),
        };
        let v6 = u_v6 < Self::v6_share(p, net.tech);
        let v6_fw = if cell {
            if u_fw < p.share(R::CellV6FwBlocked) { V6Fw::Blocked } else { V6Fw::Stateful }
        } else if u_fw < p.share(R::FixedV6FwOpen) {
            V6Fw::Open
        } else if u_fw < p.share(R::FixedV6FwOpen) + p.share(R::FixedV6FwBlocked) {
            V6Fw::Blocked
        } else {
            V6Fw::Stateful
        };
        NetBehaviour {
            net,
            cgn: cgn_on.then_some(cgn_box),
            cpe: (!cell).then_some(cpe_box),
            cpe_upnp: !cell && u_upnp < p.share(R::RouterUpnpShare),
            v6,
            v6_fw,
        }
    }

    /// A home on `nb`: a pure function of `seed`, the network and `home` (a stable home index).
    #[must_use]
    pub fn home(&self, seed: u64, nb: &NetBehaviour, home: u64) -> Endpoint {
        let p = &self.params;
        let mut r = Rng::new(mix(seed, 0x484F_4D45 ^ home, u64::from(nb.net.id)));
        let (u_own, u_dbl, u_pf, u_v6, u_fw, u_upnp) = (r.f(), r.f(), r.f(), r.f(), r.f(), r.f());
        let own = NatBox {
            mapping: pick_mapping(&mut r, self.cal.router_eim_share),
            filtering: pick_filter(&mut r, p.share(R::RouterFilterEif), p.share(R::RouterFilterAdf)),
            hairpin: r.chance(p.share(R::RouterHairpinShare)),
        };
        let inner2 = NatBox {
            mapping: pick_mapping(&mut r, self.cal.router_eim_share),
            filtering: pick_filter(&mut r, p.share(R::RouterFilterEif), p.share(R::RouterFilterAdf)),
            hairpin: r.chance(p.share(R::RouterHairpinShare)),
        };
        let own_router = u_own < p.share(R::OwnRouterShare);
        let router = if own_router { own } else { nb.cpe.expect("a fixed network ships a router") };
        let upnp = if own_router { u_upnp < p.share(R::RouterUpnpShare) } else { nb.cpe_upnp };
        let double_nat = u_dbl < p.share(R::DoubleNatShare);
        let mut nat = router;
        if double_nat {
            nat = compose(inner2, nat);
        }
        if let Some(cgn) = nb.cgn {
            nat = compose(nat, cgn);
        }
        let cgnat = nb.cgn.is_some();
        let v4_public = !cgnat && !double_nat;
        let v6 = (nb.v6 && u_v6 < p.share(R::HomeV6EnabledShare)).then(|| {
            if !own_router {
                nb.v6_fw
            } else if u_fw < p.share(R::FixedV6FwOpen) {
                V6Fw::Open
            } else if u_fw < p.share(R::FixedV6FwOpen) + p.share(R::FixedV6FwBlocked) {
                V6Fw::Blocked
            } else {
                V6Fw::Stateful
            }
        });
        Endpoint {
            net: nb.net,
            cgnat,
            double_nat,
            nat: Some(nat),
            cgn_hairpin: nb.cgn.is_some_and(|c| c.hairpin),
            port_forward: v4_public && u_pf < self.cal.port_forward_share,
            port_map_capable: v4_public && upnp,
            v6,
            client_only: false,
        }
    }

    /// A phone on carrier `nb`. A carrier without a CGN gives the phone a public IPv4 address
    /// behind a stateful firewall: endpoint-independent, address-and-port-dependent filtering.
    #[must_use]
    pub fn phone_on_carrier(&self, nb: &NetBehaviour) -> Endpoint {
        let nat = nb.cgn.unwrap_or(NatBox { mapping: Mapping::Eim, filtering: Filtering::Apdf, hairpin: false });
        Endpoint {
            net: nb.net,
            cgnat: nb.cgn.is_some(),
            double_nat: false,
            nat: Some(nat),
            cgn_hairpin: nb.cgn.is_some_and(|c| c.hairpin),
            port_forward: false,
            port_map_capable: false,
            v6: nb.v6.then_some(nb.v6_fw),
            client_only: true,
        }
    }

    /// A phone on its home Wi-Fi: the home's router, as a client-only device.
    #[must_use]
    pub fn phone_at_home(home: &Endpoint) -> Endpoint {
        Endpoint { client_only: true, ..*home }
    }

    /// Exact class marginals at these parameters: (home public, home punchable, cellular
    /// punchable). By linearity, sharing a network does not move a marginal, so the expression is
    /// the same with or without correlation.
    #[must_use]
    pub fn exact_marginals(&self) -> (f64, f64, f64) {
        let p = &self.params;
        let (d, pf, s) = (p.share(R::DoubleNatShare), self.cal.port_forward_share, self.cal.router_eim_share);
        let cgn_eim = p.share(R::CgnEimShare);
        let (mut public, mut punch) = (0.0, 0.0);
        for (t, wt) in Self::fixed_mix(p) {
            for (rg, wr) in Self::region_mix(p) {
                let c = Self::fixed_cgnat(p, t, rg);
                for (pc, cg) in [(1.0 - c, false), (c, true)] {
                    for (pd, dn) in [(1.0 - d, false), (d, true)] {
                        let w = wt * wr * pc * pd;
                        let pub_here = if !cg && !dn { pf } else { 0.0 };
                        public += w * pub_here;
                        // the EIM probability of the composed stack: router (ISP's or own: both s)
                        // x second router x CGN
                        let eim = s * if dn { s } else { 1.0 } * if cg { cgn_eim } else { 1.0 };
                        punch += w * (1.0 - pub_here) * eim;
                    }
                }
            }
        }
        let cc = p.share(R::CellCgnatShare);
        let cell = (1.0 - cc) + cc * self.cal.cell_cgn_eim_share;
        (public, punch, cell)
    }
}

// ---------------------------------------------------------------- worlds and outputs

/// Whether networks are shared (correlated) or every endpoint gets a fresh one (independent
/// draws with the same per-endpoint marginals).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correlation {
    /// Homes share their neighbourhood's ISPs; phones share carriers; a metro is one region.
    Correlated,
    /// Every household and every cellular phone draws its own network (and region).
    Independent,
}

/// Shape of a generated metro (all MODEL).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldShape {
    /// Neighbourhoods (streets) in the metro.
    pub neighbourhoods: usize,
    /// Households per neighbourhood.
    pub households: usize,
    /// Fixed ISPs in the metro.
    pub metro_isps: usize,
    /// At most this many ISPs serve one neighbourhood (last mile is local).
    pub isps_per_neighbourhood: usize,
    /// Cellular carriers in the metro.
    pub carriers: usize,
}

impl Default for WorldShape {
    fn default() -> Self {
        Self { neighbourhoods: 8, households: 60, metro_isps: 6, isps_per_neighbourhood: 3, carriers: 3 }
    }
}

/// One device's endpoint and where it lives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Device {
    /// Neighbourhood.
    pub hood: usize,
    /// Household (global index).
    pub household: usize,
    /// The endpoint the outside sees.
    pub ep: Endpoint,
    /// On its household's network.
    pub on_home_lan: bool,
}

/// A generated metro of households, devices and endpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    /// Devices, grouped by household.
    pub devices: Vec<Device>,
    /// Device index range per household.
    pub households: Vec<(usize, usize)>,
    /// Region class of the metro (correlated mode).
    pub region: RegionClass,
}

fn weights(r: &mut Rng, n: usize) -> Vec<f64> {
    // skewed market shares: the incumbent usually leads (MODEL)
    let w: Vec<f64> = (0..n).map(|_| { let u = r.f().max(1e-9); u * u }).collect();
    let s: f64 = w.iter().sum();
    w.into_iter().map(|x| x / s).collect()
}

fn pick(r: &mut Rng, w: &[f64]) -> usize {
    let u = r.f();
    let mut acc = 0.0;
    for (i, x) in w.iter().enumerate() {
        acc += x;
        if u < acc {
            return i;
        }
    }
    w.len() - 1
}

impl World {
    /// Generate a metro. Each household has one fixed endpoint (its router, with the home node or
    /// laptop behind it) and 1 to 4 phones; a phone is on its home Wi-Fi with `phone_wifi_share`,
    /// else on a carrier (the household's with `family_same_carrier_share`).
    #[must_use]
    pub fn generate(m: &ReachModel, seed: u64, shape: WorldShape, corr: Correlation) -> Self {
        let p = &m.params;
        let mut r = Rng::new(mix(seed, 0x574F_524C, corr as u64));
        let region = m.draw_region(&mut r);
        let mut next_id = 1u32;
        let mut new_net = |r: &mut Rng, cellular: bool, region: RegionClass| {
            let tech = m.draw_tech(cellular, r);
            let id = next_id;
            next_id += 1;
            m.network(seed, AccessNet { id, tech, region })
        };
        let isps: Vec<NetBehaviour> = (0..shape.metro_isps).map(|_| new_net(&mut r, false, region)).collect();
        let carriers: Vec<NetBehaviour> = (0..shape.carriers).map(|_| new_net(&mut r, true, region)).collect();
        let carrier_w = weights(&mut r, carriers.len());
        let mut devices = Vec::new();
        let mut households = Vec::new();
        let mut hh = 0usize;
        for hood in 0..shape.neighbourhoods {
            let k = 1 + r.below(shape.isps_per_neighbourhood.min(isps.len()).max(1));
            let mut local: Vec<usize> = (0..isps.len()).collect();
            for i in 0..local.len() {
                let j = i + r.below(local.len() - i);
                local.swap(i, j);
            }
            local.truncate(k);
            let local_w = weights(&mut r, k);
            for _ in 0..shape.households {
                let start = devices.len();
                let (isp, hh_carrier) = match corr {
                    Correlation::Correlated => (isps[local[pick(&mut r, &local_w)]], carriers[pick(&mut r, &carrier_w)]),
                    Correlation::Independent => {
                        let (rf, rc) = (m.draw_region(&mut r), m.draw_region(&mut r));
                        (new_net(&mut r, false, rf), new_net(&mut r, true, rc))
                    }
                };
                let home = m.home(seed, &isp, hh as u64);
                devices.push(Device { hood, household: hh, ep: home, on_home_lan: true });
                let phones = 1 + r.below(4);
                for _ in 0..phones {
                    let (u_wifi, u_same) = (r.f(), r.f());
                    let carrier = if u_same < p.share(R::FamilySameCarrierShare) {
                        hh_carrier
                    } else {
                        match corr {
                            Correlation::Correlated => carriers[pick(&mut r, &carrier_w)],
                            Correlation::Independent => {
                                let rc = m.draw_region(&mut r);
                                new_net(&mut r, true, rc)
                            }
                        }
                    };
                    let (ep, lan) = if u_wifi < p.share(R::PhoneWifiShare) {
                        (ReachModel::phone_at_home(&home), true)
                    } else {
                        (m.phone_on_carrier(&carrier), false)
                    };
                    devices.push(Device { hood, household: hh, ep, on_home_lan: lan });
                }
                households.push((start, devices.len()));
                hh += 1;
            }
        }
        Self { devices, households, region }
    }

    /// Room-peer pairs by kind, each a list of (a, b, ctx): every family pair; `community_pairs`
    /// sampled pairs of devices in different households of one neighbourhood; as many sampled
    /// pairs across neighbourhoods (not room peers: they may not punch, `accepts_direct`).
    #[must_use]
    pub fn pairs(&self, seed: u64, community_pairs: usize) -> [Vec<(usize, usize, PairCtx)>; 3] {
        let mut fam = Vec::new();
        for &(s, e) in &self.households {
            for a in s..e {
                for b in a + 1..e {
                    let lan = self.devices[a].on_home_lan && self.devices[b].on_home_lan;
                    fam.push((a, b, PairCtx { same_lan: lan, may_punch: true }));
                }
            }
        }
        let mut r = Rng::new(mix(seed, 0x5041_4952, self.devices.len() as u64));
        let n = self.devices.len();
        let mut com = Vec::new();
        let mut other = Vec::new();
        let mut guard = 0usize;
        while (com.len() < community_pairs || other.len() < community_pairs) && guard < community_pairs * 200 {
            guard += 1;
            let (a, b) = (r.below(n), r.below(n));
            let (da, db) = (&self.devices[a], &self.devices[b]);
            if da.household == db.household {
                continue;
            }
            if da.hood == db.hood {
                if com.len() < community_pairs {
                    com.push((a, b, PairCtx::ROOM_PEERS));
                }
            } else if other.len() < community_pairs {
                other.push((a, b, PairCtx { same_lan: false, may_punch: false }));
            }
        }
        [fam, com, other]
    }

    /// Expected share of pairs needing a forwarder (1 - mean direct probability).
    #[must_use]
    pub fn relay_share(&self, pairs: &[(usize, usize, PairCtx)], p: &ReachParams) -> f64 {
        if pairs.is_empty() {
            return 0.0;
        }
        let s: f64 = pairs.iter().map(|&(a, b, c)| 1.0 - direct_path(&self.devices[a].ep, &self.devices[b].ep, c, p).p).sum();
        s / pairs.len() as f64
    }
}

/// Act mix that weights pair kinds (micro `family_share`, `community_share`, rest other; MODEL).
pub const PAIR_MIX: [f64; 3] = [0.65, 0.30, 0.05];

/// Relay share of one world, by kind and weighted: [family, community, other, weighted].
#[must_use]
pub fn world_relay(m: &ReachModel, seed: u64, shape: WorldShape, corr: Correlation, community_pairs: usize) -> [f64; 4] {
    let w = World::generate(m, seed, shape, corr);
    let pr = w.pairs(seed, community_pairs);
    let k: Vec<f64> = pr.iter().map(|x| w.relay_share(x, &m.params)).collect();
    [k[0], k[1], k[2], PAIR_MIX[0] * k[0] + PAIR_MIX[1] * k[1] + PAIR_MIX[2] * k[2]]
}

/// The micro tier's rule ([`legacy_direct`] on each end's class, LAN kept) on the same world and
/// pairs: [family, community, other, weighted]. What the old independent shares would say about
/// exactly these endpoints.
#[must_use]
pub fn world_relay_legacy(m: &ReachModel, seed: u64, shape: WorldShape, corr: Correlation, community_pairs: usize) -> [f64; 4] {
    let w = World::generate(m, seed, shape, corr);
    let pr = w.pairs(seed, community_pairs);
    let k: Vec<f64> = pr
        .iter()
        .map(|x| {
            if x.is_empty() {
                return 0.0;
            }
            let s: f64 = x
                .iter()
                .map(|&(a, b, c)| {
                    let d = if c.same_lan {
                        1.0
                    } else {
                        let l = legacy_direct(w.devices[a].ep.class(), w.devices[b].ep.class(), &m.params);
                        if !c.may_punch && l < 1.0 { 0.0 } else { l }
                    };
                    1.0 - d
                })
                .sum();
            s / x.len() as f64
        })
        .collect();
    [k[0], k[1], k[2], PAIR_MIX[0] * k[0] + PAIR_MIX[1] * k[1] + PAIR_MIX[2] * k[2]]
}

/// Summary of a sample: mean, 10th, 50th and 90th percentile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dist {
    /// Mean.
    pub mean: f64,
    /// 10th percentile.
    pub p10: f64,
    /// Median.
    pub p50: f64,
    /// 90th percentile.
    pub p90: f64,
    /// Standard error of the mean (sample sd / sqrt(n)).
    pub se: f64,
}

impl Dist {
    /// Summarise `xs`.
    #[must_use]
    pub fn of(xs: &[f64]) -> Self {
        let mut v = xs.to_vec();
        v.sort_by(|a, b| a.total_cmp(b));
        let q = |f: f64| if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f64 * f).round() as usize] };
        let n = v.len() as f64;
        let mean = if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / n };
        let se = if v.len() < 2 { 0.0 } else { (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() / n.sqrt() };
        Self { mean, p10: q(0.1), p50: q(0.5), p90: q(0.9), se }
    }
}

/// Relay share over `worlds` seeded metros: weighted share and community share per world.
#[must_use]
pub fn relay_distribution(m: &ReachModel, seed: u64, worlds: usize, corr: Correlation) -> (Dist, Dist) {
    let mut wt = Vec::with_capacity(worlds);
    let mut com = Vec::with_capacity(worlds);
    for i in 0..worlds {
        let r = world_relay(m, mix(seed, 0x5745_4C44, i as u64), WorldShape::default(), corr, 400);
        wt.push(r[3]);
        com.push(r[1]);
    }
    (Dist::of(&wt), Dist::of(&com))
}

/// Per-neighbourhood community relay share across `worlds` metros: the picture the R5 street
/// shows. Returns the distribution over neighbourhoods.
#[must_use]
pub fn neighbourhood_distribution(m: &ReachModel, seed: u64, worlds: usize, corr: Correlation) -> Dist {
    let shape = WorldShape::default();
    let mut xs = Vec::new();
    for i in 0..worlds {
        let ws = mix(seed, 0x484F_4F44, i as u64);
        let w = World::generate(m, ws, shape, corr);
        // every cross-household pair of each neighbourhood
        for h in 0..shape.neighbourhoods {
            let idx: Vec<usize> = (0..w.devices.len()).filter(|&d| w.devices[d].hood == h).collect();
            let mut pairs = Vec::new();
            for (x, &a) in idx.iter().enumerate() {
                for &b in &idx[x + 1..] {
                    if w.devices[a].household != w.devices[b].household {
                        pairs.push((a, b, PairCtx::ROOM_PEERS));
                    }
                }
            }
            xs.push(w.relay_share(&pairs, &m.params));
        }
    }
    Dist::of(&xs)
}

/// One cell of the access-class matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// Mean direct-path probability.
    pub p_direct: f64,
    /// Share of pairs by mechanism, in [`Mechanism::ALL`] order.
    pub by_mechanism: [f64; 7],
    /// Pairs sampled.
    pub n: usize,
}

fn sample_endpoint(m: &ReachModel, seed: u64, r: &mut Rng, tech: AccessTech, id: u32) -> (NetBehaviour, Endpoint) {
    let region = m.draw_region(r);
    let nb = m.network(seed, AccessNet { id, tech, region });
    let ep = if tech.is_cellular() { m.phone_on_carrier(&nb) } else { m.home(seed, &nb, u64::from(id)) };
    (nb, ep)
}

/// Direct-path probability by access-technology pair, room peers with a facilitator. A fixed end
/// is a home (its home node listens if it can); a cellular end is a client-only phone.
/// `same_network`: both ends on one network (one ISP or one carrier; off the diagonal the two
/// technologies cannot share one, so those cells are empty).
#[must_use]
pub fn tech_matrix(m: &ReachModel, seed: u64, n: usize, same_network: bool) -> [[Cell; 5]; 5] {
    let empty = Cell { p_direct: 0.0, by_mechanism: [0.0; 7], n: 0 };
    let mut out = [[empty; 5]; 5];
    let mut r = Rng::new(mix(seed, 0x4D41_5452, u64::from(same_network)));
    let mut id = 1u32;
    for (i, &ta) in AccessTech::ALL.iter().enumerate() {
        for (j, &tb) in AccessTech::ALL.iter().enumerate() {
            if same_network && ta != tb {
                continue;
            }
            let mut c = empty;
            for _ in 0..n {
                let (nb, a) = sample_endpoint(m, seed, &mut r, ta, id);
                id += 1;
                let b = if same_network {
                    if ta.is_cellular() { m.phone_on_carrier(&nb) } else { m.home(seed, &nb, u64::from(id) | 1 << 40) }
                } else {
                    let (_, b) = sample_endpoint(m, seed, &mut r, tb, id);
                    b
                };
                id += 1;
                let d = direct_path(&a, &b, PairCtx::ROOM_PEERS, &m.params);
                c.p_direct += d.p;
                c.by_mechanism[d.mechanism as usize] += 1.0;
            }
            c.n = n;
            c.p_direct /= n as f64;
            for x in &mut c.by_mechanism {
                *x /= n as f64;
            }
            out[i][j] = c;
        }
    }
    out
}

/// Direct-path probability by micro class pair (public / punchable / hard). Each end is a home or
/// a phone on a carrier with even odds, on its own network; room peers with a facilitator.
/// Returns (p_direct, pairs) per cell, upper triangle.
#[must_use]
pub fn class_matrix(m: &ReachModel, seed: u64, n: usize) -> [[(f64, usize); 3]; 3] {
    let mut acc = [[(0.0, 0usize); 3]; 3];
    let mut r = Rng::new(mix(seed, 0x434C_4153, 0));
    let mut id = 1u32;
    for _ in 0..n {
        let ta = m.draw_tech(r.chance(0.5), &mut r);
        let tb = m.draw_tech(r.chance(0.5), &mut r);
        let (_, a) = sample_endpoint(m, seed, &mut r, ta, id);
        let (_, b) = sample_endpoint(m, seed, &mut r, tb, id + 1);
        id += 2;
        let d = direct_path(&a, &b, PairCtx::ROOM_PEERS, &m.params);
        let (x, y) = (a.class() as usize, b.class() as usize);
        let (x, y) = (x.min(y), x.max(y));
        acc[x][y].0 += d.p;
        acc[x][y].1 += 1;
    }
    for row in &mut acc {
        for c in row.iter_mut() {
            if c.1 > 0 {
                c.0 /= c.1 as f64;
            }
        }
    }
    acc
}

/// The micro tier's own rule on the same endpoints' classes, for comparison: public end → direct;
/// both punchable → `punch_success_share`; else relay. (Cellular has no public class.)
#[must_use]
pub fn legacy_direct(a: ReachClass, b: ReachClass, p: &ReachParams) -> f64 {
    match (a, b) {
        (ReachClass::Public, _) | (_, ReachClass::Public) => 1.0,
        (ReachClass::Punchable, ReachClass::Punchable) => p.share(R::PunchSuccessShare),
        _ => 0.0,
    }
}

/// Metros per regime in [`report`].
const METROS: usize = 256;

/// The text report `cewp-cli nat` prints.
#[must_use]
pub fn report(seed: u64) -> String {
    use std::fmt::Write as _;
    use crate::workload::ParameterRegime as G;
    let mut s = String::new();
    let m = ReachModel::default_model();
    let (pubs, punch, cell) = m.exact_marginals();
    let _ = writeln!(s, "CEWP twin: joint reachability model (crates/cewp-sim/src/reach.rs), seed {seed}");
    let _ = writeln!(s, "\nCalibration (DERIVED) so the class marginals equal the micro defaults:");
    let _ = writeln!(s, "  port_forward_share {:.4}  router_eim_share {:.4}  cell_cgn_eim_share {:.4}", m.cal.port_forward_share, m.cal.router_eim_share, m.cal.cell_cgn_eim_share);
    let _ = writeln!(s, "  marginals: home public {pubs:.4} (target 0.12), home punchable {punch:.4} (0.28), cellular punchable {cell:.4} (0.15)");
    let _ = writeln!(s, "\nDirect-path probability by access pair, room peers, facilitator up, distinct networks (MODEL):");
    let mx = tech_matrix(&m, seed, 4000, false);
    let _ = write!(s, "  {:>6}", "");
    for t in AccessTech::ALL {
        let _ = write!(s, " {:>6}", t.as_str());
    }
    let _ = writeln!(s);
    for (i, ta) in AccessTech::ALL.iter().enumerate() {
        let _ = write!(s, "  {:>6}", ta.as_str());
        for c in &mx[i] {
            let _ = write!(s, " {:>6.3}", c.p_direct);
        }
        let _ = writeln!(s);
    }
    let same = tech_matrix(&m, seed, 4000, true);
    let _ = write!(s, "  same network (one ISP / one carrier):");
    for (i, t) in AccessTech::ALL.iter().enumerate() {
        let _ = write!(s, " {} {:.3}", t.as_str(), same[i][i].p_direct);
    }
    let _ = writeln!(s);
    let _ = writeln!(s, "  mechanism mix over all distinct-network cells:");
    let mut mech = [0.0; 7];
    for row in &mx {
        for c in row {
            for (k, x) in c.by_mechanism.iter().enumerate() {
                mech[k] += x / 25.0;
            }
        }
    }
    for (k, mm) in Mechanism::ALL.iter().enumerate() {
        let _ = writeln!(s, "    {:<22} {:.3}", mm.as_str(), mech[k]);
    }
    let cm = class_matrix(&m, seed, 40_000);
    let _ = writeln!(s, "\nBy micro class pair (joint model vs the micro rule):");
    for (x, (row, cx)) in cm.iter().zip(ReachClass::ALL).enumerate() {
        for (&(pd, n), cy) in row.iter().zip(ReachClass::ALL).skip(x) {
            let l = legacy_direct(cx, cy, &m.params);
            let _ = writeln!(s, "  {:>9} x {:<9} joint {pd:.3}  micro {l:.3}  (n {n})", cx.as_str(), cy.as_str());
        }
    }
    let _ = writeln!(s, "\nRelay share (pairs needing a forwarder, weighted family {:.2} / community {:.2} / other {:.2}), {} metros, mean (se) [p10,p90]:", PAIR_MIX[0], PAIR_MIX[1], PAIR_MIX[2], METROS);
    let _ = writeln!(s, "  {:<12} {:>30} {:>30} {:>16}", "regime", "correlated", "independent", "delta (se)");
    for g in [G::Optimistic, G::Expected, G::Pessimistic, G::Adversarial] {
        let mg = match ReachModel::new(ReachParams::for_regime(g)) {
            Ok(x) => x,
            Err(e) => {
                let _ = writeln!(s, "  {g:?}: calibration failed: {e}");
                continue;
            }
        };
        let (c, _) = relay_distribution(&mg, seed, METROS, Correlation::Correlated);
        let (i, _) = relay_distribution(&mg, seed, METROS, Correlation::Independent);
        let _ = writeln!(s, "  {:<12} {:>7.3} ({:.3}) [{:.3},{:.3}]   {:>7.3} ({:.3}) [{:.3},{:.3}]   {:+.3} ({:.3})", format!("{g:?}"), c.mean, c.se, c.p10, c.p90, i.mean, i.se, i.p10, i.p90, c.mean - i.mean, c.se.hypot(i.se));
    }
    let legacy: Vec<f64> = (0..METROS).map(|i| world_relay_legacy(&m, mix(seed, 0x5745_4C44, i as u64), WorldShape::default(), Correlation::Correlated, 400)[3]).collect();
    let lg = Dist::of(&legacy);
    let _ = writeln!(s, "  micro rule on the same Expected worlds: {:.3} [{:.3},{:.3}]", lg.mean, lg.p10, lg.p90);
    let mut v4 = m.params.clone();
    v4.set(R::Ipv6Candidates, 0.0);
    let m4 = ReachModel::new(v4.clone()).expect("calibrates");
    let (c4, _) = relay_distribution(&m4, seed, METROS, Correlation::Correlated);
    let (i4, _) = relay_distribution(&m4, seed, METROS, Correlation::Independent);
    let _ = writeln!(s, "  Expected, IPv6 candidates off: correlated {:.3} [{:.3},{:.3}]  independent {:.3} [{:.3},{:.3}]  delta {:+.3}", c4.mean, c4.p10, c4.p90, i4.mean, i4.p10, i4.p90, c4.mean - i4.mean);
    let mut pr = v4.clone();
    pr.set(R::PeerReflexive, 0.0);
    let mp = ReachModel::new(pr).expect("calibrates");
    let (cp, _) = relay_distribution(&mp, seed, METROS, Correlation::Correlated);
    let _ = writeln!(s, "  Expected, IPv6 and peer-reflexive off (closest to the micro rule): correlated {:.3}", cp.mean);
    let (_, cc) = relay_distribution(&m, seed, METROS, Correlation::Correlated);
    let (_, ci) = relay_distribution(&m, seed, METROS, Correlation::Independent);
    let _ = writeln!(s, "\nCommunity pairs only (Expected): correlated {:.3} ({:.3}) [{:.3},{:.3}]  independent {:.3} ({:.3}) [{:.3},{:.3}]", cc.mean, cc.se, cc.p10, cc.p90, ci.mean, ci.se, ci.p10, ci.p90);
    let nc = neighbourhood_distribution(&m, seed, 32, Correlation::Correlated);
    let ni = neighbourhood_distribution(&m, seed, 32, Correlation::Independent);
    let _ = writeln!(s, "Per-street community relay share (256 streets, every cross-household pair):");
    let _ = writeln!(s, "  correlated  mean {:.3} ({:.3}) p10 {:.3} p50 {:.3} p90 {:.3}", nc.mean, nc.se, nc.p10, nc.p50, nc.p90);
    let _ = writeln!(s, "  independent mean {:.3} ({:.3}) p10 {:.3} p50 {:.3} p90 {:.3}", ni.mean, ni.se, ni.p10, ni.p50, ni.p90);
    let e = hooks::energy_counters(&m, seed, Correlation::Correlated, 400);
    let _ = writeln!(s, "\nEnergy counters, one realised Expected metro (DERIVED counts; HOOKS.md section 1; no time, no watts):");
    let _ = write!(s, "  pairs by outcome:");
    for o in hooks::Outcome::ALL {
        let _ = write!(s, " {} {}", o.as_str(), e.per_outcome[o as usize]);
    }
    let _ = writeln!(s);
    for k in hooks::NodeClass::ALL {
        let c = e.per_class[k as usize];
        let nodes = e.class.iter().filter(|&&x| x == k).count();
        let _ = writeln!(s, "  {:<15} nodes {nodes:>5}  pair-ends {:>6}  direct {:>6}  relayed {:>6}  punch attempts {:>6} (failed {:>5})  held mappings {:>6}", k.as_str(), c.pairs, c.direct, c.relayed, c.punch_attempts, c.punch_failures, c.held_mappings);
    }
    let _ = writeln!(s, "\nEvery input is MODEL or FRONTIER except the DERIVED switches and calibration; no output is MEASURED.");
    s
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(mapping: Mapping, filtering: Filtering) -> Endpoint {
        Endpoint {
            net: AccessNet { id: 0, tech: AccessTech::Cable, region: RegionClass::V4Rich },
            cgnat: false,
            double_nat: false,
            nat: Some(NatBox { mapping, filtering, hairpin: false }),
            cgn_hairpin: false,
            port_forward: false,
            port_map_capable: false,
            v6: None,
            client_only: false,
        }
    }

    fn on_net(mut e: Endpoint, id: u32) -> Endpoint {
        e.net.id = id;
        e
    }

    #[test]
    fn every_parameter_has_a_class_and_a_source() {
        for s in REACH_PARAM_SPECS {
            assert!(!s.source.is_empty(), "{}", s.name);
        }
        assert_eq!(REACH_PARAM_SPECS[R::PortMappingClient as usize].class, Class::Derived);
        assert_eq!(REACH_PARAM_SPECS[R::PunchSuccessShare as usize].class, Class::Frontier);
    }

    #[test]
    fn the_marginals_at_the_defaults_equal_the_micro_defaults() {
        let m = ReachModel::default_model();
        let (public, punch, cell) = m.exact_marginals();
        assert!((public - 0.40 * 0.30).abs() < 1e-9, "{public}");
        assert!((punch - (0.40 - 0.12)).abs() < 1e-9, "{punch}");
        assert!((cell - 0.15).abs() < 1e-9, "{cell}");
        // and the targets are micro's own defaults
        let micro = crate::micro::MicroParams::default();
        assert_eq!(micro.get(crate::micro::P::ReachHomeShare), m.params.get(R::ReachHomeShare));
        assert_eq!(micro.get(crate::micro::P::PublicListenerShare), m.params.get(R::PublicListenerShare));
        assert_eq!(micro.get(crate::micro::P::CellularPunchableShare), m.params.get(R::CellularPunchableShare));
        assert_eq!(micro.get(crate::micro::P::PunchSuccessShare), m.params.get(R::PunchSuccessShare));
    }

    #[test]
    fn sampled_classes_match_the_exact_marginals() {
        // 60,000 homes and 60,000 phones on fresh networks: the binomial sd of a share is under
        // 0.002, so 0.01 is five sd
        let m = ReachModel::default_model();
        let mut r = Rng::new(7);
        let (mut public, mut punch, mut cell) = (0usize, 0usize, 0usize);
        let n = 60_000;
        for i in 0..n {
            let t = m.draw_tech(false, &mut r);
            let (_, h) = sample_endpoint(&m, 11, &mut r, t, 2 * i as u32 + 1);
            match h.class() {
                ReachClass::Public => public += 1,
                ReachClass::Punchable => punch += 1,
                ReachClass::Hard => {}
            }
            let t = m.draw_tech(true, &mut r);
            let (_, c) = sample_endpoint(&m, 11, &mut r, t, 2 * i as u32 + 2);
            if c.class() == ReachClass::Punchable {
                cell += 1;
            }
        }
        let f = |k: usize| k as f64 / n as f64;
        assert!((f(public) - 0.12).abs() < 0.01, "{}", f(public));
        assert!((f(punch) - 0.28).abs() < 0.01, "{}", f(punch));
        assert!((f(cell) - 0.15).abs() < 0.01, "{}", f(cell));
    }

    #[test]
    fn punchable_pairs_on_distinct_networks_punch_at_punch_success_share() {
        let mut p = ReachParams::default();
        p.set(R::Ipv6Candidates, 0.0);
        let m = ReachModel::new(p).unwrap();
        let mut r = Rng::new(3);
        let (mut sum, mut n) = (0.0, 0usize);
        for i in 0..20_000u32 {
            let (ta, tb) = (m.draw_tech(r.chance(0.5), &mut r), m.draw_tech(r.chance(0.5), &mut r));
            let (_, a) = sample_endpoint(&m, 5, &mut r, ta, 2 * i + 1);
            let (_, b) = sample_endpoint(&m, 5, &mut r, tb, 2 * i + 2);
            if a.class() == ReachClass::Punchable && b.class() == ReachClass::Punchable {
                let d = direct_path(&a, &b, PairCtx::ROOM_PEERS, &m.params);
                assert_eq!(d.mechanism, Mechanism::HolePunch);
                sum += d.p;
                n += 1;
            }
        }
        assert!(n > 100);
        assert!((sum / n as f64 - 0.90).abs() < 1e-12);
    }

    #[test]
    fn symmetric_by_symmetric_never_punches() {
        let p = ReachParams::default();
        for ma in [Mapping::Adm, Mapping::Apdm] {
            for mb in [Mapping::Adm, Mapping::Apdm] {
                for fa in [Filtering::Eif, Filtering::Adf, Filtering::Apdf] {
                    for fb in [Filtering::Eif, Filtering::Adf, Filtering::Apdf] {
                        let (a, b) = (on_net(ep(ma, fa), 1), on_net(ep(mb, fb), 2));
                        let d = direct_path(&a, &b, PairCtx::ROOM_PEERS, &p);
                        assert_eq!(d.mechanism, Mechanism::Relay, "{ma:?}/{fa:?} x {mb:?}/{fb:?}");
                        assert_eq!(d.p, 0.0);
                    }
                }
            }
        }
    }

    #[test]
    fn eim_against_symmetric_needs_a_filter_that_admits_the_new_port() {
        let p = ReachParams::default();
        let s = on_net(ep(Mapping::Apdm, Filtering::Apdf), 2);
        for (f, ok) in [(Filtering::Eif, true), (Filtering::Adf, true), (Filtering::Apdf, false)] {
            let e = on_net(ep(Mapping::Eim, f), 1);
            let d = direct_path(&e, &s, PairCtx::ROOM_PEERS, &p);
            assert_eq!(d.mechanism == Mechanism::PeerReflexivePunch, ok, "{f:?}");
            assert_eq!(direct_path(&s, &e, PairCtx::ROOM_PEERS, &p), d, "symmetric in its arguments");
        }
        let mut off = p.clone();
        off.set(R::PeerReflexive, 0.0);
        let e = on_net(ep(Mapping::Eim, Filtering::Eif), 1);
        assert_eq!(direct_path(&e, &s, PairCtx::ROOM_PEERS, &off).mechanism, Mechanism::Relay);
    }

    #[test]
    fn two_eim_ends_punch_under_any_filtering() {
        let p = ReachParams::default();
        for fa in [Filtering::Eif, Filtering::Adf, Filtering::Apdf] {
            for fb in [Filtering::Eif, Filtering::Adf, Filtering::Apdf] {
                let d = direct_path(&on_net(ep(Mapping::Eim, fa), 1), &on_net(ep(Mapping::Eim, fb), 2), PairCtx::ROOM_PEERS, &p);
                assert_eq!(d.mechanism, Mechanism::HolePunch);
                assert_eq!(d.p, p.get(R::PunchSuccessShare));
            }
        }
    }

    #[test]
    fn a_public_listener_is_always_reachable() {
        let p = ReachParams::default();
        let m = ReachModel::default_model();
        let mut r = Rng::new(9);
        let mut listener = ep(Mapping::Apdm, Filtering::Apdf);
        listener.port_forward = true;
        for i in 0..5_000u32 {
            let t = m.draw_tech(r.chance(0.5), &mut r);
            let (_, other) = sample_endpoint(&m, 1, &mut r, t, i + 10);
            for ctx in [PairCtx::ROOM_PEERS, PairCtx { same_lan: false, may_punch: false }] {
                let d = direct_path(&listener, &other, ctx, &p);
                assert_eq!((d.mechanism, d.p), (Mechanism::PublicListener, 1.0));
                assert_eq!(direct_path(&other, &listener, ctx, &p).p, 1.0);
            }
        }
        // a client-only phone never listens, whatever its address
        let mut phone = listener;
        phone.client_only = true;
        let hard = on_net(ep(Mapping::Apdm, Filtering::Apdf), 3);
        assert_eq!(direct_path(&phone, &hard, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::Relay);
    }

    #[test]
    fn no_facilitator_or_no_room_means_no_punch() {
        let p = ReachParams::default();
        let (a, b) = (on_net(ep(Mapping::Eim, Filtering::Eif), 1), on_net(ep(Mapping::Eim, Filtering::Eif), 2));
        let d = direct_path(&a, &b, PairCtx { same_lan: false, may_punch: false }, &p);
        assert_eq!(d.mechanism, Mechanism::Relay);
    }

    #[test]
    fn same_cgn_needs_hairpin() {
        let p = ReachParams::default();
        let mut a = on_net(ep(Mapping::Eim, Filtering::Eif), 4);
        a.cgnat = true;
        let mut b = a;
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::Relay);
        a.cgn_hairpin = true;
        b.cgn_hairpin = true;
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::HolePunch);
        // on different networks no hairpin is needed
        let mut c = b;
        c.cgn_hairpin = false;
        c.net.id = 5;
        let mut a2 = a;
        a2.cgn_hairpin = false;
        assert_eq!(direct_path(&a2, &c, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::HolePunch);
    }

    #[test]
    fn ipv6_punches_past_two_symmetric_nats_unless_a_firewall_blocks() {
        let p = ReachParams::default();
        let (mut a, mut b) = (on_net(ep(Mapping::Apdm, Filtering::Apdf), 1), on_net(ep(Mapping::Apdm, Filtering::Apdf), 2));
        a.v6 = Some(V6Fw::Stateful);
        b.v6 = Some(V6Fw::Stateful);
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::Ipv6Punch);
        b.v6 = Some(V6Fw::Blocked);
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::Relay);
        b.v6 = None;
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::Relay);
    }

    #[test]
    fn port_mapping_is_a_counterfactual_edge_does_not_have() {
        let mut p = ReachParams::default();
        let mut a = on_net(ep(Mapping::Apdm, Filtering::Apdf), 1);
        a.port_map_capable = true;
        let b = on_net(ep(Mapping::Apdm, Filtering::Apdf), 2);
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::Relay);
        p.set(R::PortMappingClient, 1.0);
        assert_eq!(direct_path(&a, &b, PairCtx::ROOM_PEERS, &p).mechanism, Mechanism::PortMapped);
    }

    #[test]
    fn draws_are_deterministic_by_seed() {
        let m = ReachModel::default_model();
        let a = World::generate(&m, 42, WorldShape::default(), Correlation::Correlated);
        let b = World::generate(&m, 42, WorldShape::default(), Correlation::Correlated);
        assert_eq!(a, b);
        let c = World::generate(&m, 43, WorldShape::default(), Correlation::Correlated);
        assert_ne!(a, c);
        assert_eq!(world_relay(&m, 42, WorldShape::default(), Correlation::Correlated, 300), world_relay(&m, 42, WorldShape::default(), Correlation::Correlated, 300));
        assert_eq!(tech_matrix(&m, 1, 200, false), tech_matrix(&m, 1, 200, false));
        let net = AccessNet { id: 9, tech: AccessTech::Fibre, region: RegionClass::V4Scarce };
        assert_eq!(m.network(5, net), m.network(5, net));
    }

    #[test]
    fn a_network_is_consistent_across_its_homes() {
        // with no CGN, a home that keeps the ISP's router and has no second NAT shows exactly the
        // ISP router's behaviour: own_router 0.30 and double NAT 0.10 leave about 63 % such homes
        let m = ReachModel::default_model();
        let nb = m.network(1, AccessNet { id: 3, tech: AccessTech::Dsl, region: RegionClass::V4Rich });
        assert!(nb.cgn.is_none(), "this seed's network has no CGN");
        let cpe = nb.cpe.unwrap();
        let kept = (0..400u64).filter(|&h| m.home(1, &nb, h).nat == Some(cpe)).count();
        assert!(kept > 200, "{kept}");
        // every phone on one carrier sees the carrier's CGN
        let cnb = m.network(1, AccessNet { id: 4, tech: AccessTech::Cell4g, region: RegionClass::V4Rich });
        assert_eq!(m.phone_on_carrier(&cnb), m.phone_on_carrier(&cnb));
    }

    #[test]
    fn correlated_and_independent_draws_differ_on_the_relay_share_of_a_fixed_world() {
        let m = ReachModel::default_model();
        let c = world_relay(&m, 2024, WorldShape::default(), Correlation::Correlated, 2000);
        let i = world_relay(&m, 2024, WorldShape::default(), Correlation::Independent, 2000);
        assert!((c[3] - i[3]).abs() > 1e-3, "correlated {c:?} independent {i:?}");
    }

    #[test]
    fn shared_networks_widen_the_spread_across_streets() {
        // the finding, pinned on a fixed seed: with the same per-endpoint marginals, streets
        // sharing their ISPs and carriers spread their community relay share much wider
        let m = ReachModel::default_model();
        let c = neighbourhood_distribution(&m, 77, 8, Correlation::Correlated);
        let i = neighbourhood_distribution(&m, 77, 8, Correlation::Independent);
        assert!(c.p90 - c.p10 > 2.0 * (i.p90 - i.p10), "correlated {c:?} independent {i:?}");
    }

    #[test]
    fn every_regime_calibrates() {
        use crate::workload::ParameterRegime as G;
        for g in [G::Optimistic, G::Expected, G::Pessimistic, G::Adversarial] {
            let m = ReachModel::new(ReachParams::for_regime(g)).unwrap_or_else(|e| panic!("{g:?}: {e}"));
            let (public, punch, _) = m.exact_marginals();
            let reach = crate::workload::RegimeParameters::for_regime(g).reach_home_share;
            assert!((public + punch - reach).abs() < 1e-9, "{g:?}");
        }
        // adversarial: nothing reachable over IPv4 and no IPv6, so only LAN is direct
        let m = ReachModel::new(ReachParams::for_regime(G::Adversarial)).unwrap();
        let w = World::generate(&m, 1, WorldShape::default(), Correlation::Correlated);
        let pr = w.pairs(1, 300);
        assert!((w.relay_share(&pr[1], &m.params) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn an_infeasible_target_is_refused() {
        let mut p = ReachParams::default();
        p.set(R::ReachHomeShare, 1.0);
        p.set(R::PublicListenerShare, 1.0);
        assert!(ReachModel::new(p).is_err());
        let mut p = ReachParams::default();
        p.set(R::CellularPunchableShare, 0.01); // below the 5 % of carriers with no CGN
        assert!(ReachModel::new(p).is_err());
    }
}
