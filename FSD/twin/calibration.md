# Twin package `calibration`: seat capacity against edge 8e46165, with intervals

Code: `crates/cewp-sim/src/capacity.rs` (the cost ledger, owned by this package),
`crates/cewp-sim/src/seat_calibration.rs` (fit split, held-out check, input intervals,
propagation, residual decomposition, growth sweep), `crates/cewp-sim/src/seat_hooks.rs`
(HOOKS.md: energy counters and the globe views). Run: `cargo run --release -p cewp-sim
--example capacity_sweep` prints the whole report, including the sections below.

This file is the spec for the package. Integration folds pointers into
`FSD/SIMULATION_ENGINE.md` §4.7; the exact corrections owed there are listed at the end.

## 1. What is modelled

One serve seat ("seat-1": one host, 4 cores, a single canonical on purpose) and the fleet of
bootstrap devices that round with it, under edge **release/45.0.0 at 8e46165** (untagged when
read). The device schedule, the seat's read paths and their permits, the four key-plane locks
and the CPU are booked per second in a cost ledger (`capacity::ledger`), from the rounds and
Pulls arriving at the seat (`capacity::run`). The knee is the largest fleet whose seat stays
under the CPU ceiling (70%) on every term: cores, the two sweep permits, and the busiest
key-plane lock (one server each).

What changed against the f73b918 model, each read at 8e46165 (file, symbol, line cited in
`COST_SPECS` and in `capacity::edge_971::NEEDLES`):

- **The device schedule matches.** `BUNDLE_PULL_PLANES`, `BUNDLE_OWNER_PLANES`,
  `BUNDLE_ROUND_PLANES = [Revocation]`, the 10-minute jittered Pull timer, 30 s min spacing,
  6 first-contact tries (runtime.rs :1113-:1683). No change to the twin's schedule.
- **The listing cache and the read budget are built and on by default**
  (`BridgeConfig::default` bridge.rs :1220-:1221, `KEY_PLANE_READ_BUDGET = 256`,
  `LISTING_CACHE_SLOTS = 4`, key_plane_memo.rs :296/:304). They were MODEL scenario flags; they
  are now DERIVED structure with MODEL timings (`warm_listing_s`).
- **The TTL no longer clears the memo.** `PlaneMemo::revalidate` (key_plane_memo.rs :462):
  stale-while-revalidate, at most 256 reads a listing. The v44.1.2 burst (the whole plane
  under the lock) cannot occur at 8e46165.
- **The Global planes walk every key the seat holds**, not "tens".
  `subjects_for_projection(Global)` = self_provider ∪ cohort (bridge.rs :7478/:7492), and on a
  canonical that is every key (`list_revocation_rows` :13152 comment). `global_subjects_at_seat`
  30 (MODEL) → 8,698 (DERIVED, interval 8,211 to 8,698).
- **The reach rule (6244795) adds seat reads.** `identity_row_reach` → `unowned_reach`
  (bridge.rs :13985/:14189): an unowned subject's reach reads
  `list_identity_occurrences_by_occurrence_key`, maybe `lookup_public_key` and the own set,
  memoised 30 s (`OWNER_BINDING_MEMO_TTL`), cleared by ownership and local-write events.
  Booked as reach reads on listings and Pull legs.
- **Consent send-set reads take permits.** `resolved_peer_set` takes a sweep permit (:11363)
  behind a 10 s memo (`CONSENT_SEND_SET_MEMO_TTL`), at least 6 a minute.
- **The publish set's growth is a scenario axis.** `key_publish_set_growth` g (FRONTIER):
  subjects(N) = at_seat × (1 + g (N / announced − 1)) on Key, occurrence and the Global
  planes. g = 0 keeps today's set; g = 1 grows it with announced nodes. The server decides it.
- **Seat-to-device initiator rounds are the server's choice.** At 8e46165 a node initiates
  only to host-named peers, bundle seats and homed nodes; there is no default seat → device
  initiator. `seat_initiator_device_share` (FRONTIER, default 0) books five planes a cadence
  per named device plus their reply Summaries.
- **The CPU fit reads window B's CPU through a parameter** (`cpu_window_b_cores`) so the
  split can be enforced; `composer_per_att_round` is window A's ratio only (window B's is held
  out).

## 2. Fit split, held-out check

`seat_calibration::FIT` names every (window, counter) a parameter may read; `HELD_OUT` names
what the fit must predict. `fit` reads only through a `FitView`, which panics on any pair
outside `FIT`; tests check that the two sets are disjoint, that moving a held-out value moves
no fitted parameter, and that the defaults are the fit.

- Fit: window A's store doors (attestation queries, rows, composer, consent, withholds,
  Attestation finals) and window B's CPU at window B's round mix.
- Held out: window B's attestation queries, composer and permits; window A′ permits; window C
  permits and attestation queries; the busy-hour CPU.

Result (sweep, 2026-11-11; predicted vs measured, each a held-out counter):

| window | counter | predicted | measured | held-out error |
|---|---|---|---|---|
| B | attestation queries | 21,440.5 | 19,754 | +8.5% |
| B | composer lookups | 3,867 | 4,512 | −14.3% |
| B | sweep permits | 1,583.5 | 2,262 | −30.0% |
| A′ | sweep permits | 1,689 | 6,386 | −73.6% |
| C | sweep permits | 3,111 | 7,512 | −58.6% |
| C | attestation queries | 21,670 | 15,068 | +43.8% |
| busy hour | CPU (cores) | 1.117 | 1.000 | +11.7% |

The door counters (queries, composer) and CPU land within 15% in window B and the busy hour.
Permits are under-predicted in every window, by 30% to 74%: the model is missing permit
terms (section 3). Window C's attestation queries are over-predicted by 44%.

This is a thin check: four 60 s windows of one seat, one CPU reading. Five of the seven
held-out counters are permit or query counts the fit has no free parameter for, so a held-out
error says a term is missing, not that a parameter is mis-set.

## 3. Residuals, term by term

Permits a minute, each term booked from edge source (sweep output; no constant absorbs the
remainder):

| term | B | A′ | C |
|---|---|---|---|
| index drains on the Attestation index (Key, occurrence, Attestation finals × 42 pages) | 1,302 | 1,386 | 2,772 |
| index drains on the route index (route finals × 18 pages) | 198 | 216 | 216 |
| Attestation listing and required-rows pages | 40 | 40 | 44 |
| route listing pages | 16.5 | 18 | 18 |
| key-plane change probes | 21 | 23 | 55 |
| consent send-set reads (≥ one per 10 s memo, added 2026-11-11) | 6 | 6 | 6 |
| **model** | 1,583.5 | 1,689 | 3,111 |
| **measured** | 2,262 | 6,386 | 7,512 |
| **UNEXPLAINED** | 678.5 | 4,697 | 4,401 |

The 2026-11-11 corrections close 6 a minute (0.9% / 0.1% / 0.1%). Candidate causes, each a
permit site v44.1.2 has that the model does not book, with the 8e46165 counter that settles it
(the sweep prints all seven, H1 to H7):

- H1, a TTL refill burst under the plane lock (v44.1.2). A Key burst is 8,211 permits, an
  occurrence burst 5,650 to 11,300; A′ and C are the size of part of one, B is not. Two of
  three windows holding one is more than the one-in-ten-per-plane phase predicts. Settled by
  `sweep_permits.key_plane_subject` and `key_plane_sweep.rebuilds_ttl`. At 8e46165 the TTL
  revalidates at most 256 reads a listing, so a v45 window cannot show this burst.
- H2, rounds on the twelve planes the windows did not count: at most 720 (B), 864 (A′),
  960 (C) a minute. Could close B; cannot close A′ or C alone. Settled by
  `sweep_permits.plane_page` and responder finals on all planes.
- H4, the seat's own initiator rounds: the residual is worth 16.2 (B), 111.8 (A′), 104.8 (C)
  whole 42-page drains a minute. Settled by the `initiator_peers` gauges and initiator finals.
- H3, H5 to H7: printed by the sweep with their counters.

None is confirmed. The residual stays UNEXPLAINED until the per-site counters are read.

## 4. Intervals and the knee

Every MEASURED input has an interval (`input_intervals`); where two windows count it the ends
are computed from the windows (`window_spreads`), where one window counts it a band is
borrowed and named. DERIVED and MODEL inputs carry stated bands. Each term's knee is propagated
by the vertex method (each input alone at each end, then all at the ends that lower (raise)
it, with the CPU constant refitted at every point) and the knee interval is the outer bound
over the terms. These are bounds over the stated input ranges, not a
probability.

Correction to the method, found by a failing test: the knee is the minimum of three term
knees (CPU, sweep permits, the busiest lock), and the minimum switches term across the input
box, so the vertex method run on the knee itself is not a bound (it put g = 1's upper end at
13,723 above g = 0.75's 12,008). `knee_interval` now propagates each term's knee alone and
takes `[min lo, min hi]`: an outer bound. The globe curve's rho band is built the same way
(`[max lo, max hi]` over terms).

v45.0.0 knee, devices on one 4-core seat at a 70% ceiling (MODEL with an interval: outer bounds over the
input intervals, not a probability), against the server's `key_publish_set` growth g
(FRONTIER):

| g | knee | binds low / high | 25% of devices on v44 | seat initiating to 1% of devices |
|---|---|---|---|---|
| 0 | 14,253 to 75,879 | sweep permits / CPU | 2,925 to 22,933 | 10,015 to 67,076 |
| 0.25 | 7,961 to 24,430 | CPU / CPU | 2,623 to 18,057 | 6,177 to 23,730 |
| 0.5 | 6,316 to 18,438 | CPU / CPU | 2,418 to 14,003 | 4,691 to 17,830 |
| 0.75 | 5,440 to 15,534 | CPU / CPU | 2,268 to 11,835 | 3,903 to 14,955 |
| 1 | 4,882 to 13,723 | CPU / CPU | 2,155 to 10,462 | 3,413 to 13,145 |

By version at g = 1 (MODEL, intervals): v44 whole-plane rounds 5 to 36 devices (permits), lock knee
1,877 to 3,398, today's fleet 42.7 to 136 cores; v45 without the read fix (never released)
33 to 179 (permits); v45.0.0 4,882 to 13,723 (CPU), lock knee 8,175 to 18,091, today's fleet
0.135 to 0.563 cores.

The single 18,229 (about 18,200) knee no longer stands. It came from `global_subjects_at_seat`
= 30 (now 8,211 to 8,698), no reach-rule reads, no send-set floor, and point inputs. Which term
binds turns on g: permits at the low end when the publish set stays at today's size, CPU
otherwise. No single device count is claimed.

## 5. Parameters

Every parameter, its value, class and source, is in `capacity::COST_SPECS` and printed by the
sweep's parameter table; the input intervals are printed beside it. New or changed at
8e46165:

| parameter | value | class | source |
|---|---|---|---|
| `global_subjects_at_seat` | 8,698 (8,211 to 8,698) | DERIVED | own ∪ cohort on a canonical is every key held; bridge.rs `subjects_for_projection` :7478, `list_revocation_rows` :13152 |
| `key_publish_set_growth` | swept 0 to 1 (default 1) | FRONTIER | the server's `key_publish_set` rule; not in edge |
| `listing_budget_k` | 256 | DERIVED | `KEY_PLANE_READ_BUDGET` key_plane_memo.rs :296 |
| `warm_listing_s` | MODEL timing on a BUILT path | MODEL | `cached_listing` key_plane_memo.rs :550 |
| `probe_per_s` | 1 / 5 s | DERIVED | `FEED_PROBE_SPACING` store_feed.rs :86 |
| `send_set_memo_ttl_s` | 10 | DERIVED | `CONSENT_SEND_SET_MEMO_TTL` bridge.rs :3150; `resolved_peer_set` permit :11363 |
| `reach_unowned_share` | 0.5 (0.24 to 0.76) | MODEL | bounds DERIVED from owners and announced nodes over occurrence subjects |
| `reach_reads_per_unowned_subject` | 1.5 (1 to 2) | DERIVED | `unowned_reach` bridge.rs :14189 |
| `announce_memo_clears_per_hour` | 6 (0 to 60) | MODEL | `invalidate_owner_memo` :13446, `invalidate_send_set_memos` :13621 |
| `subject_set_build_per_subject_s` | 0.1 µs | MODEL | own ∪ cohort as a set before each listing |
| `key_record_writes_per_device_day` | 0.05 | MODEL | each key:record row dirties Key and Revocation (`dirt_of` :240) |
| `owner_leg_permit_share` | 1 (0 to 1) | MODEL | owner legs answered in the entitled arm or refused |
| `seat_initiator_device_share` | 0 | FRONTIER | the server's choice of seat peers |
| `today_rounds_scale` | 1 (window spread) | MEASURED | other windows' finals per plane against window B's |
| `cpu_window_b_cores` | 0.652 | MEASURED | window B, the one CPU window (fit input) |
| `composer_per_att_round` | 386.7 | MEASURED | window A only (was the A/B mean 418.95) |

The source test `edge_971_constants_match_the_source` reads edge from `CEWP_EDGE_SRC` (a
working tree), or `CEWP_EDGE_REPO` (+ `CEWP_EDGE_REV`, default 8e46165) through `git show`, or
the legacy `CEWP_EDGE_RUNTIME_RS`; with none set it skips with a note. No path is built in.

## 6. What a real dataset would replace

No external dataset. Every input is either edge source or an aggregate of seat-1's own
counters. What would replace the MODEL and FRONTIER inputs is more aggregate measurement of
the same seat (section 8), and the server's publish-set rule.

## 7. Validity envelope: what the model cannot say

- One seat, four 60 s windows on one day, one of them with CPU. Nothing about other hours,
  days, seats or seat sizes. Intervals on single-window inputs are borrowed bands, named in
  `input_intervals`.
- The windows ran v44.1.2; the knee is for 8e46165. Every v45 term is read from source, not
  measured on a v45 seat.
- Pull and drain-page timings are edge debug-build wall times on an in-memory fixture store;
  the release end of their band (4× faster) is MODEL.
- Growth of `key_publish_set` with the fleet is FRONTIER and decides the knee between the
  table's rows.
- Intervals are bounds over stated input ranges, not confidence levels. They assume each term
  is monotone in each input over its range (checked by tests at the ends, not proven).
- Permit residuals of 678 to 4,697 a minute are UNEXPLAINED; a v45 seat may carry a term of
  that size the model does not book.
- No revocation was ever observed: the Revocation carry rate is 0 by construction.
- Past 1,349 devices every figure is an extrapolation of one seat's model. It is classed MODEL with its interval, not PROJECTED: FSD §8.2 reserves PROJECTED for extrapolation from certified lower tiers, and none is certified.

## 8. Aggregate measurements that would make it sound

Listed in `seat_calibration::MEASUREMENT_REQUESTS` (aggregates only; no identifier, no
per-device row), to be put to the server project.

## 9. Hooks (HOOKS.md)

Code: `crates/cewp-sim/src/seat_hooks.rs`.

Energy counters (counts only, DERIVED from the cost model at the point inputs; no watts, no
joules):

- Per device (`device_counters`, every `DEVICE_KINDS` kind alike: runtime.rs at 8e46165 has no
  host-kind branch in the schedule): 126.75 wake-ups an hour = 120 Revocation rounds (all idle)
  + 6.75 Pull cycles (5.60 idle); 77,313 bytes an hour each way claimed, 76,049 unclaimed; seat
  store reads it causes 117.9 an hour claimed, 86.4 unclaimed.
- Seat (`seat_counters`) at 1,349 / 5,000 / 10,000 devices: rounds 164,400 / 602,520 /
  1,202,520 an hour; Pulls 82,570 / 306,042 / 612,083; store reads 1,534,827 / 4,623,386 /
  8,853,129; index pages 3,600 / 13,260 / 26,460; permits 242,377 / 884,228 / 1,763,206;
  egress bytes 102.6 M / 380.2 M / 760.5 M; reads per round 6.215 / 5.089 / 4.879.
- Not in this model: bytes forwarded for others, signature operations, storage writes, AS
  hops. The seat model has no event for them.

Globe views (flat `Vec<f32>`, aggregates only, no identifier, no coordinate):

- `calibration_view_curve(scenario)`, stride 6: `[devices, rho central, rho lower, rho upper,
  class, binding code]` over 24 geometric points 500 to 100,000; scenario 0 to 4 indexes
  `GROWTH_AXIS` (g 0 to 1).
- `calibration_view_knee(scenario)`, 7 fields: `[lo, central, hi, interval kind (0 = outer
  bounds), binds lo, binds hi, class]`; −1 where no knee is claimed.
- `calibration_view_residuals()`, stride 4: `[counter id, observed, predicted,
  out_of_sample]`; ids 0 to 2 in-sample, 3 to 9 held out.

The page must always draw the interval, never the central value alone.

## 10. Corrections owed in `FSD/SIMULATION_ENGINE.md` §4.7 and README

For integration (this package does not edit the main FSD, README or `docs/`). Line numbers at
f73b918.

- `FSD/SIMULATION_ENGINE.md` :438 `bundle_bootstrap` source: "CIRISEdge#971 c580298" →
  "release/45.0.0 at 8e46165 (runtime.rs `BUNDLE_ROUND_PLANES` :1140)".
- :553, :569, :577–579, :600, :616, :627, :671, :794: "v45.0.0 (in release)" and the c580298
  cites → "release/45.0.0 at 8e46165, untagged when read"; c580298 → e3c20ec, bb6ca83 →
  3bffbdc, 4897789 → 7fa7105, 95a85ca → 5f86c2d, 9a04428 → 6660677 (none of the old shas is an
  ancestor of 8e46165).
- :579, :674, :757: Revocation and occurrence-withdrawal walk own ∪ cohort, which on a
  canonical is every key held: 8,211 to 8,698 at seat-1, not "tens"/30.
- :627: "the listing cache and read budget are MODEL until measured" → both BUILT and on by
  default at 8e46165; only their timings are MODEL.
- :584–585, :620, :714: residuals 684.5 / 4,703 / 4,407 → 678.5 / 4,697 / 4,401 (send-set
  floor added); add that the TTL burst cannot occur at 8e46165 (stale-while-revalidate, ≤256
  reads a listing).
- :636–637, :646, :674–675, :802: drop 18,229 / 18,200 / "about 18,000", 47,709 / 47,700 and
  7,355 / 7,400 → the growth table in section 4 (knee interval against g, binding term, never
  one count).
- :600–760 generated tables and parameter rows: regenerate from `capacity_sweep` on this branch.
- :727, :736, :754: c580298 line cites → 8e46165 (runtime.rs :1113, :2637; bridge.rs :7478,
  :13152).
- `README.md` :13: "CIRISEdge v45.0.0 (in release), read at commit c580298" → "release/45.0.0
  at 8e46165 (untagged)"; any knee figure → the interval against growth.

`docs/` (owner `twin-evidence`): globe.js :291 (version/commit line), :1909, :1914
`BS_TITLE[1]`, :1915–1917, :2485 (single `k45.knee` → interval from
`calibration_view_knee`), :2486 (cache and budget built), :2487 (Global planes walk every key;
TTL revalidates ≤256 reads a listing). No figure in `docs/*.html`.
