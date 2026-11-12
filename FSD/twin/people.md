# Twin package `people`: workload rhythms and emergent social locality

Code: `crates/cewp-sim/src/social.rs` (graph), `activity.rs` (who acts when, in which room),
`locality.rs` (measurement and report). CLI: `cewp-cli people [--persons N] [--days D]
[--sweep-days D] [--seed S] [--geo docs/globe.js|stand-in]`, run from the repo root.

## 1. Why

The outside review (REVIEW.md §3 items 4 and 5) says two things about the twin's workload. Both
were checked against the repo before any code was written.

* **Arrivals are an inhomogeneous compound Poisson process** (FSD §4.1, `workload.rs`). True of
  the spec and of the micro tier (`micro.rs` `generate`: memoryless per device, times
  `diurnal_multiplier`). The scale engine is weaker than the review says: `kernel-wasm/src/lib.rs`
  computes `let _diurnal = diurnal_multiplier(local_sec)` and discards it, and draws the signer
  uniformly, so the page's acts have no clock at all.
* **Locality is an input presented as an output.** True. README §2 says "As a result, ~92% of
  signed acts never leave the signer's locality". `docs/globe.js` `buildWorld` draws every family
  inside the signer's locality and every community room 90% local (`u<0.90`), and `tickWasm`
  passes the act mix 650 / 300 / 40 (public the remaining 10 per mille). The page's figure is
  therefore `0.65 + 0.30 × 0.90 = 0.92` by arithmetic on its inputs. Running the page's own JS
  engine (`buildWorld` + `tickJs` copied out of globe.js into node, 20 ticks of 20,000 acts)
  gives act locality 0.9165 at 100 k nodes and 0.9157 at 1 M nodes (MEASURED on the page's
  engine, not on any mesh), and **delivery** locality 0.58 and 0.21: the public rooms that are
  1% of acts carry most deliveries, and the page's headline is the act share.
* The repo states the figure three ways. README: ~92% of acts. FSD §4.4 Expected regime:
  "Locality Share (self + family + local community) 65% (~90% incl. local comm)", whose name
  includes local community while its value does not. `workload.rs` `RegimeParameters` carries
  `locality_share = 0.65` and nothing reads it beyond a `> 0` check in `cewp-cli` phase 5.

This package makes locality a measured output of a generated society plus a workload, and gives
the workload the rhythms and bursts the review asks for, while keeping today's Poisson process
selectable and mean-equal.

## 2. What is modelled

### 2.1 Geography

`Geography` is cells → localities → regions. The default is the page's own population grid
(2,961 cells, 181 localities, 9 regions), read from `docs/globe.js` `POP_GRID_B64` by
`Geography::from_globe_js` (already in the repo; nothing is downloaded). A twin *locality* is a
cluster of ~16 grid cells, often a large sub-national area (the largest holds 8.4% of the
grid's population), not a town; *region* is one of the page's 9 GSMA/UN regions. There is no
country layer, so "abroad" is approximated by "another region", which undercounts international
ties between neighbouring countries in one region. `Geography::stand_in` is a generated MODEL
grid for tests; `Geography::uniform` gives equal localities for the null-model tests.

### 2.2 Persons and rooms

Persons come in households drawn per cell ∝ population weight, with micro.rs's household rule
(`round(2.5 + 1.1·N(0,1))` clamped to 1..6). Each person is a student, worker or neither. Rooms
are built by seven mechanisms, each with its own seeded sub-stream:

| kind | scope | members | placement |
|---|---|---|---|
| household | family | co-resident persons | one cell |
| kin | family | the founder household + `1 + Poisson(kin_households_mean - 1)` others | same locality, or out of it with `diaspora_share`, of which `diaspora_abroad_share` in another region |
| work | community | workers chunked into lognormal teams | the team's locality; `work_cross_locality_share` of workers sit in another locality |
| school | community | students chunked into classes | the student's locality, neighbours first |
| neighbourhood | community | joining households (`community_join_share`) chunked by `neighbourhood_room_households` | the locality, neighbours first |
| interest | community | `Poisson(interest_groups_per_person)` joins per person into Pareto-popular groups | each join goes to a locality pool, a region pool or the world pool by `interest_local_share` / `interest_regional_share` |
| public | public | `Poisson(public_follows_per_person)` follows, Pareto audience | locality / region / world pools by `public_local_share` / `public_regional_share` |

Ties (for graph statistics): every pair in a room of up to `clique_cap` members; in a larger room
each member ties to `large_group_ties` random co-members; public follows make no tie.

`Placement::Placeless` is the null model: every non-household room ignores geography.

### 2.3 Activity

`Process::Poisson` is today's law: `λ(t) = μ_diurnal(t_local) · acts_per_person_day / 86,400`
per person (it calls `workload::compute_arrival_rate`), local time from the cell's longitude
(solar time, not civil time zones).

`Process::Bursty` keeps that mean and adds:

1. per-person activity, lognormal (`activity_sigma`), normalised to mean 1;
2. a weekly rhythm (`weekly`, mean 1) and a weekend clock `weekend_shift_h` later;
3. a Hawkes cascade: every act begets `Poisson(branching)` replies, each by a random other
   member of the same room, after a Lomax delay (`reply_scale_s`, `reply_alpha`, cut at
   `reply_max_s`);
4. synchronized events: a share `event_share` of the budget arrives as events that reach one
   locality, one region or the world (`event_level_shares`), where `event_participation` of the
   reached join and sign `Poisson(event_acts_per_participant)` acts with an instant rise and an
   exponential decay (`event_decay_s`); event acts use `event_mix` for their scope.

Mean preservation: background immigrants run at `(1 − branching)(1 − event_share)` of the rate,
events at `(1 − branching) event_share`, and each immigrant brings `1/(1 − branching)` acts.

Room choice. `ScopeChoice::FixedMix` (default) draws the scope from the page's 65 / 30 / 4 / 1;
inside the scope the graph picks the room: family → household (`family_household_share`) or a
kin room; community → work, school, neighbourhood or interest rooms weighted by
`community_kind_weight`, with work and school down-weighted off weekday hours; public → a
followed public room. Like the page, a community act by someone with no community room goes to
the family room. `ScopeChoice::TieWeighted` drops the fixed mix: per-kind weights over the rooms
the person actually has, so the scope mix is also an output.

### 2.4 Measurement

Definitions follow the scale engine (kernel-wasm out slots 16/17): an act is local when it has an
audience and every delivery is in the signer's locality; act locality = local acts / acts with
an audience. Delivery locality (local deliveries / deliveries) and the same two at region grain
are reported beside it. A consent-free act goes, as in kernel-wasm, to one seat in the signer's
locality and nine seats drawn by population across the world (`CONSENT_FREE_REMOTE_SEATS`).

Load: a seat homes 2,000 persons of the most populous locality (the page draws one seat per
~2,000 nodes) and sees every act signed by one of them or posted in a room that holds one; a
relay is an interior member of the largest public room's relay tree and forwards every act posted
in it. Counts are per UTC second; the report gives mean, p50, p95, p99, p99.9, max, peak/mean and
the index of dispersion of 60-s windows.

## 3. Parameters

Every default with its class is in code: `social::SOCIAL_PARAM_SPECS` and
`activity::ACTIVITY_PARAM_SPECS` (same `ParamSpec` / `Tag` as `micro.rs`). Literature values in
the sources are quoted from memory and were not re-read in this package; they name the kind of
source, not a checked figure. Summary:

* **MODEL** (a stated modelling choice with a named kind of source): household size and spread,
  student and worker shares, kin room size, diaspora share and abroad share, team sizes, remote
  work shares, class size, neighbourhood join share and size, Pareto tails, clique cap and large
  room ties, acts per person per day (= 86,400 / micro `msg_mean_interval_s`), the page's act
  mix, work hours, weekly rhythm, weekend shift, activity spread, reply delay law, event decay.
* **FRONTIER** (nothing supports a value): interest-group count, size and geography
  (`interest_local_share`, `interest_regional_share`), public follows and geography, family
  household-vs-kin split, community kind weights, every TieWeighted weight, branching ratio,
  every event parameter except decay.

Graph bands, stated before any run (`social::BANDS`), all MODEL: mean degree 10..150; degree
CV 0.5..3; mean local clustering 0.10..0.60; degree assortativity 0..0.40; modularity by
locality 0.30..0.95; ties crossing a locality 0.03..0.37; ties crossing a region 0.005..0.10.

## 4. Results

RESULTS_PLACEHOLDER

## 5. What a real dataset would replace (approval requests, nothing downloaded)

Nothing below was downloaded. Each row is an approval request for the owner; URLs are the
publishers' landing pages; licence and size are as I recall them and must be checked on the
page before approval (marked "check").

| dataset | URL | licence | size | would replace |
|---|---|---|---|---|
| UN DESA International Migrant Stock | https://www.un.org/development/desa/pd/content/international-migrant-stock | UN terms of use, free with attribution (check) | a few MB of spreadsheets (check) | `diaspora_share`, `diaspora_abroad_share`; with a country layer, where kin abroad live |
| Meta Social Connectedness Index | https://data.humdata.org/dataset/social-connectedness-index | CC BY 4.0 via HDX (check) | country pairs a few MB; sub-national pairs ~GB (check) | the tie-crossing placement of kin, interest groups and public follows (`interest_local_share`, `public_local_share` and their regional shares, all FRONTIER today); also a check on `cross_locality_tie_share` |
| Multinational Time Use Study (MTUS) / national time-use surveys (e.g. US ATUS) | https://www.timeuse.org/mtus ; https://www.bls.gov/tus/ | MTUS: registered research use (check); ATUS: US public domain | tens to hundreds of MB (check) | the diurnal curve (`workload::diurnal_multiplier`), `weekly`, `weekend_shift_h`, `work_hours` |
| Aggregate traffic rhythm (e.g. Cloudflare Radar, IXP public traffic graphs) | https://radar.cloudflare.com/ | Cloudflare Radar data CC BY-NC 4.0 (check) | small time series | a check on diurnal/weekly peak-to-mean of the bursty process |
| UN household size and composition | https://www.un.org/development/desa/pd/data/household-size-and-composition | UN terms, free with attribution (check) | a few MB | `persons_per_household`, `household_size_sd` per region |
| UN World Population Prospects (age structure) + ILOSTAT employment-to-population ratio | https://population.un.org/wpp/ ; https://ilostat.ilo.org/ | CC BY 3.0 IGO (WPP), ILO terms (check) | tens of MB | `student_share`, `worker_share` per region |
| Census commuting flows (e.g. US LODES / ACS commuting) | https://lehd.ces.census.gov/data/ | US public domain | GB-scale (check) | `work_cross_locality_share`, `work_cross_region_share` |

Each lands behind an existing seam: `SocialParams` fields (per-region values would add a
per-region table), and `Geography` for any finer place grid. None changes the code path.

## 6. Validity envelope: what this model cannot say

* The emergent locality moves the assumption, it does not remove it. The page assumed "community
  rooms are 90% local"; this model assumes how interest groups, public rooms, kin and work are
  placed. Those are closer to things that can be measured, but at defaults several are FRONTIER,
  and the sweeps show they set the answer.
* A twin locality is huge (~16 grid cells). "Never leaves the locality" at that grain is a weak
  statement about network distance; the same acts measured at cell grain would be less local.
  Nothing here says anything about AS paths or hop counts (package `topology`).
* No country layer: "abroad" is "another region".
* Rooms are static over the horizon (no tie churn, no migration in time).
* The act rate is a per-person mean taken from the micro tier's per-device gap; real per-person
  rates, their diurnal shape and weekly rhythm need the time-use and traffic data in §5.
* Event parameters are a stress design, not a calibration: the flash-crowd rows show what such
  an event does to a seat and a relay, not how often it happens.
* Small worlds: at 20,000 persons a locality holds tens to ~1,700 persons, so per-locality
  spreads at small localities are noisy and the per-second seat and relay counts are those of a
  2,000-person seat and a room of a few thousand, not of production scale.
* No Sybil identities, no adversarial workload (out of scope).

## 7. Hooks (`crates/cewp-sim/src/people_hooks.rs`, per `HOOKS.md`)

### 7.1 Energy counters (counts only)

The model has persons, not devices. Each person acts and receives on one device, counted as
`HostKind::Phone` (micro tier host kinds); consent-free rows land on serve seats, counted as
`HostKind::ServeSeat` (one seat per 2,000 persons per locality, as the page). `energy(...)`
returns `EnergyCounts { phone, seat: ClassCounts, wakes: WakeCounts }`:

| counter | class | how |
|---|---|---|
| acts originated, signs | DERIVED | one sign per act signed |
| acts received, verifies | DERIVED | one copy per other room member; one verify per copy (assumes each receiver verifies once) |
| seat rows received | DERIVED | consent-free act x (1 local + 9 remote seats), as `kernel-wasm` `tick` |
| busy seconds, wake-ups, seconds awake | DERIVED | per sampled phone, second-resolution bitmap of sends and receipts; the radio comes up at a busy second when idle and stays up `RADIO_TAIL_S` after the last |
| `RADIO_TAIL_S` = 10 s | MODEL | order of an LTE RRC inactivity timer as commonly cited; not measured on CIRIS devices |

Not counted, because the model has no such event: bytes (sent, received, forwarded), link
class, encryption bytes, storage, AS hops and kilometres. No watts.

### 7.2 Globe views (flat buffers, layouts in the doc comments)

* `view_localities(geo) -> Vec<f32>`: `[lat, lon, population_share]` per locality
  (population-weighted unit-vector centroid of the locality's grid cells).
* `view_week_intensity(w, p, seed) -> Vec<f32>`: `[hour * L + k]`, 168 UTC hours, acts per
  person-hour over the mean hourly rate (1.0 = average hour).
* `view_cross_ties(geo, w, ties, max_pairs) -> Vec<f32>`: `[lat_a, lon_a, lat_b, lon_b, ties]`
  per crossing locality pair, sorted by ties descending.
* `flash_scenario(w, lead_s)` + `view_flash(w, p, seed, event, frames, frame_s) -> Vec<f32>`:
  `[(f * L + k) * 2 + {0: acts signed, 1: copies delivered}]` per frame and locality; the
  default scenario is a region-wide event (30% join, 3 acts each, 300 s decay, FRONTIER) in the
  most populous locality's region.
* `globe_scenario(geo, persons, seed)`: the deterministic world these are drawn from.

All coordinates are grid-cell centroids of the page's population grid; all persons generated.
