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
    SummariesPerRound = "summaries_per_round", 1.0, Derived, "one drain per completed responder round (edge code trace); with the Revocation and occurrence-revocation finals at 0 (window C) window B predicts 1,500 pages against 2,262 permits measured, so it no longer fits: see the permit residuals";
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
    KeyMemoExpiryS = "key_memo_expiry_s", 600.0, Derived, "key-plane memo MEMO_TTL 10 min, cleared whole and refilled per subject under the plane lock (9a04428 key_plane_memo.rs:61, needs_rebuild :253); lazy, so 0 at seat-1 today (no Global finals in window C)";
    ReplyDrain = "reply_drain", 1.0, Model, "an initiator drains its index on each reply Summary (window A candidate; UNEXPLAINED until the counters split)";
    PermitResidualBPerMin = "permit_residual_b_per_min", 762.0, Unexplained, "window B: 2,262 sweep permits less 1,500 predicted by one drain per responder final ((10 + 11 + 10) x 42 + 11 x 18); permits = recipient_not_in_send_set withholds (2,262 vs 2,263) and the ledger has no per-withhold permit";
    PermitResidualAPrimePerMin = "permit_residual_a_prime_per_min", 4_784.0, Unexplained, "window A': 6,386 sweep permits less 1,602 predicted ((10 + 12 + 11) x 42 + 12 x 18); not absorbed into any constant; withhold-rate is mapping permit sites to persist doors";
    PermitResidualCPerMin = "permit_residual_c_per_min", 4_524.0, Unexplained, "window C: 7,512 sweep permits less 2,988 predicted ((11 + 43 + 12) x 42 + 12 x 18); the 43 IO finals are themselves unexplained";
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
    OldPeerShare = "old_peer_share", 0.0, Model, "share of devices still on v44 that round with the seat on every bootstrap plane (old peers are kept on purpose, so production is mixed for a long time); with consent peers, the share that lists both Global planes";
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
    // ---- the two Global key planes (Revocation, occurrence withdrawal), apart from the read ledger --
    // Source: CIRISEdge#971 at 9a04428, bridge.rs:7303 `key_plane_listing`, :7390 `refresh_key_plane_memo`,
    // key_plane_memo.rs, key_plane_rows.rs. Each plane's memo lock is held for the whole listing.
    GlobalSubjectsAtSeat = "global_subjects_at_seat", 7_226.0, Provisional, "subjects_for_projection(Global) at seat-1 (withhold-rate's reading, relayed 2026-11-09); scaled per announced node; every listing on a Global plane walks all of them";
    ListingProbeReads = "listing_probe_reads", 1.0, Derived, "change-probe page reads per listing when under 500 rows landed since the cursor (refresh_key_plane_memo, list_attestations_since, PROBE_PAGE 500, 9a04428 bridge.rs:7412); after the read fix, one shared probe per second per plane (probe_per_s)";
    RevWalkCpuBaseS = "rev_walk_cpu_base_s", 20.0e-6, Model, "a in a + b x subjects: fixed CPU seconds per Global listing (the call, the empty offer); MODEL until the #971 idle-cost witness prints the walk time";
    RevWalkCpuPerKeyS = "rev_walk_cpu_per_key_s", 0.25e-6, Model, "b in a + b x subjects: CPU seconds per subject the listing walks (one memo lookup each); MODEL until the witness prints the walk at ~8,700 keys and at 2x";
    StoreReadS = "store_read_s", 50.0e-6, Model, "wall seconds of one store read made while a plane lock is held (a probe page or one subject query); sets lock hold and refill stall";
    RevQueriesPerSubject = "rev_queries_per_subject", 1.0, Model, "store queries to refill one Revocation-plane subject: revocations_for, then get_attestation per revocation held, and revocations are few (9a04428 key_plane_rows.rs:153-165)";
    IorQueriesPerSubject = "ior_queries_per_subject", 5.0, Model, "store queries to refill one occurrence-withdrawal subject: 4 to 6 (9a04428 key_plane_rows.rs:260-283, withhold-rate's reading)";
    RevPlaneClearsPerHour = "rev_plane_clears_per_hour", 0.0, Measured, "key:revocation rows landing per hour: each drops the Revocation memo whole (Dirt::Plane, 9a04428 key_plane_memo.rs:224); seat-1 has held 0 key:revocation:v1 rows ever (server session, 2026-11-09); STRESS_PLANE_CLEARS_PER_HOUR is the named stress value";
    IorPlaneClearsPerHour = "ior_plane_clears_per_hour", 0.0, Measured, "occurrence-revocation carrier rows landing per hour: each drops the occurrence-withdrawal memo whole (Dirt::Plane, key_plane_memo.rs:230); seat-1 holds 9 withdraws rows in all, none since this boot (server session, 2026-11-09)";
    RevNonEmptySubjects = "rev_non_empty_subjects", 0.0, Measured, "Revocation-plane subjects with any row at seat-1: 0 (no key:revocation:v1 row ever); scaled per announced node; what a rebuild from the rows that exist reads";
    IorNonEmptySubjects = "ior_non_empty_subjects", 9.0, Measured, "occurrence-withdrawal subjects with any row at seat-1: at most 9 (9 withdraws rows in all); scaled per announced node";
    RowScanReads = "row_scan_reads", 1.0, Model, "from-rows rebuild only (not built; whether the persist surface lists a plane's rows by type is withhold-rate's question): reads to list the rows that exist on one Global plane";
    RevListingCacheS = "rev_listing_cache_s", 1.0, Model, "listing-cache scenario only (not built; asked of withhold-rate): a Global listing cached across peers for this long, so at most one walk per plane per this many seconds";
    RestartArrivalS = "restart_arrival_s", 30.0, Model, "cold-restart scenario only: after a seat restart every device's Revocation round arrives within this window (first contact at once, retries U[0.5, 1.5] x 30 s)";
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

    /// Share of devices that round on every bootstrap plane under the v45 baseline, so list
    /// both Global planes: consent peers and old (v44) peers.
    #[must_use]
    pub fn lists_both_share(&self) -> f64 {
        let c = self.get(C::ConsentPeerShare).clamp(0.0, 1.0);
        let o = self.get(C::OldPeerShare).clamp(0.0, 1.0);
        1.0 - (1.0 - c) * (1.0 - o)
    }

    /// Subjects with any row on `plane` (a Global plane) when `nodes` nodes are announced.
    #[must_use]
    pub fn non_empty_subjects(&self, plane: Plane, nodes: f64) -> f64 {
        let at_seat = if plane == Plane::Revocation {
            self.get(C::RevNonEmptySubjects)
        } else {
            self.get(C::IorNonEmptySubjects)
        };
        at_seat / self.get(C::AnnouncedNodes) * nodes
    }

    /// Subjects a Global key-plane listing walks when `nodes` nodes are announced.
    #[must_use]
    pub fn global_subjects(&self, nodes: f64) -> f64 {
        self.get(C::GlobalSubjectsAtSeat) / self.get(C::AnnouncedNodes) * nodes
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

/// How a Global key plane's memo is refilled after its TTL (scenario flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoRebuild {
    /// As shipped: cleared whole every `key_memo_expiry_s`, refilled in one listing under the lock.
    #[default]
    Whole,
    /// Rebuilt from the rows that exist: one listing of the plane's rows by type
    /// (`row_scan_reads`) and the queries for the non-empty subjects only (not built; the
    /// persist surface is unconfirmed).
    FromRows,
    /// No TTL rebuild; only `Dirt::Plane` clears refill the plane (not built).
    Removed,
}

impl MemoRebuild {
    /// Label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Whole => "TTL rebuild whole (as shipped)",
            Self::FromRows => "TTL rebuild from existing rows (not built)",
            Self::Removed => "TTL rebuild removed (not built)",
        }
    }
}

/// The two Global key planes.
pub const GLOBAL_PLANES: [Plane; 2] = [Plane::Revocation, Plane::IdentityOccurrenceRevocation];

/// One Global key plane's lock, walk and refill (MODEL until the #971 witness prints lock wait).
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalPlaneLoad {
    /// Listings per second (one per responder round on the plane, or the cached rate).
    pub listings_per_s: f64,
    /// Subjects each listing walks.
    pub subjects: f64,
    /// Wall seconds one listing holds the lock (walk plus its probe reads).
    pub hold_s: f64,
    /// Probe reads per second.
    pub probe_reads_per_s: f64,
    /// TTL and plane-clear refills per second (lazy: none without a listing).
    pub rebuilds_per_s: f64,
    /// Refill store reads per second.
    pub refill_reads_per_s: f64,
    /// Lock held by one refill, seconds: every round on the plane waits this long.
    pub stall_s: f64,
    /// Rounds that arrive during one stall.
    pub rounds_per_stall: f64,
    /// Lock utilisation: listings x hold + refill occupancy. The seat's cores do not help.
    pub rho_lock: f64,
    /// CPU in cores (walk, probes, refills).
    pub cpu: f64,
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
    /// Subjects a Global listing walks.
    pub n_global: f64,
    /// Non-empty subjects per Global plane (Revocation, occurrence withdrawal).
    pub n_non_empty: [f64; 2],
    /// Book the window A' permit residual (UNEXPLAINED) as a constant permit load.
    pub book_permit_residual: bool,
    /// Global listings are cached across peers (`rev_listing_cache_s`; not built).
    pub rev_listing_cache: bool,
    /// How the Global memos are refilled.
    pub memo_rebuild: MemoRebuild,
    /// Book the TTL refills (a measured 60 s window holds a whole refill or none, so the
    /// calibration windows leave them out and report them apart).
    pub refills_booked: bool,
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
    /// The two Global planes (Revocation, occurrence withdrawal), by [`GLOBAL_PLANES`] order.
    pub global: [GlobalPlaneLoad; 2],
    /// Global listings walked per second, both planes. Not part of the read ledger.
    pub rev_walks_per_s: f64,
    /// Subjects each walk visits.
    pub rev_walk_keys: f64,
    /// Probe reads per second on the Global planes.
    pub rev_reads_per_s: f64,
    /// TTL and plane-clear refill reads per second on the Global planes.
    pub ttl_reads_per_s: f64,
    /// CPU in cores on the Global planes (walk, probes, refills).
    pub cpu_rev_walk: f64,
    /// The busier Global plane's lock utilisation.
    pub rho_lock: f64,
}

impl Ledger {
    /// All counted store calls per second (what the store counter shows).
    #[must_use]
    pub fn counted_per_s(&self) -> f64 {
        self.att_queries_per_s
            + self.composer_per_s
            + self.consent_per_s
            + self.other_reads_per_s
            + self.rev_reads_per_s
            + self.ttl_reads_per_s
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

    // No per-withhold permit and no memo-refill permit: the measured excess over the drain
    // prediction stays an UNEXPLAINED residual, booked only when asked.
    let memo_permits = 0.0;
    let excess = if mix.book_permit_residual { p.get(C::PermitResidualAPrimePerMin) / 60.0 } else { 0.0 };
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
    // The two Global key planes, apart from the read ledger: each listing holds the plane's
    // memo lock for its whole walk (9a04428 bridge.rs:7303), so a plane is one server whatever
    // the cores; the TTL refill holds the same lock (bridge.rs:7402, key_plane_memo.rs:253).
    let mut cpu_g = 0.0;
    for (gi, plane) in GLOBAL_PLANES.into_iter().enumerate() {
        let g = global_plane(p, mix, read, plane);
        l.rev_walks_per_s += g.listings_per_s;
        l.rev_reads_per_s += g.probe_reads_per_s;
        l.ttl_reads_per_s += g.refill_reads_per_s;
        l.rho_lock = l.rho_lock.max(g.rho_lock);
        l.rev_walk_keys = g.subjects;
        cpu_g += g.cpu;
        l.global[gi] = g;
    }
    l.cpu_rev_walk = cpu_g;
    l
}

/// One Global plane's listings, lock and refills at this mix.
#[must_use]
pub fn global_plane(p: &CostParams, mix: &RoundMix, read: ReadSet, plane: Plane) -> GlobalPlaneLoad {
    let rounds = mix.rounds_per_min[plane as usize] / 60.0;
    let listings = if mix.rev_listing_cache { rounds.min(1.0 / p.get(C::RevListingCacheS)) } else { rounds };
    let subjects = mix.n_global;
    let t = p.get(C::StoreReadS);
    let probes = match read {
        ReadSet::AsShipped => listings * p.get(C::ListingProbeReads),
        ReadSet::AfterReadMin => listings.min(p.get(C::ProbePerS)),
    };
    let walk_s = p.get(C::RevWalkCpuBaseS) + p.get(C::RevWalkCpuPerKeyS) * subjects;
    let hold_s = if listings > 0.0 { walk_s + probes / listings * t } else { walk_s };
    let (q, clears_h) = if plane == Plane::Revocation {
        (p.get(C::RevQueriesPerSubject), p.get(C::RevPlaneClearsPerHour))
    } else {
        (p.get(C::IorQueriesPerSubject), p.get(C::IorPlaneClearsPerHour))
    };
    let ttl = match mix.memo_rebuild {
        MemoRebuild::Removed => 0.0,
        MemoRebuild::Whole | MemoRebuild::FromRows => 1.0 / p.get(C::KeyMemoExpiryS),
    };
    // Lazy: a refill happens on the next listing, so never more refills than listings.
    let rebuilds = if mix.refills_booked { (ttl + clears_h / 3_600.0).min(listings) } else { 0.0 };
    // Reads per rebuild: every subject's queries as shipped; from the rows that exist, one
    // listing of the plane's rows and the non-empty subjects' queries.
    let non_empty = mix.n_non_empty[usize::from(plane != Plane::Revocation)].min(subjects);
    let per_rebuild = match mix.memo_rebuild {
        MemoRebuild::FromRows => p.get(C::RowScanReads) + non_empty * q,
        MemoRebuild::Whole | MemoRebuild::Removed => subjects * q,
    };
    let refill_reads = rebuilds * per_rebuild;
    let stall_s = per_rebuild * t;
    let c = p.get(C::CpuPerByte);
    let per_read_cpu = (p.get(C::RowBytes) + p.get(C::CallOverheadBytes)) * c;
    GlobalPlaneLoad {
        listings_per_s: listings,
        subjects,
        hold_s,
        probe_reads_per_s: probes,
        rebuilds_per_s: rebuilds,
        refill_reads_per_s: refill_reads,
        stall_s: if rebuilds > 0.0 { stall_s } else { 0.0 },
        rounds_per_stall: rounds * stall_s,
        // every refill read is made under the plane's lock
        rho_lock: listings * hold_s + refill_reads * t,
        cpu: listings * walk_s + (probes + refill_reads) * per_read_cpu,
    }
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
    /// The busier Global plane lock's utilisation (one server per plane; MODEL).
    pub rho_lock: f64,
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
    let rho = rho_cpu.max(rho_permit).max(l.rho_lock);
    let service = if l.summaries_per_s > 0.0 { l.cpu_edge() / l.summaries_per_s } else { 0.0 };
    let latency = if rho >= 1.0 { f64::INFINITY } else { service / (1.0 - rho) };
    Load { cpu_cores: cpu, rho_cpu, rho_permit, rho_lock: l.rho_lock, rho, round_service_s: service, round_latency_s: latency }
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
    /// Revocation and occurrence-revocation finals: not counted in B; 0 in window C
    /// (MEASURED), applied here, so nobody lists the Global planes on seat-1 today.
    pub const REV_ROUNDS: f64 = super::window_c::REV_ROUNDS;
    /// Window length, seconds (two snapshots 60 s apart).
    pub const SECONDS: f64 = 60.0;
}

/// Window A (MEASURED, seat-1, 60.0 s, 18 inbound links; store doors only, no permits, no CPU).
pub mod window_a {
    /// Window length, seconds.
    pub const SECONDS: f64 = 60.0;
    /// attestation_query calls.
    pub const ATT_QUERIES: f64 = 19_550.0;
    /// Rows returned by attestation_query.
    pub const ATT_ROWS: f64 = 147_410.0;
    /// list_composers_referencing_any calls (0 rows).
    pub const COMPOSER: f64 = 3_867.0;
    /// list_live_consent_grants_by calls.
    pub const CONSENT: f64 = 2_104.0;
    /// Rows returned by list_live_consent_grants_by.
    pub const CONSENT_ROWS: f64 = 319.0;
    /// get_attestation calls.
    pub const GET_ATTESTATION: f64 = 613.0;
    /// list_attestations_referencing calls.
    pub const REFERENCING: f64 = 156.0;
    /// list_attestations_since calls.
    pub const SINCE: f64 = 55.0;
    /// Rows returned by list_attestations_since.
    pub const SINCE_ROWS: f64 = 2_048.0;
    /// Folded reads.
    pub const FOLDED: f64 = 719.0;
    /// routed_to_responder.
    pub const ROUTED: f64 = 222.0;
    /// recipient_not_in_send_set withholds.
    pub const WITHHOLDS: f64 = 2_078.0;
    /// identity_row_node_not_announced withholds.
    pub const NOT_ANNOUNCED: f64 = 12.0;
    /// Responder finals: attestation, occurrence, key, route.
    pub const ROUNDS: [f64; 4] = [10.0, 12.0, 10.0, 10.0];
}

/// Window A' (MEASURED, seat-1, 60.0 s, 18 inbound links; sweep permits only).
pub mod window_a_prime {
    /// Window length, seconds.
    pub const SECONDS: f64 = 60.0;
    /// Sweep permits.
    pub const PERMITS: f64 = 6_386.0;
    /// Sum of permit waits, seconds.
    pub const WAIT_SUM_S: f64 = 2.13;
    /// routed_to_responder.
    pub const ROUTED: f64 = 227.0;
    /// Responder finals: attestation, occurrence, key, route.
    pub const ROUNDS: [f64; 4] = [10.0, 12.0, 11.0, 12.0];
}

/// Window C (MEASURED, seat-1, 60.0 s, 20 inbound links).
pub mod window_c {
    /// Window length, seconds.
    pub const SECONDS: f64 = 60.0;
    /// Sweep permits.
    pub const PERMITS: f64 = 7_512.0;
    /// attestation_query calls.
    pub const ATT_QUERIES: f64 = 15_068.0;
    /// Responder finals: attestation, occurrence, key, route.
    pub const ROUNDS: [f64; 4] = [11.0, 43.0, 12.0, 12.0];
    /// Revocation and occurrence-revocation responder finals.
    pub const REV_ROUNDS: f64 = 0.0;
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
    m.n_global = p.get(C::GlobalSubjectsAtSeat);
    m.n_non_empty = [p.get(C::RevNonEmptySubjects), p.get(C::IorNonEmptySubjects)];
    // a 60 s window holds a whole refill or none: the windows are fitted without them
    m.refills_booked = false;
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
    /// Predicted permits per minute from responder finals alone: B, A', C.
    pub permits_pred: [f64; 3],
    /// Measured permits less the prediction: B, A', C (UNEXPLAINED).
    pub permit_residual: [f64; 3],
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

    let pages_at = |rounds: [f64; 4]| {
        ledger(&p, &measured_mix(&p, rounds, window_c::REV_ROUNDS, 0.0), ReadSet::AsShipped).index_pages_per_s * 60.0
    };
    let permits_pred = [pages_at(window_b::ROUNDS), pages_at(window_a_prime::ROUNDS), pages_at(window_c::ROUNDS)];
    let measured = [window_b::PERMITS, window_a_prime::PERMITS, window_c::PERMITS];

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
        permits_pred,
        permit_residual: [0, 1, 2].map(|i| measured[i] - permits_pred[i]),
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
    /// Scenario flag, not built: Global listings cached across peers (`rev_listing_cache_s`).
    pub rev_listing_cache: bool,
    /// Scenario flag: how the Global memos refill (as shipped: whole, every 10 min).
    pub memo_rebuild: MemoRebuild,
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
            memo_rebuild: MemoRebuild::Whole,
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
            let share = p.lists_both_share();
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
            let share = p.lists_both_share();
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
            // Consent peers and old (v44) peers round on every plane, as on v44.
            let share = p.lists_both_share();
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
    let n_global = p.global_subjects(nodes);
    let n_non_empty = GLOBAL_PLANES.map(|g| p.non_empty_subjects(g, nodes));
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
        n_global,
        n_non_empty,
        rev_listing_cache: s.rev_listing_cache,
        memo_rebuild: s.memo_rebuild,
        refills_booked: true,
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
            n_global: n_global * store,
            n_non_empty: n_non_empty.map(|x| x * store),
            rev_listing_cache: s.rev_listing_cache,
            memo_rebuild: s.memo_rebuild,
            refills_booked: true,
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

/// Named scenario, not a default: the seat restarts with an empty memo on both Global planes
/// and every device's Revocation round arrives within `restart_arrival_s`. The plane lock makes
/// the refill single flight (9a04428 bridge.rs:7303: N rounds arriving together cost one fill),
/// so each plane reads every subject once, and every round arriving meanwhile waits on the lock.
#[derive(Debug, Clone, Copy)]
pub struct ColdRestart {
    /// Subjects a Global listing walks.
    pub subjects: f64,
    /// Store reads of the two refills.
    pub store_reads: f64,
    /// The longer of the two planes' refill stalls, seconds.
    pub stall_s: f64,
    /// Revocation rounds that arrive during that stall (each waits on the lock).
    pub rounds_waiting: f64,
}

/// Cost the cold-restart scenario for `devices` devices on one seat.
#[must_use]
pub fn cold_restart(p: &CostParams, devices: f64) -> ColdRestart {
    let nodes = devices.max(p.get(C::StoreFloorNodes));
    let subjects = p.global_subjects(nodes);
    let t = p.get(C::StoreReadS);
    let rev = subjects * p.get(C::RevQueriesPerSubject);
    let ior = subjects * p.get(C::IorQueriesPerSubject);
    let stall_s = rev.max(ior) * t;
    let arrivals_per_s = devices / p.get(C::RestartArrivalS).max(1.0e-9);
    ColdRestart {
        subjects,
        store_reads: rev + ior,
        stall_s,
        rounds_waiting: (arrivals_per_s * rev * t).min(devices),
    }
}

/// The largest fleet whose busier Global plane lock stays under utilisation 1 (bisection).
#[must_use]
pub fn lock_knee(p: &CostParams, base: &Scenario) -> f64 {
    let ok = |d: f64| {
        let mut s = *base;
        s.devices = d;
        run(p, &s).seat.rho_lock < 1.0
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
    }
    lo
}

/// The Global planes at today's fleet: v44 as measured (window B's rounds) and v45 projected.
#[must_use]
pub fn global_today_table(p: &CostParams) -> String {
    let v44 = ledger(p, &measured_mix(p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS), ReadSet::AsShipped);
    let nodes = p.get(C::AnnouncedNodes);
    let v45 = run(p, &Scenario::one_seat(nodes, RoundShape::RequiredSet, ReadSet::AsShipped)).seat;
    let mut s = String::from(
        "| row | Rev listings/s | IOR listings/s | hold ms (Rev) | rho lock Rev | rho lock IOR |\n|---|---|---|---|---|---|\n",
    );
    for (label, l) in [
        ("v44 today (MEASURED rounds, window B)", &v44),
        ("v45 at today's fleet (DERIVED, default ON, one seat)", &v45),
    ] {
        let [r, i] = l.global;
        s.push_str(&format!(
            "| {} | {} | {} | {} | {:.4} | {:.4} |\n",
            label,
            fmt_n(r.listings_per_s),
            fmt_n(i.listings_per_s),
            fmt_n(r.hold_s * 1_000.0),
            r.rho_lock,
            i.rho_lock,
        ));
    }
    s.push_str(&format!(
        "\nToday's fleet is {} announced nodes. Under v45 no-consent devices reach the occurrence-withdrawal plane by subject Pull only, so its listing and its lazy refill do not run. The v44 row books window C's count of Revocation and occurrence-revocation finals ({}), so nobody lists either plane today; under v45 every device lists Revocation.\n",
        fmt_n(nodes),
        fmt_n(window_c::REV_ROUNDS)
    ));
    s
}

/// The old-peer axis: the share of devices that still list both Global planes (v44 peers kept
/// on purpose, consent peers), by fleet size; v45 baseline, as shipped, one seat.
#[must_use]
pub fn mixed_fleet_table(p: &CostParams) -> String {
    let mut s = String::from(
        "| devices | old peer share | rho lock Rev | rho lock IOR | refill reads/s | seat CPU cores | seat rho | lock knee | seat knee |\n|---|---|---|---|---|---|---|---|---|\n",
    );
    for d in [p.get(C::AnnouncedNodes), 10_000.0, 100_000.0] {
        for share in OLD_PEER_SHARES {
            let mut q = p.clone();
            q.set(C::OldPeerShare, share);
            let sc = Scenario::one_seat(d, RoundShape::RequiredSet, ReadSet::AsShipped);
            let o = run(&q, &sc);
            let [r, i] = o.seat.global;
            s.push_str(&format!(
                "| {} | {} | {:.3} | {:.3} | {} | {} | {:.3} | {} | {} |\n",
                fmt_n(d),
                share,
                r.rho_lock,
                i.rho_lock,
                fmt_n(o.seat.ttl_reads_per_s),
                fmt_n(o.seat_load.cpu_cores),
                o.seat_load.rho,
                fmt_n(lock_knee(&q, &sc)),
                fmt_n(knee(&q, &sc)),
            ));
        }
    }
    s
}

/// Old-peer shares the mixed-fleet table sweeps (MODEL axis; production's share is not measured).
pub const OLD_PEER_SHARES: [f64; 5] = [0.0, 0.1, 0.25, 0.5, 1.0];

/// The Global planes per fleet size (required-set, as shipped), apart from the read ledger.
#[must_use]
pub fn rev_walk_table(p: &CostParams) -> String {
    let mut s = format!(
        "a = {} s, b = {} s per subject, one store read = {} s inside the lock (all MODEL); a plane's lock is one server, the seat's {} cores do not help\n\n| devices | subjects | Rev listings/s | hold ms | rho lock Rev | rho lock IOR | refill reads/s | stall per refill s (Rev / IOR) | rounds per stall (Rev) | Global CPU cores | seat CPU cores |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
        p.get(C::RevWalkCpuBaseS),
        p.get(C::RevWalkCpuPerKeyS),
        p.get(C::StoreReadS),
        p.get(C::SeatCores)
    );
    for d in SWEEP_DEVICES {
        let o = run(p, &Scenario::one_seat(d, RoundShape::RequiredSet, ReadSet::AsShipped));
        let [r, i] = o.seat.global;
        s.push_str(&format!(
            "| {} | {} | {} | {} | {:.3} | {:.3} | {} | {} / {} | {} | {} | {} |\n",
            fmt_n(d),
            fmt_n(r.subjects),
            fmt_n(r.listings_per_s),
            fmt_n(r.hold_s * 1_000.0),
            r.rho_lock,
            i.rho_lock,
            fmt_n(o.seat.ttl_reads_per_s),
            fmt_n(r.stall_s),
            fmt_n(i.stall_s),
            fmt_n(r.rounds_per_stall),
            fmt_n(o.seat.cpu_rev_walk),
            fmt_n(o.seat_load.cpu_cores),
        ));
    }
    let c = cold_restart(p, 10_000.0);
    s.push_str(&format!(
        "\nCold restart at 10,000 devices (named scenario): {} subjects, {} store reads, stall {} s, {} Revocation rounds waiting on the lock.\n",
        fmt_n(c.subjects),
        fmt_n(c.store_reads),
        fmt_n(c.stall_s),
        fmt_n(c.rounds_waiting)
    ));
    s
}

/// Which fix moves which term: the three scenario flags apart, at `devices`, required-set.
#[must_use]
pub fn fix_table(p: &CostParams, devices: f64) -> String {
    let mut s = format!(
        "**{} devices, required-set, one seat**\n\n| read fix | listing cache | TTL rebuild | drain pages/s | probe reads/s | refill reads/s | Global CPU cores | rho lock | stall s | lock knee | seat knee |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
        fmt_n(devices)
    );
    let rows: [(ReadSet, bool, MemoRebuild); 6] = [
        (ReadSet::AsShipped, false, MemoRebuild::Whole),
        (ReadSet::AfterReadMin, false, MemoRebuild::Whole),
        (ReadSet::AsShipped, true, MemoRebuild::Whole),
        (ReadSet::AsShipped, false, MemoRebuild::FromRows),
        (ReadSet::AsShipped, false, MemoRebuild::Removed),
        (ReadSet::AfterReadMin, true, MemoRebuild::Removed),
    ];
    for (read, cache, rebuild) in rows {
        let mut sc = Scenario::one_seat(devices, RoundShape::RequiredSet, read);
        sc.rev_listing_cache = cache;
        sc.memo_rebuild = rebuild;
        let o = run(p, &sc);
        let stall = o.seat.global.iter().map(|g| g.stall_s).fold(0.0, f64::max);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {:.3} | {} | {} | {} |\n",
            if read == ReadSet::AfterReadMin { "yes" } else { "no" },
            if cache { "yes" } else { "no" },
            rebuild.as_str(),
            fmt_n(o.seat.index_pages_per_s),
            fmt_n(o.seat.rev_reads_per_s),
            fmt_n(o.seat.ttl_reads_per_s),
            fmt_n(o.seat.cpu_rev_walk),
            o.seat.rho_lock,
            fmt_n(stall),
            fmt_n(lock_knee(p, &sc)),
            fmt_n(knee(p, &sc)),
        ));
    }
    // Stress (named, not the default): plane clears at STRESS_PLANE_CLEARS_PER_HOUR on both.
    let mut q = p.clone();
    q.set(C::RevPlaneClearsPerHour, STRESS_PLANE_CLEARS_PER_HOUR);
    q.set(C::IorPlaneClearsPerHour, STRESS_PLANE_CLEARS_PER_HOUR);
    let sc = Scenario::one_seat(devices, RoundShape::RequiredSet, ReadSet::AsShipped);
    let o = run(&q, &sc);
    s.push_str(&format!(
        "\nStress row (named, not a default; seat-1 measures 0 clears per hour): {} clears per hour on each plane, as shipped: refill reads/s {}, rho lock {:.3}.\n",
        STRESS_PLANE_CLEARS_PER_HOUR,
        fmt_n(o.seat.ttl_reads_per_s),
        o.seat.rho_lock,
    ));
    s
}

/// The TTL refill set against the measured windows. Not fitted: a refill is one burst inside
/// one listing, so a 60 s window holds a whole refill or none.
#[derive(Debug, Clone, Copy)]
pub struct TtlCheck {
    /// Mean refill reads per minute at seat-1 if both planes were listed.
    pub mean_per_min: f64,
    /// One Revocation-plane refill, reads.
    pub rev_burst: f64,
    /// One occurrence-withdrawal refill, reads.
    pub ior_burst: f64,
    /// Revocation and occurrence-revocation finals per minute measured at seat-1 (window C).
    pub global_finals_per_min: f64,
}

/// Named stress value for `rev_plane_clears_per_hour` and `ior_plane_clears_per_hour`
/// (MODEL); seat-1 measures 0 per hour on both.
pub const STRESS_PLANE_CLEARS_PER_HOUR: f64 = 0.5;

/// The TTL refill at seat-1 size.
#[must_use]
pub fn ttl_check(p: &CostParams) -> TtlCheck {
    let n = p.get(C::GlobalSubjectsAtSeat);
    let rev = n * p.get(C::RevQueriesPerSubject);
    let ior = n * p.get(C::IorQueriesPerSubject);
    let per_min = 60.0 / p.get(C::KeyMemoExpiryS);
    TtlCheck {
        mean_per_min: (rev + ior) * per_min,
        rev_burst: rev,
        ior_burst: ior,
        global_finals_per_min: window_c::REV_ROUNDS,
    }
}

/// The TTL check as text.
#[must_use]
pub fn ttl_check_report(p: &CostParams) -> String {
    let t = ttl_check(p);
    let c = calibrate(p);
    format!(
        "Does not fit as derived. All three permit windows are 60.0 s. A whole refill at seat-1 ({} subjects, MODEL {} and {} queries per subject) is one burst of {} (Rev) or {} (IOR) reads, or nothing, in a 60 s window; the windows show permit residuals of {}, {} and {} per minute (B, A', C), none of them a burst of that size. Not tuned to fit.\n\n\
- The refill is lazy and window C counts {} Revocation and occurrence-revocation finals, so on seat-1 today nobody lists the Global planes and the refill does not run: the v44-today row books 0. The term stays for v45, where every device lists Revocation.\n\
- A rebuild from the rows that exist (not built; persist surface unconfirmed) would read {} (Rev) and {} (IOR) per refill at seat-1: {} and {} non-empty subjects (MEASURED upper bound) plus {} row listing per plane.\n",
        fmt_n(p.get(C::GlobalSubjectsAtSeat)),
        p.get(C::RevQueriesPerSubject),
        p.get(C::IorQueriesPerSubject),
        fmt_n(t.rev_burst),
        fmt_n(t.ior_burst),
        fmt_n(c.permit_residual[0]),
        fmt_n(c.permit_residual[1]),
        fmt_n(c.permit_residual[2]),
        fmt_n(t.global_finals_per_min),
        fmt_n(p.get(C::RowScanReads) + p.get(C::RevNonEmptySubjects) * p.get(C::RevQueriesPerSubject)),
        fmt_n(p.get(C::RowScanReads) + p.get(C::IorNonEmptySubjects) * p.get(C::IorQueriesPerSubject)),
        fmt_n(p.get(C::RevNonEmptySubjects)),
        fmt_n(p.get(C::IorNonEmptySubjects)),
        p.get(C::RowScanReads),
    )
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
    row(&mut s, "window B permits / min (one drain per responder final)", c.b_pages_pred, window_b::PERMITS, "UNEXPLAINED residual");
    row(&mut s, "window B attestation queries / min", c.b_att_pred, window_b::ATT_QUERIES, "MEASURED ratios");
    row(&mut s, "window B composer lookups / min", c.b_composer_pred, window_b::COMPOSER, "MEASURED ratios");
    row(&mut s, "window B CPU, cores", c.b_cpu_index + c.b_cpu_query + c.b_cpu_rest, window_b::CPU_CORES, "FITTED (exact by construction)");
    row(&mut s, "busy hour CPU, cores (at its page and query rates)", c.busy_cpu_pred, busy_hour::CPU_CORES, "check, not fitted");
    row(&mut s, "window A' permits / min (one drain per responder final)", c.permits_pred[1], window_a_prime::PERMITS, "UNEXPLAINED residual");
    row(&mut s, "window C permits / min (one drain per responder final)", c.permits_pred[2], window_c::PERMITS, "UNEXPLAINED residual");
    s.push_str(&format!(
        "\nFitted `cpu_per_byte_s` = {:.3e} CPU-s per decoded byte ({:.1} MB/s per core); one index page = {:.2} ms.\n\
         Window B CPU split: index pages {:.3} cores, counted queries {:.3} cores, verifies + frames + server context {:.3} cores (split PROVISIONAL: one CPU window cannot separate the two byte classes).\n\
         Summaries per completed round, fitted from window B: {:.3} (derived: {:.1}).\n\
         Permit residuals (measured less one drain per responder final, Revocation and occurrence-revocation finals 0 as counted in window C): B {:.0}, A' {:.0}, C {:.0} per minute, all UNEXPLAINED and absorbed into no constant. The excess is in permits, not in attestation_query (window C: {} queries over 66 finals that drain the attestation index). Window B's permits equal its recipient_not_in_send_set withholds (2,262 vs 2,263); the ledger has no per-withhold permit, so if that equality is causal it is a missing term, and with the drains as modelled a per-withhold permit would over-predict B.\n",
        c.cpu_per_byte_s,
        1.0 / c.cpu_per_byte_s / 1.0e6,
        p.page_cpu_s() * 1_000.0,
        c.b_cpu_index,
        c.b_cpu_query,
        c.b_cpu_rest,
        c.b_fitted_summaries_per_round,
        p.get(C::SummariesPerRound),
        c.permit_residual[0],
        c.permit_residual[1],
        c.permit_residual[2],
        fmt_n(window_c::ATT_QUERIES),
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
                    memo_rebuild: MemoRebuild::Whole,
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
        Term { name: "Global plane listings", value: format!("{} walks/s over {} subjects, {} cores, {} probe reads/s, {} refill reads/s, lock rho {:.2} (one server per plane)", fmt_n(l.rev_walks_per_s), fmt_n(l.rev_walk_keys), fmt_n(l.cpu_rev_walk), fmt_n(l.rev_reads_per_s), fmt_n(l.ttl_reads_per_s), l.rho_lock), tag: "MODEL (a + b x subjects, lock held per listing, 9a04428 bridge.rs:7303; TTL refill key_plane_memo.rs:61)" },
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
    s.push_str("## Global key planes: lock, walk and TTL refill, apart from the read ledger (MODEL)\n\n");
    s.push_str("### Today's fleet\n\n");
    s.push_str(&global_today_table(p));
    s.push_str("\n### By fleet size (no old peers)\n\n");
    s.push_str(&rev_walk_table(p));
    s.push_str("\n### Old peers: share that still lists both Global planes\n\n");
    s.push_str(&mixed_fleet_table(p));
    s.push_str("\n### Which fix moves which term\n\n");
    for d in [10_000.0, 100_000.0] {
        s.push_str(&fix_table(p, d));
        s.push('\n');
    }
    s.push_str("### TTL refill against the measured windows\n\n");
    s.push_str(&ttl_check_report(p));
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
    fn window_b_pages_follow_the_finals_without_global_rounds() {
        let c = calibrate(&CostParams::default());
        // (10 + 11 + 10) x 42 + 11 x 18 = 1,500 against 2,262 measured: no longer fits.
        assert!((c.b_pages_pred - 1_500.0).abs() < 1.0, "{}", c.b_pages_pred);
        assert!(within(c.b_pages_pred, c.permits_pred[0], 1e-12));
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
    fn the_permit_residuals_stay_unexplained() {
        let p = CostParams::default();
        let c = calibrate(&p);
        let named = [C::PermitResidualBPerMin, C::PermitResidualAPrimePerMin, C::PermitResidualCPerMin];
        for (i, k) in named.into_iter().enumerate() {
            assert!((c.permit_residual[i] - p.get(k)).abs() < 1.0, "{i}: {} vs {}", c.permit_residual[i], p.get(k));
        }
        assert!(c.permit_residual[1] > 4_000.0 && c.permit_residual[2] > 4_000.0);
        // No per-withhold permit: the permits do not move with the withholds.
        let m0 = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, 0.0);
        let m1 = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS);
        let (l0, l1) = (ledger(&p, &m0, ReadSet::AsShipped), ledger(&p, &m1, ReadSet::AsShipped));
        assert!(within(l0.permits_per_s, l1.permits_per_s, 1e-12), "a per-withhold permit appeared");
        // Today nobody lists the Global planes, so the refill does not run.
        assert!(within(l1.rev_walks_per_s, 0.0, 0.0));
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

    /// Each fix moves its own term: the read fix the drains and probes, a listing cache the
    /// walks, a TTL change the refills; the walk is a + b x subjects.
    #[test]
    fn each_fix_moves_its_own_term() {
        let p = CostParams::calibrated();
        let d = 100_000.0;
        let at = |read, cache, rebuild| {
            let mut sc = Scenario::one_seat(d, RoundShape::RequiredSet, read);
            sc.rev_listing_cache = cache;
            sc.memo_rebuild = rebuild;
            run(&p, &sc).seat
        };
        let base = at(ReadSet::AsShipped, false, MemoRebuild::Whole);
        let read = at(ReadSet::AfterReadMin, false, MemoRebuild::Whole);
        let cache = at(ReadSet::AsShipped, true, MemoRebuild::Whole);
        let removed = at(ReadSet::AsShipped, false, MemoRebuild::Removed);
        let rows = at(ReadSet::AsShipped, false, MemoRebuild::FromRows);
        assert!(read.cpu_index < base.cpu_index, "the read fix moves the drains");
        assert!(read.rev_reads_per_s < base.rev_reads_per_s, "and the probes");
        assert!(within(read.ttl_reads_per_s, base.ttl_reads_per_s, 1e-9), "not the refills");
        assert!(within(read.rev_walks_per_s, base.rev_walks_per_s, 1e-9), "not the walks");
        assert!(cache.rev_walks_per_s < base.rev_walks_per_s / 100.0, "a listing cache moves the walks");
        assert!(within(cache.cpu_index, base.cpu_index, 1e-9), "not the drains");
        assert!(removed.ttl_reads_per_s < base.ttl_reads_per_s, "removing the TTL moves the refills");
        assert!(within(removed.cpu_index, base.cpu_index, 1e-9));
        assert!(rows.global[0].stall_s < base.global[0].stall_s / 100.0, "a rebuild from the rows shortens the stall");
        assert!(rows.ttl_reads_per_s < base.ttl_reads_per_s / 100.0, "and reads a handful, not every subject");
        assert!(within(rows.rev_walks_per_s, base.rev_walks_per_s, 1e-9), "but walks the same");
        // a + b x subjects
        let two = run(&p, &Scenario::one_seat(2.0 * d, RoundShape::RequiredSet, ReadSet::AsShipped)).seat;
        let walk = |g: &GlobalPlaneLoad| p.get(C::RevWalkCpuBaseS) + p.get(C::RevWalkCpuPerKeyS) * g.subjects;
        assert!(within(two.global[0].subjects, 2.0 * base.global[0].subjects, 1e-9));
        assert!(within(walk(&two.global[0]) - walk(&base.global[0]), p.get(C::RevWalkCpuPerKeyS) * base.global[0].subjects, 1e-6));
    }

    /// A plane's lock is one server: more seat cores leave its utilisation unchanged, and at
    /// production's mix (window B) it is low; it reaches 1 at some fleet size.
    #[test]
    fn the_plane_lock_is_one_server_whatever_the_cores() {
        let mut p = CostParams::calibrated();
        let sc = Scenario::one_seat(10_000.0, RoundShape::RequiredSet, ReadSet::AsShipped);
        let four = run(&p, &sc);
        p.set(C::SeatCores, 64.0);
        let many = run(&p, &sc);
        assert!(within(many.seat.rho_lock, four.seat.rho_lock, 1e-9));
        assert!(many.seat_load.rho_cpu < four.seat_load.rho_cpu);
        let p = CostParams::calibrated();
        let mix = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS);
        assert!(ledger(&p, &mix, ReadSet::AsShipped).rho_lock < 0.5, "production today is far from the lock");
        let k = lock_knee(&p, &Scenario::one_seat(1.0, RoundShape::RequiredSet, ReadSet::AsShipped));
        assert!(k > 1_000.0 && k < 1.0e9, "the lock saturates at a finite fleet: {k}");
    }

    /// The cold restart is single flight under the lock: each plane reads every subject once.
    #[test]
    fn a_cold_restart_reads_each_subject_once_per_plane() {
        let p = CostParams::calibrated();
        let c = cold_restart(&p, 10_000.0);
        let per = p.get(C::RevQueriesPerSubject) + p.get(C::IorQueriesPerSubject);
        assert!(within(c.store_reads, c.subjects * per, 1e-9));
        assert!(c.stall_s > 0.0 && c.rounds_waiting > 0.0);
        let steady = run(&p, &Scenario::one_seat(10_000.0, RoundShape::RequiredSet, ReadSet::AsShipped));
        assert!(steady.seat.ttl_reads_per_s < c.store_reads, "the steady run books the mean, not the burst");
    }

    /// The TTL refill is reported against windows B and C, not fitted: the calibration windows
    /// book no refill, so the fitted CPU parameter does not move with the refill parameters.
    #[test]
    fn the_ttl_refill_is_reported_not_fitted() {
        let mut p = CostParams::default();
        let c0 = calibrate(&p).cpu_per_byte_s;
        p.set(C::IorQueriesPerSubject, 50.0);
        assert!(within(calibrate(&p).cpu_per_byte_s, c0, 1e-12));
        let t = ttl_check(&CostParams::default());
        assert!(t.ior_burst > t.rev_burst);
        assert!(ttl_check_report(&CostParams::default()).contains("Does not fit as derived"));
        assert!(within(t.global_finals_per_min, 0.0, 0.0));
    }

    /// Old peers bring the occurrence-withdrawal listing back: with none, the IOR plane is
    /// not listed under v45 (no lock, no refill); with some, it is, and the seat costs more.
    /// Plane clears default to the measured 0.
    #[test]
    fn old_peers_bring_the_second_global_plane_back() {
        let mut p = CostParams::calibrated();
        assert!(within(p.get(C::RevPlaneClearsPerHour), 0.0, 0.0));
        assert!(within(p.get(C::IorPlaneClearsPerHour), 0.0, 0.0));
        let sc = Scenario::one_seat(10_000.0, RoundShape::RequiredSet, ReadSet::AsShipped);
        let none = run(&p, &sc).seat;
        assert!(within(none.global[1].listings_per_s, 0.0, 0.0), "IOR not listed under v45");
        assert!(within(none.global[1].refill_reads_per_s, 0.0, 0.0), "so never refilled");
        p.set(C::OldPeerShare, 0.25);
        let some = run(&p, &sc).seat;
        assert!(some.global[1].listings_per_s > 0.0);
        assert!(some.global[1].refill_reads_per_s > 0.0);
        assert!(some.rho_lock >= none.rho_lock);
        assert!(global_today_table(&CostParams::calibrated()).contains("v44 today (MEASURED rounds"));
        assert!(mixed_fleet_table(&CostParams::calibrated()).contains("| 0.25 |"));
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
