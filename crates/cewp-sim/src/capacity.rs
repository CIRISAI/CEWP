//! Seat capacity: a per-node cost ledger, the bootstrap round schedule, a
//! one-queue saturation model and a capacity sweep.
//!
//! The question this module answers is what one serve seat ("seat-1") can
//! carry once devices round with it by default (edge v45, #881), and which
//! term binds first. Every number it uses is a named parameter in
//! [`COST_SPECS`] with a tag saying where it comes from:
//!
//! * `MEASURED` — read off the production seat (counters or a profile window);
//!   the source names the window.
//! * `DERIVED` — read from edge code (the source names the trace).
//! * `DERIVED-FITS-MEASURED` — derived from code and checked against one
//!   measured rate.
//! * `FITTED` — the one free parameter, fitted to one measured window.
//! * `PROVISIONAL` — a placeholder an edge or server trace is replacing.
//! * `MODEL` — a stated modelling choice with no measurement behind it.
//! * `UNEXPLAINED` — a measured excess the model does not account for.
//! * `MEASURED-CONTEXT` — measured on the seat but not an edge cost (carried so
//!   the CPU total closes, never scaled with devices).
//!
//! Cost model. Two read classes share one CPU parameter, CPU per decoded byte:
//! index pages (a holdings drain decodes up to 1,024 hashes per page and takes
//! a sweep permit while it does) and counted queries (full rows with JSON
//! decode, what the store's own counter reports). Only one measured window
//! carries a CPU figure, so only that one parameter is fitted; the byte sizes
//! that turn reads into decoded bytes are `MODEL`/`DERIVED`.
//!
//! What the saturation model leaves out: it is one queue with `seat_cores`
//! servers treated as a single fast server (waiting factor `1/(1-rho)`), plus
//! a separate check that the sweep permits are not over-subscribed. It does
//! not model tokio scheduling, SQLite lock contention, page cache misses, the
//! allocator, burstiness of round arrivals (all arrivals are Poisson-smooth),
//! or memory. Round latency is the time a round waits for CPU, not network.

use crate::transport::BudgetClass;

/// Where a cost parameter's value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CTag {
    /// Read off the production seat.
    Measured,
    /// Read from edge code.
    Derived,
    /// Derived from code and checked against one measured rate.
    DerivedFitsMeasured,
    /// The single free parameter, fitted to one measured window.
    Fitted,
    /// A placeholder being replaced by an edge or server trace.
    Provisional,
    /// A stated modelling choice.
    Model,
    /// A measured excess the model does not account for.
    Unexplained,
    /// Measured on the seat but not an edge cost.
    Context,
}

impl CTag {
    /// The tag as printed in tables.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "MEASURED",
            Self::Derived => "DERIVED",
            Self::DerivedFitsMeasured => "DERIVED-FITS-MEASURED",
            Self::Fitted => "FITTED",
            Self::Provisional => "PROVISIONAL",
            Self::Model => "MODEL",
            Self::Unexplained => "UNEXPLAINED",
            Self::Context => "MEASURED-CONTEXT",
        }
    }
}

/// One cost parameter: its name, default, tag and source.
#[derive(Debug, Clone, Copy)]
pub struct CostSpec {
    /// Parameter name.
    pub name: &'static str,
    /// Default value.
    pub default: f64,
    /// Where the value comes from.
    pub tag: CTag,
    /// The citation or the reason.
    pub source: &'static str,
}

macro_rules! cost_params {
    ($( $idx:ident = $name:literal, $def:expr, $tag:ident, $src:literal; )*) => {
        /// Index of each cost parameter in [`CostParams::v`].
        #[repr(usize)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum C { $( $idx, )* }
        /// Every cost parameter, in index order.
        pub const COST_SPECS: &[CostSpec] = &[
            $( CostSpec { name: $name, default: $def, tag: CTag::$tag, source: $src }, )*
        ];
    };
}


/// The v45 bundle path as CIRISEdge#971 has it at c580298 (`src/replication/runtime.rs`;
/// the series is frozen there, 0dc3274 on top changes tests only and leaves runtime.rs as is).
/// Each parameter that rests on one of these cites it; the test
/// `edge_971_constants_match_the_source` compares them with the branch when
/// `CEWP_EDGE_RUNTIME_RS` names the file, so a change before the tag fails loudly.
pub mod edge_971 {
    /// The commit read.
    pub const COMMIT: &str = "c580298";
    /// runtime.rs:1016, subject-Pulled toward a no-consent seat (self and seat legs).
    pub const BUNDLE_PULL_PLANES: [&str; 4] =
        ["Key", "IdentityOccurrence", "TransportDestination", "IdentityOccurrenceRevocation"];
    /// runtime.rs:1027, the owner leg's planes (timer cycles only).
    pub const BUNDLE_OWNER_PLANES: [&str; 3] = ["Key", "IdentityOccurrence", "IdentityOccurrenceRevocation"];
    /// runtime.rs:1037, the one ordinary round a device keeps with a seat (after attribution).
    pub const BUNDLE_ROUND_PLANES: [&str; 1] = ["Revocation"];
    /// runtime.rs:1051.
    pub const BUNDLE_PULL_INTERVAL_S: f64 = 600.0;
    /// runtime.rs:1056; first-contact retries draw U[0.5, 1.5] of it (`first_contact_delay` :1545).
    pub const BUNDLE_PULL_MIN_SPACING_S: f64 = 30.0;
    /// runtime.rs:1262.
    pub const BUNDLE_FIRST_CONTACT_TRIES: u32 = 6;
}

cost_params! {
    // ---- store state at the calibration point --------------------------
    SeatAttRows = "seat_att_rows", 42_520.0, Measured, "seat-1 store count, window A (scores 32,293, delegates_to 6,338, supersedes 3,057, consent 732)";
    SeatTdRows = "seat_td_rows", 18_313.0, Measured, "seat-1 transport destinations, window A";
    SeatKeyRows = "seat_key_rows", 8_698.0, Measured, "seat-1 keys, window A; also the code bound on the Key-plane offer";
    AnnouncedNodes = "announced_nodes", 1_349.0, Measured, "seat-1 relay report announced_nodes";
    AnnouncedOwners = "announced_owners", 1_338.0, Measured, "seat-1 relay report owners";
    StoreFloorNodes = "store_floor_nodes", 1_349.0, Model, "the store never holds fewer nodes than today's announced set: N_nodes = max(devices, floor)";
    PageRows = "page_rows", 1_024.0, Derived, "holdings drain page size (edge code trace, team-lead 2026-11-10)";
    // ---- round schedule --------------------------------------------------
    CadenceS = "cadence_s", 30.0, Derived, "bootstrap round cadence per plane per peer (edge #881)";
    SmallReadsPerCadence = "small_reads_per_cadence", 6.0, Derived, "about 6 small reads per device per cadence beside the drains (edge code trace)";
    SummariesPerRound = "summaries_per_round", 1.0, DerivedFitsMeasured, "window B closes on one drain per completed responder round: 2,340 pages predicted vs 2,262 measured (4%)";
    AttRequiredQueries = "att_required_queries", 15.0, Derived, "required read of 10-20 counted queries per attestation round (midpoint)";
    RefusedRowsPerAttRound = "refused_rows_per_att_round", 208.0, Measured, "2,078 rows withheld / 10 attestation rounds, window A";
    QueriesPerRefusedRow = "queries_per_refused_row", 9.4, Measured, "19,550 attestation queries / 2,078 withheld rows, window A";
    ComposerPerRefusedRow = "composer_per_refused_row", 1.86, Measured, "3,867 composer lookups / 2,078 withheld rows, window A (all returned 0 rows)";
    ConsentPerRefusedRow = "consent_per_refused_row", 1.0, Measured, "2,104 live-consent-grant reads / 2,078 withheld rows, window A";
    RowsPerAttQuery = "rows_per_att_query", 7.5, Measured, "attestation queries return ~7.5 rows each, window A";
    StaleRoutesPerS = "stale_routes_per_s", 2.8, Measured, "seat-1 stale routes re-delivered per second";
    StaleRouteReads = "stale_route_reads", 2.0, Derived, "two store reads per stale route delivery (edge code trace)";
    HybridVerifyCpuS = "hybrid_verify_cpu_s", 0.0015, Model, "CPU-seconds per hybrid (Ed25519 + ML-DSA-65) signature verify; not profiled";
    SweepPermits = "sweep_permits", 2.0, Derived, "two sweep permits per node, one held per index page (edge code trace)";
    KeyMemoExpiryS = "key_memo_expiry_s", 600.0, Derived, "key-plane memo expires after 10 min, rebuild takes one permit per subject (window A candidate)";
    ReplyDrain = "reply_drain", 1.0, Model, "an initiator drains its index on each reply Summary (window A candidate; UNEXPLAINED until the counters split)";
    WindowAExcessPerMin = "window_a_excess_per_min", 3_074.0, Unexplained, "window A: 6,386 permits less 2,442 predicted from its responder rounds and 870 key-memo candidate; source not named";
    // ---- after read minimisation (v45 plan) -------------------------------
    ProbePerS = "probe_per_s", 1.0, Derived, "v45 plan: one shared holdings probe at most once a second per index kind";
    AuditDrainS = "audit_drain_s", 60.0, Derived, "v45 plan: one full audit drain per 60 s per index kind";
    HoldingsBytesPerHash = "holdings_bytes_per_hash", 60.0, Derived, "v45 plan: 40-80 bytes per hash held in the shared holdings set";
    // ---- CPU --------------------------------------------------------------
    HashRefBytes = "hash_ref_bytes", 72.0, Model, "decoded bytes per index entry (64 hex chars + framing)";
    RowBytes = "row_bytes", 800.0, Derived, "43-103 MB decoded over 8.5k-14.4k calls per 30 s at 7.5 rows a call: 675-954 B a row (seat-1 profile)";
    CallOverheadBytes = "call_overhead_bytes", 400.0, Model, "fixed cost of one counted call in decoded-byte equivalents (a 0-row call still costs)";
    CpuPerByte = "cpu_per_byte_s", 1.05e-7, Fitted, "fitted so window B's predicted CPU equals 65.2% of a core; see calibrate()";
    RoundOverheadCpuS = "round_overhead_cpu_s", 0.000_2, Model, "frame handling per received Summary beside its reads";
    ServerRefreshCores = "server_refresh_cores", 0.10, Context, "server announced-relay refresh, ~1/min at p50 6.7 s, ~10% of a core, being fixed server-side; not an edge cost";
    SeatCores = "seat_cores", 4.0, Model, "cores the seat can use (seat-1 reached 171% in the roll hour, so at least 2)";
    ProxyCores = "proxy_cores", 2.0, Model, "cores a server-class proxy can use";
    CpuCeiling = "cpu_ceiling", 0.70, Model, "utilisation the sweep treats as the ceiling (cores and permits)";
    // ---- required-set Pull (v45 baseline shape) ---------------------------
    PullPlanes = "pull_planes", edge_971::BUNDLE_PULL_PLANES.len() as f64, Derived, "BUNDLE_PULL_PLANES: Key, IdentityOccurrence, TransportDestination, IdentityOccurrenceRevocation; the self and seat legs (CIRISEdge#971 c580298 runtime.rs:1016)";
    OwnerPlanes = "owner_planes", edge_971::BUNDLE_OWNER_PLANES.len() as f64, Derived, "BUNDLE_OWNER_PLANES: Key, IdentityOccurrence, IdentityOccurrenceRevocation, never the route plane; the owner leg, timer cycles only (c580298 runtime.rs:1027, :1461, :1503)";
    PullCycleS = "pull_cycle_s", edge_971::BUNDLE_PULL_INTERVAL_S, Derived, "BUNDLE_PULL_INTERVAL, each device draws 300-900 s, mean 600 s (c580298 runtime.rs:1051, bundle_pull_delay :1538, jittered :1550)";
    FirstContactSpacingS = "first_contact_spacing_s", edge_971::BUNDLE_PULL_MIN_SPACING_S, Derived, "BUNDLE_PULL_MIN_SPACING: kicked cycles at least this far apart; first-contact retries drawn U[0.5, 1.5] of it, mean 30 s (c580298 runtime.rs:1056, first_contact_delay :1545)";
    FirstContactTries = "first_contact_tries", edge_971::BUNDLE_FIRST_CONTACT_TRIES as f64, Derived, "BUNDLE_FIRST_CONTACT_TRIES: quick cycles before the jittered 600 s timer when a seat never attributes; the give-up is counted and warned once (c580298 runtime.rs:1262, :1628)";
    FirstContactCycles = "first_contact_cycles", 2.0, Model, "first-contact cycles a start costs when the seat already holds the key and route: the offering Key and TD answers of the first cycle attribute at the second (c580298 runtime.rs:1522); at most first_contact_tries";
    RuntimeStartsPerDeviceDay = "runtime_starts_per_device_day", 4.0, Model, "replication runtime starts per device per day; each is first contact with every seat (self and seat legs on Key, IO and TD; IOR held until attributed, c580298 runtime.rs:1474)";
    PullPlaneWritesPerDeviceDay = "pull_plane_writes_per_device_day", 10.0, Model, "kicked write cycles per device per day: own rows on a Pull plane (a route change on a network move dominates), merged 30 s apart, one self leg each; Attestation-plane writes kick nothing (c580298 runtime.rs:1320)";
    HostKicksPerDeviceDay = "host_kicks_per_device_day", 0.0, Model, "host round_now kicks per device per day (self and seat legs on the four Pull planes); an idle phone makes none";
    ConsentPeerShare = "consent_peer_share", 0.0, Model, "share of devices whose host names the seat as a consent peer: those round on every bootstrap plane as on v44 (c580298 runtime.rs:1271 targets, :2534); the case that brings the whole-plane bytes back";
    PullReadsPerLeg = "pull_reads_per_leg", 1.0, Model, "counted reads per Pull leg (one subject on one plane); unknown until the edge witness";
    PullRefsPerLeg = "pull_refs_per_leg", 2.0, Model, "refs returned per Pull leg when in sync";
    ClaimedShare = "claimed_share", 0.5, Model, "share of bootstrap devices with an owner (adds the owner to the Pull subjects)";
    AttributionReadsPerCadence = "attribution_reads_per_cadence", 1.0, Derived, "attribution cache lasts 3 s against a 30 s cadence: one store read per device per round";
    // ---- bytes ------------------------------------------------------------
    RefBytes = "ref_bytes", 90.0, Derived, "~90 B per ref in a Summary (edge code; witness pending)";
    FrameOverheadBytes = "frame_overhead_bytes", 100.0, Model, "RNS header, transit seal and framing per frame";
    KeyRecordBytes = "key_record_bytes", 7_500.0, Derived, "7-8 KB per hybrid key record: ML-DSA-65 public key 1,952 B and signature 3,309 B (FIPS 204) plus Ed25519 and encoding";
    OccurrenceRecordBytes = "occurrence_record_bytes", 1_500.0, Model, "bytes per identity occurrence record";
    RouteRecordBytes = "route_record_bytes", 600.0, Model, "bytes per transport destination record";
    RevPlaneRefs = "rev_plane_refs", 5.0, Model, "refs in a revocation plane's offer: the seat's Projection::Global set, revocations it holds of any key in own ∪ cohort, which on a seat is every key it holds (c580298 bridge.rs:11640, the #930 comment there); the refs are few, MODEL until the #971 idle-cost test prints it, but the listing walks a per-subject memo over every key each round, a store read only for subjects the memo lacks";
    // ---- the Revocation listing walk (c580298 bridge.rs:11640), kept apart from the read ledger --
    RevRoundStoreReads = "rev_round_store_reads", 0.0, Model, "counted store reads per Revocation-plane listing with a warm per-subject memo: 0, as the #930 comment says (reads only for subjects the memo lacks); MODEL until the #971 idle-cost witness prints it";
    RevWalkCpuBaseS = "rev_walk_cpu_base_s", 20.0e-6, Model, "a in a + b x keys_held: fixed CPU seconds per Revocation-plane listing (the call, the empty offer); MODEL until the #971 idle-cost witness prints the walk time";
    RevWalkCpuPerKeyS = "rev_walk_cpu_per_key_s", 0.25e-6, Model, "b in a + b x keys_held: CPU seconds per key the listing walks (subjects_for_projection(Global) is every key on a seat, one memo lookup each); MODEL until the witness prints the walk at ~8,700 keys and at 2x";
    RevListingCacheS = "rev_listing_cache_s", 1.0, Model, "life of a Revocation listing cached across peers, in the named listing-cache scenario only (not built; asked of withhold-rate): at most one walk per plane per this many seconds";
    RevColdReadsPerKey = "rev_cold_reads_per_key", 1.0, Model, "store reads for one key the memo lacks (cold-restart scenario only)";
    RevColdReadS = "rev_cold_read_s", 50.0e-6, Model, "wall seconds of one cold per-subject read inside a walk; sets how long the memo takes to fill (cold-restart scenario only)";
    RestartArrivalS = "restart_arrival_s", 30.0, Model, "after a seat restart every device's Revocation round arrives within this window (first contact at once, retries U[0.5, 1.5] x 30 s), cold-restart scenario only";
    SeatOwnRoutes = "seat_own_routes", 4.0, Model, "routes a seat offers a stranger: its own";
    UnownedRouteShare = "unowned_route_share", 0.02, Provisional, "share of routes that are unowned and so served to a stranger";
    StretchFactor = "stretch_factor", 10.0, Model, "stretched idle cadence multiplier (not built)";
    DigestBytes = "digest_bytes", 64.0, Model, "an in-sync round as one small digest frame (not built)";
    PhoneAwakeShare = "phone_awake_share", 0.85, Model, "1 - micro mobile_offline_share: a suspended phone runs no rounds";
    LiveLinkCap = "live_link_cap", 1_024.0, Provisional, "live-link cap with FIFO eviction (CIRISEdge#508, 2026-10); re-read at v45";
    // ---- offload: referral + proxy (MODEL until edge confirms) -----------
    ProxyStoreShare = "proxy_store_share", 1.0, Model, "a proxy's federation-tier store as a share of the seat's";
    NewRowsPerDeviceDay = "new_rows_per_device_day", 20.0, Model, "new federation-tier rows a device's subjects produce per day";
    ProxyMissShare = "proxy_miss_share", 1.0, Model, "share of new rows a proxy lacks and pulls from the seat (one pull per row per proxy)";
    DeviceStartsPerDay = "device_starts_per_day", 60.0, Model, "starts per device per day when it returns to the seat at each start";
    NewDeviceShareDay = "new_device_share_day", 0.01, Model, "share of the fleet that is new each day (first contact under durable homing)";
    ReferralLookupReads = "referral_lookup_reads", 3.0, Model, "counted reads to pick up to three relay-offering nodes with a free shaped slot";
    RootedShare = "rooted_share", 0.0, Model, "share of offloaded devices that share a trust root with their proxy (CC 5.3.4)";
    BindingPullsPerCadence = "binding_pulls_per_cadence", 1.0, Model, "owner-binding pulls a claimed unrooted offloaded device keeps on the seat per cadence";
    // ---- rounds that carry a new row --------------------------------------
    IngestReadsPerRow = "ingest_reads_per_row", 3.0, Model, "counted reads to admit one new row (dedup, supersede, standing) beside its verify";
}

/// Number of cost parameters.
pub const COST_COUNT: usize = COST_SPECS.len();

/// The cost parameters.
#[derive(Debug, Clone)]
pub struct CostParams {
    /// Values in [`C`] order.
    pub v: Vec<f64>,
}

impl Default for CostParams {
    fn default() -> Self {
        Self { v: COST_SPECS.iter().map(|s| s.default).collect() }
    }
}

impl CostParams {
    /// A parameter's value.
    #[must_use]
    pub fn get(&self, c: C) -> f64 {
        self.v[c as usize]
    }

    /// Set a parameter.
    pub fn set(&mut self, c: C, x: f64) {
        self.v[c as usize] = x;
    }

    /// The defaults with `cpu_per_byte_s` fitted to window B.
    #[must_use]
    pub fn calibrated() -> Self {
        let mut p = Self::default();
        let c = calibrate(&p).cpu_per_byte_s;
        p.set(C::CpuPerByte, c);
        p
    }

    /// Index pages one drain of an index of `rows` rows takes.
    #[must_use]
    pub fn pages(&self, rows: f64) -> f64 {
        (rows / self.get(C::PageRows)).ceil().max(1.0)
    }

    /// CPU-seconds one index page costs (also how long it holds a permit).
    #[must_use]
    pub fn page_cpu_s(&self) -> f64 {
        self.get(C::PageRows) * self.get(C::HashRefBytes) * self.get(C::CpuPerByte)
    }

    /// Attestation-index rows for a store holding `nodes` nodes.
    #[must_use]
    pub fn att_rows(&self, nodes: f64) -> f64 {
        self.get(C::SeatAttRows) / self.get(C::AnnouncedNodes) * nodes
    }

    /// Transport-destination rows for a store holding `nodes` nodes.
    #[must_use]
    pub fn td_rows(&self, nodes: f64) -> f64 {
        self.get(C::SeatTdRows) / self.get(C::AnnouncedNodes) * nodes
    }

    /// Key-plane refs offered to a stranger when `nodes` nodes are announced
    /// (announced devices plus their owners, bounded by every key held).
    #[must_use]
    pub fn key_offer_refs(&self, nodes: f64) -> f64 {
        let per = (self.get(C::AnnouncedNodes) + self.get(C::AnnouncedOwners))
            / self.get(C::AnnouncedNodes);
        let bound = self.get(C::SeatKeyRows) / self.get(C::AnnouncedNodes) * nodes;
        (per * nodes).min(bound)
    }
}

/// The six replication planes a seat answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    /// Key plane (drains the attestation index).
    Key = 0,
    /// Identity occurrence (drains the attestation index).
    IdentityOccurrence = 1,
    /// Transport destination (drains the route index).
    TransportDestination = 2,
    /// Key revocation (drains the attestation index).
    Revocation = 3,
    /// Identity occurrence revocation (drains the attestation index).
    IdentityOccurrenceRevocation = 4,
    /// Attestation (drains the attestation index; the only plane with counted reads per refused row).
    Attestation = 5,
}

/// All planes in index order.
pub const PLANES: [Plane; 6] = [
    Plane::Key,
    Plane::IdentityOccurrence,
    Plane::TransportDestination,
    Plane::Revocation,
    Plane::IdentityOccurrenceRevocation,
    Plane::Attestation,
];

impl Plane {
    /// Short name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::IdentityOccurrence => "identity_occurrence",
            Self::TransportDestination => "transport_destination",
            Self::Revocation => "revocation",
            Self::IdentityOccurrenceRevocation => "io_revocation",
            Self::Attestation => "attestation",
        }
    }
}

/// Which read path the seat runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSet {
    /// Edge as shipped: every received Summary drains a whole index.
    AsShipped,
    /// The v45 read-minimisation plan (DERIVED until measured).
    AfterReadMin,
}

impl ReadSet {
    /// Label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AsShipped => "as shipped",
            Self::AfterReadMin => "after read minimisation",
        }
    }
}

/// The shape of a device's bootstrap round with a seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundShape {
    /// Whole-plane diff on five planes each cadence (not shipped as the v45 default).
    WholePlane,
    /// v45 baseline (CIRISEdge#971 c580298): Key, IdentityOccurrence, TransportDestination and
    /// IdentityOccurrenceRevocation by subject Pull of self and seat (owner on Key, IO and IOR,
    /// timer only) on a 300-900 s jittered cycle; first contact at once then every 30 s, up to
    /// six tries, until the seat holds the device's key and route; a write on a Pull plane pulls
    /// self on that plane. The one ordinary 30 s round is Revocation, held until attribution.
    /// A device whose host names the seat as a consent peer rounds on every plane instead
    /// (`consent_peer_share`).
    RequiredSet,
    /// Whole-plane rounds whose in-sync Summary is one digest frame (after the LTS; not built).
    Digest,
    /// Whole-plane rounds at a stretched idle cadence (not built).
    Stretched,
}

impl RoundShape {
    /// Label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WholePlane => "whole-plane diff (not shipped)",
            Self::RequiredSet => "required-set Pull (v45 baseline)",
            Self::Digest => "digest (after the LTS, not built)",
            Self::Stretched => "stretched cadence (not built)",
        }
    }
}

/// Which seats a device rounds with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatSelect {
    /// Every seat in the bundle (as #881 ships).
    AllSeats,
    /// One home seat per device (MODEL).
    HomeOne,
}

/// The rounds and reads arriving at one node, per minute.
#[derive(Debug, Clone, Default)]
pub struct RoundMix {
    /// Received Summaries per minute, by [`Plane`] index.
    pub rounds_per_min: [f64; 6],
    /// Attestation rounds per minute from first-contact peers (each refuses rows).
    pub refusing_att_rounds_per_min: f64,
    /// Rows refused per refusing attestation round.
    pub refused_rows_per_round: f64,
    /// Small reads per minute (device bookkeeping beside the drains).
    pub small_reads_per_min: f64,
    /// Subject-plane Pull reads per minute.
    pub pull_reads_per_min: f64,
    /// Pull requests per minute (frame handling beside their reads).
    pub pull_requests_per_min: f64,
    /// Attribution-check reads per minute.
    pub attribution_reads_per_min: f64,
    /// Owner-binding pulls per minute (offloaded unrooted claimed devices).
    pub binding_pulls_per_min: f64,
    /// Refusals with referrals per minute.
    pub referrals_per_min: f64,
    /// Missing-row pulls from proxies per minute.
    pub row_pulls_per_min: f64,
    /// Reply Summaries this node receives for rounds it initiated, per minute.
    pub reply_summaries_per_min: f64,
    /// New rows arriving to be admitted, per minute (each a verify plus reads).
    pub ingest_rows_per_min: f64,
    /// Stale routes re-delivered per second (seat-1 only).
    pub stale_routes_per_s: f64,
    /// Measured non-edge CPU on the node, in cores.
    pub context_cores: f64,
    /// Attestation-index rows held.
    pub n_att: f64,
    /// Route-index rows held.
    pub n_td: f64,
    /// Key rows held.
    pub n_keys: f64,
    /// Include the window-A candidate terms (memo rebuild, unexplained excess).
    pub window_a_terms: bool,
    /// Revocation-plane listings are cached across peers (`rev_listing_cache_s`; not built).
    pub rev_listing_cache: bool,
}

/// What one node pays per second.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    /// Summaries received per second.
    pub summaries_per_s: f64,
    /// Index pages read per second (uncounted by the store counter).
    pub index_pages_per_s: f64,
    /// Sweep permit acquires per second (pages plus memo rebuilds).
    pub permits_per_s: f64,
    /// Counted attestation queries per second.
    pub att_queries_per_s: f64,
    /// Composer (owner) lookups per second.
    pub composer_per_s: f64,
    /// Consent-grant reads per second.
    pub consent_per_s: f64,
    /// Other counted reads per second (small, pulls, attribution, binding, referral, row pulls, stale).
    pub other_reads_per_s: f64,
    /// Of which attribution-check reads per second.
    pub attribution_per_s: f64,
    /// Counted rows decoded per second.
    pub rows_per_s: f64,
    /// Hybrid verifies per second.
    pub verifies_per_s: f64,
    /// CPU in cores spent on index pages.
    pub cpu_index: f64,
    /// CPU in cores spent on counted queries.
    pub cpu_query: f64,
    /// CPU in cores spent on verifies.
    pub cpu_verify: f64,
    /// CPU in cores spent on per-Summary frame handling.
    pub cpu_frames: f64,
    /// Measured non-edge CPU in cores.
    pub cpu_context: f64,
    /// Holdings-set memory under the v45 plan, bytes.
    pub holdings_bytes: f64,
    /// Global listings walked per second on the Revocation and occurrence-revocation planes
    /// (one per responder round on those planes, or the cached rate). Not part of the read ledger.
    pub rev_walks_per_s: f64,
    /// Keys each walk visits (every key the node holds: Global on a seat).
    pub rev_walk_keys: f64,
    /// Counted store reads those walks make (`rev_round_store_reads` each; 0 with a warm memo).
    pub rev_reads_per_s: f64,
    /// CPU in cores the walks cost: walks/s x (a + b x keys).
    pub cpu_rev_walk: f64,
}

impl Ledger {
    /// All counted store calls per second (what the store counter shows).
    #[must_use]
    pub fn counted_per_s(&self) -> f64 {
        self.att_queries_per_s + self.composer_per_s + self.consent_per_s + self.other_reads_per_s + self.rev_reads_per_s
    }

    /// Edge CPU in cores.
    #[must_use]
    pub fn cpu_edge(&self) -> f64 {
        self.cpu_index + self.cpu_query + self.cpu_verify + self.cpu_frames + self.cpu_rev_walk
    }

    /// Total CPU in cores.
    #[must_use]
    pub fn cpu_total(&self) -> f64 {
        self.cpu_edge() + self.cpu_context
    }
}

/// Book one node's costs from the rounds arriving at it.
#[must_use]
pub fn ledger(p: &CostParams, mix: &RoundMix, read: ReadSet) -> Ledger {
    let pa = p.pages(mix.n_att);
    let pt = p.pages(mix.n_td);
    let spr = p.get(C::SummariesPerRound);
    let mut l = Ledger::default();
    let mut summaries = 0.0;
    let mut drain_pages = 0.0;
    for plane in PLANES {
        let s = mix.rounds_per_min[plane as usize] * spr;
        summaries += s;
        let per = if plane == Plane::TransportDestination { pt } else { pa };
        drain_pages += s * per;
    }
    // Reply Summaries on rounds this node initiated drain the attestation index (MODEL).
    summaries += mix.reply_summaries_per_min;
    drain_pages += mix.reply_summaries_per_min * p.get(C::ReplyDrain) * pa;
    l.summaries_per_s = summaries / 60.0;

    let mut memo_permits = 0.0;
    let mut excess = 0.0;
    if mix.window_a_terms {
        memo_permits = mix.n_keys / p.get(C::KeyMemoExpiryS);
        excess = p.get(C::WindowAExcessPerMin) / 60.0;
    }
    let refusing = mix.refusing_att_rounds_per_min / 60.0;
    let refused = refusing * mix.refused_rows_per_round;
    let required = mix.rounds_per_min[Plane::Attestation as usize] / 60.0 * p.get(C::AttRequiredQueries);
    let mut other = (mix.small_reads_per_min
        + mix.pull_reads_per_min
        + mix.attribution_reads_per_min
        + mix.binding_pulls_per_min
        + mix.referrals_per_min * p.get(C::ReferralLookupReads)
        + mix.row_pulls_per_min
        + mix.ingest_rows_per_min * p.get(C::IngestReadsPerRow))
        / 60.0;
    l.attribution_per_s = mix.attribution_reads_per_min / 60.0;
    match read {
        ReadSet::AsShipped => {
            l.index_pages_per_s = drain_pages / 60.0;
            l.att_queries_per_s = required + refused * p.get(C::QueriesPerRefusedRow);
            l.composer_per_s = refused * p.get(C::ComposerPerRefusedRow);
            l.consent_per_s = refused * p.get(C::ConsentPerRefusedRow);
            l.verifies_per_s = mix.stale_routes_per_s + mix.ingest_rows_per_min / 60.0;
            other += mix.stale_routes_per_s * p.get(C::StaleRouteReads);
        }
        ReadSet::AfterReadMin => {
            // One audit drain per period per index kind, constant in peers.
            l.index_pages_per_s = (pa + pt) / p.get(C::AuditDrainS);
            // One shared probe per second per index kind, when anything arrives.
            if summaries > 0.0 {
                other += 2.0 * p.get(C::ProbePerS);
            }
            l.att_queries_per_s = required;
            l.verifies_per_s = mix.ingest_rows_per_min / 60.0;
            l.holdings_bytes = (mix.n_att + mix.n_td) * p.get(C::HoldingsBytesPerHash);
        }
    }
    l.other_reads_per_s = other;
    l.permits_per_s = l.index_pages_per_s + memo_permits + excess;
    l.rows_per_s = l.att_queries_per_s * p.get(C::RowsPerAttQuery) + l.consent_per_s + other;
    let calls = l.counted_per_s();
    let c = p.get(C::CpuPerByte);
    l.cpu_index = (l.index_pages_per_s + memo_permits + excess)
        * p.get(C::PageRows)
        * p.get(C::HashRefBytes)
        * c;
    l.cpu_query = (l.rows_per_s * p.get(C::RowBytes) + calls * p.get(C::CallOverheadBytes)) * c;
    l.cpu_verify = l.verifies_per_s * p.get(C::HybridVerifyCpuS);
    l.cpu_frames = (l.summaries_per_s + mix.pull_requests_per_min / 60.0) * p.get(C::RoundOverheadCpuS);
    l.cpu_context = mix.context_cores;
    // The Global listing walk on the two revocation planes (c580298 bridge.rs:11640 and the
    // occurrence-withdrawal listing beside it): its own term, so the read fix (which moves the
    // drains above) and a listing cache (which moves this) show apart. Read set does not touch it.
    let rev_rounds = (mix.rounds_per_min[Plane::Revocation as usize]
        + mix.rounds_per_min[Plane::IdentityOccurrenceRevocation as usize])
        / 60.0;
    l.rev_walks_per_s = if mix.rev_listing_cache {
        // at most one walk per plane per cache life, and never more than the rounds asked
        rev_rounds.min(2.0 / p.get(C::RevListingCacheS))
    } else {
        rev_rounds
    };
    l.rev_walk_keys = mix.n_keys;
    l.rev_reads_per_s = l.rev_walks_per_s * p.get(C::RevRoundStoreReads);
    l.cpu_rev_walk = l.rev_walks_per_s * (p.get(C::RevWalkCpuBaseS) + p.get(C::RevWalkCpuPerKeyS) * mix.n_keys)
        + l.rev_reads_per_s * (p.get(C::RowBytes) + p.get(C::CallOverheadBytes)) * c;
    l
}

/// Saturation of one node: one queue over its cores, and its sweep permits.
#[derive(Debug, Clone, Copy, Default)]
pub struct Load {
    /// Demanded CPU in cores.
    pub cpu_cores: f64,
    /// CPU utilisation (demanded / cores).
    pub rho_cpu: f64,
    /// Sweep-permit utilisation (acquires x hold time / permits).
    pub rho_permit: f64,
    /// The binding utilisation.
    pub rho: f64,
    /// CPU-seconds one received round costs on average.
    pub round_service_s: f64,
    /// Round latency: service time times the waiting factor 1/(1-rho); infinite at rho >= 1.
    pub round_latency_s: f64,
}

/// Saturation from a ledger on a node with `cores` cores.
#[must_use]
pub fn load(p: &CostParams, l: &Ledger, cores: f64) -> Load {
    let cpu = l.cpu_total();
    let rho_cpu = cpu / cores;
    let rho_permit = l.permits_per_s * p.page_cpu_s() / p.get(C::SweepPermits);
    let rho = rho_cpu.max(rho_permit);
    let service = if l.summaries_per_s > 0.0 { l.cpu_edge() / l.summaries_per_s } else { 0.0 };
    let latency = if rho >= 1.0 { f64::INFINITY } else { service / (1.0 - rho) };
    Load { cpu_cores: cpu, rho_cpu, rho_permit, rho, round_service_s: service, round_latency_s: latency }
}

// ---------------------------------------------------------------------------
// Calibration
// ---------------------------------------------------------------------------

/// Window B (MEASURED, seat-1, quiet 60 s, 15 inbound links).
pub mod window_b {
    /// Process CPU, cores (utime+stime over the window).
    pub const CPU_CORES: f64 = 0.652;
    /// attestation_query calls.
    pub const ATT_QUERIES: f64 = 19_754.0;
    /// list_composers_referencing_any calls.
    pub const COMPOSER: f64 = 4_512.0;
    /// Sweep permits (index pages, DERIVED-FITS-MEASURED).
    pub const PERMITS: f64 = 2_262.0;
    /// recipient_not_in_send_set withholds.
    pub const WITHHOLDS: f64 = 2_263.0;
    /// Responder rounds completed: attestation, occurrence, key, route.
    pub const ROUNDS: [f64; 4] = [10.0, 11.0, 10.0, 11.0];
    /// Revocation and occurrence-revocation rounds (not counted; ~10 each, DERIVED).
    pub const REV_ROUNDS: f64 = 10.0;
}

/// Window A (MEASURED, seat-1, quiet 60 s, 18 inbound links; no CPU reading).
pub mod window_a {
    /// attestation_query calls.
    pub const ATT_QUERIES: f64 = 19_550.0;
    /// Sweep permits.
    pub const PERMITS: f64 = 6_386.0;
    /// Rows withheld.
    pub const WITHHOLDS: f64 = 2_078.0;
    /// Responder rounds completed: attestation, occurrence, key, route.
    pub const ROUNDS: [f64; 4] = [10.0, 12.0, 11.0, 12.0];
}

/// The 64-minute busy window (MEASURED, seat-1 at 64 min after boot).
pub mod busy_hour {
    /// Sweep permits over the window.
    pub const PERMITS: f64 = 273_750.0;
    /// Window length, seconds.
    pub const SECONDS: f64 = 64.0 * 60.0;
    /// Store attestation queries per second (approximate).
    pub const ATT_QUERIES_PER_S: f64 = 400.0;
    /// Process CPU in steady state, cores (approximate, around one core).
    pub const CPU_CORES: f64 = 1.0;
}

/// The round mix at seat-1 in a measured window: rounds as counted, a store
/// of the measured size, measured stale routes and server context CPU.
#[must_use]
pub fn measured_mix(p: &CostParams, rounds: [f64; 4], rev_rounds: f64, withholds: f64) -> RoundMix {
    let mut m = RoundMix::default();
    m.rounds_per_min[Plane::Attestation as usize] = rounds[0];
    m.rounds_per_min[Plane::IdentityOccurrence as usize] = rounds[1];
    m.rounds_per_min[Plane::Key as usize] = rounds[2];
    m.rounds_per_min[Plane::TransportDestination as usize] = rounds[3];
    m.rounds_per_min[Plane::Revocation as usize] = rev_rounds;
    m.rounds_per_min[Plane::IdentityOccurrenceRevocation as usize] = rev_rounds;
    m.refusing_att_rounds_per_min = rounds[0];
    m.refused_rows_per_round = if rounds[0] > 0.0 { withholds / rounds[0] } else { 0.0 };
    m.stale_routes_per_s = p.get(C::StaleRoutesPerS);
    m.context_cores = p.get(C::ServerRefreshCores);
    m.n_att = p.get(C::SeatAttRows);
    m.n_td = p.get(C::SeatTdRows);
    m.n_keys = p.get(C::SeatKeyRows);
    m
}

/// The result of fitting the one CPU parameter to window B, with the checks
/// against the other measured series.
#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    /// Fitted CPU-seconds per decoded byte.
    pub cpu_per_byte_s: f64,
    /// Window B predicted index pages per minute (derived) vs measured permits.
    pub b_pages_pred: f64,
    /// Window B predicted attestation queries per minute.
    pub b_att_pred: f64,
    /// Window B predicted composer lookups per minute.
    pub b_composer_pred: f64,
    /// Window B CPU split: index pages, counted queries, rest (cores).
    pub b_cpu_index: f64,
    /// See `b_cpu_index`.
    pub b_cpu_query: f64,
    /// See `b_cpu_index`.
    pub b_cpu_rest: f64,
    /// Fitted summaries per round from window B (pages measured / pages per round derived).
    pub b_fitted_summaries_per_round: f64,
    /// Window A predicted permits per minute from its responder rounds alone.
    pub a_pages_pred: f64,
    /// Window A key-memo rebuild permits per minute (candidate term).
    pub a_memo_per_min: f64,
    /// Window A measured permits left after the prediction and the memo candidate.
    pub a_unexplained_per_min: f64,
    /// Busy hour predicted CPU (cores) at its measured page and query rates.
    pub busy_cpu_pred: f64,
}

/// Fit `cpu_per_byte_s` to window B and check the rest.
#[must_use]
pub fn calibrate(base: &CostParams) -> Calibration {
    let mut p = base.clone();
    let mix_b = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS);
    // Decoded bytes at unit CPU cost: set c = 1 and read the CPU terms as bytes.
    p.set(C::CpuPerByte, 1.0);
    let lb = ledger(&p, &mix_b, ReadSet::AsShipped);
    let byte_terms = lb.cpu_index + lb.cpu_query;
    let fixed = lb.cpu_verify + lb.cpu_frames + lb.cpu_context + lb.cpu_rev_walk;
    let c = (window_b::CPU_CORES - fixed) / byte_terms;
    p.set(C::CpuPerByte, c);
    let lb = ledger(&p, &mix_b, ReadSet::AsShipped);

    let mix_a = measured_mix(&p, window_a::ROUNDS, window_b::REV_ROUNDS, window_a::WITHHOLDS);
    let la = ledger(&p, &mix_a, ReadSet::AsShipped);
    let a_pred = la.index_pages_per_s * 60.0;
    let a_memo = p.get(C::SeatKeyRows) / p.get(C::KeyMemoExpiryS) * 60.0;

    // Busy hour: put the measured page and query rates through the CPU terms.
    let pages = busy_hour::PERMITS / busy_hour::SECONDS;
    let q = busy_hour::ATT_QUERIES_PER_S;
    let busy = (pages * p.get(C::PageRows) * p.get(C::HashRefBytes)
        + q * p.get(C::RowsPerAttQuery) * p.get(C::RowBytes)
        + q * p.get(C::CallOverheadBytes))
        * c
        + p.get(C::ServerRefreshCores)
        + p.get(C::StaleRoutesPerS) * p.get(C::HybridVerifyCpuS);

    Calibration {
        cpu_per_byte_s: c,
        b_pages_pred: lb.index_pages_per_s * 60.0,
        b_att_pred: lb.att_queries_per_s * 60.0,
        b_composer_pred: lb.composer_per_s * 60.0,
        b_cpu_index: lb.cpu_index,
        b_cpu_query: lb.cpu_query,
        b_cpu_rest: lb.cpu_verify + lb.cpu_frames + lb.cpu_context + lb.cpu_rev_walk,
        b_fitted_summaries_per_round: window_b::PERMITS / (lb.index_pages_per_s * 60.0 / p.get(C::SummariesPerRound)),
        a_pages_pred: a_pred,
        a_memo_per_min: a_memo,
        a_unexplained_per_min: window_a::PERMITS - a_pred - a_memo,
        busy_cpu_pred: busy,
    }
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// One capacity scenario.
#[derive(Debug, Clone, Copy)]
pub struct Scenario {
    /// Bootstrap devices in the fleet.
    pub devices: f64,
    /// Serve seats.
    pub seats: u32,
    /// Which seats a device rounds with.
    pub select: SeatSelect,
    /// Round shape.
    pub shape: RoundShape,
    /// Read path on every node.
    pub read: ReadSet,
    /// Share of devices referred to a proxy (offload axis; MODEL).
    pub offload_f: f64,
    /// Server-class proxies sharing the offloaded devices evenly.
    pub proxies: u32,
    /// Offloaded devices home on their proxy durably (else return to the seat each start).
    pub durable_homing: bool,
    /// Named scenario, not built: Revocation listings cached across peers (`rev_listing_cache_s`).
    pub rev_listing_cache: bool,
}

impl Scenario {
    /// One seat, no offload.
    #[must_use]
    pub const fn one_seat(devices: f64, shape: RoundShape, read: ReadSet) -> Self {
        Self {
            devices,
            seats: 1,
            select: SeatSelect::AllSeats,
            shape,
            read,
            offload_f: 0.0,
            proxies: 0,
            durable_homing: false,
            rev_listing_cache: false,
        }
    }
}

/// What a scenario costs.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// One seat's ledger.
    pub seat: Ledger,
    /// One seat's saturation.
    pub seat_load: Load,
    /// One proxy's ledger, when there are proxies.
    pub proxy: Option<Ledger>,
    /// One proxy's saturation.
    pub proxy_load: Option<Load>,
    /// Device links held at one seat.
    pub seat_links: f64,
    /// Seat egress, bytes per second (idle rounds).
    pub seat_egress_bytes_per_s: f64,
    /// Bytes per cadence each direction for one in-sync device round set with one seat.
    pub device_bytes_per_cadence: f64,
    /// First-contact bytes a device receives from its first seat.
    pub first_contact_bytes: f64,
}

/// Bytes each direction for one device's in-sync rounds with one seat per
/// cadence, when `nodes` nodes are announced. For [`RoundShape::RequiredSet`] it is the
/// fleet mean over `consent_peer_share` (those devices pay the whole-plane bytes).
#[must_use]
pub fn bytes_per_cadence(p: &CostParams, shape: RoundShape, nodes: f64) -> f64 {
    let rb = p.get(C::RefBytes);
    let fo = p.get(C::FrameOverheadBytes);
    let whole = || {
        let td = p.get(C::SeatOwnRoutes) + p.get(C::UnownedRouteShare) * p.td_rows(nodes);
        (p.key_offer_refs(nodes) + nodes + td + 2.0 * p.get(C::RevPlaneRefs)) * rb + 5.0 * fo
    };
    match shape {
        RoundShape::WholePlane => whole(),
        RoundShape::Stretched => whole() / p.get(C::StretchFactor),
        RoundShape::Digest => 5.0 * (p.get(C::DigestBytes) + fo),
        RoundShape::RequiredSet => {
            let share = p.get(C::ConsentPeerShare).clamp(0.0, 1.0);
            share * whole() + (1.0 - share) * required_set_bytes_per_cadence(p)
        }
    }
}

/// One no-consent device's bytes per cadence in the v45 baseline: the Revocation round (the
/// seat's Projection::Global offer) and its Pull legs amortised.
#[must_use]
pub fn required_set_bytes_per_cadence(p: &CostParams) -> f64 {
    let rb = p.get(C::RefBytes);
    let fo = p.get(C::FrameOverheadBytes);
    let revocation_round = p.get(C::RevPlaneRefs) * rb + fo;
    let legs = pull_legs_per_device_s(p) * p.get(C::CadenceS);
    revocation_round + legs * (p.get(C::PullRefsPerLeg) * rb + fo)
}

/// Pull legs (one subject on one plane) one no-consent device sends one seat per second in
/// the v45 baseline (CIRISEdge#971 c580298 runtime.rs:1447 `cycle`):
/// - timer: self and seat on the four Pull planes, owner on three when claimed, every 600 s mean;
/// - each runtime start: `first_contact_cycles` cycles of self and seat on Key, IO and TD (IOR
///   waits for attribution);
/// - each kicked write cycle: self on the written plane;
/// - each host kick: self and seat on the four Pull planes.
#[must_use]
pub fn pull_legs_per_device_s(p: &CostParams) -> f64 {
    let pull = p.get(C::PullPlanes);
    let timer = 2.0 * pull + p.get(C::ClaimedShare) * p.get(C::OwnerPlanes);
    let first = p.get(C::FirstContactCycles).min(p.get(C::FirstContactTries)) * 2.0 * (pull - 1.0);
    timer / p.get(C::PullCycleS)
        + (p.get(C::RuntimeStartsPerDeviceDay) * first
            + p.get(C::PullPlaneWritesPerDeviceDay)
            + p.get(C::HostKicksPerDeviceDay) * 2.0 * pull)
            / 86_400.0
}

/// First-contact bytes a device receives from its first seat.
#[must_use]
pub fn first_contact_bytes(p: &CostParams, shape: RoundShape, nodes: f64) -> f64 {
    let td = p.get(C::SeatOwnRoutes) + p.get(C::UnownedRouteShare) * p.td_rows(nodes);
    let whole = p.key_offer_refs(nodes) * p.get(C::KeyRecordBytes)
        + nodes * p.get(C::OccurrenceRecordBytes)
        + td * p.get(C::RouteRecordBytes);
    match shape {
        RoundShape::RequiredSet => {
            // The seat's key, occurrence and route (seat legs), then the owner's key and
            // occurrence on the first timer cycle when claimed.
            let kio = p.get(C::KeyRecordBytes) + p.get(C::OccurrenceRecordBytes);
            let required = kio + p.get(C::RouteRecordBytes) + p.get(C::ClaimedShare) * kio;
            let share = p.get(C::ConsentPeerShare).clamp(0.0, 1.0);
            share * whole + (1.0 - share) * required
        }
        _ => whole,
    }
}

/// Add `n` devices' rounds with one seat (per cadence) to a mix.
fn add_devices(p: &CostParams, m: &mut RoundMix, n: f64, shape: RoundShape) {
    let per_min = 60.0 / p.get(C::CadenceS);
    let k = n * per_min;
    match shape {
        RoundShape::WholePlane | RoundShape::Digest | RoundShape::Stretched => {
            let s = if shape == RoundShape::Stretched { 1.0 / p.get(C::StretchFactor) } else { 1.0 };
            for plane in [
                Plane::Key,
                Plane::IdentityOccurrence,
                Plane::TransportDestination,
                Plane::Revocation,
                Plane::IdentityOccurrenceRevocation,
            ] {
                m.rounds_per_min[plane as usize] += k * s;
            }
            m.small_reads_per_min += k * s * p.get(C::SmallReadsPerCadence);
        }
        RoundShape::RequiredSet => {
            // Devices whose host names the seat as a consent peer round as on v44.
            let share = p.get(C::ConsentPeerShare).clamp(0.0, 1.0);
            if share > 0.0 {
                add_devices(p, m, n * share, RoundShape::WholePlane);
            }
            let nr = n * (1.0 - share);
            // The one ordinary round: Revocation (BUNDLE_ROUND_PLANES, c580298 runtime.rs:1037).
            let kr = nr * per_min;
            m.rounds_per_min[Plane::Revocation as usize] += kr;
            m.small_reads_per_min += kr * p.get(C::SmallReadsPerCadence) / 5.0;
            m.attribution_reads_per_min += kr * p.get(C::AttributionReadsPerCadence);
            let legs = nr * pull_legs_per_device_s(p) * 60.0;
            m.pull_requests_per_min += legs;
            m.pull_reads_per_min += legs * p.get(C::PullReadsPerLeg);
            return;
        }
    }
    m.attribution_reads_per_min += k * p.get(C::AttributionReadsPerCadence);
}

/// Cost one scenario.
#[must_use]
pub fn run(p: &CostParams, s: &Scenario) -> Outcome {
    let seats = f64::from(s.seats.max(1));
    let nodes = s.devices.max(p.get(C::StoreFloorNodes));
    let n_att = p.att_rows(nodes);
    let n_td = p.td_rows(nodes);
    let n_keys = p.get(C::SeatKeyRows) / p.get(C::AnnouncedNodes) * nodes;
    let per_min = 60.0 / p.get(C::CadenceS);
    let proxies = if s.offload_f > 0.0 { f64::from(s.proxies.max(1)) } else { 0.0 };
    let offloaded = s.devices * s.offload_f.clamp(0.0, 1.0);
    let at_seat_fleet = s.devices - offloaded;
    let share = match s.select {
        SeatSelect::AllSeats => 1.0,
        SeatSelect::HomeOne => 1.0 / seats,
    };
    let mut m = RoundMix {
        n_att,
        n_td,
        n_keys,
        stale_routes_per_s: p.get(C::StaleRoutesPerS),
        context_cores: p.get(C::ServerRefreshCores),
        rev_listing_cache: s.rev_listing_cache,
        ..RoundMix::default()
    };
    add_devices(p, &mut m, at_seat_fleet * share, s.shape);
    // Fellow seats: each rounds on all six planes, not first contact (no refusals),
    // and this seat drains on the reply to each round it opens with them.
    let fellows = seats - 1.0;
    for plane in PLANES {
        m.rounds_per_min[plane as usize] += fellows * per_min;
    }
    m.reply_summaries_per_min += fellows * 6.0 * per_min;
    // Proxies round with the seat like fellows (all six planes), pull each new row they lack.
    let proxies_here = proxies * share;
    for plane in PLANES {
        m.rounds_per_min[plane as usize] += proxies_here * per_min;
    }
    let new_rows_per_min = s.devices * p.get(C::NewRowsPerDeviceDay) / 1_440.0;
    // Rounds that carry a new row: the seat admits every new row once (from a
    // device it serves or from a proxy pushing one up).
    m.ingest_rows_per_min += new_rows_per_min;
    m.row_pulls_per_min += proxies_here * new_rows_per_min * p.get(C::ProxyMissShare);
    // First hop is always a seat: each offloaded device costs a refusal with referrals.
    let refusals_per_day = if s.durable_homing {
        p.get(C::NewDeviceShareDay)
    } else {
        p.get(C::DeviceStartsPerDay)
    };
    m.referrals_per_min += offloaded * share * refusals_per_day / 1_440.0;
    // Owner-binding pulls stay on a root-sharing node (the seat) for unrooted devices.
    m.binding_pulls_per_min += offloaded
        * share
        * p.get(C::ClaimedShare)
        * (1.0 - p.get(C::RootedShare))
        * p.get(C::BindingPullsPerCadence)
        * per_min;

    let seat = ledger(p, &m, s.read);
    let seat_load = load(p, &seat, p.get(C::SeatCores));

    let (proxy, proxy_load) = if proxies > 0.0 {
        let store = p.get(C::ProxyStoreShare);
        let mut pm = RoundMix {
            n_att: n_att * store,
            n_td: n_td * store,
            n_keys: n_keys * store,
            rev_listing_cache: s.rev_listing_cache,
            ..RoundMix::default()
        };
        add_devices(p, &mut pm, offloaded / proxies, s.shape);
        pm.ingest_rows_per_min += new_rows_per_min * s.offload_f / proxies;
        // The proxy opens six rounds per cadence with each seat it uses and drains on the replies.
        pm.reply_summaries_per_min += 6.0 * per_min * (seats * share).max(1.0);
        let l = ledger(p, &pm, s.read);
        let ld = load(p, &l, p.get(C::ProxyCores));
        (Some(l), Some(ld))
    } else {
        (None, None)
    };

    let dev_bytes = bytes_per_cadence(p, s.shape, nodes);
    let at_seat_devices = at_seat_fleet * share;
    let egress = at_seat_devices * dev_bytes / p.get(C::CadenceS);
    Outcome {
        seat,
        seat_load,
        proxy,
        proxy_load,
        seat_links: at_seat_devices + fellows + proxies_here,
        seat_egress_bytes_per_s: egress,
        device_bytes_per_cadence: dev_bytes,
        first_contact_bytes: first_contact_bytes(p, s.shape, nodes),
    }
}

/// Named scenario, not a default: the seat restarts with an empty per-subject memo and every
/// device's Revocation round arrives within `restart_arrival_s`. Each walk reads the keys the
/// memo still lacks; with no single flight (MODEL), every round that arrives while the memo
/// fills reads, on average, half of them.
#[derive(Debug, Clone, Copy)]
pub struct ColdRestart {
    /// Keys the seat holds (each walk visits all of them).
    pub keys: f64,
    /// Seconds one walk takes to fill the memo from cold.
    pub fill_s: f64,
    /// Revocation rounds that arrive while the memo fills.
    pub rounds_in_fill: f64,
    /// Store reads over the fill.
    pub store_reads: f64,
    /// Store reads per second during the fill.
    pub reads_per_s: f64,
    /// Seat CPU in cores during the fill (reads plus the walks themselves).
    pub cpu_cores: f64,
    /// CPU utilisation during the fill.
    pub rho_cpu: f64,
}

/// Cost the cold-restart scenario for `devices` devices on one seat.
#[must_use]
pub fn cold_restart(p: &CostParams, devices: f64) -> ColdRestart {
    let nodes = devices.max(p.get(C::StoreFloorNodes));
    let keys = p.get(C::SeatKeyRows) / p.get(C::AnnouncedNodes) * nodes;
    let per_key_reads = p.get(C::RevColdReadsPerKey);
    let fill_s = keys * per_key_reads * p.get(C::RevColdReadS);
    let arrivals_per_s = devices / p.get(C::RestartArrivalS).max(1.0e-9);
    let rounds_in_fill = (arrivals_per_s * fill_s).min(devices);
    let store_reads = keys * per_key_reads * (1.0 + rounds_in_fill / 2.0);
    let window = fill_s.max(1.0e-9);
    let reads_per_s = store_reads / window;
    let c = p.get(C::CpuPerByte);
    let walk_cpu = (rounds_in_fill.max(1.0)) * (p.get(C::RevWalkCpuBaseS) + p.get(C::RevWalkCpuPerKeyS) * keys) / window;
    let cpu = reads_per_s * (p.get(C::RowBytes) + p.get(C::CallOverheadBytes)) * c + walk_cpu + p.get(C::ServerRefreshCores);
    ColdRestart {
        keys,
        fill_s,
        rounds_in_fill,
        store_reads,
        reads_per_s,
        cpu_cores: cpu,
        rho_cpu: cpu / p.get(C::SeatCores),
    }
}

/// The Revocation walk apart from the read ledger: per fleet size, the walk as shipped and
/// with a listing cache, and the cold restart (MODEL throughout until the witness prints it).
#[must_use]
pub fn rev_walk_table(p: &CostParams) -> String {
    let mut s = format!(
        "a = {} s, b = {} s per key, warm reads per listing = {} (all MODEL)\n\n| devices | keys held | walks/s | walk CPU cores | with listing cache | warm store reads/s | cold restart: reads | cold fill s | cold rho cpu |\n|---|---|---|---|---|---|---|---|---|\n",
        p.get(C::RevWalkCpuBaseS),
        p.get(C::RevWalkCpuPerKeyS),
        p.get(C::RevRoundStoreReads)
    );
    for d in SWEEP_DEVICES {
        let sc = Scenario::one_seat(d, RoundShape::RequiredSet, ReadSet::AfterReadMin);
        let o = run(p, &sc);
        let mut cached = sc;
        cached.rev_listing_cache = true;
        let oc = run(p, &cached);
        let cold = cold_restart(p, d);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:.2} |\n",
            fmt_n(d),
            fmt_n(o.seat.rev_walk_keys),
            fmt_n(o.seat.rev_walks_per_s),
            fmt_n(o.seat.cpu_rev_walk),
            fmt_n(oc.seat.cpu_rev_walk),
            fmt_n(o.seat.rev_reads_per_s),
            fmt_n(cold.store_reads),
            fmt_n(cold.fill_s),
            cold.rho_cpu,
        ));
    }
    s
}

/// The largest fleet whose seat stays under `cpu_ceiling` on both cores and
/// permits (bisection; cost is monotone in devices).
#[must_use]
pub fn knee(p: &CostParams, base: &Scenario) -> f64 {
    let ceiling = p.get(C::CpuCeiling);
    let ok = |d: f64| {
        let mut s = *base;
        s.devices = d;
        run(p, &s).seat_load.rho <= ceiling
    };
    if !ok(1.0) {
        return 0.0;
    }
    let (mut lo, mut hi) = (1.0_f64, 1.0e9_f64);
    if ok(hi) {
        return hi;
    }
    for _ in 0..80 {
        let mid = (lo * hi).sqrt();
        if ok(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi / lo < 1.001 {
            break;
        }
    }
    lo.floor()
}

// ---------------------------------------------------------------------------
// Invalid configurations (referral + proxy)
// ---------------------------------------------------------------------------

/// What a proposed proxy is and what it would serve.
#[derive(Debug, Clone, Copy)]
pub struct ProxyConfig {
    /// The proxy is a phone.
    pub is_phone: bool,
    /// The proxy is on external power.
    pub on_charge: bool,
    /// The proxy is on a metered network.
    pub metered: bool,
    /// The proxy offers relay and announces it.
    pub offers_relay: bool,
    /// The proxy reports a free shaped slot.
    pub free_shaped_slot: bool,
    /// The proxy would serve an owned node's route to a stranger.
    pub serves_owned_routes: bool,
    /// The proxy would serve rows that are not federation-tier.
    pub serves_non_federation: bool,
    /// Hops on the shaped link from asker to proxy.
    pub shaped_hops: u32,
}

/// Why a proxy configuration is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyViolation {
    /// A phone off charge or on a metered network is never referred.
    PhoneNotReferable,
    /// Referral targets are announced relay-offering nodes only.
    NotRelayOffering,
    /// Referral targets report a free shaped slot.
    NoFreeShapedSlot,
    /// A proxy never serves an owned node's route.
    ServesOwnedRoute,
    /// A proxy serves only federation-tier rows.
    ServesNonFederation,
    /// A shaped link is one hop.
    ShapedLinkMultiHop,
}

impl ProxyConfig {
    /// Every rule this configuration breaks (empty when valid).
    #[must_use]
    pub fn violations(&self) -> Vec<ProxyViolation> {
        let mut v = Vec::new();
        if self.is_phone && (!self.on_charge || self.metered) {
            v.push(ProxyViolation::PhoneNotReferable);
        }
        if !self.offers_relay {
            v.push(ProxyViolation::NotRelayOffering);
        }
        if !self.free_shaped_slot {
            v.push(ProxyViolation::NoFreeShapedSlot);
        }
        if self.serves_owned_routes {
            v.push(ProxyViolation::ServesOwnedRoute);
        }
        if self.serves_non_federation {
            v.push(ProxyViolation::ServesNonFederation);
        }
        if self.shaped_hops != 1 {
            v.push(ProxyViolation::ShapedLinkMultiHop);
        }
        v
    }

    /// A server-class proxy that breaks no rule.
    #[must_use]
    pub const fn server() -> Self {
        Self {
            is_phone: false,
            on_charge: true,
            metered: false,
            offers_relay: true,
            free_shaped_slot: true,
            serves_owned_routes: false,
            serves_non_federation: false,
            shaped_hops: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// Device counts the sweep walks.
pub const SWEEP_DEVICES: [f64; 9] =
    [10.0, 30.0, 100.0, 300.0, 1_000.0, 3_000.0, 10_000.0, 30_000.0, 100_000.0];

fn fmt_n(x: f64) -> String {
    if !x.is_finite() {
        return "inf".to_string();
    }
    let a = x.abs();
    if a >= 1.0e9 {
        format!("{:.2}G", x / 1.0e9)
    } else if a >= 1.0e6 {
        format!("{:.2}M", x / 1.0e6)
    } else if a >= 1.0e4 {
        format!("{:.1}k", x / 1.0e3)
    } else if a >= 100.0 {
        format!("{x:.0}")
    } else if a >= 1.0 {
        format!("{x:.2}")
    } else if a == 0.0 {
        "0".to_string()
    } else {
        format!("{x:.3}")
    }
}

fn fmt_s(x: f64) -> String {
    if !x.is_finite() {
        "unbounded".to_string()
    } else if x >= 1.0 {
        format!("{x:.1} s")
    } else {
        format!("{:.1} ms", x * 1_000.0)
    }
}

fn fmt_bytes(x: f64) -> String {
    if x >= 1.0e9 {
        format!("{:.2} GB", x / 1.0e9)
    } else if x >= 1.0e6 {
        format!("{:.1} MB", x / 1.0e6)
    } else if x >= 1.0e3 {
        format!("{:.1} KB", x / 1.0e3)
    } else {
        format!("{x:.0} B")
    }
}

/// The parameter table, markdown.
#[must_use]
pub fn param_table(p: &CostParams) -> String {
    let mut s = String::from("| parameter | value | tag | source |\n|---|---|---|---|\n");
    for (i, spec) in COST_SPECS.iter().enumerate() {
        let v = p.v[i];
        let shown = if v != 0.0 && v.abs() < 0.001 { format!("{v:.3e}") } else { fmt_n(v) };
        s.push_str(&format!("| `{}` | {} | {} | {} |\n", spec.name, shown, spec.tag.as_str(), spec.source));
    }
    s
}

/// The calibration statement, markdown.
#[must_use]
pub fn calibration_report(p: &CostParams) -> String {
    let c = calibrate(p);
    let mut s = String::new();
    s.push_str("| series | model | measured | gap | tag |\n|---|---|---|---|---|\n");
    let row = |s: &mut String, name: &str, m: f64, x: f64, tag: &str| {
        s.push_str(&format!(
            "| {name} | {} | {} | {:+.1}% | {tag} |\n",
            fmt_n(m),
            fmt_n(x),
            (m - x) / x * 100.0
        ));
    };
    row(&mut s, "window B index pages / min", c.b_pages_pred, window_b::PERMITS, "DERIVED-FITS-MEASURED");
    row(&mut s, "window B attestation queries / min", c.b_att_pred, window_b::ATT_QUERIES, "MEASURED ratios");
    row(&mut s, "window B composer lookups / min", c.b_composer_pred, window_b::COMPOSER, "MEASURED ratios");
    row(&mut s, "window B CPU, cores", c.b_cpu_index + c.b_cpu_query + c.b_cpu_rest, window_b::CPU_CORES, "FITTED (exact by construction)");
    row(&mut s, "busy hour CPU, cores (at its page and query rates)", c.busy_cpu_pred, busy_hour::CPU_CORES, "check, not fitted");
    row(&mut s, "window A permits / min (responder rounds only)", c.a_pages_pred, window_a::PERMITS, "UNEXPLAINED excess");
    s.push_str(&format!(
        "\nFitted `cpu_per_byte_s` = {:.3e} CPU-s per decoded byte ({:.1} MB/s per core); one index page = {:.2} ms.\n\
         Window B CPU split: index pages {:.3} cores, counted queries {:.3} cores, verifies + frames + server context {:.3} cores (split PROVISIONAL: one CPU window cannot separate the two byte classes).\n\
         Summaries per completed round, fitted from window B: {:.3} (derived: {:.1}).\n\
         Window A: {:.0} permits/min predicted from responder rounds, {:.0}/min key-memo rebuild candidate, {:.0}/min left UNEXPLAINED.\n",
        c.cpu_per_byte_s,
        1.0 / c.cpu_per_byte_s / 1.0e6,
        p.page_cpu_s() * 1_000.0,
        c.b_cpu_index,
        c.b_cpu_query,
        c.b_cpu_rest,
        c.b_fitted_summaries_per_round,
        p.get(C::SummariesPerRound),
        c.a_pages_pred,
        c.a_memo_per_min,
        c.a_unexplained_per_min,
    ));
    s
}

/// The one-seat sweep for one shape and read set, markdown.
#[must_use]
pub fn sweep_table(p: &CostParams, shape: RoundShape, read: ReadSet) -> String {
    let mut s = format!(
        "**{}, {}, one seat ({} cores)**\n\n| devices | att-index rows | counted reads/s | index pages/s | CPU cores | rho cpu | rho permits | round latency | egress Mbit/s |\n|---|---|---|---|---|---|---|---|---|\n",
        shape.as_str(),
        read.as_str(),
        p.get(C::SeatCores)
    );
    for d in SWEEP_DEVICES {
        let o = run(p, &Scenario::one_seat(d, shape, read));
        let nodes = d.max(p.get(C::StoreFloorNodes));
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.2} | {:.2} | {} | {} |\n",
            fmt_n(d),
            fmt_n(p.att_rows(nodes)),
            fmt_n(o.seat.counted_per_s()),
            fmt_n(o.seat.index_pages_per_s),
            fmt_n(o.seat_load.cpu_cores),
            o.seat_load.rho_cpu,
            o.seat_load.rho_permit,
            fmt_s(o.seat_load.round_latency_s),
            fmt_n(o.seat_egress_bytes_per_s * 8.0 / 1.0e6),
        ));
    }
    s
}

/// Knee table: largest fleet under the ceiling, by shape, read set, seats and selection.
#[must_use]
pub fn knee_table(p: &CostParams) -> String {
    let mut s = String::from(
        "| round shape | read set | 1 seat | 2 seats, all | 2 seats, home one | 4 seats, all | 4 seats, home one |\n|---|---|---|---|---|---|---|\n",
    );
    for shape in [RoundShape::WholePlane, RoundShape::RequiredSet, RoundShape::Digest, RoundShape::Stretched] {
        for read in [ReadSet::AsShipped, ReadSet::AfterReadMin] {
            let mut cells = Vec::new();
            for (seats, select) in [
                (1, SeatSelect::AllSeats),
                (2, SeatSelect::AllSeats),
                (2, SeatSelect::HomeOne),
                (4, SeatSelect::AllSeats),
                (4, SeatSelect::HomeOne),
            ] {
                let mut sc = Scenario::one_seat(1.0, shape, read);
                sc.seats = seats;
                sc.select = select;
                cells.push(fmt_n(knee(p, &sc)));
            }
            s.push_str(&format!("| {} | {} | {} |\n", shape.as_str(), read.as_str(), cells.join(" | ")));
        }
    }
    s
}

/// Offload table (referral + proxy, MODEL) at `devices`, markdown.
#[must_use]
pub fn offload_table(p: &CostParams, devices: f64, shape: RoundShape) -> String {
    let mut s = format!(
        "**Offload at {} devices, {}, one seat** (seat cores / one proxy's cores; returns to the seat each start)\n\n| f | S | seat, as shipped | proxy, as shipped | seat, after | proxy, after | seat CPU per proxy peer, as shipped | per proxy peer, after |\n|---|---|---|---|---|---|---|---|\n",
        fmt_n(devices),
        shape.as_str()
    );
    for f in [0.0, 0.5, 0.9, 1.0] {
        for proxies in [1u32, 4, 16, 64] {
            if f == 0.0 && proxies > 1 {
                continue;
            }
            let mut cells = Vec::new();
            let mut per_peer = Vec::new();
            for read in [ReadSet::AsShipped, ReadSet::AfterReadMin] {
                let sc = Scenario {
                    devices,
                    seats: 1,
                    select: SeatSelect::AllSeats,
                    shape,
                    read,
                    offload_f: f,
                    proxies,
                    durable_homing: false,
                    rev_listing_cache: false,
                };
                let o = run(p, &sc);
                cells.push(fmt_n(o.seat_load.cpu_cores));
                cells.push(o.proxy_load.map_or("-".to_string(), |l| fmt_n(l.cpu_cores)));
                // Marginal seat cost of one more proxy peer with the same devices.
                let mut sc2 = sc;
                sc2.proxies = proxies + 1;
                let o2 = run(p, &sc2);
                per_peer.push(if f > 0.0 { fmt_n(o2.seat_load.cpu_cores - o.seat_load.cpu_cores) } else { "-".to_string() });
            }
            s.push_str(&format!(
                "| {f} | {} | {} | {} |\n",
                if f == 0.0 { "-".to_string() } else { proxies.to_string() },
                cells.join(" | "),
                per_peer.join(" | ")
            ));
        }
    }
    s
}

/// Bandwidth table over the announced-key axis, markdown.
#[must_use]
pub fn bandwidth_table(p: &CostParams) -> String {
    let cell = f64::from(BudgetClass::PhoneCellular.real_share_bytes_per_s());
    let wifi = f64::from(BudgetClass::PhoneWifi.real_share_bytes_per_s());
    let cad = p.get(C::CadenceS);
    let mut s = String::from(
        "| announced refs K | round shape | bytes / cadence each way | phone bytes / day (both ways, awake share) | fits cellular shaped | fits Wi-Fi shaped | seat egress 1k / 10k / 100k devices (Mbit/s) | first contact | first contact at cellular shaped rate |\n|---|---|---|---|---|---|---|---|---|\n",
    );
    // K is the Key-plane offer (announced nodes + owners); nodes = K / refs-per-node.
    let per = (p.get(C::AnnouncedNodes) + p.get(C::AnnouncedOwners)) / p.get(C::AnnouncedNodes);
    for k in [100.0, 1_000.0, 2_690.0, 8_698.0] {
        let nodes = k / per;
        for shape in [RoundShape::WholePlane, RoundShape::Stretched, RoundShape::Digest, RoundShape::RequiredSet] {
            let b = bytes_per_cadence(p, shape, nodes);
            let day = b * 2.0 * 86_400.0 / cad * p.get(C::PhoneAwakeShare);
            let fc = first_contact_bytes(p, shape, nodes);
            let eg: Vec<String> =
                [1_000.0, 10_000.0, 100_000.0].iter().map(|n| fmt_n(n * b / cad * 8.0 / 1.0e6)).collect();
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                fmt_n(k),
                shape.as_str(),
                fmt_bytes(b),
                fmt_bytes(day),
                if b <= cell * cad { "yes" } else { "no" },
                if b <= wifi * cad { "yes" } else { "no" },
                eg.join(" / "),
                fmt_bytes(fc),
                fmt_s(fc / cell),
            ));
        }
    }
    s
}

/// One term of the 1M-node run.
#[derive(Debug, Clone)]
pub struct Term {
    /// What the term is.
    pub name: &'static str,
    /// Value, formatted.
    pub value: String,
    /// Where its inputs come from.
    pub tag: &'static str,
}

/// The 1M-device v45 baseline (required-set Pull, one seat), term by term.
#[must_use]
pub fn million_terms(p: &CostParams, read: ReadSet) -> Vec<Term> {
    let d = 1.0e6;
    let o = run(p, &Scenario::one_seat(d, RoundShape::RequiredSet, read));
    let l = &o.seat;
    let c = p.get(C::CpuPerByte);
    let per_read = (p.get(C::RowBytes) + p.get(C::CallOverheadBytes)) * c;
    let pull_reads = d * pull_legs_per_device_s(p) * p.get(C::PullReadsPerLeg);
    let pull_cpu = pull_reads * per_read;
    let attr_cpu = l.attribution_per_s * per_read;
    vec![
        Term { name: "index pages (drains / audit)", value: format!("{} pages/s, {} cores", fmt_n(l.index_pages_per_s), fmt_n(l.cpu_index)), tag: "DERIVED (page model) x FITTED (cpu/byte)" },
        Term { name: "subject Pull reads", value: format!("{} reads/s, {} cores", fmt_n(pull_reads), fmt_n(pull_cpu)), tag: "MODEL (reads per Pull unknown)" },
        Term { name: "attribution-check reads", value: format!("{} reads/s, {} cores", fmt_n(l.attribution_per_s), fmt_n(attr_cpu)), tag: "DERIVED (3 s cache vs 30 s cadence)" },
        Term { name: "frame handling", value: format!("{} cores", fmt_n(l.cpu_frames)), tag: "MODEL" },
        Term { name: "server relay refresh", value: format!("{} cores", fmt_n(l.cpu_context)), tag: "MEASURED-CONTEXT (not edge)" },
        Term { name: "seat CPU total", value: format!("{} cores of {} (rho {:.1})", fmt_n(o.seat_load.cpu_cores), fmt_n(p.get(C::SeatCores)), o.seat_load.rho), tag: "sum" },
        Term { name: "sweep permits", value: format!("rho {:.2}", o.seat_load.rho_permit), tag: "DERIVED" },
        Term { name: "seat egress", value: format!("{} Mbit/s", fmt_n(o.seat_egress_bytes_per_s * 8.0 / 1.0e6)), tag: "DERIVED (ref bytes) / MODEL (refs per Pull)" },
        Term { name: "device links at the seat", value: format!("{} (cap {})", fmt_n(o.seat_links), fmt_n(p.get(C::LiveLinkCap))), tag: "PROVISIONAL cap" },
        Term { name: "shaped device links at the seat", value: format!("8 of {} (the rest run unshaped on the same homing link)", fmt_n(o.seat_links)), tag: "SOURCED (edge link_shaping.rs:751 NODE_CAP_LINKS; seat class is seat to seat only, :2867)" },
        Term { name: "subject Pull legs", value: format!("{} per s ({} per device per day)", fmt_n(d * pull_legs_per_device_s(p)), fmt_n(pull_legs_per_device_s(p) * 86_400.0)), tag: "DERIVED (#971 c580298 planes, 600 s timer) + MODEL (starts, writes, kicks)" },
        Term { name: "Revocation rounds", value: format!("{} per s (one per device per 30 s, the seat's Projection::Global offer)", fmt_n(d / p.get(C::CadenceS))), tag: "DERIVED (#971 c580298 BUNDLE_ROUND_PLANES) / MODEL (offer size)" },
        Term { name: "Revocation listing walk", value: format!("{} walks/s over {} keys, {} cores, {} store reads/s", fmt_n(l.rev_walks_per_s), fmt_n(l.rev_walk_keys), fmt_n(l.cpu_rev_walk), fmt_n(l.rev_reads_per_s)), tag: "MODEL (a + b x keys_held; c580298 bridge.rs:11640 walks Global per round)" },
        Term { name: "holdings memory (v45 plan)", value: fmt_bytes(l.holdings_bytes), tag: "DERIVED" },
    ]
}

/// The whole capacity report, markdown (what `capacity_sweep` prints).
#[must_use]
pub fn report_markdown(p: &CostParams) -> String {
    let mut s = String::new();
    s.push_str("## Calibration (seat-1)\n\n");
    s.push_str(&calibration_report(p));
    s.push_str("\n## Knee: largest fleet with the seat under the ceiling\n\n");
    s.push_str(&knee_table(p));
    s.push_str("\n## One-seat sweep\n\n");
    for shape in [RoundShape::WholePlane, RoundShape::RequiredSet] {
        for read in [ReadSet::AsShipped, ReadSet::AfterReadMin] {
            s.push_str(&sweep_table(p, shape, read));
            s.push('\n');
        }
    }
    s.push_str("## Revocation listing walk, apart from the read ledger (required-set, after read minimisation)\n\n");
    s.push_str(&rev_walk_table(p));
    s.push_str("\n## Offload: referral + proxy (MODEL)\n\n");
    s.push_str(&offload_table(p, 10_000.0, RoundShape::WholePlane));
    s.push('\n');
    s.push_str(&offload_table(p, 10_000.0, RoundShape::RequiredSet));
    s.push_str("\n## Idle bytes\n\n");
    s.push_str(&bandwidth_table(p));
    for read in [ReadSet::AsShipped, ReadSet::AfterReadMin] {
        s.push_str(&format!("\n## 1M devices, v45 baseline (required-set Pull), one seat, {}\n\n| term | value | tag |\n|---|---|---|\n", read.as_str()));
        for t in million_terms(p, read) {
            s.push_str(&format!("| {} | {} | {} |\n", t.name, t.value, t.tag));
        }
    }
    s.push_str("\n## Parameters\n\n");
    s.push_str(&param_table(p));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn within(x: f64, target: f64, tol: f64) -> bool {
        ((x - target) / target).abs() <= tol
    }

    #[test]
    fn every_parameter_has_a_source() {
        for s in COST_SPECS {
            assert!(!s.source.is_empty(), "{} has no source", s.name);
            assert!(s.default.is_finite(), "{}", s.name);
        }
        assert_eq!(CostParams::default().v.len(), COST_COUNT);
    }

    #[test]
    fn window_b_index_pages_close_within_five_percent() {
        let c = calibrate(&CostParams::default());
        // (10 + 11 + 10 + 10 + 10) x 42 + 11 x 18 = 2,340 against 2,262 measured.
        assert!((c.b_pages_pred - 2_340.0).abs() < 1.0, "{}", c.b_pages_pred);
        assert!(within(c.b_pages_pred, window_b::PERMITS, 0.05), "{}", c.b_pages_pred);
    }

    #[test]
    fn window_b_counted_reads_follow_the_measured_ratios() {
        let c = calibrate(&CostParams::default());
        assert!(within(c.b_att_pred, window_b::ATT_QUERIES, 0.12), "att {}", c.b_att_pred);
        assert!(within(c.b_composer_pred, window_b::COMPOSER, 0.12), "composer {}", c.b_composer_pred);
    }

    #[test]
    fn calibrated_model_reproduces_seat_one() {
        let p = CostParams::calibrated();
        let c = calibrate(&p);
        let total = c.b_cpu_index + c.b_cpu_query + c.b_cpu_rest;
        assert!(within(total, window_b::CPU_CORES, 1e-9), "{total}");
        // The busy hour is not fitted: its page and query rates must land within 25%.
        assert!(within(c.busy_cpu_pred, busy_hour::CPU_CORES, 0.25), "busy {}", c.busy_cpu_pred);
        assert!(c.cpu_per_byte_s > 0.0);
        println!("{}", calibration_report(&p));
    }

    #[test]
    fn window_a_excess_stays_unexplained() {
        let c = calibrate(&CostParams::default());
        assert!(c.a_unexplained_per_min > 2_000.0, "{}", c.a_unexplained_per_min);
    }

    #[test]
    fn an_idle_bootstrap_device_costs_192_pages_per_cadence_as_shipped() {
        let mut p = CostParams::calibrated();
        // Idle: no round carries a new row.
        p.set(C::NewRowsPerDeviceDay, 0.0);
        let one = run(&p, &Scenario::one_seat(1.0, RoundShape::WholePlane, ReadSet::AsShipped));
        let none = run(&p, &Scenario::one_seat(0.0, RoundShape::WholePlane, ReadSet::AsShipped));
        let pages = (one.seat.index_pages_per_s - none.seat.index_pages_per_s) * 30.0;
        let small = (one.seat.other_reads_per_s - none.seat.other_reads_per_s) * 30.0;
        // 4 x 42 + 18 page reads and 6 small reads (+1 attribution read) per 30 s.
        assert!((pages - 186.0).abs() < 1e-6, "{pages}");
        assert!((pages + 6.0 - 192.0).abs() < 1e-6);
        assert!((small - 7.0).abs() < 1e-6, "{small}");
    }

    #[test]
    fn after_read_min_pages_are_flat_in_devices() {
        let p = CostParams::calibrated();
        let a = run(&p, &Scenario::one_seat(2_000.0, RoundShape::WholePlane, ReadSet::AfterReadMin));
        let b = run(&p, &Scenario::one_seat(4_000.0, RoundShape::WholePlane, ReadSet::AfterReadMin));
        // Pages grow only with the store (audit drains), never with rounds.
        let ratio = b.seat.index_pages_per_s / a.seat.index_pages_per_s;
        assert!(ratio < 2.05 && ratio > 1.5, "{ratio}");
        assert!(b.seat.index_pages_per_s < 10.0, "{}", b.seat.index_pages_per_s);
        assert_eq!(b.seat.composer_per_s, 0.0);
    }

    #[test]
    fn shipped_seat_cost_is_superlinear_in_fleet() {
        let p = CostParams::calibrated();
        let a = run(&p, &Scenario::one_seat(10_000.0, RoundShape::WholePlane, ReadSet::AsShipped));
        let b = run(&p, &Scenario::one_seat(20_000.0, RoundShape::WholePlane, ReadSet::AsShipped));
        assert!(b.seat.cpu_index / a.seat.cpu_index > 3.9, "peers x store size");
    }

    #[test]
    fn home_one_splits_load_all_seats_does_not() {
        let p = CostParams::calibrated();
        let mut one = Scenario::one_seat(500.0, RoundShape::WholePlane, ReadSet::AsShipped);
        let base = run(&p, &one).seat.index_pages_per_s;
        one.seats = 2;
        let all = run(&p, &one).seat.index_pages_per_s;
        one.select = SeatSelect::HomeOne;
        let home = run(&p, &one).seat.index_pages_per_s;
        assert!(all >= base, "all seats: each seat keeps every device");
        assert!(home < 0.6 * base, "home one: {home} vs {base}");
    }

    #[test]
    fn saturation_has_a_knee_and_the_fixes_move_it() {
        let p = CostParams::calibrated();
        let o = run(&p, &Scenario::one_seat(100_000.0, RoundShape::WholePlane, ReadSet::AsShipped));
        assert!(o.seat_load.rho >= 1.0 && o.seat_load.round_latency_s.is_infinite());
        let shipped = knee(&p, &Scenario::one_seat(1.0, RoundShape::WholePlane, ReadSet::AsShipped));
        let after = knee(&p, &Scenario::one_seat(1.0, RoundShape::WholePlane, ReadSet::AfterReadMin));
        let rs_after = knee(&p, &Scenario::one_seat(1.0, RoundShape::RequiredSet, ReadSet::AfterReadMin));
        assert!(shipped > 0.0 && after > shipped && rs_after >= after, "{shipped} {after} {rs_after}");
    }

    #[test]
    fn offload_relieves_the_seat_before_the_read_fix() {
        let p = CostParams::calibrated();
        let mut sc = Scenario::one_seat(5_000.0, RoundShape::WholePlane, ReadSet::AsShipped);
        let none = run(&p, &sc).seat_load.cpu_cores;
        sc.offload_f = 0.9;
        sc.proxies = 4;
        let o = run(&p, &sc);
        assert!(o.seat_load.cpu_cores < 0.5 * none);
        assert!(o.proxy.is_some());
        // Each proxy peer costs the seat: more proxies, more seat load.
        sc.proxies = 64;
        assert!(run(&p, &sc).seat_load.cpu_cores > o.seat_load.cpu_cores);
    }

    #[test]
    fn whole_plane_idle_bytes_are_over_a_gigabyte_a_day() {
        let p = CostParams::calibrated();
        let per = (p.get(C::AnnouncedNodes) + p.get(C::AnnouncedOwners)) / p.get(C::AnnouncedNodes);
        let nodes = 2_690.0 / per;
        let b = bytes_per_cadence(&p, RoundShape::WholePlane, nodes);
        let day_one_way = b * 2_880.0;
        // ~4,400 refs x 90 B per cadence, 2,880 cadences a day.
        assert!(day_one_way > 1.0e9 && day_one_way < 1.3e9, "{day_one_way}");
        assert!(b > 5_250.0);
    }

    /// The Revocation round and the consent-peer share decide whether the v45 baseline fits a
    /// cellular shaped cadence (175 B/s real share x 30 s = 5,250 B). A no-consent device's Pull
    /// legs cost about 140 B per 30 s and its Revocation round (the seat's Projection::Global
    /// offer) a few hundred; a device whose host names the seat as a consent peer rounds on
    /// every plane as on v44 and does not fit.
    #[test]
    fn the_revocation_round_and_consent_peer_share_decide_whether_the_baseline_fits_cellular() {
        let mut p = CostParams::calibrated();
        let nodes = p.get(C::AnnouncedNodes);
        let legs = pull_legs_per_device_s(&p)
            * p.get(C::CadenceS)
            * (p.get(C::PullRefsPerLeg) * p.get(C::RefBytes) + p.get(C::FrameOverheadBytes));
        // by hand: (8 + 0.5 x 3) / 600 s + (4 starts x 12 + 10 writes) / day = 0.0165 legs/s,
        // x 30 s x (2 refs x 90 B + 100 B) = 139 B
        assert!(legs > 100.0 && legs < 200.0, "{legs}");
        let no_consent = bytes_per_cadence(&p, RoundShape::RequiredSet, nodes);
        assert!(no_consent < 5_250.0, "{no_consent}");
        assert!((no_consent - required_set_bytes_per_cadence(&p)).abs() < 1e-9);
        p.set(C::ConsentPeerShare, 1.0);
        let consent_peer = bytes_per_cadence(&p, RoundShape::RequiredSet, nodes);
        assert!(consent_peer > 5_250.0, "{consent_peer}");
        assert!((consent_peer - bytes_per_cadence(&p, RoundShape::WholePlane, nodes)).abs() < 1e-6);
    }

    /// An idle device (no writes, no host kicks) makes no edge-side kicks: its legs are the
    /// timer and its starts only. Attestation writes are not in the write term at all.
    #[test]
    fn an_idle_device_pulls_on_the_timer_and_at_its_starts_only() {
        let mut p = CostParams::calibrated();
        p.set(C::PullPlaneWritesPerDeviceDay, 0.0);
        p.set(C::HostKicksPerDeviceDay, 0.0);
        p.set(C::ClaimedShare, 1.0);
        p.set(C::RuntimeStartsPerDeviceDay, 0.0);
        // self + seat on 4 planes, owner on 3: 11 legs per 600 s.
        assert!((pull_legs_per_device_s(&p) - 11.0 / 600.0).abs() < 1e-12);
        p.set(C::RuntimeStartsPerDeviceDay, 1.0);
        // one start, two first-contact cycles, self + seat on Key, IO, TD.
        assert!((pull_legs_per_device_s(&p) - 11.0 / 600.0 - 12.0 / 86_400.0).abs() < 1e-12);
    }

    /// The twin's v45 constants are the ones CIRISEdge#971 has at the cited commit. Set
    /// `CEWP_EDGE_RUNTIME_RS` to `src/replication/runtime.rs` of an edge checkout to compare;
    /// unset, only the parameters' agreement with [`edge_971`] is checked.
    #[test]
    fn edge_971_constants_match_the_source() {
        let p = CostParams::default();
        assert_eq!(p.get(C::PullPlanes), 4.0);
        assert_eq!(p.get(C::OwnerPlanes), 3.0);
        assert_eq!(p.get(C::PullCycleS), 600.0);
        assert_eq!(p.get(C::FirstContactSpacingS), 30.0);
        assert_eq!(p.get(C::FirstContactTries), 6.0);
        assert_eq!(p.get(C::CadenceS), edge_971::BUNDLE_PULL_MIN_SPACING_S);
        let Ok(path) = std::env::var("CEWP_EDGE_RUNTIME_RS") else {
            eprintln!(
                "edge_971_constants_match_the_source: CEWP_EDGE_RUNTIME_RS unset, source NOT compared (twin cites {})",
                edge_971::COMMIT
            );
            return;
        };
        let src = std::fs::read_to_string(&path).expect("CEWP_EDGE_RUNTIME_RS names a readable file");
        let planes = |name: &str| -> Vec<String> {
            let at = src.find(&format!("pub const {name}: [EnvelopeKind;")).unwrap_or_else(|| panic!("{name} not in {path}"));
            let body = &src[at..];
            let open = body.find("= [").expect("array") + 3;
            let close = body[open..].find("];").expect("end") + open;
            body[open..close]
                .split(',')
                .map(|t| t.trim().trim_start_matches("EnvelopeKind::").to_owned())
                .filter(|t| !t.is_empty())
                .collect()
        };
        assert_eq!(planes("BUNDLE_PULL_PLANES"), edge_971::BUNDLE_PULL_PLANES, "edge changed since {}", edge_971::COMMIT);
        assert_eq!(planes("BUNDLE_OWNER_PLANES"), edge_971::BUNDLE_OWNER_PLANES, "edge changed since {}", edge_971::COMMIT);
        assert_eq!(planes("BUNDLE_ROUND_PLANES"), edge_971::BUNDLE_ROUND_PLANES, "edge changed since {}", edge_971::COMMIT);
        for (needle, what) in [
            ("BUNDLE_PULL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(600)", "BUNDLE_PULL_INTERVAL 600 s"),
            ("BUNDLE_PULL_MIN_SPACING: std::time::Duration = std::time::Duration::from_secs(30)", "BUNDLE_PULL_MIN_SPACING 30 s"),
            ("const BUNDLE_FIRST_CONTACT_TRIES: u32 = 6;", "BUNDLE_FIRST_CONTACT_TRIES 6"),
            ("jittered(BUNDLE_PULL_MIN_SPACING, &mut rand::thread_rng())", "first-contact retry jitter"),
            ("jittered(BUNDLE_PULL_INTERVAL, &mut rand::thread_rng())", "timer jitter"),
            ("rng.gen_range(ms / 2..=ms.saturating_mul(3) / 2)", "jitter range U[0.5, 1.5]"),
        ] {
            assert!(src.contains(needle), "{what} changed since {}", edge_971::COMMIT);
        }
    }

    /// The walk is its own term: the read fix leaves it, a listing cache moves it, it grows with
    /// keys held (a + b x keys), and a warm memo makes no store reads.
    #[test]
    fn the_revocation_walk_is_kept_apart_from_the_read_ledger() {
        let p = CostParams::calibrated();
        let d = 100_000.0;
        let shipped = run(&p, &Scenario::one_seat(d, RoundShape::RequiredSet, ReadSet::AsShipped));
        let fixed = run(&p, &Scenario::one_seat(d, RoundShape::RequiredSet, ReadSet::AfterReadMin));
        assert!(shipped.seat.cpu_rev_walk > 0.0);
        assert!(within(fixed.seat.cpu_rev_walk, shipped.seat.cpu_rev_walk, 1e-9), "the read fix does not move the walk");
        assert!(fixed.seat.cpu_index < shipped.seat.cpu_index, "the read fix moves the drains");
        let mut sc = Scenario::one_seat(d, RoundShape::RequiredSet, ReadSet::AfterReadMin);
        sc.rev_listing_cache = true;
        let cached = run(&p, &sc);
        assert!(cached.seat.cpu_rev_walk < fixed.seat.cpu_rev_walk / 100.0, "a listing cache moves the walk");
        assert!(within(cached.seat.cpu_index, fixed.seat.cpu_index, 1e-9), "and not the drains");
        assert_eq!(fixed.seat.rev_reads_per_s, 0.0, "warm memo: no store reads");
        // a + b x keys: twice the fleet (above the store floor) is twice the keys, and the
        // per-walk cost rises by b x keys.
        let two = run(&p, &Scenario::one_seat(2.0 * d, RoundShape::RequiredSet, ReadSet::AfterReadMin));
        assert!(within(two.seat.rev_walk_keys, 2.0 * fixed.seat.rev_walk_keys, 1e-9));
        let per_walk = |o: &Outcome| o.seat.cpu_rev_walk / o.seat.rev_walks_per_s;
        let b = p.get(C::RevWalkCpuPerKeyS);
        assert!(within(per_walk(&two) - per_walk(&fixed), b * fixed.seat.rev_walk_keys, 1e-6));
    }

    /// The cold restart is a named scenario: it reads at least every key once, more when
    /// rounds arrive during the fill, and it does not enter the steady-state run.
    #[test]
    fn a_cold_restart_reads_every_key_and_more_when_rounds_overlap_the_fill() {
        let p = CostParams::calibrated();
        let small = cold_restart(&p, 10.0);
        let big = cold_restart(&p, 100_000.0);
        assert!(small.store_reads >= small.keys);
        assert!(big.rounds_in_fill > small.rounds_in_fill);
        assert!(big.store_reads / big.keys > small.store_reads / small.keys);
        let steady = run(&p, &Scenario::one_seat(100_000.0, RoundShape::RequiredSet, ReadSet::AfterReadMin));
        assert_eq!(steady.seat.rev_reads_per_s, 0.0, "the steady run stays warm");
    }

    #[test]
    fn invalid_proxy_configurations_are_refused() {
        assert!(ProxyConfig::server().violations().is_empty());
        let mut c = ProxyConfig::server();
        c.is_phone = true;
        c.on_charge = false;
        assert_eq!(c.violations(), vec![ProxyViolation::PhoneNotReferable]);
        c.on_charge = true;
        c.metered = true;
        assert_eq!(c.violations(), vec![ProxyViolation::PhoneNotReferable]);
        let mut c = ProxyConfig::server();
        c.serves_owned_routes = true;
        c.serves_non_federation = true;
        c.shaped_hops = 2;
        assert_eq!(
            c.violations(),
            vec![
                ProxyViolation::ServesOwnedRoute,
                ProxyViolation::ServesNonFederation,
                ProxyViolation::ShapedLinkMultiHop
            ]
        );
    }

    #[test]
    fn report_renders() {
        let r = report_markdown(&CostParams::calibrated());
        assert!(r.contains("Calibration") && r.contains("1M devices"));
        assert!(!r.contains("NaN"));
    }
}
