# Twin package `holon`: distributional certificates, validity envelopes, refinement

Code: `crates/cewp-sim/src/holon_dist.rs` (the machinery), `crates/cewp-sim/src/holon_demo.rs`
(the worked demonstration on the R5 micro tier), `crates/cewp-sim/src/holon.rs`
(`SecondSquareFence::use_certificate`, two new `TransportRefusal` variants). CLI:
`cewp-cli holon-demo [SECONDS]`.

Answers REVIEW.md §1 items A to D. Item E (validation against GROMACS, OpenFOAM, Qiskit and the
like) is about the CIRISHolon repository, not this twin: out of scope, not started.

## 1. What was there

`holon.rs` admits a coarse tier on one scalar: `ClosureCertificate::compute` takes two residual
norms and returns `D_v² = ‖diff‖² / ‖motion‖² ≤ β` and `K ≤ 1.05`. In this repository nothing
computes those norms from a fine and a coarse run:

- `cewp-cli` phase 5 certifies the constants `0.014, 1.0, 0.030, 1.018`.
- `kernel-wasm/src/lib.rs` (the page's `o[28..32]`) computes a raw ratio over the 181 locality
  holons, then multiplies it by `0.12 × stress_mult` (an invented formula in `direct_nat_pm` and
  `shaping_pm`) and caps it at 0.20; its `K` is a formula (`1.012 + 0.045 × …`), not a measured
  Lipschitz constant; its witness-pair count is synthesised from the excess, not a found pair.
  These page numbers are not measured closure defects. This package does not change the wasm
  (RULES 8); it is a finding for the lead.

So before this package there was no measured certificate of any CEWP tier, mean or otherwise.

## 2. Distributional closure (REVIEW 1.A)

Per observable, the fine tier's and the coarse tier's samples are compared as distributions:

- **W1**, exact: in one dimension `W1(μ, ν) = ∫ |F_μ − F_ν|`; both empirical CDFs are steps, so
  one merge walk over the two sorted lists gives the integral as an integer numerator over
  `n·m` (`wasserstein1`). Values are integer fixed point (`FP_ONE` = 10⁶ units per unit), so the
  result is an exact rational, independent of sample order and of float rounding.
- **Quantile defects** `|Q_fine(p) − Q_coarse(p)|` at p50 / p95 / p99 / p99.9 (levels in ppm),
  with the lower empirical quantile `Q(p) = x_(⌈p·n⌉)`, exact.
- Each has its own budget (`ObservableSpec`). The old mean test is computed beside them, through
  the existing `ClosureCertificate::compute`, with ‖diff‖² = (μ_f − μ_c)² and ‖motion‖² = μ_f²
  (a relative squared mean defect; MODEL normalisation, because the observables here are
  stationary and the FSD's ‖v(T x) − v(x)‖² would be near zero).
- **Resolution floor.** A level is judged only if both samples hold at least
  `min_tail_samples` order statistics beyond it (`n·(1 − p) ≥ m`, exact in integers,
  `tail_resolved`). A level below the floor is UNRESOLVED: not certified and not refused
  (`ObservableDefect::unresolved_levels`, `DistributionalCertificate::covers_level`). The twin
  then answers "this run cannot say" for that quantile instead of passing or failing it on a
  dozen correlated samples.
- A certificate lists the observables it covers. `SecondSquareFence::use_certificate` refuses an
  observable not listed (`NotCertified { ObservableNotCertified }`) and one whose W1 or any
  quantile broke its budget (`NotCertified { W1 | Quantile }`), naming each breach.

`holon.rs` did not use fixed point before (it is all `f64`); the fixed-point convention is new
here, following the integer-only style of `alm.rs` and `tiling.rs`.

## 3. Validity envelope (REVIEW 1.B)

Every `DistributionalCertificate` carries a `ValidityEnvelope`: population range (devices),
workload range (rows per device-hour), topology class (`micro::DensityClass`, serve seats M,
homing k), the failure regimes it was measured in (the four FSD §4.4 regimes as
`FailureRegime::Standard`, plus `CorrelatedAccessLoss`), the horizon (longest run checked), and
the range of every coarse cell's state seen while it was measured.

The envelope records what was checked and nothing more: the demonstration's envelope is a
single population, a single workload and a single regime, so any other value is refused. Using
a certificate outside it goes through the same fence that already refused uncertified claim
transport (`SecondSquareFence`), now with typed refusals: `TransportRefusal::OutsideEnvelope
{ broken }` lists every bound that broke by name (`population`, `workload`, `topology`,
`failure regime`, `horizon`). No parallel mechanism: `transport_claim`'s untyped predicate is the
special case.

## 4. Uncertainty propagation (REVIEW 1.C)

Inputs are `UncertainInput::{Point, Interval, Samples}`. The coarse map here is the cohort map
(online devices = Σ over cells of a binomial-like count), which is monotone in every cell's
share. `CohortModel::sample` draws on one random stream whose use does not depend on the laws
(one u64 per factor, per cell, per device), so models over the same cells are coupled draw by
draw; under that coupling the online total is nondecreasing in every share, draw by draw.
Therefore:

- **Interval** (`propagate_interval`): every quantile of the image of a box of shares lies
  between the quantiles at the box's two corners. The certificate records these as
  `OutputInterval`s. Tested exactly (no margin): the quantiles of a run whose share moves inside
  the interval lie inside the recorded output interval.
- **Samples / non-monotone** (`propagate_samples`): each draw takes the share from the input set
  (an interval is read as uniform on a 1% grid: MODEL).

Midpoint collapse, shown (`midpoint_collapse_passes_the_mean_and_fails_the_tail`): 100 phones
whose online share is known only to lie in [0.70, 0.95] (the FSD §4.4 p_offline spread 5% to
30%, read as an input interval, MODEL) and moves for all of them together. The midpoint model
(every phone at 0.825) passes the mean test and fails p99 of devices off the seat; the
propagated model passes.

Numbers (MODEL outputs, 20,000 draws each, seeds 11/12/13, devices off the seat):

| model vs truth | mean (truth / model) | D² (≤ 0.02) | W1 (≤ 1) | p50 | p95 | p99 | p99.9 (truth / model, budget 3) |
|---|---|---|---|---|---|---|---|
| midpoint | 17.49 / 17.57 | 0.000020 pass | 3.97 **breach** | 17 / 17 | 31 / 24 **breach** | 36 / 27 **breach** | 40 / 31 **breach** |
| propagated | 17.49 / 17.55 | 0.000009 pass | 0.08 | 17 / 17 | 31 / 32 | 36 / 36 | 40 / 41 |

## 5. Out-of-envelope detection and refinement (REVIEW 1.D)

`refinement_trigger(envelope, live)` takes a live coarse state (operating point + each cell's
occupancy) and returns `Inside` or `Refine { broken, refine_cells }`. A cell whose occupancy is
outside the range seen at certification, or a cell never seen, is named for re-running at the
finer tier. A whole-tier bound (regime, topology, population, workload, horizon) names every
cell.

**Where `WitnessPair` fits.** For a deterministic fine motion it is the trigger's proof that a
view is not closed: in the demonstration, x has network 0's home nodes off and y the same number
of home nodes off on other networks. The per-kind view agrees on them; the motion "network 0
restored" splits them (`WitnessPair::verify` true). The refined (kind, network) cells separate
the pair, so it is no collision of the refined view (the logic of
`CIRISHolon.ClosureLadder.separation_dissolves_collision`, which is about exact views).

**Where it does not.** (a) A broken envelope bound (regime, horizon, population) is not a closure
failure: there is no pair of states to exhibit; the refusal is the fence's. (b) `WitnessPair`
needs a deterministic `fine_motion: Fn(&X) -> X` and compares views with `!=`. A distributional
defect under stochastic motion (the mean-field cohort tier in the outage regime) has fine states
whose next laws differ only in correlation; exhibiting it needs a comparison of laws (W1,
quantiles), which is what the certificate measures, not a pair.

## 6. Worked demonstration on the R5 tier (REVIEW 1.A + B + D)

Fine tier: R5 micro world, one generated urban patch (3,000 /km², seed 43; patch centre a public
city centre at coarse grain, every street, household and device generated), production-shape
parameters, one serve seat, k = 1. 221 devices round with the seat. Each household is put on one
of 3 access networks by a hash (MODEL). Observables per 30 s cadence: devices whose round arrives
at the seat, and devices off the seat. **These two are complements** (they sum to the 221
eligible devices every cadence), so their W1 is identical; both are kept because the outage's
signature is the lower tail of arrivals, which is the upper tail of devices off the seat, and the
quantile levels are upper levels.

Coarse tier R4c: the cohort reduction: cells = (host kind × access network), each cell's
devices independent, at the cell's online share measured in the fine run (mean-field). R4c+gate:
the refined tier, the same cells, with network 0's home nodes and laptops gated on one shared
up/down factor at the measured down share. 4 coarse draws per fine sample.

Regimes: Expected (FSD §4.4), and a minimal local correlated loss: network 0 alternates up
(mean 6 h) and down (mean 1.5 h); while down, its households' home nodes and laptops are off,
phones fall back to cellular and are untouched. Down 2,476 of 11,520 cadences (21.5%).

All numbers below are MODEL outputs of the regression test (4 days per regime, 11,520 fine
cadences, 46,080 coarse draws; `cewp-cli holon-demo` prints the same table; debug and release
builds give the same numbers).

| tier / regime | observable | mean fine / coarse | mean-only (D² ≤ 0.02) | W1 (≤ 1) | p50 | p95 | p99 | p99.9 |
|---|---|---|---|---|---|---|---|---|
| R4c / Expected | arrivals | 165.43 / 165.41 | pass (1.6e-8) | 0.557 | 0 | 1 | 3 (budget 3) | 5, unresolved |
| R4c / Expected | off seat | 55.57 / 55.59 | pass (1.4e-7) | 0.557 | 0 | 1 | 0 | 4, unresolved |
| R4c / outage | arrivals | 160.82 / 160.84 | pass (6.6e-9) | **5.636** | **4** | **4** | **5** | 6, unresolved |
| R4c / outage | off seat | 60.18 / 60.16 | pass (4.7e-8) | **5.636** | **4** | **14** | **14** | 16, unresolved |
| R4c+gate / outage | arrivals | 160.82 / 160.83 | pass | 0.299 | 1 | 0 | 0 | 2, unresolved |
| R4c+gate / outage | off seat | 60.18 / 60.17 | pass | 0.299 | 1 | 0 | 1 | 1, unresolved |

(quantile columns are defects |Q_fine − Q_coarse| in devices; budgets 2, 2, 3, 4; bold = breach.)

What it shows:

- In the outage regime the mean-field tier **passes the mean test** (D² ~ 10⁻⁸) and **fails every
  resolved distributional bound**. Fine p95 of devices off the seat is 83, the mean-field tier says
  69: a seat provisioned from the coarse tail would be 14 devices per cadence short whenever the
  network is down. The certificate refuses; the mean-only certificate would have certified it.
- The refined tier (one shared factor for network 0) is certified again, at W1 0.299.
- Using the Expected certificate for the outage regime is refused by name:
  `OutsideEnvelope { broken: [FailureRegime { value: CorrelatedAccessLoss{Expected, 3},
  certified: [Standard(Expected)] }] }`.
- The trigger, given one live coarse state from the outage run (network 0 down for 10 cadences)
  while the operator still declares the regime Expected, finds the outage from the cells alone:
  `Refine` on `home node/net0` (occupancy 0, certified [1, 1]) and `laptop/net0` (0, certified
  [0.033, 0.567]); no whole-tier bound broke, no other cell is sent back.

**Honest record of the one change after the first run.** The budgets were set before any run and
were not moved. In the first run the Expected-regime certificate refused at p99.9 of arrivals
(defect 5 > 4), with 11.5 fine order statistics beyond that level, from autocorrelated cadences.
Rather than widen the budget, the resolution floor of §2 was added (30 order statistics, MODEL,
a rule of thumb): at 4 days p99.9 is reported unresolved in every row and is not certified. The
Expected p99 of arrivals sits exactly at its budget (3 of 3). Without the floor the Expected tier
would be refused, and that refusal would have been a statement about the sample, not the tier.

## 7. Status: what is checked, and by what

| Statement | Lean (CIRISHolon) | Numerically, by these tests | Neither |
|---|---|---|---|
| Exact closure ⇔ no non-factoring pair | `Object.nonfactoring_iff_not_closed` | `holon::witness_pair_detection_when_not_closed`, `holon_demo::witness_pair_…` | |
| Exact tiers stack | `Tiers.closed_comp`, `Object.Tier.stack` | `holon::tier_stack_composition` | |
| RMS horizon budget, linear for K ≤ 1 | `StatClosure.iterate_dist_le_horizonBudget`, `Budget.horizonBudget_le_of_nonexpansive` | `holon::lipschitz_gate_and_horizon_budget` | |
| A claim map does not carry a certificate | `Transport.claim_transport_does_not_grant_certificate` | `holon::second_square_fence_…`, `holon_dist::refused_outside_each_envelope_dimension` | |
| A finer view separates a pair the coarser view collides on | `ClosureLadder.separation_dissolves_collision` (exact views) | `holon_demo::witness_pair_…` | |
| The `D_v² ≤ β` ratio test and `K ≤ 1.05` gate as `holon.rs` computes them | no: Lean's `Closed` is exact; the 1.05 appears only in `Budget.lean`'s prose | constants only (`cewp-cli` phase 5) | |
| W1 exact on samples; lower quantiles | no | `w1_hand_cases`, `w1_is_order_independent…`, `quantiles_are_lower_order_statistics` | |
| W1 / quantile defects add under composition | no (proved by hand on `compose`) | `composition_bounds_the_composed_defect` | |
| The C→D defect applies to the C-law X→C feeds it | no | no | **neither**: envelope containment is the engineering stand-in (MODEL) |
| Interval propagation through the cohort map | no | `interval_propagation_is_monotone_and_brackets_the_samples` | |
| Envelope refusal per dimension | no | `refused_outside_each_envelope_dimension` | |
| The demonstration's numbers | no | `mean_passes_tail_refuses_under_correlated_access_loss` (regression) | |
| That the twin's outage model resembles a real access-network outage | no | no | **neither** (MODEL; needs data, §8) |

No Lean theorem covers anything this package adds.

## 8. Parameters

| name | value | class | source / why |
|---|---|---|---|
| `FP_ONE` | 10⁶ | (convention) | fixed-point scale |
| quantile levels | p50, p95, p99, p99.9 | (convention) | REVIEW 1.A |
| `MEAN_BETA` | 0.02 | SOURCED | FSD §4.5 R4 row, `D_v² ≤ 0.02` |
| `COHORT_LIPSCHITZ_K` | 1.0 | DERIVED | a sum of cell counts is 1-Lipschitz in ℓ¹ |
| W1 budget | 1 device | MODEL | set before the first run; not moved |
| quantile budgets | 2, 2, 3, 4 devices (p50, p95, p99, p99.9) | MODEL | set before the first run; deeper tails have fewer samples behind them |
| `DEMO_MIN_TAIL_SAMPLES` | 30 order statistics | MODEL | **added after the first run** (§6); a rule of thumb, no effective-sample-size correction (FRONTIER) |
| `ACCESS_NETWORKS` | 3 | MODEL | providers on one street; no data |
| `OUTAGE_UP_MEAN_S` | 6 h | MODEL | a few dozen outages per week of sim; not a real outage rate |
| `OUTAGE_DOWN_MEAN_S` | 1.5 h | MODEL | as above |
| outage effect | home nodes and laptops of the network's households suspended; phones untouched | MODEL | phones fall back to cellular |
| `DEMO_DENSITY`, `DEMO_SEED` | 3,000 /km², 43 | MODEL | the micro tests' urban case |
| `WARMUP_S` | 600 s | MODEL | homing links come up |
| `COARSE_OVERSAMPLE` | 4 | (convention) | coarse draws per fine sample |
| regression horizon | 4 days per regime | (convention) | 11,520 cadences |
| midpoint case interval | [0.70, 0.95] | MODEL | FSD §4.4 p_offline 5%..30% read as an input interval |

Every R5 parameter is `micro.rs` `PARAM_SPECS` at `MicroParams::production_shape()`.

## 9. What real data would replace

- The access-network split and outage chain: a per-provider outage history (start, duration,
  footprint) for residential fixed-line access. Not requested: any such dataset (e.g. public
  ISP outage trackers, RIPE Atlas disconnection events) needs Eric's approval (RULES 1); the
  loader seam is `holon_demo::access_network` and the two `OUTAGE_*` constants. The full failure
  library is package `failures`.
- The budgets: a production consumer of the coarse tier's tail (the seat's provisioning
  headroom) would set them; today they are MODEL.

## 10. What this cannot say (its own envelope)

- The demonstration certifies two observables (seat round arrivals and devices off the seat, per
  30 s cadence) for one patch, one seed, one workload, one topology (one seat, k = 1), and two
  regimes, over 4 days. Nothing else is certified, and the fence says so.
- CI runners are not in the certified set; their arrival stream at the seat is not certified.
- Per-device reachable peers, latency, bandwidth, durability: not certified (no observable spec).
- The fine samples are cadences of one long run, autocorrelated (phone dwell 900 s / 5,100 s,
  laptop 2,400 s); the effective sample count is well below the cadence count, so the p99.9
  estimate is coarse. Budgets were not widened for it; at 4 days p99.9 falls under the
  resolution floor and is reported unresolved, not certified.
- The composition rule is proved; its precondition (the second tier's measured defect applies
  to the law the first feeds it) is not.

## 11. Hooks (HOOKS.md)

**Energy counters.** This package has no traffic events of its own: it compares samples of
observables other tiers produce, so it counts no bytes, wakes, signatures or storage. A
certificate that later covers an energy observable (e.g. radio-awake seconds per phone-hour from
the `energy` package's counters) would need, in its envelope: the device-class mix (phone share
dominates radio time), the link-class mix (cellular vs Wi-Fi), the workload range, and the
failure regime (an outage moves phones to cellular, which is exactly the correlated shift the
mean-field tier misses here); and per-class cells, so the trigger can send back the class that
left, not the whole tier.

**Globe view** (`holon_view.rs`, in the layout `twin-globe` asked for; strides, fields and a
class per field in the doc comments). `holon_view_cells(seed) -> Vec<f32>`, stride 8:
`[lat, lon, half, level, state, bound, margin, defect]`. Record 0 is the coarse tier as a whole
(the patch square); records 1.. are the coarse cells. `state` is 0 certified inside the
envelope, 1 refused (sent back to the finer tier), 2 not evaluated. `bound` indexes
`BOUND_NAMES` (`holon_view_bounds()` gives the same table as numbers). `margin` is signed, in
online share of the cell, 0 at the bound. `defect` on record 0 is the tier's measured worst
normalised defect in the outage regime. `holon_view_certificates(seed)` gives the per (tier,
observable) table with a state per level (within, breach, unresolved). The scenario is 2 days
per regime; seed 43 is the reported one.

Cells are (host kind × access network), not places: their devices are spread over the whole
generated patch, at most 1.3 km across (`MicroWorld::half` is clamped to 70..650 m). They cannot align with the page's 1.5° grid
or `tiling.rs` `HolonId` cells. The view lays them out as a grid of sub-squares inside the patch
square (row = kind, column = network), class MODEL (a drawing choice). Coordinates: only the
patch centre (a public city centre at coarse grain) and that layout; no device or household
position leaves the module.
