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
    /// CIRISHolon's row this value is taken from, with its basis (`""`: the value is ours).
    pub source: &'static str,
    /// Reasoning.
    pub why: &'static str,
}

/// CIRISHolon POWER-0 (`conformance/scene/tables/power0/power_table.json`, version 3, read at
/// CIRISHolon `origin/main` c6677a1, sha256 d103390a...). Every row's verdict is "unresolved"
/// (priors from documents); `EnergySpec::source` names the row and carries its basis.
pub const POWER0_PIN: &str = "CIRISHolon c6677a147076bdd8472e4b0761585e9f3d0961cc conformance/scene/tables/power0/power_table.json v3 sha256 d103390ab4f7ae9c8bf45f2a071cc3b87159f5cf84796f1e65f3facd7fa58b5f";
/// CIRISHolon DEVICES-0 (US 2020): `conformance/scene/tables/devices0_us/device_table.json` v1.
pub const DEVICES0_PIN: &str = "CIRISHolon c6677a147076bdd8472e4b0761585e9f3d0961cc conformance/scene/tables/devices0_us/device_table.json v1 sha256 e4869380d2133be1ed532a0e15cb13d497f7050102924eb72bbc845eed5a6620";

macro_rules! energy_rows {
    ($( $idx:ident = $name:literal, $dev:literal, $state:literal, $unit:literal, [$lo:expr, $mid:expr, $hi:expr], $cls:ident, $on:expr, $year:literal, $meas:literal, $src:literal, $why:literal; )*) => {
        /// Index into [`ENERGY_SPECS`].
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum E { $( $idx, )* }
        /// Every input row, in index order.
        pub const ENERGY_SPECS: &[EnergySpec] = &[
            $( EnergySpec { name: $name, device: $dev, state: $state, unit: $unit, lo: $lo, mid: $mid, hi: $hi, class: Class::$cls, would_be_on_anyway: $on, year: $year, needs_measurement: $meas, source: $src, why: $why }, )*
        ];
        impl E {
            /// Every row.
            pub const ALL: &'static [E] = &[ $( E::$idx, )* ];
        }
    };
}

energy_rows! {
    // ---- phone --------------------------------------------------------------------------------
    PhoneDeepIdleW = "phone.deep_idle_w", "phone", "deep idle (screen off, radio idle)", "W", [0.008, 0.015, 0.03], Model, Some(true), 2026, "", "",
        "order of a modern smartphone's suspended draw (tens of mW). Listed for completeness: the mesh does not change deep idle, so no term charges it";
    PhoneWakeJWifi = "phone.wake_j.wifi", "phone", "one small exchange on Wi-Fi, wake-up and tail included", "J", [0.02, 0.04, 0.2], Model, Some(false), 2012,
        "phone on a power monitor: J per edge round on Wi-Fi (screen off, Doze off and on), net of the idle baseline",
        "POWER-0 row 8 (Huang et al. 2012, MEASURED on one handset; tail 0.24 s at 119 mW; verdict unresolved)",
        "one 2012 handset; a current phone's Wi-Fi power save is of the same order";
    PhoneCellPromoJ = "phone.cell.promotion_j", "phone", "one exchange on a carrier: promotion and transfer, tail excluded", "J", [0.2, 0.46, 1.5], Model, Some(false), 2012,
        "phone on a power monitor: energy of an edge round on LTE and 5G NSA up to the end of transfer",
        "POWER-0 row 6 (Huang et al. 2012: 12.76 J per small packet incl. an 11.6 s tail at 1.06 W, so 12.76 - 11.6 x 1.06 = 0.46 J; MEASURED on one handset, verdict unresolved)",
        "the whole-exchange figure less its tail";
    PhoneCellTailS = "phone.cell.tail_s", "phone", "radio held in its high-power tail after the last packet", "s", [2.0, 11.6, 12.0], Model, Some(false), 2012,
        "the carrier's inactivity timer and C-DRX as seen on a current phone's power trace",
        "POWER-0 row 6 (11.6 s, 2012) and its note (two 5G phones 2021: still about 10 s); the low end is fast dormancy",
        "a regime, not a constant: the carrier sets it";
    PhoneCellTailW = "phone.cell.tail_w", "phone", "radio power in the tail", "W", [0.5, 1.06, 1.5], Model, Some(false), 2012, "",
        "POWER-0 row 6 (1,060 mW on one 2012 LTE handset)", "C-DRX in the tail lowers it on newer modems";
    PhoneJPerMbWifi = "phone.j_per_mb.wifi", "phone", "active radio, Wi-Fi", "J/MB", [0.02, 0.08, 0.3], Model, Some(false), 2026, "", "",
        "Wi-Fi transfer energy on a phone of the order of 0.1 J/MB at moderate rates";
    PhoneJPerMbCell = "phone.j_per_mb.cellular", "phone", "active radio, cellular", "J/MB", [0.2, 0.8, 3.0], Model, Some(false), 2026, "", "",
        "cellular transfer energy several times Wi-Fi's per byte";
    PhoneJPerSigOp = "phone.j_per_sig_op", "phone", "one hybrid sign or verify on the CPU", "J", [0.0002, 0.0008, 0.003], Frontier, Some(false), 2026,
        "phone on a power monitor: J per hybrid verify and per sign at the edge's batch sizes", "",
        "hybrid verify 276 us MEASURED on an EPYC core (cewp-cli metrics); a phone core several times slower at ~1.5 W";
    PhoneJPerChunkSigOp = "phone.j_per_chunk_sig_op", "phone", "one Ed25519 sign or verify of a media chunk", "J", [0.00002, 0.0001, 0.0005], Frontier, Some(false), 2026,
        "phone on a power monitor during a call, against the media engine's per-chunk sign and verify counts", "",
        "media chunks are Ed25519 (media.rs NodeEnergy): tens of us on a phone core at ~1.5 W";
    // ---- levers: what the edge does today (fixed; the break-even table finds their zero) ----------
    PhoneScheduleShare = "lever.phone_schedule_share", "lever", "share of the edge round and Pull schedule a phone runs itself", "share", [1.0, 1.0, 1.0], Model, None, 2026, "", "",
        "seat_hooks: the edge at 8e46165 has no host-kind branch, so a phone runs the whole schedule; a phone that left its rounds to its home node would run less";
    PhoneCellRoundShare = "lever.cell_round_share", "lever", "share of schedule wake-ups on a carrier network that touch the radio", "share", [1.0, 1.0, 1.0], Model, None, 2026, "", "",
        "the twin assumes per-round: edge's round runs over every live link on any network (micro.rs changes only the budget class and relay offering with the network; capacity.rs has no metered or battery path). 0.1 is a 5-minute batch, 0 push-woken or suspended on cellular";
    RoundCadenceS = "lever.round_cadence_s", "lever", "seconds between Revocation rounds", "s", [30.0, 30.0, 30.0], Model, None, 2026, "", "",
        "capacity.rs CadenceS = 30 s (DERIVED, edge #881); Pull cycles ride on top";
    PhoneRoundsWhileUnsuspended = "phone.rounds_while_unsuspended", "phone", "share of the phone's unsuspended hours in which it runs edge rounds", "share", [0.7, 1.0, 1.0], Model, None, 2026, "",
        "PRESENCE-0 (US 2022-2024, 15+): a person is awake 0.623 of the week, against the twin's 0.85 unsuspended (refused)",
        "1.0 is edge today: a phone node keeps its rounds while the OS lets it run, whether or not its person is asleep (the twin's suspended share, micro mobile_offline_share 0.15, is the only pause); 0.733 = 0.623 / 0.85 is rounds only while the person is awake";
    // ---- home --------------------------------------------------------------------------------
    GatewayW = "home.gateway_w", "home gateway", "ready, Wi-Fi on (already on: not charged)", "W", [3.3, 13.77, 25.0], Model, Some(true), 2025, "",
        "POWER-0 rows 10 (13.77 W, US new integrated gateways 2025, MEASURED sales-weighted) and 0 (9.1 W, Arcep 36 boxes, range 3.3 to 25)",
        "a 9 to 14 W baseline the mesh neither adds nor removes; listed so the scale of a home node's marginal draw is visible";
    RouterNodeW = "home_node.on_router.marginal_w", "home node", "running on the household's existing gateway (marginal)", "W", [0.1, 0.4, 1.5], Frontier, Some(true), 2026,
        "gateway at the wall with and without the edge daemon at the twin's schedule", "",
        "the gateway is on anyway; the daemon adds CPU wake-ups on a SoC idling near 10 W; POWER-0 has no row (not_available)";
    ComputerNodeW = "home_node.on_computer.marginal_w", "home node", "running on a computer already on (marginal)", "W", [0.2, 0.8, 3.0], Frontier, Some(true), 2026,
        "desktop or NAS already on, with and without the edge daemon, at the wall", "",
        "a daemon keeps a desktop out of its deepest idle states";
    HomeNodeJPerMb = "home_node.j_per_mb", "home node", "bytes through the gateway, relayed bytes included", "J/MB", [0.05, 0.16, 0.5], Frontier, Some(true), 2024,
        "home node at the wall forwarding a known byte stream for others (relay), J per MB in plus out",
        "POWER-0 row 0 note (traffic adds 0.1 W at 5 Mbit/s: 0.16 J/MB; 0.3 to 0.8 W at 50 Mbit/s: 0.05 to 0.13 J/MB)",
        "relayed bytes count twice (in and out)";
    HomeNodeJPerSigOp = "home_node.j_per_sig_op", "home node", "one hybrid sign or verify", "J", [0.0001, 0.0004, 0.002], Model, Some(true), 2026, "", "",
        "the EPYC verify (276 us MEASURED) on a slower core at a few W";
    HomeNodeJPerChunkSigOp = "home_node.j_per_chunk_sig_op", "home node", "one Ed25519 sign or verify of a media chunk", "J", [0.00001, 0.00005, 0.0003], Model, Some(true), 2026, "", "",
        "Ed25519 verify of the order of 50 us on a desktop or SBC core at a few W";
    AddedBoxW = "home_node.added_box.w", "home node", "a new always-on box bought or kept on for the mesh (full draw)", "W", [3.3, 6.0, 13.77], Model, Some(false), 2025, "",
        "POWER-0 rows 20 (simple Ethernet router target 3.3 W), 11 (routers and APs 10.15 W), 10 (gateways 13.77 W)",
        "a box of the gateway's own order: more than the fixed network spends on that home";
    CellModemW = "home_node.cell_modem.w", "home node", "a cellular modem in an added box (cellular-only home)", "W", [1.0, 2.0, 4.0], Frontier, Some(false), 2026,
        "the box at the wall with its modem attached and idle", "", "an LTE modem idling attached, mains-powered";
    LaptopW = "laptop.marginal_w", "laptop", "running the edge while awake (marginal)", "W", [0.1, 0.5, 1.5], Frontier, Some(true), 2026,
        "laptop on battery with and without the daemon", "POWER-0 row 29 gives 22 W in use, duty-cycled (not the marginal)",
        "listed for CIRISHolon's table; no scenario puts the edge on a laptop";
    // ---- hosted --------------------------------------------------------------------------------
    HostedSeatW = "hosted.seat.w", "hosted server", "a serve seat, idle included, with PUE", "W", [60.0, 120.0, 300.0], Model, Some(false), 2026, "",
        "POWER-0 rows 41-42 for the overhead (PUE 1.09 hyperscale, 1.54 survey average); the server draw is ours",
        "a one-socket rented server at the wall";
    DevicesPerSeat = "hosted.devices_per_seat", "hosted server", "devices one seat serves", "devices", [1000.0, 2000.0, 10000.0], Model, None, 2026, "", "",
        "the page draws one seat per ~2,000 nodes (docs/globe.js buildWorld)";
    HostedJPerRead = "hosted.j_per_store_read", "hosted server", "one counted store read", "J", [0.001, 0.01, 0.05], Frontier, Some(false), 2026,
        "seat at the wall against its store-read counter over a window", "", "a few ms of one core at ~5 to 10 W per core";
    HostedJPerMb = "hosted.j_per_mb", "hosted server", "relayed bytes on a hosted relay", "J/MB", [0.01, 0.05, 0.3], Frontier, Some(false), 2026,
        "hosted relay at the wall against bytes forwarded", "", "server NIC, CPU and seal work per byte with PUE";
    // ---- network: baseload per line plus a traffic term, never an average per GB ------------
    FixedTrafficJPerMb = "network.fixed.traffic_j_per_mb", "network", "fixed network, the traffic-dependent part", "J/MB", [0.08, 0.24, 0.5], Model, Some(false), 2020, "",
        "POWER-0 row 35 (0.03 W per Mbit/s = 0.24 J/MB; power model from operator data); the short-run saving on a fixed line is this alone",
        "fixed networks are a per-line baseload plus this small term (POWER-0 warning)";
    FixedCoreBaseloadW = "network.fixed.core_baseload_w", "network", "fixed core baseload per line (scales with capacity in the long run)", "W", [0.5, 1.5, 2.3], Model, Some(true), 2024, "",
        "POWER-0 rows 34 (1.5 W core per line, 2020 model, high for fibre) and 4 (Arcep: 2.3 W per line all in, under 10 kWh a year on fibre)",
        "the access line itself stays whether or not the mesh runs; only core capacity is avoidable, and only in the long run";
    LineMbPerHour = "network.fixed.line_mb_per_hour", "network", "a fixed line's own traffic, to share its core capacity", "MB/h", [200.0, 700.0, 2000.0], Frontier, None, 2026, "", "",
        "of the order of 500 GB a month for a household line";
    MobileTrafficJPerMb = "network.mobile.traffic_j_per_mb", "network", "mobile network, the traffic-dependent part", "J/MB", [5.0, 12.24, 25.0], Model, Some(false), 2020, "",
        "POWER-0 row 37 (1.53 W per Mbit/s, 4G, core and access summed = 12.24 J/MB)",
        "cellular is where traffic costs energy";
    MobileBaseloadW = "network.mobile.baseload_w", "network", "mobile baseload per subscriber (scales with capacity in the long run)", "W", [0.6, 1.2, 4.6], Model, Some(true), 2024, "",
        "POWER-0 rows 36 (1.2 W per subscriber, 4G model) and 5 (Arcep 40 kWh per SIM a year = 4.6 W all in)", "";
    SimMbPerHour = "network.mobile.sim_mb_per_hour", "network", "a SIM's own traffic, to share its capacity", "MB/h", [10.0, 25.0, 60.0], Frontier, None, 2026, "", "",
        "of the order of 15 to 20 GB a month";
    VillageUplinkJPerMb = "network.thin_uplink.j_per_mb", "network", "a village's one thin uplink (satellite or long microwave)", "J/MB", [5.0, 50.0, 500.0], Frontier, None, 2026, "", "",
        "a satellite terminal's 50 to 100 W over a few Mbit/s of use; capacity-bound, so the same in both runs";
    // ---- data centre ----------------------------------------------------------------------------
    DcMsgWhPerPersonHour = "datacentre.messaging.wh_per_person_hour", "data centre", "messaging and social workload one person drives, per hour of the day", "Wh", [0.02, 0.1, 0.5], Frontier, None, 2026, "",
        "POWER-0 not_available: no published figure for messaging alone; row 43 (415 TWh, every workload) is not this",
        "a stated bound: a few Wh per user-day for a whole platform, of which messaging is an unknown share";
    DcCallWhPerParticipantHour = "datacentre.call.wh_per_participant_hour", "data centre", "SFU work per participant-hour of a video call", "Wh", [0.1, 1.0, 5.0], Frontier, None, 2026, "", "",
        "an SFU forwards without transcoding; the high end includes recording and transcoding";
    // ---- village ---------------------------------------------------------------------------------
    VillageNodeW = "village.node.w", "village node", "one added node with its radio and solar controller (full draw)", "W", [5.0, 10.0, 20.0], Model, Some(false), 2026, "", "",
        "a small server plus a Wi-Fi or mesh radio";
    VillageHouseholds = "village.households", "workload", "households behind one village node and uplink", "households", [20.0, 50.0, 200.0], Model, None, 2026, "", "",
        "a village of a few hundred people";
    // ---- household -------------------------------------------------------------------------------
    PersonsPerHousehold = "household.persons", "workload", "persons", "persons", [2.0, 2.5, 3.0], Model, None, 2026, "", "",
        "micro.rs household rule round(2.5 + 1.1 N(0,1)) clamped to 1..6";
    PhonesPerPerson = "household.phones_per_person", "workload", "smartphones per person", "phones", [0.8, 0.813, 1.0], Model, None, 2020, "",
        "DEVICES-0 (US 2020, RECS): 0.813 per person (the twin's 1.0 REFUSED); 0.999 per adult",
        "envelope: United States, 2020";
    WifiShare = "phone.wifi_share", "workload", "share of a phone's awake hours on Wi-Fi (home with a fixed line)", "share", [0.4, 0.65, 0.9], Model, None, 2026, "", "",
        "micro.rs phone_wifi_share 0.65 (globe slider default); CIRISHolon attach.phone.network (168 bins) replaces it";
    CellOnlyWifiShare = "phone.wifi_share.cell_only_home", "workload", "share of awake hours on Wi-Fi in a home with no fixed line", "share", [0.0, 0.1, 0.3], Frontier, None, 2026, "", "",
        "no home Wi-Fi: only Wi-Fi met away from home";
    BytesPerAct = "workload.bytes_per_act", "workload", "wire bytes of one act's content copy", "bytes", [1000.0, 5000.0, 50000.0], Model, None, 2026, "", "",
        "a signed text act with a thumbnail; media-heavy acts run higher";
    MeshExtraContentWakes = "phone.mesh_extra_content_wakes", "workload", "extra content wake-ups under the mesh, as a share of the status quo's push wake-ups", "share", [-0.3, 0.0, 0.3], Frontier, None, 2026,
        "phone wake log under the mesh and under a push-notification app for one scripted day", "",
        "mesh content arrives on a live link, push on the OS channel; whether batching differs is unknown";
    KeepaliveBytesPerMappingHour = "phone.keepalive_bytes_per_mapping_hour", "workload", "bytes to hold one NAT mapping for an hour", "bytes", [0.0, 2000.0, 20000.0], Frontier, None, 2026,
        "packet capture of a phone holding punched paths", "",
        "keepalives ride the 30 s round wake, so they add bytes, not wake-ups";
    // ---- the household mix (DEVICES-0, US 2020) ---------------------------------------------------
    ShareFixedBroadband = "mix.fixed_broadband_share", "workload", "households with fixed broadband (a gateway)", "share", [0.728, 0.728, 0.728], Model, None, 2020, "",
        "DEVICES-0 (US 2020, RECS) via the lead: 72.8%", "envelope: United States, 2020, a pandemic year";
    ShareCellOnly = "mix.cellular_only_share", "workload", "households with cellular data only", "share", [0.146, 0.146, 0.146], Model, None, 2020, "",
        "DEVICES-0 (US 2020): 14.6%", "for them an always-on node has no fixed line to sit behind";
    ShareNoInternet = "mix.no_internet_share", "workload", "households with no internet", "share", [0.071, 0.071, 0.071], Model, None, 2020, "",
        "DEVICES-0 (US 2020): 7.1%", "they take no part: delta 0; the undeclared 5.5% is left out and the mix renormalised over 94.5%";
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
    /// The midpoint (used for fixed lever rows only, whose interval is a point).
    fn mid(self) -> f64;
    /// `min(self, c)`.
    fn min_c(self, c: f64) -> Self;
}
impl Q for f64 {
    fn k(x: f64) -> Self {
        x
    }
    fn recip(self) -> Self {
        1.0 / self
    }
    fn mid(self) -> f64 {
        self
    }
    fn min_c(self, c: f64) -> Self {
        self.min(c)
    }
}
impl Q for Iv {
    fn k(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn recip(self) -> Self {
        Self::new(1.0 / self.hi, 1.0 / self.lo)
    }
    fn mid(self) -> f64 {
        0.5 * (self.lo + self.hi)
    }
    fn min_c(self, c: f64) -> Self {
        Self::new(self.lo.min(c), self.hi.min(c))
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
    /// seat_hooks: Revocation rounds per device-hour (one per cadence).
    pub sync_rounds_per_device_hour: f64,
    /// seat_hooks: Pull cycles per device-hour (timer and events).
    pub sync_pulls_per_device_hour: f64,
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
            sync_rounds_per_device_hour: d.rounds_per_hour_milli as f64 / 1000.0,
            sync_pulls_per_device_hour: d.pull_cycles_per_hour_milli as f64 / 1000.0,
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
    /// Home node (on a gateway, a computer or an added box).
    HomeNode = 1,
    /// Hosted seat or relay.
    Hosted = 2,
    /// Access and core networks.
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
    /// The count (in the row's unit's denominator; seconds for W rows).
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
    /// Phones plus a home node on the household's existing gateway (per household-hour).
    RouterNode,
    /// Phones plus a home node on a computer that is on anyway.
    ComputerNode,
    /// Phones plus a home node on a new always-on box.
    AddedBox,
    /// A home with a fixed line, phones only; the hosted seat does mailbox work.
    PhonesOnly,
    /// A home with cellular data only: phones only, mostly on the carrier.
    CellOnlyPhones,
    /// A home with cellular data only that adds an always-on box with a modem; the phones use
    /// its Wi-Fi at home and every byte rides the carrier.
    CellOnlyBox,
    /// A four-person call relayed by a neighbour, against an SFU (per participant-hour).
    CallNeighbourRelay,
    /// The same call relayed by a hosted relay in a data centre, against an SFU.
    CallHostedRelay,
    /// One household of a village behind one thin uplink with one added village node.
    Village,
    /// The US 2020 household mix (DEVICES-0): fixed-line homes with a node on the gateway,
    /// cellular-only homes with phones only, homes with no internet at zero.
    MixGateway,
    /// The same mix with a new box in every home that has a line, and a box with a modem in
    /// every cellular-only home.
    MixBox,
}

impl Scenario {
    /// Every scenario.
    pub const ALL: [Self; 11] = [Self::RouterNode, Self::ComputerNode, Self::AddedBox, Self::PhonesOnly, Self::CellOnlyPhones, Self::CellOnlyBox, Self::CallNeighbourRelay, Self::CallHostedRelay, Self::Village, Self::MixGateway, Self::MixBox];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RouterNode => "home node on existing gateway",
            Self::ComputerNode => "home node on computer already on",
            Self::AddedBox => "home node on a new always-on box",
            Self::PhonesOnly => "phones only (home with a fixed line)",
            Self::CellOnlyPhones => "phones only (cellular-only home)",
            Self::CellOnlyBox => "new box on cellular (cellular-only home)",
            Self::CallNeighbourRelay => "call relayed by a neighbour vs SFU",
            Self::CallHostedRelay => "call relayed in a data centre vs SFU",
            Self::Village => "village, one thin uplink",
            Self::MixGateway => "US 2020 household mix, node on gateway",
            Self::MixBox => "US 2020 household mix, new box",
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
}

/// How the network's avoided energy is counted (POWER-0's warning: network energy is mostly
/// baseload per line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Run {
    /// Capacity fixed: only the traffic-dependent term moves.
    Short = 0,
    /// Capacity not built: the traffic term plus the line's or SIM's share of core or cell
    /// capacity, by its own traffic.
    Long = 1,
}

impl Run {
    /// Both.
    pub const ALL: [Self; 2] = [Self::Short, Self::Long];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Short => "short run",
            Self::Long => "long run",
        }
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
    /// Network lines for `fixed` and `mobile` MB: the traffic term, plus in the long run the
    /// line's or SIM's capacity share (baseload W x 3600 / its own MB an hour).
    #[allow(clippy::too_many_arguments)]
    fn net(&mut self, val: &dyn Fn(E) -> T, run: Run, what: &'static str, fixed: T, mobile: T, sign: f64) {
        self.put(Term::AvoidedUpstream, Dev::Network, E::FixedTrafficJPerMb, what, fixed, val(E::FixedTrafficJPerMb), sign);
        self.put(Term::AvoidedUpstream, Dev::Network, E::MobileTrafficJPerMb, what, mobile, val(E::MobileTrafficJPerMb), sign);
        if run == Run::Long {
            let k = T::k(3600.0);
            self.put(Term::AvoidedUpstream, Dev::Network, E::FixedCoreBaseloadW, "long run: core capacity share (s of a line's baseload per MB moved)", fixed * k * val(E::LineMbPerHour).recip(), val(E::FixedCoreBaseloadW), sign);
            self.put(Term::AvoidedUpstream, Dev::Network, E::MobileBaseloadW, "long run: cell capacity share (s of a SIM's baseload per MB moved)", mobile * k * val(E::SimMbPerHour).recip(), val(E::MobileBaseloadW), sign);
        }
    }
}

/// Wake-ups an hour a device makes at the lever cadence (rounds at the cadence plus Pull
/// cycles), times the phone schedule share.
fn wakes_per_hour<T: Q>(val: &dyn Fn(E) -> T, k: &Counters) -> f64 {
    (3600.0 / val(E::RoundCadenceS).mid() + k.sync_pulls_per_device_hour) * val(E::PhoneScheduleShare).mid()
}

/// One phone's added radio and CPU lines for an hour, `phones` phones online, `w` of the time on
/// Wi-Fi.
fn phone_lines<T: Q>(l: &mut Ledger<T>, val: &dyn Fn(E) -> T, k: &Counters, phones: T, w: T, sends: T, copies: T) {
    use Term::AddedOnExisting as A;
    let one = T::k(1.0);
    let c = one - w;
    let n = wakes_per_hour(val, k);
    let share = val(E::PhoneCellRoundShare).mid();
    let running = phones * val(E::PhoneRoundsWhileUnsuspended);
    // Wi-Fi: every round is a short exchange
    l.put(A, Dev::Phone, E::PhoneWakeJWifi, "edge round and Pull exchanges on Wi-Fi (seat_hooks)", running * w * T::k(n), val(E::PhoneWakeJWifi), 1.0);
    // carrier: the share of rounds that touch the radio; each pays a promotion and a tail cut
    // short by the next round when rounds come faster than the tail
    let cell_wakes = running * c * T::k(n * share);
    let gap = if n * share > 0.0 { 3600.0 / (n * share) } else { f64::INFINITY };
    l.put(A, Dev::Phone, E::PhoneCellPromoJ, "edge round and Pull wake-ups on a carrier (seat_hooks x round share)", cell_wakes, val(E::PhoneCellPromoJ), 1.0);
    l.put(A, Dev::Phone, E::PhoneCellTailW, "radio tail seconds after those wake-ups (tail, cut at the gap to the next)", cell_wakes * val(E::PhoneCellTailS).min_c(gap), val(E::PhoneCellTailW), 1.0);
    let cw = T::k(k.content_wakes_per_phone_hour) * phones * val(E::MeshExtraContentWakes);
    l.put(A, Dev::Phone, E::PhoneWakeJWifi, "extra content wake-ups on Wi-Fi (people_hooks x share)", cw * w, val(E::PhoneWakeJWifi), 1.0);
    l.put(A, Dev::Phone, E::PhoneCellPromoJ, "extra content wake-ups on a carrier", cw * c, val(E::PhoneCellPromoJ), 1.0);
    l.put(A, Dev::Phone, E::PhoneCellTailW, "tail seconds after extra content wake-ups", cw * c * val(E::PhoneCellTailS), val(E::PhoneCellTailW), 1.0);
    let sched = val(E::PhoneScheduleShare);
    let sync_mb = T::k(2.0 * k.sync_bytes_per_device_hour / 1e6) * sched * phones * val(E::PhoneRoundsWhileUnsuspended);
    let ka_mb = T::k(k.held_mappings_per_phone / 1e6) * val(E::KeepaliveBytesPerMappingHour) * phones;
    l.put(A, Dev::Phone, E::PhoneJPerMbWifi, "edge sync MB both ways on Wi-Fi (seat_hooks)", sync_mb * w, val(E::PhoneJPerMbWifi), 1.0);
    l.put(A, Dev::Phone, E::PhoneJPerMbCell, "edge sync MB both ways on a carrier", sync_mb * c, val(E::PhoneJPerMbCell), 1.0);
    l.put(A, Dev::Phone, E::PhoneJPerMbWifi, "NAT keepalive MB on Wi-Fi (reach held mappings)", ka_mb * w, val(E::PhoneJPerMbWifi), 1.0);
    l.put(A, Dev::Phone, E::PhoneJPerMbCell, "NAT keepalive MB on a carrier", ka_mb * c, val(E::PhoneJPerMbCell), 1.0);
    l.put(A, Dev::Phone, E::PhoneJPerSigOp, "signs and verifies, one per act sent and per copy received (people_hooks)", sends + copies, val(E::PhoneJPerSigOp), 1.0);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Home {
    Gateway,
    Computer,
    Box,
    None,
}

/// One household-hour. `cell_only`: the home has no fixed line. `village`: behind one thin
/// uplink.
#[allow(clippy::too_many_lines)]
fn household<T: Q>(l: &mut Ledger<T>, val: &dyn Fn(E) -> T, k: &Counters, r: ParameterRegime, run: Run, home: Home, cell_only: bool, village: bool) {
    use Dev as D;
    use Term::{AddedDevices as B, AddedOnExisting as A, AvoidedUpstream as C};
    let rp = RegimeParameters::for_regime(r);
    let one = T::k(1.0);
    let zero = T::k(0.0);
    let on = T::k(1.0 - rp.mobile_offline_churn);
    let loc = T::k(rp.locality_share);
    let relay_offer = T::k(rp.relay_offering_share);
    let h = val(E::PersonsPerHousehold);
    let phones = h * val(E::PhonesPerPerson);
    let phones_on = phones * on;
    // the phones' network: a cellular-only home with a box gets the box's Wi-Fi at home
    let w = if village { one } else if cell_only && home == Home::None { val(E::CellOnlyWifiShare) } else { val(E::WifiShare) };
    let c = one - w;
    let mb_act = val(E::BytesPerAct) * T::k(1e-6);
    let copies = T::k(k.acts_received_per_person_hour) * h;
    let sends = T::k(k.acts_sent_per_person_hour) * h;
    // ---- A: phones -------------------------------------------------------------------------
    phone_lines(l, val, k, phones_on, w, sends, copies);
    let relayed = T::k(k.relayed_share_wifi) * w + T::k(k.relayed_share_cell) * c;
    // ---- home node -------------------------------------------------------------------------
    let has_node = home != Home::None;
    match home {
        Home::Gateway => l.put(A, D::HomeNode, E::RouterNodeW, "seconds on the existing gateway (marginal W)", T::k(3600.0), val(E::RouterNodeW), 1.0),
        Home::Computer => l.put(A, D::HomeNode, E::ComputerNodeW, "seconds on a computer already on (marginal W)", T::k(3600.0), val(E::ComputerNodeW), 1.0),
        Home::Box => {
            l.put(B, D::HomeNode, E::AddedBoxW, "seconds of a new always-on box (full W)", T::k(3600.0), val(E::AddedBoxW), 1.0);
            if cell_only {
                l.put(B, D::HomeNode, E::CellModemW, "seconds of the box's cellular modem", T::k(3600.0), val(E::CellModemW), 1.0);
            }
        }
        Home::None => {}
    }
    if has_node {
        let node_mb = T::k(2.0 * k.sync_bytes_per_device_hour / 1e6);
        l.put(A, D::HomeNode, E::HomeNodeJPerMb, "home node's own edge sync MB both ways", node_mb, val(E::HomeNodeJPerMb), 1.0);
        // INFERRED: no package counts bytes forwarded for others per device outside a call;
        // a node carries, on average, its household's relayed copies times the relay offering
        // share (in and out)
        let fwd_mb = copies * mb_act * relayed * relay_offer * T::k(2.0);
        l.put(A, D::HomeNode, E::HomeNodeJPerMb, "MB relayed for others, in and out (estimate: relayed share x copies; no counter)", fwd_mb, val(E::HomeNodeJPerMb), 1.0);
    }
    // ---- B: hosted seat share, relays, village node --------------------------------------------
    let devices = phones + if has_node { one } else { zero };
    l.put(B, D::Hosted, E::HostedSeatW, "the household's share of a seat's seconds (devices / devices per seat x 3600)", devices * val(E::DevicesPerSeat).recip() * T::k(3600.0), val(E::HostedSeatW), 1.0);
    l.put(B, D::Hosted, E::HostedJPerRead, "seat store reads the household's devices cause (seat_hooks)", devices * T::k(k.seat_reads_per_device_hour), val(E::HostedJPerRead), 1.0);
    if !has_node {
        let hosted_mb = copies * mb_act * relayed * (one - relay_offer) * T::k(2.0);
        l.put(B, D::Hosted, E::HostedJPerMb, "relayed MB on a hosted relay (no home node; estimate)", hosted_mb, val(E::HostedJPerMb), 1.0);
    }
    if village {
        l.put(B, D::Village, E::VillageNodeW, "the household's share of the village node's seconds", val(E::VillageHouseholds).recip() * T::k(3600.0), val(E::VillageNodeW), 1.0);
    }
    // ---- C: avoided upstream ----------------------------------------------------------------
    l.put(C, D::DataCentre, E::DcMsgWhPerPersonHour, "person-hours of the messaging workload (Wh -> J x3600)", h * T::k(3600.0), val(E::DcMsgWhPerPersonHour), 1.0);
    // bytes by the access they ride: in a cellular-only home everything at home rides the
    // carrier (with a box, the phones' Wi-Fi is the box's)
    let (fx, mo) = if cell_only { (if home == Home::None { w } else { zero }, if home == Home::None { c } else { one }) } else { (w, c) };
    let sq_mb = (sends + copies) * mb_act;
    let far = if village { one - loc } else { one };
    let mesh_mb = copies * mb_act * far * (one + relayed) * T::k(2.0);
    let sync_mb = T::k(2.0 * k.sync_bytes_per_device_hour / 1e6) * (phones_on * val(E::PhoneScheduleShare) + if has_node { one } else { zero });
    if village {
        l.put(C, D::Network, E::VillageUplinkJPerMb, "status-quo content MB over the village uplink (each act up, each copy down)", sq_mb, val(E::VillageUplinkJPerMb), 1.0);
        l.put(C, D::Network, E::VillageUplinkJPerMb, "mesh content MB over the uplink (non-local copies, both ends)", mesh_mb, val(E::VillageUplinkJPerMb), -1.0);
        l.put(C, D::Network, E::VillageUplinkJPerMb, "mesh sync MB to the seat over the uplink", sync_mb, val(E::VillageUplinkJPerMb), -1.0);
    } else {
        l.net(val, run, "status-quo content MB on access (each act up once, each copy down once)", sq_mb * fx, sq_mb * mo, 1.0);
        l.net(val, run, "mesh content MB on access, sender and receiver ends", mesh_mb * fx, mesh_mb * mo, -1.0);
        l.net(val, run, "mesh sync MB to the seat on access", sync_mb * fx, sync_mb * mo, -1.0);
    }
    let _ = loc;
}

fn scaled<T: Q>(v: Vec<Contribution<T>>, f: T) -> Vec<Contribution<T>> {
    v.into_iter().map(|mut c| {
        c.count = c.count * f;
        c.joules = c.count * c.value;
        c
    }).collect()
}

/// Every contribution of `s` under regime `r` and network run `run`, valued by `val` (a point
/// or an interval).
pub fn contributions<T: Q>(val: &dyn Fn(E) -> T, k: &Counters, s: Scenario, r: ParameterRegime, run: Run) -> Vec<Contribution<T>> {
    use Dev as D;
    use Term::{AddedDevices as B, AddedOnExisting as A, AvoidedUpstream as C};
    let mut l = Ledger { v: Vec::new() };
    match s {
        Scenario::RouterNode => household(&mut l, val, k, r, run, Home::Gateway, false, false),
        Scenario::ComputerNode => household(&mut l, val, k, r, run, Home::Computer, false, false),
        Scenario::AddedBox => household(&mut l, val, k, r, run, Home::Box, false, false),
        Scenario::PhonesOnly => household(&mut l, val, k, r, run, Home::None, false, false),
        Scenario::CellOnlyPhones => household(&mut l, val, k, r, run, Home::None, true, false),
        Scenario::CellOnlyBox => household(&mut l, val, k, r, run, Home::Box, true, false),
        Scenario::Village => household(&mut l, val, k, r, run, Home::None, false, true),
        Scenario::MixGateway | Scenario::MixBox => {
            let (fb, co, ni) = (val(E::ShareFixedBroadband), val(E::ShareCellOnly), val(E::ShareNoInternet));
            let norm = (fb + co + ni).recip();
            let (a, b) = if s == Scenario::MixGateway { (Scenario::RouterNode, Scenario::CellOnlyPhones) } else { (Scenario::AddedBox, Scenario::CellOnlyBox) };
            let mut v = scaled(contributions(val, k, a, r, run), fb * norm);
            v.extend(scaled(contributions(val, k, b, r, run), co * norm));
            // homes with no internet take no part: zero lines
            return v;
        }
        Scenario::CallNeighbourRelay | Scenario::CallHostedRelay => {
            let hosted = s == Scenario::CallHostedRelay;
            let n = k.call_participants.max(2.0);
            let total_h: f64 = k.call.iter().map(|x| x.hours).sum::<f64>().max(1e-9);
            let (mut stream, mut own, mut fwd) = (0.0, 0.0, 0.0);
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
                // the mesh's own copies (and, with a neighbour relay, forwarded copies) against
                // the SFU call's one upload and N - 1 downloads; with a hosted relay the device
                // uploads one copy, as to an SFU
                let mesh_mb = if hosted { stream + x.rx_mb } else { x.own_tx_mb + x.fwd_tx_mb + x.rx_mb };
                l.put(A, devs[i], per_mb[i], "call MB on this class's radio or NIC (mesh), per participant-hour", T::k(f * mesh_mb), val(per_mb[i]), 1.0);
                l.put(A, devs[i], per_mb[i], "call MB on this class's radio or NIC (SFU status quo), per participant-hour", T::k(f * stream * n), val(per_mb[i]), -1.0);
                let sig = if i == 0 { E::HomeNodeJPerChunkSigOp } else { E::PhoneJPerChunkSigOp };
                l.put(A, devs[i], sig, "media chunk signs and verifies (the SFU call signs none)", T::k(f * x.sig_ops), val(sig), 1.0);
            }
            let fixed_h = (k.call[0].hours + k.call[1].hours) / total_h;
            let (fx, mo) = (T::k(fixed_h), T::k(1.0 - fixed_h));
            l.put(C, D::DataCentre, E::DcCallWhPerParticipantHour, "SFU participant-hours (Wh -> J x3600)", T::k(3600.0), val(E::DcCallWhPerParticipantHour), 1.0);
            let sq = T::k(stream * n);
            l.net(val, run, "SFU call MB on access (one up, N - 1 down)", sq * fx, sq * mo, 1.0);
            if hosted {
                // one upload to the hosted relay and N - 1 downloads, as the SFU's; the relay
                // is an SFU in a data centre, priced with the SFU's own row
                l.net(val, run, "mesh call MB on access, to and from the hosted relay", sq * fx, sq * mo, -1.0);
                l.put(B, D::Hosted, E::DcCallWhPerParticipantHour, "hosted relay participant-hours, priced as the SFU (Wh -> J x3600)", T::k(3600.0), val(E::DcCallWhPerParticipantHour), 1.0);
            } else {
                let wire = T::k(2.0 * (own + fwd));
                l.net(val, run, "mesh call MB on access, sender and receiver ends", wire * fx, wire * mo, -1.0);
            }
        }
    }
    l.v
}

/// The three terms and their sum, each with its bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct Delta {
    /// Scenario.
    pub scenario: Scenario,
    /// Regime.
    pub regime: ParameterRegime,
    /// Network run.
    pub run: Run,
    /// Point (every row at its default) per term, [`Term`] order.
    pub point: [f64; 3],
    /// Per term, the lowest and highest value found over the rows' ends (attained by a real
    /// combination of input ends: the seat_calibration corners refined by a vertex search).
    pub bounds: [Iv; 3],
    /// `A + B - C`, point.
    pub net: f64,
    /// `A + B - C`, lowest and highest found the same way (searched on the net itself).
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

fn outputs(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime, run: Run) -> [f64; 4] {
    let t = fold(&contributions(&|e| p.get(e), k, s, r, run));
    [t[0], t[1], t[2], t[0] + t[1] - t[2]]
}

/// The point net of `s`.
#[must_use]
pub fn net_at(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime, run: Run) -> f64 {
    outputs(p, k, s, r, run)[3]
}

/// The lowest (`sense = -1`) or highest (`+1`) value of output `o` over the rows' ends: start
/// at seat_calibration's corner (every row at the end that moves `o` that way one at a time),
/// then sweep the rows, keeping any end that improves `o`, until a sweep changes nothing.
fn vertex_search(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime, run: Run, o: usize, sense: f64) -> f64 {
    let free: Vec<E> = E::ALL.iter().copied().filter(|&e| spec(e).lo < spec(e).hi).collect();
    let mut x = p.clone();
    for &e in &free {
        let sp = spec(e);
        let a = outputs(&p.with(e, sp.lo), k, s, r, run)[o];
        let b = outputs(&p.with(e, sp.hi), k, s, r, run)[o];
        x.set(e, if sense * b >= sense * a { sp.hi } else { sp.lo });
    }
    let mut best = outputs(&x, k, s, r, run)[o];
    for _ in 0..4 {
        let mut moved = false;
        for &e in &free {
            let sp = spec(e);
            for end in [sp.lo, sp.hi] {
                if x.get(e) == end {
                    continue;
                }
                let y = outputs(&x.with(e, end), k, s, r, run)[o];
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
    let pt = outputs(p, k, s, r, run)[o];
    if sense > 0.0 { best.max(pt) } else { best.min(pt) }
}

/// The delta of `s` under `r` and `run`: point at the defaults, bounds over every row's interval.
#[must_use]
pub fn delta(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime, run: Run) -> Delta {
    let pts = contributions(&|e| p.get(e), k, s, r, run);
    let ivs = contributions(&|e| { let sp = spec(e); Iv::new(sp.lo.min(p.get(e)), sp.hi.max(p.get(e))) }, k, s, r, run);
    let t = fold(&pts);
    let enc = fold(&ivs);
    let b: [Iv; 4] = core::array::from_fn(|o| Iv::new(vertex_search(p, k, s, r, run, o, -1.0), vertex_search(p, k, s, r, run, o, 1.0)));
    let mut by_dev = [0.0; 6];
    let mut class = Class::Derived;
    for c in &pts {
        let j = c.sign * c.joules * if c.term == Term::AvoidedUpstream { -1.0 } else { 1.0 };
        by_dev[c.dev as usize] += j;
        class = class.weaker(spec(c.row).class);
    }
    Delta { scenario: s, regime: r, run, point: t, bounds: [b[0], b[1], b[2]], net: t[0] + t[1] - t[2], net_bounds: b[3], net_enclosure: enc[0] + enc[1] - enc[2], by_dev, class }
}

/// One break-even row: the value of `row` (others at their defaults) where the net is zero.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakEven {
    /// The input.
    pub row: E,
    /// The net at the row's low end.
    pub net_at_lo: f64,
    /// At the high end.
    pub net_at_hi: f64,
    /// The zero, if one lies in `[min(0, lo), 100 x hi]`.
    pub at: Option<f64>,
    /// Whether the zero lies inside the row's own interval (the sign is open on this input).
    pub inside: bool,
}

/// The phone-radio rows, listed first in the break-even table.
pub const PHONE_RADIO_ROWS: [E; 8] = [E::PhoneCellRoundShare, E::WifiShare, E::CellOnlyWifiShare, E::PhoneCellTailS, E::RoundCadenceS, E::PhoneCellTailW, E::PhoneCellPromoJ, E::PhoneScheduleShare];

/// Every row whose value moves the net, with its zero; phone-radio rows first.
#[must_use]
pub fn break_even(p: &EnergyParams, k: &Counters, s: Scenario, r: ParameterRegime, run: Run) -> Vec<BreakEven> {
    let mut out = Vec::new();
    for &e in E::ALL {
        let sp = spec(e);
        let f = |x: f64| net_at(&p.with(e, x), k, s, r, run);
        let (a, b) = (f(sp.lo), f(sp.hi));
        // a fixed row (a lever) is tested over [0, max(hi, 1)]
        let (ta, tb) = if sp.lo == sp.hi { (f(if e == E::RoundCadenceS { 300.0 } else { 0.0 }), f(sp.hi.max(1.0))) } else { (a, b) };
        if (ta - tb).abs() < 1e-12 * (ta.abs() + tb.abs() + 1.0) {
            continue;
        }
        // a zero at exactly 0 is trivial where the row scales the whole net (persons): start
        // just above it
        let lo0 = if e == E::RoundCadenceS { 1.0 } else if sp.lo.min(0.0) == 0.0 { 1e-9 * sp.hi.max(1.0) } else { sp.lo.min(0.0) };
        let hi0 = if sp.hi > 0.0 { sp.hi * 100.0 } else { 1.0 };
        let steps = 400;
        let mut at = None;
        let mut prev = (lo0, f(lo0));
        for i in 1..=steps {
            let x = lo0 + (hi0 - lo0) * f64::from(i) / f64::from(steps);
            let y = f(x);
            if prev.1.signum() != y.signum() {
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
    let rank = |b: &BreakEven| PHONE_RADIO_ROWS.iter().position(|&x| x == b.row).unwrap_or(PHONE_RADIO_ROWS.len());
    out.sort_by(|x, y| rank(x).cmp(&rank(y)).then((y.net_at_hi - y.net_at_lo).abs().partial_cmp(&(x.net_at_hi - x.net_at_lo).abs()).unwrap_or(core::cmp::Ordering::Equal)));
    out
}

/// One phone's added radio energy a day, Wh: the edge schedule on Wi-Fi and on a carrier, at a
/// round share, cadence, tail and Wi-Fi share; online `online` of the day; other rows at `p`.
#[must_use]
pub fn phone_wh_per_day(p: &EnergyParams, k: &Counters, round_share: f64, cadence_s: f64, tail_s: f64, wifi: f64, online: f64) -> f64 {
    let n = 3600.0 / cadence_s + k.sync_pulls_per_device_hour;
    let cell_n = n * round_share;
    let gap = if cell_n > 0.0 { 3600.0 / cell_n } else { f64::INFINITY };
    let per_hour = wifi * n * p.get(E::PhoneWakeJWifi) + (1.0 - wifi) * cell_n * (p.get(E::PhoneCellPromoJ) + tail_s.min(gap) * p.get(E::PhoneCellTailW));
    24.0 * online * per_hour / 3600.0
}

/// Presence settings the delta is reported under, side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceSetting {
    /// The twin's stand-in: unsuspended 0.85 of the day, Wi-Fi 0.65 flat.
    StandIn,
    /// PRESENCE-0, edge as it is: rounds run whenever unsuspended (asleep or not); the Wi-Fi
    /// share of device-hours from where people are, hour by hour.
    Presence0Edge,
    /// PRESENCE-0, rounds only while the person is awake (0.623 / 0.85); the Wi-Fi share of
    /// awake hours.
    Presence0Awake,
}

impl PresenceSetting {
    /// All three.
    pub const ALL: [Self; 3] = [Self::StandIn, Self::Presence0Edge, Self::Presence0Awake];
    /// Name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StandIn => "stand-in (unsuspended 0.85, Wi-Fi 0.65)",
            Self::Presence0Edge => "PRESENCE-0, edge as is (rounds while unsuspended)",
            Self::Presence0Awake => "PRESENCE-0, rounds only while awake",
        }
    }
}

/// Wi-Fi at a workplace or school, and elsewhere awake or asleep (FRONTIER: PRESENCE-0 says
/// nothing about networks).
pub const WORK_WIFI: f64 = 0.8;
/// See [`WORK_WIFI`].
pub const AWAY_WIFI: f64 = 0.2;

/// The rows a presence setting sets, from PRESENCE-0 hour by hour (DERIVED from the table, with
/// the two Wi-Fi shares above).
#[must_use]
pub fn presence_params(base: &EnergyParams, s: PresenceSetting) -> EnergyParams {
    use crate::presence0 as p0;
    let mut p = base.clone();
    match s {
        PresenceSetting::StandIn => {}
        PresenceSetting::Presence0Edge => {
            p.set(E::WifiShare, p0::week_mean(|h| p0::on_wifi(h, WORK_WIFI, AWAY_WIFI)));
        }
        PresenceSetting::Presence0Awake => {
            let awake = p0::week_mean(p0::awake);
            let wifi_awake = p0::week_mean(|h| {
                let r = p0::row(h);
                r[1] + r[2] * WORK_WIFI + r[3] * AWAY_WIFI
            }) / awake;
            p.set(E::WifiShare, wifi_awake);
            p.set(E::PhoneRoundsWhileUnsuspended, awake / 0.85);
        }
    }
    p
}

/// The grid the report prints: round share, cadence, tail, Wi-Fi share.
pub const PHONE_GRID: ([f64; 3], [f64; 3], [f64; 3], [f64; 3]) = ([1.0, 0.1, 0.0], [30.0, 120.0, 300.0], [2.0, 5.0, 11.6], [0.1, 0.65, 0.9]);

// ---------------------------------------------------------------------------
// Report and CLI
// ---------------------------------------------------------------------------

fn j(x: f64) -> String {
    if x.abs() >= 100.0 { format!("{x:.0}") } else if x.abs() >= 1.0 { format!("{x:.1}") } else { format!("{x:.3}") }
}

fn sign_word(d: &Delta) -> &'static str {
    if d.net_bounds.lo > 0.0 { "cost (interval > 0)" } else if d.net_bounds.hi < 0.0 { "saving (interval < 0)" } else if d.net > 0.0 { "open (point > 0)" } else { "open (point < 0)" }
}

/// The markdown report (`cewp-cli energy [seed]`).
#[must_use]
pub fn report(seed: u64) -> String {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut s = String::new();
    s.push_str(&format!("# Energy delta from the status quo (seed {seed})\n\n"));
    s.push_str("Joules per household-hour (calls: per participant-hour). delta = A + B - C; positive is a cost, negative a saving. Bounds per term and on the net: the lowest and highest values found over every row's interval ends (seat_calibration's corners refined by a vertex search; each is attained by a real combination of input ends); the interval-arithmetic enclosure, guaranteed but wider, has its own column. No row is MEASURED in this repo's sense; rows taken from CIRISHolon POWER-0 carry its basis and its verdict (unresolved). The network is priced as a traffic term (short run) plus a capacity share (long run), never at an average per GB.\n\n");
    s.push_str(&format!("Tables: {POWER0_PIN}; {DEVICES0_PIN}.\n\n"));
    s.push_str("## Counters read (DERIVED from the packages' events)\n\n");
    s.push_str(&format!("- seat_hooks: {:.2} wake-ups / device-hour ({:.1} rounds, {:.2} Pull cycles), {:.0} bytes / device-hour each way, {:.1} seat reads / device-hour\n", k.sync_wakes_per_device_hour, k.sync_rounds_per_device_hour, k.sync_pulls_per_device_hour, k.sync_bytes_per_device_hour, k.seat_reads_per_device_hour));
    s.push_str(&format!("- people_hooks (Poisson, 2,000-person stand-in, 1 day): {:.3} acts sent and {:.2} copies received per person-hour, {:.2} content wake-ups per phone-hour\n", k.acts_sent_per_person_hour, k.acts_received_per_person_hour, k.content_wakes_per_phone_hour));
    s.push_str(&format!("- reach hooks: relayed share of pairs fixed {:.3}, phone Wi-Fi {:.3}, phone cellular {:.3}; held NAT mappings per phone {:.2}\n", k.relayed_share_fixed, k.relayed_share_wifi, k.relayed_share_cell, k.held_mappings_per_phone));
    for (name, m) in [("same-metro peer", k.path_same_metro), ("any peer", k.path_any_peer), ("to data centre", k.path_to_dc)] {
        s.push_str(&format!("- netgraph {name} paths (n={}): {:.2} ASes crossed, {:.2} IXPs, {:.0} long-haul km (read, not priced: network energy is per line and per Mbit/s, not per hop)\n", m.n, m.ases_crossed, m.ixp_crossings, m.long_haul_km));
    }
    for (i, n) in ["fixed", "Wi-Fi", "cellular"].iter().enumerate() {
        let c = k.call[i];
        s.push_str(&format!("- media call4 {n}: {:.4} participant-hours, own tx {:.0} MB/h, forwarded {:.0} MB/h, rx {:.0} MB/h, {:.0} chunk sig ops/h\n", c.hours, c.own_tx_mb, c.fwd_tx_mb, c.rx_mb, c.sig_ops));
    }
    // the phone term first
    s.push_str("\n## The phone radio term, Wh per phone per day\n\nThe edge schedule alone (no content), phone online 85% of the day (expected regime). Rows: share of rounds on a carrier that touch the radio (1 = per round, what the twin assumes the edge does today; 0.1 = a 5-minute batch; 0 = push-woken or suspended on cellular) and the round cadence. Columns: radio tail (s) and share of awake hours on Wi-Fi. Other rows at their defaults (POWER-0 rows 6 and 8: 0.46 J promotion, 1.06 W tail, 0.04 J per Wi-Fi exchange).\n\n| round share | cadence s |");
    let (shares, cads, tails, wifis) = PHONE_GRID;
    for t in tails {
        for w in wifis {
            s.push_str(&format!(" tail {t} s, Wi-Fi {w} |"));
        }
    }
    s.push_str("\n|---|---|");
    for _ in 0..9 {
        s.push_str("---|");
    }
    s.push('\n');
    for sh in shares {
        for c in cads {
            s.push_str(&format!("| {sh} | {c} |"));
            for t in tails {
                for w in wifis {
                    s.push_str(&format!(" {:.2} |", phone_wh_per_day(&p, &k, sh, c, t, w, 0.85)));
                }
            }
            s.push('\n');
        }
    }
    s.push_str(&format!("\n## Presence side by side (expected regime, short run)\n\n{}. Wi-Fi at a workplace or school {WORK_WIFI}, elsewhere {AWAY_WIFI} (FRONTIER). Bounds as below.\n\n| scenario |", crate::presence0::PRESENCE0_PIN));
    for ps in PresenceSetting::ALL {
        s.push_str(&format!(" {} |", ps.as_str()));
    }
    s.push_str("\n|---|---|---|---|\n");
    let pp: Vec<EnergyParams> = PresenceSetting::ALL.iter().map(|&ps| presence_params(&p, ps)).collect();
    s.push_str("| (inputs) |");
    for q in &pp {
        s.push_str(&format!(" Wi-Fi share {:.3}, rounds while unsuspended {:.3}, phone Wh/day {:.2} |", q.get(E::WifiShare), q.get(E::PhoneRoundsWhileUnsuspended), phone_wh_per_day(q, &k, 1.0, 30.0, q.get(E::PhoneCellTailS), q.get(E::WifiShare), 0.85 * q.get(E::PhoneRoundsWhileUnsuspended))));
    }
    s.push('\n');
    for sc in Scenario::ALL {
        if sc.per() != "household-hour" {
            continue;
        }
        s.push_str(&format!("| {} |", sc.as_str()));
        for q in &pp {
            let d = delta(q, &k, sc, ParameterRegime::Expected, Run::Short);
            s.push_str(&format!(" {} [{}, {}] {} |", j(d.net), j(d.net_bounds.lo), j(d.net_bounds.hi), sign_word(&d)));
        }
        s.push('\n');
    }
    s.push_str("\n## The delta by scenario, regime and network run\n\n| scenario | per | regime | run | A added on existing | B added devices | C avoided upstream (net) | delta = A + B - C | enclosure | sign | class |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for sc in Scenario::ALL {
        for r in REGIMES {
            for run in Run::ALL {
                let d = delta(&p, &k, sc, r, run);
                let iv = |i: usize| format!("{} [{}, {}]", j(d.point[i]), j(d.bounds[i].lo), j(d.bounds[i].hi));
                s.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} | {} [{}, {}] | [{}, {}] | {} | {} |\n", sc.as_str(), sc.per(), regime_name(r), run.as_str(), iv(0), iv(1), iv(2), j(d.net), j(d.net_bounds.lo), j(d.net_bounds.hi), j(d.net_enclosure.lo), j(d.net_enclosure.hi), sign_word(&d), d.class.as_str()));
            }
        }
    }
    s.push_str("\n## By device class (expected regime, short run, point, J, signed as it enters delta)\n\n| scenario |");
    for dv in Dev::ALL {
        s.push_str(&format!(" {} |", dv.as_str()));
    }
    s.push_str("\n|---|---|---|---|---|---|---|\n");
    for sc in Scenario::ALL {
        let d = delta(&p, &k, sc, ParameterRegime::Expected, Run::Short);
        s.push_str(&format!("| {} |", sc.as_str()));
        for x in d.by_dev {
            s.push_str(&format!(" {} |", j(x)));
        }
        s.push('\n');
    }
    s.push_str("\n## Break-even (expected regime): the value of one input, others at default, where delta = 0\n\nPhone-radio inputs first (levers are fixed at what the edge does today; their zero is where a design change would flip the sign), then every input whose zero lies inside its own interval, then the largest movers.\n\n| scenario | run | input | class | interval | delta at lo / hi | zero at | inside interval |\n|---|---|---|---|---|---|---|---|\n");
    for sc in Scenario::ALL {
        for run in Run::ALL {
            let be = break_even(&p, &k, sc, ParameterRegime::Expected, run);
            let mut others = 0;
            for b in &be {
                let radio = PHONE_RADIO_ROWS.contains(&b.row);
                if !radio && !b.inside {
                    others += 1;
                    if others > 2 {
                        continue;
                    }
                }
                let sp = spec(b.row);
                s.push_str(&format!("| {} | {} | `{}` | {} | [{}, {}] {} | {} / {} | {} | {} |\n", sc.as_str(), run.as_str(), sp.name, sp.class.as_str(), sp.lo, sp.hi, sp.unit, j(b.net_at_lo), j(b.net_at_hi), b.at.map_or("none in range".to_string(), |z| format!("{z:.4}")), if b.inside { "yes" } else { "no" }));
            }
        }
    }
    s
}

/// The input table as markdown (FSD/twin/energy.md section 2).
#[must_use]
pub fn inputs_table() -> String {
    let mut s = String::from("| row | device | state | unit | lo / default / hi | class | on anyway | year | needs measurement on device | taken from (basis) | reasoning |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for r in ENERGY_SPECS {
        let on = match r.would_be_on_anyway { Some(true) => "yes", Some(false) => "no", None => "n/a" };
        s.push_str(&format!("| `{}` | {} | {} | {} | {} / {} / {} | {} | {} | {} | {} | {} | {} |\n", r.name, r.device, r.state, r.unit, r.lo, r.mid, r.hi, r.class.as_str(), on, r.year, if r.needs_measurement.is_empty() { "-" } else { r.needs_measurement }, if r.source.is_empty() { "ours" } else { r.source }, r.why));
    }
    s
}

// ---------------------------------------------------------------------------
// Globe views (flat buffers)
// ---------------------------------------------------------------------------

/// Floats per row of [`view_delta`].
pub const DELTA_STRIDE: usize = 16;

/// The delta per scenario, regime and run, `DELTA_STRIDE` f32 per row; scenarios in
/// [`Scenario::ALL`] order, then regimes in [`REGIMES`] order, then runs in [`Run::ALL`] order:
/// `[scenario, regime, run (0 short, 1 long), per (0 household-hour, 1 participant-hour),
///   A, A_lo, A_hi, B, B_lo, B_hi, C, C_lo, C_hi, net, net_lo, net_hi]` in joules. Sign: cost if
/// `net_lo > 0`, saving if `net_hi < 0`, else open. Class per row: [`view_delta_class`].
#[must_use]
pub fn view_delta(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut v = Vec::with_capacity(Scenario::ALL.len() * REGIMES.len() * 2 * DELTA_STRIDE);
    for (si, sc) in Scenario::ALL.iter().enumerate() {
        for (ri, r) in REGIMES.iter().enumerate() {
            for run in Run::ALL {
                let d = delta(&p, &k, *sc, *r, run);
                v.extend_from_slice(&[si as f32, ri as f32, run as usize as f32, if sc.per() == "participant-hour" { 1.0 } else { 0.0 }]);
                for t in 0..3 {
                    v.extend_from_slice(&[d.point[t] as f32, d.bounds[t].lo as f32, d.bounds[t].hi as f32]);
                }
                v.extend_from_slice(&[d.net as f32, d.net_bounds.lo as f32, d.net_bounds.hi as f32]);
            }
        }
    }
    v
}

/// The class code of every [`view_delta`] row, one f32 each.
#[must_use]
pub fn view_delta_class(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut v = Vec::new();
    for sc in Scenario::ALL {
        for r in REGIMES {
            for run in Run::ALL {
                v.push(delta(&p, &k, sc, r, run).class.code());
            }
        }
    }
    v
}

/// Floats per row of [`view_break_even`].
pub const BREAK_EVEN_STRIDE: usize = 9;

/// The break-even table at the expected regime, `BREAK_EVEN_STRIDE` f32 per row, phone-radio
/// rows first within each (scenario, run): `[scenario, run, row index into ENERGY_SPECS, class,
/// lo, hi, net_at_lo, net_at_hi, zero (NaN when none in range)]`. A row decides the sign when
/// `lo <= zero <= hi`; a lever row has `lo == hi` and its zero is a design threshold.
#[must_use]
pub fn view_break_even(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let mut v = Vec::new();
    for (si, sc) in Scenario::ALL.iter().enumerate() {
        for run in Run::ALL {
            for b in break_even(&p, &k, *sc, ParameterRegime::Expected, run) {
                let sp = spec(b.row);
                v.extend_from_slice(&[si as f32, run as usize as f32, b.row as usize as f32, sp.class.code(), sp.lo as f32, sp.hi as f32, b.net_at_lo as f32, b.net_at_hi as f32, b.at.map_or(f32::NAN, |z| z as f32)]);
            }
        }
    }
    v
}

/// Floats per row of [`view_phone_wh`].
pub const PHONE_STRIDE: usize = 5;

/// The phone radio term grid, `PHONE_STRIDE` f32 per row: `[round_share, cadence_s, tail_s,
/// wifi_share, wh_per_day]`, over [`PHONE_GRID`] (online 0.85). Class MODEL (POWER-0 rows 6, 8:
/// one 2012 handset).
#[must_use]
pub fn view_phone_wh(seed: u64) -> Vec<f32> {
    let k = Counters::from_hooks(seed);
    let p = EnergyParams::default();
    let (shares, cads, tails, wifis) = PHONE_GRID;
    let mut v = Vec::new();
    for sh in shares {
        for c in cads {
            for t in tails {
                for w in wifis {
                    v.extend_from_slice(&[sh as f32, c as f32, t as f32, w as f32, phone_wh_per_day(&p, &k, sh, c, t, w, 0.85) as f32]);
                }
            }
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
            assert!(matches!(s.class, Class::Model | Class::Frontier), "{}: no row is MEASURED in this repo's sense", s.name);
            assert!(!s.why.is_empty() || !s.source.is_empty());
        }
        for e in [E::PhoneWakeJWifi, E::PhoneCellPromoJ, E::PhoneCellTailS, E::HomeNodeJPerMb, E::PhoneJPerSigOp, E::RouterNodeW] {
            assert!(!spec(e).needs_measurement.is_empty(), "{}", spec(e).name);
        }
        // POWER-0 row 6: promotion + tail = the whole small-packet exchange
        let whole = spec(E::PhoneCellPromoJ).mid + spec(E::PhoneCellTailS).mid * spec(E::PhoneCellTailW).mid;
        assert!((whole - 12.76).abs() < 0.01, "{whole}");
    }

    #[test]
    fn conservation_every_joule_is_a_count_times_a_row() {
        let k = counters();
        let p = EnergyParams::default();
        for sc in Scenario::ALL {
            for r in REGIMES {
                for run in Run::ALL {
                    let v = contributions(&|e| p.get(e), &k, sc, r, run);
                    let mut t = [0.0; 3];
                    for c in &v {
                        assert!((c.joules - c.count * c.value).abs() <= 1e-9 * c.joules.abs().max(1.0));
                        assert_eq!(c.value, p.get(c.row), "{:?} {}", c.row, c.what);
                        t[c.term as usize] += c.sign * c.joules;
                    }
                    let d = delta(&p, &k, sc, r, run);
                    for i in 0..3 {
                        assert!((t[i] - d.point[i]).abs() <= 1e-9 * t[i].abs().max(1.0));
                    }
                    let by: f64 = d.by_dev.iter().sum();
                    assert!((by - d.net).abs() <= 1e-6 * d.net.abs().max(1.0), "device split sums to the net");
                }
            }
        }
    }

    #[test]
    fn bounds_enclose_the_point_and_every_corner_tried() {
        let k = counters();
        let p = EnergyParams::default();
        for sc in Scenario::ALL {
            for run in Run::ALL {
                let d = delta(&p, &k, sc, ParameterRegime::Expected, run);
                assert!(d.net_bounds.lo <= d.net && d.net <= d.net_bounds.hi);
                assert!(d.net_enclosure.lo <= d.net_bounds.lo + 1e-6 && d.net_bounds.hi <= d.net_enclosure.hi + 1e-6, "{sc:?}: the vertex range sits inside the enclosure");
                for &e in E::ALL {
                    for x in [spec(e).lo, spec(e).hi] {
                        let n = net_at(&p.with(e, x), &k, sc, ParameterRegime::Expected, run);
                        assert!(n >= d.net_bounds.lo - 1e-6 && n <= d.net_bounds.hi + 1e-6, "{sc:?} {e:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn monotone_where_the_formula_says() {
        let k = counters();
        let p = EnergyParams::default();
        let r = ParameterRegime::Expected;
        for run in Run::ALL {
            for sc in [Scenario::RouterNode, Scenario::AddedBox, Scenario::PhonesOnly, Scenario::CellOnlyPhones] {
                for e in [E::PhoneCellTailS, E::PhoneCellTailW, E::PhoneWakeJWifi, E::HostedSeatW, E::PhoneJPerSigOp] {
                    assert!(net_at(&p.with(e, spec(e).hi), &k, sc, r, run) >= net_at(&p.with(e, spec(e).lo), &k, sc, r, run), "{sc:?} {e:?}");
                }
                assert!(net_at(&p.with(E::DcMsgWhPerPersonHour, 0.5), &k, sc, r, run) <= net_at(&p.with(E::DcMsgWhPerPersonHour, 0.02), &k, sc, r, run));
                // fewer carrier rounds never cost more
                assert!(net_at(&p.with(E::PhoneCellRoundShare, 0.1), &k, sc, r, run) <= net_at(&p, &k, sc, r, run));
            }
            assert!(net_at(&p, &k, Scenario::AddedBox, r, run) > net_at(&p, &k, Scenario::RouterNode, r, run));
        }
    }

    #[test]
    fn the_phone_term_matches_powers_own_arithmetic() {
        let k = counters();
        let p = EnergyParams::default();
        // POWER-0 arithmetic: an exchange every 30 s with an 11.6 s tail is about 0.4 W, about
        // 10 Wh a day on LTE; about 0.03 Wh a day on Wi-Fi; every 5 minutes about 1 Wh a day
        let lte = phone_wh_per_day(&p, &k, 1.0, 30.0, 11.6, 0.0, 1.0);
        assert!((9.0..12.0).contains(&lte), "{lte}");
        let wifi = phone_wh_per_day(&p, &k, 1.0, 30.0, 11.6, 1.0, 1.0);
        assert!((0.02..0.05).contains(&wifi), "{wifi}");
        let push = phone_wh_per_day(&p, &k, 0.0, 30.0, 11.6, 0.0, 1.0);
        assert!(push.abs() < 1e-12);
        // a tail longer than the gap is cut at the gap
        let t30 = phone_wh_per_day(&p, &k, 1.0, 30.0, 40.0, 0.0, 1.0);
        assert!(t30 < phone_wh_per_day(&p, &k, 1.0, 30.0, 11.6, 0.0, 1.0) * 3.0);
    }

    #[test]
    fn the_sign_flips_across_its_break_even() {
        let k = counters();
        let p = EnergyParams::default();
        let r = ParameterRegime::Expected;
        let mut flips = 0;
        for sc in Scenario::ALL {
            for run in Run::ALL {
                for b in break_even(&p, &k, sc, r, run) {
                    if let Some(z) = b.at {
                        let sp = spec(b.row);
                        let eps = (sp.hi - sp.lo).abs().max(sp.hi.abs()).max(1e-6) * 1e-4;
                        let below = net_at(&p.with(b.row, z - eps), &k, sc, r, run);
                        let above = net_at(&p.with(b.row, z + eps), &k, sc, r, run);
                        assert!(below.signum() != above.signum() || below.abs() < 1e-3 || above.abs() < 1e-3, "{sc:?} {:?} at {z}", b.row);
                        flips += 1;
                    }
                }
            }
        }
        assert!(flips > 0);
    }

    #[test]
    fn the_headline_can_come_out_either_way() {
        let k = counters();
        let p = EnergyParams::default();
        let (mut pos, mut neg) = (false, false);
        for sc in Scenario::ALL {
            for run in Run::ALL {
                let d = delta(&p, &k, sc, ParameterRegime::Expected, run);
                pos |= d.net_bounds.hi > 0.0;
                neg |= d.net_bounds.lo < 0.0;
            }
        }
        assert!(pos && neg);
    }

    #[test]
    fn the_mix_is_the_weighted_sum_of_its_types() {
        let k = counters();
        let p = EnergyParams::default();
        let r = ParameterRegime::Expected;
        for run in Run::ALL {
            let wsum = 0.728 + 0.146 + 0.071;
            let want = (0.728 * net_at(&p, &k, Scenario::RouterNode, r, run) + 0.146 * net_at(&p, &k, Scenario::CellOnlyPhones, r, run)) / wsum;
            assert!((net_at(&p, &k, Scenario::MixGateway, r, run) - want).abs() < 1e-6 * want.abs().max(1.0));
        }
    }

    #[test]
    fn presence0_lowers_the_phone_term() {
        let k = counters();
        let p = EnergyParams::default();
        let a = presence_params(&p, PresenceSetting::Presence0Awake);
        let e = presence_params(&p, PresenceSetting::Presence0Edge);
        assert!((a.get(E::PhoneRoundsWhileUnsuspended) - 0.623 / 0.85).abs() < 0.01);
        // the Wi-Fi share of awake hours sits in PRESENCE-0's bracket [0.61, 0.79]
        assert!((0.6..0.8).contains(&a.get(E::WifiShare)), "{}", a.get(E::WifiShare));
        for sc in [Scenario::PhonesOnly, Scenario::RouterNode] {
            let r = ParameterRegime::Expected;
            assert!(net_at(&a, &k, sc, r, Run::Short) < net_at(&p, &k, sc, r, Run::Short));
            assert!(net_at(&e, &k, sc, r, Run::Short) < net_at(&p, &k, sc, r, Run::Short), "nights at home put rounds on Wi-Fi");
        }
    }

    #[test]
    fn deterministic_by_seed() {
        assert_eq!(Counters::from_hooks(5), Counters::from_hooks(5));
        assert_eq!(view_delta(5), view_delta(5));
        assert_eq!(view_delta(5).len(), Scenario::ALL.len() * REGIMES.len() * 2 * DELTA_STRIDE);
        assert_eq!(view_break_even(5).len() % BREAK_EVEN_STRIDE, 0);
        assert_eq!(view_phone_wh(5).len(), 81 * PHONE_STRIDE);
    }

    #[test]
    fn counters_are_read_from_the_hooks() {
        let k = counters();
        assert!((k.sync_wakes_per_device_hour - 126.75).abs() < 0.01, "seat_hooks schedule: {}", k.sync_wakes_per_device_hour);
        assert!((k.sync_rounds_per_device_hour - 120.0).abs() < 1e-9);
        assert!(k.sync_bytes_per_device_hour > 0.0);
        assert!(k.acts_received_per_person_hour > 0.0);
        assert!(k.path_to_dc.n > 0 && k.path_any_peer.n > 0);
        assert!(k.call_participants >= 2.0);
    }

    #[test]
    #[ignore = "prints the report; run with --ignored --nocapture"]
    fn print_report() {
        println!("{}", report(1));
        println!("{}", inputs_table());
    }
}
