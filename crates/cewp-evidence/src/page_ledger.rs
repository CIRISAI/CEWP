//! The public twin page's claims ledger (`docs/evidence/claims.json`).
//!
//! One entry per figure or statement of fact the page (`docs/globe.html`, `docs/index.html`, both
//! built by `docs/globe.js`) shows. The page marks each figure with `data-claim="<id>"`; a click or
//! a keyboard press on it opens the entry. Every entry carries its FSD §8.2 class, what supports it,
//! the interval where one exists, how it can be wrong, and how to reproduce it.
//!
//! The ledger lives here, as data the crate can validate, and the two files under `docs/evidence/`
//! are generated from it (`cargo run -p cewp-evidence --example page_ledger -- --write`). The tests
//! in this module fail when an entry would be refused by [`crate::validate_epistemic_integrity`],
//! when the committed files differ from what this table generates, and (in [`crate::page_scan`])
//! when the page shows a figure with no entry.
//!
//! Sources name the file and symbol a value is read from, at the CEWP commit [`CEWP_BASE`] unless
//! the entry names another; measured entries name their window and hold aggregates only.
//! An entry whose support could not be found is `FRONTIER` with a blocker that says what is missing.

use crate::{validate_epistemic_integrity, EpistemicClass, TaggedMetric};

/// CEWP commit every `crates/` and `docs/` source in this ledger was read at.
pub const CEWP_BASE: &str = "f73b918";

/// Schema tag of the generated ledger.
pub const LEDGER_SCHEMA: &str = "cewp-page-claims/v1";

/// The command that regenerates `docs/evidence/claims.json` and `docs/evidence/claims.js`.
pub const GENERATE_CMD: &str = "cargo run -p cewp-evidence --example page_ledger -- --write";

/// Whether the page may state a figure as it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Stated as the entry describes.
    Stated,
    /// A correction is pending; the page shows the figure marked "under correction".
    UnderCorrection,
}

impl Status {
    /// Token used in the generated ledger.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stated => "stated",
            Self::UnderCorrection => "under_correction",
        }
    }
}

/// An interval `[lo, hi]` and what it is (sensitivity range, measured spread, ...).
#[derive(Debug, Clone, Copy)]
pub struct Interval {
    /// Lower bound.
    pub lo: f64,
    /// Upper bound.
    pub hi: f64,
    /// What the interval is and its unit.
    pub what: &'static str,
}

/// One ledger entry.
#[derive(Debug, Clone, Copy)]
pub struct PageClaim {
    /// Stable id, referenced by `data-claim` on the page. Lowercase letters, digits and `-`.
    pub id: &'static str,
    /// FSD §8.2 class.
    pub class: EpistemicClass,
    /// What the page shows (the text, or for a live figure what it computes).
    pub shows: &'static str,
    /// The page computes the value at run time.
    pub live: bool,
    /// File and symbol (and upstream citation) the value is read from or computed by.
    pub source: &'static str,
    /// Commit of `source` (CEWP unless the source names another repository).
    pub commit: &'static str,
    /// MEASURED: the measurement window (aggregates only).
    pub window: &'static str,
    /// MODEL: the parameter name(s) the value rests on.
    pub param: &'static str,
    /// Interval, where one exists.
    pub interval: Option<Interval>,
    /// FRONTIER: what is missing and what would resolve it.
    pub blocker: &'static str,
    /// Known conditions under which the figure is wrong or misleading.
    pub failure: &'static [&'static str],
    /// A command that reproduces the figure, where one exists.
    pub reproduce: &'static str,
    /// Stated, or under correction.
    pub status: Status,
    /// When under correction: what is being corrected and by whom.
    pub correction: &'static str,
}

/// The globe package's commit the layer entries were read at (branch twin/globe).
const LAYERS_COMMIT: &str = "twin/globe 07865ef";
/// Sources of the Phase 2 layers (package view functions, merged on twin/globe).
const VIEWS_COMMIT: &str = "twin/globe 8b70422";
/// The calibration layer: package source at twin/calibration, wiring at twin/globe.
const CAL_LAYER_COMMIT: &str = "twin/calibration c13cee5; twin/globe e42b504";

const BASE: PageClaim = PageClaim {
    id: "",
    class: EpistemicClass::Frontier,
    shows: "",
    live: false,
    source: "",
    commit: CEWP_BASE,
    window: "",
    param: "",
    interval: None,
    blocker: "",
    failure: &[],
    reproduce: "",
    status: Status::Stated,
    correction: "",
};

use EpistemicClass::{Derived, Frontier, Measured, Model};

/// twin-calibration's correction list (twin/calibration 1b1d8a2, FSD/twin/calibration.md §4 and §10).
const CALIBRATION_COMMIT: &str = "twin/calibration c13cee5 (CEWP); release/45.0.0 8e46165 (CIRISEdge, untagged when read)";
/// The live seat figures still come from the f73b918 kernel until the globe package rebuilds it on
/// the calibrated model.
const SEAT_KERNEL_CORRECTION: &str = "These live figures are computed by the kernel built at f73b918, whose seat model counts the Global planes at 30 subjects (8,211 to 8,698 at 8e46165), books no reach-rule reads and no send-set floor, and uses point inputs. twin-calibration (1b1d8a2) replaces it; the figures here follow when the kernel is rebuilt on that model. The knee on this card is already the calibrated interval.";

/// Every figure or statement of fact the page shows, by id.
pub const PAGE_CLAIMS: &[PageClaim] = &[
    // ------------------------------------------------------------------ MEASURED: seat-1, 2026-11-10
    PageClaim {
        id: "seat1-windows",
        class: Measured,
        shows: "The serve seat's costs are calibrated on the production CIRIS mesh on 2026-11-10: four 60-second windows of the serve node's own counters, one of them with CPU, plus its store composition and announced-node count.",
        source: "crates/cewp-sim/src/capacity.rs: COST_SPECS rows tagged MEASURED, mod window_a, mod window_b; FSD/SIMULATION_ENGINE.md §4.7",
        window: "2026-11-10, seat-1 (the one production serve seat), four 60 s counter windows; window B carries process CPU. Aggregate counts only.",
        failure: &[
            "One seat on one day: quiet-hour samples, not a diurnal or peak measurement.",
            "The counters are the serve node's own; nothing outside it checked them.",
            "Calibrated at a fleet of about 1,349 announced nodes; every figure beyond that fleet is MODEL.",
        ],
        reproduce: "cargo test -p cewp-sim capacity",
        ..BASE
    },
    PageClaim {
        id: "seat1-cpu-quiet",
        class: Measured,
        shows: "0.65 of a core in a quiet 60 s window (seat-1).",
        source: "crates/cewp-sim/src/capacity.rs: window_b::CPU_CORES = 0.652",
        window: "seat-1 window B, 2026-11-10, 60 s, 15 inbound links, process utime + stime.",
        failure: &[
            "A single 60 s sample: no interval exists.",
            "Process CPU includes server-side work that is not an edge cost (MEASURED-CONTEXT rows in COST_SPECS).",
        ],
        reproduce: "cargo test -p cewp-sim today_peers_alone_reproduce_window_b",
        ..BASE
    },
    PageClaim {
        id: "seat1-att-queries",
        class: Measured,
        shows: "About 329 attestation queries per second (seat-1).",
        source: "crates/cewp-sim/src/capacity.rs: window_b::ATT_QUERIES = 19,754 over window_b::SECONDS = 60",
        window: "seat-1 window B, 2026-11-10, 60 s. Window A counted 19,550 (326/s).",
        failure: &["Two quiet windows; a busy hour was not sampled."],
        ..BASE
    },
    PageClaim {
        id: "seat1-rounds",
        class: Measured,
        shows: "About 10 to 12 replication rounds per minute per plane (quiet window).",
        source: "crates/cewp-sim/src/capacity.rs: window_b::ROUNDS = [10, 11, 10, 11] (attestation, occurrence, key, route)",
        window: "seat-1 window B, 2026-11-10, 60 s.",
        failure: &[
            "Window B records 10 to 11 per plane; the upper figure of 12 is not in window B and this ledger has not traced it to another window.",
            "Revocation-plane rounds were 0 in all four windows.",
        ],
        ..BASE
    },
    PageClaim {
        id: "seat1-store",
        class: Measured,
        shows: "A store of about 42.5k attestations, 18.3k transport destinations and 8.7k keys (seat-1).",
        source: "crates/cewp-sim/src/capacity.rs: COST_SPECS seat_att_rows = 42,520, seat_td_rows = 18,313, seat_key_rows = 8,698",
        window: "seat-1 store counts, window A, 2026-11-10.",
        failure: &["A store snapshot; it grows with the fleet."],
        ..BASE
    },
    PageClaim {
        id: "seat1-announced",
        class: Measured,
        shows: "1,349 announced nodes under 1,338 owners (seat-1).",
        source: "crates/cewp-sim/src/capacity.rs: COST_SPECS announced_nodes = 1,349, announced_owners = 1,338",
        window: "seat-1 relay report, 2026-11-10.",
        failure: &["Announced nodes, not active users: a node that announced and went quiet still counts."],
        ..BASE
    },
    PageClaim {
        id: "seat1-no-revocations",
        class: Measured,
        shows: "0 key-revocation rows at seat-1.",
        source: "crates/cewp-sim/src/capacity.rs: COST_SPECS rev_plane_clears_per_hour = 0, rev_non_empty_subjects = 0",
        window: "seat-1 store, 2026-11-10: no key:revocation:v1 row ever held.",
        failure: &["A young fleet: revocations will appear as keys rotate, and the Revocation plane's cost with them."],
        ..BASE
    },
    PageClaim {
        id: "pull-cost-debug",
        class: Measured,
        shows: "The idle Pull's cost was measured on an edge debug build; a release rerun is pending.",
        source: "crates/cewp-sim/src/capacity.rs: COST_SPECS pull_calls_per_cycle, pull_permits_per_cycle, pull_reply_bytes_per_cycle, drain_page_*_s (tag MeasuredDebug)",
        window: "edge debug build, in-memory store, wall time, one sample per point (contact-4413's idle-round witness at edge 26efbc4).",
        failure: &["A debug build on an in-memory store: release-build cost on a disk store is not yet measured."],
        ..BASE
    },
    // ------------------------------------------------------------------ MEASURED: this browser
    PageClaim {
        id: "browser-kernel-time",
        class: Measured,
        live: true,
        shows: "Milliseconds per tick, rulebook checks per second, and the per-tick estimate on the acts buttons: the kernel's wall time in your browser.",
        source: "docs/globe.js: tick() (performance.now around tickWasm / tickJs), updateWorkloadLimits()",
        window: "This browser, this page load: each tick's wall time, smoothed 0.6 old / 0.4 new.",
        failure: &[
            "Measures your device, not the mesh.",
            "Includes garbage collection and tab throttling; a background tab reads slow.",
        ],
        ..BASE
    },
    PageClaim {
        id: "browser-tiling-stats",
        class: Measured,
        live: true,
        shows: "The street tiling's acuity ring, frame rate, leaves drawn, queue and cache size.",
        source: "docs/tiling.js: TL.stats(), written to #s_tiling",
        window: "This browser, live renderer counters.",
        failure: &["Renderer statistics of your device; they say nothing about the mesh."],
        ..BASE
    },
    // ------------------------------------------------------------------ DERIVED: read from source
    PageClaim {
        id: "budget-class-rates",
        class: Derived,
        shows: "Shaped budget classes: server 7,000 B/s, home 2,800 B/s, phone-wifi 1,400 B/s, phone-cellular 350 B/s, seat 33,000 B/s; rho = 0.5; window W = 600 s (3,600 s on cellular).",
        source: "crates/cewp-sim/src/transport.rs: BudgetClass::default_bytes_per_s, BudgetClass::rho, BudgetClass::window_secs (CC rc17 5.4.3; CIRISEdge src/transport/link_shaping.rs); docs/globe.js MCLASS_BPS",
        failure: &[
            "A CEWP copy of edge's class table: if edge changes it, this drifts until transport.rs is updated.",
            "Nominal rates; the real payload share is rho x rate (BudgetClass::real_share_bytes_per_s).",
        ],
        reproduce: "cargo test -p cewp-sim transport",
        ..BASE
    },
    PageClaim {
        id: "shaped-link-bound",
        class: Derived,
        shows: "A node shapes at most 8 device links (L = 8); each reachable home or server that relays adds L - 1 = 7 shaped slots; the helper evens out traffic for up to eight devices.",
        source: "crates/cewp-sim/src/transport.rs: SHAPED_LINK_BOUND = 8 (Tiers.bound, CC 5.4.3); CIRISConstitution formal/transport_tiers/Tiers.lean: each_home_adds_seven, reach_ne_capacity",
        failure: &["The seat class sits outside the bound of eight (seat-to-seat links); the bound counts device-tier links only."],
        reproduce: "cargo test -p cewp-sim shaped_link_bound",
        ..BASE
    },
    PageClaim {
        id: "shaped-link-one-hop",
        class: Derived,
        shows: "A shaped link is one direct hop and is never forwarded; peers with no direct path carry the same classes on ordinary links.",
        source: "CC rc17 5.4.3 (\"A shaped link is one hop\"); crates/cewp-sim/src/transport.rs module doc; CIRISEdge src/transport/reticulum.rs link_is_forwarded",
        failure: &["A one-hop link is direct only when a direct path exists: behind hard NAT the peer gets no shaped link at all."],
        ..BASE
    },
    PageClaim {
        id: "shaping-scope",
        class: Derived,
        shows: "Traffic is sent at a steady pace, so private conversations blend in with everything else.",
        source: "CC rc17 5.4.3 link shaping; crates/cewp-sim/src/micro.rs PARAM_SPECS home_shaping_opt_in, phone_shaping_opt_in (shaping is off by default on home and phones: CIRISEdge FLEET_DEFAULTS)",
        failure: &[
            "Only shaped links run at a steady pace: on by default for servers and seats, opt-in for homes and phones. An unshaped link's timing is visible.",
            "Rows over 64 KiB and live A/V never ride a shaped link.",
        ],
        ..BASE
    },
    PageClaim {
        id: "shaping-defaults",
        class: Derived,
        shows: "Link shaping defaults: ON for server, OFF for home and phones. Relay: ON by default for servers and homes.",
        source: "crates/cewp-sim/src/micro.rs: module doc and PARAM_SPECS home_shaping_opt_in / phone_shaping_opt_in (CIRISEdge FLEET_DEFAULTS, link_shaping.rs); relay policy CC 5.4.6 item 1, CIRISEdge src/onion/policy.rs device_relays",
        failure: &["The slider positions next to these defaults (95%, 25%, 10%, 90%) are scenario inputs, not measured opt-in rates."],
        ..BASE
    },
    PageClaim {
        id: "cover-volume",
        class: Derived,
        shows: "Continuous cover per link per month: about 18.1 GB (server), 7.2 GB (home), 3.6 GB (phone-wifi), 0.9 GB (phone-cellular); about 7 GB a month at 8 cellular links.",
        source: "Arithmetic on budget-class-rates: rate x 86,400 s x 30 days, decimal GB (crates/cewp-sim/src/transport.rs BudgetClass::default_bytes_per_s)",
        failure: &[
            "Assumes the link stays open all month at the nominal rate; a phone that sleeps closes its links and sends nothing.",
            "30-day month.",
        ],
        reproduce: "python3 -c \"print([r*86400*30/1e9 for r in (7000, 2800, 1400, 350)])\"",
        ..BASE
    },
    PageClaim {
        id: "frame-interval",
        class: Derived,
        shows: "One frame every ~2.3 s (server), ~5.8 s (home), 11.6 to 46.5 s (phones) at the 16 KB TCP MDU.",
        source: "Arithmetic: MDU_TCP = 16,272 B (crates/cewp-sim/src/transport.rs) / BudgetClass::default_bytes_per_s",
        failure: &[
            "At the TCP MDU. Over base Reticulum (MDU_RETICULUM = 400 B) a frame takes 0.06 to 1.1 s.",
            "At the nominal rate, not the rho share.",
        ],
        reproduce: "python3 -c \"print([16272/r for r in (7000, 2800, 1400, 350)])\"",
        ..BASE
    },
    PageClaim {
        id: "payload-limit",
        class: Derived,
        shows: "Objects over 64 KiB never ride a shaped link (they go unshaped: ALM, bulk).",
        source: "crates/cewp-sim/src/transport.rs: SHAPED_PAYLOAD_LIMIT_BYTES = 65,536 (CC 5.3.3.2)",
        failure: &["The live count beside it (refused on shaped) is a kernel output over the kernel's own payload mix (MODEL)."],
        reproduce: "cargo test -p cewp-sim shaped_link_payload_boundary_sixty_four_kib",
        ..BASE
    },
    PageClaim {
        id: "onion-tier",
        class: Derived,
        shows: "Anonymous mode: 3-hop X-Wing onion walks; 15,296 B onion packets replace cover frames at 0 extra wire bytes; at least 3 answering relays per layer.",
        source: "crates/cewp-sim/src/transport.rs: CONP_FRAME_SIZE = 15,296, CONP_EXTRA_BYTES = 0, ONION_TIER_FLOOR_RELAYS = 3 (CC 5.4.6)",
        failure: &["Zero extra bytes holds only where a standing shaped link has a cover frame to replace."],
        ..BASE
    },
    PageClaim {
        id: "max-referrals",
        class: Derived,
        shows: "A relay discloses its signed route and locality to at most 3 referred strangers.",
        source: "crates/cewp-sim/src/transport.rs: MAX_REFERRALS = 3 (CC 5.4.3)",
        failure: &["Per refusal; a node refused many times refers many times."],
        ..BASE
    },
    PageClaim {
        id: "no-displacement",
        class: Derived,
        shows: "Edge v45 has no displacement by rank (CIRISEdge#939).",
        source: "crates/cewp-sim/src/transport.rs module doc (CIRISEdge src/transport/link_shaping.rs:356)",
        failure: &["Changes when CIRISEdge#939 ships; the live count beside it is a kernel output (MODEL)."],
        ..BASE
    },
    PageClaim {
        id: "punch-budget",
        class: Derived,
        shows: "A direct-path punch gets a 20 s budget.",
        source: "crates/cewp-sim/src/micro.rs module doc (CIRISEdge src/transport/direct_path.rs: 20 s budget, 1 h back-off)",
        failure: &["Whether a punch succeeds is FRONTIER (punch_success_share, cellular_punchable_share)."],
        ..BASE
    },
    PageClaim {
        id: "key-plane-refill",
        class: Derived,
        shows: "Each key plane's listing holds one lock; the 10-minute TTL now revalidates at most 256 reads a listing (stale-while-revalidate), so no listing holds the lock for a whole refill.",
        source: "FSD/twin/calibration.md §1 and §3 (key_plane_memo.rs KEY_PLANE_READ_BUDGET = 256 :296); crates/cewp-sim/src/capacity.rs key-plane lock terms",
        commit: CALIBRATION_COMMIT,
        failure: &[
            "How long a listing holds the lock (b) is DERIVED from one warm listing; the probe time is MODEL.",
            "The v44.1.2 burst (the whole plane under the lock) cannot occur at 8e46165, so the measured windows (v44.1.2) do not show the v45 lock.",
        ],
        ..BASE
    },
    PageClaim {
        id: "edge-source-read",
        class: Derived,
        shows: "Device behaviour is derived from edge source: edge release/45.0.0 at 8e46165 (untagged when read 2026-11-11). At that commit the listing cache and read budget are built and on by default; only their timings are MODEL.",
        source: "FSD/twin/calibration.md §1 and §10 (bridge.rs BridgeConfig::default :1220-:1221, key_plane_memo.rs KEY_PLANE_READ_BUDGET = 256 :296, runtime.rs BUNDLE_ROUND_PLANES :1140)",
        commit: CALIBRATION_COMMIT,
        failure: &[
            "Edge moves on: a behaviour changed after 8e46165 is not in the twin until re-read.",
            "8e46165 was untagged when read; the tag may land on a later commit.",
        ],
        ..BASE
    },
    PageClaim {
        id: "all-simulated",
        class: Derived,
        shows: "Everyone and everything on the page is simulated; the globe shows a model, not real people.",
        source: "docs/globe.js buildWorld (cohorts drawn from the population grid with a seeded RNG, seed 20261105); crates/cewp-sim/src/micro.rs (generated neighbourhood)",
        failure: &["The serve seat's cost model is calibrated on real measurements (seat1-windows); the people and devices are not."],
        ..BASE
    },
    PageClaim {
        id: "generated-below-region",
        class: Derived,
        shows: "Real geography is drawn only down to region scale; every street, home and device below it is generated, never placed on a real road or building.",
        source: "FSD/SIMULATION_ENGINE.md §4.6 (\"No real places\"); crates/cewp-sim/src/tiling.rs; docs/tiling.js; docs/kernel/README.md \"Places\"",
        failure: &["Generated streets follow population density and the land mask, so one can fall where a real street also runs: by coincidence, not by data."],
        ..BASE
    },
    PageClaim {
        id: "tier-cohorts",
        class: Derived,
        live: true,
        shows: "Above street zoom the page runs the cohort tier: each dot is a cohort standing for many people (1,000,000 cohorts on a desktop, 100,000 on a phone); only the street patch is simulated 1:1.",
        source: "docs/globe.js: currentSize (1,000,000 / 100,000), buildWorld(n), tickWasm; crates/cewp-sim/src/micro.rs for the 1:1 patch",
        failure: &[
            "The cohort tier is not a CIRISHolon-certified coarse tier: no closure certificate is computed for it (see holon-indicator).",
            "The R1 to R4 tiers named in the FSD are not run on this page.",
        ],
        ..BASE
    },
    PageClaim {
        id: "tier-micro",
        class: Derived,
        live: true,
        shows: "At street zoom the page runs the R5 micro tier: one generated neighbourhood, every device in it a node of its own (cewp_sim::micro::MicroWorld in the wasm kernel).",
        source: "crates/cewp-sim/src/micro.rs: MicroWorld; kernel-wasm/src/lib.rs micro_build; docs/globe.js microFreeRoam",
        failure: &[
            "One patch of about 60 households; everything outside it is cohorts or a drawing, not simulated 1:1.",
            "The streets, homes and devices are generated (generated-below-region).",
        ],
        ..BASE
    },
    PageClaim {
        id: "pop-grid-cells",
        class: Derived,
        shows: "2,961 populated land cells in 181 localities and 9 regions.",
        source: "docs/globe.js: POP_GRID_B64 header (nCells, nLocs) read by initPopGrid; REG",
        failure: &["The counts describe the embedded grid. Where the grid's densities come from is not recorded (pop-grid-source, FRONTIER)."],
        ..BASE
    },
    // ------------------------------------------------------------------ MODEL
    PageClaim {
        id: "devices-per-person",
        class: Model,
        shows: "2.25 devices per person (about 1 phone, 1 home node or laptop or tablet, and 0.25 of a server).",
        source: "docs/globe.js: DEVICES_PER_PERSON (CC 3.3.6.3 self roster as the reasoning)",
        param: "DEVICES_PER_PERSON = 2.25",
        failure: &["No device census behind it; a modelling choice."],
        ..BASE
    },
    PageClaim {
        id: "world-population",
        class: Model,
        live: true,
        shows: "The population and device count of the world on screen (people x 2.25 devices).",
        source: "docs/globe.js: currentPop (the size buttons and the story set it), DEVICES_PER_PERSON",
        param: "currentPop (your choice: 100 M to 8.1 B), DEVICES_PER_PERSON",
        failure: &["A scenario you choose, not an adoption forecast."],
        ..BASE
    },
    PageClaim {
        id: "cohort-world",
        class: Model,
        live: true,
        shows: "How many cohorts carry the population, on how many land cells and localities.",
        source: "docs/globe.js: buildWorld(n) (families of 2 to 5 cohorts; about one community room per 25 cohorts, lognormal size 6 to 400, 90% within a locality, 9% within a region, 1% world; one public room per region and one global)",
        param: "currentSize, buildWorld room constants",
        failure: &[
            "Cohorts, not people: at 5 B people on a desktop one cohort stands for 5,000 people.",
            "The room structure is generated, so where traffic goes follows these constants.",
        ],
        ..BASE
    },
    PageClaim {
        id: "kernel-locality",
        class: Model,
        live: true,
        shows: "Share of rows that never leave the signer's locality, deliveries within a locality, across a locality, across a region, and the mean crossing distance.",
        source: "kernel-wasm/src/lib.rs tick (outputs 1, 5, 6, 7, 8, 16, 17) over the world from docs/globe.js buildWorld",
        param: "act mix 65% family / 30% community room / 4% consent-free / 1% public (docs/kernel/README.md); room locality shares 90/9/1 (buildWorld)",
        failure: &[
            "The locality share is set largely by the generated room mix (90% of community rooms sit in one locality): an input, not a discovery.",
            "A locality is one of 181 grid localities, not a town.",
        ],
        reproduce: "node --test docs/kernel/kernel.test.js",
        ..BASE
    },
    PageClaim {
        id: "kernel-traffic",
        class: Model,
        live: true,
        shows: "Hops and bytes per mesh layer, onion acts, ALM savings, acts signed, deliveries, refusals, wire bytes, cumulative deliveries, and the story's 'each person uses only a little data'.",
        source: "kernel-wasm/src/lib.rs tick (outputs 0 to 4, 9, 12 to 15, 18 to 25); docs/globe.js tick()",
        param: "kernel IMIX: 35% 512 B rows, 30% 16 KiB, 20% 14 KiB traces, 15% 128 KiB bulk; onion 15,296 B; live ALM chunk 64 KiB",
        failure: &[
            "The four mesh-layer byte figures count 4,096 B per hop (docs/globe.js), while the kernel's own wire-byte total uses its payload mix: the two disagree by construction.",
            "Payload shares are kernel constants with no measured traffic behind them.",
        ],
        reproduce: "node --test docs/kernel/kernel.test.js",
        ..BASE
    },
    PageClaim {
        id: "kernel-baselines",
        class: Model,
        live: true,
        shows: "What a broadcast network, a datacenter in the middle, a CDN, an SFU mesh and a hybrid P2P system would move, as multiples of CEWP's bytes.",
        source: "kernel-wasm/src/lib.rs tick (outputs 10, 11, 33 to 35); FSD/SIMULATION_ENGINE.md §4.3 baseline table; docs/globe.js tick() (broadcast x every device)",
        param: "A1 payload x (2 + deliveries); A2 2 or 9 replicas; A3 1.25 + 1.5 x NAT share; broadcast = deliveries x all devices x 4,096 B",
        failure: &[
            "No baseline is simulated: each is a byte multiplier on CEWP's own deliveries.",
            "The multipliers are point choices inside the FSD's ranges, not calibrated.",
        ],
        ..BASE
    },
    PageClaim {
        id: "chunk-custody",
        class: Model,
        live: true,
        shows: "Share of community-room chunks reconstructed in full (layered fidelity), with partial or deleted counts.",
        source: "kernel-wasm/src/lib.rs tick (outputs 26, 27): classify(chunk_reach, 20, 6), revocation 1 in 250",
        param: "chunk reach = deliveries x 28 / room size, capped at 30; thresholds 20 / 6; revocation rate 1/250",
        failure: &[
            "Shows 95.0% when a tick had no community-room act: a fixed default in the kernel, not a result.",
            "No custody network is simulated; reach is a proxy from the delivery count.",
        ],
        ..BASE
    },
    PageClaim {
        id: "server-tier-spec",
        class: Model,
        shows: "Server tier: a 1 TiB / 1 Gbps always-on machine.",
        source: "FSD/SIMULATION_ENGINE.md §8.3 claim 7 (commodity L1 home server, 1 TB / 1 Gbps / 1 core)",
        param: "server_tier_share",
        failure: &["The FSD says 1 TB; the page says 1 TiB. Hardware calibration is FSD Phase 5, not done."],
        ..BASE
    },
    PageClaim {
        id: "trace-size",
        class: Model,
        shows: "A publishable agent trace is about 14 KiB per decision.",
        source: "crates/cewp-sim/src/workload.rs (H3ERE agent traces ~14 KiB = 14,336 B); kernel-wasm/src/lib.rs IMIX",
        param: "trace payload 14,336 B",
        failure: &["One fixed size; no measured trace-size distribution."],
        ..BASE
    },
    PageClaim {
        id: "micro-generated",
        class: Model,
        live: true,
        shows: "The street patch's density, size and device mix, and every distance in it: how far Bob lives, how far the helper seat or the test computer is.",
        source: "crates/cewp-sim/src/micro.rs: MicroWorld build; PARAM_SPECS density_sigma, target_households, persons_per_household, phones_per_person, laptops_per_person, home_node_household_share, seat_km_min, seat_km_max",
        param: "density_sigma 0.9, target_households 60, persons_per_household 2.5, seat_km_min 1,500, seat_km_max 8,000",
        failure: &[
            "The patch's density comes from the population grid, whose source is not recorded (pop-grid-source).",
            "Seat and data-centre distances are drawn between 1,500 and 8,000 km; no real site is used.",
        ],
        reproduce: "cargo test -p cewp-sim micro",
        ..BASE
    },
    PageClaim {
        id: "micro-activity",
        class: Model,
        live: true,
        shows: "Rows sent, direct, shaped, relayed, held and handed over, punches tried and up, replication rounds served, and each device's state in the street patch.",
        source: "crates/cewp-sim/src/micro.rs: MicroWorld step, exported through kernel-wasm micro_buf; docs/globe.js updateFocalChip, renderNodeCard",
        param: "PARAM_SPECS (MODEL rows) plus three FRONTIER rows: reach_home_share, cellular_punchable_share, punch_success_share",
        failure: &[
            "Who can punch to whom rests on three FRONTIER shares (leviculum#100: no field NAT mix yet).",
            "Device rhythms (sleep, Wi-Fi dwell, charging) are MODEL means, not traces.",
        ],
        reproduce: "cargo test -p cewp-sim micro",
        ..BASE
    },
    PageClaim {
        id: "seat-load-model",
        class: Model,
        live: true,
        shows: "Seat load by fleet size (cores, reads per second, saturated or not) on the seat card, with today's measured peer load booked as a fixed baseline.",
        source: "crates/cewp-sim/src/capacity.rs: cap_seat, COST_SPECS; kernel-wasm cap_seat; FSD/SIMULATION_ENGINE.md §4.7",
        param: "COST_SPECS (MEASURED, DERIVED, FITTED, PROVISIONAL and MODEL rows; one fitted CPU-per-byte parameter)",
        failure: &[
            "One queue with Poisson-smooth arrivals; no tokio scheduling, SQLite contention, cache misses or memory (capacity.rs module doc).",
            "Fitted to one CPU window.",
        ],
        reproduce: "cargo run -p cewp-sim --example capacity_sweep",
        status: Status::UnderCorrection,
        correction: SEAT_KERNEL_CORRECTION,
        ..BASE
    },
    PageClaim {
        id: "seat-knee",
        class: Model,
        live: true,
        shows: "v45.0.0 on one 4-core seat at a 70% ceiling: 4,882 to 13,723 devices if the server's publish set grows with the fleet (binds on CPU); 14,253 to 75,879 if it stays at today's size (binds on sweep permits at the low end, CPU at the high end).",
        source: "kernel-wasm views.rs (twin/globe e42b504) view_calibration_knee(0) (g 0) and (4) (g 1), wrapping crates/cewp-sim/src/seat_hooks.rs calibration_view_knee (lower and upper fields only; the central field is not shown); FSD/twin/calibration.md §4 (knee_interval: each term's knee propagated alone by the vertex method, [min lo, min hi] over CPU, sweep permits and the busiest lock); crates/cewp-sim/src/seat_calibration.rs",
        commit: CALIBRATION_COMMIT,
        param: "key_publish_set growth g from 0 (today's size) to 1 (grows with the fleet), FRONTIER",
        interval: Some(Interval { lo: 4_882.0, hi: 75_879.0, what: "devices at the knee, outer bounds over the input intervals for g in [0, 1]; g = 1 gives 4,882 to 13,723, g = 0 gives 14,253 to 75,879; bounds, not a probability" }),
        failure: &[
            "Bounds over stated input ranges, not confidence levels; they assume each term is monotone in each input over its range (checked at the ends, not proven).",
            "The windows ran v44.1.2; every v45 term is read from source, not measured on a v45 seat.",
            "Permit residuals of 678.5, 4,697 and 4,401 a minute (windows B, A', C) are UNEXPLAINED; a v45 seat may carry a term of that size the model does not book.",
            "Pull and drain-page timings are edge debug-build wall times; the release end of their band is MODEL.",
            "Past 1,349 devices every figure is an extrapolation of one seat's model: MODEL with its interval, not PROJECTED (FSD §8.2 reserves PROJECTED for extrapolation from certified lower tiers, and none is certified).",
        ],
        reproduce: "cargo run --release -p cewp-sim --example capacity_sweep (on twin/calibration c13cee5)",
        ..BASE
    },
    PageClaim {
        id: "seat-lock-model",
        class: Model,
        live: true,
        shows: "The busiest key-plane lock's utilisation at this patch's fleet and the subjects it walks. Where the lock binds the knee, seat-knee's interval says so.",
        source: "crates/cewp-sim/src/capacity.rs: key-plane lock model (kernel-wasm cap_seat outputs 15 to 19)",
        param: "listing hold b (DERIVED), probe-page time (MODEL), 10-minute refill",
        failure: &["One server per plane by construction; the probe-page time is MODEL."],
        status: Status::UnderCorrection,
        correction: SEAT_KERNEL_CORRECTION,
        ..BASE
    },
    PageClaim {
        id: "pull-permit-provisional",
        class: Model,
        shows: "v45's figure is PROVISIONAL in two terms: how long a Pull holds its sweep permit (about 2 ms, derived from a key-plane refill, not measured on a Pull) and the Pull's own cost (measured on a debug build).",
        source: "crates/cewp-sim/src/capacity.rs: COST_SPECS store_read_s, pull_calls_per_cycle, pull_permits_per_cycle; report text 'PROVISIONAL in them'",
        param: "store_read_s ~2 ms",
        failure: &["Until a release-build Pull is measured, the v45 knee can move by the ratio of the true hold time to 2 ms (the calibrated knee interval carries this band; FSD/twin/calibration.md §7)."],
        ..BASE
    },
    PageClaim {
        id: "map-rules",
        class: Model,
        shows: "Display rules of this twin: lakes added where ETOPO holds a flat water surface Natural Earth has as land; the street view's ground tint from latitude and ETOPO elevation in five bands; a city's Natural Earth population standing in where the grid has no cell near it.",
        source: "docs/tiling.js: tlTerrain, tlRoot; tools/geo/build_dem.py (lake detection); docs/kernel/README.md \"Places\"",
        param: "lake rule, five ground bands, rock above 2,200 m, 1.6 degree place fallback",
        failure: &[
            "No land-cover or climate data: a desert city and a wet one at the same latitude and height are tinted alike.",
            "A few lakes are missing or in pieces.",
        ],
        ..BASE
    },
    // ------------------------------------------------------------------ FRONTIER: no support found
    PageClaim {
        id: "pop-grid-source",
        class: Frontier,
        shows: "A land-bounded world population density grid.",
        source: "docs/globe.js: POP_GRID_B64 (added in 60b95b6, 2026-11-06)",
        blocker: "The grid's source, licence and build script are not recorded anywhere in the repo (no tool under tools/ builds it; not one of the three approved datasets). Its densities place every cohort and size every street patch. Needs its provenance recorded, or a rebuild from a dataset Eric approves.",
        failure: &["Every density-driven figure inherits this gap."],
        ..BASE
    },
    PageClaim {
        id: "seat-count-rule",
        class: Frontier,
        live: true,
        shows: "Serve seats = max(500, people / 100,000): 50,000 seats at 5 B people.",
        source: "docs/globe.js: updateStatusLine, computeSustainability (const M)",
        blocker: "No source or reasoning for one seat per 100,000 people or the floor of 500. Production runs one seat on purpose (micro.rs serve_seats, steward ruling 2026-11-09).",
        failure: &["Every cohort-tier capacity figure that adds seat slots inherits this rule."],
        ..BASE
    },
    PageClaim {
        id: "nat-leaf-capacity",
        class: Frontier,
        live: true,
        shows: "Reachable-home share, peers with no direct path against k = 2 parent capacity (load and tick or warning), and home parents.",
        source: "docs/globe.js: computeSustainability (maxHardNatLeaves = 36 per seat + 32 per server + 8 per home + 2 per phone)",
        blocker: "The leaves-per-parent constants 36, 32, 8 and 2 have no source; the reach share itself is FRONTIER (leviculum#100 field NAT mix).",
        failure: &["The tick or warning flips on unsourced constants."],
        ..BASE
    },
    PageClaim {
        id: "shaped-slot-supply",
        class: Frontier,
        live: true,
        shows: "Shaped-slot supply against demand (share seated), and how many devices shape a direct one-hop link.",
        source: "docs/globe.js: computeSustainability (supply = 8 per seat + 7 per shaping server or home + 3 per phone; demand = 1.8 per shaping device; onion weights 0.5, 0.3, 0.35)",
        blocker: "Only the 7 is sourced (Tiers.lean each_home_adds_seven). The 8 per seat, 3 per phone, 1.8 slots of demand per device and the onion weights have no source.",
        failure: &["The seated share and its colour rest on unsourced constants."],
        ..BASE
    },
    PageClaim {
        id: "onion-cover",
        class: Frontier,
        live: true,
        shows: "Onion cover ratio, onion peers, and answering onion relays per layer (healthy, small set, or below the floor of 3).",
        source: "docs/globe.js: computeSustainability (nOnionPeers, answeringOnionRelays = seats + relays x onion-relay share, divided by 3 layers)",
        blocker: "The floor of 3 is sourced (onion-tier); the 'healthy at 10 or more' threshold and the population weights are not.",
        failure: &["Inherits seat-count-rule and the unsourced opt-in defaults."],
        ..BASE
    },
    PageClaim {
        id: "holon-indicator",
        class: Frontier,
        live: true,
        shows: "Commuting-square defect, Lipschitz constant K, retention rent G and a within-budget flag, shown in the Holon pill and the Metrics panel.",
        source: "kernel-wasm/src/lib.rs tick (outputs 28 to 32)",
        blocker: "Not a CIRISHolon certificate. The kernel computes a raw locality defect, then scales it by 0.12 x (1 + 0.6 x NAT share + 1.2 x unshaped share), caps it at 0.20 and compares it with beta = 0.030; K is the formula 1.012 + 0.045 x (...), not an estimate from data. None of these constants has a source. Real distributional certificates are the twin-holon package's work (REVIEW 1A).",
        failure: &[
            "The flag can read 'within budget' for any world the constants allow; it tests nothing about the cohort tier's fidelity.",
            "The pill's earlier label 'CERT' and the panel's 'CERTIFIED (R3 to R0)' claimed a certificate that is not computed; the page no longer says so.",
        ],
        ..BASE
    },
    PageClaim {
        id: "optin-defaults",
        class: Frontier,
        shows: "Defaults for announcing the owner binding (ON), anonymous mode (OFF), onion relay participation (ON) and publishable agent traces (OFF).",
        source: "docs/globe.js: static markup, Population Opt-In Choices",
        blocker: "No CC section or edge constant is cited for these four defaults, and this ledger could not find one. Needs the citation (CC 5.4.6 / 4.5.13 or edge config) or removal.",
        failure: &["A default stated wrongly misleads about what a new node does."],
        ..BASE
    },
    PageClaim {
        id: "onion-cost-share",
        class: Frontier,
        shows: "Anonymous mode costs about a third of a home's shaped share in cover.",
        source: "docs/globe.js: static markup, option 6",
        blocker: "No derivation or source for 'about 1/3' was found.",
        failure: &["Unsupported figure."],
        ..BASE
    },
    PageClaim {
        id: "onion-relay-reserve",
        class: Frontier,
        shows: "Onion relay layer links reserve up to 4 of the node's 8 shaped link slots.",
        source: "docs/globe.js: static markup, option 7",
        blocker: "No source for 'up to 4' was found (the bound of 8 is sourced: shaped-link-bound).",
        failure: &["Unsupported figure."],
        ..BASE
    },
    PageClaim {
        id: "shaping-mean-wait",
        class: Frontier,
        shows: "Shaping adds about 1.16 s (server) and 2.9 s (home) mean Poisson wait.",
        source: "docs/globe.js: static markup, options 1 and 2",
        blocker: "No derivation in source. They are half the frame interval at the TCP MDU (frame-interval: 2.3 s, 5.8 s); under a Poisson schedule the mean wait for the next frame equals the mean interval, not half of it. Needs the derivation or a correction.",
        failure: &["Possibly understated by a factor of two."],
        ..BASE
    },
    PageClaim {
        id: "publish-set-growth",
        class: Frontier,
        shows: "Whether the server's key publish set grows with the fleet or stays at today's size is not known; the v45 knee turns on it.",
        source: "FSD/twin/calibration.md §4 and §7 (key_publish_set growth g)",
        commit: CALIBRATION_COMMIT,
        blocker: "No measurement of CIRISServer's key_publish_set against fleet size; the twin sweeps g from 0 to 1.",
        failure: &["Decides the knee between the rows of the calibration table (g = 0: 14,253 to 75,879; g = 1: 4,882 to 13,723)."],
        ..BASE
    },
    PageClaim {
        id: "global-subjects-at-seat",
        class: Derived,
        shows: "The Global key planes walk every key the seat holds: 8,698 at seat-1.",
        source: "FSD/twin/calibration.md §1 and §5 global_subjects_at_seat (bridge.rs subjects_for_projection :7478, list_revocation_rows :13152: own and cohort on a canonical is every key held); capacity.rs COST_SPECS seat_key_rows",
        commit: CALIBRATION_COMMIT,
        interval: Some(Interval { lo: 8_211.0, hi: 8_698.0, what: "subjects walked by a Global plane listing at seat-1" }),
        failure: &["8,698 is seat-1's key-row count from one day's store composition; the walk grows with the keys the seat holds."],
        ..BASE
    },
    // ------------------------------------------------------------------ globe layers (docs/layers/layers.js, the globe package)
    // One entry per layer registered in window.CEWP_LAYERS; the scan checks that the class here equals
    // the layer's declared `cls`. The card adds the layer's own source, contract and `last` values.
    PageClaim {
        id: "layer-illustration",
        class: Model,
        shows: "Globe layer: the ocean's colour, the sun's glint and the atmosphere's limb, labelled decoration.",
        source: "docs/layers/layers.js: CEWP_LAYERS illustration; gfx.js gfxOcean (shader only, reads no kernel export); lit by kernel-wasm clock_sun",
        commit: "twin/globe b4cbfb9",
        param: "shader colours and glint chosen for looks; sun position from clock_sun(t)",
        failure: &["Decoration, not data: nothing in it is measured or modelled from the mesh. Turned off, the globe shows the plain ocean disc of f73b918."],
        ..BASE
    },
    PageClaim {
        id: "layer-glow",
        class: Model,
        live: true,
        shows: "Globe layer: the night side lit where the simulated population lives.",
        source: "docs/layers/layers.js: CEWP_LAYERS glow; docs/globe.js POP_GRID (population grid); kernel-wasm clock_sun",
        commit: LAYERS_COMMIT,
        param: "the population grid (1.5 degree cells, blurred 3 px into a 1024 x 512 image); sun position from clock_sun(t)",
        failure: &[
            "It is a picture of the model's input, not satellite night lights.",
            "The population grid it draws has no recorded source or licence (pop-grid-source, FRONTIER).",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-arcs",
        class: Derived,
        live: true,
        shows: "Globe layer: a sample of deliveries crossing localities, coloured by tier (shaped hop, seat replication, relay tree or unshaped, onion).",
        source: "docs/layers/layers.js: CEWP_LAYERS arcs; kernel-wasm tick, arcs buffer (u32 x 2: signer, target | tier << 28)",
        commit: LAYERS_COMMIT,
        failure: &[
            "A sample: the tick keeps up to 420 pairs (1 in 3 to 1 in 48 of its deliveries) and the page draws at most 10 a tick, so arc density is not traffic volume.",
            "Derived from the kernel's MODEL world (kernel-traffic, kernel-locality); no real traffic.",
        ],
        reproduce: "node --test docs/kernel/kernel.test.js",
        ..BASE
    },
    PageClaim {
        id: "layer-flows",
        class: Derived,
        live: true,
        shows: "Globe layer: exact counts of deliveries between localities over the last 25 ticks, the largest 60 pairs drawn.",
        source: "docs/layers/layers.js: CEWP_LAYERS flows; kernel-wasm tick (globe buffer 10, a 181 x 181 u32 matrix) and locality_centroids (buffer 2)",
        commit: LAYERS_COMMIT,
        failure: &[
            "The counts are exact for the simulated world, which is a MODEL (kernel-locality: the 90/9/1 room mix largely sets them).",
            "The layer checks every tick that the matrix sums to out[6] + out[7]; a mismatch shows on its status line and as check_ok = 0 on this card.",
        ],
        reproduce: "node --test docs/kernel/kernel.test.js",
        ..BASE
    },
    PageClaim {
        id: "layer-alm",
        class: Derived,
        live: true,
        shows: "Globe layer: the relay tree of one region's room (CC 6.1.6), its depth, busiest parent and fan-out bound.",
        source: "docs/layers/layers.js: CEWP_LAYERS alm; kernel-wasm alm_room, alm_bounds; crates/cewp-sim/src/alm.rs compute_alm_topology_verified_with_publisher",
        commit: LAYERS_COMMIT,
        param: "MODEL inputs: device class shares 30/30/15/25 %, uplinks 2/8/12/20 Mbit/s, max subscribers 1/2/4/8, RTT = 10 ms + 2 x 1.6 x km / 200, trust depth 1 for every pair, first 64 members",
        failure: &[
            "The tree is computed exactly, but over MODEL inputs: uplinks, class shares and the RTT formula are not measured.",
            "Trust depth 1 for all pairs removes the trust constraint the real tree builder applies.",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-custody",
        class: Derived,
        live: true,
        shows: "Globe layer: which members of a room hold its sealed chunk, and whether custody is Full, Partial or envelope only.",
        source: "docs/layers/layers.js: CEWP_LAYERS custody; kernel-wasm custody_room (globe buffers 7 and 8); chunk_custody classify(present, 20, 6), effective_source_count",
        commit: LAYERS_COMMIT,
        param: "present_pm (the slider): the share of holders reachable, drawn per holder; target 20, minimum viable 6",
        failure: &[
            "Who is reachable is a MODEL draw from the slider, a stand-in until the failures package supplies correlated outages.",
            "Holders are the room's members; no custody placement policy beyond that is modelled.",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-reach",
        class: Model,
        live: true,
        shows: "Globe layer at street scale: each generated home marked public listener, punchable NAT or hard NAT / CGNAT.",
        source: "docs/layers/layers.js: CEWP_LAYERS reach; kernel-wasm micro_export (micro buffer 6, router class); crates/cewp-sim/src/micro.rs PARAM_SPECS",
        commit: LAYERS_COMMIT,
        param: "micro.rs PARAM_SPECS router class shares (MODEL)",
        failure: &[
            "The homes are generated and the router shares are modelling choices; no NAT survey stands behind them.",
            "Independent per home: no ISP-wide CGNAT correlation (the nat package's work).",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-seat",
        class: Model,
        live: true,
        shows: "Globe layer: serve-seat load (CPU and permit utilisation) across fleet sizes from 10 to 100,000 devices, and this street's point on it. No knee is drawn.",
        source: "docs/layers/layers.js: CEWP_LAYERS seat; kernel-wasm cap_curve (globe buffer 9), cap_seat; crates/cewp-sim/src/capacity.rs run, COST_SPECS",
        commit: LAYERS_COMMIT,
        param: "COST_SPECS (calibrated to seat-1 aggregates); bundle-bootstrap setting from the street chip (v44, v45, whole-plane); 64 fleet sizes",
        failure: &[
            "Rests on the same cost model as seat-load-model, which is being recalibrated.",
            "One queue with Poisson-smooth arrivals (capacity.rs module doc).",
        ],
        reproduce: "cargo run -p cewp-sim --example capacity_sweep",
        status: Status::UnderCorrection,
        correction: SEAT_KERNEL_CORRECTION,
        ..BASE
    },
    // Phase 2 layers: one kernel-wasm door (kernel-wasm/src/views.rs, slots 0-15) runs each package's
    // own view function once per page. Sources below are at VIEWS_COMMIT; views.rs itself lands with
    // the globe commit that registers these layers.
    PageClaim {
        id: "layer-topology",
        class: Model,
        live: true,
        shows: "Globe layer: a generated network of ASes, IXPs and cables over real cities (seed 7: 181 metros, 839 ASes, 95 IXPs, 3,732 links), and one route's AS hops and long-haul km.",
        source: "docs/layers/layers.js: CEWP_LAYERS topology; kernel-wasm views.rs view_topology, view_topology_path; crates/cewp-sim/src/netgraph/view.rs TopologyView, view_classes (every field MODEL); FSD/twin/topology.md",
        commit: VIEWS_COMMIT,
        param: "netgraph NetParams::default() at seed 7 (FSD/twin/topology.md parameter table, all MODEL)",
        failure: &[
            "Every AS, IXP, peering relationship and cable is invented; no PeeringDB, CAIDA or cable-map data is approved or used.",
            "AS positions are a display spiral around their metro, not locations.",
            "Only the metros are real places, at city grain.",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-nat",
        class: Model,
        live: true,
        shows: "Globe layer (drawn in a dock panel, not on the globe): devices of a generated street frame by access class and NAT behaviour, each pair's outcome (IPv6, listener, punched, relayed without a punch, punch failed then relayed, same LAN), and the 3 x 7 access-by-behaviour joint.",
        source: "docs/layers/layers.js: CEWP_LAYERS nat; kernel-wasm views.rs view_nat; crates/cewp-sim/src/reach/hooks.rs nat_view_nodes, nat_view_node_pos, nat_view_pairs, nat_view_joint; FSD/twin/nat.md",
        commit: VIEWS_COMMIT,
        param: "reach ReachModel behaviour shares, calibrated to micro.rs PARAM_SPECS router-class marginals (MODEL)",
        failure: &[
            "The street frame is generated, in metres, at no real place.",
            "Behaviour shares are MODEL, fitted to micro's marginals; no NAT survey stands behind the joint.",
            "One seeded draw per pair: an outcome shown is one sample, not a rate.",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-people",
        class: Model,
        live: true,
        shows: "Globe layer: localities sized by the week-intensity multiplier at the simulation's hour of week, the strongest cross-locality ties, and the week's synchronized events (flash crowd first).",
        source: "docs/layers/layers.js: CEWP_LAYERS people; kernel-wasm views.rs view_people; crates/cewp-sim/src/people_hooks.rs people_view_localities, people_view_week, people_view_ties, people_view_flash over GLOBE_PERSONS = 6000 generated persons; FSD/twin/people.md",
        commit: VIEWS_COMMIT,
        param: "people ActivityParams (MODEL); locality centroids and weights DERIVED from the population grid; synchronized-event inputs FRONTIER",
        failure: &[
            "Rhythm and ties come from one MODEL one-week bursty run of 6,000 generated persons; small localities are noisy.",
            "The flash crowd's peak multiplier is DERIVED from FRONTIER event inputs.",
            "The population grid behind the localities has no recorded source or licence (pop-grid-source, FRONTIER).",
            "Hour of week maps the simulation's day 314 (a Tuesday) onto the hook's Monday 00:00 UTC origin.",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-holon",
        class: Derived,
        live: true,
        shows: "Globe layer: the coarse-tier cells of the R5 demonstration (host kind x access network, laid out as a patch near Zurich; not places), each certified, refused or not evaluated with its binding bound and margin, and the per-tier certificate table.",
        source: "docs/layers/layers.js: CEWP_LAYERS holon; kernel-wasm views.rs view_holon; crates/cewp-sim/src/holon_view.rs cells_of, certificates_of, holon_view_bounds over holon_demo::demonstration_seeded(VIEW_HORIZON_S, 43); FSD/twin/holon.md",
        commit: VIEWS_COMMIT,
        param: "inputs MODEL: the micro world under the tier; seed 43",
        failure: &[
            "A certificate here says the coarse tier matched the finer tier's distributions within budget over the twin's MODEL world; it certifies nothing about the real mesh.",
            "Refused cells are shown as refusals and sent back to the finer tier, not hidden.",
            "The patch position (centred 47.3769, 8.5417) is layout only.",
        ],
        ..BASE
    },
    PageClaim {
        id: "layer-calibration",
        class: Model,
        live: true,
        shows: "Globe layer: one v45.0.0 seat's utilisation band against fleet size (500 to 100,000 devices), the knee as a lower-to-upper bracket with no central mark, and each seat-1 counter's predicted over observed, in-sample and held-out marked apart.",
        source: "docs/layers/layers.js: CEWP_LAYERS calibration; kernel-wasm views.rs view_calibration_curve(g), view_calibration_knee(g), view_calibration_residuals(); crates/cewp-sim/src/seat_hooks.rs calibration_view_curve, calibration_view_knee, calibration_view_residuals; FSD/twin/calibration.md",
        commit: CAL_LAYER_COMMIT,
        param: "cost model fit to seat-1 counters (MEASURED, 2026-11-10 windows, aggregates); input intervals per seat_calibration::input_intervals (MODEL); key_publish_set growth g FRONTIER (publish-set-growth)",
        interval: Some(Interval { lo: 4_882.0, hi: 75_879.0, what: "devices at the knee, outer bounds over the input intervals for g in [0, 1]; bounds, not a probability" }),
        failure: &[
            "g is not measured; the knee spans 4,882 to 75,879 devices across g from 0 to 1.",
            "The band is outer bounds over the input intervals, not a probability; it assumes each term is monotone in each input over its range.",
            "v45.0.0 only, read from source at edge 8e46165; the fitted windows ran v44.1.2.",
            "Held-out counters include permit residuals of 678.5, 4,697 and 4,401 a minute (windows B, A', C) that are UNEXPLAINED; the furthest, window A' permits, is predicted at 1,689 against 6,386 observed (x0.264, FSD/twin/calibration.md residual table).",
        ],
        reproduce: "cargo run --release -p cewp-sim --example capacity_sweep (on twin/calibration c13cee5)",
        ..BASE
    },
];

/// Looks up an entry by id.
#[must_use]
pub fn lookup(id: &str) -> Option<&'static PageClaim> {
    PAGE_CLAIMS.iter().find(|c| c.id == id)
}

/// Short code shown beside a figure on the page, so a class is never told by colour alone.
#[must_use]
pub const fn class_code(class: EpistemicClass) -> &'static str {
    match class {
        EpistemicClass::Measured => "ME",
        EpistemicClass::Derived => "DE",
        EpistemicClass::Model => "MO",
        EpistemicClass::Projected => "PR",
        EpistemicClass::Frontier => "FR",
    }
}

/// What each class means, in the page legend's words (FSD §8.2 and the `capacity.rs` tags).
pub const CLASS_NOTES: &[(EpistemicClass, &str)] = &[
    (EpistemicClass::Measured, "Read off real code on real hardware: the production serve seat's counters, or your own browser timing the page. Names its window."),
    (EpistemicClass::Derived, "Read from source code or the constitution, or computed by stated arithmetic from source constants and measured counts. Names the file and symbol."),
    (EpistemicClass::Model, "A stated modelling choice, or the output of the simulation under stated inputs. Names its parameters."),
    (EpistemicClass::Projected, "Extrapolated from certified lower tiers (FSD §8.2); always carries an interval. Nothing on this page is PROJECTED: no tier here is certified, so extrapolations past what was measured (the seat knee) are MODEL with their intervals."),
    (EpistemicClass::Frontier, "Open: no source or measurement supports it yet. Names what is missing."),
];

fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Builds the [`TaggedMetric`] the crate's integrity rules check an entry as.
#[must_use]
pub fn as_tagged_metric(c: &PageClaim) -> TaggedMetric {
    let opt = |s: &str| if s.trim().is_empty() { None } else { Some(s.to_string()) };
    TaggedMetric {
        value: None,
        ci95: c.interval.map(|i| [i.lo, i.hi]),
        epistemic_class: c.class,
        source: opt(c.source),
        blocker: opt(c.blocker),
        hardware_provenance: opt(c.window),
    }
}

/// Validates one entry: the crate's §8.2 integrity rules plus the ledger's own.
///
/// # Errors
/// A message naming the entry and the rule it breaks.
pub fn validate_page_claim(c: &PageClaim) -> Result<(), String> {
    let e = |m: &str| Err(format!("claim '{}': {}", c.id, m));
    if !is_valid_id(c.id) {
        return e("id must be non-empty lowercase letters, digits and '-'");
    }
    validate_epistemic_integrity(c.id, &as_tagged_metric(c)).map_err(|err| format!("claim '{}': {}", c.id, err))?;
    if c.shows.trim().is_empty() {
        return e("'shows' is empty");
    }
    if c.source.trim().is_empty() {
        return e("every entry names its source (file and symbol), FRONTIER included");
    }
    if c.failure.is_empty() || c.failure.iter().any(|f| f.trim().is_empty()) {
        return e("needs at least one known failure condition");
    }
    match c.class {
        EpistemicClass::Measured => {
            if c.window.trim().is_empty() {
                return e("MEASURED needs its measurement window");
            }
        }
        EpistemicClass::Derived => {
            if c.commit.trim().is_empty() {
                return e("DERIVED needs the commit its source was read at");
            }
        }
        EpistemicClass::Model => {
            if c.param.trim().is_empty() {
                return e("MODEL needs the parameter name(s) it rests on");
            }
        }
        EpistemicClass::Projected => {
            let Some(i) = c.interval else { return e("PROJECTED needs an interval") };
            if i.what.trim().is_empty() {
                return e("PROJECTED interval must say what it is");
            }
        }
        EpistemicClass::Frontier => {}
    }
    if let Some(i) = c.interval {
        if !(i.lo.is_finite() && i.hi.is_finite() && i.lo <= i.hi) {
            return e("interval must be finite with lo <= hi");
        }
    }
    if c.status == Status::UnderCorrection && c.correction.trim().is_empty() {
        return e("an entry under correction says what is being corrected");
    }
    if c.status == Status::Stated && !c.correction.is_empty() {
        return e("a correction note on a stated entry: set status UnderCorrection or drop the note");
    }
    Ok(())
}

/// Validates the whole ledger (every entry, unique ids).
///
/// # Errors
/// Every problem found.
pub fn validate_ledger() -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    for (i, c) in PAGE_CLAIMS.iter().enumerate() {
        if let Err(m) = validate_page_claim(c) {
            errs.push(m);
        }
        if PAGE_CLAIMS[..i].iter().any(|d| d.id == c.id) {
            errs.push(format!("claim '{}': duplicate id", c.id));
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs) }
}

fn js(buf: &mut String, s: &str) {
    buf.push('"');
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            // keep the file safe to inline in a <script> and free of control characters
            '<' => buf.push_str("\\u003c"),
            c if (c as u32) < 0x20 => buf.push_str(&format!("\\u{:04x}", c as u32)),
            c => buf.push(c),
        }
    }
    buf.push('"');
}

fn num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 { format!("{}", x as i64) } else { format!("{}", x) }
}

/// The ledger as deterministic, pretty-printed JSON (`docs/evidence/claims.json`).
#[must_use]
pub fn ledger_json() -> String {
    let mut b = String::new();
    b.push_str("{\n  \"schema\": ");
    js(&mut b, LEDGER_SCHEMA);
    b.push_str(",\n  \"cewp_base\": ");
    js(&mut b, CEWP_BASE);
    b.push_str(",\n  \"generated_by\": ");
    js(&mut b, GENERATE_CMD);
    b.push_str(",\n  \"classes\": [\n");
    for (i, (cls, note)) in CLASS_NOTES.iter().enumerate() {
        b.push_str("    { \"class\": ");
        js(&mut b, cls.as_str());
        b.push_str(", \"code\": ");
        js(&mut b, class_code(*cls));
        b.push_str(", \"means\": ");
        js(&mut b, note);
        b.push_str(if i + 1 < CLASS_NOTES.len() { " },\n" } else { " }\n" });
    }
    b.push_str("  ],\n  \"claims\": [\n");
    for (i, c) in PAGE_CLAIMS.iter().enumerate() {
        b.push_str("    {\n      \"id\": ");
        js(&mut b, c.id);
        b.push_str(",\n      \"class\": ");
        js(&mut b, c.class.as_str());
        b.push_str(",\n      \"code\": ");
        js(&mut b, class_code(c.class));
        b.push_str(",\n      \"status\": ");
        js(&mut b, c.status.as_str());
        b.push_str(",\n      \"live\": ");
        b.push_str(if c.live { "true" } else { "false" });
        for (k, v) in [
            ("shows", c.shows),
            ("source", c.source),
            ("commit", c.commit),
            ("window", c.window),
            ("param", c.param),
            ("blocker", c.blocker),
            ("reproduce", c.reproduce),
            ("correction", c.correction),
        ] {
            b.push_str(",\n      \"");
            b.push_str(k);
            b.push_str("\": ");
            js(&mut b, v);
        }
        b.push_str(",\n      \"interval\": ");
        match c.interval {
            None => b.push_str("null"),
            Some(iv) => {
                b.push_str("{ \"lo\": ");
                b.push_str(&num(iv.lo));
                b.push_str(", \"hi\": ");
                b.push_str(&num(iv.hi));
                b.push_str(", \"what\": ");
                js(&mut b, iv.what);
                b.push_str(" }");
            }
        }
        b.push_str(",\n      \"failure\": [");
        for (j, f) in c.failure.iter().enumerate() {
            if j > 0 {
                b.push_str(", ");
            }
            js(&mut b, f);
        }
        b.push_str("]\n    }");
        b.push_str(if i + 1 < PAGE_CLAIMS.len() { ",\n" } else { "\n" });
    }
    b.push_str("  ]\n}\n");
    b
}

/// The same ledger as a script the page loads before `globe.js` (`docs/evidence/claims.js`).
#[must_use]
pub fn ledger_js() -> String {
    format!("// Generated by `{}` from crates/cewp-evidence/src/page_ledger.rs. Do not edit.\nwindow.CEWP_LEDGER = {}", GENERATE_CMD, ledger_json().trim_end()) + ";\n"
}

/// `docs/` relative to this crate.
#[must_use]
pub fn docs_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_ledger_entries_validate() {
        if let Err(errs) = validate_ledger() {
            panic!("ledger refused:\n{}", errs.join("\n"));
        }
    }

    #[test]
    fn page_ledger_refuses_what_the_crate_refuses() {
        let projected_no_interval = PageClaim { id: "x", class: EpistemicClass::Projected, shows: "s", source: "f", failure: &["f"], ..BASE };
        assert!(validate_page_claim(&projected_no_interval).unwrap_err().contains("EPISTEMIC_PROMOTION_VIOLATION"));
        let frontier_no_blocker = PageClaim { id: "x", class: Frontier, shows: "s", source: "f", failure: &["f"], ..BASE };
        assert!(validate_page_claim(&frontier_no_blocker).is_err());
        let measured_no_window = PageClaim { id: "x", class: Measured, shows: "s", source: "f", failure: &["f"], ..BASE };
        assert!(validate_page_claim(&measured_no_window).is_err());
        let model_no_param = PageClaim { id: "x", class: Model, shows: "s", source: "f", failure: &["f"], ..BASE };
        assert!(validate_page_claim(&model_no_param).is_err());
        let derived_no_source = PageClaim { id: "x", class: Derived, shows: "s", failure: &["f"], ..BASE };
        assert!(validate_page_claim(&derived_no_source).is_err());
        let no_failure = PageClaim { id: "x", class: Derived, shows: "s", source: "f", ..BASE };
        assert!(validate_page_claim(&no_failure).is_err());
        let bad_interval = PageClaim {
            id: "x",
            class: EpistemicClass::Projected,
            shows: "s",
            source: "f",
            failure: &["f"],
            interval: Some(Interval { lo: 2.0, hi: 1.0, what: "w" }),
            ..BASE
        };
        assert!(validate_page_claim(&bad_interval).is_err());
        let ok_projected = PageClaim { interval: Some(Interval { lo: 1.0, hi: 2.0, what: "w" }), ..bad_interval };
        assert!(validate_page_claim(&ok_projected).is_ok());
        let correction_without_note = PageClaim { id: "x", class: Derived, shows: "s", source: "f", failure: &["f"], status: Status::UnderCorrection, ..BASE };
        assert!(validate_page_claim(&correction_without_note).is_err());
    }

    #[test]
    fn page_ledger_derived_is_never_promoted_to_measured() {
        for c in PAGE_CLAIMS.iter().filter(|c| c.class == Derived) {
            assert!(crate::validate_promotion(c.id, Derived, Measured, false).is_err());
        }
    }

    #[test]
    fn page_ledger_json_is_deterministic_and_escaped() {
        assert_eq!(ledger_json(), ledger_json());
        let j = ledger_json();
        assert!(j.starts_with("{\n  \"schema\": \"cewp-page-claims/v1\""));
        assert!(!j.contains('<'), "'<' is escaped so the ledger can sit in a script");
        assert_eq!(j.matches("\"id\": ").count(), PAGE_CLAIMS.len());
    }

    #[test]
    fn page_ledger_files_are_current() {
        let dir = docs_dir().join("evidence");
        let json = std::fs::read_to_string(dir.join("claims.json")).unwrap_or_default();
        let script = std::fs::read_to_string(dir.join("claims.js")).unwrap_or_default();
        assert!(json == ledger_json() && script == ledger_js(), "docs/evidence/claims.json or claims.js is stale: run `{}`", GENERATE_CMD);
    }

    #[test]
    fn page_ledger_seat_figures_are_under_correction() {
        // still computed by the f73b918 kernel's seat model
        for id in ["seat-load-model", "seat-lock-model", "layer-seat"] {
            assert_eq!(lookup(id).map(|c| c.status), Some(Status::UnderCorrection), "{id}");
        }
        // the knee is twin-calibration's interval, never one count
        let k = lookup("seat-knee").expect("seat-knee");
        assert_eq!((k.class, k.status, k.live), (EpistemicClass::Model, Status::Stated, true));
        let i = k.interval.expect("an interval");
        assert!(i.lo < i.hi && !k.shows.contains("18,2"), "{:?}", k.shows);
    }
}
