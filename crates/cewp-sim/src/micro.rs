//! R5 micro tier: a focal 1:1 neighbourhood of individual devices (FSD §4.2 "focal subgraph at
//! 1:1", §4.5 tier R5).
//!
//! The globe's cells are cohorts. This module is the other end of the ladder: one generated
//! neighbourhood (streets, buildings, households), every device in it as its own node with a
//! host kind, plus the off-street nodes those devices actually talk to (serve seats, hosted
//! servers, CI runners in data centres). Each step moves the devices through the states their
//! host kind gives them (sleep, network change, charging, join/leave) and derives the links and
//! routes the transport rules allow at that instant.
//!
//! # Where each behaviour comes from
//!
//! Constants copied from the production stack carry their citation. Everything else is a named
//! scenario parameter in [`PARAM_SPECS`] with a default and an FSD §8.2 tag (`MODEL` or
//! `FRONTIER`); nothing is hidden in the code.
//!
//! - Budget classes, the bound of eight, the lower of two offers, the seat class outside the
//!   eight: [`crate::transport`] (CC rc17 5.4.3; CIRISEdge `src/transport/link_shaping.rs`).
//! - A phone's class follows its network; every v44/v45 phone is client-only (dials, listens for
//!   nothing, gives no relay slot, is never referred): CIRISEdge `DeviceState::of`,
//!   `DeviceState::budget_class` (`link_shaping.rs`).
//! - A laptop, a desktop, and a CI runner run as `home`: a non-mobile build's
//!   `DeviceKind::platform_default()` is `Home`, and no host class names a laptop or a CI runner.
//! - Relay policy: server and home relay by default; a phone only while unmetered and charging
//!   (CC 5.4.6 item 1; CIRISEdge `src/onion/policy.rs` `device_relays`); a seat may not opt out.
//! - A direct path is a shared interface (LAN), one end's public listener, or a punched UDP path
//!   (CIRISEdge `FSD/CIRIS_MESH.md` §"Direct path"). Without one, a row crosses a forwarder sealed
//!   and unshaped (CC 5.4.3).
//! - Direct-path proposals: on by default from v44, room peers only, facilitated by the canonical,
//!   20 s budget, 1 h back-off after a failed punch (CIRISEdge `src/transport/direct_path.rs`).
//! - A phone is never a mailbox; a member-owned always-on node is; the federation rendezvous
//!   mailbox is a serve seat (CC rc17 5.4.6 item 6).
//! - Canonical homing on the top k of M seats by `H(node ‖ seat ‖ period)` (CIRISEdge#935,
//!   `FSD/TRANSPORT_TIERS_MODEL.md` §0 item 3): a model, not shipped code, so k and M are MODEL.
//! - Suspend closes every link; resume re-announces (CIRISEdge `src/edge.rs` `suspend`/`resume`,
//!   `RESUME_ANNOUNCE_WAIT`).
//! - CIRISEdge v45 has no rank displacement yet (CIRISEdge#939): a full node runs the next link
//!   unshaped (or refuses it, if shaped-only). This module follows v45, not
//!   [`crate::transport::ShapedLinkManager`], which models the CC's displacement.

use crate::transport::{BudgetClass, SHAPED_ASK_IDLE_TIMEOUT_SECS, SHAPED_LINK_BOUND};
use crate::workload::diurnal_multiplier;
use std::collections::{BTreeMap, VecDeque};

// ------------------------------------------------------------------ sourced constants

/// `DIRECT_PATH_BUDGET` (CIRISEdge `src/transport/direct_path.rs`): how long a punch may take.
pub const DIRECT_PATH_BUDGET_S: f64 = 20.0;
/// `DIRECT_PUNCH_FAILED_BACKOFF` (`direct_path.rs`): no new proposal to a pair for an hour after
/// a failed punch.
pub const DIRECT_PUNCH_FAILED_BACKOFF_S: f64 = 3_600.0;
/// `RESUME_ANNOUNCE_WAIT` (CIRISEdge `src/transport/reticulum.rs`): a resumed node waits for
/// its re-announce before it is reachable again.
pub const RESUME_ANNOUNCE_WAIT_S: f64 = 10.0;
/// `DEFAULT_LINK_POOL_IDLE_BOUND` (`reticulum.rs`): an idle pooled peer link is closed after two
/// minutes without traffic.
pub const LINK_POOL_IDLE_BOUND_S: f64 = 120.0;
/// `announce_interval` default (`reticulum.rs`; PyO3 `announce_interval_seconds=300`): announces
/// refresh a node's homing links every five minutes.
pub const ANNOUNCE_INTERVAL_S: f64 = 300.0;
/// FSD §4.5: tier R5 is 1 to 1,000 nodes at 1:1.
pub const R5_MAX_NODES: usize = 1_000;
/// Off-patch CI runner slots kept in the node table (a runner that leaves frees its slot).
pub const CI_SLOTS: usize = 6;
/// Ring of recent message events kept for the picture.
pub const EVENT_RING: usize = 384;
/// Sentinel for "no node".
pub const NONE: u32 = u32::MAX;

// ------------------------------------------------------------------ scenario parameters

/// FSD §8.2 provenance of a parameter's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    /// A stated modelling choice inside the simulation.
    Model,
    /// An open empirical question: the default is a placeholder until field data exists.
    Frontier,
    /// Read from the production stack or the constitution; the citation names where.
    Sourced,
}

impl Tag {
    /// The tag as written in the FSD.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Model => "MODEL",
            Self::Frontier => "FRONTIER",
            Self::Sourced => "SOURCED",
        }
    }
}

/// One named scenario parameter.
#[derive(Debug, Clone, Copy)]
pub struct ParamSpec {
    /// Stable name (scenario TOML key).
    pub name: &'static str,
    /// Default value.
    pub default: f64,
    /// Provenance.
    pub tag: Tag,
    /// Where the default comes from, or why it is a placeholder.
    pub source: &'static str,
}

macro_rules! params {
    ($( $idx:ident = $name:literal, $def:expr, $tag:ident, $src:literal; )*) => {
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        /// Index of each scenario parameter in [`MicroParams::values`].
        pub enum P { $( $idx, )* }
        /// Every micro-tier scenario parameter, in index order.
        pub const PARAM_SPECS: &[ParamSpec] = &[
            $( ParamSpec { name: $name, default: $def, tag: Tag::$tag, source: $src }, )*
        ];
    };
}

params! {
    ReachHomeShare = "reach_home_share", 0.40, Frontier, "FSD 4.4 Expected regime; real NAT mix awaits leviculum#100";
    PublicListenerShare = "public_listener_share", 0.30, Model, "of reachable home routers, the share with an inbound listener (IPv6 / forwarded port) rather than punch-only";
    CellularPunchableShare = "cellular_punchable_share", 0.15, Frontier, "direct_path.rs: failed punch = symmetric NAT, 'common on cellular'; no number exists";
    PunchSuccessShare = "punch_success_share", 0.90, Frontier, "two punchable ends still miss sometimes (one-sided punch, leviculum#89); no fleet number";
    ServerTierShare = "server_tier_share", 0.08, Model, "FSD 4.4 Expected regime (share of devices that are hosted servers)";
    RelayOfferingShare = "relay_offering_share", 0.90, Model, "FSD 4.4 Expected regime (owners who leave the relay default on)";
    MobileOfflineShare = "mobile_offline_share", 0.15, Model, "FSD 4.4 Expected regime p_offline (stationary share of time a phone is suspended)";
    PhoneWifiShare = "phone_wifi_share", 0.65, Model, "globe.html device-tier slider default (phone on Wi-Fi vs cellular)";
    PhoneChargingShare = "phone_charging_share", 0.35, Model, "globe.html slider default (share of Wi-Fi time on external power)";
    HomeShapingOptIn = "home_shaping_opt_in", 0.25, Model, "globe.html opt-in slider default (shaping is off by default on home: FLEET_DEFAULTS)";
    PhoneShapingOptIn = "phone_shaping_opt_in", 0.10, Model, "globe.html opt-in slider default (off by default on phones)";
    Adoption = "adoption", 0.62, Model, "globe population / 8.05 B people (the page passes its own)";
    PersonsPerHousehold = "persons_per_household", 2.5, Model, "typical household size; no CIRIS source";
    PhonesPerPerson = "phones_per_person", 1.0, Model, "globe.html DEVICES_PER_PERSON note: ~1 phone per person";
    HomeNodeHouseholdShare = "home_node_household_share", 0.60, Model, "share of adopting households running an always-on home node (NAS/desktop)";
    LaptopsPerPerson = "laptops_per_person", 0.50, Model, "rest of the ~1 home/laptop/tablet per person in DEVICES_PER_PERSON";
    LaptopAwakeShare = "laptop_awake_share", 0.35, Model, "stationary share of time a laptop is open";
    ServeSeats = "serve_seats", 3.0, Model, "M canonicals in the focal region (production runs one; k=2 homing needs M>=2)";
    HomingK = "homing_k", 2.0, Model, "top-k canonical homing, CIRISEdge#935 / TRANSPORT_TIERS_MODEL.md §0.3 (not shipped code)";
    CiArrivalsPerHour = "ci_arrivals_per_hour", 3.0, Model, "QA/CI runners joining the focal seats per hour";
    CiJobMeanS = "ci_job_mean_s", 1200.0, Model, "mean CI job length (boot, join, run, leave)";
    CiOldVersionShare = "ci_old_version_share", 0.30, Model, "share of runners pinned to an older edge (soak / compat jobs)";
    ServerOldVersionShare = "server_old_version_share", 0.15, Model, "hosted/agent servers deliberately kept on an older edge (long-soak peers)";
    PhoneAsleepMeanS = "phone_asleep_mean_s", 900.0, Model, "mean length of one suspension; awake mean follows from mobile_offline_share";
    WifiDwellMeanS = "wifi_dwell_mean_s", 5400.0, Model, "mean stay on Wi-Fi; cellular dwell follows from phone_wifi_share";
    ChargeDwellMeanS = "charge_dwell_mean_s", 3600.0, Model, "mean time on external power";
    LaptopDwellMeanS = "laptop_dwell_mean_s", 2400.0, Model, "mean laptop open period";
    MsgMeanIntervalS = "msg_mean_interval_s", 480.0, Model, "mean gap between rows a person's awake device signs (times diurnal_multiplier)";
    CiMsgIntervalS = "ci_msg_interval_s", 45.0, Model, "mean gap between rows a running CI job sends";
    FamilyShare = "family_share", 0.65, Model, "globe kernel act mix: 65% self/family";
    CommunityShare = "community_share", 0.30, Model, "globe kernel act mix: 30% community room";
    CommunityJoinShare = "community_join_share", 0.50, Model, "share of adopting households in the street's community room";
    LanAutointerface = "lan_autointerface", 1.0, Model, "host enables AutoInterface (LAN discovery; reticulum.rs umbrella feature default); 1 = on";
    PhoneClientOnly = "phone_client_only", 1.0, Sourced, "DeviceState::of(Phone): every v44/v45 phone is client-only (link_shaping.rs)";
    DensitySigma = "density_sigma", 0.9, Model, "lognormal spread of a tile's density around its cell's mean";
    TargetHouseholds = "target_households", 60.0, Model, "households the patch is sized to hold (R5 visual budget)";
    UrbanCoreDensity = "urban_core_density", 6000.0, Model, "people/km2 above which the patch is apartment blocks";
    UrbanDensity = "urban_density", 1500.0, Model, "people/km2 above which the patch is terraced streets";
    SuburbanDensity = "suburban_density", 250.0, Model, "people/km2 above which the patch is suburban; below it a village";
    StartLocalHour = "start_local_hour", 9.0, Model, "local clock at build (the page passes its own)";
}

/// Number of scenario parameters.
pub const PARAM_COUNT: usize = PARAM_SPECS.len();

/// The scenario parameters of one micro world.
#[derive(Debug, Clone, PartialEq)]
pub struct MicroParams {
    /// Values in [`P`] order.
    pub values: [f64; PARAM_COUNT],
}

impl Default for MicroParams {
    fn default() -> Self {
        let mut values = [0.0; PARAM_COUNT];
        for (v, s) in values.iter_mut().zip(PARAM_SPECS) {
            *v = s.default;
        }
        Self { values }
    }
}

impl MicroParams {
    /// Read one parameter.
    #[must_use]
    pub fn get(&self, p: P) -> f64 {
        self.values[p as usize]
    }
    /// Set one parameter.
    pub fn set(&mut self, p: P, v: f64) {
        self.values[p as usize] = v;
    }
    fn share(&self, p: P) -> f64 {
        self.get(p).clamp(0.0, 1.0)
    }
}

// ------------------------------------------------------------------ rng

/// splitmix64: small, seedable, the same in every build.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Seeded generator.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Uniform in [a, b).
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
    /// Bernoulli.
    pub fn chance(&mut self, p: f64) -> bool {
        self.f() < p
    }
    /// Uniform index below `n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    /// Standard normal.
    pub fn gauss(&mut self) -> f64 {
        let u = self.f().max(1e-12);
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * self.f()).cos()
    }
}

/// Probability that a memoryless state with mean dwell `mean_s` ends within `dt` seconds.
#[must_use]
pub fn leave_probability(dt: f64, mean_s: f64) -> f64 {
    if mean_s <= 0.0 {
        1.0
    } else {
        1.0 - (-dt / mean_s).exp()
    }
}

/// Mean dwell of the complementary state of a two-state chain whose stationary share of the
/// first state is `share` and whose first-state dwell is `mean_first`.
#[must_use]
pub fn complementary_dwell(share: f64, mean_first: f64) -> f64 {
    let s = share.clamp(0.001, 0.999);
    mean_first * (1.0 - s) / s
}

// ------------------------------------------------------------------ the place

/// What the patch is, from its density (thresholds are MODEL parameters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DensityClass {
    /// Apartment blocks on a street grid.
    UrbanCore = 0,
    /// Terraced houses on a street grid.
    Urban = 1,
    /// Detached houses on curving streets.
    Suburban = 2,
    /// A main road, a few lanes, scattered houses.
    Village = 3,
}

impl DensityClass {
    /// Classify a density (people per km2).
    #[must_use]
    pub fn of(density_km2: f64, p: &MicroParams) -> Self {
        if density_km2 >= p.get(P::UrbanCoreDensity) {
            Self::UrbanCore
        } else if density_km2 >= p.get(P::UrbanDensity) {
            Self::Urban
        } else if density_km2 >= p.get(P::SuburbanDensity) {
            Self::Suburban
        } else {
            Self::Village
        }
    }
    /// Name for the page.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UrbanCore => "apartment blocks",
            Self::Urban => "terraced streets",
            Self::Suburban => "suburban streets",
            Self::Village => "village",
        }
    }
}

/// One straight road segment (metres, east/north from the patch centre).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Road {
    /// Start east.
    pub x0: f64,
    /// Start north.
    pub y0: f64,
    /// End east.
    pub x1: f64,
    /// End north.
    pub y1: f64,
    /// Carriageway width.
    pub width: f64,
}

/// One building footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Building {
    /// Centre east.
    pub cx: f64,
    /// Centre north.
    pub cy: f64,
    /// Frontage length (along the road).
    pub w: f64,
    /// Depth (away from the road).
    pub d: f64,
    /// Orientation of the frontage (radians from east).
    pub angle: f64,
    /// Storeys.
    pub floors: u8,
    /// Dwellings (0 = barn / shop / garage).
    pub units: u16,
}

/// A data-centre site off the street.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Site {
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lon: f64,
    /// Great-circle distance from the patch centre.
    pub km: f64,
    /// What it hosts.
    pub kind: SiteKind,
}

/// What a site hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SiteKind {
    /// A canonical serve seat's hosting.
    Seat = 0,
    /// Rented hosting for people's own servers.
    Hosting = 1,
    /// A hosted CI provider.
    Ci = 2,
}

/// The router a household sits behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RouterNat {
    /// Inbound listener reachable (IPv6 or forwarded port): a direct path to anyone.
    Public = 0,
    /// NAT that a facilitated UDP punch can traverse.
    Punchable = 1,
    /// No direct path from outside (symmetric NAT / CGNAT).
    Hard = 2,
}

/// One dwelling.
#[derive(Debug, Clone, PartialEq)]
pub struct Household {
    /// Building index.
    pub building: usize,
    /// Runs CIRIS at all.
    pub adopting: bool,
    /// People.
    pub persons: u8,
    /// Its router.
    pub router: RouterNat,
    /// In the street's community room.
    pub in_community: bool,
    /// Its devices.
    pub nodes: Vec<usize>,
    /// Position of the dwelling inside the building.
    pub x: f64,
    /// Position of the dwelling inside the building.
    pub y: f64,
}

// ------------------------------------------------------------------ the nodes

/// The host a node runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum HostKind {
    /// A phone: class follows its network, client-only, suspended by its OS.
    Phone = 0,
    /// An always-on home node (NAS, desktop, mini-PC): `home` class, the member-owned mailbox.
    HomeNode = 1,
    /// A laptop: runs as `home` (platform default) but sleeps with its lid.
    Laptop = 2,
    /// A rented server in a data centre: `server` class, shapes every identity link by default.
    HostedServer = 3,
    /// A canonical serve seat: hosted, seat class to other seats, facilitator and mailbox.
    ServeSeat = 4,
    /// An ephemeral QA / CI runner: boots, homes, runs a job, leaves.
    CiRunner = 5,
}

impl HostKind {
    /// Name for the page.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::HomeNode => "home node",
            Self::Laptop => "laptop",
            Self::HostedServer => "hosted server",
            Self::ServeSeat => "serve seat",
            Self::CiRunner => "CI runner",
        }
    }
    /// Always on (no sleep in this model).
    #[must_use]
    pub const fn always_on(self) -> bool {
        matches!(self, Self::HomeNode | Self::HostedServer | Self::ServeSeat)
    }
    /// Lives in a data centre, not on the street.
    #[must_use]
    pub const fn off_patch(self) -> bool {
        matches!(self, Self::HostedServer | Self::ServeSeat | Self::CiRunner)
    }
}

/// What a node is doing now (for the picture and the detail card).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Activity {
    /// Online, nothing in flight.
    Idle = 0,
    /// Suspended / lid closed: every link closed.
    Asleep = 1,
    /// Resumed, waiting for its re-announce.
    Resuming = 2,
    /// A facilitated punch in progress.
    Punching = 3,
    /// Signed and sent a row just now.
    Sending = 4,
    /// Forwarded someone else's sealed row just now.
    Relaying = 5,
    /// Holding rows for a sleeping member.
    HoldingMail = 6,
    /// CI runner booting and homing.
    Booting = 7,
    /// CI runner running its job.
    RunningJob = 8,
    /// Not present (CI slot free, or a household that does not run CIRIS).
    Gone = 9,
}

/// One device.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Host kind.
    pub kind: HostKind,
    /// Household (on-patch devices and the household's hosted server).
    pub household: Option<usize>,
    /// Owning person's index within the household.
    pub owner: u8,
    /// Data-centre site for off-patch nodes.
    pub site: Option<usize>,
    /// Position (metres east of the patch centre). Off-patch: tangent-plane approximation.
    pub x: f64,
    /// Position (metres north).
    pub y: f64,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lon: f64,
    /// Present in the mesh at all (CI slots come and go).
    pub present: bool,
    /// Awake (not suspended, lid open).
    pub awake: bool,
    /// On an unmetered network (Wi-Fi at home); off = cellular, away from home.
    pub unmetered: bool,
    /// On external power.
    pub charging: bool,
    /// Has an inbound listener (public server, seat, or a home router with one).
    pub public: bool,
    /// Its current network can be punched.
    pub punchable: bool,
    /// Client-only (dials, listens for nothing, no relay slot).
    pub client_only: bool,
    /// The person opted into shaping every identity link at this class.
    pub shaping_opt_in: bool,
    /// The owner left the relay default on.
    pub relay_pref: bool,
    /// Runs an older edge (pre-v44: no direct-path proposals).
    pub old_version: bool,
    /// The k serve seats it homes on (node indices; `NONE` if unused).
    pub homes: [u32; 2],
    /// Time its resume completes (re-announce done).
    pub resume_at: f64,
    /// Time a CI job ends.
    pub leave_at: f64,
    /// Current activity.
    pub activity: Activity,
    /// Until when the activity label holds (transient labels).
    pub activity_until: f64,
    /// Story control: autonomous churn frozen.
    pub pinned: bool,
    /// Device shaped slots in use (bound 8).
    pub shaped_used: u8,
    /// Rows held here for sleeping members.
    pub holding: u32,
    /// Rows waiting for this node at a mailbox.
    pub pending: u32,
    /// Last time an announce refreshed its homing links.
    pub last_announce: f64,
    /// Short label (story names; empty otherwise).
    pub label: &'static str,
}

impl Node {
    /// The budget class the device offers (CC 5.4.3; `DeviceState::budget_class`).
    #[must_use]
    pub fn budget_class(&self) -> BudgetClass {
        match self.kind {
            HostKind::Phone if self.unmetered => BudgetClass::PhoneWifi,
            HostKind::Phone => BudgetClass::PhoneCellular,
            HostKind::HomeNode | HostKind::Laptop | HostKind::CiRunner => BudgetClass::Home,
            HostKind::HostedServer => BudgetClass::Server,
            HostKind::ServeSeat => BudgetClass::Seat,
        }
    }
    /// The class offered to a non-seat peer: a seat travels as `server` (`BudgetClass::wire_byte`).
    #[must_use]
    pub fn offered_to_non_seat(&self) -> BudgetClass {
        match self.budget_class() {
            BudgetClass::Seat => BudgetClass::Server,
            c => c,
        }
    }
    /// Shapes every identity link (scope `AllIdentityLinks`): on by default for server and seat,
    /// otherwise only with the person's opt-in at this class (`FLEET_DEFAULTS`).
    #[must_use]
    pub fn shapes_all(&self) -> bool {
        match self.kind {
            HostKind::HostedServer | HostKind::ServeSeat => true,
            _ => self.shaping_opt_in,
        }
    }
    /// Reachable and accepting rows right now.
    #[must_use]
    pub fn online(&self, t: f64) -> bool {
        self.present && self.awake && self.resume_at <= t
    }
    /// The device relay policy (CC 5.4.6 item 1; `onion/policy.rs` `device_relays`).
    #[must_use]
    pub fn device_relays(&self) -> bool {
        match self.kind {
            HostKind::Phone => self.unmetered && self.charging,
            _ => true,
        }
    }
    /// Offers relay now: forwards, is reachable without the canonical, and is not client-only;
    /// a seat may not opt out (`policy.rs`).
    #[must_use]
    pub fn relay_offering(&self, t: f64) -> bool {
        self.online(t)
            && !self.client_only
            && self.device_relays()
            && (self.kind == HostKind::ServeSeat || self.relay_pref)
            && (self.public || self.punchable)
    }
    /// May hold rows for a sleeping member (a member-owned always-on node; never a phone).
    #[must_use]
    pub fn can_be_mailbox(&self) -> bool {
        matches!(self.kind, HostKind::HomeNode | HostKind::HostedServer)
    }
}

/// What a physical (one-hop) link is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LinkKind {
    /// A node's dial to one of its k homed serve seats.
    Homing = 0,
    /// Seat to seat (the canonicals' full mesh).
    SeatMesh = 1,
    /// Same LAN (AutoInterface).
    Lan = 2,
    /// One end dialled the other's public listener.
    Dialled = 3,
    /// A facilitated UDP punch.
    Punched = 4,
}

impl LinkKind {
    /// Name for the page.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Homing => "homing (to serve seat)",
            Self::SeatMesh => "seat mesh",
            Self::Lan => "LAN",
            Self::Dialled => "dialled (public listener)",
            Self::Punched => "punched direct",
        }
    }
}

/// One physical link: always one hop between its two ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Link {
    /// Lower node index.
    pub a: usize,
    /// Higher node index.
    pub b: usize,
    /// What it is.
    pub kind: LinkKind,
    /// Agreed shaped class (`Off` = runs unshaped).
    pub shaped: BudgetClass,
    /// It asked for shaping and both ends were at their bound: runs unshaped (the ninth link).
    pub at_bound: bool,
    /// One end shapes only because the other asked (dropped 10 min after the last real frame).
    pub asked: bool,
    /// Created at.
    pub since: f64,
    /// Last real frame.
    pub last_frame: f64,
}

/// How one row travelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Route {
    /// Over a one-hop link between the two ends (any kind).
    Direct = 0,
    /// Crossed a forwarder sealed and unshaped (CC 5.4.3).
    Relayed = 1,
    /// Recipient asleep: held at a mailbox.
    Mailboxed = 2,
    /// A held row handed over when its recipient woke.
    MailDelivered = 3,
    /// To the federation (a seat).
    Federation = 4,
}

/// One row in the event ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MsgEvent {
    /// Sim time.
    pub t: f64,
    /// Sender.
    pub src: usize,
    /// Recipient.
    pub dst: usize,
    /// Forwarder or mailbox (`NONE` if none).
    pub via: u32,
    /// Second forwarder (seat to seat), `NONE` if none.
    pub via2: u32,
    /// Route.
    pub route: Route,
    /// Rode a shaped link (the direct hop, or the first hop of a relay).
    pub shaped: bool,
}

/// Running counts (all MODEL outputs).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MicroStats {
    /// Rows signed.
    pub sent: u64,
    /// Delivered over a one-hop link.
    pub direct: u64,
    /// Of `direct`, over a shaped link.
    pub direct_shaped: u64,
    /// Crossed a forwarder.
    pub relayed: u64,
    /// Of `relayed`, the forwarder was a serve seat (no member-owned relay could reach both).
    pub relayed_via_seat: u64,
    /// Held at a mailbox.
    pub mailboxed: u64,
    /// Held rows delivered on wake.
    pub mail_delivered: u64,
    /// Summed hold time of delivered mail.
    pub mail_wait_s: f64,
    /// To the federation.
    pub federation: u64,
    /// Punches proposed.
    pub punch_tried: u64,
    /// Punches that came up.
    pub punch_ok: u64,
    /// Punches that failed.
    pub punch_failed: u64,
    /// Links that asked for shaping but ran unshaped at the bound.
    pub ninth_unshaped: u64,
    /// CI runners joined.
    pub ci_joined: u64,
    /// CI runners left.
    pub ci_left: u64,
    /// Phone suspensions.
    pub suspends: u64,
    /// Phone network changes.
    pub network_changes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Punch {
    a: usize,
    b: usize,
    done_at: f64,
    facilitator: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Held {
    src: usize,
    dst: usize,
    holder: usize,
    since: f64,
}

/// The focal 1:1 world.
#[derive(Debug, Clone)]
pub struct MicroWorld {
    /// Parameters it was built with.
    pub params: MicroParams,
    /// Seed (from the tile).
    pub seed: u64,
    rng: Rng,
    /// Patch centre latitude.
    pub lat0: f64,
    /// Patch centre longitude.
    pub lon0: f64,
    /// People per km2 at the tile.
    pub density_km2: f64,
    /// Layout class.
    pub class: DensityClass,
    /// Half the patch side (metres).
    pub half: f64,
    /// Streets.
    pub roads: Vec<Road>,
    /// Buildings.
    pub buildings: Vec<Building>,
    /// Dwellings.
    pub households: Vec<Household>,
    /// Data-centre sites.
    pub sites: Vec<Site>,
    /// Devices (fixed after build; CI runners reuse their slots).
    pub nodes: Vec<Node>,
    /// Physical links, in creation order.
    pub links: Vec<Link>,
    /// Recent rows.
    pub events: VecDeque<MsgEvent>,
    /// Counts.
    pub stats: MicroStats,
    /// Sim seconds since build.
    pub t: f64,
    /// Generate no background traffic (story acts that follow two people).
    pub quiet: bool,
    punches: Vec<Punch>,
    backoff: BTreeMap<(usize, usize), f64>,
    mail: Vec<Held>,
    link_index: BTreeMap<(usize, usize, LinkKind), usize>,
    /// Index of the CI provider's QA maintainer household's home node (receives test rows).
    pub qa_peer: u32,
}

fn key(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Point at `km` along `bearing` (radians from north) from (lat, lon).
#[must_use]
pub fn destination(lat: f64, lon: f64, bearing: f64, km: f64) -> (f64, f64) {
    let r = 6371.0;
    let d = km / r;
    let (la, lo) = (lat.to_radians(), lon.to_radians());
    let la2 = (la.sin() * d.cos() + la.cos() * d.sin() * bearing.cos()).asin();
    let lo2 = lo + (bearing.sin() * d.sin() * la.cos()).atan2(d.cos() - la.sin() * la2.sin());
    (la2.to_degrees(), ((lo2.to_degrees() + 540.0) % 360.0) - 180.0)
}

fn seg_dist(px: f64, py: f64, r: &Road) -> f64 {
    let (dx, dy) = (r.x1 - r.x0, r.y1 - r.y0);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((px - r.x0) * dx + (py - r.y0) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let (qx, qy) = (r.x0 + t * dx, r.y0 + t * dy);
    ((px - qx).powi(2) + (py - qy).powi(2)).sqrt()
}

fn rect_corners(b: &Building, margin: f64) -> [(f64, f64); 4] {
    let (c, s) = (b.angle.cos(), b.angle.sin());
    let (hw, hd) = (b.w / 2.0 + margin, b.d / 2.0 + margin);
    [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)].map(|(u, v)| (b.cx + c * u - s * v, b.cy + s * u + c * v))
}

/// Separating-axis test for two building footprints (with a gap of `margin` metres).
fn rects_overlap(a: &Building, b: &Building, margin: f64) -> bool {
    let ca = rect_corners(a, margin / 2.0);
    let cb = rect_corners(b, margin / 2.0);
    for ang in [a.angle, a.angle + std::f64::consts::FRAC_PI_2, b.angle, b.angle + std::f64::consts::FRAC_PI_2] {
        let (ax, ay) = (ang.cos(), ang.sin());
        let proj = |pts: &[(f64, f64); 4]| {
            pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                let d = p.0 * ax + p.1 * ay;
                (lo.min(d), hi.max(d))
            })
        };
        let (a0, a1) = proj(&ca);
        let (b0, b1) = proj(&cb);
        if a1 <= b0 || b1 <= a0 {
            return false;
        }
    }
    true
}

/// Seed for a tile of the globe (the page uses the same mix).
#[must_use]
pub fn tile_seed(ti: i32, tj: i32) -> u64 {
    let mut r = Rng::new(((ti as u32 as u64) << 32) ^ (tj as u32 as u64) ^ 0xC1_25_C0DE);
    r.next_u64()
}

impl MicroWorld {
    /// Build the focal world for a tile: `cell_density_km2` is the containing cell's mean density
    /// (people per km2), spread per tile by `density_sigma`.
    #[must_use]
    pub fn build(lat0: f64, lon0: f64, cell_density_km2: f64, seed: u64, params: MicroParams) -> Self {
        let mut rng = Rng::new(seed);
        let spread = (params.get(P::DensitySigma) * rng.gauss()).exp();
        let density_km2 = (cell_density_km2 * spread).clamp(5.0, 60_000.0);
        Self::build_at_density(lat0, lon0, density_km2, seed, params)
    }

    /// Build at an exact density (the story picks its street; tests pin a class).
    #[must_use]
    pub fn build_at_density(lat0: f64, lon0: f64, density_km2: f64, seed: u64, params: MicroParams) -> Self {
        let class = DensityClass::of(density_km2, &params);
        let pph = params.get(P::PersonsPerHousehold).max(1.0);
        let area_km2 = params.get(P::TargetHouseholds).max(4.0) * pph / density_km2.max(1.0);
        let half = (area_km2.sqrt() * 1000.0 / 2.0).clamp(70.0, 650.0);
        let mut w = Self {
            params,
            seed,
            rng: Rng::new(seed.rotate_left(17) ^ 0x51_7E),
            lat0,
            lon0,
            density_km2,
            class,
            half,
            roads: Vec::new(),
            buildings: Vec::new(),
            households: Vec::new(),
            sites: Vec::new(),
            nodes: Vec::new(),
            links: Vec::new(),
            events: VecDeque::new(),
            stats: MicroStats::default(),
            t: 0.0,
            quiet: false,
            punches: Vec::new(),
            backoff: BTreeMap::new(),
            mail: Vec::new(),
            link_index: BTreeMap::new(),
            qa_peer: NONE,
        };
        w.lay_out_streets();
        w.place_buildings();
        w.thin_to_density();
        w.place_sites();
        w.populate();
        w
    }

    // -------------------------------------------------------------- geometry

    fn push_line(&mut self, a: (f64, f64), b: (f64, f64), width: f64) {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (len / 20.0).ceil().max(1.0) as usize;
        let pts: Vec<(f64, f64)> = (0..=n)
            .map(|k| {
                let t = k as f64 / n as f64;
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            })
            .collect();
        self.push_polyline(&pts, width);
    }

    fn push_polyline(&mut self, pts: &[(f64, f64)], width: f64) {
        let lim = self.half + 30.0;
        for p in pts.windows(2) {
            let (a, b) = (p[0], p[1]);
            let out = |q: (f64, f64)| q.0.abs() > lim || q.1.abs() > lim;
            if out(a) && out(b) {
                continue;
            }
            self.roads.push(Road { x0: a.0, y0: a.1, x1: b.0, y1: b.1, width });
        }
    }

    fn lay_out_streets(&mut self) {
        let h = self.half;
        let theta = self.rng.range(0.0, std::f64::consts::PI);
        let (dx, dy) = (theta.cos(), theta.sin());
        let (nx, ny) = (-dy, dx);
        let reach = h * 1.6;
        match self.class {
            DensityClass::UrbanCore | DensityClass::Urban => {
                let block = if self.class == DensityClass::UrbanCore {
                    self.rng.range(78.0, 105.0)
                } else {
                    self.rng.range(62.0, 86.0)
                };
                let block2 = block * self.rng.range(0.9, 1.5);
                let off = self.rng.range(0.0, block);
                let main = self.rng.below(3) as i32 - 1;
                let n = (reach / block).ceil() as i32 + 1;
                for k in -n..=n {
                    let o = k as f64 * block + off - block / 2.0;
                    let width = if k == main { 14.0 } else { 9.0 };
                    self.push_line((o * nx - reach * dx, o * ny - reach * dy), (o * nx + reach * dx, o * ny + reach * dy), width);
                }
                let off2 = self.rng.range(0.0, block2);
                let n2 = (reach / block2).ceil() as i32 + 1;
                for k in -n2..=n2 {
                    let o = k as f64 * block2 + off2 - block2 / 2.0;
                    self.push_line((o * dx - reach * nx, o * dy - reach * ny), (o * dx + reach * nx, o * dy + reach * ny), 9.0);
                }
            }
            DensityClass::Suburban => {
                let spacing = self.rng.range(72.0, 92.0);
                let n = (reach / spacing).ceil() as i32;
                for k in -n..=n {
                    let amp = self.rng.range(6.0, 22.0);
                    let lam = self.rng.range(90.0, 180.0);
                    let ph = self.rng.range(0.0, std::f64::consts::TAU);
                    let base = k as f64 * spacing;
                    let mut pts = Vec::new();
                    let mut s = -reach;
                    while s <= reach {
                        let o = base + amp * (s / lam + ph).sin();
                        pts.push((s * dx + o * nx, s * dy + o * ny));
                        s += 16.0;
                    }
                    self.push_polyline(&pts, 7.0);
                }
                // one collector road across the curving streets
                let c = self.rng.range(-h * 0.3, h * 0.3);
                self.push_line((c * dx - reach * nx, c * dy - reach * ny), (c * dx + reach * nx, c * dy + reach * ny), 10.0);
            }
            DensityClass::Village => {
                let amp = self.rng.range(15.0, 45.0);
                let lam = self.rng.range(160.0, 320.0);
                let mut main = Vec::new();
                let mut s = -reach;
                while s <= reach {
                    let o = amp * (s / lam).sin();
                    main.push((s * dx + o * nx, s * dy + o * ny));
                    s += 20.0;
                }
                self.push_polyline(&main, 7.5);
                let lanes = 2 + self.rng.below(3);
                for _ in 0..lanes {
                    let at = self.rng.range(-h * 0.7, h * 0.7);
                    let o = amp * (at / lam).sin();
                    let (sx, sy) = (at * dx + o * nx, at * dy + o * ny);
                    let side = if self.rng.chance(0.5) { 1.0 } else { -1.0 };
                    let turn = self.rng.range(0.9, 1.6) * side;
                    let (lx, ly) = ((theta + turn).cos(), (theta + turn).sin());
                    let len = self.rng.range(h * 0.4, h * 1.1);
                    let bend = self.rng.range(-0.4, 0.4);
                    let mut pts = vec![(sx, sy)];
                    let mut d = 0.0;
                    let (mut px, mut py, mut a) = (sx, sy, ly.atan2(lx));
                    while d < len {
                        a += bend * 16.0 / len;
                        px += 16.0 * a.cos();
                        py += 16.0 * a.sin();
                        pts.push((px, py));
                        d += 16.0;
                    }
                    self.push_polyline(&pts, 4.5);
                }
            }
        }
    }

    fn building_fits(&self, cx: f64, cy: f64, w: f64, d: f64, angle: f64) -> bool {
        let lim = self.half - 4.0;
        if cx.abs() > lim || cy.abs() > lim {
            return false;
        }
        for r2 in &self.roads {
            if seg_dist(cx, cy, r2) < r2.width / 2.0 + 0.5 * w.min(d) + 1.0 {
                return false;
            }
        }
        let me = Building { cx, cy, w, d, angle, floors: 1, units: 0 };
        !self.buildings.iter().any(|b| rects_overlap(&me, b, 0.6))
    }

    fn place_buildings(&mut self) {
        let roads = self.roads.clone();
        // (frontage, depth, floors, units per floor, spacing, setback)
        for r in &roads {
            let (dx, dy) = (r.x1 - r.x0, r.y1 - r.y0);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1.0 {
                continue;
            }
            let (ux, uy) = (dx / len, dy / len);
            let angle = uy.atan2(ux);
            for side in [-1.0f64, 1.0] {
                let mut s = self.rng.range(2.0, 10.0);
                while s < len {
                    let (w, d, floors, units, gap, setback) = match self.class {
                        DensityClass::UrbanCore => {
                            let w = self.rng.range(16.0, 28.0);
                            let floors = 4 + self.rng.below(4) as u8;
                            let per = (w / 9.0).round().max(1.0) as u16;
                            (w, self.rng.range(11.0, 15.0), floors, per * floors as u16, self.rng.range(0.5, 3.0), 1.5)
                        }
                        DensityClass::Urban => {
                            let floors = 2 + self.rng.below(2) as u8;
                            (self.rng.range(6.5, 9.0), self.rng.range(8.5, 11.0), floors, 1 + self.rng.below(2) as u16, 0.4, 3.0)
                        }
                        DensityClass::Suburban => {
                            (self.rng.range(9.5, 13.0), self.rng.range(8.5, 12.0), 1 + self.rng.below(2) as u8, 1, self.rng.range(9.0, 15.0), self.rng.range(7.0, 11.0))
                        }
                        DensityClass::Village => {
                            let barn = self.rng.chance(0.18);
                            if barn {
                                (self.rng.range(14.0, 22.0), self.rng.range(10.0, 14.0), 1, 0, self.rng.range(12.0, 40.0), self.rng.range(8.0, 14.0))
                            } else {
                                (self.rng.range(8.5, 13.0), self.rng.range(8.0, 11.0), 1 + self.rng.below(2) as u8, 1, self.rng.range(14.0, 50.0), self.rng.range(6.0, 13.0))
                            }
                        }
                    };
                    let mid = s + w / 2.0;
                    if mid > len {
                        break;
                    }
                    let off = r.width / 2.0 + setback + d / 2.0;
                    let (px, py) = (r.x0 + ux * mid, r.y0 + uy * mid);
                    let (cx, cy) = (px - uy * off * side, py + ux * off * side);
                    if self.building_fits(cx, cy, w, d, angle) {
                        self.buildings.push(Building { cx, cy, w, d, angle, floors, units });
                    }
                    s += w + gap;
                }
            }
        }
    }

    /// The street geometry packs dwellings at its own rate; the tile's density decides how many
    /// there are. Flats lose dwellings per building; houses leave vacant lots.
    fn thin_to_density(&mut self) {
        let pph = self.params.get(P::PersonsPerHousehold).max(1.0);
        let area_km2 = (2.0 * self.half / 1000.0).powi(2);
        let want = (self.density_km2 * area_km2 / pph).max(3.0);
        let have: f64 = self.buildings.iter().map(|b| f64::from(b.units)).sum();
        if have <= want * 1.1 {
            return;
        }
        let keep = want / have;
        match self.class {
            DensityClass::UrbanCore | DensityClass::Urban => {
                for b in &mut self.buildings {
                    if b.units > 0 {
                        b.units = ((f64::from(b.units) * keep).round() as u16).max(1);
                    }
                }
                // terraces that are still too many: every dropped one becomes a garage / shop
                let mut total: f64 = self.buildings.iter().map(|b| f64::from(b.units)).sum();
                let mut i = 0;
                while total > want * 1.1 && i < self.buildings.len() * 4 {
                    let j = self.rng.below(self.buildings.len());
                    if self.buildings[j].units > 0 {
                        self.buildings[j].units -= 1;
                        total -= 1.0;
                    }
                    i += 1;
                }
            }
            DensityClass::Suburban | DensityClass::Village => {
                let mut kept = Vec::with_capacity(self.buildings.len());
                for b in std::mem::take(&mut self.buildings) {
                    if b.units == 0 || self.rng.chance(keep) {
                        kept.push(b);
                    }
                }
                self.buildings = kept;
            }
        }
    }

    fn place_sites(&mut self) {
        let m = self.params.get(P::ServeSeats).round().clamp(1.0, 9.0) as usize;
        for _ in 0..m {
            let km = self.rng.range(120.0, 650.0);
            let b = self.rng.range(0.0, std::f64::consts::TAU);
            let (lat, lon) = destination(self.lat0, self.lon0, b, km);
            self.sites.push(Site { lat, lon, km, kind: SiteKind::Seat });
        }
        for _ in 0..2 {
            let km = self.rng.range(40.0, 900.0);
            let b = self.rng.range(0.0, std::f64::consts::TAU);
            let (lat, lon) = destination(self.lat0, self.lon0, b, km);
            self.sites.push(Site { lat, lon, km, kind: SiteKind::Hosting });
        }
        let km = self.rng.range(1_500.0, 7_500.0);
        let b = self.rng.range(0.0, std::f64::consts::TAU);
        let (lat, lon) = destination(self.lat0, self.lon0, b, km);
        self.sites.push(Site { lat, lon, km, kind: SiteKind::Ci });
    }

    fn local_xy(&self, lat: f64, lon: f64) -> (f64, f64) {
        let k = 111_320.0;
        ((lon - self.lon0) * k * self.lat0.to_radians().cos(), (lat - self.lat0) * k)
    }

    fn latlon_of(&self, x: f64, y: f64) -> (f64, f64) {
        let k = 111_320.0;
        (self.lat0 + y / k, self.lon0 + x / (k * self.lat0.to_radians().cos().max(0.05)))
    }

    fn new_node(&self, kind: HostKind) -> Node {
        Node {
            kind,
            household: None,
            owner: 0,
            site: None,
            x: 0.0,
            y: 0.0,
            lat: self.lat0,
            lon: self.lon0,
            present: true,
            awake: true,
            unmetered: true,
            charging: true,
            public: false,
            punchable: false,
            client_only: false,
            shaping_opt_in: false,
            relay_pref: true,
            old_version: false,
            homes: [NONE; 2],
            resume_at: 0.0,
            leave_at: 0.0,
            activity: Activity::Idle,
            activity_until: 0.0,
            pinned: false,
            shaped_used: 0,
            holding: 0,
            pending: 0,
            last_announce: 0.0,
            label: "",
        }
    }

    fn place_at_site(&mut self, n: &mut Node, site: usize) {
        let s = self.sites[site];
        let b = self.rng.range(0.0, std::f64::consts::TAU);
        let (lat, lon) = destination(s.lat, s.lon, b, self.rng.range(0.2, 3.0));
        n.lat = lat;
        n.lon = lon;
        let (x, y) = self.local_xy(lat, lon);
        n.x = x;
        n.y = y;
        n.site = Some(site);
        n.public = true;
        n.punchable = true;
    }

    fn populate(&mut self) {
        let p = self.params.clone();
        // serve seats first: always on, public, relay mandatory
        let seat_sites: Vec<usize> = (0..self.sites.len()).filter(|&i| self.sites[i].kind == SiteKind::Seat).collect();
        for &s in &seat_sites {
            let mut n = self.new_node(HostKind::ServeSeat);
            self.place_at_site(&mut n, s);
            self.nodes.push(n);
        }
        let hosting: Vec<usize> = (0..self.sites.len()).filter(|&i| self.sites[i].kind != SiteKind::Ci).collect();
        let budget = R5_MAX_NODES - CI_SLOTS - self.nodes.len();
        let mut used = 0usize;
        let order: Vec<usize> = (0..self.buildings.len()).collect();
        for bi in order {
            let b = self.buildings[bi];
            for u in 0..b.units {
                let adopting = self.rng.chance(p.share(P::Adoption));
                let persons = (p.get(P::PersonsPerHousehold) + 1.1 * self.rng.gauss()).round().clamp(1.0, 6.0) as u8;
                let router = if self.rng.chance(p.share(P::ReachHomeShare)) {
                    if self.rng.chance(p.share(P::PublicListenerShare)) {
                        RouterNat::Public
                    } else {
                        RouterNat::Punchable
                    }
                } else {
                    RouterNat::Hard
                };
                // dwellings spread through the footprint (flats stack: spread along the frontage)
                let (ca, sa) = (b.angle.cos(), b.angle.sin());
                let fu = if b.units > 1 { (u as f64 + 0.5) / b.units as f64 - 0.5 } else { 0.0 };
                let along = fu * b.w * 0.8 + self.rng.range(-1.0, 1.0);
                let across = self.rng.range(-b.d * 0.25, b.d * 0.25);
                let (x, y) = (b.cx + ca * along - sa * across, b.cy + sa * along + ca * across);
                let mut hh = Household {
                    building: bi,
                    adopting,
                    persons,
                    router,
                    in_community: false,
                    nodes: Vec::new(),
                    x,
                    y,
                };
                if adopting {
                    hh.in_community = self.rng.chance(p.share(P::CommunityJoinShare));
                    let mut kinds: Vec<(HostKind, u8)> = Vec::new();
                    for person in 0..persons {
                        let phones = p.get(P::PhonesPerPerson).floor() as usize
                            + usize::from(self.rng.chance(p.get(P::PhonesPerPerson).fract()));
                        for _ in 0..phones {
                            kinds.push((HostKind::Phone, person));
                        }
                        if self.rng.chance(p.share(P::LaptopsPerPerson)) {
                            kinds.push((HostKind::Laptop, person));
                        }
                    }
                    if self.rng.chance(p.share(P::HomeNodeHouseholdShare)) {
                        kinds.push((HostKind::HomeNode, 0));
                    }
                    let server_p = (p.get(P::ServerTierShare) * 2.25 * f64::from(persons)).clamp(0.0, 0.9);
                    if self.rng.chance(server_p) {
                        kinds.push((HostKind::HostedServer, 0));
                    }
                    if used + kinds.len() > budget {
                        hh.adopting = false;
                        kinds.clear();
                    }
                    let hi = self.households.len();
                    for (kind, owner) in kinds {
                        let mut n = self.new_node(kind);
                        n.household = Some(hi);
                        n.owner = owner;
                        n.relay_pref = self.rng.chance(p.share(P::RelayOfferingShare));
                        match kind {
                            HostKind::HostedServer => {
                                let site = hosting[self.rng.below(hosting.len())];
                                self.place_at_site(&mut n, site);
                                n.old_version = self.rng.chance(p.share(P::ServerOldVersionShare));
                            }
                            _ => {
                                n.x = x + self.rng.range(-2.5, 2.5);
                                n.y = y + self.rng.range(-2.5, 2.5);
                                let (la, lo) = self.latlon_of(n.x, n.y);
                                n.lat = la;
                                n.lon = lo;
                                n.public = router == RouterNat::Public;
                                n.punchable = router != RouterNat::Hard;
                            }
                        }
                        match kind {
                            HostKind::Phone => {
                                n.client_only = p.get(P::PhoneClientOnly) >= 0.5;
                                n.shaping_opt_in = self.rng.chance(p.share(P::PhoneShapingOptIn));
                                n.unmetered = self.rng.chance(p.share(P::PhoneWifiShare));
                                n.charging = n.unmetered && self.rng.chance(p.share(P::PhoneChargingShare));
                                n.awake = !self.rng.chance(p.share(P::MobileOfflineShare));
                                if !n.unmetered {
                                    self.go_out(&mut n);
                                }
                            }
                            HostKind::Laptop => {
                                n.shaping_opt_in = self.rng.chance(p.share(P::HomeShapingOptIn));
                                n.awake = self.rng.chance(p.share(P::LaptopAwakeShare));
                            }
                            HostKind::HomeNode => {
                                n.shaping_opt_in = self.rng.chance(p.share(P::HomeShapingOptIn));
                            }
                            _ => {}
                        }
                        if !n.awake {
                            n.activity = Activity::Asleep;
                        }
                        hh.nodes.push(self.nodes.len());
                        self.nodes.push(n);
                        used += 1;
                    }
                }
                self.households.push(hh);
            }
        }
        // CI slots: empty until a runner arrives
        let ci_site = self.sites.iter().position(|s| s.kind == SiteKind::Ci).unwrap_or(0);
        for _ in 0..CI_SLOTS {
            let mut n = self.new_node(HostKind::CiRunner);
            self.place_at_site(&mut n, ci_site);
            n.public = false; // a hosted runner dials out; no inbound listener (MODEL)
            n.punchable = false;
            n.present = false;
            n.awake = false;
            n.activity = Activity::Gone;
            self.nodes.push(n);
        }
        // homing: top k seats by H(node ‖ seat ‖ period), period 0
        let seats = self.seat_ids();
        let k = (self.params.get(P::HomingK).round() as usize).clamp(1, 2).min(seats.len());
        for i in 0..self.nodes.len() {
            self.rehome(i, &seats, k);
        }
        // the QA maintainer: a household with a home node receives the CI jobs' test rows
        self.qa_peer = self
            .nodes
            .iter()
            .position(|n| n.kind == HostKind::HomeNode)
            .map_or(NONE, |i| i as u32);
        for i in 0..self.nodes.len() {
            let n = &mut self.nodes[i];
            n.last_announce = -(i as f64 % ANNOUNCE_INTERVAL_S);
        }
    }

    fn rehome(&mut self, i: usize, seats: &[usize], k: usize) {
        if self.nodes[i].kind == HostKind::ServeSeat {
            return;
        }
        let mut ranked: Vec<(u64, usize)> = seats
            .iter()
            .map(|&s| {
                let mut r = Rng::new(self.seed ^ ((i as u64) << 20) ^ (s as u64).wrapping_mul(0x9E37));
                (r.next_u64(), s)
            })
            .collect();
        ranked.sort_unstable();
        let mut homes = [NONE; 2];
        for (j, (_, s)) in ranked.iter().take(k).enumerate() {
            homes[j] = *s as u32;
        }
        self.nodes[i].homes = homes;
    }

    /// The serve seats.
    #[must_use]
    pub fn seat_ids(&self) -> Vec<usize> {
        (0..self.nodes.len()).filter(|&i| self.nodes[i].kind == HostKind::ServeSeat).collect()
    }

    fn go_out(&mut self, n: &mut Node) {
        // left home: out on a street of the patch, on the carrier's network
        if let Some(r) = self.roads.get(self.rng.below(self.roads.len().max(1))).copied() {
            let t = self.rng.f();
            n.x = (r.x0 + (r.x1 - r.x0) * t).clamp(-self.half, self.half);
            n.y = (r.y0 + (r.y1 - r.y0) * t).clamp(-self.half, self.half);
        }
        n.charging = false;
        n.public = false;
        n.punchable = self.rng.chance(self.params.share(P::CellularPunchableShare));
    }

    fn come_home(&mut self, i: usize) {
        let Some(h) = self.nodes[i].household else { return };
        let (x, y, router) = (self.households[h].x, self.households[h].y, self.households[h].router);
        let n = &mut self.nodes[i];
        n.x = x + self.rng.range(-2.5, 2.5);
        n.y = y + self.rng.range(-2.5, 2.5);
        n.public = router == RouterNat::Public;
        n.punchable = router != RouterNat::Hard;
    }

    // -------------------------------------------------------------- relations

    /// Same household (self/family room), including the household's hosted server.
    #[must_use]
    pub fn family(&self, a: usize, b: usize) -> bool {
        a != b && self.nodes[a].household.is_some() && self.nodes[a].household == self.nodes[b].household
    }

    /// Both in the street's community room (different households).
    #[must_use]
    pub fn community(&self, a: usize, b: usize) -> bool {
        match (self.nodes[a].household, self.nodes[b].household) {
            (Some(x), Some(y)) => x != y && self.households[x].in_community && self.households[y].in_community,
            _ => false,
        }
    }

    /// Room peers (direct-path proposals go only to room peers: `DirectAccept::RoomPeers`).
    #[must_use]
    pub fn room_peers(&self, a: usize, b: usize) -> bool {
        self.family(a, b) || self.community(a, b) || self.is_qa_pair(a, b)
    }

    fn is_qa_pair(&self, a: usize, b: usize) -> bool {
        let q = self.qa_peer as usize;
        (self.nodes[a].kind == HostKind::CiRunner && b == q) || (self.nodes[b].kind == HostKind::CiRunner && a == q)
    }

    fn on_home_lan(&self, i: usize) -> bool {
        let n = &self.nodes[i];
        n.household.is_some() && !n.kind.off_patch() && (n.kind != HostKind::Phone || n.unmetered)
    }

    /// The direct path the pair could use without a punch, if any (CIRIS_MESH.md "Direct path").
    #[must_use]
    pub fn unpunched_direct(&self, a: usize, b: usize) -> Option<LinkKind> {
        let (na, nb) = (&self.nodes[a], &self.nodes[b]);
        if self.params.get(P::LanAutointerface) >= 0.5 && self.family(a, b) && self.on_home_lan(a) && self.on_home_lan(b) {
            return Some(LinkKind::Lan);
        }
        // one end's public listener: the other dials it (a client-only node dials; it does not listen)
        if (nb.public && !nb.client_only) || (na.public && !na.client_only) {
            return Some(LinkKind::Dialled);
        }
        None
    }

    /// The pair could bring up a punched path (both v44+, both on punchable networks).
    #[must_use]
    pub fn punch_possible(&self, a: usize, b: usize) -> bool {
        let (na, nb) = (&self.nodes[a], &self.nodes[b]);
        na.punchable && nb.punchable && !na.old_version && !nb.old_version
    }

    /// The physical link between two nodes, if one is up.
    #[must_use]
    pub fn link_between(&self, a: usize, b: usize) -> Option<&Link> {
        let (x, y) = key(a, b);
        [LinkKind::Lan, LinkKind::Punched, LinkKind::Dialled, LinkKind::Homing, LinkKind::SeatMesh]
            .iter()
            .find_map(|k| self.link_index.get(&(x, y, *k)).map(|&i| &self.links[i]))
    }

    fn has_link(&self, a: usize, b: usize, kind: LinkKind) -> bool {
        let (x, y) = key(a, b);
        self.link_index.contains_key(&(x, y, kind))
    }

    // -------------------------------------------------------------- links

    fn rebuild_index(&mut self) {
        self.link_index.clear();
        for (i, l) in self.links.iter().enumerate() {
            self.link_index.insert((l.a, l.b, l.kind), i);
        }
    }

    fn open_link(&mut self, a: usize, b: usize, kind: LinkKind) -> usize {
        let (x, y) = key(a, b);
        if let Some(&i) = self.link_index.get(&(x, y, kind)) {
            self.links[i].last_frame = self.t;
            return i;
        }
        let mut l = Link { a: x, b: y, kind, shaped: BudgetClass::Off, at_bound: false, asked: false, since: self.t, last_frame: self.t };
        self.negotiate(&mut l);
        self.links.push(l);
        let i = self.links.len() - 1;
        self.link_index.insert((x, y, kind), i);
        i
    }

    /// The shaping a link gets when it comes up (CC 5.4.3): shaped when one end shapes every
    /// identity link (the other accepts when asked), at the lower of the two offers; within the
    /// bound of eight on each device-tier end; seat to seat at the seat class outside the eight.
    fn negotiate(&mut self, l: &mut Link) {
        let (na, nb) = (&self.nodes[l.a], &self.nodes[l.b]);
        if na.kind == HostKind::ServeSeat && nb.kind == HostKind::ServeSeat {
            l.shaped = BudgetClass::Seat;
            return;
        }
        if !(na.shapes_all() || nb.shapes_all()) {
            return;
        }
        // one end a seat, the other not: the seat travels as `server` (both-seat returned above)
        let ca = na.offered_to_non_seat();
        let cb = nb.offered_to_non_seat();
        let class = BudgetClass::agree(ca, cb, false);
        if class == BudgetClass::Off {
            return;
        }
        let bound = SHAPED_LINK_BOUND as u8;
        if na.shaped_used >= bound || nb.shaped_used >= bound {
            l.at_bound = true;
            self.stats.ninth_unshaped += 1;
            return;
        }
        l.shaped = class;
        l.asked = !(na.shapes_all() && nb.shapes_all());
        self.nodes[l.a].shaped_used += 1;
        self.nodes[l.b].shaped_used += 1;
    }

    fn release(&mut self, l: &Link) {
        if l.shaped != BudgetClass::Off && l.shaped != BudgetClass::Seat {
            self.nodes[l.a].shaped_used = self.nodes[l.a].shaped_used.saturating_sub(1);
            self.nodes[l.b].shaped_used = self.nodes[l.b].shaped_used.saturating_sub(1);
        }
    }

    fn close_links_of(&mut self, i: usize, keep_homing: bool) {
        let mut kept = Vec::with_capacity(self.links.len());
        let links = std::mem::take(&mut self.links);
        for l in links {
            if (l.a == i || l.b == i) && !(keep_homing && l.kind == LinkKind::Homing) {
                self.release(&l);
            } else {
                kept.push(l);
            }
        }
        self.links = kept;
        self.punches.retain(|p| p.a != i && p.b != i);
        self.rebuild_index();
    }

    fn maintain_links(&mut self) {
        let t = self.t;
        // drop links whose ends went offline, idle peer links, asked shaping past its TTL
        let links = std::mem::take(&mut self.links);
        let mut kept = Vec::with_capacity(links.len());
        for mut l in links {
            let up = self.nodes[l.a].online(t) && self.nodes[l.b].online(t);
            let idle = t - l.last_frame;
            let peer = matches!(l.kind, LinkKind::Lan | LinkKind::Dialled | LinkKind::Punched);
            let still_direct = match l.kind {
                LinkKind::Lan => self.unpunched_direct(l.a, l.b) == Some(LinkKind::Lan),
                LinkKind::Dialled => self.unpunched_direct(l.a, l.b).is_some(),
                LinkKind::Punched => self.nodes[l.a].punchable && self.nodes[l.b].punchable,
                _ => true,
            };
            if !up || !still_direct || (peer && idle > LINK_POOL_IDLE_BOUND_S) {
                self.release(&l);
                continue;
            }
            if l.asked && idle >= SHAPED_ASK_IDLE_TIMEOUT_SECS as f64 {
                self.release(&l);
                l.shaped = BudgetClass::Off;
                l.asked = false;
            }
            kept.push(l);
        }
        self.links = kept;
        self.rebuild_index();
        // seats: full mesh
        let seats = self.seat_ids();
        for (x, &a) in seats.iter().enumerate() {
            for &b in &seats[x + 1..] {
                let i = self.open_link(a, b, LinkKind::SeatMesh);
                self.links[i].last_frame = t;
            }
        }
        // every online node keeps its homing links (announces refresh them)
        for i in 0..self.nodes.len() {
            if !self.nodes[i].online(t) || self.nodes[i].kind == HostKind::ServeSeat {
                continue;
            }
            let announce = t - self.nodes[i].last_announce >= ANNOUNCE_INTERVAL_S;
            if announce {
                self.nodes[i].last_announce = t;
            }
            for h in self.nodes[i].homes {
                if h == NONE {
                    continue;
                }
                let s = h as usize;
                if !self.has_link(i, s, LinkKind::Homing) {
                    self.open_link(i, s, LinkKind::Homing);
                } else if announce {
                    let (x, y) = key(i, s);
                    if let Some(&li) = self.link_index.get(&(x, y, LinkKind::Homing)) {
                        self.links[li].last_frame = t;
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------- churn

    fn set_activity(&mut self, i: usize, a: Activity, for_s: f64) {
        let n = &mut self.nodes[i];
        if matches!(n.activity, Activity::Asleep | Activity::Gone | Activity::Booting) && !matches!(a, Activity::Asleep | Activity::Gone | Activity::Booting | Activity::Resuming | Activity::Idle | Activity::RunningJob) {
            return;
        }
        n.activity = a;
        n.activity_until = self.t + for_s;
    }

    fn suspend(&mut self, i: usize) {
        self.nodes[i].awake = false;
        self.nodes[i].activity = Activity::Asleep;
        self.stats.suspends += u64::from(self.nodes[i].kind == HostKind::Phone);
        self.close_links_of(i, false);
    }

    fn resume(&mut self, i: usize) {
        let t = self.t;
        let n = &mut self.nodes[i];
        n.awake = true;
        n.resume_at = t + RESUME_ANNOUNCE_WAIT_S;
        n.last_announce = t;
        n.activity = Activity::Resuming;
        n.activity_until = n.resume_at;
    }

    fn change_network(&mut self, i: usize, unmetered: bool) {
        if self.nodes[i].unmetered == unmetered {
            return;
        }
        self.stats.network_changes += 1;
        self.nodes[i].unmetered = unmetered;
        if unmetered {
            self.come_home(i);
        } else {
            let mut n = self.nodes[i].clone();
            self.go_out(&mut n);
            self.nodes[i] = n;
        }
        // the old paths are gone; the homing links re-dial at the new class
        self.close_links_of(i, false);
    }

    fn churn(&mut self, dt: f64) {
        let p = self.params.clone();
        let t = self.t;
        let asleep_mean = p.get(P::PhoneAsleepMeanS);
        let awake_mean = complementary_dwell(p.share(P::MobileOfflineShare), asleep_mean);
        let wifi_mean = p.get(P::WifiDwellMeanS);
        let cell_mean = complementary_dwell(p.share(P::PhoneWifiShare), wifi_mean).max(1.0);
        let cell_mean = if p.share(P::PhoneWifiShare) <= 0.001 { f64::INFINITY } else { cell_mean };
        let chg_mean = p.get(P::ChargeDwellMeanS);
        let unchg_mean = complementary_dwell(p.share(P::PhoneChargingShare), chg_mean);
        let lap_mean = p.get(P::LaptopDwellMeanS);
        let lap_closed = complementary_dwell(p.share(P::LaptopAwakeShare), lap_mean);
        for i in 0..self.nodes.len() {
            if self.nodes[i].pinned {
                continue;
            }
            let kind = self.nodes[i].kind;
            match kind {
                HostKind::Phone => {
                    let awake = self.nodes[i].awake;
                    if awake && self.rng.chance(leave_probability(dt, awake_mean)) {
                        self.suspend(i);
                    } else if !awake && self.rng.chance(leave_probability(dt, asleep_mean)) {
                        self.resume(i);
                    }
                    let um = self.nodes[i].unmetered;
                    let mean = if um { wifi_mean } else { cell_mean };
                    if self.rng.chance(leave_probability(dt, mean)) {
                        self.change_network(i, !um);
                    }
                    if self.nodes[i].unmetered {
                        let c = self.nodes[i].charging;
                        if self.rng.chance(leave_probability(dt, if c { chg_mean } else { unchg_mean })) {
                            self.nodes[i].charging = !c;
                        }
                    }
                }
                HostKind::Laptop => {
                    let awake = self.nodes[i].awake;
                    if awake && self.rng.chance(leave_probability(dt, lap_mean)) {
                        self.suspend(i);
                    } else if !awake && self.rng.chance(leave_probability(dt, lap_closed)) {
                        self.resume(i);
                    }
                }
                HostKind::CiRunner => {
                    let n = &self.nodes[i];
                    if n.present && t >= n.leave_at {
                        self.nodes[i].present = false;
                        self.nodes[i].awake = false;
                        self.nodes[i].activity = Activity::Gone;
                        self.stats.ci_left += 1;
                        self.close_links_of(i, false);
                    } else if n.present && n.activity == Activity::Booting && n.resume_at <= t {
                        self.nodes[i].activity = Activity::RunningJob;
                    }
                }
                _ => {}
            }
        }
        // CI arrivals
        let rate = p.get(P::CiArrivalsPerHour).max(0.0) / 3600.0;
        if self.rng.chance(1.0 - (-rate * dt).exp()) {
            self.spawn_ci(None);
        }
    }

    /// Bring a CI runner up in a free slot (returns its node index).
    pub fn spawn_ci(&mut self, old_version: Option<bool>) -> Option<usize> {
        let i = (0..self.nodes.len()).find(|&i| self.nodes[i].kind == HostKind::CiRunner && !self.nodes[i].present)?;
        let job = self.params.get(P::CiJobMeanS).max(60.0) * self.rng.range(0.5, 1.5);
        let old = old_version.unwrap_or_else(|| self.rng.chance(self.params.share(P::CiOldVersionShare)));
        let seats = self.seat_ids();
        let k = (self.params.get(P::HomingK).round() as usize).clamp(1, 2).min(seats.len());
        let t = self.t;
        {
            let n = &mut self.nodes[i];
            n.present = true;
            n.awake = true;
            n.old_version = old;
            n.resume_at = t + RESUME_ANNOUNCE_WAIT_S;
            n.leave_at = t + job;
            n.activity = Activity::Booting;
            n.activity_until = n.resume_at;
        }
        // a fresh runner is a fresh identity: it homes by its own hash, per run
        self.seed = self.seed.wrapping_add(0x9E37_79B9);
        self.rehome(i, &seats, k);
        self.stats.ci_joined += 1;
        Some(i)
    }

    // -------------------------------------------------------------- traffic

    fn pick_destination(&mut self, src: usize) -> Option<usize> {
        let p = self.params.clone();
        let n = &self.nodes[src];
        if n.kind == HostKind::CiRunner {
            if self.qa_peer != NONE && self.rng.chance(0.5) {
                return Some(self.qa_peer as usize);
            }
            let h = n.homes[0];
            return (h != NONE).then_some(h as usize);
        }
        let hh = n.household?;
        let roll = self.rng.f();
        if roll < p.share(P::FamilyShare) {
            let fam: Vec<usize> = self.households[hh].nodes.iter().copied().filter(|&j| j != src).collect();
            if fam.is_empty() {
                return None;
            }
            return Some(fam[self.rng.below(fam.len())]);
        }
        if roll < p.share(P::FamilyShare) + p.share(P::CommunityShare) {
            if !self.households[hh].in_community {
                return None;
            }
            let others: Vec<usize> = (0..self.households.len()).filter(|&h| h != hh && self.households[h].in_community && !self.households[h].nodes.is_empty()).collect();
            if others.is_empty() {
                return None;
            }
            let h = others[self.rng.below(others.len())];
            let devs = &self.households[h].nodes;
            return Some(devs[self.rng.below(devs.len())]);
        }
        let h = n.homes[0];
        (h != NONE).then_some(h as usize)
    }

    /// The mailbox that holds rows for `dst` while it sleeps: a member-owned always-on node of its
    /// household that is online, else its first homed seat (CC 5.4.6 item 6).
    #[must_use]
    pub fn mailbox_for(&self, dst: usize) -> Option<usize> {
        if let Some(h) = self.nodes[dst].household {
            if let Some(&m) = self.households[h]
                .nodes
                .iter()
                .find(|&&j| j != dst && self.nodes[j].can_be_mailbox() && self.nodes[j].online(self.t))
            {
                return Some(m);
            }
        }
        let s = self.nodes[dst].homes[0];
        (s != NONE).then_some(s as usize)
    }

    /// The forwarder a pair with no direct path crosses: a relay-offering member-owned node with a
    /// direct path to both ends (the sender's household first), else a serve seat they both home
    /// on, else the sender's seat to the recipient's seat.
    #[must_use]
    pub fn forwarder(&self, a: usize, b: usize) -> (u32, u32) {
        let t = self.t;
        let mut cands: Vec<usize> = Vec::new();
        for h in [self.nodes[a].household, self.nodes[b].household].into_iter().flatten() {
            for &f in &self.households[h].nodes {
                if f != a && f != b && self.nodes[f].relay_offering(t) {
                    cands.push(f);
                }
            }
        }
        for f in cands {
            let reach = |x: usize| self.link_between(x, f).is_some() || self.unpunched_direct(x, f).is_some();
            if reach(a) && reach(b) {
                return (f as u32, NONE);
            }
        }
        let ha = self.nodes[a].homes;
        let hb = self.nodes[b].homes;
        if self.nodes[a].kind == HostKind::ServeSeat {
            return (a as u32, NONE);
        }
        for s in ha {
            if s != NONE && (hb.contains(&s) || b == s as usize) {
                return (s, NONE);
            }
        }
        (ha[0], hb[0])
    }

    fn record(&mut self, ev: MsgEvent) {
        if self.events.len() >= EVENT_RING {
            self.events.pop_front();
        }
        self.events.push_back(ev);
    }

    /// Sign and send one row from `src` to `dst` now, by whatever path the rules give.
    pub fn send(&mut self, src: usize, dst: usize) -> Route {
        let t = self.t;
        self.stats.sent += 1;
        self.set_activity(src, Activity::Sending, 4.0);
        if !self.nodes[dst].online(t) {
            let holder = self.mailbox_for(dst).unwrap_or(dst);
            self.mail.push(Held { src, dst, holder, since: t });
            self.nodes[holder].holding += 1;
            self.nodes[dst].pending += 1;
            self.stats.mailboxed += 1;
            // the row still travels to the mailbox
            let shaped = self.carry(src, holder);
            if self.nodes[holder].kind != HostKind::ServeSeat {
                self.set_activity(holder, Activity::HoldingMail, 1e9);
            }
            self.record(MsgEvent { t, src, dst, via: holder as u32, via2: NONE, route: Route::Mailboxed, shaped });
            return Route::Mailboxed;
        }
        if self.nodes[dst].kind == HostKind::ServeSeat {
            self.stats.federation += 1;
            let shaped = self.carry(src, dst);
            self.record(MsgEvent { t, src, dst, via: NONE, via2: NONE, route: Route::Federation, shaped });
            return Route::Federation;
        }
        if let Some(kind) = self.direct_kind_now(src, dst) {
            let li = self.open_link(src, dst, kind);
            let shaped = self.links[li].shaped != BudgetClass::Off;
            self.stats.direct += 1;
            self.stats.direct_shaped += u64::from(shaped);
            self.record(MsgEvent { t, src, dst, via: NONE, via2: NONE, route: Route::Direct, shaped });
            return Route::Direct;
        }
        let (f1, f2) = self.forwarder(src, dst);
        self.stats.relayed += 1;
        if f1 != NONE && self.nodes[f1 as usize].kind == HostKind::ServeSeat {
            self.stats.relayed_via_seat += 1;
        }
        let shaped = if f1 != NONE { self.carry(src, f1 as usize) } else { false };
        if f1 != NONE {
            let last = if f2 != NONE { f2 as usize } else { f1 as usize };
            self.carry(last, dst);
            if self.nodes[f1 as usize].kind != HostKind::ServeSeat {
                self.set_activity(f1 as usize, Activity::Relaying, 4.0);
            }
        }
        self.record(MsgEvent { t, src, dst, via: f1, via2: f2, route: Route::Relayed, shaped });
        self.maybe_propose(src, dst);
        Route::Relayed
    }

    /// Carry a frame over the one-hop link between `a` and `b` (opening it if a direct path
    /// exists); returns whether that hop was shaped.
    fn carry(&mut self, a: usize, b: usize) -> bool {
        if a == b {
            return false;
        }
        let kind = if let Some(l) = self.link_between(a, b) {
            l.kind
        } else if let Some(k) = self.direct_kind_now(a, b) {
            k
        } else {
            return false;
        };
        let li = self.open_link(a, b, kind);
        self.links[li].shaped != BudgetClass::Off
    }

    fn direct_kind_now(&self, a: usize, b: usize) -> Option<LinkKind> {
        let (na, nb) = (&self.nodes[a], &self.nodes[b]);
        if !(na.online(self.t) && nb.online(self.t)) {
            return None;
        }
        if na.kind == HostKind::ServeSeat || nb.kind == HostKind::ServeSeat {
            // a seat is reached over the homing link, or dialled (public listener)
            let (x, y) = key(a, b);
            if self.link_index.contains_key(&(x, y, LinkKind::Homing)) {
                return Some(LinkKind::Homing);
            }
            if na.kind == HostKind::ServeSeat && nb.kind == HostKind::ServeSeat {
                return Some(LinkKind::SeatMesh);
            }
            return Some(LinkKind::Dialled);
        }
        if self.has_link(a, b, LinkKind::Punched) {
            return Some(LinkKind::Punched);
        }
        self.unpunched_direct(a, b)
    }

    /// Propose a direct path for a relayed pair (v44+: on by default; room peers only).
    fn maybe_propose(&mut self, a: usize, b: usize) {
        let t = self.t;
        if !self.room_peers(a, b) || self.nodes[a].old_version || self.nodes[b].old_version {
            return;
        }
        let k = key(a, b);
        if self.backoff.get(&k).is_some_and(|&until| t < until) || self.punches.iter().any(|p| key(p.a, p.b) == k) {
            return;
        }
        let fac = self.nodes[a].homes.iter().copied().find(|&s| s != NONE && self.nodes[b].homes.contains(&s)).unwrap_or(self.nodes[a].homes[0]);
        if fac == NONE {
            return;
        }
        self.stats.punch_tried += 1;
        self.punches.push(Punch { a: k.0, b: k.1, done_at: t + DIRECT_PATH_BUDGET_S * self.rng.range(0.25, 1.0), facilitator: fac as usize });
        self.set_activity(a, Activity::Punching, DIRECT_PATH_BUDGET_S);
        self.set_activity(b, Activity::Punching, DIRECT_PATH_BUDGET_S);
    }

    fn settle_punches(&mut self) {
        let t = self.t;
        let p_ok = self.params.share(P::PunchSuccessShare);
        let due: Vec<Punch> = self.punches.iter().copied().filter(|p| p.done_at <= t).collect();
        self.punches.retain(|p| p.done_at > t);
        for p in due {
            let online = self.nodes[p.a].online(t) && self.nodes[p.b].online(t) && self.nodes[p.facilitator].online(t);
            let ok = online && self.punch_possible(p.a, p.b) && self.rng.chance(p_ok);
            if ok {
                self.stats.punch_ok += 1;
                self.open_link(p.a, p.b, LinkKind::Punched);
            } else {
                self.stats.punch_failed += 1;
                self.backoff.insert((p.a, p.b), t + DIRECT_PUNCH_FAILED_BACKOFF_S);
            }
            for i in [p.a, p.b] {
                if self.nodes[i].activity == Activity::Punching {
                    self.nodes[i].activity = Activity::Idle;
                }
            }
        }
    }

    /// The pending punch of a pair, if any (for the picture).
    #[must_use]
    pub fn punching(&self, a: usize, b: usize) -> bool {
        let k = key(a, b);
        self.punches.iter().any(|p| key(p.a, p.b) == k)
    }

    fn deliver_mail(&mut self) {
        let t = self.t;
        let mut i = 0;
        while i < self.mail.len() {
            let h = self.mail[i];
            if self.nodes[h.dst].online(t) && self.nodes[h.holder].online(t) {
                self.mail.swap_remove(i);
                self.nodes[h.holder].holding = self.nodes[h.holder].holding.saturating_sub(1);
                self.nodes[h.dst].pending = self.nodes[h.dst].pending.saturating_sub(1);
                if self.nodes[h.holder].holding == 0 && self.nodes[h.holder].activity == Activity::HoldingMail {
                    self.nodes[h.holder].activity = Activity::Idle;
                }
                let shaped = self.carry(h.holder, h.dst);
                self.stats.mail_delivered += 1;
                self.stats.mail_wait_s += t - h.since;
                let _ = h.src;
                self.record(MsgEvent { t, src: h.holder, dst: h.dst, via: NONE, via2: NONE, route: Route::MailDelivered, shaped });
            } else {
                i += 1;
            }
        }
    }

    /// Local hour at the patch now.
    #[must_use]
    pub fn local_hour(&self) -> f64 {
        (self.params.get(P::StartLocalHour) + self.t / 3600.0).rem_euclid(24.0)
    }

    fn generate(&mut self, dt: f64) {
        if self.quiet {
            return;
        }
        let t = self.t;
        let mu = diurnal_multiplier(self.local_hour() * 3600.0);
        let human = self.params.get(P::MsgMeanIntervalS).max(1.0);
        let ci = self.params.get(P::CiMsgIntervalS).max(1.0);
        for i in 0..self.nodes.len() {
            let n = &self.nodes[i];
            if !n.online(t) || n.pinned {
                continue;
            }
            let mean = match n.kind {
                HostKind::Phone | HostKind::Laptop => human / mu,
                HostKind::HomeNode | HostKind::HostedServer => 2.0 * human / mu,
                HostKind::CiRunner if n.activity == Activity::RunningJob => ci,
                _ => continue,
            };
            if self.rng.chance(leave_probability(dt, mean)) {
                if let Some(d) = self.pick_destination(i) {
                    if d != i {
                        self.send(i, d);
                    }
                }
            }
        }
    }

    fn expire_activity(&mut self) {
        let t = self.t;
        for n in &mut self.nodes {
            match n.activity {
                Activity::Resuming if n.resume_at <= t => n.activity = Activity::Idle,
                Activity::Sending | Activity::Relaying | Activity::Punching if n.activity_until <= t => {
                    n.activity = if n.holding > 0 { Activity::HoldingMail } else { Activity::Idle };
                }
                Activity::HoldingMail if n.holding == 0 => n.activity = Activity::Idle,
                _ => {}
            }
        }
    }

    /// Advance the world by `dt` seconds (internally in steps of at most 2 s).
    pub fn step(&mut self, dt: f64) {
        let mut left = dt.max(0.0);
        while left > 0.0 {
            let h = left.min(2.0);
            self.t += h;
            self.churn(h);
            self.maintain_links();
            self.settle_punches();
            self.deliver_mail();
            self.generate(h);
            self.expire_activity();
            left -= h;
        }
    }

    // -------------------------------------------------------------- story controls

    /// Freeze a node's autonomous churn and set its state (story control). The transport rules
    /// still decide what that state can do.
    pub fn pin(&mut self, i: usize, awake: bool, unmetered: bool, charging: bool) {
        self.nodes[i].pinned = true;
        if self.nodes[i].kind == HostKind::Phone {
            self.change_network(i, unmetered);
            self.nodes[i].charging = unmetered && charging;
        }
        if awake && !self.nodes[i].awake {
            self.resume(i);
        } else if !awake && self.nodes[i].awake {
            self.suspend(i);
        }
    }

    /// Release a pinned node back to its host kind's churn.
    pub fn unpin(&mut self, i: usize) {
        self.nodes[i].pinned = false;
    }

    /// Put two households in the community room together (a shared contact link).
    pub fn introduce(&mut self, a: usize, b: usize) {
        for i in [a, b] {
            if let Some(h) = self.nodes[i].household {
                self.households[h].in_community = true;
            }
        }
    }

    /// Two households two streets apart whose routers a punch can traverse, each with a phone and
    /// a home node, picked deterministically: `[alice_phone, alice_home, bob_phone, bob_home]`.
    #[must_use]
    pub fn story_actors(&self) -> Option<[usize; 4]> {
        let ok = |h: &Household| {
            h.adopting
                && h.router == RouterNat::Punchable
                && h.nodes.iter().any(|&j| self.nodes[j].kind == HostKind::Phone)
                && h.nodes.iter().any(|&j| self.nodes[j].kind == HostKind::HomeNode)
        };
        let mut cands: Vec<usize> = (0..self.households.len()).filter(|&h| ok(&self.households[h])).collect();
        if cands.len() < 2 {
            cands = (0..self.households.len())
                .filter(|&h| {
                    let hh = &self.households[h];
                    hh.adopting && hh.nodes.iter().any(|&j| self.nodes[j].kind == HostKind::Phone)
                        && hh.nodes.iter().any(|&j| self.nodes[j].kind == HostKind::HomeNode)
                })
                .collect();
        }
        let dist = |h: usize| (self.households[h].x.powi(2) + self.households[h].y.powi(2)).sqrt();
        cands.sort_by(|&x, &y| dist(x).total_cmp(&dist(y)));
        let alice = *cands.first()?;
        let (ax, ay) = (self.households[alice].x, self.households[alice].y);
        let bob = cands.iter().copied().skip(1).min_by(|&x, &y| {
            let d = |h: usize| {
                let dd = ((self.households[h].x - ax).powi(2) + (self.households[h].y - ay).powi(2)).sqrt();
                (dd - 110.0).abs()
            };
            d(x).total_cmp(&d(y))
        })?;
        let pick = |h: usize, k: HostKind| self.households[h].nodes.iter().copied().find(|&j| self.nodes[j].kind == k);
        Some([pick(alice, HostKind::Phone)?, pick(alice, HostKind::HomeNode)?, pick(bob, HostKind::Phone)?, pick(bob, HostKind::HomeNode)?])
    }

    /// Counts by host kind of present nodes.
    #[must_use]
    pub fn kind_counts(&self) -> [usize; 6] {
        let mut c = [0usize; 6];
        for n in &self.nodes {
            if n.present {
                c[n.kind as usize] += 1;
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZRH: (f64, f64) = (47.3769, 8.5417);

    fn world(density: f64, seed: u64) -> MicroWorld {
        MicroWorld::build_at_density(ZRH.0, ZRH.1, density, seed, MicroParams::default())
    }

    #[test]
    fn same_tile_same_place() {
        let a = world(1200.0, 7);
        let b = world(1200.0, 7);
        assert_eq!(a.roads, b.roads);
        assert_eq!(a.buildings, b.buildings);
        assert_eq!(a.nodes.len(), b.nodes.len());
        let c = world(1200.0, 8);
        assert_ne!(a.buildings, c.buildings);
    }

    #[test]
    fn density_picks_the_layout() {
        let p = MicroParams::default();
        assert_eq!(DensityClass::of(12_000.0, &p), DensityClass::UrbanCore);
        assert_eq!(DensityClass::of(3_000.0, &p), DensityClass::Urban);
        assert_eq!(DensityClass::of(800.0, &p), DensityClass::Suburban);
        assert_eq!(DensityClass::of(60.0, &p), DensityClass::Village);
        let city = world(12_000.0, 3);
        let village = world(60.0, 3);
        assert!(city.half < village.half, "a dense block is a smaller patch");
        let flats = city.buildings.iter().map(|b| b.units).max().unwrap_or(0);
        assert!(flats >= 8, "apartment blocks hold many dwellings, got {flats}");
        assert!(village.buildings.iter().all(|b| b.units <= 1));
    }

    #[test]
    fn every_kind_is_present_and_r5_bounded() {
        for (d, s) in [(12_000.0, 1u64), (3_000.0, 2), (900.0, 3), (90.0, 4)] {
            let mut w = world(d, s);
            assert!(w.nodes.len() <= R5_MAX_NODES);
            w.spawn_ci(Some(false));
            let c = w.kind_counts();
            assert!(c[HostKind::ServeSeat as usize] == 3);
            assert_eq!(c[HostKind::CiRunner as usize], 1);
            if d > 500.0 {
                assert!(c[HostKind::Phone as usize] > 0 && c[HostKind::HomeNode as usize] > 0, "{d}: {c:?}");
            }
        }
    }

    #[test]
    fn buildings_do_not_sit_on_roads() {
        let w = world(900.0, 11);
        for b in &w.buildings {
            for r in &w.roads {
                assert!(seg_dist(b.cx, b.cy, r) >= r.width / 2.0, "building centre on a road");
            }
        }
    }

    #[test]
    fn buildings_never_overlap() {
        for (d, seed) in [(12_000.0, 1u64), (3_000.0, 2), (900.0, 3), (90.0, 4)] {
            let w = world(d, seed);
            assert!(w.buildings.len() > 3, "{d}: {} buildings", w.buildings.len());
            for (i, a) in w.buildings.iter().enumerate() {
                for b in &w.buildings[i + 1..] {
                    assert!(!rects_overlap(a, b, 0.0), "{d}: overlapping footprints");
                }
            }
        }
    }

    #[test]
    fn phone_class_follows_network_and_is_client_only() {
        let w = world(1200.0, 5);
        let mut n = w.nodes.iter().find(|n| n.kind == HostKind::Phone).unwrap().clone();
        n.unmetered = true;
        assert_eq!(n.budget_class(), BudgetClass::PhoneWifi);
        n.unmetered = false;
        assert_eq!(n.budget_class(), BudgetClass::PhoneCellular);
        assert!(n.client_only);
        n.unmetered = true;
        n.charging = true;
        n.public = true;
        assert!(!n.relay_offering(1e9), "a v45 phone is client-only: no relay slot even on Wi-Fi and power");
        n.client_only = false;
        assert!(n.relay_offering(1e9), "with client-only off, P-R1: unmetered and charging relays");
        n.charging = false;
        assert!(!n.relay_offering(1e9));
    }

    #[test]
    fn laptop_and_ci_run_as_home() {
        let w = world(1200.0, 5);
        let lap = w.nodes.iter().find(|n| n.kind == HostKind::Laptop).unwrap();
        assert_eq!(lap.budget_class(), BudgetClass::Home);
        assert!(lap.device_relays(), "platform default home: relays with no battery gate");
        let ci = w.nodes.iter().find(|n| n.kind == HostKind::CiRunner).unwrap();
        assert_eq!(ci.budget_class(), BudgetClass::Home);
        assert!(!ci.public && !ci.punchable);
    }

    #[test]
    fn seat_offers_server_to_non_seats_and_seat_to_seats() {
        let mut w = world(1200.0, 9);
        w.step(4.0);
        for l in &w.links {
            if l.kind == LinkKind::SeatMesh {
                assert_eq!(l.shaped, BudgetClass::Seat);
            } else {
                assert_ne!(l.shaped, BudgetClass::Seat, "seat class only between two seats");
            }
        }
    }

    #[test]
    fn the_bound_of_eight_holds_and_the_ninth_runs_unshaped() {
        let mut w = world(3_000.0, 13);
        w.step(30.0);
        for (i, n) in w.nodes.iter().enumerate() {
            let shaped = w.links.iter().filter(|l| (l.a == i || l.b == i) && l.shaped != BudgetClass::Off && l.shaped != BudgetClass::Seat).count();
            assert!(shaped <= SHAPED_LINK_BOUND, "node {i} ({:?}) holds {shaped} shaped", n.kind);
            assert_eq!(shaped, n.shaped_used as usize);
        }
        let seat = w.seat_ids()[0];
        let homing = w.links.iter().filter(|l| l.kind == LinkKind::Homing && (l.a == seat || l.b == seat)).count();
        assert!(homing > SHAPED_LINK_BOUND, "seat should have more homed nodes than slots, has {homing}");
        assert!(w.stats.ninth_unshaped > 0);
        assert!(w.links.iter().any(|l| l.at_bound && l.shaped == BudgetClass::Off));
    }

    #[test]
    fn homing_is_top_k_of_m_and_pairs_share_a_seat() {
        let w = world(1200.0, 17);
        let seats = w.seat_ids();
        for n in w.nodes.iter().filter(|n| n.kind != HostKind::ServeSeat) {
            assert!(n.homes[0] != NONE && n.homes[1] != NONE && n.homes[0] != n.homes[1]);
            assert!(n.homes.iter().all(|h| seats.contains(&(*h as usize))));
        }
        // k=2 of M=3: any two nodes share at least one seat
        let a = w.nodes.iter().position(|n| n.kind == HostKind::Phone).unwrap();
        let b = w.nodes.iter().rposition(|n| n.kind == HostKind::Phone).unwrap();
        let (f, f2) = w.forwarder(a, b);
        assert!(f != NONE && f2 == NONE || w.nodes[a].household == w.nodes[b].household);
    }

    #[test]
    fn suspend_closes_every_link_and_mail_waits_at_a_member_node() {
        let mut w = world(1200.0, 21);
        let [ap, ah, _bp, _bh] = w.story_actors().expect("actors");
        w.quiet = true;
        w.pin(ap, true, true, false);
        w.pin(ah, true, true, true);
        w.step(15.0);
        assert!(w.links.iter().any(|l| l.a == ap || l.b == ap), "awake phone homes");
        w.pin(ap, false, true, false);
        assert!(!w.links.iter().any(|l| l.a == ap || l.b == ap), "suspend closes every link");
        let r = w.send(ah, ap);
        assert_eq!(r, Route::Mailboxed);
        let held_at = w.events.back().unwrap().via as usize;
        assert_eq!(w.nodes[held_at].kind, HostKind::HomeNode, "a member-owned always-on node holds it");
        assert_eq!(w.nodes[ap].pending, 1);
        w.pin(ap, true, true, false);
        w.step(RESUME_ANNOUNCE_WAIT_S + 2.0);
        assert_eq!(w.nodes[ap].pending, 0, "delivered after resume");
        assert_eq!(w.stats.mail_delivered, 1);
    }

    #[test]
    fn a_phone_is_never_a_mailbox() {
        let mut w = world(1200.0, 23);
        w.step(10.0);
        for i in 0..w.nodes.len() {
            if let Some(m) = w.mailbox_for(i) {
                assert_ne!(w.nodes[m].kind, HostKind::Phone);
                assert_ne!(w.nodes[m].kind, HostKind::Laptop);
            }
        }
    }

    #[test]
    fn lan_inside_a_home_and_relay_or_punch_between_homes() {
        let mut w = world(1200.0, 29);
        let [ap, ah, bp, bh] = w.story_actors().expect("actors");
        w.quiet = true;
        for i in [ap, ah, bp, bh] {
            w.pin(i, true, true, false);
        }
        w.introduce(ap, bp);
        w.step(12.0);
        assert_eq!(w.send(ap, ah), Route::Direct);
        assert_eq!(w.link_between(ap, ah).unwrap().kind, LinkKind::Lan);
        // two punchable NATs, no listener: the first row is relayed and a punch is proposed
        let first = w.send(ap, bp);
        if w.unpunched_direct(ap, bp).is_none() {
            assert_eq!(first, Route::Relayed);
            assert!(w.punching(ap, bp));
            w.params.set(P::PunchSuccessShare, 1.0);
            w.step(DIRECT_PATH_BUDGET_S + 1.0);
            assert_eq!(w.link_between(ap, bp).map(|l| l.kind), Some(LinkKind::Punched));
            assert_eq!(w.send(ap, bp), Route::Direct);
        }
    }

    #[test]
    fn a_failed_punch_backs_off_an_hour_and_old_peers_never_propose() {
        let mut w = world(1200.0, 31);
        let [ap, _ah, bp, _bh] = w.story_actors().expect("actors");
        w.quiet = true;
        w.pin(ap, true, true, false);
        w.pin(bp, true, true, false);
        w.introduce(ap, bp);
        w.params.set(P::PunchSuccessShare, 0.0);
        w.step(12.0);
        if w.unpunched_direct(ap, bp).is_some() {
            return;
        }
        w.send(ap, bp);
        w.step(DIRECT_PATH_BUDGET_S + 1.0);
        assert_eq!(w.stats.punch_failed, 1);
        w.send(ap, bp);
        assert!(!w.punching(ap, bp), "backed off");
        w.step(DIRECT_PUNCH_FAILED_BACKOFF_S + 1.0);
        w.send(ap, bp);
        assert!(w.punching(ap, bp), "proposes again after the back-off");
        // an old peer
        w.nodes[bp].old_version = true;
        w.step(DIRECT_PATH_BUDGET_S + 1.0);
        let tried = w.stats.punch_tried;
        w.step(DIRECT_PUNCH_FAILED_BACKOFF_S + 1.0);
        w.send(ap, bp);
        assert_eq!(w.stats.punch_tried, tried, "pre-v44 peer: no proposal");
    }

    #[test]
    fn cellular_drops_lan_and_punched_paths() {
        let mut w = world(1200.0, 37);
        let [ap, ah, _, _] = w.story_actors().expect("actors");
        w.quiet = true;
        w.pin(ap, true, true, false);
        w.pin(ah, true, true, true);
        w.step(12.0);
        w.send(ap, ah);
        assert!(w.link_between(ap, ah).is_some());
        w.pin(ap, true, false, false);
        w.step(RESUME_ANNOUNCE_WAIT_S + 2.0);
        assert_eq!(w.nodes[ap].budget_class(), BudgetClass::PhoneCellular);
        assert!(w.link_between(ap, ah).map_or(true, |l| l.kind != LinkKind::Lan), "left the LAN");
    }

    #[test]
    fn ci_runner_boots_runs_and_leaves() {
        let mut w = world(1200.0, 41);
        w.params.set(P::CiArrivalsPerHour, 0.0);
        let i = w.spawn_ci(Some(true)).unwrap();
        assert_eq!(w.nodes[i].activity, Activity::Booting);
        w.step(RESUME_ANNOUNCE_WAIT_S + 2.0);
        assert_eq!(w.nodes[i].activity, Activity::RunningJob);
        assert!(w.links.iter().any(|l| l.kind == LinkKind::Homing && (l.a == i || l.b == i)));
        let leave = w.nodes[i].leave_at;
        w.step(leave - w.t + 3.0);
        assert!(!w.nodes[i].present);
        assert!(!w.links.iter().any(|l| l.a == i || l.b == i));
        assert_eq!(w.stats.ci_left, 1);
    }

    #[test]
    fn stationary_shares_follow_the_parameters() {
        let mut w = world(3_000.0, 43);
        w.quiet = true;
        let phones: Vec<usize> = (0..w.nodes.len()).filter(|&i| w.nodes[i].kind == HostKind::Phone).collect();
        let (mut asleep, mut cell, mut samples) = (0usize, 0usize, 0usize);
        for _ in 0..400 {
            w.step(60.0);
            for &i in &phones {
                samples += 1;
                asleep += usize::from(!w.nodes[i].awake);
                cell += usize::from(!w.nodes[i].unmetered);
            }
        }
        let a = asleep as f64 / samples as f64;
        let c = cell as f64 / samples as f64;
        assert!((a - 0.15).abs() < 0.06, "asleep share {a}");
        assert!((c - 0.35).abs() < 0.08, "cellular share {c}");
    }

    #[test]
    fn traffic_flows_by_every_route() {
        let mut w = world(3_000.0, 47);
        w.step(3_600.0);
        let s = &w.stats;
        assert!(s.sent > 100, "{s:?}");
        assert!(s.direct > 0 && s.relayed > 0 && s.mailboxed > 0 && s.mail_delivered > 0 && s.federation > 0, "{s:?}");
        assert!(s.punch_tried > 0, "{s:?}");
        assert!(s.network_changes > 0 && s.suspends > 0);
    }

    #[test]
    fn params_are_all_named_and_tagged() {
        assert_eq!(PARAM_SPECS.len(), PARAM_COUNT);
        for s in PARAM_SPECS {
            assert!(!s.name.is_empty() && !s.source.is_empty());
        }
        let p = MicroParams::default();
        assert_eq!(p.get(P::ReachHomeShare), 0.40);
        assert_eq!(PARAM_SPECS[P::CellularPunchableShare as usize].tag, Tag::Frontier);
    }

    #[test]
    fn destination_round_trip() {
        let (la, lo) = destination(ZRH.0, ZRH.1, 0.0, 111.195);
        assert!((la - (ZRH.0 + 1.0)).abs() < 0.01 && (lo - ZRH.1).abs() < 1e-6);
    }
}
