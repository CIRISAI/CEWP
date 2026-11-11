# Twin package `calibration`: seat capacity against edge 8e46165, with intervals

Code: `crates/cewp-sim/src/capacity.rs` (the cost ledger, owned by this package),
`crates/cewp-sim/src/seat_calibration.rs` (fit split, held-out check, input intervals,
propagation, residual decomposition, growth sweep), `crates/cewp-sim/src/seat_hooks.rs`
(HOOKS.md: energy counters and the globe views). Run: `cargo run --release -p cewp-sim
--example capacity_sweep` prints the whole report, including the sections below.

This file is the spec for the package. The lead folds pointers into
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

RESULTS_HELD_OUT

This is a thin check: four 60 s windows of one seat, one CPU reading. Five of the seven
held-out counters are permit or query counts the fit has no free parameter for, so a held-out
error says a term is missing, not that a parameter is mis-set.

## 3. Residuals, term by term

RESULTS_RESIDUALS

## 4. Intervals and the knee

Every MEASURED input has an interval (`input_intervals`); where two windows count it the ends
are computed from the windows (`window_spreads`), where one window counts it a band is
borrowed and named. DERIVED and MODEL inputs carry stated bands. The knee is propagated by the
vertex method: each input alone at each end, then all at the ends that lower (raise) it, with
the CPU constant refitted at every point. These are bounds over the stated input ranges, not a
probability.

RESULTS_KNEE

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

RESULTS_ENVELOPE

## 8. Aggregate measurements that would make it sound

Listed in `seat_calibration::MEASUREMENT_REQUESTS` (aggregates only; no identifier, no
per-device row), for the lead to pass to the server session.

## 9. Hooks (HOOKS.md)

RESULTS_HOOKS

## 10. Corrections owed in `FSD/SIMULATION_ENGINE.md` §4.7 and README

RESULTS_CORRECTIONS
