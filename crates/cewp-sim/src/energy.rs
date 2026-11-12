//! The energy delta pass (twin package `theory`, `FSD/twin/energy.md`).
//!
//! The quantity is a **delta from the status quo**, not a gross total ("people use phones
//! regardless"). Per household-hour (or per participant-hour of a call) it is three terms:
//!
//! * **A, added on existing devices**: extra draw on devices people already run (phone wake-ups
//!   and their radio tails, radio bytes, signature work, the marginal draw of a node on a router
//!   or computer that is on anyway), net of the device work the status-quo app does for the
//!   same workload;
//! * **B, added devices**: the full draw of devices added or kept on FOR the mesh (a new
//!   always-on box, a village node, the household's share of a hosted seat or relay);
//! * **C, avoided upstream (net)**: data-centre and network energy of the workload that moves
//!   off the status quo, minus the network energy the mesh's own bytes take.
//!
//! `delta = A + B - C`. Positive is a cost, negative a saving; the model can come out either
//! way, and [`break_even`] reports which inputs flip it and at what value.
//!
//! Counts come from the other packages' hooks ([`Counters::from_hooks`]); every joule is a
//! count times a row of [`ENERGY_SPECS`] ([`Contribution`]). Power rows have no dataset behind
//! them: each is MODEL or FRONTIER from stated reasoning, laid out so CIRISHolon's sourced
//! table (per class, per state, with `would_be_on_anyway` and a year) replaces it row for row.
//! Intervals are propagated with interval arithmetic over the same code path that computes the
//! point, so the bounds enclose every combination of input ends (and are wider than the true
//! range where an input appears in more than one term).

use core::ops::{Add, Mul, Neg, Sub};

use crate::capacity::CostParams;
use crate::link::Discipline;
use crate::media::{self, Engine, LinkClass};
use crate::micro::HostKind;
use crate::netgraph::{self, NetGraph, NetParams, PathCounters};
use crate::people_hooks;
use crate::reach::hooks::{energy_counters, NodeClass};
use crate::reach::{Correlation, ReachModel};
use crate::seat_hooks;
use crate::social::{Geography, Placement, SocialParams, SocialWorld};
use crate::activity::ActivityParams;
use crate::workload::{ParameterRegime, RegimeParameters};

// ---------------------------------------------------------------------------
// Number class
// ---------------------------------------------------------------------------

/// FSD 8.2 number classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Measured on a real system.
    Measured,
    /// Derived from the model's own events.
    Derived,
    /// A stated modelling choice with its reasoning.
    Model,
    /// Extrapolated.
    Projected,
    /// Nothing supports a value; a placeholder.
    Frontier,
}

impl Class {
    /// As written in the FSD.
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
    /// The globe's class code (`seat_hooks::class`).
    #[must_use]
    pub const fn code(self) -> f32 {
        match self {
            Self::Measured => 0.0,
            Self::Derived => 1.0,
            Self::Model => 2.0,
            Self::Projected => 3.0,
            Self::Frontier => 4.0,
        }
    }
    /// The weaker of two classes (a result is no stronger than its weakest input).
    #[must_use]
    pub fn weaker(self, o: Self) -> Self {
        let rank = |c: Self| match c {
            Self::Measured => 0,
            Self::Derived => 1,
            Self::Model => 2,
            Self::Projected => 3,
            Self::Frontier => 4,
        };
        if rank(o) > rank(self) { o } else { self }
    }
}

// ---------------------------------------------------------------------------
// The input table
// ---------------------------------------------------------------------------

/// One row of [`ENERGY_SPECS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergySpec {
    /// Stable name (CIRISHolon's table keys rows by it).
    pub name: &'static str,
    /// Device class (or `network`, `datacentre`, `workload`).
    pub device: &'static str,
    /// State of the device the figure is for.
    pub state: &'static str,
    /// Unit.
    pub unit: &'static str,
    /// Low end.
    pub lo: f64,
    /// Default.
    pub mid: f64,
    /// High end.
    pub hi: f64,
    /// Class.
    pub class: Class,
    /// Whether the device in this state would be on anyway without the mesh (`None` for rows
    /// that are not a device draw).
    pub would_be_on_anyway: Option<bool>,
    /// Year the figure describes.
    pub year: u16,
    /// Which measurement on real hardware would replace it (`""` when a published sourced table
    /// can).
    pub needs_measurement: &'static str,
    /// Reasoning.
    pub why: &'static str,
}

macro_rules! energy_rows {
    ($( $idx:ident = $name:literal, $dev:literal, $state:literal, $unit:literal, [$lo:expr, $mid:expr, $hi:expr], $cls:ident, $on:expr, $year:literal, $meas:literal, $why:literal; )*) => {
        /// Index into [`ENERGY_SPECS`].
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum E { $( $idx, )* }
        /// Every input row, in index order.
        pub const ENERGY_SPECS: &[EnergySpec] = &[
            $( EnergySpec { name: $name, device: $dev, state: $state, unit: $unit, lo: $lo, mid: $mid, hi: $hi, class: Class::$cls, would_be_on_anyway: $on, year: $year, needs_measurement: $meas, why: $why }, )*
        ];
        impl E {
            /// Every row.
            pub const ALL: &'static [E] = &[ $( E::$idx, )* ];
        }
    };
}

energy_rows! {
    // ---- phone ------------------------------------------------------------------------------
    PhoneDeepIdleW = "phone.deep_idle_w", "phone", "deep idle (screen off, radio idle)", "W", [0.008, 0.015, 0.03], Model, Some(true), 2026, "",
        "order of a modern smartphone's suspended draw (tens of mW). Listed for completeness: the mesh does not change deep idle, so no term charges it";
    PhoneWakeJWifi = "phone.wake_j.wifi", "phone", "one wake-up burst on Wi-Fi, radio tail included", "J", [0.03, 0.15, 0.6], Frontier, Some(true), 2026,
        "phone on a power monitor: J per edge round wake on Wi-Fi (screen off, Doze off and on), net of the idle baseline",
        "Wi-Fi power-save tail of ~0.1 to 0.3 s at ~0.5 to 1 W plus an application-processor wake; no CIRIS device measured";
    PhoneWakeJCell = "phone.wake_j.cellular", "phone", "one wake-up burst on cellular, RRC tail included", "J", [0.5, 3.0, 12.0], Frontier, Some(true), 2026,
        "phone on a power monitor: J per edge round wake on LTE and 5G NSA, with the carrier's own inactivity timer and C-DRX",
        "an LTE RRC inactivity tail of ~10 s at ~1 W (people_hooks RADIO_TAIL_S = 10 s, MODEL) is the high end; C-DRX and fast dormancy bring it down; no CIRIS device measured";
    PhoneJPerMbWifi = "phone.j_per_mb.wifi", "phone", "active radio, Wi-Fi", "J/MB", [0.02, 0.08, 0.3], Model, Some(true), 2026, "",
        "Wi-Fi transfer energy on a phone of the order of 0.1 J/MB at moderate rates, falling at high rates";
    PhoneJPerMbCell = "phone.j_per_mb.cellular", "phone", "active radio, cellular", "J/MB", [0.2, 0.8, 3.0], Model, Some(true), 2026, "",
        "cellular transfer energy several times Wi-Fi's per byte (higher transmit power, scheduling)";
    PhoneJPerSigOp = "phone.j_per_sig_op", "phone", "one hybrid sign or verify on the CPU", "J", [0.0002, 0.0008, 0.003], Frontier, Some(true), 2026,
        "phone on a power monitor: J per hybrid verify and per sign at the edge's batch sizes",
        "hybrid verify 276 us MEASURED on an EPYC core (cewp-cli metrics); a phone core several times slower at ~1.5 W gives ~0.5 to 3 mJ";
    PhoneJPerChunkSigOp = "phone.j_per_chunk_sig_op", "phone", "one Ed25519 sign or verify of a media chunk", "J", [0.00002, 0.0001, 0.0005], Frontier, Some(true), 2026,
        "phone on a power monitor during a call, against the media engine's per-chunk sign and verify counts",
        "media chunks are Ed25519 (media.rs NodeEnergy signs and verifies): tens of us on a phone core at ~1.5 W";
    // ---- home node ----------------------------------------------------------------------------
    RouterNodeW = "home_node.on_router.marginal_w", "home node", "running on the household's existing router (marginal)", "W", [0.1, 0.4, 1.5], Frontier, Some(true), 2026,
        "router with and without the edge daemon at the twin's schedule, at the wall",
        "a router is on anyway; the daemon adds CPU wake-ups and a little RAM traffic on a SoC that idles at a few W";
    ComputerNodeW = "home_node.on_computer.marginal_w", "home node", "running on a computer already on (marginal)", "W", [0.2, 0.8, 3.0], Frontier, Some(true), 2026,
        "desktop or NAS already on, with and without the edge daemon, at the wall",
        "a desktop's package power rises by tenths of a W to a few W when a daemon keeps it out of its deepest idle states";
    HomeNodeJPerMb = "home_node.j_per_mb", "home node", "bytes sent or received, relayed bytes included", "J/MB", [0.005, 0.02, 0.1], Frontier, Some(true), 2026,
        "home node at the wall forwarding a known byte stream for others (relay), J per MB in plus out",
        "wired NIC and CPU per byte on small hardware; relayed bytes count twice (in and out)";
    HomeNodeJPerSigOp = "home_node.j_per_sig_op", "home node", "one hybrid sign or verify", "J", [0.0001, 0.0004, 0.002], Model, Some(true), 2026, "",
        "the EPYC verify (276 us MEASURED) on a slower core at a few W";
    HomeNodeJPerChunkSigOp = "home_node.j_per_chunk_sig_op", "home node", "one Ed25519 sign or verify of a media chunk", "J", [0.00001, 0.00005, 0.0003], Model, Some(true), 2026, "",
        "Ed25519 verify of the order of 50 us on a desktop or SBC core at a few W";
    AddedBoxW = "home_node.added_box.w", "home node", "a new always-on box bought or kept on for the mesh (full draw)", "W", [2.5, 5.0, 12.0], Model, Some(false), 2026, "",
        "single-board computer at 2.5 to 5 W, mini-PC idle 6 to 12 W";
    LaptopW = "laptop.marginal_w", "laptop", "running the edge while awake (marginal)", "W", [0.1, 0.5, 1.5], Frontier, Some(true), 2026,
        "laptop on battery with and without the daemon",
        "listed for CIRISHolon's table; no scenario below puts the edge on a laptop";
    // ---- hosted ---------------------------------------------------------------------------------
    HostedSeatW = "hosted.seat.w", "hosted server", "a serve seat, idle included", "W", [60.0, 120.0, 300.0], Model, Some(false), 2026, "",
        "a one-socket rented server at the wall including its share of data-centre overhead (PUE)";
    DevicesPerSeat = "hosted.devices_per_seat", "hosted server", "devices one seat serves", "devices", [1000.0, 2000.0, 10000.0], Model, None, 2026, "",
        "the page draws one seat per ~2,000 nodes (docs/globe.js buildWorld); the seat knee interval runs higher";
    HostedJPerRead = "hosted.j_per_store_read", "hosted server", "one counted store read", "J", [0.001, 0.01, 0.05], Frontier, Some(false), 2026,
        "seat at the wall against its store-read counter over a window",
        "a few ms of one core at ~5 to 10 W per core";
    HostedJPerMb = "hosted.j_per_mb", "hosted server", "relayed bytes on a hosted relay", "J/MB", [0.01, 0.05, 0.3], Frontier, Some(false), 2026,
        "hosted relay at the wall against bytes forwarded",
        "server NIC, CPU and seal work per byte with PUE";
    // ---- network ---------------------------------------------------------------------------------
    AccessFixedJPerMb = "network.access_fixed.j_per_mb", "network", "fixed access (one end, one direction)", "J/MB", [0.0, 5.0, 100.0], Frontier, None, 2026, "",
        "the decisive unknown: an attributional average of ~0.03 kWh/GB (~100 J/MB) in studies of the late 2010s, against a marginal near zero because fixed access power barely varies with traffic; 5 J/MB is a placeholder between them";
    AccessMobileJPerMb = "network.access_mobile.j_per_mb", "network", "mobile access (one end, one direction)", "J/MB", [5.0, 50.0, 500.0], Frontier, None, 2026, "",
        "attributional averages of ~0.1 to 0.2 kWh/GB for mobile networks around 2020 (360 to 720 J/MB), marginal much lower";
    CoreJPerMbPerAs = "network.core.j_per_mb_per_as", "network", "per AS crossed (routers and metro transport)", "J/MB", [0.005, 0.05, 0.3], Frontier, None, 2026, "",
        "a few routers per AS at ~1 to 10 nJ/bit each";
    LongHaulJPerMbPer1000Km = "network.long_haul.j_per_mb_per_1000km", "network", "per 1,000 km of long-haul", "J/MB", [0.002, 0.02, 0.1], Frontier, None, 2026, "",
        "optical amplifiers and regenerators per km per byte, small against routers";
    VillageUplinkJPerMb = "network.thin_uplink.j_per_mb", "network", "a village's one thin uplink (satellite or long microwave)", "J/MB", [5.0, 50.0, 500.0], Frontier, None, 2026, "",
        "a satellite terminal's 50 to 100 W over a few Mbit/s of use, or a solar microwave hop; per byte far above urban access";
    // ---- data centre ----------------------------------------------------------------------------
    DcMsgWhPerPersonHour = "datacentre.messaging.wh_per_person_hour", "data centre", "messaging and social workload one person drives, per hour of the day", "Wh", [0.02, 0.1, 0.5], Frontier, None, 2026, "",
        "public operator data-centre electricity over monthly users is of the order of a few Wh per user-day (quoted from memory, not re-read); the share that is messaging the mesh replaces is unknown";
    DcCallWhPerParticipantHour = "datacentre.call.wh_per_participant_hour", "data centre", "SFU work per participant-hour of a video call", "Wh", [0.1, 1.0, 5.0], Frontier, None, 2026, "",
        "an SFU forwards without transcoding; server share per participant small, the high end includes recording and transcoding";
    // ---- village ---------------------------------------------------------------------------------
    VillageNodeW = "village.node.w", "village node", "one added node with its radio and solar controller (full draw)", "W", [5.0, 10.0, 20.0], Model, Some(false), 2026, "",
        "a small server plus a Wi-Fi or mesh radio";
    VillageHouseholds = "village.households", "workload", "households behind one village node and uplink", "households", [20.0, 50.0, 200.0], Model, None, 2026, "",
        "a village of a few hundred people";
    // ---- workload and household ---------------------------------------------------------------
    PersonsPerHousehold = "household.persons", "workload", "persons (one phone each)", "persons", [2.0, 2.5, 3.0], Model, None, 2026, "",
        "micro.rs household rule round(2.5 + 1.1 N(0,1)) clamped to 1..6";
    WifiShare = "phone.wifi_share", "workload", "share of a phone's online hours on Wi-Fi", "share", [0.5, 0.7, 0.85], Frontier, None, 2026, "",
        "CIRISHolon's attach.phone.network rhythm (168 bins) replaces it";
    BytesPerAct = "workload.bytes_per_act", "workload", "wire bytes of one act's content copy", "bytes", [1000.0, 5000.0, 50000.0], Model, None, 2026, "",
        "a signed text act with a thumbnail; media-heavy acts run higher";
    MeshExtraContentWakes = "phone.mesh_extra_content_wakes", "workload", "extra content wake-ups under the mesh, as a share of the status quo's push wake-ups", "share", [-0.3, 0.0, 0.3], Frontier, None, 2026,
        "phone wake log under the mesh and under a push-notification app for one scripted day",
        "mesh content arrives on a live link, push on the OS channel; whether batching differs is unknown, so zero with a symmetric band";
    PhoneScheduleShare = "phone.schedule_share", "workload", "share of the edge round and Pull schedule a phone runs itself", "share", [1.0, 1.0, 1.0], Model, None, 2026, "",
        "the edge at 8e46165 has no host-kind branch (seat_hooks), so a phone runs the whole schedule; a phone that left its rounds to its home node would run less (a design lever: see the break-even table)";
    KeepaliveBytesPerMappingHour = "phone.keepalive_bytes_per_mapping_hour", "workload", "bytes to hold one NAT mapping for an hour", "bytes", [0.0, 2000.0, 20000.0], Frontier, None, 2026,
        "packet capture of a phone holding punched paths",
        "keepalives ride the 30 s round wake (shorter than any NAT timeout), so they add bytes, not wake-ups";
}

/// One valuation of every row.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyParams {
    v: Vec<f64>,
}

impl Default for EnergyParams {
    fn default() -> Self {
        Self { v: ENERGY_SPECS.iter().map(|s| s.mid).collect() }
    }
}

impl EnergyParams {
    /// A row's value.
    #[must_use]
    pub fn get(&self, e: E) -> f64 {
        self.v[e as usize]
    }
    /// Set a row.
    pub fn set(&mut self, e: E, x: f64) {
        self.v[e as usize] = x;
    }
    /// A copy with one row set.
    #[must_use]
    pub fn with(&self, e: E, x: f64) -> Self {
        let mut p = self.clone();
        p.set(e, x);
        p
    }
}

/// A row's spec.
#[must_use]
pub fn spec(e: E) -> &'static EnergySpec {
    &ENERGY_SPECS[e as usize]
}

// ---------------------------------------------------------------------------
// Interval arithmetic (the same code path computes points and bounds)
// ---------------------------------------------------------------------------

/// A closed interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Iv {
    /// Low end.
    pub lo: f64,
    /// High end.
    pub hi: f64,
}

impl Iv {
    /// `[lo, hi]`.
    #[must_use]
    pub fn new(lo: f64, hi: f64) -> Self {
        Self { lo: lo.min(hi), hi: lo.max(hi) }
    }
    /// Whether 0 is outside the interval.
    #[must_use]
    pub fn sign_known(&self) -> bool {
        self.lo > 0.0 || self.hi < 0.0
    }
}

impl Add for Iv {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self { lo: self.lo + o.lo, hi: self.hi + o.hi }
    }
}
impl Sub for Iv {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self { lo: self.lo - o.hi, hi: self.hi - o.lo }
    }
}
impl Neg for Iv {
    type Output = Self;
    fn neg(self) -> Self {
        Self { lo: -self.hi, hi: -self.lo }
    }
}
impl Mul for Iv {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        let c = [self.lo * o.lo, self.lo * o.hi, self.hi * o.lo, self.hi * o.hi];
        Self { lo: c.iter().copied().fold(f64::INFINITY, f64::min), hi: c.iter().copied().fold(f64::NEG_INFINITY, f64::max) }
    }
}

/// A number the model can be evaluated in: `f64` for points, [`Iv`] for bounds.
pub trait Q: Copy + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self> {
    /// A constant.
    fn k(x: f64) -> Self;
    /// `1 / self` (inputs are positive where this is used).
    fn recip(self) -> Self;
}
impl Q for f64 {
    fn k(x: f64) -> Self {
        x
    }
    fn recip(self) -> Self {
        1.0 / self
    }
}
impl Q for Iv {
    fn k(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn recip(self) -> Self {
        Self::new(1.0 / self.hi, 1.0 / self.lo)
    }
}

// ---------------------------------------------------------------------------
// Counters (DERIVED from the other packages' hooks)
// ---------------------------------------------------------------------------

/// Path counters averaged over a sample (netgraph `PathCounters`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MeanPath {
    /// Paths averaged.
    pub n: u32,
    /// Mean ASes crossed (both ends included).
    pub ases_crossed: f64,
    /// Mean IXP crossings.
    pub ixp_crossings: f64,
    /// Mean long-haul km.
    pub long_haul_km: f64,
}

impl MeanPath {
    fn of<'a>(it: impl Iterator<Item = &'a PathCounters>) -> Self {
        let mut m = Self::default();
        for c in it {
            m.n += 1;
            m.ases_crossed += f64::from(c.ases_crossed);
            m.ixp_crossings += f64::from(c.ixp_crossings);
            m.long_haul_km += f64::from(c.long_haul_km);
        }
        if m.n > 0 {
            let n = f64::from(m.n);
            m.ases_crossed /= n;
            m.ixp_crossings /= n;
            m.long_haul_km /= n;
        }
        m
    }
}

/// Media counts per participant-hour for one link class.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CallClass {
    /// Participant-hours of this class in the run.
    pub hours: f64,
    /// Own stream copies sent, MB.
    pub own_tx_mb: f64,
    /// Copies forwarded for others, MB.
    pub fwd_tx_mb: f64,
    /// Media received, MB (duplicates included).
    pub rx_mb: f64,
    /// Signs and verifies.
    pub sig_ops: f64,
}

/// Every count the delta reads, with where it came from. All DERIVED from the named package's
/// events (whose own inputs carry their classes).
#[derive(Debug, Clone, PartialEq)]
pub struct Counters {
    /// seat_hooks `device_counters(Phone, unclaimed)`: radio wake-ups per device-hour.
    pub sync_wakes_per_device_hour: f64,
    /// seat_hooks: bytes per device-hour, each way.
    pub sync_bytes_per_device_hour: f64,
    /// seat_hooks: seat store reads per device-hour.
    pub seat_reads_per_device_hour: f64,
    /// people_hooks (Poisson process): acts one person signs per hour.
    pub acts_sent_per_person_hour: f64,
    /// people_hooks: act copies one person receives per hour.
    pub acts_received_per_person_hour: f64,
    /// people_hooks: content wake-ups per phone-hour (radio tail 10 s, MODEL).
    pub content_wakes_per_phone_hour: f64,
    /// reach hooks: share of a fixed endpoint's pairs that ride a relay.
    pub relayed_share_fixed: f64,
    /// reach hooks: same for a phone on Wi-Fi.
    pub relayed_share_wifi: f64,
    /// reach hooks: same for a phone on a carrier.
    pub relayed_share_cell: f64,
    /// reach hooks: NAT mappings a phone holds per pair it reaches.
    pub held_mappings_per_phone: f64,
    /// netgraph: peer pairs inside one metro.
    pub path_same_metro: MeanPath,
    /// netgraph: all sampled fixed peer pairs (half inside one region).
    pub path_any_peer: MeanPath,
    /// netgraph: a fixed endpoint to the nearest content (data-centre) network.
    pub path_to_dc: MeanPath,
    /// media `call4` (seed, busy, tail-drop): per class, in [`LinkClass`] order Fixed, WiFi,
    /// Cellular.
    pub call: [CallClass; 3],
    /// Participants in the call.
    pub call_participants: f64,
}

/// Nearest metro (by backbone km) that holds a content network, and that network.
fn nearest_content(g: &NetGraph, m: netgraph::MetroId) -> Option<(netgraph::Asn, netgraph::MetroId)> {
    let mut best: Option<(u32, netgraph::Asn, netgraph::MetroId)> = None;
    for mc in 0..g.metros.len() {
        let mc = mc as netgraph::MetroId;
        let Some(&(asn, _)) = g.content_market(mc).first() else { continue };
        let km = g.backbone_km(m, mc);
        if best.map_or(true, |b| km < b.0) {
            best = Some((km, asn, mc));
        }
    }
    best.map(|(_, a, m)| (a, m))
}

impl Counters {
    /// Read every hook at `seed`. Small worlds, so it runs under the twin's memory cap.
    #[must_use]
    pub fn from_hooks(seed: u64) -> Self {
        // seat schedule
        let d = seat_hooks::device_counters(&CostParams::default(), HostKind::Phone, false);
        // people: a 2,000-person stand-in world, one day, Poisson (today's law)
        let geo = Geography::stand_in(seed, 20, 4, 4);
        let w = SocialWorld::build(&geo, &SocialParams::default(), 2000, seed, Placement::Placed);
        let pe = people_hooks::energy(&w, &ActivityParams::poisson(), 86_400.0, seed, &[], 200, people_hooks::RADIO_TAIL_S);
        let person_hours = pe.phone.devices.max(1) as f64 * 24.0;
        // reach
        let rc = energy_counters(&ReachModel::default_model(), seed, Correlation::Correlated, 120);
        let share = |k: NodeClass| {
            let c = rc.per_class[k as usize];
            if c.pairs == 0 { 0.0 } else { f64::from(c.relayed) / f64::from(c.pairs) }
        };
        let (mut held, mut ph_pairs) = (0.0, 0.0);
        for k in [NodeClass::PhoneWifi, NodeClass::PhoneCellular] {
            let c = rc.per_class[k as usize];
            held += f64::from(c.held_mappings);
            ph_pairs += f64::from(c.pairs);
        }
        let phones = rc.class.iter().filter(|c| !matches!(c, NodeClass::Fixed)).count().max(1) as f64;
        // netgraph
        let g = netgraph::generate(seed, &NetParams::default());
        let pairs = netgraph::sample_fixed_pairs(&g, seed, 400);
        let same = MeanPath::of(pairs.iter().filter(|p| p.same_metro).map(|p| &p.path.counters));
        let any = MeanPath::of(pairs.iter().map(|p| &p.path.counters));
        let mut dc = Vec::new();
        for p in &pairs {
            let a = p.path.ases[0];
            let m = p.path.metro_trail[0];
            let (Some(la), Some((casn, cm))) = (g.label(a, m), nearest_content(&g, m)) else { continue };
            let Some(lc) = g.label(casn, cm) else { continue };
            if let Some(path) = g.path(la, lc) {
                dc.push(path.counters);
            }
        }
        let to_dc = MeanPath::of(dc.iter());
        // media call4
        let r = Engine::new(media::scenario_call4(seed, true, Discipline::TailDrop)).run();
        let mut call = [CallClass::default(); 3];
        let mut parts = 0.0;
        for c in media::energy_by_class(&r) {
            let k = match c.class {
                LinkClass::Fixed => 0,
                LinkClass::WiFi => 1,
                _ => 2,
            };
            let e = c.sum;
            let x = &mut call[k];
            x.hours += e.present_us as f64 / 3.6e9;
            x.own_tx_mb += e.media_own_tx_bytes as f64 / 1e6;
            x.fwd_tx_mb += e.media_fwd_tx_bytes as f64 / 1e6;
            x.rx_mb += e.media_rx_bytes as f64 / 1e6;
            x.sig_ops += (e.signs + e.verifies) as f64;
            parts += f64::from(c.nodes);
        }
        for x in &mut call {
            if x.hours > 0.0 {
                x.own_tx_mb /= x.hours;
                x.fwd_tx_mb /= x.hours;
                x.rx_mb /= x.hours;
                x.sig_ops /= x.hours;
            }
        }
        Self {
            sync_wakes_per_device_hour: d.wakes_per_hour_milli as f64 / 1000.0,
            sync_bytes_per_device_hour: d.bytes_per_hour as f64,
            seat_reads_per_device_hour: d.seat_reads_per_hour_milli as f64 / 1000.0,
            acts_sent_per_person_hour: pe.phone.acts_originated as f64 / person_hours,
            acts_received_per_person_hour: pe.phone.acts_received as f64 / person_hours,
            content_wakes_per_phone_hour: pe.wakes.wakes_per_device_hour(),
            relayed_share_fixed: share(NodeClass::Fixed),
            relayed_share_wifi: share(NodeClass::PhoneWifi),
            relayed_share_cell: share(NodeClass::PhoneCellular),
            held_mappings_per_phone: if ph_pairs > 0.0 { held / phones } else { 0.0 },
            path_same_metro: same,
            path_any_peer: any,
            path_to_dc: to_dc,
            call,
            call_participants: parts,
        }
    }
}

// ---------------------------------------------------------------------------
// The delta
// ---------------------------------------------------------------------------

/// The three terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    /// Added on devices people already run.
    AddedOnExisting = 0,
    /// Devices added or kept on for the mesh.
    AddedDevices = 1,
    /// Upstream energy avoided, net of the mesh's own upstream.
    AvoidedUpstream = 2,
}

impl Term {
    /// Every term.
    pub const ALL: [Self; 3] = [Self::AddedOnExisting, Self::AddedDevices, Self::AvoidedUpstream];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AddedOnExisting => "A added on existing devices",
            Self::AddedDevices => "B added devices",
            Self::AvoidedUpstream => "C avoided upstream (net)",
        }
    }
}

/// Device classes the joules land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dev {
    /// Phones.
    Phone = 0,
    /// Home node (on a router, a computer or an added box).
    HomeNode = 1,
    /// Hosted seat or relay.
    Hosted = 2,
    /// Access, core and long-haul networks.
    Network = 3,
    /// Data centres of the status quo.
    DataCentre = 4,
    /// A village node.
    Village = 5,
}

impl Dev {
    /// Every class.
    pub const ALL: [Self; 6] = [Self::Phone, Self::HomeNode, Self::Hosted, Self::Network, Self::DataCentre, Self::Village];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::HomeNode => "home node",
            Self::Hosted => "hosted",
            Self::Network => "network",
            Self::DataCentre => "data centre",
            Self::Village => "village node",
        }
    }
}

/// One joule line: `joules = count * value(row)`, entering its term with `sign`.
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution<T> {
    /// Term.
    pub term: Term,
    /// Device class.
    pub dev: Dev,
    /// The priced row.
    pub row: E,
    /// What the count is.
    pub what: &'static str,
    /// The count (in the row's unit's denominator; hours for W rows).
    pub count: T,
    /// The row's value.
    pub value: T,
    /// `count * value`.
    pub joules: T,
    /// +1 adds to the term, -1 subtracts (C's mesh-side upstream).
    pub sign: f64,
}

/// The scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scenario {
    /// Phones plus a home node on the household's existing router (per household-hour).
    RouterNode,
    /// Phones plus a home node on a computer that is on anyway.
    ComputerNode,
    /// Phones plus a home node on a new always-on box.
    AddedBox,
    /// Phones only; the hosted seat does mailbox work.
    PhonesOnly,
    /// A four-person call relayed by a neighbour's home node, against an SFU in a data centre
    /// (per participant-hour).
    CallNeighbourRelay,
    /// The same call relayed by a hosted relay in a data centre, against an SFU.
    CallHostedRelay,
    /// One household of a village behind one thin uplink with one added village node.
    Village,
}

impl Scenario {
    /// Every scenario.
    pub const ALL: [Self; 7] = [Self::RouterNode, Self::ComputerNode, Self::AddedBox, Self::PhonesOnly, Self::CallNeighbourRelay, Self::CallHostedRelay, Self::Village];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RouterNode => "home node on existing router",
            Self::ComputerNode => "home node on computer already on",
            Self::AddedBox => "home node on a new always-on box",
            Self::PhonesOnly => "phones only",
            Self::CallNeighbourRelay => "call relayed by a neighbour vs SFU",
            Self::CallHostedRelay => "call relayed in a data centre vs SFU",
            Self::Village => "village, one thin uplink",
        }
    }
    /// The unit one result is per.
    #[must_use]
    pub const fn per(self) -> &'static str {
        match self {
            Self::CallNeighbourRelay | Self::CallHostedRelay => "participant-hour",
            _ => "household-hour",
        }
    }
    const fn has_home_node(self) -> bool {
        matches!(self, Self::RouterNode | Self::ComputerNode | Self::AddedBox)
    }
}

/// The regimes (FSD 4.4 / `workload.rs`) the delta is run under. Read: `mobile_offline_churn`
/// (phone online share), `locality_share` (share of content copies whose recipient sits in the
/// signer's metro or village), `relay_offering_share` (share of relayed copies a neighbour's
/// home node carries; the rest go to a hosted relay).
pub const REGIMES: [ParameterRegime; 4] = [ParameterRegime::Optimistic, ParameterRegime::Expected, ParameterRegime::Pessimistic, ParameterRegime::Adversarial];

/// Regime name.
#[must_use]
pub const fn regime_name(r: ParameterRegime) -> &'static str {
    match r {
        ParameterRegime::Optimistic => "optimistic",
        ParameterRegime::Expected => "expected",
        ParameterRegime::Pessimistic => "pessimistic",
        ParameterRegime::Adversarial => "adversarial",
    }
}

struct Ledger<T> {
    v: Vec<Contribution<T>>,
}

impl<T: Q> Ledger<T> {
    fn put(&mut self, term: Term, dev: Dev, row: E, what: &'static str, count: T, value: T, sign: f64) {
        self.v.push(Contribution { term, dev, row, what, count, value, joules: count * value, sign });
    }
}

/// Network joules per MB over a path: per AS crossed, per IXP (priced as one AS), per km.
fn path_j<T: Q>(val: &dyn Fn(E) -> T, p: &MeanPath) -> T {
    val(E::CoreJPerMbPerAs) * T::k(p.ases_crossed + p.ixp_crossings) + val(E::LongHaulJPerMbPer1000Km) * T::k(p.long_haul_km / 1000.0)
}

/// Every contribution of `s` under regime `r`, valued by `val` (a point or an interval).
pub fn contributions<T: Q>(val: &dyn Fn(E) -> T, k: &Counters, s: Scenario, r: ParameterRegime) -> Vec<Contribution<T>> {
    use Dev as D;
    use Term::{AddedDevices as B, AddedOnExisting as A, AvoidedUpstream as C};
    let rp = RegimeParameters::for_regime(r);
    let mut l = Ledger { v: Vec::new() };
    let one = T::k(1.0);
    let on = T::k(1.0 - rp.mobile_offline_churn);
    let loc = T::k(rp.locality_share);
    let relay_offer = T::k(rp.relay_offering_share);
    let w = val(E::WifiShare);
    let c = one - w;
    match s {
        Scenario::CallNeighbourRelay | Scenario::CallHostedRelay => {
            let hosted = s == Scenario::CallHostedRelay;
            let n = k.call_participants.max(2.0);
            let total_h: f64 = k.call.iter().map(|x| x.hours).sum::<f64>().max(1e-9);
            // the stream rate one publisher sends one receiver: distinct media over N - 1
            let mut stream = 0.0;
            let mut own = 0.0;
            let mut fwd = 0.0;
            for x in &k.call {
                let f = x.hours / total_h;
                stream += f * x.rx_mb / (n - 1.0);
                own += f * x.own_tx_mb;
                fwd += f * x.fwd_tx_mb;
            }
            let per_mb = [E::HomeNodeJPerMb, E::PhoneJPerMbWifi, E::PhoneJPerMbCell];
            let devs = [D::HomeNode, D::Phone, D::Phone];
            for (i, x) in k.call.iter().enumerate() {
                if x.hours <= 0.0 {
                    continue;
                }
                let f = x.hours / total_h;
                // device bytes: the mesh's own copies (and, with a neighbour relay, forwarded
                // copies) against the SFU call's one upload and N - 1 downloads; with a hosted
                // relay the device uploads one copy, as to an SFU
                let mesh_mb = if hosted { stream + x.rx_mb } else { x.own_tx_mb + x.fwd_tx_mb + x.rx_mb };
                let sq_mb = stream * n;
                l.put(A, devs[i], per_mb[i], "call MB on this class's radio or NIC (mesh), per participant-hour", T::k(f * mesh_mb), val(per_mb[i]), 1.0);
                l.put(A, devs[i], per_mb[i], "call MB on this class's radio or NIC (SFU status quo), per participant-hour", T::k(f * sq_mb), val(per_mb[i]), -1.0);
                let sig = if i == 0 { E::HomeNodeJPerChunkSigOp } else { E::PhoneJPerChunkSigOp };
                l.put(A, devs[i], sig, "media signs and verifies (the SFU call signs none)", T::k(f * x.sig_ops), val(sig), 1.0);
            }
            // access per MB: the call's classes (Fixed and Wi-Fi ride fixed access)
            let fixed_h: f64 = (k.call[0].hours + k.call[1].hours) / total_h;
            let acc = val(E::AccessFixedJPerMb) * T::k(fixed_h) + val(E::AccessMobileJPerMb) * T::k(1.0 - fixed_h);
            let dc_path = path_j(val, &k.path_to_dc);
            let peer_path = path_j(val, &k.path_same_metro) * loc + path_j(val, &k.path_any_peer) * (one - loc);
            // status quo: one upload and N - 1 downloads per participant, each over access and
            // the path to the data centre; the SFU's own work
            l.put(C, D::DataCentre, E::DcCallWhPerParticipantHour, "SFU participant-hours (Wh -> J x3600)", T::k(3600.0), val(E::DcCallWhPerParticipantHour), 1.0);
            l.put(C, D::Network, E::AccessFixedJPerMb, "SFU call MB on access (blended by class)", T::k(stream * n), acc, 1.0);
            l.put(C, D::Network, E::CoreJPerMbPerAs, "SFU call MB over the path to the data centre", T::k(stream * n), dc_path, 1.0);
            if hosted {
                // one upload to the hosted relay and N - 1 downloads from it, as the SFU's
                l.put(C, D::Network, E::AccessFixedJPerMb, "mesh call MB on access, to and from the hosted relay", T::k(stream * n), acc, -1.0);
                l.put(C, D::Network, E::CoreJPerMbPerAs, "mesh call MB over the path to the hosted relay", T::k(stream * n), dc_path, -1.0);
                // a hosted relay that forwards every stream to every member is an SFU in a data
                // centre: priced with the SFU's own row, so B and the SFU line of C cancel and
                // the delta is the mesh's crypto and seal work on the devices
                l.put(B, D::Hosted, E::DcCallWhPerParticipantHour, "hosted relay participant-hours, priced as the SFU (Wh -> J x3600)", T::k(3600.0), val(E::DcCallWhPerParticipantHour), 1.0);
            } else {
                // every copy on the wire crosses two access ends and a peer path
                l.put(C, D::Network, E::AccessFixedJPerMb, "mesh call MB on access (sender and receiver ends)", T::k(2.0 * (own + fwd)), acc, -1.0);
                l.put(C, D::Network, E::CoreJPerMbPerAs, "mesh call MB over peer paths (local share by regime)", T::k(own + fwd), peer_path, -1.0);
            }
        }
        _ => {
            let village = s == Scenario::Village;
            let h = val(E::PersonsPerHousehold);
            let phones_on = h * on;
            // ---- A: phones run the edge schedule ---------------------------------------
            let sched = val(E::PhoneScheduleShare);
            let wakes = T::k(k.sync_wakes_per_device_hour) * sched * phones_on;
            l.put(A, D::Phone, E::PhoneWakeJWifi, "edge round and Pull wake-ups on Wi-Fi (seat_hooks)", wakes * w, val(E::PhoneWakeJWifi), 1.0);
            if !village {
                l.put(A, D::Phone, E::PhoneWakeJCell, "edge round and Pull wake-ups on cellular (seat_hooks)", wakes * c, val(E::PhoneWakeJCell), 1.0);
            }
            let cw = T::k(k.content_wakes_per_phone_hour) * phones_on * val(E::MeshExtraContentWakes);
            l.put(A, D::Phone, E::PhoneWakeJWifi, "extra content wake-ups on Wi-Fi (people_hooks x share)", cw * w, val(E::PhoneWakeJWifi), 1.0);
            if !village {
                l.put(A, D::Phone, E::PhoneWakeJCell, "extra content wake-ups on cellular", cw * c, val(E::PhoneWakeJCell), 1.0);
            }
            let sync_mb = T::k(2.0 * k.sync_bytes_per_device_hour / 1e6) * sched * phones_on;
            let ka_mb = T::k(k.held_mappings_per_phone / 1e6) * val(E::KeepaliveBytesPerMappingHour) * phones_on;
            let (wb, cb) = if village { (one, T::k(0.0)) } else { (w, c) };
            l.put(A, D::Phone, E::PhoneJPerMbWifi, "edge sync MB both ways on Wi-Fi (seat_hooks)", sync_mb * wb, val(E::PhoneJPerMbWifi), 1.0);
            l.put(A, D::Phone, E::PhoneJPerMbCell, "edge sync MB both ways on cellular", sync_mb * cb, val(E::PhoneJPerMbCell), 1.0);
            l.put(A, D::Phone, E::PhoneJPerMbWifi, "NAT keepalive MB on Wi-Fi (reach held mappings)", ka_mb * wb, val(E::PhoneJPerMbWifi), 1.0);
            l.put(A, D::Phone, E::PhoneJPerMbCell, "NAT keepalive MB on cellular", ka_mb * cb, val(E::PhoneJPerMbCell), 1.0);
            let sigs = T::k(k.acts_sent_per_person_hour + k.acts_received_per_person_hour) * h;
            l.put(A, D::Phone, E::PhoneJPerSigOp, "signs and verifies, one per act sent and per copy received (people_hooks)", sigs, val(E::PhoneJPerSigOp), 1.0);
            // content MB per household-hour (copies received); the status quo moves the same
            let mb_act = val(E::BytesPerAct) * T::k(1e-6);
            let copies = T::k(k.acts_received_per_person_hour) * h;
            let sends = T::k(k.acts_sent_per_person_hour) * h;
            let relayed = T::k(k.relayed_share_wifi) * w + T::k(k.relayed_share_cell) * c;
            // ---- home node ---------------------------------------------------------------
            if s.has_home_node() {
                let (row, term) = match s {
                    Scenario::RouterNode => (E::RouterNodeW, A),
                    Scenario::ComputerNode => (E::ComputerNodeW, A),
                    _ => (E::AddedBoxW, B),
                };
                l.put(term, D::HomeNode, row, "home-node hours (W x 3600 s)", T::k(3600.0), val(row), 1.0);
                let node_mb = T::k(2.0 * k.sync_bytes_per_device_hour / 1e6);
                l.put(A, D::HomeNode, E::HomeNodeJPerMb, "home node's own edge sync MB both ways", node_mb, val(E::HomeNodeJPerMb), 1.0);
                // INFERRED: bytes forwarded for others per device is not a counter anywhere;
                // a node carries, on average, its household's relayed copies times the relay
                // offering share (in and out)
                let fwd_mb = copies * mb_act * relayed * relay_offer * T::k(2.0);
                l.put(A, D::HomeNode, E::HomeNodeJPerMb, "MB relayed for others, in and out (estimate: relayed share x copies; no counter)", fwd_mb, val(E::HomeNodeJPerMb), 1.0);
            }
            // ---- B: hosted seat share, village node ----------------------------------------
            let devices = h + if s.has_home_node() { one } else { T::k(0.0) };
            let seat_share = devices * val(E::DevicesPerSeat).recip();
            l.put(B, D::Hosted, E::HostedSeatW, "the household's share of a seat's hours (devices / devices per seat x 3600 s)", seat_share * T::k(3600.0), val(E::HostedSeatW), 1.0);
            l.put(B, D::Hosted, E::HostedJPerRead, "seat store reads the household's devices cause (seat_hooks)", devices * T::k(k.seat_reads_per_device_hour), val(E::HostedJPerRead), 1.0);
            if !s.has_home_node() {
                // relayed copies a neighbour does not carry go to a hosted relay
                let hosted_mb = copies * mb_act * relayed * (one - relay_offer) * T::k(2.0);
                l.put(B, D::Hosted, E::HostedJPerMb, "relayed MB on a hosted relay (no home node; estimate)", hosted_mb, val(E::HostedJPerMb), 1.0);
            }
            if village {
                let per_hh = val(E::VillageHouseholds).recip() * T::k(3600.0);
                l.put(B, D::Village, E::VillageNodeW, "the household's share of the village node's hours", per_hh, val(E::VillageNodeW), 1.0);
            }
            // ---- C: avoided upstream ------------------------------------------------------
            l.put(C, D::DataCentre, E::DcMsgWhPerPersonHour, "person-hours of the messaging workload (Wh -> J x3600)", h * T::k(3600.0), val(E::DcMsgWhPerPersonHour), 1.0);
            let acc = if village { val(E::VillageUplinkJPerMb) } else { val(E::AccessFixedJPerMb) * w + val(E::AccessMobileJPerMb) * c };
            let acc_row = if village { E::VillageUplinkJPerMb } else { E::AccessFixedJPerMb };
            let dc_path = path_j(val, &k.path_to_dc);
            // status quo: each act up once, each copy down once, over access and the DC path
            let sq_mb = (sends + copies) * mb_act;
            l.put(C, D::Network, acc_row, "status-quo content MB on access (or the village uplink)", sq_mb, acc, 1.0);
            l.put(C, D::Network, E::CoreJPerMbPerAs, "status-quo content MB over the path to the data centre", sq_mb, dc_path, 1.0);
            // mesh: each copy crosses the sender's and the receiver's access and a peer path;
            // in the village, local copies stay on the village LAN (devices only)
            let far = if village { one - loc } else { one };
            let peer_path = if village { path_j(val, &k.path_any_peer) } else { path_j(val, &k.path_same_metro) * loc + path_j(val, &k.path_any_peer) * (one - loc) };
            let mesh_copy_mb = copies * mb_act * far * (one + relayed);
            l.put(C, D::Network, acc_row, "mesh content MB on access, both ends (or the uplink)", mesh_copy_mb * T::k(2.0), acc, -1.0);
            l.put(C, D::Network, E::CoreJPerMbPerAs, "mesh content MB over peer paths", mesh_copy_mb, peer_path, -1.0);
            // the mesh's sync traffic to the seat (the status quo's push channel is not counted
            // on either side: phones keep it for other apps)
            let sync_up = T::k(2.0 * k.sync_bytes_per_device_hour / 1e6) * (phones_on * sched + if s.has_home_node() { one } else { T::k(0.0) });
            l.put(C, D::Network, acc_row, "mesh sync MB to the seat on access (or the uplink)", sync_up, acc, -1.0);
            l.put(C, D::Network, E::CoreJPerMbPerAs, "mesh sync MB over the path to the seat", sync_up, dc_path, -1.0);
        }
    }
    l.v
}

/// The three terms and their sum, as an interval each.
#[derive(Debug, Clone, PartialEq)]
pub struct Delta {
    /// Scenario.
    pub scenario: Scenario,
    /// Regime.
    pub regime: ParameterRegime,
    /// Point (every row at its default) per term, [`Term`] order.
    pub point: [f64; 3],
    /// Per term, the lowest and highest value found over the rows' ends (attained by a real
    /// combination of input ends: the seat_calibration corners refined by a vertex search).
    pub bounds: [Iv; 3],
    /// `A + B - C`, point.
    pub net: f64,
    /// `A + B - C`, lowest and highest found the same way (searched on the net itself, not
    /// assembled from the term bounds).
    pub net_bounds: Iv,
    /// Interval-arithmetic enclosure of the net: guaranteed to contain every combination, wider
    /// than `net_bounds` where an input enters more than one line.
    pub net_enclosure: Iv,
    /// Point per device class ([`Dev`] order), signed as it enters the net.
    pub by_dev: [f64; 6],
    /// The weakest class of any row priced.
    pub class: Class,
}

fn fold<T: Q>(v: &[Contribution<T>]) -> [T; 3] {
    let mut t = [T::k(0.0); 3];
    for c in v {
        let j = if c.sign < 0.0 { -c.joules } else { c.joules };
        t[c.term as usize] = t[c.term as usize] + j;
    }
    t
}

/// The point net of `s` under `r` at `p`.
#[must_use]
pub fn net_at(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime) -> f64 {
    let t = fold(&contributions(&|e| p.get(e), k, s, r));
    t[0] + t[1] - t[2]
}

/// The four outputs at `p`: A, B, C, net.
fn outputs(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime) -> [f64; 4] {
    let t = fold(&contributions(&|e| p.get(e), k, s, r));
    [t[0], t[1], t[2], t[0] + t[1] - t[2]]
}

/// The lowest (`sense = -1`) or highest (`+1`) value of output `o` over the rows' ends: start
/// at seat_calibration's corner (every row at the end that moves `o` that way one at a time),
/// then sweep the rows, keeping any end that improves `o`, until a sweep changes nothing. The
/// result is attained by a real combination of input ends.
fn vertex_search(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime, o: usize, sense: f64) -> f64 {
    let free: Vec<E> = E::ALL.iter().copied().filter(|&e| spec(e).lo < spec(e).hi).collect();
    let mut x = p.clone();
    for &e in &free {
        let sp = spec(e);
        let a = outputs(&p.with(e, sp.lo), k, s, r)[o];
        let b = outputs(&p.with(e, sp.hi), k, s, r)[o];
        x.set(e, if sense * b >= sense * a { sp.hi } else { sp.lo });
    }
    let mut best = outputs(&x, k, s, r)[o];
    for _ in 0..4 {
        let mut moved = false;
        for &e in &free {
            let sp = spec(e);
            for end in [sp.lo, sp.hi] {
                if x.get(e) == end {
                    continue;
                }
                let y = outputs(&x.with(e, end), k, s, r)[o];
                if sense * y > sense * best + 1e-9 * best.abs().max(1.0) {
                    best = y;
                    x.set(e, end);
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
    let pt = outputs(p, k, s, r)[o];
    if sense > 0.0 { best.max(pt) } else { best.min(pt) }
}

/// The delta of `s` under `r`: point at the defaults, bounds over every row's interval.
#[must_use]
pub fn delta(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime) -> Delta {
    let pts = contributions(&|e| p.get(e), k, s, r);
    let ivs = contributions(&|e| { let sp = spec(e); Iv::new(sp.lo.min(p.get(e)), sp.hi.max(p.get(e))) }, k, s, r);
    let t = fold(&pts);
    let enc = fold(&ivs);
    let b: [Iv; 4] = core::array::from_fn(|o| Iv::new(vertex_search(p, k, s, r, o, -1.0), vertex_search(p, k, s, r, o, 1.0)));
    let mut by_dev = [0.0; 6];
    let mut class = Class::Derived;
    for c in &pts {
        let j = c.sign * c.joules * if c.term == Term::AvoidedUpstream { -1.0 } else { 1.0 };
        by_dev[c.dev as usize] += j;
        class = class.weaker(spec(c.row).class);
    }
    Delta { scenario: s, regime: r, point: t, bounds: [b[0], b[1], b[2]], net: t[0] + t[1] - t[2], net_bounds: b[3], net_enclosure: enc[0] + enc[1] - enc[2], by_dev, class }
}

/// One break-even row: the value of `row` (others at their defaults) where the net is zero.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakEven {
    /// The input.
    pub row: E,
    /// The net at the row's low end and high end.
    pub net_at_lo: f64,
    /// At the high end.
    pub net_at_hi: f64,
    /// The zero, if one lies in `[0, 100 x hi]`.
    pub at: Option<f64>,
    /// Whether the zero lies inside the row's own interval (the sign is open on this input).
    pub inside: bool,
}

/// Every row whose value moves the net, with its zero.
#[must_use]
pub fn break_even(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime) -> Vec<BreakEven> {
    let mut out = Vec::new();
    for &e in E::ALL {
        let sp = spec(e);
        let f = |x: f64| net_at(&p.with(e, x), k, s, r);
        let (a, b) = (f(sp.lo), f(sp.hi));
        // a fixed row (lo == hi) is a design lever: test whether it moves the net over [0, hi]
        let (ta, tb) = if sp.lo == sp.hi { (f(0.0), f(sp.hi.max(1.0))) } else { (a, b) };
        if (ta - tb).abs() < 1e-12 * (ta.abs() + tb.abs() + 1.0) {
            continue;
        }
        // scan for a sign change over [min(0, lo), 100 hi]
        let lo0 = sp.lo.min(0.0);
        let hi0 = if sp.hi > 0.0 { sp.hi * 100.0 } else { 1.0 };
        let steps = 400;
        let mut at = None;
        let mut prev = (lo0, f(lo0));
        for i in 1..=steps {
            let x = lo0 + (hi0 - lo0) * f64::from(i) / f64::from(steps);
            let y = f(x);
            if prev.1 == 0.0 || prev.1.signum() != y.signum() {
                let (mut l, mut h) = (prev.0, x);
                for _ in 0..60 {
                    let m = 0.5 * (l + h);
                    if f(m).signum() == f(l).signum() { l = m } else { h = m }
                }
                at = Some(0.5 * (l + h));
                break;
            }
            prev = (x, y);
        }
        let inside = at.is_some_and(|z| z >= sp.lo && z <= sp.hi);
        out.push(BreakEven { row: e, net_at_lo: a, net_at_hi: b, at, inside });
    }
    out
}

// ---------------------------------------------------------------------------
// Report and CLI
// ---------------------------------------------------------------------------

fn j(x: f64) -> String {
    if x.abs() >= 100.0 { format!("{x:.0}") } else if x.abs() >= 1.0 { format!("{x:.1}") } else { format!("{x:.3}") }
}

/// The markdown report (`cewp-cli energy [seed]`).
#[must_use]
pub fn report(seed: u64) -> String {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut s = String::new();
    s.push_str(&format!("# Energy delta from the status quo (seed {seed})\n\n"));
    s.push_str("Joules per household-hour (calls: per participant-hour). delta = A + B - C; positive is a cost, negative a saving. Bounds per term and on the net: the lowest and highest values found over every row's interval ends (seat_calibration's corners refined by a vertex search; each bound is attained by a real combination of input ends). The interval-arithmetic enclosure of the net, guaranteed but wider, is in its own column. Every power row is MODEL or FRONTIER: no figure here is MEASURED, and the class of each result is the weakest row it prices.\n\n");
    s.push_str("## Counters read (DERIVED from the packages' events)\n\n");
    s.push_str(&format!("- seat_hooks: {:.2} wake-ups / device-hour, {:.0} bytes / device-hour each way, {:.1} seat reads / device-hour\n", k.sync_wakes_per_device_hour, k.sync_bytes_per_device_hour, k.seat_reads_per_device_hour));
    s.push_str(&format!("- people_hooks (Poisson, 2,000-person stand-in, 1 day): {:.3} acts sent and {:.2} copies received per person-hour, {:.2} content wake-ups per phone-hour\n", k.acts_sent_per_person_hour, k.acts_received_per_person_hour, k.content_wakes_per_phone_hour));
    s.push_str(&format!("- reach hooks: relayed share of pairs fixed {:.3}, phone Wi-Fi {:.3}, phone cellular {:.3}; held NAT mappings per phone {:.2}\n", k.relayed_share_fixed, k.relayed_share_wifi, k.relayed_share_cell, k.held_mappings_per_phone));
    for (name, m) in [("same-metro peer", k.path_same_metro), ("any peer", k.path_any_peer), ("to data centre", k.path_to_dc)] {
        s.push_str(&format!("- netgraph {name} paths (n={}): {:.2} ASes crossed, {:.2} IXPs, {:.0} long-haul km\n", m.n, m.ases_crossed, m.ixp_crossings, m.long_haul_km));
    }
    for (i, n) in ["fixed", "Wi-Fi", "cellular"].iter().enumerate() {
        let c = k.call[i];
        s.push_str(&format!("- media call4 {n}: {:.4} participant-hours, own tx {:.0} MB/h, forwarded {:.0} MB/h, rx {:.0} MB/h, {:.0} sig ops/h\n", c.hours, c.own_tx_mb, c.fwd_tx_mb, c.rx_mb, c.sig_ops));
    }
    s.push_str("\n## The delta by scenario and regime\n\n| scenario | per | regime | A added on existing | B added devices | C avoided upstream (net) | delta = A + B - C | enclosure | sign | class |\n|---|---|---|---|---|---|---|---|---|---|\n");
    for sc in Scenario::ALL {
        for r in REGIMES {
            let d = delta(&p, &k, sc, r);
            let iv = |i: usize| format!("{} [{}, {}]", j(d.point[i]), j(d.bounds[i].lo), j(d.bounds[i].hi));
            let sign = if d.net_bounds.lo > 0.0 { "cost (interval > 0)" } else if d.net_bounds.hi < 0.0 { "saving (interval < 0)" } else if d.net > 0.0 { "open (point > 0)" } else { "open (point < 0)" };
            s.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} [{}, {}] | [{}, {}] | {} | {} |\n", sc.as_str(), sc.per(), regime_name(r), iv(0), iv(1), iv(2), j(d.net), j(d.net_bounds.lo), j(d.net_bounds.hi), j(d.net_enclosure.lo), j(d.net_enclosure.hi), sign, d.class.as_str()));
        }
    }
    s.push_str("\n## By device class (expected regime, point, J, signed as it enters delta)\n\n| scenario |");
    for dv in Dev::ALL {
        s.push_str(&format!(" {} |", dv.as_str()));
    }
    s.push_str("\n|---|---|---|---|---|---|---|\n");
    for sc in Scenario::ALL {
        let d = delta(&p, &k, sc, ParameterRegime::Expected);
        s.push_str(&format!("| {} |", sc.as_str()));
        for x in d.by_dev {
            s.push_str(&format!(" {} |", j(x)));
        }
        s.push('\n');
    }
    s.push_str("\n## Break-even (expected regime): the value of one input, others at default, where delta = 0\n\nListed per scenario: inputs whose zero lies inside their own interval (they decide the sign), the three largest movers, and `phone.schedule_share` (fixed at 1 by the edge today; its zero is the design lever).\n\n| scenario | input | class | interval | delta at lo / hi | zero at | inside interval |\n|---|---|---|---|---|---|---|\n");
    for sc in Scenario::ALL {
        let mut be = break_even(&p, &k, sc, ParameterRegime::Expected);
        be.sort_by(|a, b| (b.net_at_hi - b.net_at_lo).abs().partial_cmp(&(a.net_at_hi - a.net_at_lo).abs()).unwrap_or(core::cmp::Ordering::Equal));
        for (i, b) in be.iter().enumerate() {
            if !(b.inside || i < 3 || b.row == E::PhoneScheduleShare) {
                continue;
            }
            let sp = spec(b.row);
            s.push_str(&format!("| {} | `{}` | {} | [{}, {}] {} | {} / {} | {} | {} |\n", sc.as_str(), sp.name, sp.class.as_str(), sp.lo, sp.hi, sp.unit, j(b.net_at_lo), j(b.net_at_hi), b.at.map_or("none in [0, 100 x hi]".to_string(), |z| format!("{z:.4}")), if b.inside { "yes" } else { "no" }));
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Globe views (flat buffers)
// ---------------------------------------------------------------------------

/// Floats per row of [`view_delta`].
pub const DELTA_STRIDE: usize = 15;

/// The delta per scenario and regime, `DELTA_STRIDE` f32 per row, scenarios in
/// [`Scenario::ALL`] order, regimes in [`REGIMES`] order within each:
/// `[scenario, regime, per (0 household-hour, 1 participant-hour),
///   A, A_lo, A_hi, B, B_lo, B_hi, C, C_lo, C_hi, net, net_lo, net_hi]` in joules; then the
/// caller reads the sign from `net_lo > 0` (cost), `net_hi < 0` (saving), else open. Every row's
/// class is [`view_delta_class`] (MODEL or FRONTIER; never MEASURED).
#[must_use]
pub fn view_delta(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut v = Vec::with_capacity(Scenario::ALL.len() * REGIMES.len() * DELTA_STRIDE);
    for (si, sc) in Scenario::ALL.iter().enumerate() {
        for (ri, r) in REGIMES.iter().enumerate() {
            let d = delta(&p, &k, *sc, *r);
            v.extend_from_slice(&[si as f32, ri as f32, if sc.per() == "participant-hour" { 1.0 } else { 0.0 }]);
            for t in 0..3 {
                v.extend_from_slice(&[d.point[t] as f32, d.bounds[t].lo as f32, d.bounds[t].hi as f32]);
            }
            v.extend_from_slice(&[d.net as f32, d.net_bounds.lo as f32, d.net_bounds.hi as f32]);
        }
    }
    v
}

/// The class code of every [`view_delta`] row, one f32 each (`seat_hooks::class` codes).
#[must_use]
pub fn view_delta_class(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    Scenario::ALL.iter().flat_map(|sc| REGIMES.iter().map(move |r| (*sc, *r))).map(|(sc, r)| delta(&p, &k, sc, r).class.code()).collect()
}

/// Floats per row of [`view_break_even`].
pub const BREAK_EVEN_STRIDE: usize = 8;

/// The break-even table at the expected regime, `BREAK_EVEN_STRIDE` f32 per row:
/// `[scenario, row index into ENERGY_SPECS, class code, lo, hi, net_at_lo, net_at_hi,
///   zero (NaN when none in [0, 100 x hi])]`. A row is "deciding" when `lo <= zero <= hi`.
/// Row names are `ENERGY_SPECS[row].name`.
#[must_use]
pub fn view_break_even(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut v = Vec::new();
    for (si, sc) in Scenario::ALL.iter().enumerate() {
        for b in break_even(&p, &k, *sc, ParameterRegime::Expected) {
            let sp = spec(b.row);
            v.extend_from_slice(&[si as f32, b.row as usize as f32, sp.class.code(), sp.lo as f32, sp.hi as f32, b.net_at_lo as f32, b.net_at_hi as f32, b.at.map_or(f32::NAN, |z| z as f32)]);
        }
    }
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn counters() -> Counters {
        Counters::from_hooks(3)
    }

    #[test]
    fn every_row_is_ordered_and_classed() {
        for s in ENERGY_SPECS {
            assert!(s.lo <= s.mid && s.mid <= s.hi, "{}", s.name);
            assert!(matches!(s.class, Class::Model | Class::Frontier), "{}: no row is MEASURED without a dataset", s.name);
            assert!(!s.why.is_empty());
        }
        // the rows the brief marks as ours to measure say how
        for e in [E::PhoneWakeJWifi, E::PhoneWakeJCell, E::HomeNodeJPerMb, E::PhoneJPerSigOp] {
            assert!(!spec(e).needs_measurement.is_empty(), "{}", spec(e).name);
        }
    }

    #[test]
    fn conservation_every_joule_is_a_count_times_a_row() {
        let k = counters();
        let p = EnergyParams::default();
        for sc in Scenario::ALL {
            for r in REGIMES {
                let v = contributions(&|e| p.get(e), &k, sc, r);
                let mut t = [0.0; 3];
                for c in &v {
                    // the priced value is the row's value (or a path/blend built only from rows)
                    assert!((c.joules - c.count * c.value).abs() <= 1e-9 * c.joules.abs().max(1.0));
                    if !matches!(c.row, E::AccessFixedJPerMb | E::CoreJPerMbPerAs) {
                        assert_eq!(c.value, p.get(c.row), "{:?} {}", c.row, c.what);
                    }
                    t[c.term as usize] += c.sign * c.joules;
                }
                let d = delta(&p, &k, sc, r);
                for i in 0..3 {
                    assert!((t[i] - d.point[i]).abs() <= 1e-9 * t[i].abs().max(1.0));
                }
                assert!((d.net - (t[0] + t[1] - t[2])).abs() <= 1e-9 * d.net.abs().max(1.0));
                let by: f64 = d.by_dev.iter().sum();
                assert!((by - d.net).abs() <= 1e-6 * d.net.abs().max(1.0), "device split sums to the net");
            }
        }
    }

    #[test]
    fn bounds_enclose_the_point_and_every_corner_tried() {
        let k = counters();
        let p = EnergyParams::default();
        for sc in Scenario::ALL {
            let d = delta(&p, &k, sc, ParameterRegime::Expected);
            assert!(d.net_bounds.lo <= d.net && d.net <= d.net_bounds.hi);
            assert!(d.net_enclosure.lo <= d.net_bounds.lo + 1e-6 && d.net_bounds.hi <= d.net_enclosure.hi + 1e-6, "{sc:?}: the vertex range sits inside the enclosure");
            for t in 0..3 {
                assert!(d.bounds[t].lo <= d.point[t] + 1e-9 && d.point[t] <= d.bounds[t].hi + 1e-9);
            }
            for &e in E::ALL {
                for x in [spec(e).lo, spec(e).hi] {
                    let n = net_at(&p.with(e, x), &k, sc, ParameterRegime::Expected);
                    assert!(n >= d.net_bounds.lo - 1e-6 && n <= d.net_bounds.hi + 1e-6, "{sc:?} {e:?}");
                }
            }
        }
    }

    #[test]
    fn monotone_where_the_formula_says() {
        let k = counters();
        let p = EnergyParams::default();
        let r = ParameterRegime::Expected;
        for sc in [Scenario::RouterNode, Scenario::AddedBox, Scenario::PhonesOnly] {
            // more joules per wake-up or per added watt never lowers the net
            for e in [E::PhoneWakeJCell, E::PhoneWakeJWifi, E::HostedSeatW, E::PhoneJPerSigOp] {
                assert!(net_at(&p.with(e, spec(e).hi), &k, sc, r) >= net_at(&p.with(e, spec(e).lo), &k, sc, r), "{sc:?} {e:?}");
            }
            // more avoided data-centre energy never raises it
            assert!(net_at(&p.with(E::DcMsgWhPerPersonHour, 0.5), &k, sc, r) <= net_at(&p.with(E::DcMsgWhPerPersonHour, 0.02), &k, sc, r));
        }
        // the added box costs more than the router for the same household
        assert!(net_at(&p, &k, Scenario::AddedBox, r) > net_at(&p, &k, Scenario::RouterNode, r));
    }

    #[test]
    fn the_sign_flips_across_its_break_even() {
        let k = counters();
        let p = EnergyParams::default();
        let r = ParameterRegime::Expected;
        let mut flips = 0;
        for sc in Scenario::ALL {
            for b in break_even(&p, &k, sc, r) {
                if let Some(z) = b.at {
                    let sp = spec(b.row);
                    let eps = (sp.hi - sp.lo).abs().max(1e-6) * 1e-3;
                    let below = net_at(&p.with(b.row, z - eps), &k, sc, r);
                    let above = net_at(&p.with(b.row, z + eps), &k, sc, r);
                    assert!(below.signum() != above.signum() || below.abs() < 1e-6 || above.abs() < 1e-6, "{sc:?} {:?} at {z}", b.row);
                    flips += 1;
                }
            }
        }
        assert!(flips > 0, "some input must be able to flip some scenario's sign");
    }

    #[test]
    fn the_headline_can_come_out_either_way() {
        let k = counters();
        let p = EnergyParams::default();
        let mut pos = false;
        let mut neg = false;
        for sc in Scenario::ALL {
            for r in REGIMES {
                let d = delta(&p, &k, sc, r);
                pos |= d.net_bounds.hi > 0.0;
                neg |= d.net_bounds.lo < 0.0;
            }
        }
        assert!(pos && neg);
    }

    #[test]
    fn deterministic_by_seed() {
        assert_eq!(Counters::from_hooks(5), Counters::from_hooks(5));
        assert_eq!(view_delta(5), view_delta(5));
        assert_eq!(view_delta(5).len(), Scenario::ALL.len() * REGIMES.len() * DELTA_STRIDE);
        assert_eq!(view_break_even(5).len() % BREAK_EVEN_STRIDE, 0);
    }

    #[test]
    fn counters_are_read_from_the_hooks() {
        let k = counters();
        assert!((k.sync_wakes_per_device_hour - 126.75).abs() < 0.01, "seat_hooks schedule: {}", k.sync_wakes_per_device_hour);
        assert!(k.sync_bytes_per_device_hour > 0.0);
        assert!(k.acts_received_per_person_hour > 0.0);
        assert!(k.path_to_dc.n > 0 && k.path_any_peer.n > 0);
        assert!(k.call_participants >= 2.0);
    }

    #[test]
    #[ignore = "prints the report; run with --ignored --nocapture"]
    fn print_report() {
        println!("{}", report(1));
    }
}
