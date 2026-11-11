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
//! * `FRONTIER` — nothing supports the value (a server-side unknown); an explicit axis.
//!
//! Every interval on an input, the fit/held-out split of the measured windows and the
//! residual decomposition live in [`crate::seat_calibration`].
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
    /// Measured on an edge debug build (in-memory store, wall time, one sample per point); the
    /// release-build rerun on a disk store is pending.
    MeasuredDebug,
    /// Nothing supports the value: a server-side or future unknown, carried as an explicit
    /// scenario axis (never a hidden constant).
    Frontier,
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
            Self::MeasuredDebug => "MEASURED (debug); release rerun pending",
            Self::Frontier => "FRONTIER",
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


/// The v45 seat and bundle path as edge ships it: release/45.0.0 at 8e46165 (untagged when read,
/// 2026-11-11). Every parameter that rests on edge source cites a file, a symbol and this commit.
/// Earlier reads cited c580298, bb6ca83, 4897789, 95a85ca and 9a04428 on the #971 branch; those
/// commits are not ancestors of 8e46165 (the series was rebased: bb6ca83 is 3bffbdc, 4897789 is
/// 7fa7105, 95a85ca is 5f86c2d, c580298 is e3c20ec, 9a04428 is 6660677).
///
/// The test `edge_971_constants_match_the_source` checks every value here against the edge
/// source when one is reachable (see [`edge_971::source_file`]): with no source it says so and
/// checks only the twin's own agreement.
pub mod edge_971 {
    /// The commit read.
    pub const COMMIT: &str = "8e46165";
    /// runtime.rs `BUNDLE_PULL_PLANES` (:1113): subject-Pulled toward a no-consent seat.
    pub const BUNDLE_PULL_PLANES: [&str; 4] =
        ["Key", "IdentityOccurrence", "TransportDestination", "IdentityOccurrenceRevocation"];
    /// runtime.rs `BUNDLE_OWNER_PLANES` (:1124): the owner leg's planes (timer cycles only).
    pub const BUNDLE_OWNER_PLANES: [&str; 3] = ["Key", "IdentityOccurrence", "IdentityOccurrenceRevocation"];
    /// runtime.rs `BUNDLE_ROUND_PLANES` (:1140): the one ordinary round a device keeps with a seat.
    pub const BUNDLE_ROUND_PLANES: [&str; 1] = ["Revocation"];
    /// runtime.rs `BUNDLE_PULL_INTERVAL` (:1154), `Duration::from_mins(10)`.
    pub const BUNDLE_PULL_INTERVAL_S: f64 = 600.0;
    /// runtime.rs `BUNDLE_PULL_MIN_SPACING` (:1159); first-contact retries draw U[0.5, 1.5] of it
    /// (`first_contact_delay` :1648, `jittered` :1653).
    pub const BUNDLE_PULL_MIN_SPACING_S: f64 = 30.0;
    /// runtime.rs `BUNDLE_FIRST_CONTACT_TRIES` (:1365).
    pub const BUNDLE_FIRST_CONTACT_TRIES: u32 = 6;
    /// key_plane_memo.rs `MEMO_TTL` (:97), `Duration::from_mins(10)`: past it a memo opens a new
    /// epoch (`PlaneMemo::revalidate` :462, stale-while-revalidate), it is no longer cleared.
    pub const MEMO_TTL_S: f64 = 600.0;
    /// key_plane_memo.rs `KEY_PLANE_READ_BUDGET` (:296): store reads per key-plane listing, every
    /// cause together, the host's own keys outside it; on by default (bridge.rs
    /// `BridgeConfig::default` :1221).
    pub const KEY_PLANE_READ_BUDGET: usize = 256;
    /// key_plane_memo.rs `LISTING_CACHE_SLOTS` (:304); the listing cache is on by default
    /// (bridge.rs `default_key_plane_listing_cache` :1195).
    pub const LISTING_CACHE_SLOTS: usize = 4;
    /// store_feed.rs `FEED_PROBE_SPACING` (:86): one probe per cursor per 5 s unless a door
    /// marked the feed dirty.
    pub const FEED_PROBE_SPACING_S: f64 = 5.0;
    /// bridge.rs `OWNER_BINDING_MEMO_TTL` (:3168): the reach memo (`identity_row_reach` :13985)
    /// and the own-set memo (`own_set_of` :14245) live this long.
    pub const OWNER_BINDING_MEMO_TTL_S: f64 = 30.0;
    /// bridge.rs `CONSENT_SEND_SET_MEMO_TTL` (:3150): a consent send-set read (one sweep permit,
    /// `resolved_peer_set` :11338) at most once per this when nothing invalidates it.
    pub const CONSENT_SEND_SET_MEMO_TTL_S: f64 = 10.0;
    /// holdings_index.rs `DEFAULT_AUDIT_EVERY` (:222), `Duration::from_mins(1)`.
    pub const HOLDINGS_AUDIT_EVERY_S: f64 = 60.0;
    /// bridge.rs `DEFAULT_ADVERTISE_SWEEP_PERMITS` (:1128).
    pub const ADVERTISE_SWEEP_PERMITS: usize = 2;
    /// bridge.rs `DEFAULT_SWEEP_PAGE_ROWS` (:1069).
    pub const SWEEP_PAGE_ROWS: u32 = 1024;

    /// One check of the twin against the source: a file under `src/`, and text it must contain.
    #[derive(Debug, Clone, Copy)]
    pub struct Needle {
        /// Path under the edge checkout.
        pub file: &'static str,
        /// Text the file must contain at [`COMMIT`].
        pub text: &'static str,
        /// What the twin rests on it for.
        pub what: &'static str,
    }

    /// Every source fact the cost parameters cite, as text the file holds at [`COMMIT`].
    pub const NEEDLES: &[Needle] = &[
        Needle { file: "src/replication/runtime.rs", text: "pub const BUNDLE_PULL_INTERVAL: std::time::Duration = std::time::Duration::from_mins(10);", what: "BUNDLE_PULL_INTERVAL 600 s" },
        Needle { file: "src/replication/runtime.rs", text: "pub const BUNDLE_PULL_MIN_SPACING: std::time::Duration = std::time::Duration::from_secs(30);", what: "BUNDLE_PULL_MIN_SPACING 30 s" },
        Needle { file: "src/replication/runtime.rs", text: "const BUNDLE_FIRST_CONTACT_TRIES: u32 = 6;", what: "BUNDLE_FIRST_CONTACT_TRIES 6" },
        Needle { file: "src/replication/runtime.rs", text: "jittered(BUNDLE_PULL_MIN_SPACING, &mut rand::thread_rng())", what: "first-contact retry jitter" },
        Needle { file: "src/replication/runtime.rs", text: "jittered(BUNDLE_PULL_INTERVAL, &mut rand::thread_rng())", what: "timer jitter" },
        Needle { file: "src/replication/runtime.rs", text: "rng.gen_range(ms / 2..=ms.saturating_mul(3) / 2)", what: "jitter range U[0.5, 1.5]" },
        Needle { file: "src/replication/runtime.rs", text: "if kind == EnvelopeKind::IdentityOccurrenceRevocation && !attributed {", what: "the IOR leg waits for attribution" },
        Needle { file: "src/replication/key_plane_memo.rs", text: "pub(crate) const MEMO_TTL: Duration = Duration::from_mins(10);", what: "MEMO_TTL 600 s" },
        Needle { file: "src/replication/key_plane_memo.rs", text: "pub(crate) const KEY_PLANE_READ_BUDGET: usize = 256;", what: "read budget 256" },
        Needle { file: "src/replication/key_plane_memo.rs", text: "pub(crate) const LISTING_CACHE_SLOTS: usize = 4;", what: "listing cache slots 4" },
        Needle { file: "src/replication/bridge.rs", text: "RebuildCause::Ttl => memo.revalidate(now)", what: "the TTL revalidates (stale-while-revalidate), it does not clear" },
        Needle { file: "src/replication/key_plane_memo.rs", text: "(R::KeyRecord | R::Revocation, Some(KeyPlaneFamily::KeyRecord)) => {", what: "a key:record row dirties the Revocation memo" },
        Needle { file: "src/replication/bridge.rs", text: "key_plane_listing_cache: Self::default_key_plane_listing_cache(),", what: "listing cache on by default" },
        Needle { file: "src/replication/bridge.rs", text: "key_plane_read_budget: super::key_plane_memo::KEY_PLANE_READ_BUDGET,", what: "read budget on by default" },
        Needle { file: "src/replication/bridge.rs", text: "if let Some(refs) = memo.cached_listing(&subjects) {", what: "a cache hit is served under the plane lock" },
        Needle { file: "src/replication/bridge.rs", text: ".enter_at(crate::observability::SWEEP_PERMIT_KEY_PLANE_SUBJECT)", what: "one permit per key-plane subject read" },
        Needle { file: "src/replication/bridge.rs", text: "self.subjects_for_projection(Projection::Global)", what: "the Global planes list own ∪ cohort" },
        Needle { file: "src/replication/bridge.rs", text: "// CIRISEdge#930 — Global is every key on a canonical: answered from", what: "the Global set is every key on a canonical" },
        Needle { file: "src/replication/bridge.rs", text: "const OWNER_BINDING_MEMO_TTL: Duration = Duration::from_secs(30);", what: "reach memo 30 s" },
        Needle { file: "src/replication/bridge.rs", text: "const CONSENT_SEND_SET_MEMO_TTL: Duration = Duration::from_secs(10);", what: "send-set memo 10 s" },
        Needle { file: "src/replication/bridge.rs", text: ".list_identity_occurrences_by_occurrence_key(subject)", what: "the reach rule's read (6244795)" },
        Needle { file: "src/replication/bridge.rs", text: "pub const DEFAULT_ADVERTISE_SWEEP_PERMITS: usize = 2;", what: "two sweep permits" },
        Needle { file: "src/replication/bridge.rs", text: "pub const DEFAULT_SWEEP_PAGE_ROWS: u32 = 1024;", what: "1,024 rows a page" },
        Needle { file: "src/replication/store_feed.rs", text: "pub(crate) const FEED_PROBE_SPACING: Duration = Duration::from_secs(5);", what: "feed probe spacing 5 s" },
        Needle { file: "src/replication/holdings_index.rs", text: "pub(crate) const DEFAULT_AUDIT_EVERY: Duration = Duration::from_mins(1);", what: "holdings audit every 60 s" },
    ];

    /// The text of `file` from an edge source the environment names, or `None` with why:
    /// - `CEWP_EDGE_SRC`: an edge checkout's working tree (the file is read as it stands);
    /// - `CEWP_EDGE_REPO`: an edge git repository, read at `CEWP_EDGE_REV` (default
    ///   [`COMMIT`]) with `git -C <repo> show <rev>:<file>`, so no checkout is changed;
    /// - `CEWP_EDGE_RUNTIME_RS` (older form): runtime.rs alone.
    ///
    /// No path is built in: with none of these set the comparison is skipped and said so.
    pub fn source_file(file: &str) -> Result<String, String> {
        if let Ok(dir) = std::env::var("CEWP_EDGE_SRC") {
            let path = std::path::Path::new(&dir).join(file);
            return std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()));
        }
        if let Ok(repo) = std::env::var("CEWP_EDGE_REPO") {
            let rev = std::env::var("CEWP_EDGE_REV").unwrap_or_else(|_| COMMIT.to_owned());
            let out = std::process::Command::new("git")
                .args(["-C", &repo, "show", &format!("{rev}:{file}")])
                .output()
                .map_err(|e| format!("git show {rev}:{file} in {repo}: {e}"))?;
            if !out.status.success() {
                return Err(format!("git show {rev}:{file} in {repo}: {}", String::from_utf8_lossy(&out.stderr).trim()));
            }
            return String::from_utf8(out.stdout).map_err(|e| format!("{file} is not UTF-8: {e}"));
        }
        if file == "src/replication/runtime.rs" {
            if let Ok(path) = std::env::var("CEWP_EDGE_RUNTIME_RS") {
                return std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"));
            }
        }
        Err("no edge source named (set CEWP_EDGE_REPO, CEWP_EDGE_SRC or CEWP_EDGE_RUNTIME_RS)".to_owned())
    }
}

cost_params! {
    // ---- store state at the calibration point --------------------------
    SeatAttRows = "seat_att_rows", 42_520.0, Measured, "seat-1 store count, window A (scores 32,293, delegates_to 6,338, supersedes 3,057, consent 732)";
    SeatTdRows = "seat_td_rows", 18_313.0, Measured, "seat-1 transport destinations, window A";
    SeatKeyRows = "seat_key_rows", 8_698.0, Measured, "seat-1 keys, window A; also the code bound on the Key-plane offer";
    AnnouncedNodes = "announced_nodes", 1_349.0, Measured, "seat-1 relay report announced_nodes";
    AnnouncedOwners = "announced_owners", 1_338.0, Measured, "seat-1 relay report owners";
    TodayPeersBooked = "today_peers_booked", 1.0, Model, "1 books window B's measured round mix (today's peers at seat-1: 10/11/10/11 responder finals a minute on attestation/occurrence/key/route, 0 Global, 2,263 rows withheld; MEASURED) as a fixed baseline in every scenario; the rounds and the rows withheld per attestation round stay fixed (the withhold gates scale with Attestation rounds, not devices: edge source read 2026-11-10 at v44.1.2 (withhold-rate)). Devices drawn as consent or old peers on top may double-count some of it. On the v45 read path bb6ca83 removes the first-contact withholds and the read fix the reach half's reads. 0 shows the devices alone";
    TodayRoundsScale = "today_rounds_scale", 1.0, Measured, "multiplies window B's responder finals and withholds when today's peers are booked; 1 is window B itself; the interval [0.91, 1.20] is the range of the other windows' finals per plane against B's (A, A' and C on Attestation, Key, route and occurrence; C's 43 occurrence finals are left out as unexplained), seat_calibration::INPUT_INTERVALS";
    StoreFloorNodes = "store_floor_nodes", 1_349.0, Model, "the store never holds fewer nodes than today's announced set: N_nodes = max(devices, floor)";
    PageRows = "page_rows", 1_024.0, Derived, "holdings drain page size (edge code trace, team-lead 2026-11-10)";
    // ---- round schedule --------------------------------------------------
    CadenceS = "cadence_s", 30.0, Derived, "bootstrap round cadence per plane per peer (edge #881)";
    SmallReadsPerCadence = "small_reads_per_cadence", 6.0, Derived, "about 6 small reads per device per cadence beside the drains (edge code trace)";
    SummariesPerRound = "summaries_per_round", 1.0, Derived, "one drain per completed responder round (edge code trace); with the Revocation and occurrence-revocation finals at 0 (window C) window B predicts 1,500 pages against 2,262 permits measured, so it no longer fits: see the permit residuals";
    AttRequiredQueries = "att_required_queries", 15.0, Derived, "required read of 10-20 counted queries per attestation round (midpoint)";
    RefusedRowsPerAttRound = "refused_rows_per_att_round", window_a::WITHHOLDS / 10.0, Measured, "2,078 rows withheld / 10 attestation rounds, window A (fit split); window B gives 226.3, the interval's other end";
    QueriesPerRefusedRow = "queries_per_refused_row", window_a::ATT_QUERIES / window_a::WITHHOLDS, Measured, "19,550 attestation queries / 2,078 withheld rows, window A (fit split); window B gives 19,754 / 2,263 = 8.73, the interval's other end";
    ComposerPerRefusedRow = "composer_per_refused_row", 1.86, Measured, "3,867 composer lookups / 2,078 withheld rows, window A (all returned 0 rows); kept for the calibration table only: composers are not a per-withheld-row cost (edge source read 2026-11-10 at v44.1.2 (withhold-rate)), so the ledger books them per Attestation round";
    ComposerPerAttRound = "composer_per_att_round", 386.7, Measured, "composer lookups per Attestation round, fitted on window A alone (3,867 / 10 rounds; the declared fit split, seat_calibration::FIT); window B (4,512 / 10 = 451.2) is held out and predicted at -14.3%; the two windows bound the interval [386.7, 451.2]; booked per round in both read sets, untouched by the read fix";
    FirstContactWithholdShare = "first_contact_withhold_share", 0.884, Derived, "share of today's withholds that are first-contact withholds, which bb6ca83 (8e46165: 3bffbdc) removes: ~2,000 of window B's 2,263 a minute (edge source read 2026-11-10 at v44.1.2 (withhold-rate)); applied on the v45 read path";
    ConsentPerRefusedRow = "consent_per_refused_row", window_a::CONSENT / window_a::WITHHOLDS, Measured, "2,104 live-consent-grant reads / 2,078 withheld rows, window A (fit split; no other window counts them); each read runs inside the page permit and takes none of its own (edge source read 2026-11-10 at v44.1.2 (withhold-rate))";
    GrantReadsPerAttListingAfterFix = "grant_reads_per_att_listing_after_fix", 5.0, Derived, "after the read fix (commit 3, 4897789, in 8e46165 as 7fa7105: the reach half runs before the gates that read the store) a refused row reads no grant: about one grant read per local key per Attestation listing, ~50 a minute at window B's 10 listings, from ~2,100 (edge source read 2026-11-10 at v44.1.2 (withhold-rate)); the quarantine consults fall by a similar count and are not in the seat figures (no counter books them)";
    RefusedReadsLeftAfterFix = "refused_reads_left_after_fix", 1.0, Model, "after the read fix, the share of withheld rows refused by the row half (may_receive), which still read at today's cost per row; the reach half reads nothing (edge source read 2026-11-10 at v44.1.2 (withhold-rate)). The share is not measured: 1 is the upper bound (every withheld row refused by the row half), 0 makes a withheld row free";
    RowsPerAttQuery = "rows_per_att_query", window_a::ATT_ROWS / window_a::ATT_QUERIES, Measured, "attestation queries return 147,410 / 19,550 = 7.54 rows each, window A (fit split; no other window counts rows)";
    StaleRoutesPerS = "stale_routes_per_s", 2.8, Measured, "seat-1 stale routes re-delivered per second";
    StaleRouteReads = "stale_route_reads", 2.0, Derived, "two store reads per stale route delivery (edge code trace)";
    HybridVerifyCpuS = "hybrid_verify_cpu_s", 0.0015, Model, "CPU-seconds per hybrid (Ed25519 + ML-DSA-65) signature verify; not profiled";
    SweepPermits = "sweep_permits", edge_971::ADVERTISE_SWEEP_PERMITS as f64, Derived, "two sweep permits per node (8e46165 bridge.rs DEFAULT_ADVERTISE_SWEEP_PERMITS :1128): a drain takes one per PAGE, a key-plane listing one per subject it reads (key_plane_listing :7770), a subject Pull answer one (subject_pull_answer :5339), a feed page one (:8357, :8455), a send-set read one (resolved_peer_set :11363); no withhold site takes one (edge source read 2026-11-10 at v44.1.2 (withhold-rate))";
    AttListingPermits = "att_listing_permits", 3.0, Derived, "one Attestation listing takes 2 to 4 permits however many rows it withholds (watermarked sweep, head and after-head pages, v44.1.2 bridge.rs:3979, :3998; edge source read 2026-11-10 at v44.1.2 (withhold-rate)); midpoint";
    RequiredRowsPermits = "required_rows_permits", 1.0, Derived, "the required-rows read of an Attestation round takes one permit (v44.1.2 bridge.rs:10966)";
    TdListingPermits = "td_listing_permits", 1.5, Model, "a route listing takes the same watermarked sweep (v44.1.2 bridge.rs:7374): 1 or 2 permits, midpoint; not traced per round";
    SendSetMemoTtlS = "send_set_memo_ttl_s", edge_971::CONSENT_SEND_SET_MEMO_TTL_S, Derived, "a consent send-set read takes one sweep permit and is memoized this long (v44.1.2 bridge.rs resolved_peer_set :9828; 8e46165 :11338, permit :11363, CONSENT_SEND_SET_MEMO_TTL :3150): at least one read per 10 s while Attestation rounds arrive that often, so >= 6 a minute in every window; ADDED 2026-11-11, it closes 6 a minute of each permit residual; invalidations add more (not counted at v44.1.2)";
    KeyMemoExpiryS = "key_memo_expiry_s", edge_971::MEMO_TTL_S, Derived, "MEMO_TTL 600 s on each of the four key planes (8e46165 key_plane_memo.rs :97). At v44.1.2 (the calibration windows) the TTL cleared the memo and the next listing re-read every subject in one burst under the lock; at 8e46165 it opens a new epoch (PlaneMemo::revalidate :462): the answers are served stale while they are re-read, at most KEY_PLANE_READ_BUDGET a listing, so every subject is still re-read once per TTL (reads per minute unchanged) but no listing holds the lock for more than K reads; lazy, so a plane nobody lists never refills";
    ReplyDrain = "reply_drain", 1.0, Model, "an initiator drains its index on each reply Summary (window A candidate; UNEXPLAINED until the counters split)";
    PermitResidualBPerMin = "permit_residual_b_per_min", 678.5, Unexplained, "window B: 2,262 sweep permits less 1,583.5 predicted (drains (10 + 11 + 10) x 42 + 11 x 18 = 1,500; listings 10 x (3 + 1) Attestation, 11 x 1.5 route, 21 key-plane probes, 6 send-set reads); no TTL refill fits it (a refill is thousands of permits or none); decomposed with its candidate causes in seat_calibration::residuals";
    PermitResidualAPrimePerMin = "permit_residual_a_prime_per_min", 4_697.0, Unexplained, "window A': 6,386 less 1,689 predicted (drains 1,602, listings 81, send-set reads 6); a Key or occurrence TTL refill in the window is the candidate (MODEL, fits A' and C, not proven)";
    PermitResidualCPerMin = "permit_residual_c_per_min", 4_401.0, Unexplained, "window C: 7,512 less 3,111 predicted (drains 2,988, listings 117, send-set reads 6); the 43 occurrence finals are themselves unexplained; a TTL refill is the candidate (MODEL, not proven)";
    // ---- after read minimisation (v45 plan) -------------------------------
    ProbePerS = "probe_per_s", 1.0 / edge_971::FEED_PROBE_SPACING_S, Derived, "probes per second per serve-cursor feed when no door marked it dirty: FEED_PROBE_SPACING 5 s (8e46165 store_feed.rs :86), one feed shared by the four key-plane memos and the holdings index, two cursors (Attestation, route), one permit a page (bridge.rs :8357, :8455); every admitted row marks the feed dirty, so the next ask drains again (the earlier 'one probe per second per index kind' was the v45 plan, not the code)";
    AuditDrainS = "audit_drain_s", edge_971::HOLDINGS_AUDIT_EVERY_S, Derived, "one full audit drain per 60 s per index kind (8e46165 holdings_index.rs DEFAULT_AUDIT_EVERY :222)";
    HoldingsBytesPerHash = "holdings_bytes_per_hash", 60.0, Derived, "v45 plan: 40-80 bytes per hash held in the shared holdings set";
    // ---- CPU --------------------------------------------------------------
    HashRefBytes = "hash_ref_bytes", 72.0, Model, "decoded bytes per index entry (64 hex chars + framing)";
    RowBytes = "row_bytes", 800.0, Derived, "43-103 MB decoded over 8.5k-14.4k calls per 30 s at 7.5 rows a call: 675-954 B a row (seat-1 profile)";
    CallOverheadBytes = "call_overhead_bytes", 400.0, Model, "fixed cost of one counted call in decoded-byte equivalents (a 0-row call still costs)";
    CpuPerByte = "cpu_per_byte_s", 1.05e-7, Fitted, "fitted so window B's predicted CPU equals cpu_window_b_cores; see calibrate() and seat_calibration::FIT";
    CpuWindowBCores = "cpu_window_b_cores", 0.652, Measured, "window B process CPU, cores (utime+stime over 60 s, the one window with CPU); the input cpu_per_byte_s is fitted to; interval in seat_calibration::INPUT_INTERVALS";
    RoundOverheadCpuS = "round_overhead_cpu_s", 0.000_2, Model, "frame handling per received Summary beside its reads";
    ServerRefreshCores = "server_refresh_cores", 0.10, Context, "server announced-relay refresh, ~1/min at p50 6.7 s, ~10% of a core, being fixed server-side; not an edge cost";
    SeatCores = "seat_cores", 4.0, Model, "cores the seat can use (seat-1 reached 171% in the roll hour, so at least 2)";
    ProxyCores = "proxy_cores", 2.0, Model, "cores a server-class proxy can use";
    CpuCeiling = "cpu_ceiling", 0.70, Model, "utilisation the sweep treats as the ceiling (cores and permits)";
    // ---- required-set Pull (v45 baseline shape) ---------------------------
    PullPlanes = "pull_planes", edge_971::BUNDLE_PULL_PLANES.len() as f64, Derived, "BUNDLE_PULL_PLANES: Key, IdentityOccurrence, TransportDestination, IdentityOccurrenceRevocation; the self and seat legs (8e46165 runtime.rs :1113, BundlePull::cycle :1550)";
    OwnerPlanes = "owner_planes", edge_971::BUNDLE_OWNER_PLANES.len() as f64, Derived, "BUNDLE_OWNER_PLANES: Key, IdentityOccurrence, IdentityOccurrenceRevocation, never the route plane; the owner leg, timer cycles only (8e46165 runtime.rs :1124, cycle :1550)";
    PullCycleS = "pull_cycle_s", edge_971::BUNDLE_PULL_INTERVAL_S, Derived, "BUNDLE_PULL_INTERVAL from_mins(10), each device draws 300-900 s, mean 600 s (8e46165 runtime.rs :1154, bundle_pull_delay :1641, jittered :1653)";
    FirstContactSpacingS = "first_contact_spacing_s", edge_971::BUNDLE_PULL_MIN_SPACING_S, Derived, "BUNDLE_PULL_MIN_SPACING: kicked cycles at least this far apart; first-contact retries drawn U[0.5, 1.5] of it, mean 30 s (8e46165 runtime.rs :1159, first_contact_delay :1648)";
    FirstContactTries = "first_contact_tries", edge_971::BUNDLE_FIRST_CONTACT_TRIES as f64, Derived, "BUNDLE_FIRST_CONTACT_TRIES: quick cycles before the jittered 600 s timer when a seat never attributes; the give-up is counted and warned once (8e46165 runtime.rs :1365, note_first_contact_gave_up :1665, spawn_bundle_pull :1683)";
    FirstContactCycles = "first_contact_cycles", 2.0, Model, "first-contact cycles a start costs when the seat already holds the key and route: the offering Key and TD answers of the first cycle attribute at the second (8e46165 runtime.rs cycle :1620); at most first_contact_tries";
    RuntimeStartsPerDeviceDay = "runtime_starts_per_device_day", 4.0, Model, "replication runtime starts per device per day; each is first contact with every seat (self and seat legs on Key, IO and TD; IOR held until attributed, 8e46165 runtime.rs :1574)";
    PullPlaneWritesPerDeviceDay = "pull_plane_writes_per_device_day", 10.0, Model, "kicked write cycles per device per day: own rows on a Pull plane (a route change on a network move dominates), merged 30 s apart, one self leg each; Attestation-plane writes kick nothing (8e46165 runtime.rs kick_wrote :1422)";
    HostKicksPerDeviceDay = "host_kicks_per_device_day", 0.0, Model, "host round_now kicks per device per day (self and seat legs on the four Pull planes); an idle phone makes none";
    ConsentPeerShare = "consent_peer_share", 0.0, Model, "share of devices whose host names the seat as a consent peer: those round on every bootstrap plane as on v44 (8e46165 runtime.rs BundlePull::host, set_peers :2637); the case that brings the whole-plane bytes back";
    OldPeerShare = "old_peer_share", 0.0, Model, "share of devices still on v44 that round with the seat on every bootstrap plane (old peers are kept on purpose, so production is mixed for a long time); with consent peers, the share that lists both Global planes";
    PullReadsPerLeg = "pull_reads_per_leg", 1.0, Model, "counted reads per Pull leg (one subject on one plane); unknown until the edge witness";
    PullRefsPerLeg = "pull_refs_per_leg", 2.0, Model, "refs returned per Pull leg when in sync";
    PullCallsPerCycle = "pull_calls_per_cycle", 10.0, MeasuredDebug, "store directory calls per device per 10-minute Pull cycle (contact-4413's idle-round witness at 26efbc4); about 22 reads/s at 1,349 devices; pull_reads_per_leg stays for first-contact, write and kick legs";
    PullPermitsPerCycle = "pull_permits_per_cycle", 8.0, MeasuredDebug, "sweep permits per UNCLAIMED device per 10-minute Pull cycle, 0 index pages (contact-4413 at 26efbc4, an ancestor of 8e46165; a count, so not build-dependent); a claimed device adds its owner legs (owner_leg_permit_share)";
    OwnerLegPermitShare = "owner_leg_permit_share", 1.0, Model, "share of owner legs the seat answers inside the entitled arm, taking a permit (8e46165 bridge.rs subject_pull_answer :5339; entitled by a mutual trust root, refused without one, CIRISEdge#979); 1 is the upper bound";
    PullReplyBytesPerCycle = "pull_reply_bytes_per_cycle", 562.0, MeasuredDebug, "reply bytes per device per 10-minute Pull cycle (contact-4413 at 26efbc4)";
    DrainPageKeyS = "drain_page_key_s", 2.63e-3, MeasuredDebug, "seconds (wall) per 1,024-row index page drained on a Key round, intercept ~0 (contact-4413 at 26efbc4); used only for the 'without the read fix (not released)' row";
    DrainPageIoS = "drain_page_io_s", 1.84e-3, MeasuredDebug, "the same on an occurrence round";
    DrainPageRevS = "drain_page_rev_s", 3.31e-3, MeasuredDebug, "the same on a Revocation round";
    DrainPageIorS = "drain_page_ior_s", 3.77e-3, MeasuredDebug, "the same on an occurrence-withdrawal round; a route (TD) round reads no index";
    ClaimedShare = "claimed_share", 0.5, Model, "share of bootstrap devices with an owner (adds the owner to the Pull subjects)";
    AttributionReadsPerCadence = "attribution_reads_per_cadence", 1.0, Derived, "attribution cache lasts 3 s against a 30 s cadence: one store read per device per round";
    // ---- bytes ------------------------------------------------------------
    RefBytes = "ref_bytes", 90.0, Derived, "~90 B per ref in a Summary (edge code; witness pending)";
    FrameOverheadBytes = "frame_overhead_bytes", 100.0, Model, "RNS header, transit seal and framing per frame";
    KeyRecordBytes = "key_record_bytes", 7_500.0, Derived, "7-8 KB per hybrid key record: ML-DSA-65 public key 1,952 B and signature 3,309 B (FIPS 204) plus Ed25519 and encoding";
    OccurrenceRecordBytes = "occurrence_record_bytes", 1_500.0, Model, "bytes per identity occurrence record";
    RouteRecordBytes = "route_record_bytes", 600.0, Model, "bytes per transport destination record";
    RevPlaneRefs = "rev_plane_refs", 5.0, Model, "refs in a revocation plane's offer: revocations the seat holds of any key in own ∪ cohort (8e46165 bridge.rs list_revocation_rows :13152); seat-1 holds 0 key:revocation rows, so few; MODEL until the #971 idle-cost test prints it";
    // ---- the four key planes (Key, occurrence, Revocation, occurrence withdrawal), apart from the read ledger --
    // Source: v44.1.2 (the windows) and 8e46165 bridge.rs `key_plane_listing` (:7664),
    // `refresh_key_plane_memo` (:7883), key_plane_memo.rs, key_plane_rows.rs. Each plane's tokio Mutex is held for the whole
    // listing, its probe and any subject reads; a subject Pull reads the store directly, outside it.
    KeySubjectsAtSeat = "key_subjects_at_seat", 8_211.0, Derived, "subjects a Key-plane listing walks at seat-1: self_own_subjects -> the host's KindPublishSelector (8e46165 bridge.rs :7465; edge source read 2026-11-10 at v44.1.2 (withhold-rate)); grows with the fleet by key_publish_set_growth";
    IoSubjectsAtSeat = "io_subjects_at_seat", 5_650.0, Derived, "subjects an occurrence-plane listing walks at seat-1, ~5.6k to 5.7k, the same selector (edge source read 2026-11-10 at v44.1.2 (withhold-rate); 5,652 in 8e46165 key_plane_memo.rs :296 docs); grows with the fleet by key_publish_set_growth";
    GlobalSubjectsAtSeat = "global_subjects_at_seat", 8_698.0, Derived, "subjects a Revocation or occurrence-withdrawal listing walks: subjects_for_projection(Global) = self_provider ∪ cohort (8e46165 bridge.rs :7478, :7492), which on a canonical is every key it holds (list_revocation_rows :13152 'Global is every key on a canonical'; key_plane_memo.rs header: 7,226 on the canonical at #930); seat-1 holds 8,698 keys (window A), the Key publish set is 8,211, so the interval is [8,211, 8,698]. CORRECTED: the twin had 30 ('tens', a MODEL pick) until 2026-11-11; grows with the fleet by key_publish_set_growth";
    KeyPublishSetGrowth = "key_publish_set_growth", 1.0, Frontier, "how the host's publish sets (the server's key_publish_set, which self_provider and every KindPublishSelector answer from) grow with the fleet: subjects(N) = subjects_at_seat x (1 + g (N / announced_nodes - 1)); g = 1 grows with announced nodes like the store, g = 0 stays at today's size. Server-side, not read by the twin: an explicit scenario axis, swept 0 to 1 in seat_calibration::knee_by_growth";
    ListingProbeReads = "listing_probe_reads", 1.0, Derived, "as shipped at v44.1.2 (the calibration windows): change-probe pages per key-plane listing when under 500 rows landed since the cursor, one permit each (refresh_key_plane_memo, PROBE_PAGE 500, v44.1.2 bridge.rs:7233); at 8e46165 the probe is the shared feed (probe_per_s)";
    KeyPlaneWalkBaseS = "key_plane_walk_base_s", 20.0e-6, Model, "a in a + b x subjects: fixed seconds per key-plane listing (the call, the empty offer)";
    KeyPlaneWalkPerSubjectS = "key_plane_walk_per_subject_s", 0.55e-6, Derived, "b in a + b x subjects: a warm Key listing at 8,211 subjects takes ~3 to 6 ms (edge source read 2026-11-10 at v44.1.2 (withhold-rate)), 4.5 ms / 8,211; taken as the same per subject on every key plane (MODEL); at 8e46165 it is what a listing-cache MISS costs (the classify walk, bridge.rs key_plane_listing :7664)";
    StoreReadS = "store_read_s", 2.0e-3, Derived, "~2 ms per subject read made under a plane's lock, from which a Key refill stalls ~16 s and an occurrence refill ~11 to 22 s (edge source read 2026-11-10 at v44.1.2 (withhold-rate))";
    ProbeReadS = "probe_read_s", 0.5e-3, Model, "seconds one change-probe page holds the plane lock (usually an empty page); not measured";
    KeyQueriesPerSubject = "key_queries_per_subject", 1.0, Derived, "store reads to refill one Key-plane subject: its key:record rows (key_plane_rows::key_record_rows); 8,211 per refill at seat-1";
    IoQueriesPerSubject = "io_queries_per_subject", 1.5, Derived, "store reads to refill one occurrence-plane subject: 1 to 2 (an 11 to 22 s refill at ~2 ms over ~5.6k subjects, edge source read 2026-11-10 at v44.1.2 (withhold-rate)); mostly not attestation_query, so a refill moves permits and not that counter";
    RevQueriesPerSubject = "rev_queries_per_subject", 1.0, Model, "store queries to refill one Revocation-plane subject: revocations_for, then get_attestation per revocation held, and revocations are few (8e46165 key_plane_rows.rs revocation_rows_of :153)";
    IorQueriesPerSubject = "ior_queries_per_subject", 5.0, Model, "store queries to refill one occurrence-withdrawal subject: 4 to 6 (8e46165 key_plane_rows.rs occurrence_withdrawals_binding :260, withhold-rate's reading)";
    RevPlaneClearsPerHour = "rev_plane_clears_per_hour", 0.0, Measured, "key:revocation rows landing per hour: each drops the Revocation memo whole (Dirt::Plane, 8e46165 key_plane_memo.rs dirt_of :261); seat-1 has held 0 key:revocation:v1 rows ever (server session, 2026-11-09); STRESS_PLANE_CLEARS_PER_HOUR is the named stress value";
    IorPlaneClearsPerHour = "ior_plane_clears_per_hour", 0.0, Measured, "occurrence-revocation carrier rows landing per hour: each drops the occurrence-withdrawal memo whole (Dirt::Plane, 8e46165 key_plane_memo.rs dirt_of :267); seat-1 holds 9 withdraws rows in all, none since this boot (server session, 2026-11-09)";
    RevNonEmptySubjects = "rev_non_empty_subjects", 0.0, Measured, "Revocation-plane subjects with any row at seat-1: 0 (no key:revocation:v1 row ever); scaled per announced node; what a rebuild from the rows that exist reads";
    IorNonEmptySubjects = "ior_non_empty_subjects", 9.0, Measured, "occurrence-withdrawal subjects with any row at seat-1: at most 9 (9 withdraws rows in all); scaled per announced node";
    RowScanReads = "row_scan_reads", 1.0, Model, "from-rows rebuild only (not built; whether the persist surface lists a plane's rows by type is withhold-rate's question): reads to list the rows that exist on one Global plane";
    WarmListingS = "warm_listing_s", 0.2e-3, Model, "BUILT (8e46165, on by default: bridge.rs BridgeConfig::key_plane_listing_cache :1004, default :1195): a listing every subject of which was answered fresh is kept per plane generation and served for an equal subject set (key_plane_memo.rs cached_listing :550) under the plane lock (bridge.rs :7674). A hit still compares the whole subject set (HashSet equality), so it scales per subject: ~0.2 ms at 8,211 (edge source read 2026-11-10 at v44.1.2 (withhold-rate)); the timing is MODEL, not measured";
    SubjectSetBuildPerSubjectS = "subject_set_build_per_subject_s", 0.1e-6, Model, "CPU seconds per subject to build a listing's subject set before the lock (8e46165 bridge.rs list_revocation_rows :13152: subjects_for_projection(Global) calls self_provider and cohort and collects a HashSet<String>); paid on every listing, cache hit or not; not measured";
    KeyRecordWritesPerDeviceDay = "key_record_writes_per_device_day", 0.05, Model, "key:record rows landing at the seat per device per day (enrolment, re-publish, rotation): each dirties that subject on the Key and Revocation memos (8e46165 key_plane_memo.rs dirt_of :240), so the generation moves and the next listing on each misses the listing cache and walks the whole set; not measured";
    ListingBudgetK = "listing_budget_k", edge_971::KEY_PLANE_READ_BUDGET as f64, Derived, "BUILT (8e46165 key_plane_memo.rs KEY_PLANE_READ_BUDGET :296, on by default bridge.rs :1221): a key-plane listing reads at most K subjects, every cause together, the host's own keys outside it; a stale answer past the budget is served as it stands, an absent one is left out (a missed offer); so a refill spreads over listings with the reads per minute unchanged and a stall capped at ~K reads";
    RestartArrivalS = "restart_arrival_s", 30.0, Model, "cold-restart scenario only: after a seat restart every peer's first round on each plane arrives within this window (first contact at once, retries U[0.5, 1.5] x 30 s)";
    SeatOwnRoutes = "seat_own_routes", 4.0, Model, "routes a seat offers a stranger: its own";
    UnownedRouteShare = "unowned_route_share", 0.02, Provisional, "share of routes that are unowned and so served to a stranger";
    StretchFactor = "stretch_factor", 10.0, Model, "stretched idle cadence multiplier (not built)";
    DigestBytes = "digest_bytes", 64.0, Model, "an in-sync round as one small digest frame (not built)";
    PhoneAwakeShare = "phone_awake_share", 0.85, Model, "1 - micro mobile_offline_share: a suspended phone runs no rounds";
    LiveLinkCap = "live_link_cap", 1_024.0, Provisional, "live-link cap with FIFO eviction (CIRISEdge#508, 2026-10); not re-read at 8e46165";
    // ---- offload: referral + proxy (MODEL until edge confirms) -----------
    ProxyStoreShare = "proxy_store_share", 1.0, Model, "a proxy's federation-tier store as a share of the seat's";
    NewRowsPerDeviceDay = "new_rows_per_device_day", 20.0, Model, "new federation-tier rows a device's subjects produce per day";
    ProxyMissShare = "proxy_miss_share", 1.0, Model, "share of new rows a proxy lacks and pulls from the seat (one pull per row per proxy)";
    DeviceStartsPerDay = "device_starts_per_day", 60.0, Model, "starts per device per day when it returns to the seat at each start";
    NewDeviceShareDay = "new_device_share_day", 0.01, Model, "share of the fleet that is new each day (first contact under durable homing)";
    ReferralLookupReads = "referral_lookup_reads", 3.0, Model, "counted reads to pick up to three relay-offering nodes with a free shaped slot";
    RootedShare = "rooted_share", 0.0, Model, "share of offloaded devices that share a trust root with their proxy (CC 5.3.4)";
    BindingPullsPerCadence = "binding_pulls_per_cadence", 1.0, Model, "owner-binding pulls a claimed unrooted offloaded device keeps on the seat per cadence";
    // ---- the reach rule (6244795, in 8e46165) --------------------------------
    ReachUnownedShare = "reach_unowned_share", 0.5, Model, "share of the identity-plane publish set with no live owner-binding at the seat, which the reach rule now reads for (8e46165 bridge.rs identity_row_reach :13985 -> unowned_reach): at most 1 - announced_nodes / io_subjects_at_seat = 0.76 (an announced node is owned), at least announced_owners / io_subjects_at_seat = 0.24 (an identity key has no owner); interval [0.24, 0.76]; not measured";
    ReachReadsPerUnownedSubject = "reach_reads_per_unowned_subject", 1.5, Model, "store reads one reach evaluation of an unowned subject adds (6244795): list_identity_occurrences_by_occurrence_key always (1); when another identity holds it, lookup_public_key (1) and that identity's own set, nodes_owned_by + list_identity_occurrences_active (2, memoized per identity for OWNER_BINDING_MEMO_TTL and shared by its nodes); interval [1, 2]";
    AnnounceMemoClearsPerHour = "announce_memo_clears_per_hour", 6.0, Model, "whole clears per hour of the seat's reach memo besides its 30 s TTL: every ownership event (invalidate_owner_memo :13446) and every host local-write door (invalidate_send_set_memos :13621) drops it, so the next identity-plane listing re-reads every subject's reach; server-side rate, not measured; interval [0, 60]";
    SeatInitiatorDeviceShare = "seat_initiator_device_share", 0.0, Frontier, "share of devices the server's host names as the seat's own replication peers, so the seat INITIATES a round to each on the five bootstrap planes every cadence (it lists to build its Summary and receives a reply); a node initiates only to host-named peers (8e46165 runtime.rs set_peers :2637), bundle seats (bundle_bootstrap_peers :1162) and homed nodes (HomedPeers::register :1243), so this is the server's choice and not read; 0 unless the server names devices";
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
    /// every key plane: consent peers and old (v44) peers.
    #[must_use]
    pub fn lists_both_share(&self) -> f64 {
        let c = self.get(C::ConsentPeerShare).clamp(0.0, 1.0);
        let o = self.get(C::OldPeerShare).clamp(0.0, 1.0);
        1.0 - (1.0 - c) * (1.0 - o)
    }

    /// Subjects with any row on a key plane when `nodes` nodes are announced: on the two
    /// Global planes the measured non-empty count, on Key and occurrence every subject.
    #[must_use]
    pub fn non_empty_subjects(&self, plane: Plane, nodes: f64) -> f64 {
        let per = |x: f64| x / self.get(C::AnnouncedNodes) * nodes;
        match plane {
            Plane::Revocation => per(self.get(C::RevNonEmptySubjects)),
            Plane::IdentityOccurrenceRevocation => per(self.get(C::IorNonEmptySubjects)),
            _ => self.plane_subjects(plane, nodes),
        }
    }

    /// How the host's publish sets scale at `nodes` announced nodes against today's:
    /// `1 + g (nodes / announced_nodes - 1)` with g = `key_publish_set_growth` (FRONTIER, the
    /// server's choice), never below 0.
    #[must_use]
    pub fn publish_set_scale(&self, nodes: f64) -> f64 {
        let g = self.get(C::KeyPublishSetGrowth);
        (1.0 + g * (nodes / self.get(C::AnnouncedNodes) - 1.0)).max(0.0)
    }

    /// Subjects a key-plane listing walks when `nodes` nodes are announced. Key and occurrence
    /// list the host's publish selector; the two Global planes list own ∪ cohort, which on a
    /// seat is every key it holds (8e46165 bridge.rs :7492, :13152). All three are host-provided
    /// sets and grow by [`Self::publish_set_scale`]. 0 for a plane with no memo.
    #[must_use]
    pub fn plane_subjects(&self, plane: Plane, nodes: f64) -> f64 {
        let k = self.publish_set_scale(nodes);
        match plane {
            Plane::Key => self.get(C::KeySubjectsAtSeat) * k,
            Plane::IdentityOccurrence => self.get(C::IoSubjectsAtSeat) * k,
            Plane::Revocation | Plane::IdentityOccurrenceRevocation => self.get(C::GlobalSubjectsAtSeat) * k,
            Plane::TransportDestination | Plane::Attestation => 0.0,
        }
    }

    /// Store reads to refill one subject of a key plane.
    #[must_use]
    pub fn queries_per_subject(&self, plane: Plane) -> f64 {
        match plane {
            Plane::Key => self.get(C::KeyQueriesPerSubject),
            Plane::IdentityOccurrence => self.get(C::IoQueriesPerSubject),
            Plane::Revocation => self.get(C::RevQueriesPerSubject),
            Plane::IdentityOccurrenceRevocation => self.get(C::IorQueriesPerSubject),
            Plane::TransportDestination | Plane::Attestation => 0.0,
        }
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
    /// v45 baseline (8e46165 runtime.rs BundlePull): Key, IdentityOccurrence, TransportDestination and
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

/// How a key plane's memo is refilled after its TTL (scenario flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoRebuild {
    /// As shipped: cleared whole every `key_memo_expiry_s`, refilled by the next listing under
    /// the lock.
    #[default]
    Whole,
    /// Rebuilt from the rows that exist: one listing of the plane's rows by type
    /// (`row_scan_reads`) and the queries for the non-empty subjects only (not built; the
    /// persist surface is unconfirmed). On Key and occurrence every subject has rows, so it
    /// reads as many as `Whole`.
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

/// The four key planes, each with its own memo and its own lock (`key_plane_listing`).
pub const KEY_PLANES: [Plane; 4] =
    [Plane::Key, Plane::IdentityOccurrence, Plane::Revocation, Plane::IdentityOccurrenceRevocation];

/// The two Global key planes (own ∪ cohort, small on a seat).
pub const GLOBAL_PLANES: [Plane; 2] = [Plane::Revocation, Plane::IdentityOccurrenceRevocation];

/// The v45 candidate changes to the key-plane listing, as scenario flags (MODEL until built).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListingFix {
    /// (b) a listing cached per plane generation: warm, `warm_listing_s` per 8,211 subjects
    /// and no store read.
    pub cache: bool,
    /// A per-listing read budget of `listing_budget_k` with stale-while-revalidate: the lock
    /// is held for at most K reads; the reads per minute are unchanged.
    pub budget: bool,
}

/// One key plane's lock, walk and refill (MODEL until v45's sweep counters print them).
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyPlaneLoad {
    /// Listings per second (one per responder round on the plane).
    pub listings_per_s: f64,
    /// Subjects each listing walks.
    pub subjects: f64,
    /// Seconds one listing holds the lock outside a refill (walk plus its probe).
    pub hold_s: f64,
    /// Probe page reads per second (one permit each).
    pub probe_reads_per_s: f64,
    /// TTL and plane-clear refills per second (lazy: none without a listing).
    pub rebuilds_per_s: f64,
    /// Refill store reads per second (one permit each).
    pub refill_reads_per_s: f64,
    /// Reads in one refill: the burst a 60 s window sees whole or not at all.
    pub burst_reads: f64,
    /// The longest the lock is held at once by a refill, seconds (K reads with the budget).
    pub stall_s: f64,
    /// Rounds that arrive during one stall.
    pub rounds_per_stall: f64,
    /// Lock utilisation: listings x hold + refill reads x read time. Cores do not help.
    pub rho_lock: f64,
    /// CPU in cores (walk, probes, refills).
    pub cpu: f64,
    /// Of which the store reads (probes, refills).
    pub cpu_reads: f64,
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
    /// Sweep permits the Pulls take per minute.
    pub pull_permits_per_min: f64,
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
    /// Subjects each key plane's listing walks, by [`KEY_PLANES`] order.
    pub n_subjects: [f64; 4],
    /// Subjects with rows per key plane, by [`KEY_PLANES`] order.
    pub n_non_empty: [f64; 4],
    /// Book the window C permit residual (UNEXPLAINED) as a constant permit load.
    pub book_permit_residual: bool,
    /// The v45 candidate listing changes.
    pub listing_fix: ListingFix,
    /// How the key-plane memos are refilled.
    pub memo_rebuild: MemoRebuild,
    /// Book the TTL refills (a measured 60 s window holds a whole refill or none, so the
    /// calibration windows leave them out and report them apart).
    pub refills_booked: bool,
    /// The reach rule of 6244795 is in the code (8e46165; not in the v44.1.2 windows).
    pub reach_rule: bool,
    /// Reach reads per minute added by Pull legs (the device's own subject and its owner's).
    pub pull_reach_reads_per_min: f64,
    /// key:record rows landing per minute (each dirties the Key and Revocation memos).
    pub key_record_writes_per_min: f64,
}

impl RoundMix {
    /// The store sizes and key-plane subjects for `nodes` announced nodes, scaled by `store`.
    #[must_use]
    pub fn sized(p: &CostParams, nodes: f64, store: f64) -> Self {
        Self {
            n_att: p.att_rows(nodes) * store,
            n_td: p.td_rows(nodes) * store,
            n_keys: p.get(C::SeatKeyRows) / p.get(C::AnnouncedNodes) * nodes * store,
            n_subjects: KEY_PLANES.map(|k| p.plane_subjects(k, nodes) * store),
            n_non_empty: KEY_PLANES.map(|k| p.non_empty_subjects(k, nodes) * store),
            ..Self::default()
        }
    }
}

/// What one node pays per second.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    /// Summaries received per second.
    pub summaries_per_s: f64,
    /// Index pages read per second by holdings drains (uncounted by the store counter).
    pub index_pages_per_s: f64,
    /// Sweep permit acquires per second: drain pages, listing pages, key-plane probes and
    /// subject reads, subject Pulls.
    pub permits_per_s: f64,
    /// Of which listing permits (Attestation and route sweeps, required rows) per second.
    pub listing_permits_per_s: f64,
    /// Of which key-plane permits (probe pages and refill reads) per second.
    pub key_plane_permits_per_s: f64,
    /// Of which subject-Pull permits per second.
    pub pull_permits_per_s: f64,
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
    /// The four key planes, by [`KEY_PLANES`] order.
    pub key_planes: [KeyPlaneLoad; 4],
    /// Key-plane listings per second, all four planes. Not part of the read ledger.
    pub listings_per_s: f64,
    /// Probe reads per second on the key planes.
    pub probe_reads_per_s: f64,
    /// TTL and plane-clear refill reads per second on the key planes.
    pub ttl_reads_per_s: f64,
    /// CPU in cores on the key planes (walk, probes, refills).
    pub cpu_key_planes: f64,
    /// Of which the store reads (probes and refills), the part that scales with CPU per byte.
    pub cpu_key_reads: f64,
    /// The busiest key plane's lock utilisation.
    pub rho_lock: f64,
    /// The busiest key plane (index into [`KEY_PLANES`]).
    pub lock_plane: usize,
    /// Of `listing_permits_per_s`, consent send-set reads (one permit each, a 10 s memo).
    pub send_set_permits_per_s: f64,
    /// Reach-rule reads per second (6244795): listing walks and Pull legs. In `other_reads_per_s`.
    pub reach_reads_per_s: f64,
    /// Feed probe and drain reads per second after read minimisation. In `other_reads_per_s`.
    pub feed_reads_per_s: f64,
    /// CPU in cores building key-plane subject sets before each listing (outside the lock).
    pub cpu_set_build: f64,
}

impl Ledger {
    /// All counted store calls per second (what the store counter shows).
    #[must_use]
    pub fn counted_per_s(&self) -> f64 {
        self.att_queries_per_s
            + self.composer_per_s
            + self.consent_per_s
            + self.other_reads_per_s
            + self.probe_reads_per_s
            + self.ttl_reads_per_s
    }

    /// Edge CPU in cores.
    #[must_use]
    pub fn cpu_edge(&self) -> f64 {
        self.cpu_index + self.cpu_query + self.cpu_verify + self.cpu_frames + self.cpu_key_planes + self.cpu_set_build
    }

    /// Total CPU in cores.
    #[must_use]
    pub fn cpu_total(&self) -> f64 {
        self.cpu_edge() + self.cpu_context
    }

    /// The busiest key plane.
    #[must_use]
    pub fn busiest_plane(&self) -> Plane {
        KEY_PLANES[self.lock_plane]
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

    // No withhold site takes a permit (edge source read 2026-11-10 at v44.1.2). The measured
    // excess over the prediction stays an UNEXPLAINED residual, booked only when asked.
    let excess = if mix.book_permit_residual { p.get(C::PermitResidualCPerMin) / 60.0 } else { 0.0 };
    let att_rounds = mix.rounds_per_min[Plane::Attestation as usize] / 60.0;
    // The consent send-set read: one permit per miss of a 10 s memo, so at most one per
    // Attestation listing and at least one per 10 s while they arrive (resolved_peer_set).
    l.send_set_permits_per_s = att_rounds.min(1.0 / p.get(C::SendSetMemoTtlS));
    l.listing_permits_per_s = att_rounds * (p.get(C::AttListingPermits) + p.get(C::RequiredRowsPermits))
        + mix.rounds_per_min[Plane::TransportDestination as usize] / 60.0 * p.get(C::TdListingPermits)
        + l.send_set_permits_per_s;
    l.pull_permits_per_s = mix.pull_permits_per_min / 60.0;
    let refusing = mix.refusing_att_rounds_per_min / 60.0;
    let refused = refusing * mix.refused_rows_per_round;
    let required = att_rounds * p.get(C::AttRequiredQueries);
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
            l.composer_per_s = att_rounds * p.get(C::ComposerPerAttRound);
            l.consent_per_s = refused * p.get(C::ConsentPerRefusedRow);
            l.verifies_per_s = mix.stale_routes_per_s + mix.ingest_rows_per_min / 60.0;
            other += mix.stale_routes_per_s * p.get(C::StaleRouteReads);
        }
        ReadSet::AfterReadMin => {
            // One audit drain per period per index kind, constant in peers.
            l.index_pages_per_s = (pa + pt) / p.get(C::AuditDrainS);
            // The shared serve-cursor feed (store_feed.rs): each of its two cursors probes at
            // most once per FEED_PROBE_SPACING when no door marked it dirty, and once more after
            // each admitted row (a dirt mark), never more often than asks arrive. One permit a
            // page. The four key-plane memos take their delta from this feed.
            let asks = l.summaries_per_s + mix.pull_requests_per_min / 60.0;
            if asks > 0.0 {
                let feed = (2.0 * p.get(C::ProbePerS) + mix.ingest_rows_per_min / 60.0).min(asks.max(2.0 * p.get(C::ProbePerS)));
                l.feed_reads_per_s = feed;
                other += feed;
                l.listing_permits_per_s += feed;
            }
            // bb6ca83 (3bffbdc in 8e46165) removes the first-contact withholds. The read fix: a row the reach half
            // refuses reads nothing and no refused row reads a grant (about one per local key
            // per listing instead); the row half's refusals still read at today's cost per row,
            // `refused_reads_left_after_fix` of them (MODEL, not measured).
            let left = p.get(C::RefusedReadsLeftAfterFix).clamp(0.0, 1.0);
            let refused = refused * (1.0 - p.get(C::FirstContactWithholdShare).clamp(0.0, 1.0));
            l.att_queries_per_s = required + refused * p.get(C::QueriesPerRefusedRow) * left;
            l.composer_per_s = att_rounds * p.get(C::ComposerPerAttRound);
            l.consent_per_s = if refusing > 0.0 { att_rounds * p.get(C::GrantReadsPerAttListingAfterFix) } else { 0.0 };
            l.verifies_per_s = mix.ingest_rows_per_min / 60.0;
            l.holdings_bytes = (mix.n_att + mix.n_td) * p.get(C::HoldingsBytesPerHash);
        }
    }
    // The reach rule (6244795, 8e46165 bridge.rs identity_row_reach -> unowned_reach): an
    // identity-plane listing re-evaluates every subject of its publish set whose 30 s memo has
    // lapsed or been cleared; an unowned subject now costs reads. Pull legs add the device's
    // own subject and its owner's (booked in `add_devices`).
    if mix.reach_rule {
        // Occurrence and route listings each walk the publish set's reach (per peer,
        // list_identity_plane_for_peer); the memo is shared, so the rate is capped by its TTL.
        let listings = (mix.rounds_per_min[Plane::IdentityOccurrence as usize]
            + mix.rounds_per_min[Plane::TransportDestination as usize])
            / 60.0;
        let unowned = p.get(C::ReachUnownedShare).clamp(0.0, 1.0) * mix.n_subjects[1];
        let recompute = listings.min(1.0 / edge_971::OWNER_BINDING_MEMO_TTL_S + p.get(C::AnnounceMemoClearsPerHour) / 3_600.0);
        l.reach_reads_per_s = recompute * unowned * p.get(C::ReachReadsPerUnownedSubject) + mix.pull_reach_reads_per_min / 60.0;
        other += l.reach_reads_per_s;
    }
    l.other_reads_per_s = other;
    // The four key planes, apart from the read ledger: each listing holds its plane's lock
    // for its whole walk and probe, and a TTL refill reads every subject under the same lock,
    // one permit per read (8e46165 bridge.rs :7664, :7883; key_plane_memo.rs). A plane's
    // lock is one server whatever the cores.
    let mut cpu_k = 0.0;
    let mut key_permits = 0.0;
    for (ki, plane) in KEY_PLANES.into_iter().enumerate() {
        let g = key_plane(p, mix, read, ki, plane);
        l.listings_per_s += g.listings_per_s;
        l.probe_reads_per_s += g.probe_reads_per_s;
        l.ttl_reads_per_s += g.refill_reads_per_s;
        key_permits += g.probe_reads_per_s + g.refill_reads_per_s;
        if g.rho_lock > l.rho_lock {
            l.rho_lock = g.rho_lock;
            l.lock_plane = ki;
        }
        cpu_k += g.cpu;
        l.cpu_key_reads += g.cpu_reads;
        // Each listing builds its subject set before it takes the lock.
        l.cpu_set_build += g.listings_per_s * g.subjects * p.get(C::SubjectSetBuildPerSubjectS);
        l.key_planes[ki] = g;
    }
    l.key_plane_permits_per_s = key_permits;
    l.cpu_key_planes = cpu_k;
    l.permits_per_s = l.index_pages_per_s + l.listing_permits_per_s + key_permits + l.pull_permits_per_s + excess;
    l.rows_per_s = l.att_queries_per_s * p.get(C::RowsPerAttQuery) + l.consent_per_s + other;
    let calls = l.counted_per_s() - l.probe_reads_per_s - l.ttl_reads_per_s;
    // (the reach reads and feed probes are counted calls in `other`; key-plane reads apart)
    let c = p.get(C::CpuPerByte);
    l.cpu_index = (l.index_pages_per_s + excess) * p.get(C::PageRows) * p.get(C::HashRefBytes) * c;
    l.cpu_query = (l.rows_per_s * p.get(C::RowBytes) + calls * p.get(C::CallOverheadBytes)) * c;
    l.cpu_verify = l.verifies_per_s * p.get(C::HybridVerifyCpuS);
    l.cpu_frames = (l.summaries_per_s + mix.pull_requests_per_min / 60.0) * p.get(C::RoundOverheadCpuS);
    l.cpu_context = mix.context_cores;
    l
}

/// One key plane's listings, lock and refills at this mix (`ki` indexes [`KEY_PLANES`]).
///
/// As shipped at v44.1.2 every listing walks the memo (a + b x subjects) and probes once; a TTL
/// refill reads every subject in one burst under the lock. At 8e46165 (the listing cache and
/// the read budget, both on by default): a listing whose subject set and memo generation are
/// unchanged is a cache HIT, which compares the set (`warm_listing_s`, per subject) and reads
/// nothing; a MISS walks the memo and reads at most K subjects. Misses come from the TTL
/// revalidation (every subject re-read once per TTL, K per listing, so ⌈subjects x q / K⌉
/// missed listings an epoch) and from key:record rows landing (each moves the generation of the
/// Key and Revocation memos). Every read is made under the plane's lock with one permit.
#[must_use]
pub fn key_plane(p: &CostParams, mix: &RoundMix, read: ReadSet, ki: usize, plane: Plane) -> KeyPlaneLoad {
    let rounds = mix.rounds_per_min[plane as usize] / 60.0;
    let listings = rounds;
    let subjects = mix.n_subjects[ki];
    let fix = mix.listing_fix;
    let t = p.get(C::StoreReadS);
    let walk_s = p.get(C::KeyPlaneWalkBaseS) + p.get(C::KeyPlaneWalkPerSubjectS) * subjects;
    let hit_s = p.get(C::WarmListingS) * subjects / p.get(C::KeySubjectsAtSeat);
    let clears_h = match plane {
        Plane::Revocation => p.get(C::RevPlaneClearsPerHour),
        Plane::IdentityOccurrenceRevocation => p.get(C::IorPlaneClearsPerHour),
        _ => 0.0,
    };
    let q = p.queries_per_subject(plane);
    let ttl = match mix.memo_rebuild {
        MemoRebuild::Removed => 0.0,
        MemoRebuild::Whole | MemoRebuild::FromRows => 1.0 / p.get(C::KeyMemoExpiryS),
    };
    // Lazy: a refill happens on the next listing, so never more refills than listings.
    let rebuilds = if mix.refills_booked { (ttl + clears_h / 3_600.0).min(listings) } else { 0.0 };
    let non_empty = mix.n_non_empty[ki].min(subjects);
    let per_rebuild = match mix.memo_rebuild {
        MemoRebuild::FromRows => p.get(C::RowScanReads) + non_empty * q,
        MemoRebuild::Whole | MemoRebuild::Removed => subjects * q,
    };
    // key:record rows dirty one subject each on the Key and Revocation memos (dirt_of).
    let dirt_per_s = match plane {
        Plane::Key | Plane::Revocation if listings > 0.0 => mix.key_record_writes_per_min / 60.0,
        _ => 0.0,
    };
    let refill_reads = rebuilds * per_rebuild + dirt_per_s * q;
    let k = p.get(C::ListingBudgetK).max(1.0);
    // The budget caps one listing's reads; stale-while-revalidate serves the rest.
    let held_reads = if fix.budget { per_rebuild.min(k) } else { per_rebuild };
    let stall_s = held_reads * t;
    // Probes: one page per listing as shipped at v44.1.2; after read minimisation the memos take
    // their delta from the shared feed, booked once in the ledger.
    let probes = if fix.cache {
        0.0
    } else {
        match read {
            ReadSet::AsShipped => listings * p.get(C::ListingProbeReads),
            ReadSet::AfterReadMin => 0.0,
        }
    };
    let probe_s = if listings > 0.0 { probes / listings * p.get(C::ProbeReadS) } else { 0.0 };
    // Listings that walk the memo: all of them without the cache; with it, the misses.
    let misses = if fix.cache {
        let refill_listings = rebuilds * if fix.budget { (per_rebuild / k).ceil().max(1.0) } else { 1.0 };
        (refill_listings + dirt_per_s).min(listings)
    } else {
        listings
    };
    let hits = listings - misses;
    let walk_total_s = misses * walk_s + hits * hit_s;
    let hold_s = if listings > 0.0 { walk_total_s / listings + probe_s } else { walk_s + probe_s };
    let c = p.get(C::CpuPerByte);
    let per_read_cpu = (p.get(C::RowBytes) + p.get(C::CallOverheadBytes)) * c;
    KeyPlaneLoad {
        listings_per_s: listings,
        subjects,
        hold_s,
        probe_reads_per_s: probes,
        rebuilds_per_s: rebuilds,
        refill_reads_per_s: refill_reads,
        burst_reads: per_rebuild,
        stall_s: if rebuilds > 0.0 { stall_s } else { 0.0 },
        rounds_per_stall: if rebuilds > 0.0 { rounds * stall_s } else { 0.0 },
        // every refill read is made under the plane's lock, budget or not
        rho_lock: listings * hold_s + refill_reads * t,
        cpu: walk_total_s + (probes + refill_reads) * per_read_cpu,
        cpu_reads: (probes + refill_reads) * per_read_cpu,
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
    /// The busiest key plane lock's utilisation (one server per plane; MODEL).
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
    // Each permit is held for its own read: a page for a drain or listing page, a probe page,
    // one subject read for a refill read or a subject Pull.
    let pages = l.permits_per_s - l.key_plane_permits_per_s - l.pull_permits_per_s;
    let held = pages * p.page_cpu_s()
        + l.probe_reads_per_s * p.get(C::ProbeReadS)
        + (l.ttl_reads_per_s + l.pull_permits_per_s) * p.get(C::StoreReadS);
    let rho_permit = held / p.get(C::SweepPermits);
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
    /// Revocation and occurrence-revocation responder finals (MEASURED 0 in all four windows:
    /// neither plane showed a non-zero completed_responder_final delta).
    pub const REV_ROUNDS: f64 = 0.0;
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
    /// Revocation and occurrence-revocation responder finals (MEASURED 0, as in every window).
    pub const REV_ROUNDS: f64 = 0.0;
}

/// Lifetime totals at one scrape (MEASURED, seat-1; the same scrape as `busy_hour`, 64 min
/// after boot: its permit count is this one).
pub mod lifetime {
    /// Sweep permits since boot.
    pub const PERMITS: f64 = 273_750.0;
    /// recipient_not_in_send_set withholds since boot.
    pub const WITHHOLDS: f64 = 272_482.0;
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
    let mut m = RoundMix::sized(p, p.get(C::AnnouncedNodes), 1.0);
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
    /// Predicted permits per minute from the responder finals (drains one per page, listing
    /// pages, key-plane probes; no TTL refill), B, A', C (Global finals MEASURED 0 throughout).
    pub permits_pred: [f64; 3],
    /// Of which drain pages: B, A', C.
    pub drain_pred: [f64; 3],
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
    let byte_terms = lb.cpu_index + lb.cpu_query + lb.cpu_key_reads;
    let fixed = lb.cpu_verify + lb.cpu_frames + lb.cpu_context + (lb.cpu_key_planes - lb.cpu_key_reads);
    let fixed = fixed + lb.cpu_set_build;
    let c = (p.get(C::CpuWindowBCores) - fixed) / byte_terms;
    p.set(C::CpuPerByte, c);
    let lb = ledger(&p, &mix_b, ReadSet::AsShipped);

    let at = |rounds: [f64; 4], rev: f64| ledger(&p, &measured_mix(&p, rounds, rev, 0.0), ReadSet::AsShipped);
    let windows = [
        at(window_b::ROUNDS, window_b::REV_ROUNDS),
        at(window_a_prime::ROUNDS, window_b::REV_ROUNDS),
        at(window_c::ROUNDS, window_c::REV_ROUNDS),
    ];
    let permits_pred = [0, 1, 2].map(|i| windows[i].permits_per_s * 60.0);
    let drain_pred = [0, 1, 2].map(|i| windows[i].index_pages_per_s * 60.0);
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
        b_cpu_rest: lb.cpu_verify + lb.cpu_frames + lb.cpu_context + lb.cpu_key_planes + lb.cpu_set_build,
        b_fitted_summaries_per_round: window_b::PERMITS / (lb.index_pages_per_s * 60.0 / p.get(C::SummariesPerRound)),
        permits_pred,
        drain_pred,
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
    /// Scenario flags, v45 candidate, not built: (b) the generation listing cache and the
    /// per-listing read budget.
    pub listing_fix: ListingFix,
    /// Scenario flag: how the key-plane memos refill (as shipped: whole, every 10 min).
    pub memo_rebuild: MemoRebuild,
    /// The reach rule of 6244795 (in 8e46165): an unowned subject's reach reads the store.
    pub reach_rule: bool,
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
            listing_fix: ListingFix { cache: false, budget: false },
            memo_rebuild: MemoRebuild::Whole,
            reach_rule: false,
        }
    }

    /// The same with what 8e46165 adds on the seat beside the read path: the listing cache, the
    /// read budget (both on by default) and the reach rule of 6244795.
    #[must_use]
    pub const fn with_candidate(mut self) -> Self {
        self.listing_fix = ListingFix { cache: true, budget: true };
        self.reach_rule = true;
        self
    }

    /// v45.0.0 as release/45.0.0 has it at 8e46165: the required-set Pull, the read
    /// minimisation series, the listing cache, the read budget and the reach rule; one seat.
    #[must_use]
    pub const fn v45(devices: f64) -> Self {
        Self::one_seat(devices, RoundShape::RequiredSet, ReadSet::AfterReadMin).with_candidate()
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
    let cycles = p.get(C::CadenceS) / p.get(C::PullCycleS);
    let timer = cycles * (p.get(C::PullReplyBytesPerCycle) + p.get(C::PullCallsPerCycle) * fo);
    let events = pull_event_legs_per_device_s(p) * p.get(C::CadenceS);
    revocation_round + timer + events * (p.get(C::PullRefsPerLeg) * rb + fo)
}

/// Pull legs (one subject on one plane) one no-consent device sends one seat per second in
/// the v45 baseline (8e46165 runtime.rs BundlePull::cycle :1550):
/// - timer: self and seat on the four Pull planes, owner on three when claimed, every 600 s mean;
/// - each runtime start: `first_contact_cycles` cycles of self and seat on Key, IO and TD (IOR
///   waits for attribution);
/// - each kicked write cycle: self on the written plane;
/// - each host kick: self and seat on the four Pull planes.
#[must_use]
pub fn pull_legs_per_device_s(p: &CostParams) -> f64 {
    pull_timer_legs_per_device_s(p) + pull_event_legs_per_device_s(p)
}

/// The timer's share of [`pull_legs_per_device_s`]: one cycle every `pull_cycle_s`.
#[must_use]
pub fn pull_timer_legs_per_device_s(p: &CostParams) -> f64 {
    (2.0 * p.get(C::PullPlanes) + p.get(C::ClaimedShare) * p.get(C::OwnerPlanes)) / p.get(C::PullCycleS)
}

/// First-contact, write and kick legs per device per second (the rest of the Pull legs).
#[must_use]
pub fn pull_event_legs_per_device_s(p: &CostParams) -> f64 {
    let pull = p.get(C::PullPlanes);
    let first = p.get(C::FirstContactCycles).min(p.get(C::FirstContactTries)) * 2.0 * (pull - 1.0);
    (p.get(C::RuntimeStartsPerDeviceDay) * first
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
            // The one ordinary round: Revocation (BUNDLE_ROUND_PLANES, 8e46165 runtime.rs :1140).
            let kr = nr * per_min;
            m.rounds_per_min[Plane::Revocation as usize] += kr;
            m.small_reads_per_min += kr * p.get(C::SmallReadsPerCadence) / 5.0;
            m.attribution_reads_per_min += kr * p.get(C::AttributionReadsPerCadence);
            // The 10-minute cycle at its measured cost (debug build); event legs one read and
            // one permit each (MODEL).
            let legs = nr * pull_legs_per_device_s(p) * 60.0;
            let cycles = nr * 60.0 / p.get(C::PullCycleS);
            let events = nr * pull_event_legs_per_device_s(p) * 60.0;
            m.pull_requests_per_min += legs;
            m.pull_reads_per_min += cycles * p.get(C::PullCallsPerCycle) + events * p.get(C::PullReadsPerLeg);
            // The measured 8 permits are an unclaimed device's self and seat legs; a claimed
            // device's owner legs are answered in the entitled arm, one permit each.
            let owner_permits = p.get(C::ClaimedShare) * p.get(C::OwnerPlanes) * p.get(C::OwnerLegPermitShare).clamp(0.0, 1.0);
            m.pull_permits_per_min += cycles * (p.get(C::PullPermitsPerCycle) + owner_permits) + events;
            // The reach rule on the IO and route legs (6244795): per cycle the device's own
            // subject (an unclaimed device is unowned: the reach reads) and, when claimed, its
            // owner's (an identity key, unowned: one read); each memoized 30 s, shorter than
            // the cycle, so read once a cycle.
            let claimed = p.get(C::ClaimedShare).clamp(0.0, 1.0);
            m.pull_reach_reads_per_min += cycles * ((1.0 - claimed) * p.get(C::ReachReadsPerUnownedSubject) + claimed);
            return;
        }
    }
    m.attribution_reads_per_min += k * p.get(C::AttributionReadsPerCadence);
}

/// Today's peers at the seat: window B's measured round mix (MEASURED), held fixed, with the
/// rows withheld per attestation round scaled by the attestation store (MODEL).
fn book_today_peers(p: &CostParams, m: &mut RoundMix) {
    let k = p.get(C::TodayRoundsScale);
    let r = window_b::ROUNDS.map(|x| x * k);
    m.rounds_per_min[Plane::Attestation as usize] += r[0];
    m.rounds_per_min[Plane::IdentityOccurrence as usize] += r[1];
    m.rounds_per_min[Plane::Key as usize] += r[2];
    m.rounds_per_min[Plane::TransportDestination as usize] += r[3];
    m.rounds_per_min[Plane::Revocation as usize] += window_b::REV_ROUNDS;
    m.rounds_per_min[Plane::IdentityOccurrenceRevocation as usize] += window_b::REV_ROUNDS;
    // Devices add no refusing rounds, so the baseline is the only source of withheld rows.
    m.refusing_att_rounds_per_min += r[0];
    // The withhold gates are on the Attestation plane only and scale with its rounds, not with
    // devices or the store (edge source read 2026-11-10 at v44.1.2 (withhold-rate)).
    m.refused_rows_per_round = window_b::WITHHOLDS * k / r[0];
}

/// Cost one scenario.
#[must_use]
pub fn run(p: &CostParams, s: &Scenario) -> Outcome {
    let seats = f64::from(s.seats.max(1));
    let nodes = s.devices.max(p.get(C::StoreFloorNodes));
    let per_min = 60.0 / p.get(C::CadenceS);
    let proxies = if s.offload_f > 0.0 { f64::from(s.proxies.max(1)) } else { 0.0 };
    let offloaded = s.devices * s.offload_f.clamp(0.0, 1.0);
    let at_seat_fleet = s.devices - offloaded;
    let share = match s.select {
        SeatSelect::AllSeats => 1.0,
        SeatSelect::HomeOne => 1.0 / seats,
    };
    let mut m = RoundMix {
        stale_routes_per_s: p.get(C::StaleRoutesPerS),
        context_cores: p.get(C::ServerRefreshCores),
        listing_fix: s.listing_fix,
        memo_rebuild: s.memo_rebuild,
        refills_booked: true,
        reach_rule: s.reach_rule,
        ..RoundMix::sized(p, nodes, 1.0)
    };
    add_devices(p, &mut m, at_seat_fleet * share, s.shape);
    // key:record rows landing at the seat (enrolment, re-publish, rotation).
    m.key_record_writes_per_min += s.devices * p.get(C::KeyRecordWritesPerDeviceDay) / 1_440.0;
    // Devices the server names as the seat's own peers: the seat initiates a round to each on
    // the five bootstrap planes every cadence, lists to build its Summary (costed as a round
    // on that plane) and receives the device's reply Summary (a drain as shipped).
    let initiated = at_seat_fleet * share * p.get(C::SeatInitiatorDeviceShare).clamp(0.0, 1.0);
    if initiated > 0.0 {
        for plane in [
            Plane::Key,
            Plane::IdentityOccurrence,
            Plane::TransportDestination,
            Plane::Revocation,
            Plane::IdentityOccurrenceRevocation,
        ] {
            m.rounds_per_min[plane as usize] += initiated * per_min;
        }
        m.reply_summaries_per_min += initiated * 5.0 * per_min;
    }
    if p.get(C::TodayPeersBooked) >= 0.5 {
        book_today_peers(p, &mut m);
    }
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
            listing_fix: s.listing_fix,
            memo_rebuild: s.memo_rebuild,
            refills_booked: true,
            reach_rule: s.reach_rule,
            ..RoundMix::sized(p, nodes, store)
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

/// Named scenario, not a default: the seat restarts with every key-plane memo empty and each
/// peer's first round on each plane arrives within `restart_arrival_s`. The plane lock makes
/// each refill single flight (8e46165 bridge.rs key_plane_listing :7664: N rounds arriving together cost one
/// fill), so each listed plane reads every subject once, and the rounds arriving meanwhile
/// wait on that plane's lock. Planes nobody lists are not read.
#[derive(Debug, Clone, Copy)]
pub struct ColdRestart {
    /// Subjects per key plane, by [`KEY_PLANES`] order.
    pub subjects: [f64; 4],
    /// Store reads of the refills (listed planes only).
    pub store_reads: f64,
    /// The longest plane stall, seconds (K reads with the budget).
    pub stall_s: f64,
    /// Which plane stalls longest.
    pub stall_plane: Plane,
    /// Rounds that arrive on that plane during its stall (each waits on the lock).
    pub rounds_waiting: f64,
}

/// Cost the cold-restart scenario for one seat scenario.
#[must_use]
pub fn cold_restart(p: &CostParams, sc: &Scenario) -> ColdRestart {
    let o = run(p, sc);
    let t = p.get(C::StoreReadS);
    let mut out = ColdRestart { subjects: [0.0; 4], store_reads: 0.0, stall_s: 0.0, stall_plane: Plane::Key, rounds_waiting: 0.0 };
    for (ki, plane) in KEY_PLANES.into_iter().enumerate() {
        let g = o.seat.key_planes[ki];
        out.subjects[ki] = g.subjects;
        if g.listings_per_s <= 0.0 {
            continue;
        }
        let reads = g.subjects * p.queries_per_subject(plane);
        out.store_reads += reads;
        let held = if sc.listing_fix.budget { reads.min(p.get(C::ListingBudgetK)) } else { reads };
        let stall = held * t;
        if stall > out.stall_s {
            out.stall_s = stall;
            out.stall_plane = plane;
            // first rounds arrive spread over restart_arrival_s, at least the steady rate
            let arrivals = (g.listings_per_s * p.get(C::CadenceS) / p.get(C::RestartArrivalS).max(1.0e-9)).max(g.listings_per_s);
            out.rounds_waiting = arrivals * stall;
        }
    }
    out
}

/// The largest fleet whose busiest key-plane lock stays under utilisation 1 (bisection).
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
        if hi / lo < 1.001 {
            break;
        }
    }
    lo
}

/// Which term binds a seat: CPU, sweep permits, or a key plane's lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// The seat's cores.
    Cpu,
    /// The two sweep permits.
    Permits,
    /// One key plane's lock (one server).
    Lock(Plane),
}

impl Binding {
    /// Label.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Cpu => "CPU".into(),
            Self::Permits => "sweep permits".into(),
            Self::Lock(k) => format!("{} plane lock", k.as_str()),
        }
    }
}

/// The binding term of a scenario.
#[must_use]
pub fn binding(p: &CostParams, sc: &Scenario) -> Binding {
    let o = run(p, sc);
    let l = o.seat_load;
    if l.rho_lock >= l.rho_cpu && l.rho_lock >= l.rho_permit {
        Binding::Lock(o.seat.busiest_plane())
    } else if l.rho_permit > l.rho_cpu {
        Binding::Permits
    } else {
        Binding::Cpu
    }
}

/// The named versions the knee report compares. v45.0.0 is release/45.0.0 at 8e46165 (untagged
/// when read): the required-set Pull with the read-minimisation series, the listing cache, the read
/// budget and the reach rule. The row without the read fix was never released and is kept for the
/// receive-side drain finding.
#[must_use]
pub fn versions() -> [(&'static str, Scenario); 3] {
    [
        ("v44 (today's code): every device that rounds with the seat on v44, whole-plane rounds on five planes", Scenario::one_seat(1.0, RoundShape::WholePlane, ReadSet::AsShipped)),
        ("v45 without the read fix (never released; e3c20ec, cited before as c580298)", Scenario::one_seat(1.0, RoundShape::RequiredSet, ReadSet::AsShipped)),
        ("v45.0.0 (release/45.0.0 at 8e46165, untagged): required-set Pull, read minimisation, listing cache, read budget, reach rule", Scenario::v45(1.0)),
    ]
}

/// The receive-side drain without the read fix, at the measured debug-build cost per index page:
/// `devices` attributed devices each sending one round per 30 s on each of `planes`, every round
/// draining `pages` index pages at the seat. Wall-clock cores (debug build).
#[must_use]
pub fn drain_cores_debug(p: &CostParams, devices: f64, pages: f64, planes: &[Plane]) -> f64 {
    let per_page: f64 = planes
        .iter()
        .map(|pl| match pl {
            Plane::Key => p.get(C::DrainPageKeyS),
            Plane::IdentityOccurrence => p.get(C::DrainPageIoS),
            Plane::Revocation => p.get(C::DrainPageRevS),
            Plane::IdentityOccurrenceRevocation => p.get(C::DrainPageIorS),
            _ => 0.0,
        })
        .sum();
    devices / p.get(C::CadenceS) * pages * per_page
}

/// Knees by version, each an interval over the input intervals (PROJECTED) at this
/// `key_publish_set_growth`: the seat knee and what binds at its two ends, the lock knee, the
/// seat knee with a quarter of the devices still on v44, and the seat's cores at today's fleet.
#[must_use]
pub fn versions_table(p: &CostParams) -> String {
    use crate::seat_calibration::{cores_interval, input_intervals, knee_interval, propagate};
    let mut s = format!(
        "At key_publish_set_growth = {} (FRONTIER; swept in the growth table). Every figure is an interval over the input intervals.\n\n| version | seat knee, devices (PROJECTED) | binds at the low / high end | lock knee, devices (PROJECTED) | seat knee, 25% old peers (PROJECTED) | today's fleet: seat CPU cores |\n|---|---|---|---|---|---|\n",
        p.get(C::KeyPublishSetGrowth)
    );
    let mut q = p.clone();
    q.set(C::OldPeerShare, 0.25);
    let inputs = input_intervals(p);
    for (name, sc) in versions() {
        let k = knee_interval(p, &sc);
        let lk = propagate(p, &inputs, |x| lock_knee(x, &sc)).interval;
        let cores = cores_interval(p, &sc, p.get(C::AnnouncedNodes));
        s.push_str(&format!(
            "| {} | {} | {} / {} | {} | {} | {} to {} |\n",
            name,
            k.devices.devices(),
            k.binds_low.label(),
            k.binds_high.label(),
            lk.devices(),
            if sc.shape == RoundShape::WholePlane { "same (all v44)".to_string() } else { knee_interval(&q, &sc).devices.devices() },
            fmt_n(cores.lo),
            fmt_n(cores.hi),
        ));
    }
    s
}

/// The key planes at today's fleet: v44 as measured (window B's rounds) and v45 projected.
#[must_use]
pub fn key_planes_today_table(p: &CostParams) -> String {
    let mut v44mix = measured_mix(p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS);
    v44mix.refills_booked = true;
    let v44 = ledger(p, &v44mix, ReadSet::AsShipped);
    let nodes = p.get(C::AnnouncedNodes);
    let v45 = run(p, &Scenario::v45(nodes)).seat;
    let mut s = String::from(
        "At the point inputs (a diagnostic, MODEL). 'refill reads' is one TTL refill: at v44.1.2 one burst under the lock; at 8e46165 spread over listings at most K reads each, so the stall is K reads.\n\n| row | plane | subjects | listings/s | mean hold ms | refill reads (one TTL) | longest stall s | rho lock |\n|---|---|---|---|---|---|---|---|\n",
    );
    for (label, l) in [("v44 today (MEASURED rounds, window B)", &v44), ("v45.0.0 at today's fleet (8e46165, one seat)", &v45)] {
        for (ki, k) in KEY_PLANES.into_iter().enumerate() {
            let g = l.key_planes[ki];
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {:.4} |\n",
                label,
                k.as_str(),
                fmt_n(g.subjects),
                fmt_n(g.listings_per_s),
                fmt_n(g.hold_s * 1_000.0),
                if g.rebuilds_per_s > 0.0 { fmt_n(g.burst_reads) } else { "none (not listed)".into() },
                fmt_n(g.stall_s),
                g.rho_lock,
            ));
        }
    }
    s.push_str(&format!(
        "\nToday's fleet is {} announced nodes. Key and occurrence walk the host's publish selector; the two Global planes walk own ∪ cohort, which on a seat is every key it holds: {} subjects at seat-1 (DERIVED, 8e46165 bridge.rs :13152; the twin had 30 until 2026-11-11). Under v45 a no-consent device lists only Revocation (its one ordinary round), so the Revocation walk is the one that grows with devices; it Pulls the rest by subject, outside every plane lock. Key and occurrence are listed by today's peers and by old and consent peers.\n",
        fmt_n(nodes),
        fmt_n(p.get(C::GlobalSubjectsAtSeat)),
    ));
    s
}

/// The old-peer axis: the share of devices that still round on every bootstrap plane (v44
/// peers kept on purpose, consent peers), by fleet size; v45.0.0 (8e46165), one seat. The rho
/// and core columns are at the point inputs (diagnostic); the knees are intervals (PROJECTED).
#[must_use]
pub fn mixed_fleet_table(p: &CostParams) -> String {
    use crate::seat_calibration::{input_intervals, knee_interval, propagate};
    let mut s = String::from(
        "| devices | old peer share | rho lock Key | rho lock IO | rho lock Rev | refill reads/s | seat CPU cores | seat rho | lock knee (PROJECTED) | seat knee (PROJECTED) |\n|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for d in [p.get(C::AnnouncedNodes), 10_000.0, 100_000.0] {
        for share in OLD_PEER_SHARES {
            let mut q = p.clone();
            q.set(C::OldPeerShare, share);
            let sc = Scenario::v45(d);
            let o = run(&q, &sc);
            let k = o.seat.key_planes;
            let lk = propagate(&q, &input_intervals(&q), |x| lock_knee(x, &sc)).interval;
            s.push_str(&format!(
                "| {} | {} | {:.3} | {:.3} | {:.3} | {} | {} | {:.3} | {} | {} |\n",
                fmt_n(d),
                share,
                k[0].rho_lock,
                k[1].rho_lock,
                k[2].rho_lock,
                fmt_n(o.seat.ttl_reads_per_s),
                fmt_n(o.seat_load.cpu_cores),
                o.seat_load.rho,
                lk.devices(),
                knee_interval(&q, &sc).devices.devices(),
            ));
        }
    }
    s
}

/// Old-peer shares the mixed-fleet table sweeps (MODEL axis; production's share is not measured).
pub const OLD_PEER_SHARES: [f64; 5] = [0.0, 0.1, 0.25, 0.5, 1.0];

/// The key planes per fleet size for one round shape, as shipped, apart from the read ledger.
#[must_use]
pub fn key_plane_table(p: &CostParams, shape: RoundShape) -> String {
    let mut s = format!(
        "**{}, as shipped**: a = {} s, b = {} s per subject (DERIVED from the warm Key listing), one subject read = {} s and one probe page = {} s under the lock; a plane's lock is one server, the seat's {} cores do not help\n\n| devices | Key subjects | Key listings/s | Key hold ms | rho lock Key | rho lock IO | rho lock Rev | Key refill stall s | rounds per Key stall | key-plane CPU cores | seat CPU cores |\n|---|---|---|---|---|---|---|---|---|---|---|\n",
        shape.as_str(),
        p.get(C::KeyPlaneWalkBaseS),
        p.get(C::KeyPlaneWalkPerSubjectS),
        p.get(C::StoreReadS),
        p.get(C::ProbeReadS),
        p.get(C::SeatCores)
    );
    for d in SWEEP_DEVICES {
        let o = run(p, &Scenario::one_seat(d, shape, ReadSet::AsShipped));
        let k = o.seat.key_planes;
        s.push_str(&format!(
            "| {} | {} | {} | {} | {:.3} | {:.3} | {:.3} | {} | {} | {} | {} |\n",
            fmt_n(d),
            fmt_n(k[0].subjects),
            fmt_n(k[0].listings_per_s),
            fmt_n(k[0].hold_s * 1_000.0),
            k[0].rho_lock,
            k[1].rho_lock,
            k[2].rho_lock,
            fmt_n(k[0].stall_s),
            fmt_n(k[0].rounds_per_stall),
            fmt_n(o.seat.cpu_key_planes),
            fmt_n(o.seat_load.cpu_cores),
        ));
    }
    let c = cold_restart(p, &Scenario::one_seat(10_000.0, shape, ReadSet::AsShipped));
    s.push_str(&format!(
        "\nCold restart at 10,000 devices (named scenario): {} store reads over the listed planes, longest stall {} s on the {} plane, {} rounds waiting on it.\n",
        fmt_n(c.store_reads),
        fmt_n(c.stall_s),
        c.stall_plane.as_str(),
        fmt_n(c.rounds_waiting)
    ));
    s
}

/// Which fix moves which term at `devices`, for one round shape.
#[must_use]
pub fn fix_table(p: &CostParams, devices: f64, shape: RoundShape) -> String {
    let mut s = format!(
        "**{} devices, {}, one seat** (rates at the point inputs, a diagnostic; knees are intervals)\n\n| read fix | (b) cache | budget | TTL rebuild | drain pages/s | permits/s | probe reads/s | refill reads/s | key-plane CPU cores | rho lock (plane) | longest stall s | lock knee (PROJECTED) | seat knee (PROJECTED) |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
        fmt_n(devices),
        shape.as_str()
    );
    let rows: [(ReadSet, bool, bool, MemoRebuild); 7] = [
        (ReadSet::AsShipped, false, false, MemoRebuild::Whole),
        (ReadSet::AfterReadMin, false, false, MemoRebuild::Whole),
        (ReadSet::AsShipped, true, false, MemoRebuild::Whole),
        (ReadSet::AsShipped, false, true, MemoRebuild::Whole),
        (ReadSet::AsShipped, true, true, MemoRebuild::Whole),
        (ReadSet::AfterReadMin, true, true, MemoRebuild::Whole),
        (ReadSet::AsShipped, false, false, MemoRebuild::Removed),
    ];
    let yn = |b: bool| if b { "yes" } else { "no" };
    for (read, cache, budget, rebuild) in rows {
        let mut sc = Scenario::one_seat(devices, shape, read);
        sc.listing_fix = ListingFix { cache, budget };
        sc.memo_rebuild = rebuild;
        let o = run(p, &sc);
        let stall = o.seat.key_planes.iter().map(|g| g.stall_s).fold(0.0, f64::max);
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.3} ({}) | {} | {} | {} |\n",
            yn(read == ReadSet::AfterReadMin),
            yn(cache),
            yn(budget),
            rebuild.as_str(),
            fmt_n(o.seat.index_pages_per_s),
            fmt_n(o.seat.permits_per_s),
            fmt_n(o.seat.probe_reads_per_s),
            fmt_n(o.seat.ttl_reads_per_s),
            fmt_n(o.seat.cpu_key_planes),
            o.seat.rho_lock,
            o.seat.busiest_plane().as_str(),
            fmt_n(stall),
            crate::seat_calibration::propagate(p, &crate::seat_calibration::input_intervals(p), |x| lock_knee(x, &sc)).interval.devices(),
            crate::seat_calibration::knee_interval(p, &sc).devices.devices(),
        ));
    }
    // Stress (named, not the default): plane clears at STRESS_PLANE_CLEARS_PER_HOUR on both.
    let mut q = p.clone();
    q.set(C::RevPlaneClearsPerHour, STRESS_PLANE_CLEARS_PER_HOUR);
    q.set(C::IorPlaneClearsPerHour, STRESS_PLANE_CLEARS_PER_HOUR);
    let o = run(&q, &Scenario::one_seat(devices, shape, ReadSet::AsShipped));
    s.push_str(&format!(
        "\nStress row (named, not a default; seat-1 measures 0 clears per hour): {} clears per hour on each Global plane, as shipped: refill reads/s {}, rho lock {:.3}.\n",
        STRESS_PLANE_CLEARS_PER_HOUR,
        fmt_n(o.seat.ttl_reads_per_s),
        o.seat.rho_lock,
    ));
    s
}

/// The TTL refill set against the measured windows. Not fitted: a refill is one burst inside
/// one listing, so a 60 s window holds a whole refill, part of one at its edge, or none.
#[derive(Debug, Clone, Copy)]
pub struct TtlCheck {
    /// One Key refill at seat-1, reads (each a permit).
    pub key_burst: f64,
    /// One occurrence refill at seat-1, reads, at 1 and 2 reads per subject.
    pub io_burst: [f64; 2],
    /// Key refill stall, seconds.
    pub key_stall_s: f64,
    /// Occurrence refill stall, seconds, at 1 and 2 reads per subject.
    pub io_stall_s: [f64; 2],
    /// Mean refill reads per minute, Key and occurrence (unchanged by the budget).
    pub mean_per_min: [f64; 2],
    /// Share of 60 s windows that hold any of one plane's refill.
    pub window_share: f64,
}

/// Named stress value for `rev_plane_clears_per_hour` and `ior_plane_clears_per_hour`
/// (MODEL); seat-1 measures 0 per hour on both.
pub const STRESS_PLANE_CLEARS_PER_HOUR: f64 = 0.5;

/// The TTL refill at seat-1 size.
#[must_use]
pub fn ttl_check(p: &CostParams) -> TtlCheck {
    let t = p.get(C::StoreReadS);
    let key = p.get(C::KeySubjectsAtSeat) * p.get(C::KeyQueriesPerSubject);
    let io = [1.0, 2.0].map(|q| p.get(C::IoSubjectsAtSeat) * q);
    let ttl = p.get(C::KeyMemoExpiryS);
    TtlCheck {
        key_burst: key,
        io_burst: io,
        key_stall_s: key * t,
        io_stall_s: io.map(|r| r * t),
        mean_per_min: [key * 60.0 / ttl, p.get(C::IoSubjectsAtSeat) * p.get(C::IoQueriesPerSubject) * 60.0 / ttl],
        window_share: 60.0 / ttl,
    }
}

/// The TTL check as text.
#[must_use]
pub fn ttl_check_report(p: &CostParams) -> String {
    let t = ttl_check(p);
    let c = calibrate(p);
    format!(
        "The candidate explanation for the A' and C excess (MODEL: fits A' and C, not proven). At v44.1.2, the code the windows measured, a Key or occurrence TTL refill read every subject in one burst while holding that plane's lock, one sweep permit per read (v44.1.2 bridge.rs:7149). At 8e46165 the TTL opens a new epoch instead (stale-while-revalidate) and a listing reads at most K subjects, so the burst is gone from a v45 window: the same reads spread over listings. Each plane runs on its own phase, so about one 60 s window in {:.0} per plane holds one: a Key refill is {} reads in about {} s, an occurrence refill {} to {} reads in {} to {} s (1 to 2 reads per subject, mostly not attestation_query, which fits permits rising while that counter does not).\n\n\
- The residuals after drains and listings are {} (B), {} (A') and {} (C) a minute. A' and C are the size of part of one burst, or of an occurrence refill at the low end; neither is a whole Key burst. B's residual is not a burst at all and stays UNEXPLAINED. Two of three windows holding a refill is more than one in {:.0} per plane predicts. Not tuned to fit; all three residuals stay UNEXPLAINED until v45's `key_plane_sweep.rebuilds_ttl` and `sweep_permits.key_plane_subject` counters settle it.\n\
- Averaged, the refills add {} (Key) and {} (occurrence) reads a minute at seat-1; the per-listing budget (K = {}, built and on at 8e46165) leaves that unchanged and caps a stall at {} s.\n",
        1.0 / t.window_share,
        fmt_n(t.key_burst),
        fmt_n(t.key_stall_s),
        fmt_n(t.io_burst[0]),
        fmt_n(t.io_burst[1]),
        fmt_n(t.io_stall_s[0]),
        fmt_n(t.io_stall_s[1]),
        fmt_n(c.permit_residual[0]),
        fmt_n(c.permit_residual[1]),
        fmt_n(c.permit_residual[2]),
        1.0 / t.window_share,
        fmt_n(t.mean_per_min[0]),
        fmt_n(t.mean_per_min[1]),
        fmt_n(p.get(C::ListingBudgetK)),
        fmt_n(p.get(C::ListingBudgetK) * p.get(C::StoreReadS)),
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
    row(&mut s, "window B sweep permits / min (drains + listings + probes)", c.permits_pred[0], window_b::PERMITS, "UNEXPLAINED residual");
    row(&mut s, "window B attestation queries / min", c.b_att_pred, window_b::ATT_QUERIES, "MEASURED ratios");
    row(&mut s, "window B composer lookups / min", c.b_composer_pred, window_b::COMPOSER, "MEASURED ratios");
    row(&mut s, "window B CPU, cores", c.b_cpu_index + c.b_cpu_query + c.b_cpu_rest, window_b::CPU_CORES, "FITTED (exact by construction)");
    row(&mut s, "busy hour CPU, cores (at its permit and query rates)", c.busy_cpu_pred, busy_hour::CPU_CORES, "check, not fitted");
    row(&mut s, "window A' sweep permits / min", c.permits_pred[1], window_a_prime::PERMITS, "UNEXPLAINED residual");
    row(&mut s, "window C sweep permits / min", c.permits_pred[2], window_c::PERMITS, "UNEXPLAINED residual");
    row(&mut s, "window A consent-grant reads / min (one per withheld row)", window_a::WITHHOLDS * p.get(C::ConsentPerRefusedRow), window_a::CONSENT, "MEASURED ratio");
    s.push_str(&format!(
        "\nFitted `cpu_per_byte_s` = {:.3e} CPU-s per decoded byte ({:.1} MB/s per core); one index page = {:.2} ms.\n\
         Window B CPU split: index pages {:.3} cores, counted queries {:.3} cores, verifies + frames + key planes + server context {:.3} cores (split PROVISIONAL: one CPU window cannot separate the two byte classes).\n\
         Summaries per completed round, fitted from window B's permits: {:.3} (derived: {:.1}).\n\
         What a sweep permit counts (edge source read 2026-11-10 at v44.1.2): a holdings drain takes one per PAGE (bridge.rs:10072); an Attestation listing 2 to 4 however many rows it withholds, the required-rows read one, a route listing 1 or 2; a key-plane listing one per probe page and one per subject it reads (bridge.rs:7233, :7149); a subject Pull one. No withhold site takes one: the grant read runs inside the page permit. So the withheld-row reading of the counter is false and is dropped; what follows withholds is grant reads (A: {} against {}). Predicted from the finals with no refill: drains B {:.0}, A' {:.0}, C {:.0} of permits B {:.0}, A' {:.0}, C {:.0}. Residuals B {:.0}, A' {:.0}, C {:.0} a minute, UNEXPLAINED and absorbed into no constant. The excess is in permits, not in attestation_query (window C: {} queries over 66 finals that drain the attestation index).\n",
        c.cpu_per_byte_s,
        1.0 / c.cpu_per_byte_s / 1.0e6,
        p.page_cpu_s() * 1_000.0,
        c.b_cpu_index,
        c.b_cpu_query,
        c.b_cpu_rest,
        c.b_fitted_summaries_per_round,
        p.get(C::SummariesPerRound),
        fmt_n(window_a::CONSENT),
        fmt_n(window_a::WITHHOLDS),
        c.drain_pred[0],
        c.drain_pred[1],
        c.drain_pred[2],
        c.permits_pred[0],
        c.permits_pred[1],
        c.permits_pred[2],
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
                cells.push(crate::seat_calibration::knee_interval(p, &sc).devices.devices());
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
                    listing_fix: ListingFix::default(),
                    memo_rebuild: MemoRebuild::Whole,
                    reach_rule: false,
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
    let sc = match read {
        ReadSet::AsShipped => Scenario::one_seat(d, RoundShape::RequiredSet, read),
        ReadSet::AfterReadMin => Scenario::v45(d),
    };
    let o = run(p, &sc);
    let l = &o.seat;
    let c = p.get(C::CpuPerByte);
    let per_read = (p.get(C::RowBytes) + p.get(C::CallOverheadBytes)) * c;
    let pull_reads = d * (p.get(C::PullCallsPerCycle) / p.get(C::PullCycleS) + pull_event_legs_per_device_s(p) * p.get(C::PullReadsPerLeg));
    let pull_cpu = pull_reads * per_read;
    let attr_cpu = l.attribution_per_s * per_read;
    vec![
        Term { name: "index pages (drains / audit)", value: format!("{} pages/s, {} cores", fmt_n(l.index_pages_per_s), fmt_n(l.cpu_index)), tag: "DERIVED (page model) x FITTED (cpu/byte)" },
        Term { name: "subject Pull reads", value: format!("{} reads/s, {} cores", fmt_n(pull_reads), fmt_n(pull_cpu)), tag: "MEASURED (debug; release rerun pending): 10 calls per 10-minute cycle; MODEL: first-contact, write and kick legs" },
        Term { name: "attribution-check reads", value: format!("{} reads/s, {} cores", fmt_n(l.attribution_per_s), fmt_n(attr_cpu)), tag: "DERIVED (3 s cache vs 30 s cadence)" },
        Term { name: "frame handling", value: format!("{} cores", fmt_n(l.cpu_frames)), tag: "MODEL" },
        Term { name: "server relay refresh", value: format!("{} cores", fmt_n(l.cpu_context)), tag: "MEASURED-CONTEXT (not edge)" },
        Term { name: "seat CPU total", value: format!("{} cores of {} (rho {:.1})", fmt_n(o.seat_load.cpu_cores), fmt_n(p.get(C::SeatCores)), o.seat_load.rho), tag: "sum" },
        Term { name: "sweep permits", value: format!("rho {:.2}", o.seat_load.rho_permit), tag: "DERIVED" },
        Term { name: "seat egress", value: format!("{} Mbit/s", fmt_n(o.seat_egress_bytes_per_s * 8.0 / 1.0e6)), tag: "DERIVED (ref bytes) / MODEL (refs per Pull)" },
        Term { name: "device links at the seat", value: format!("{} (cap {})", fmt_n(o.seat_links), fmt_n(p.get(C::LiveLinkCap))), tag: "PROVISIONAL cap" },
        Term { name: "shaped device links at the seat", value: format!("8 of {} (the rest run unshaped on the same homing link)", fmt_n(o.seat_links)), tag: "SOURCED (edge link_shaping.rs:751 NODE_CAP_LINKS; seat class is seat to seat only, :2867)" },
        Term { name: "subject Pull legs", value: format!("{} per s ({} per device per day)", fmt_n(d * pull_legs_per_device_s(p)), fmt_n(pull_legs_per_device_s(p) * 86_400.0)), tag: "DERIVED (8e46165 runtime.rs BUNDLE_PULL_PLANES, 600 s timer) + MODEL (starts, writes, kicks)" },
        Term { name: "Revocation rounds", value: format!("{} per s (one per device per 30 s, the seat's Projection::Global offer)", fmt_n(d / p.get(C::CadenceS))), tag: "DERIVED (8e46165 runtime.rs BUNDLE_ROUND_PLANES) / MODEL (offer size)" },
        Term { name: "reach-rule reads (6244795)", value: format!("{} reads/s", fmt_n(l.reach_reads_per_s)), tag: if sc.reach_rule { "DERIVED (bridge.rs unowned_reach) / MODEL (unowned share, reads per subject)" } else { "not in this code" } },
        Term { name: "subject-set builds before each key-plane listing", value: format!("{} cores", fmt_n(l.cpu_set_build)), tag: "MODEL (per-subject cost); DERIVED (Global = every key, bridge.rs :13152)" },
        Term { name: "key-plane listings", value: format!("{} listings/s, {} cores, {} probe reads/s, {} refill reads/s, busiest lock rho {:.2} on {} (one server per plane)", fmt_n(l.listings_per_s), fmt_n(l.cpu_key_planes), fmt_n(l.probe_reads_per_s), fmt_n(l.ttl_reads_per_s), l.rho_lock, l.busiest_plane().as_str()), tag: "DERIVED b (warm Key listing) / MODEL a, probe, cache hit; lock held per listing (8e46165 bridge.rs key_plane_listing :7664); TTL revalidation key_plane_memo.rs :462, budget :296" },
        Term { name: "holdings memory (v45 plan)", value: fmt_bytes(l.holdings_bytes), tag: "DERIVED" },
    ]
}

/// What the knees by version rest on, with the measured debug-build drain beside the model.
#[must_use]
pub fn knee_status(p: &CostParams) -> String {
    let n = p.get(C::AnnouncedNodes);
    let pages = p.pages(p.get(C::SeatAttRows));
    let rev = drain_cores_debug(p, n, pages, &[Plane::Revocation]);
    let five = drain_cores_debug(p, n, pages, &[Plane::Key, Plane::IdentityOccurrence, Plane::Revocation, Plane::IdentityOccurrenceRevocation]);
    let mut shipped = Scenario::one_seat(n, RoundShape::RequiredSet, ReadSet::AsShipped);
    shipped.devices = n;
    let model = run(p, &shipped).seat_load.cpu_cores;
    format!(
        "v45.0.0 is release/45.0.0 at 8e46165 (untagged when read 2026-11-11): the required-set Pull with the read minimisation series, the listing cache and the read budget (both BUILT and on by default, not MODEL candidates), and the reach rule (6244795). Its Global planes walk every key the seat holds (8,698 at seat-1), not tens. Every knee below is an interval over the input intervals (seat_calibration), PROJECTED, and the growth of the server's key_publish_set (FRONTIER) is swept in its own table, because which term binds turns on it. It rests on terms not measured on a release build: how long a Pull holds its permit (`store_read_s`, ~2 ms, DERIVED from a key-plane refill), a cache hit's set comparison (`warm_listing_s`, MODEL) and the Pull's own cost (`pull_calls_per_cycle`, `pull_permits_per_cycle`: MEASURED on an edge debug build).\n\nThe row without the read fix was never released. Each attributed device's 30 s Revocation round drains the whole attestation index at the seat ({pages:.0} pages at seat-1), so it binds on the permits, and its knee rests on what one index page costs in CPU (PROVISIONAL). Measured per page on an edge debug build (wall time, in-memory store, release rerun pending), that drain at {n:.0} devices costs {rev:.1} cores; with whole-plane rounds on the four planes that read the index it would cost {five:.1}. The model puts the whole seat at {model:.1} cores at that fleet.\n\nWhat a sweep permit counts is settled from source (edge source read 2026-11-10 at v44.1.2; the sites by label at 8e46165): drains one per page, listings 2 to 4, key-plane probes and subject reads one each, a consent send-set read one, withholds none. The lock knees rest on b (DERIVED from the warm Key listing), the subject-read time (DERIVED, ~2 ms) and the probe-page time (MODEL)."
    )
}

/// The whole capacity report, markdown (what `capacity_sweep` prints).
#[must_use]
pub fn report_markdown(p: &CostParams) -> String {
    let mut s = String::new();
    s.push_str("## Calibration (seat-1)\n\n");
    s.push_str(&calibration_report(p));
    s.push('\n');
    s.push_str(&crate::seat_calibration::report(p));
    s.push_str("\n### Energy counters (HOOKS.md, counts only)\n\n");
    s.push_str(&crate::seat_hooks::energy_report(p));
    s.push_str("\n## Knees by version\n\n");
    s.push_str(&knee_status(p));
    s.push_str("\n\n");
    s.push_str(&versions_table(p));
    s.push_str("\n## Knee: largest fleet with the seat under the ceiling, by shape, read set and seats\n\n");
    s.push_str(&knee_table(p));
    s.push_str("\n## One-seat sweep\n\n");
    for shape in [RoundShape::WholePlane, RoundShape::RequiredSet] {
        for read in [ReadSet::AsShipped, ReadSet::AfterReadMin] {
            s.push_str(&sweep_table(p, shape, read));
            s.push('\n');
        }
    }
    s.push_str("## Key planes: lock, walk and TTL refill, apart from the read ledger (MODEL)\n\n");
    s.push_str("### Today's fleet\n\n");
    s.push_str(&key_planes_today_table(p));
    s.push_str("\n### By fleet size (no old peers)\n\n");
    for shape in [RoundShape::RequiredSet, RoundShape::WholePlane] {
        s.push_str(&key_plane_table(p, shape));
        s.push('\n');
    }
    s.push_str("### Old peers: share that still rounds on every bootstrap plane\n\n");
    s.push_str(&mixed_fleet_table(p));
    s.push_str("\n### Which fix moves which term\n\n");
    for (d, shape) in [(10_000.0, RoundShape::RequiredSet), (100_000.0, RoundShape::RequiredSet), (3_000.0, RoundShape::WholePlane)] {
        s.push_str(&fix_table(p, d, shape));
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

    /// Relative tolerance; a zero target is checked absolutely (a relative error against 0
    /// is NaN, which compares false and failed every `within(x, 0.0, 0.0)`).
    fn within(x: f64, target: f64, tol: f64) -> bool {
        if target == 0.0 {
            return x.abs() <= tol;
        }
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
        // (10 + 11 + 10) x 42 + 11 x 18 = 1,500 drain pages; Global finals MEASURED 0. The
        // permits add the listing pages and key-plane probes (77.5) and the consent send-set
        // reads (6, one per 10 s memo) against 2,262 measured.
        assert!((c.b_pages_pred - 1_500.0).abs() < 1.0, "{}", c.b_pages_pred);
        assert!(within(c.b_pages_pred, c.drain_pred[0], 1e-12));
        assert!((c.permits_pred[0] - 1_583.5).abs() < 1e-6, "{}", c.permits_pred[0]);
    }

    /// Window B's counted reads are held out (seat_calibration::HELD_OUT): they are predicted
    /// from window A's ratios at window B's rounds and withholds, and the errors are reported,
    /// not bounded (seat_calibration pins them: +8.5% and -14.3%).
    #[test]
    fn window_b_counted_reads_are_predicted_from_window_a() {
        let p = CostParams::default();
        let c = calibrate(&p);
        let att = 10.0 * p.get(C::AttRequiredQueries) + window_b::WITHHOLDS * window_a::ATT_QUERIES / window_a::WITHHOLDS;
        assert!((c.b_att_pred - att).abs() < 1e-6, "att {} vs {att}", c.b_att_pred);
        assert!((c.b_composer_pred - window_a::COMPOSER).abs() < 1e-6, "composer {}", c.b_composer_pred);
    }

    #[test]
    fn calibrated_model_reproduces_seat_one() {
        let p = CostParams::calibrated();
        let c = calibrate(&p);
        let total = c.b_cpu_index + c.b_cpu_query + c.b_cpu_rest;
        assert!(within(total, p.get(C::CpuWindowBCores), 1e-9), "{total}");
        assert!(within(p.get(C::CpuWindowBCores), window_b::CPU_CORES, 0.0));
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
        // No per-withhold permit: the permits do not move with the withholds.
        let m0 = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, 0.0);
        let m1 = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS);
        let (l0, l1) = (ledger(&p, &m0, ReadSet::AsShipped), ledger(&p, &m1, ReadSet::AsShipped));
        assert!(within(l0.permits_per_s, l1.permits_per_s, 1e-12), "a per-withhold permit appeared");
        // Today nobody lists the Global planes; Key and occurrence are listed by today's peers.
        assert!(within(l1.key_planes[2].listings_per_s + l1.key_planes[3].listings_per_s, 0.0, 0.0));
        assert!(l1.key_planes[0].listings_per_s > 0.0 && l1.key_planes[1].listings_per_s > 0.0);
        // The prediction is drains plus listing pages plus one probe per key-plane listing plus
        // the send-set floor (one read per 10 s memo).
        let c = calibrate(&p);
        let listings = 10.0 * (p.get(C::AttListingPermits) + p.get(C::RequiredRowsPermits))
            + 11.0 * p.get(C::TdListingPermits)
            + 10.0
            + 11.0
            + 60.0 / p.get(C::SendSetMemoTtlS);
        assert!((c.permits_pred[0] - c.drain_pred[0] - listings).abs() < 1e-6, "{} {}", c.permits_pred[0], c.drain_pred[0]);
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
        // The read fix removes the grant read per withheld row (edge source read 2026-11-10);
        // the composer lookups go only if `refused_reads_left_after_fix` says so (MODEL).
        let shipped = run(&p, &Scenario::one_seat(4_000.0, RoundShape::WholePlane, ReadSet::AsShipped));
        assert!(b.seat.consent_per_s < shipped.seat.consent_per_s / 20.0, "{} vs {}", b.seat.consent_per_s, shipped.seat.consent_per_s);
        assert!(within(b.seat.composer_per_s, shipped.seat.composer_per_s, 1e-9));
        // Composers are not a per-withheld-row cost (edge source read 2026-11-10 at v44.1.2
        // (withhold-rate)): a withheld row made free leaves them as they are, and the queries
        // fall to the required read alone.
        let mut q = p.clone();
        q.set(C::RefusedReadsLeftAfterFix, 0.0);
        let free = run(&q, &Scenario::one_seat(4_000.0, RoundShape::WholePlane, ReadSet::AfterReadMin));
        assert!(within(free.seat.composer_per_s, b.seat.composer_per_s, 1e-12));
        assert!(free.seat.att_queries_per_s < b.seat.att_queries_per_s);
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
    /// With no bundle devices, "v45 at today's fleet" is today's seat: the booked baseline
    /// reproduces window B's CPU and door counts within the calibration's stated 12% band.
    #[test]
    fn today_peers_alone_reproduce_window_b() {
        let p = CostParams::calibrated();
        assert!(within(p.get(C::TodayPeersBooked), 1.0, 0.0), "the baseline is on by default");
        let o = run(&p, &Scenario::one_seat(0.0, RoundShape::RequiredSet, ReadSet::AsShipped));
        let cpu = o.seat.cpu_total();
        assert!(within(cpu, window_b::CPU_CORES, 0.12), "cpu {cpu}");
        // B's counted reads are held out: the baseline gives the held-out prediction exactly.
        let c = calibrate(&p);
        assert!(within(o.seat.att_queries_per_s * 60.0, c.b_att_pred, 1e-9), "att {}", o.seat.att_queries_per_s * 60.0);
        assert!(within(o.seat.composer_per_s * 60.0, c.b_composer_pred, 1e-9), "composer {}", o.seat.composer_per_s * 60.0);
        // Off, the seat carries only the server's own context and stale routes.
        let mut q = p.clone();
        q.set(C::TodayPeersBooked, 0.0);
        let bare = run(&q, &Scenario::one_seat(0.0, RoundShape::RequiredSet, ReadSet::AsShipped));
        assert!(bare.seat.cpu_total() < cpu * 0.6, "bare {} vs {cpu}", bare.seat.cpu_total());
        // As shipped the baseline lowers the knee. After read minimisation it may bind on a
        // term the baseline barely touches, so it may not move; it must not rise.
        let sc = |read| Scenario::one_seat(1.0, RoundShape::RequiredSet, read);
        let (s_on, s_off) = (knee(&p, &sc(ReadSet::AsShipped)), knee(&q, &sc(ReadSet::AsShipped)));
        let (a_on, a_off) = (knee(&p, &sc(ReadSet::AfterReadMin)), knee(&q, &sc(ReadSet::AfterReadMin)));
        assert!(s_on < s_off, "as shipped {s_on} vs {s_off}");
        assert!(a_on <= a_off, "after {a_on} vs {a_off}");
        eprintln!("today's peers alone: {cpu:.3} cores (B {}), bare {:.3}; knee as shipped {s_on:.0} vs {s_off:.0} without, after read-min {a_on:.0} vs {a_off:.0}", window_b::CPU_CORES, bare.seat.cpu_total());
    }

    /// The twin's edge constants are the ones edge has at [`edge_971::COMMIT`]. The source is
    /// found through the environment only ([`edge_971::source_file`]: `CEWP_EDGE_REPO`, read at
    /// the cited commit with `git show`, or `CEWP_EDGE_SRC`, or `CEWP_EDGE_RUNTIME_RS`); unset,
    /// it says so and checks only the parameters' agreement with [`edge_971`].
    #[test]
    fn edge_971_constants_match_the_source() {
        let p = CostParams::default();
        assert_eq!(p.get(C::PullPlanes), 4.0);
        assert_eq!(p.get(C::OwnerPlanes), 3.0);
        assert_eq!(p.get(C::PullCycleS), edge_971::BUNDLE_PULL_INTERVAL_S);
        assert_eq!(p.get(C::FirstContactSpacingS), 30.0);
        assert_eq!(p.get(C::FirstContactTries), 6.0);
        assert_eq!(p.get(C::CadenceS), edge_971::BUNDLE_PULL_MIN_SPACING_S);
        assert_eq!(p.get(C::KeyMemoExpiryS), edge_971::MEMO_TTL_S);
        assert_eq!(p.get(C::ListingBudgetK), edge_971::KEY_PLANE_READ_BUDGET as f64);
        assert_eq!(p.get(C::ProbePerS), 1.0 / edge_971::FEED_PROBE_SPACING_S);
        assert_eq!(p.get(C::AuditDrainS), edge_971::HOLDINGS_AUDIT_EVERY_S);
        assert_eq!(p.get(C::SweepPermits), edge_971::ADVERTISE_SWEEP_PERMITS as f64);
        assert_eq!(p.get(C::PageRows), f64::from(edge_971::SWEEP_PAGE_ROWS));
        assert_eq!(p.get(C::SendSetMemoTtlS), edge_971::CONSENT_SEND_SET_MEMO_TTL_S);
        let runtime = match edge_971::source_file("src/replication/runtime.rs") {
            Ok(src) => src,
            Err(why) => {
                eprintln!("edge_971_constants_match_the_source: source NOT compared ({why}); the twin cites {}", edge_971::COMMIT);
                return;
            }
        };
        let planes = |name: &str| -> Vec<String> {
            let at = runtime.find(&format!("pub const {name}: [EnvelopeKind;")).unwrap_or_else(|| panic!("{name} not in runtime.rs"));
            let body = &runtime[at..];
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
        let mut files: Vec<(&str, String)> = vec![("src/replication/runtime.rs", runtime.clone())];
        for n in edge_971::NEEDLES {
            if !files.iter().any(|(f, _)| *f == n.file) {
                let src = edge_971::source_file(n.file).unwrap_or_else(|e| panic!("{}: {e}", n.file));
                files.push((n.file, src));
            }
            let src = &files.iter().find(|(f, _)| *f == n.file).expect("read").1;
            assert!(src.contains(n.text), "{} ({}) changed since {}: {}", n.what, n.file, edge_971::COMMIT, n.text);
        }
        eprintln!("edge_971_constants_match_the_source: {} needles and 3 plane lists match edge at {}", edge_971::NEEDLES.len(), edge_971::COMMIT);
    }

    /// Each fix moves its own term: the read fix the drains and probes; (b) the listing hold and
    /// the probes, not the refill reads; the budget the stall, not the reads or the lock
    /// utilisation; a TTL change the refills. The walk is a + b x subjects.
    #[test]
    fn each_fix_moves_its_own_term() {
        let p = CostParams::calibrated();
        let d = 3_000.0;
        let at = |read, cache, budget, rebuild| {
            let mut sc = Scenario::one_seat(d, RoundShape::WholePlane, read);
            sc.listing_fix = ListingFix { cache, budget };
            sc.memo_rebuild = rebuild;
            run(&p, &sc).seat
        };
        let base = at(ReadSet::AsShipped, false, false, MemoRebuild::Whole);
        let read = at(ReadSet::AfterReadMin, false, false, MemoRebuild::Whole);
        let cache = at(ReadSet::AsShipped, true, false, MemoRebuild::Whole);
        let budget = at(ReadSet::AsShipped, false, true, MemoRebuild::Whole);
        let removed = at(ReadSet::AsShipped, false, false, MemoRebuild::Removed);
        let rows = at(ReadSet::AsShipped, false, false, MemoRebuild::FromRows);
        assert!(read.cpu_index < base.cpu_index, "the read fix moves the drains");
        assert!(read.probe_reads_per_s < base.probe_reads_per_s, "and the probes");
        assert!(within(read.ttl_reads_per_s, base.ttl_reads_per_s, 1e-9), "not the refills");
        assert!(within(read.listings_per_s, base.listings_per_s, 1e-9), "not the listings");
        let key = |l: &Ledger| l.key_planes[0];
        assert!(key(&cache).hold_s < key(&base).hold_s / 10.0, "(b) moves the listing hold");
        assert!(within(cache.probe_reads_per_s, 0.0, 0.0), "a warm cached listing reads nothing");
        assert!(within(cache.ttl_reads_per_s, base.ttl_reads_per_s, 1e-9), "not the refill reads");
        assert!(within(cache.cpu_index, base.cpu_index, 1e-9), "not the drains");
        assert!(key(&budget).stall_s < key(&base).stall_s / 10.0, "the budget moves the stall");
        assert!(within(budget.ttl_reads_per_s, base.ttl_reads_per_s, 1e-9), "not the reads per minute");
        assert!(within(budget.rho_lock, base.rho_lock, 1e-9), "nor the lock's utilisation");
        assert!(removed.ttl_reads_per_s < base.ttl_reads_per_s, "removing the TTL moves the refills");
        assert!(within(removed.cpu_index, base.cpu_index, 1e-9));
        // A rebuild from the rows matters on the Global planes only: every Key subject has rows.
        assert!(rows.key_planes[3].stall_s < base.key_planes[3].stall_s, "the occurrence-withdrawal stall shortens");
        // (it reads every Key subject plus the one row listing, so it is no shorter)
        assert!(key(&rows).stall_s >= key(&base).stall_s, "the Key stall does not shorten");
        // a + b x subjects
        let two = run(&p, &Scenario::one_seat(2.0 * d, RoundShape::WholePlane, ReadSet::AsShipped)).seat;
        let walk = |g: &KeyPlaneLoad| p.get(C::KeyPlaneWalkBaseS) + p.get(C::KeyPlaneWalkPerSubjectS) * g.subjects;
        assert!(within(key(&two).subjects, 2.0 * key(&base).subjects, 1e-9));
        assert!(within(walk(&key(&two)) - walk(&key(&base)), p.get(C::KeyPlaneWalkPerSubjectS) * key(&base).subjects, 1e-6));
    }

    /// The source facts the key-plane terms rest on, at seat-1 size: the Global planes walk every
    /// key the seat holds (8e46165 bridge.rs :13152), Key and occurrence the publish selector; a
    /// warm Key listing is ~3 to 6 ms and a listing-cache hit ~0.2 ms; a v44.1.2 Key refill
    /// stalls ~16 s and an occurrence refill 11 to 22 s at ~2 ms a read; the budget caps a stall
    /// at ~0.5 s.
    #[test]
    fn the_key_planes_follow_the_source_answer() {
        let p = CostParams::calibrated();
        let mut mix = measured_mix(&p, window_b::ROUNDS, window_b::REV_ROUNDS, window_b::WITHHOLDS);
        mix.refills_booked = true;
        let l = ledger(&p, &mix, ReadSet::AsShipped);
        let [k, io, rev, _] = l.key_planes;
        assert!(within(k.subjects, 8_211.0, 1e-9) && io.subjects > 5_600.0 && io.subjects < 5_700.0);
        assert!(rev.subjects >= p.get(C::KeySubjectsAtSeat) && rev.subjects <= p.get(C::SeatKeyRows), "Global = own ∪ cohort = every key: {}", rev.subjects);
        assert!(k.hold_s > 3.0e-3 && k.hold_s < 6.0e-3, "warm Key listing {} s", k.hold_s);
        assert!(k.stall_s > 15.0 && k.stall_s < 17.0, "Key refill {} s", k.stall_s);
        let t = ttl_check(&p);
        assert!(t.io_stall_s[0] > 10.0 && t.io_stall_s[1] < 23.0, "{:?}", t.io_stall_s);
        assert!(t.mean_per_min[0] > 800.0 && t.mean_per_min[0] < 840.0, "Key ~820 a minute");
        let mut cached = mix.clone();
        cached.listing_fix = ListingFix { cache: true, budget: true };
        let c = ledger(&p, &cached, ReadSet::AsShipped);
        // with no refill and no key:record row landing, every listing is a hit
        let mut quiet = cached.clone();
        quiet.refills_booked = false;
        let h = ledger(&p, &quiet, ReadSet::AsShipped);
        assert!(within(h.key_planes[0].hold_s, 0.2e-3, 1e-9), "a hit: {}", h.key_planes[0].hold_s);
        // a revalidation epoch misses ⌈8,211 / 256⌉ = 33 listings of the 10 a minute
        assert!(c.key_planes[0].hold_s > h.key_planes[0].hold_s && c.key_planes[0].hold_s < k.hold_s, "{}", c.key_planes[0].hold_s);
        assert!(c.key_planes[0].stall_s > 0.4 && c.key_planes[0].stall_s < 0.6, "budget: {}", c.key_planes[0].stall_s);
        assert!(within(c.ttl_reads_per_s, l.ttl_reads_per_s, 1e-9), "the budget leaves the reads per minute");
        // A v45 no-consent device lists only Revocation: Key and occurrence listings at the
        // seat are today's peers' alone, whatever the fleet (its Pulls read outside the lock).
        let v45 = run(&p, &Scenario::one_seat(50_000.0, RoundShape::RequiredSet, ReadSet::AsShipped)).seat;
        assert!(within(v45.key_planes[0].listings_per_s, window_b::ROUNDS[2] / 60.0, 1e-9));
        assert!(v45.key_planes[2].listings_per_s > 1_000.0);
        assert!(v45.pull_permits_per_s > 0.0);
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
        // Whole-plane rounds list every key plane every 30 s per device over walks that grow
        // with the fleet: after read minimisation a key-plane lock is what binds (the Global
        // planes walk every key the seat holds, so any of the four can).
        let w = Scenario::one_seat(1.0, RoundShape::WholePlane, ReadSet::AfterReadMin);
        let mut at = w;
        at.devices = knee(&p, &w);
        assert!(matches!(binding(&p, &at), Binding::Lock(_)), "{:?}", binding(&p, &at));
    }

    /// The cold restart is single flight under the lock: each listed plane reads every subject
    /// once; a plane nobody lists is not read; the budget caps the stall.
    #[test]
    fn a_cold_restart_reads_each_listed_plane_once() {
        let p = CostParams::calibrated();
        let sc = Scenario::one_seat(10_000.0, RoundShape::RequiredSet, ReadSet::AsShipped);
        let c = cold_restart(&p, &sc);
        let listed: f64 = KEY_PLANES
            .into_iter()
            .enumerate()
            .filter(|(i, _)| *i != 3)
            .map(|(i, k)| c.subjects[i] * p.queries_per_subject(k))
            .sum();
        assert!(within(c.store_reads, listed, 1e-9), "IOR is not listed under v45");
        assert!(c.stall_s > 0.0 && c.rounds_waiting > 0.0);
        // the Revocation walk is every key the seat holds, so it can stall longest
        assert!(matches!(c.stall_plane, Plane::Key | Plane::IdentityOccurrence | Plane::Revocation), "{:?}", c.stall_plane);
        let steady = run(&p, &sc);
        assert!(steady.seat.ttl_reads_per_s < c.store_reads, "the steady run books the mean, not the burst");
        let b = cold_restart(&p, &sc.with_candidate());
        assert!(b.stall_s <= p.get(C::ListingBudgetK) * p.get(C::StoreReadS) + 1e-12);
    }

    /// The TTL refill is reported against the windows, not fitted: the calibration windows
    /// book no refill, so the fitted CPU parameter does not move with the refill parameters,
    /// and the residuals stay UNEXPLAINED.
    #[test]
    fn the_ttl_refill_is_reported_not_fitted() {
        let mut p = CostParams::default();
        let c0 = calibrate(&p);
        p.set(C::IoQueriesPerSubject, 50.0);
        let c1 = calibrate(&p);
        assert!(within(c1.cpu_per_byte_s, c0.cpu_per_byte_s, 1e-12));
        assert!(within(c1.permit_residual[2], c0.permit_residual[2], 1e-12));
        let r = ttl_check_report(&CostParams::default());
        assert!(r.contains("not proven") && r.contains("UNEXPLAINED"));
        let t = ttl_check(&CostParams::default());
        assert!(t.key_burst > c0.permit_residual[1], "a whole Key burst is larger than A''s residual");
    }

    /// Old peers bring the occurrence-withdrawal listing back and add Key listings: with none,
    /// the IOR plane is not listed under v45 (no lock, no refill); with some, it is, and the
    /// seat costs more. Plane clears default to the measured 0.
    #[test]
    fn old_peers_bring_the_second_global_plane_back() {
        let mut p = CostParams::calibrated();
        assert!(within(p.get(C::RevPlaneClearsPerHour), 0.0, 0.0));
        assert!(within(p.get(C::IorPlaneClearsPerHour), 0.0, 0.0));
        let sc = Scenario::one_seat(10_000.0, RoundShape::RequiredSet, ReadSet::AsShipped);
        let none = run(&p, &sc).seat;
        assert!(within(none.key_planes[3].listings_per_s, 0.0, 0.0), "IOR not listed under v45");
        assert!(within(none.key_planes[3].refill_reads_per_s, 0.0, 0.0), "so never refilled");
        p.set(C::OldPeerShare, 0.25);
        let some = run(&p, &sc).seat;
        assert!(some.key_planes[3].listings_per_s > 0.0);
        assert!(some.key_planes[3].refill_reads_per_s > 0.0);
        assert!(some.key_planes[0].listings_per_s > none.key_planes[0].listings_per_s);
        assert!(some.rho_lock >= none.rho_lock);
        assert!(key_planes_today_table(&CostParams::calibrated()).contains("v44 today (MEASURED rounds"));
        assert!(mixed_fleet_table(&CostParams::calibrated()).contains("| 0.25 |"));
    }

    /// The withheld-row reading of the permit counter is false (edge source read 2026-11-10 at
    /// v44.1.2: no withhold site takes a permit) and is dropped from the model and the report;
    /// grant reads follow withholds, and the read fix cuts them to about one per local key per
    /// listing (~50 a minute at window B).
    #[test]
    fn no_withhold_takes_a_permit_and_the_read_fix_cuts_grant_reads() {
        let p = CostParams::calibrated();
        assert!(within(window_a::CONSENT, window_a::WITHHOLDS, 0.02));
        let r = report_markdown(&p);
        assert!(r.contains("is dropped") && !r.contains("withheld-row hypothesis"));
        let o = run(&p, &Scenario::one_seat(0.0, RoundShape::RequiredSet, ReadSet::AfterReadMin));
        assert!(within(o.seat.consent_per_s * 60.0, 50.0, 1e-9), "{}", o.seat.consent_per_s * 60.0);
        let shipped = run(&p, &Scenario::one_seat(0.0, RoundShape::RequiredSet, ReadSet::AsShipped));
        assert!(within(shipped.seat.consent_per_s * 60.0, window_b::WITHHOLDS * p.get(C::ConsentPerRefusedRow), 1e-9));
        // the fix removes no permit; its store feed adds one per probe and per drained row
        // (8e46165 observability.rs SWEEP_PERMIT_FEED_*: feed reads take a sweep permit)
        assert!(o.seat.feed_reads_per_s > 0.0);
        assert!(
            within(o.seat.listing_permits_per_s - o.seat.feed_reads_per_s, shipped.seat.listing_permits_per_s, 1e-12),
            "the fix removes no permit"
        );
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

    /// The Global planes walk every key the seat holds and grow with the publish set: at g = 0
    /// they stay at today's size, at g = 1 they grow with announced nodes, like Key and IO.
    #[test]
    fn the_global_set_is_every_key_and_grows_by_the_publish_set() {
        let mut p = CostParams::calibrated();
        let n0 = p.get(C::AnnouncedNodes);
        assert!(within(p.plane_subjects(Plane::Revocation, n0), p.get(C::SeatKeyRows), 0.0));
        p.set(C::KeyPublishSetGrowth, 0.0);
        assert!(within(p.plane_subjects(Plane::Revocation, 10.0 * n0), p.get(C::SeatKeyRows), 1e-12));
        assert!(within(p.plane_subjects(Plane::Key, 10.0 * n0), p.get(C::KeySubjectsAtSeat), 1e-12));
        p.set(C::KeyPublishSetGrowth, 1.0);
        assert!(within(p.plane_subjects(Plane::Revocation, 10.0 * n0), 10.0 * p.get(C::SeatKeyRows), 1e-12));
        p.set(C::KeyPublishSetGrowth, 0.5);
        assert!(within(p.plane_subjects(Plane::IdentityOccurrence, 3.0 * n0), 2.0 * p.get(C::IoSubjectsAtSeat), 1e-12));
    }

    /// At 8e46165 a v45 device's Revocation round is a listing-cache hit unless the memo moved:
    /// it holds the plane's lock for the set comparison (per subject), builds the subject set
    /// before the lock, and reads nothing. A revalidation epoch misses ⌈subjects / K⌉ listings,
    /// and each key:record row landing misses one.
    #[test]
    fn a_v45_revocation_round_is_a_cache_hit_that_scales_with_the_set() {
        let p = CostParams::calibrated();
        let n = 10_000.0;
        let o = run(&p, &Scenario::v45(n)).seat;
        let rev = o.key_planes[2];
        let hit = p.get(C::WarmListingS) * rev.subjects / p.get(C::KeySubjectsAtSeat);
        let walk = p.get(C::KeyPlaneWalkBaseS) + p.get(C::KeyPlaneWalkPerSubjectS) * rev.subjects;
        let refill_listings = (rev.subjects / p.get(C::ListingBudgetK)).ceil() / p.get(C::KeyMemoExpiryS);
        let dirt = n * p.get(C::KeyRecordWritesPerDeviceDay) / 86_400.0;
        let misses = refill_listings + dirt;
        let expect = (misses * walk + (rev.listings_per_s - misses) * hit) / rev.listings_per_s;
        assert!(within(rev.hold_s, expect, 1e-9), "{} vs {expect}", rev.hold_s);
        assert!(rev.probe_reads_per_s == 0.0, "a hit reads nothing; the feed is booked once");
        assert!(o.cpu_set_build > 0.0);
        assert!(within(o.cpu_set_build / (rev.listings_per_s * rev.subjects), p.get(C::SubjectSetBuildPerSubjectS), 0.2));
        // no stall longer than K reads
        assert!(rev.stall_s <= p.get(C::ListingBudgetK) * p.get(C::StoreReadS) + 1e-12);
    }

    /// The reach rule (6244795) adds reads only where it is in the code, and a cleared reach
    /// memo costs a whole re-walk on the next identity-plane listing.
    #[test]
    fn the_reach_rule_adds_seat_reads() {
        let mut p = CostParams::calibrated();
        let mut off = Scenario::v45(1_349.0);
        off.reach_rule = false;
        let without = run(&p, &off).seat;
        let with = run(&p, &Scenario::v45(1_349.0)).seat;
        assert!(within(without.reach_reads_per_s, 0.0, 0.0));
        assert!(with.reach_reads_per_s > 0.0);
        assert!(with.other_reads_per_s > without.other_reads_per_s);
        // at today's fleet: the walk part is unowned subjects x reads / 30 s
        let walk = p.get(C::ReachUnownedShare) * p.get(C::IoSubjectsAtSeat) * p.get(C::ReachReadsPerUnownedSubject)
            * (1.0 / 30.0 + p.get(C::AnnounceMemoClearsPerHour) / 3_600.0);
        assert!(with.reach_reads_per_s > walk, "{} vs {walk}", with.reach_reads_per_s);
        eprintln!("reach-rule reads at 1,349 devices: {:.1}/s (audit hand figure 127 to 152)", with.reach_reads_per_s);
        p.set(C::AnnounceMemoClearsPerHour, 3_600.0);
        assert!(run(&p, &Scenario::v45(1_349.0)).seat.reach_reads_per_s > with.reach_reads_per_s);
    }

    /// A seat initiates to devices only when the server names them: zero by default, and when
    /// set the seat lists on five planes per named device per cadence and receives the replies.
    #[test]
    fn the_seat_initiator_term_is_the_servers_choice() {
        let mut p = CostParams::calibrated();
        assert!(within(p.get(C::SeatInitiatorDeviceShare), 0.0, 0.0));
        let sc = Scenario::v45(10_000.0);
        let none = run(&p, &sc).seat;
        p.set(C::SeatInitiatorDeviceShare, 0.01);
        let some = run(&p, &sc).seat;
        assert!(some.key_planes[0].listings_per_s > none.key_planes[0].listings_per_s);
        assert!(within(some.key_planes[0].listings_per_s - none.key_planes[0].listings_per_s, 100.0 / 30.0, 1e-9));
        assert!(some.summaries_per_s > none.summaries_per_s);
    }

    /// A claimed device's owner legs take a permit each (the measured 8 are an unclaimed
    /// device's), and the Pull legs of every device carry the reach rule's reads.
    #[test]
    fn owner_legs_take_permits() {
        let mut p = CostParams::calibrated();
        p.set(C::ClaimedShare, 0.0);
        let a = run(&p, &Scenario::v45(6_000.0)).seat.pull_permits_per_s;
        p.set(C::ClaimedShare, 1.0);
        let b = run(&p, &Scenario::v45(6_000.0)).seat.pull_permits_per_s;
        assert!(within(b - a, 6_000.0 * 3.0 / 600.0, 1e-9), "{}", b - a);
    }

    #[test]
    fn report_renders() {
        let r = report_markdown(&CostParams::calibrated());
        assert!(r.contains("Calibration") && r.contains("1M devices"));
        assert!(!r.contains("NaN"));
    }
}
