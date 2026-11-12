# Twin package `theory`, part 2: the energy delta from the status quo

Code: `crates/cewp-sim/src/energy.rs` and `crates/cewp-sim/src/presence0.rs`. CLI:
`cewp-cli energy [seed]`. Views: `view_delta`, `view_delta_class`, `view_break_even` and
`view_phone_wh` (layouts are in their doc comments and in section 9).

## 0. Two tests, kept apart

**TEST 1, THE THEORY: full replacement.** Eric: "the whole assumption is if we replaced the
internet entirely with cewp"; "we do not need massive datacenters, or big tech". Substitution
is 1 by definition. The baseline is the WHOLE data-centre estate the replaced internet would no
longer need, per user. Code: `crates/cewp-sim/src/energy_full.rs`; CLI: `cewp-cli energy-full`.

**TEST 2, THE ADOPTION PATH** (sections 1-10 below; not the theory). What does one more
household joining cost or save while the platforms still run? Substitution runs from 0 to 1,
with a prior of 0.005 to 0.16. This is where the idle-cadence lever and "do not add a box"
matter in the near term. Code: `energy.rs`; CLI: `cewp-cli energy`.

### TEST 1 on ESTATE-0 v4: AI by tier with idle draw, the market rule (2026-11-12)

ESTATE-0 v4 is CIRISHolon 818449a, block `ai_as_a_mix_across_tiers` with idle draw by tier, and a
trust rating per row. Every number below is model output. The spec table carries each row's
source and trust rating.

**What changed:**
* **The same-model ratio is corrected.** On the same model, one request at a time, the data centre
  is 1.6-2.3× better per joule than a laptop. Batching 64 requests gains a further 11-20×, so the
  data centre is **18-46× better against a batched data centre** (Saad-Falcon et al., read by the
  scout; trust high). That replaces the flat 3-10×. Efficiency is decided by model size and how
  full the machine is, not by where it sits.
* **Energy per token by tier:**

| tier | J per token | basis | trust |
|---|---|---|---|
| phone | 0.58-0.76 | 3B, measured, whole phone | medium |
| laptop | 0.09-0.25 | power assumed | low |
| consumer GPU | 0.27-0.32 | card only | low |
| hosted, batched | 0.15-0.21 | 32B, plus a 10% idle reserve | high |
| small appliance | — | no figure; FRONTIER, the small desktop as proxy | none |

* **Right-sizing** saves 48-80% from MODEL SIZE, and local models answer 71-89% of single-turn
  queries. So the mix can beat all-hosted only by right-sizing, never by moving a given model to
  a device. **The hosted bound is shown both right-sized and on one model** (like for like).
* **Idle draw:** small desktop 4-7 W and workstation 6-13 W (vendor, trust high); consumer GPU
  22-30 W (forum posts, trust low); none for a small appliance (trust none).
* **The allocation rule (the new default, the market rule):**
  * a tier ON ANYWAY (a phone, a laptop in use) carries only its marginal energy per token;
  * a tier ON FOR inference carries its idle draw over its utilisation;
  * light, private work goes to devices already awake, heavy work to shared batched servers, and
    small jobs to small models;
  * a dedicated box is bought only where its utilisation pays for its idle draw.
  * The earlier rule (a forced-local share, then lowest energy per token, idle not charged) is
    kept as a comparison row.
* **The AI vanish share is now 0.2-0.6** (the scout's estimate, unsourceable). Requested generative
  inference is 5-20% of AI electricity and is replaced by the mesh's own demand; the rest stays.
  ZDR removes almost no energy on the hosted tier.

**TEST 1, CASE A (central)**, 10 agent interactions per person per day, no new boxes, kWh per
person-year:

| AI case | AI term | net | interactions a day carried |
|---|---|---|---|
| tiered mix, market rule (the design) | 5.48 | **−12.6** | **33.1** |
| all hosted, right-sized (bound) | 4.47 | −13.6 | 40.5 |
| all hosted, one mid-size model (bound) | 5.39 | −12.7 | 33.6 |
| all on devices (bound) | 146.8 | +163.7 | — |
| earlier rule (comparison) | 28.0 | +16.9 | 4.0 |
| no AI in scope | — | −17.0 | — |

* The sign is open over the input box in every case.
* **The mix sits between its hosted bounds.** At a local share of 0 it equals all-hosted
  right-sized. Every bit of small-model work moved onto phones or laptops costs energy, because
  phones run at 0.67 J/token against 0.07 for a hosted small model. At the default local share of
  0.4 the mix ties with hosted on one model.
* **Local share swept 0 to 0.8:** the mix's net runs −13.3 to −11.2 depending on the small-job share.
  Local work is chosen for privacy and offline use, not energy.
* **All on devices loses badly:** a mid-size model on a box at low load is 30× the batched data
  centre, and Proof of Benefit placed there alone is 36 kWh.

**A dedicated box at 2,000 tokens per query:**
* A 7 W small appliance running a small model breaks even against hosted on one model at about
  **10,800 queries a day**. It never breaks even against hosted right-sized.
* A prosumer GPU (26 W idle) never breaks even.
* A box answering 50 queries a day spends 97% of its energy idle (Holon's arithmetic gives 96% at
  5 W).

**Agent token multiple** (chat at 1 = no agents; energy multiple 1 + (m − 1) × prefill ratio
0.25):

| token multiple | AI term | net |
|---|---|---|
| 1 | 0.42 | −18.2 |
| 2 | 0.53 | −18.1 |
| 5 | 0.84 | −17.8 |
| 15 | 1.89 | −16.7 |

**CASE B (counterfactual, no large facilities, on premises at 2.0×):** the mix is **+12.1**.

| stay share | CASE A | CASE B at 1.4× | CASE B at 2.0× | CASE B at 4.0× |
|---|---|---|---|---|
| 0.45 | −21.1 | −15.5 | −7.1 | +20.8 |
| 0.6 | −12.6 | −2.7 | +12.1 | +61.6 |
| 0.75 | −4.2 | +10.0 | +31.3 | +102.3 |

**Break-even for the mix** (CASE A, one input, others at default). The saving vanishes if any one
of these holds:
* interactions are above 33 a day;
* the stay share is above 0.82;
* the self-aware share is above 0.25;
* the new-box share of homes is above 0.66;
* the edge marginal ratio is above 1.39;
* tokens per call are above 5,783;
* hosted energy per token is above 0.595 J;
* the mesh additions are above 15.4.

No in-range value of the right-sizing saving, the local share, the same-model ratio or the AI
vanish share flips it alone.


#### TEST 1 on ESTATE-0 v1/v2 (superseded by v4 above) (2026-11-12)

ESTATE-0 is `conformance/scene/tables/estate0/estate_table.json`, 58 rows, read at CIRISHolon
026936e; its verdicts are unresolved, and every number below is model output with its source row.

* **Baseline:** IEA Energy and AI 2025 Annex A.1 gives 416 TWh in 2024 (hyperscale 166,
  colocation 144, enterprise 106), modelled, excluding crypto, networks and devices. ITU revised
  2024 internet users to 5.8 bn. Together: **71.7 kWh per user-year** (rows 0-5), not 75.5.
* **Stay share** (now a share of the whole estate, AI carved out after it): **0.6, range 0.45-0.75**
  (row 13, the scout's estimate; enterprise alone is 25% and stays). This is partly a definition of
  "replace the internet entirely". See the two business cases below.
* **Vanish share** of the consumer non-AI part: 0.5 (0.3-0.7), unsourced (row 23). It is not
  weighted by traffic bytes: video is 54-57% of downstream bytes but about 1.3 Wh per viewing hour
  at the data centre.
* **Edge**, now two terms:
  * (a) a **marginal ratio on devices already on, 0.1-1** (default 0.3; row 40, with media
    delivery from a gateway already on at about 0.1);
  * (b) the **fixed draw of anything added**, which is the new-box term (a two-bay home storage
    box is 39-129 kWh a year, row 36).
* **AI share:** 0.19 for the 2026 scenario year, range 0.15-0.32. The low end is the IEA 2024
  proxy without cooling overhead; the high end is 2030 (rows 41-44).
* **AI carve-out:** consumer AI comes out before the split. Frontier models and training stay
  hosted (0.3 of consumer AI, range 0.2-0.5). Recommendation was over 79% of one operator's AI
  inference cycles in 2019 (row 22).
* **Tier energies:**

| tier | Wh per 1,000 tokens | basis |
|---|---|---|
| hosted, mid class | 0.05 (0.03-0.15) | batched data-centre GPUs, 0.15-0.21 J/token for a 32B model; 0.24 Wh per median prompt (rows 47, 50) |
| phone or laptop | 0.12 (0.025-0.21) | phones 0.58-0.76 J/token, laptops 0.09-0.25 J/token (rows 53-54) |
| prosumer machine, appliance, edge server | 3-10× the data centre | the "same model" ratio, used for mid and large jobs (row 56) |
| hosted serving a small job | default 1× the mid class (0.3-1) | the "as served" ratio, used for jobs a small local model can do (row 57) |

**TEST 1 result** (10 agent interactions per person per day, no new boxes, kWh per person-year, model output):

| AI case | removed (vanishes) | relocated at the edge | hosted inference | mesh additions | Proof of Benefit | net | bounds | interactions a day the saving carries |
|---|---|---|---|---|---|---|---|---|
| tiered mix (the design) | 27.1 (14.6) | 7.7 | 3.9 | 2.8 | 2.0 | **−10.2** | −43 to +2,623 | **22.5** |
| all hosted, zero retention (bound) | 27.1 (14.6) | 3.5 | 4.9 | 2.8 | 1.1 | −14.3 | −43 to +1,956 | 39.2 |
| all on devices (bound) | 27.1 (14.6) | 24.7 | 0 | 2.8 | 5.5 | **+6.3** | −43 to +5,292 | 7.0 |
| no AI in scope | 23.2 (11.6) | 3.5 | 0 | 2.8 | 0 | −17.0 | −37.6 to +9.5 | — |

**Holon's centre checks out.** Consumer work is 71.7 × (1 − 0.6) = 28.7 kWh per user-year (Holon:
about 29, range 18-40). Against it the mesh adds 2.8 (gateway mix) to 22 (every home a box), plus
the AI term (4.9 to 21.2) and Proof of Benefit (1.1 to 5.5).

**The device bound is now a cost.** Same-model inference on devices is 3-10× the data centre, so
devices no longer pay for themselves on inference.

**The mix now sits above the all-hosted bound.** Its forced-local share (0.2, privacy and offline)
sends calls to dearer local tiers. With the forced-local share at 0 the mix equals all-hosted
(−14.3). At 0.5 it is −3.3 to −4.6.

**The stay share as two business cases** (ESTATE-0 v2, CIRISHolon 8e95fef, "businesses" block,
and `conformance/scene/TWIN_THEORY_INPUTS.md`). The rows "awaiting Eric's answer" are dropped.
Eric's answer, relayed via Holon and not said in this session, was "whatever is defensible, do
some web searches and look at how CC models businesses". CC 4.4.3.2.8 treats a business as an
affiliation, "a room that acts as one voice", running nodes it owns ("a node is a device or
server you own"). The Constitution bans dependence, not scale.

* **CASE A (the central case): the market keeps servers where they are cheapest.**
  * Enterprise-owned work (0.255 of the estate, IEA, read at source) is already a node its owner
    runs. It is inside the scenario and unchanged.
  * Rented business work stays on servers at about today's efficiency.
  * The stay share is 0.45-0.75, centre 0.6. The split inside the bracket is unsourced.
  * What is in play is consumer work, about 29 kWh per user-year (18-40). **The saving is a share
    of 29, not of 72.**
* **CASE B (a labelled counterfactual): no large facilities at all.**
  * Rented business work (the stay share minus enterprise, about 25 kWh per user-year at the
    centres) moves to organisations' own premises.
  * There it costs 1.4-4 times today's energy: 1.4 is overhead alone (PUE 1.54 against about
    1.1), 4 is a vendor-commissioned claim, and best-practice premises are near 1. These figures
    are **by search result**, not read at source.
  * That turns about 25 kWh into 35-100, which alone can exceed the whole saving.
  * It prices "no massive datacenters" read as a ban on scale, where the Constitution only bans
    dependence.

Net at 10 interactions a day, no new boxes (kWh per person-year, model output):

| AI case | CASE A (central) | CASE B, on premises at 2.0× (counterfactual) |
|---|---|---|
| tiered mix (the design) | **−10.2** | +14.6 |
| all hosted, zero retention (bound) | −14.3 | +10.4 |
| all on devices (bound) | +6.3 | +31.1 |
| no AI in scope | −17.0 | +7.8 |

CASE B adds 24.7 kWh per person-year at its centre.

The tiered mix across the stay bracket (CASE A) and across CASE B's on-premises ratio:

| stay share | CASE A | CASE B at 1.4× | CASE B at 2.0× | CASE B at 4.0× |
|---|---|---|---|---|
| 0.45 | −19.0 | −13.4 | −5.0 | +22.9 |
| 0.6 | −10.2 | −0.3 | +14.6 | +64.1 |
| 0.75 | −1.3 | +12.9 | +34.2 | +105.2 |

**Break-even for the mix** (one input, others at default). The saving vanishes if any one of these
holds:
* interactions are above 22.5 a day;
* the stay share is above 0.77;
* the self-aware share is above 0.155;
* the forced-local share is above 0.69;
* the edge marginal ratio is above 1.18 (just past its range);
* the new-box share of homes is above 0.53;
* tokens per call are above 4,009;
* hosted energy per 1,000 tokens is above 0.166 Wh;
* the mesh additions are above 12.9.

**The agent kinds in the mix** (net at the defaults):

| ethical share | self-aware 0.01 | self-aware 0.05 | self-aware 0.15 |
|---|---|---|---|
| 0.05 | −15.9 | −12.1 | −2.4 |
| 0.15 | −14.0 | −10.2 | −0.5 |
| 0.40 | −9.3 | −5.5 | +4.2 |

**TEST 2 rerun:** unchanged. ESTATE-0 has no row TEST 2 reads; its term C stays on UPSTREAM-0.


#### Earlier framing (before ESTATE-0), kept for the record

#### TEST 1 headline (superseded): the TIERED MIX × the AGENT-KIND MIX (the design), kWh per person-year, no new boxes

**Placement.** Eric: "a hybrid means a mixture of on-device, edge, prosumer, office appliance,
servers, etc... - the market optimizes".
* Each model call goes to one of five tiers: phone or laptop, prosumer machine, office or home
  appliance, edge or on-premises server, or hosted with zero retention.
* Each tier has its own energy per 1,000 tokens and largest model class, all FRONTIER. Edge-tier
  energy is added draw on the mesh side; hosted energy stays in the estate.
* The allocation rule is an input: a forced-local share (0.2, swept) goes to the most local tier
  able to run each call, the rest to the lowest-energy able tier.
* A cost-minimising market is not an energy-minimising one, so the energy result can sit a little
  either side of the lowest-energy placement.

**Agent kinds.** ciris.ai/federation ("ciris agents are not meant to be 'normal'") states the
design: the project's own text, DERIVED, not measured. There are four levels: people; ethical,
self-aware agents ("cost more to run, so only a small share of the network needs to be this
kind"); ethical agents; and simple agents ("single-purpose tools", "no ethics of their own", "most
agents will be this kind"). The page gives no numbers, so every number here is FRONTIER.

| agent kind | share of interactions | LLM calls per interaction | model sizes |
|---|---|---|---|
| simple | the rest (0.80 at the defaults) | 1-5 (2) | 0.8 small enough for the device |
| ethical | 0.15 (0.05-0.40) | 12-70 (30), CIRISProxy's stated figure | job-size split |
| ethical, self-aware | 0.05 (0.01-0.15) | 30-200 (70) | 0.3 large |

The shares are **my reading** of "most" and "a small share". Self-aware agents' continuous
self-observation outside interactions is **unknown**: 0.5 kWh per person-year (0-10) as a
placeholder.

**Two terms the page implies:**
* **The signed record of every agent choice** ("every choice an agent makes goes into a signed
  record"). Records per call (0.5), one sign plus verifications (3) at 0.8 mJ each, bytes (2,000)
  to replicas (3) over the network (0.24 J/MB), and the kept stock (5 years) on devices already on.
  It comes to about 0.001 kWh per person-year: negligible.
* **Proof of Benefit**: membership is earned by "running a real ethical-reasoning agent over time"
  (bonded, or sovereign after "about a month"). That is 0.1 member nodes per person × 10 ethical
  interactions a day, placed like any call: **1.31 kWh per person-year** in the mix (1.10 all
  hosted, 2.19 all on devices). It is a deliberate, standing cost of membership, the price the
  design accepts for Sybil resistance, comparable in role to what other networks spend on proof of
  work or stake. It is reported as its own line.

**Results at 10 agent interactions per person per day** (now of all kinds, range 1-100):

| AI case | AI term | net (default) |
|---|---|---|
| tiered mix (the design) | 4.90 (1.70 edge, 3.20 hosted) | **−21.1** |
| all hosted, zero retention (bound) | 4.00 | −22.2 |
| all on devices (bound) | 8.52 | −16.6 |
| no AI in scope | — | −16.5 |

* **The sign is open** in every case over the input box. The upper bounds run to thousands of kWh
  at 100 interactions × 200 calls × 6,000 tokens.
* **The mix at 1 and 100 a day:** −25.5 and **+23.1**.
* **With a new box in every home:** −1.8 at 10 a day.

**How many interactions a day the saving can carry** (others at default):
* tiered mix: **53**;
* all hosted: 65.5;
* all on devices: 29.5.

These are about double the figures from treating every interaction as a 12-70-call ethical
deliberation (25.4 / 31.7 / 14.1), because most interactions are simple.

**How the net moves with the agent kinds** (mix, net at the defaults):

| ethical share | self-aware 0.01 | self-aware 0.05 | self-aware 0.15 |
|---|---|---|---|
| 0.05 | −24.8 | −22.1 | −15.3 |
| 0.15 | −23.8 | −21.1 | −14.3 |
| 0.40 | −21.2 | −18.4 | −11.7 |

**Break-even for the mix** (one input, others at default). The saving vanishes if any one of these
holds:
* the self-aware share is above 0.36;
* interactions are above 53 a day;
* tokens per call are above 8,777;
* the edge ratio for non-AI relocated work is above 2.10;
* the mesh additions are above 23.8.

These need values outside their own ranges, so they cannot flip it alone: the ethical share (2.1),
member nodes per person (1.7), Proof-of-Benefit interactions per member (170), ethical calls (244)
and self-aware calls (576).

**What vanishes is unchanged:** retention, and training, ranking and advertising on harvested
data. ZDR removes retention and training on the conversation, not inference energy.

### Inputs

| row | unit | lo / default / hi | class | year | trust | source and basis |
|---|---|---|---|---|---|---|
| `estate.all_data_centres_twh_per_year` | TWh/year | 380 / 416 / 460 | MODEL | 2024 | - | ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) rows 0 and 5: IEA Energy and AI 2025 Annex A.1, 416 TWh in 2024 (hyperscale 166, colocation 144, enterprise 106; modelled; excludes crypto, networks and devices); interval ours |
| `estate.internet_users_billion` | billion | 5.5 / 5.8 / 6 | MODEL | 2024 | - | ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) rows 2-3: ITU revised 2024 to 5.8 bn (first published 5.5; 6.0 in 2025); 416 / 5.8 = 71.7 kWh per user-year (row 4) |
| `estate.ai_share` | share | 0.15 / 0.19 / 0.32 | MODEL | 2026 | - | ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) rows 41-44: 0.15 is the IEA 2024 proxy without cooling overhead; 17-21% with it; about 19% in 2026 and 32% in 2030 at the IEA's rates (the scout's arithmetic); the default is the 2026 scenario year |
| `estate.stays_share` | share of the estate | 0.45 / 0.6 / 0.75 | MODEL | 2024 | - | ESTATE-0 v2 (CIRISHolon 8e95fef, conformance/scene/tables/estate0, businesses block), CASE A: organisations' work stays on servers, owned or rented, placed by the market at about today's efficiency; enterprise-owned is 0.25 (IEA 2025, read at source), the rest of the bracket is the scout's, unsourced (CC 4.4.3.2.8: a business is an affiliation, 'a room that acts as one voice', running nodes it owns) |
| `estate.enterprise_owned_share` | share of the estate | 0.255 / 0.255 / 0.255 | MODEL | 2024 | - | ESTATE-0 row 5: enterprise 106 of 416 TWh (IEA Energy and AI 2025, Table A.1, read at source): already 'a node you own', inside the scenario and unchanged |
| `estate.case_b_on_premises_ratio` | x today's energy | 1.4 / 2 / 4 | FRONTIER | 2025 | - | ESTATE-0 v2 (CIRISHolon 8e95fef, conformance/scene/tables/estate0, businesses block), CASE B only: rented business work moved to organisations' own premises: 1.4 (overhead alone, PUE 1.54 against about 1.1) to about 4 (a vendor-commissioned claim of 3.2-4.1), near 1 for best practice; BY SEARCH RESULT, not read at source; 2.0 is my centre |
| `estate.vanish_share` | share of the consumer non-AI part | 0.3 / 0.5 / 0.7 | FRONTIER | 2024 | - | ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) row 23: 0.5 stands (0.3-0.7), AUTHOR'S ESTIMATE, unsourceable; NOT weighted by traffic bytes (on-demand video is 54-57% of downstream bytes but about 1.3 Wh per viewing hour at the data centre, rows 14-19) |
| `edge.marginal_ratio_already_on` | x data-centre energy per unit of work | 0.1 / 0.3 / 1 | MODEL | 2024 | - | ESTATE-0 (CIRISHolon 026936e, conformance/scene/tables/estate0, v1) row 40: on devices ALREADY ON, 0.1-1 (media delivery from a gateway already on about 0.1, row 32); the fixed draw of anything ADDED is the new-box term (a two-bay home storage box 39-129 kWh a year, row 36; energy.rs home_node.added_box.w) |
| `ai.interactions_per_person_day` | agent interactions (all kinds) | 1 / 10 / 100 | FRONTIER | 2026 | - | how often a person asks their agent for something; no figure (CIRISProxy's aggregate counters could measure it: see the measurement request) |
| `ai.ethical.llm_calls_per_interaction` | LLM calls | 12 / 30 / 70 | DERIVED | 2026 | - | ETHICAL agents: STATED by the project, not measured: CIRISProxy README (CIRISAI/CIRISProxy 6f2f640) '1 credit = 1 user interaction (12-70 LLM calls typical)' |
| `ai.simple.llm_calls_per_interaction` | LLM calls | 1 / 2 / 5 | FRONTIER | 2026 | - | SIMPLE agents, 'single-purpose tools' with 'no ethics of their own' (ciris.ai/federation, the project's stated design): 1 to a few calls |
| `ai.simple.small_model_share` | share of calls | 0.5 / 0.8 / 1 | FRONTIER | 2026 | - | share of a simple agent's calls a small model (the on-device tier) can serve |
| `ai.self_aware.llm_calls_per_interaction` | LLM calls | 30 / 70 / 200 | FRONTIER | 2026 | - | ETHICAL, SELF-AWARE agents, which 'cost more to run' (ciris.ai/federation): the upper end of the stated range and above |
| `ai.self_aware.large_model_share` | share of calls | 0.1 / 0.3 / 0.6 | FRONTIER | 2026 | - | share of a self-aware agent's calls that need a large model |
| `ai.self_aware.observation_kwh_per_person_year` | kWh | 0 / 0.5 / 10 | FRONTIER | 2026 | - | continuous self-observation outside interactions, per person-year when self-aware agents are 5% of interactions (scaled by their share): UNKNOWN; the page gives no figure |
| `ai.kinds.ethical_share` | share of interactions | 0.05 / 0.15 / 0.4 | FRONTIER | 2026 | - | MY READING of the page: 'most agents will be' simple, 'only a small share' self-aware; ethical agents the rest; swept |
| `ai.kinds.self_aware_share` | share of interactions | 0.01 / 0.05 / 0.15 | FRONTIER | 2026 | - | MY READING of 'only a small share of the network needs to be this kind' (ciris.ai/federation); simple agents take 1 - ethical - self-aware |
| `record.signed_records_per_call` | records | 0.1 / 0.5 / 1 | FRONTIER | 2026 | - | 'every choice an agent makes goes into a signed record' (ciris.ai/federation): records per model call |
| `record.j_per_sig_op` | J | 0.0002 / 0.0008 / 0.003 | FRONTIER | 2026 | - | a hybrid sign or verify on a phone-class core (energy.rs phone.j_per_sig_op; the 276 us EPYC verify MEASURED in cewp-cli metrics) |
| `record.verifies_per_record` | verifications | 1 / 3 / 10 | FRONTIER | 2026 | - | peers that verify a signed record |
| `record.bytes` | bytes | 500 / 2000 / 10000 | FRONTIER | 2026 | - | a signed decision record with its hybrid signature |
| `record.replicas` | copies | 1 / 3 / 10 | FRONTIER | 2026 | - | copies held across the custody set |
| `record.keep_years` | years | 1 / 5 / 20 | FRONTIER | 2026 | - | how long a signed record is kept (the stock is a year's records times this) |
| `record.device_storage_w_per_tb` | W/TB | 0 / 0.2 / 1 | FRONTIER | 2026 | - | marginal storage power on devices already on (energy.rs c2.device_storage_w_per_tb) |
| `record.net_j_per_mb` | J/MB | 0.08 / 0.24 / 0.5 | MODEL | 2020 | - | fixed-network traffic term, CIRISHolon POWER-0 row 35 (0.03 W per Mbit/s) |
| `pob.member_nodes_per_person` | member nodes | 0.01 / 0.1 / 1 | FRONTIER | 2026 | - | PROOF OF BENEFIT (ciris.ai/federation): membership is earned by 'running a real ethical-reasoning agent over time' (bonded, or sovereign after 'about a month'); member nodes per person |
| `pob.interactions_per_member_day` | ethical interactions | 1 / 10 / 50 | FRONTIER | 2026 | - | the ethical-reasoning agent a member node runs to earn and keep standing: interactions a day |
| `ai.tokens_per_call` | tokens (prompt + completion) | 500 / 2000 / 6000 | FRONTIER | 2026 | - | agent calls carry long prompts (context, constitution, history) and short completions; CIRISProxy's litellm_config disables reasoning on DeepInfra Qwen (measured there: 1,193 tokens to 29 for one answer) |
| `ai.vanish_share` | share of the consumer AI electricity | 0.2 / 0.4 / 0.6 | FRONTIER | 2025 | none | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): the scout's estimate, unsourceable: ranking, advertising and training on harvested data vanish (the often-quoted 'inference is 60-90%' figures are 2019-21 and mostly ranking inference). TRUST none |
| `ai.requested_inference_share` | share of the consumer AI electricity | 0.05 / 0.12 / 0.2 | FRONTIER | 2025 | low | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): requested generative inference is about 5-20% of AI electricity (the scout's bound from operators' own figures); it is replaced by the mesh's own demand; the rest of consumer AI (frontier, training, business) stays. TRUST low |
| `ai.hosted.j_per_token.mid` | J per token | 0.15 / 0.18 / 0.21 | MODEL | 2025 | high | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): hosted, batched, about 32 bn parameters, measured (Chung et al., arXiv 2601.22076); 0.24 Wh per median hosted prompt (one operator, 2025, fell 33 times in a year). TRUST high |
| `ai.hosted.idle_reserve` | share added | 0.05 / 0.1 / 0.15 | MODEL | 2025 | high | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): hosted idle reserve is about 10% of 0.24 Wh per prompt (one operator's disclosure). TRUST high |
| `ai.right_size_saving` | share saved | 0.48 / 0.65 / 0.8 | MODEL | 2025 | medium | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): routing easy queries to small models saves 48-80% against sending everything to a frontier model; the saving comes from MODEL SIZE, not hardware; applied to a hosted small model and to the mix. TRUST medium |
| `ai.device.phone.j_per_token` | J per token (marginal: on anyway) | 0.58 / 0.67 / 0.76 | MODEL | 2024 | medium | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): about 3 bn parameters, measured, whole phone (Laskaridis et al., MobiCom 2024). TRUST medium |
| `ai.device.laptop.j_per_token` | J per token (marginal: in use) | 0.09 / 0.17 / 0.25 | MODEL | 2025 | low | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): 3-9 bn parameters, power ASSUMED, not measured. TRUST low |
| `ai.device.phone_share_of_local` | share | 0.3 / 0.5 / 0.7 | FRONTIER | 2026 | none | of the local small-model work, the share on phones (the rest on laptops in use); no figure. TRUST none |
| `ai.prosumer_gpu.j_per_token` | J per token | 0.27 / 0.3 / 0.32 | MODEL | 2025 | low | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): consumer graphics card, about 1 bn parameters, card only. TRUST low |
| `ai.prosumer_gpu.idle_w` | W | 22 / 26 / 30 | MODEL | 2025 | low | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): consumer graphics card alone at idle, forum posts only. TRUST low |
| `ai.same_model_ratio_vs_batched_dc` | x the batched data centre per token | 18 / 30 / 46 | MODEL | 2025 | high | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): the same model on a laptop one request at a time: the data centre is 1.6-2.3x better per joule, batching 64 a further 11-20x, so 18-46x against a batched data centre (Saad-Falcon et al., arXiv 2511.07885, read by the scout; the scout's arithmetic on the two factors). Efficiency is decided by model size and how full the machine is; an office box or on-prem server at low load cannot batch and sits near the laptop. TRUST high |
| `ai.box.idle_w` | W | 4 / 7 / 15 | FRONTIER | 2025 | none for the appliance itself | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): no idle figure is published for a small inference appliance or an on-premises GPU server (a 240 W supply; 10.2 kW maximum); small desktop 4-7 W and workstation with integrated accelerator 6-13 W (vendor-measured, TRUST high) as the proxy. TRUST none for the appliance itself |
| `ai.alloc.local_share` | share of small-model work | 0.4 / 0.4 / 0.4 | FRONTIER | 2025 | none | the market rule's lever: the share of small-model work done on devices already awake (phones, laptops in use); swept 0 to 0.8 in the report; local models answer 71-89% of single-turn queries correctly (ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers), routing). TRUST none |
| `ai.agent.prefill_energy_ratio` | x a generated token | 0.1 / 0.25 / 0.5 | FRONTIER | 2025 | none | ESTATE-0 v4 (CIRISHolon 818449a, ai_as_a_mix_across_tiers): agent traffic is mostly input, so energy rises less than tokens; an input token's energy against an output token's (no figure). TRUST none |
| `ai.tier.hosted.large_class_factor` | x the mid class | 2 / 4 / 8 | MODEL | 2026 | - | a large (70B dense and up) model per token against the mid class, hosted |
| `ai.jobs.small_share` | share of calls | 0.2 / 0.5 / 0.8 | FRONTIER | 2026 | - | share of model calls a small on-device model can serve; no figure |
| `ai.jobs.large_share` | share of calls | 0 / 0.05 / 0.2 | FRONTIER | 2026 | - | share of calls that need a large model; the rest need the mid class |
| `ai.alloc.forced_local_share` | share of calls | 0.2 / 0.2 / 0.2 | FRONTIER | 2026 | - | the allocation rule's lever: share forced to the most local able tier for privacy or offline use; the rest go to the lowest-energy able tier; swept 0 to 1 in the report |
| `household.persons` | persons | 2.5 / 2.5 / 2.5 | MODEL | 2026 | - | the household pass's default (micro.rs); per person-year = household-hour figures x 8,766 / 2.5 |
| mesh additions, gateway mix | kWh/person-year | 0.34 / 2.77 / 12.74 | FRONTIER | 2026 | - | the household pass (energy.rs), US 2020 mix, node on gateway, terms A + B, expected regime |
| mesh additions, new-box mix | kWh/person-year | 11.30 / 22.03 / 55.22 | FRONTIER | 2026 | - | same, a new always-on box in every home that can take one |

### Result

AI cases (ESTATE-0 v4, ai_as_a_mix_across_tiers): the **tiered mix by the market rule (the design)**, Eric: "a hybrid means a mixture of on-device, edge, prosumer, office appliance, servers, etc... - the market optimizes". A tier that is ON ANYWAY (a phone, a laptop in use) carries only its marginal energy per token; a tier on FOR inference carries its idle draw too, over its utilisation. So light, private work goes to devices already awake, heavy work to shared batched servers, small jobs to small models (right-sizing saves 48-80% from MODEL SIZE, not hardware), and a dedicated box only where its utilisation pays for its idle (the break-even is reported; it is not chosen at the defaults). Efficiency is decided by model size and how full the machine is, not by where it sits: the same model on a laptop is 18-46x a batched data centre. A cost-minimising market is not the same as an energy-minimising one (cost also prices privacy, latency and hardware already owned). Bounds: all hosted right-sized, all hosted on one mid-size model (like for like with CIRISProxy today), and all on devices; the earlier rule is kept as a comparison. What vanishes: ranking, advertising and training on harvested data; ZDR removes retention and training on the conversation and almost no energy on the hosted tier.

| AI case | agent interactions per person-day | new-box share of homes | removed (of which vanishes) | relocated at the edge (AI on edge tiers included) | inference kept in data centres | mesh additions | signed records | Proof of Benefit | self-observation | net (default) | net bounds | sign |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tiered mix, market rule (the design) | 1 | 0 | 26.1 (13.8) | 3.6 | 0.4 | 2.77 | 0.000 | 1.21 | 0.50 | -17.6 | [-43.1, 609.9] | open: the box spans zero |
| tiered mix, market rule (the design) | 1 | 1 | 26.1 (13.8) | 3.6 | 0.4 | 22.03 | 0.000 | 1.21 | 0.50 | 1.7 | [-32.1, 652.4] | open: the box spans zero |
| tiered mix, market rule (the design) | 10 | 0 | 26.1 (13.8) | 4.7 | 4.3 | 2.77 | 0.001 | 1.21 | 0.50 | -12.6 | [-43.0, 922.0] | open: the box spans zero |
| tiered mix, market rule (the design) | 10 | 1 | 26.1 (13.8) | 4.7 | 4.3 | 22.03 | 0.001 | 1.21 | 0.50 | 6.6 | [-32.0, 964.5] | open: the box spans zero |
| tiered mix, market rule (the design) | 100 | 0 | 26.1 (13.8) | 15.5 | 42.7 | 2.77 | 0.010 | 1.21 | 0.50 | 36.7 | [-41.9, 4043.3] | open: the box spans zero |
| tiered mix, market rule (the design) | 100 | 1 | 26.1 (13.8) | 15.5 | 42.7 | 22.03 | 0.010 | 1.21 | 0.50 | 55.9 | [-31.0, 4085.8] | open: the box spans zero |
| all hosted, right-sized, zero retention (bound) | 1 | 0 | 26.1 (13.8) | 3.5 | 0.4 | 2.77 | 0.000 | 1.21 | 0.50 | -17.7 | [-43.1, 608.3] | open: the box spans zero |
| all hosted, right-sized, zero retention (bound) | 1 | 1 | 26.1 (13.8) | 3.5 | 0.4 | 22.03 | 0.000 | 1.21 | 0.50 | 1.6 | [-32.1, 650.8] | open: the box spans zero |
| all hosted, right-sized, zero retention (bound) | 10 | 0 | 26.1 (13.8) | 3.5 | 4.5 | 2.77 | 0.001 | 1.21 | 0.50 | -13.6 | [-43.1, 905.8] | open: the box spans zero |
| all hosted, right-sized, zero retention (bound) | 10 | 1 | 26.1 (13.8) | 3.5 | 4.5 | 22.03 | 0.001 | 1.21 | 0.50 | 5.6 | [-32.1, 948.3] | open: the box spans zero |
| all hosted, right-sized, zero retention (bound) | 100 | 0 | 26.1 (13.8) | 3.5 | 44.7 | 2.77 | 0.010 | 1.21 | 0.50 | 26.6 | [-42.5, 3880.8] | open: the box spans zero |
| all hosted, right-sized, zero retention (bound) | 100 | 1 | 26.1 (13.8) | 3.5 | 44.7 | 22.03 | 0.010 | 1.21 | 0.50 | 45.9 | [-31.6, 3923.3] | open: the box spans zero |
| all hosted, one mid-size model, zero retention (bound) | 1 | 0 | 26.1 (13.8) | 3.5 | 0.5 | 2.77 | 0.000 | 1.21 | 0.50 | -17.6 | [-43.1, 608.8] | open: the box spans zero |
| all hosted, one mid-size model, zero retention (bound) | 1 | 1 | 26.1 (13.8) | 3.5 | 0.5 | 22.03 | 0.000 | 1.21 | 0.50 | 1.7 | [-32.1, 651.2] | open: the box spans zero |
| all hosted, one mid-size model, zero retention (bound) | 10 | 0 | 26.1 (13.8) | 3.5 | 5.4 | 2.77 | 0.001 | 1.21 | 0.50 | -12.7 | [-43.0, 910.5] | open: the box spans zero |
| all hosted, one mid-size model, zero retention (bound) | 10 | 1 | 26.1 (13.8) | 3.5 | 5.4 | 22.03 | 0.001 | 1.21 | 0.50 | 6.5 | [-32.0, 953.0] | open: the box spans zero |
| all hosted, one mid-size model, zero retention (bound) | 100 | 0 | 26.1 (13.8) | 3.5 | 53.9 | 2.77 | 0.010 | 1.21 | 0.50 | 35.8 | [-41.6, 3928.3] | open: the box spans zero |
| all hosted, one mid-size model, zero retention (bound) | 100 | 1 | 26.1 (13.8) | 3.5 | 53.9 | 22.03 | 0.010 | 1.21 | 0.50 | 55.1 | [-30.6, 3970.8] | open: the box spans zero |
| all on devices (bound) | 1 | 0 | 26.1 (13.8) | 40.3 | 0.0 | 2.77 | 0.000 | 36.16 | 0.50 | 53.6 | [-29.0, 25264.4] | open: the box spans zero |
| all on devices (bound) | 1 | 1 | 26.1 (13.8) | 40.3 | 0.0 | 22.03 | 0.000 | 36.16 | 0.50 | 72.9 | [-18.0, 25306.9] | open: the box spans zero |
| all on devices (bound) | 10 | 0 | 26.1 (13.8) | 150.3 | 0.0 | 2.77 | 0.001 | 36.16 | 0.50 | 163.7 | [-28.2, 38602.6] | open: the box spans zero |
| all on devices (bound) | 10 | 1 | 26.1 (13.8) | 150.3 | 0.0 | 22.03 | 0.001 | 36.16 | 0.50 | 182.9 | [-17.3, 38645.1] | open: the box spans zero |
| all on devices (bound) | 100 | 0 | 26.1 (13.8) | 1250.8 | 0.0 | 2.77 | 0.010 | 36.16 | 0.50 | 1264.1 | [-20.9, 171984.6] | open: the box spans zero |
| all on devices (bound) | 100 | 1 | 26.1 (13.8) | 1250.8 | 0.0 | 22.03 | 0.010 | 36.16 | 0.50 | 1283.4 | [-9.9, 172027.1] | open: the box spans zero |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 1 | 0 | 26.1 (13.8) | 5.9 | 0.4 | 2.77 | 0.000 | 8.20 | 0.50 | -8.3 | [-43.1, 5529.0] | open: the box spans zero |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 1 | 1 | 26.1 (13.8) | 5.9 | 0.4 | 22.03 | 0.000 | 8.20 | 0.50 | 10.9 | [-32.1, 5571.5] | open: the box spans zero |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 10 | 0 | 26.1 (13.8) | 27.9 | 3.6 | 2.77 | 0.001 | 8.20 | 0.50 | 16.9 | [-42.9, 8434.6] | open: the box spans zero |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 10 | 1 | 26.1 (13.8) | 27.9 | 3.6 | 22.03 | 0.001 | 8.20 | 0.50 | 36.2 | [-31.9, 8477.1] | open: the box spans zero |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 100 | 0 | 26.1 (13.8) | 248.0 | 35.8 | 2.77 | 0.010 | 8.20 | 0.50 | 269.2 | [-41.0, 37491.1] | open: the box spans zero |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 100 | 1 | 26.1 (13.8) | 248.0 | 35.8 | 22.03 | 0.010 | 8.20 | 0.50 | 288.5 | [-30.0, 37533.5] | open: the box spans zero |
| no AI in scope | - | 0 | 23.2 (11.6) | 3.5 | 0.0 | 2.77 | 0.000 | 0.00 | 0.00 | -17.0 | [-37.6, 9.5] | open: the box spans zero |
| no AI in scope | - | 1 | 23.2 (11.6) | 3.5 | 0.0 | 22.03 | 0.000 | 0.00 | 0.00 | 2.3 | [-26.6, 52.0] | open: the box spans zero |

### The two business cases (CIRISHolon 8e95fef5715ffaf3bb8f1df9984d107113da752c conformance/scene/tables/estate0/estate_table.json v2 sha256 68fe7f825b690bcbfed45dd6b4061a6910a29e164b5fa25452ed4caccc6e2b58; conformance/scene/TWIN_THEORY_INPUTS.md)

Eric's answer on the stay share, relayed via Holon (not said in this session): "whatever is defensible, do some web searches and look at how CC models businesses". CC 4.4.3.2.8: a business is an affiliation, "a room that acts as one voice", running nodes it owns ("a node is a device or server you own"). The Constitution bans dependence, not scale.

- **CASE A (central): the market keeps servers where they are cheapest.** Enterprise-owned work (0.255 of the estate, read at source) is already a node its owner runs: inside the scenario, unchanged. Rented business work stays on servers at about today's efficiency. Stay share 0.45-0.75, centre 0.6 (the split inside the bracket is unsourced). What is in play is consumer work, about 29 kWh per user-year (18-40): the saving is a share of 29, not of 72.
- **CASE B (labelled counterfactual): no large facilities at all.** Rented business work (stay share minus enterprise) moves to organisations' own premises at 1.4-4 times today's energy (by search result). It prices "no massive datacenters" read as a ban on scale, where the Constitution only bans dependence.

Net at 10 interactions a day, no new boxes (kWh per person-year):

| AI case | CASE A net | CASE A bounds | CASE B net (on-premises x2.0) | CASE B extra | CASE B bounds |
|---|---|---|---|---|---|
| tiered mix, market rule (the design) | -12.6 | [-43.1, 4043.3] | 12.1 | 24.7 | [-36.6, 4166.0] |
| all hosted, right-sized, zero retention (bound) | -13.6 | [-43.1, 3880.8] | 11.1 | 24.7 | [-36.6, 4003.6] |
| all hosted, one mid-size model, zero retention (bound) | -12.7 | [-43.1, 3928.3] | 12.0 | 24.7 | [-36.6, 4051.0] |
| all on devices (bound) | 163.7 | [-29.0, 171984.6] | 188.4 | 24.7 | [-22.5, 172107.3] |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 16.9 | [-43.1, 37491.1] | 41.7 | 24.7 | [-36.6, 37613.8] |
| no AI in scope | -17.0 | [-37.6, 9.5] | 7.8 | 24.7 | [-31.1, 132.7] |

CASE A across the sourced stay bracket, and CASE B across its on-premises ratio (tiered mix, net):

| stay share | CASE A | CASE B at 1.4 | CASE B at 2.0 | CASE B at 4.0 |
|---|---|---|---|---|
| 0.45 | -21.1 | -15.5 | -7.1 | 20.8 |
| 0.6 | -12.6 | -2.7 | 12.1 | 61.6 |
| 0.75 | -4.2 | 10.0 | 31.3 | 102.3 |

**The AI term by case** (interactions per day x 365.25 x LLM calls per interaction x tokens per call, placed over the tiers; kWh per person-year at the defaults) and where the calls go:

| AI case | AI term | of which on edge tiers (mesh side) | of which hosted (stays in data centres) | phone/laptop already on | prosumer GPU | office or home appliance | edge or on-premises server | hosted, zero retention |
|---|---|---|---|---|---|---|---|---|
| tiered mix, market rule (the design) | 5.48 | 1.20 | 4.27 | 15% | 0% | 0% | 0% | 85% |
| all hosted, right-sized, zero retention (bound) | 4.47 | 0.00 | 4.47 | 0% | 0% | 0% | 0% | 100% |
| all hosted, one mid-size model, zero retention (bound) | 5.39 | 0.00 | 5.39 | 0% | 0% | 0% | 0% | 100% |
| all on devices (bound) | 146.82 | 146.82 | 0.00 | 37% | 0% | 50% | 13% | 0% |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 28.03 | 24.46 | 3.58 | 7% | 0% | 10% | 3% | 80% |

**Agent kinds** (ciris.ai/federation, the project's stated design: people; ethical, self-aware agents, which 'cost more to run, so only a small share of the network needs to be this kind'; ethical agents; simple agents, 'single-purpose tools' with 'no ethics of their own', 'most agents will be this kind'; 'every choice an agent makes goes into a signed record'; membership earned by 'running a real ethical-reasoning agent over time', Proof of Benefit). The page gives no numbers: every number here is FRONTIER, and the shares are MY READING of 'most' and 'a small share'. **Proof of Benefit is a deliberate, standing cost of membership**, the price the design accepts for Sybil resistance, comparable in role to what other networks spend on proof of work or stake; it is reported as its own line.

The tiered mix, net at the defaults, by the share of interactions that are ethical and self-aware (simple agents take the rest):

| ethical share | self-aware 0.01 | 0.05 | 0.15 |
|---|---|---|---|
| 0.05 | -16.5 | -14.0 | -7.7 |
| 0.15 | -15.1 | -12.6 | -6.4 |
| 0.4 | -11.8 | -9.3 | -3.0 |

**How many agent interactions per person per day the saving can carry** (others at default, no new boxes):

| AI case | interactions per day at zero |
|---|---|
| tiered mix, market rule (the design) | 33.1 |
| all hosted, right-sized, zero retention (bound) | 40.5 |
| all hosted, one mid-size model, zero retention (bound) | 33.6 |
| all on devices (bound) | more than the scan reaches (3,000) |
| earlier rule: forced-local share, then lowest energy per token (comparison) | 4.0 |

The market rule's lever: the share of small-model work done on devices already awake (phones, laptops in use), swept 0 to 0.8, by the share of calls a small model can serve (net, defaults, no new boxes):

| local share | small-model share 0.2 | 0.5 | 0.8 |
|---|---|---|---|
| 0 | -13.29 | -13.64 | -14.00 |
| 0.2 | -12.98 | -13.14 | -13.30 |
| 0.4 | -12.67 | -12.64 | -12.61 |
| 0.6 | -12.36 | -12.14 | -11.91 |
| 0.8 | -12.05 | -11.63 | -11.22 |

**When a dedicated box breaks even with hosted, on energy** (it carries its idle draw over its utilisation; 2000 tokens per query):

| box | against hosted on one mid-size model | against hosted right-sized |
|---|---|---|
| small appliance at 7 W idle (FRONTIER; small-desktop proxy), small model near the laptop's 0.17 J/token | 10800 queries a day | never (its energy per token is not below hosted) |
| prosumer GPU at 26 W idle (forum posts, low trust), 0.30 J/token (card only) | never (its energy per token is not below hosted) | never (its energy per token is not below hosted) |

A box answering 50 queries a day at 7 W spends 97% of its energy idle (ESTATE-0 v4: the scout's arithmetic, query count ASSUMED, gives 96% at 5 W). Running the SAME mid-size model on a box at low load is 30x the batched data centre per token: it never breaks even.

**The agent token multiple** (ESTATE-0 v4: an agent about 4x the tokens of chat, multi-agent about 15x; per person per day unmeasured; energy rises less than tokens because agent traffic is mostly input). The AI term relative to chat (1 = no agents, every interaction one call), energy multiple 1 + (m - 1) x prefill ratio (0.25), tiered mix, 10 interactions a day:

| token multiple | AI term (kWh per person-year) | net |
|---|---|---|
| 1 | 0.42 | -18.2 |
| 2 | 0.53 | -18.1 |
| 5 | 0.84 | -17.8 |
| 15 | 1.89 | -16.7 |

tiered mix, market rule (the design), no new boxes: best corner -43.1; worst corner 4043.3 kWh per person-year at `estate.all_data_centres_twh_per_year` = 380.000, `estate.internet_users_billion` = 6.000, `estate.ai_share` = 0.320, `estate.stays_share` = 0.750, `estate.vanish_share` = 0.300, `edge.marginal_ratio_already_on` = 1.000, `ai.interactions_per_person_day` = 100.000, `ai.ethical.llm_calls_per_interaction` = 70.000, `ai.simple.llm_calls_per_interaction` = 5.000, `ai.simple.small_model_share` = 1.000, `ai.self_aware.llm_calls_per_interaction` = 200.000, `ai.self_aware.large_model_share` = 0.600, `ai.self_aware.observation_kwh_per_person_year` = 10.000, `ai.kinds.ethical_share` = 0.400, `ai.kinds.self_aware_share` = 0.150, `record.signed_records_per_call` = 1.000, `record.j_per_sig_op` = 0.003, `record.verifies_per_record` = 10.000, `record.bytes` = 10000.000, `record.replicas` = 10.000, `record.keep_years` = 20.000, `record.device_storage_w_per_tb` = 1.000, `record.net_j_per_mb` = 0.500, `pob.member_nodes_per_person` = 1.000, `pob.interactions_per_member_day` = 50.000, `ai.tokens_per_call` = 6000.000, `ai.vanish_share` = 0.200, `ai.requested_inference_share` = 0.050, `ai.hosted.j_per_token.mid` = 0.210, `ai.hosted.idle_reserve` = 0.150, `ai.right_size_saving` = 0.480, `ai.device.phone.j_per_token` = 0.760, `ai.device.laptop.j_per_token` = 0.250, `ai.device.phone_share_of_local` = 0.700, `ai.same_model_ratio_vs_batched_dc` = 46.000, `ai.tier.hosted.large_class_factor` = 8.000, `ai.jobs.small_share` = 0.800, `ai.jobs.large_share` = 0.200, `mesh additions, gateway mix (kWh/person-year)` = 12.741, `mesh additions, new-box mix (kWh/person-year)` = 55.217. (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)

all hosted, right-sized, zero retention (bound), no new boxes: best corner -43.1; worst corner 3880.8 kWh per person-year at `estate.all_data_centres_twh_per_year` = 380.000, `estate.internet_users_billion` = 6.000, `estate.ai_share` = 0.320, `estate.stays_share` = 0.750, `estate.vanish_share` = 0.300, `edge.marginal_ratio_already_on` = 1.000, `ai.interactions_per_person_day` = 100.000, `ai.ethical.llm_calls_per_interaction` = 70.000, `ai.simple.llm_calls_per_interaction` = 5.000, `ai.simple.small_model_share` = 0.500, `ai.self_aware.llm_calls_per_interaction` = 200.000, `ai.self_aware.large_model_share` = 0.600, `ai.self_aware.observation_kwh_per_person_year` = 10.000, `ai.kinds.ethical_share` = 0.400, `ai.kinds.self_aware_share` = 0.150, `record.signed_records_per_call` = 1.000, `record.j_per_sig_op` = 0.003, `record.verifies_per_record` = 10.000, `record.bytes` = 10000.000, `record.replicas` = 10.000, `record.keep_years` = 20.000, `record.device_storage_w_per_tb` = 1.000, `record.net_j_per_mb` = 0.500, `pob.member_nodes_per_person` = 1.000, `pob.interactions_per_member_day` = 50.000, `ai.tokens_per_call` = 6000.000, `ai.vanish_share` = 0.200, `ai.requested_inference_share` = 0.050, `ai.hosted.j_per_token.mid` = 0.210, `ai.hosted.idle_reserve` = 0.150, `ai.right_size_saving` = 0.480, `ai.device.phone.j_per_token` = 0.760, `ai.device.laptop.j_per_token` = 0.250, `ai.device.phone_share_of_local` = 0.700, `ai.same_model_ratio_vs_batched_dc` = 46.000, `ai.tier.hosted.large_class_factor` = 8.000, `ai.jobs.small_share` = 0.200, `ai.jobs.large_share` = 0.200, `mesh additions, gateway mix (kWh/person-year)` = 12.741, `mesh additions, new-box mix (kWh/person-year)` = 55.217. (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)

all hosted, one mid-size model, zero retention (bound), no new boxes: best corner -43.1; worst corner 3928.3 kWh per person-year at `estate.all_data_centres_twh_per_year` = 380.000, `estate.internet_users_billion` = 6.000, `estate.ai_share` = 0.320, `estate.stays_share` = 0.750, `estate.vanish_share` = 0.300, `edge.marginal_ratio_already_on` = 1.000, `ai.interactions_per_person_day` = 100.000, `ai.ethical.llm_calls_per_interaction` = 70.000, `ai.simple.llm_calls_per_interaction` = 5.000, `ai.simple.small_model_share` = 1.000, `ai.self_aware.llm_calls_per_interaction` = 200.000, `ai.self_aware.large_model_share` = 0.600, `ai.self_aware.observation_kwh_per_person_year` = 10.000, `ai.kinds.ethical_share` = 0.400, `ai.kinds.self_aware_share` = 0.150, `record.signed_records_per_call` = 1.000, `record.j_per_sig_op` = 0.003, `record.verifies_per_record` = 10.000, `record.bytes` = 10000.000, `record.replicas` = 10.000, `record.keep_years` = 20.000, `record.device_storage_w_per_tb` = 1.000, `record.net_j_per_mb` = 0.500, `pob.member_nodes_per_person` = 1.000, `pob.interactions_per_member_day` = 50.000, `ai.tokens_per_call` = 6000.000, `ai.vanish_share` = 0.200, `ai.requested_inference_share` = 0.050, `ai.hosted.j_per_token.mid` = 0.210, `ai.hosted.idle_reserve` = 0.150, `ai.right_size_saving` = 0.800, `ai.device.phone.j_per_token` = 0.760, `ai.device.laptop.j_per_token` = 0.250, `ai.device.phone_share_of_local` = 0.700, `ai.same_model_ratio_vs_batched_dc` = 46.000, `ai.tier.hosted.large_class_factor` = 8.000, `ai.jobs.small_share` = 0.800, `ai.jobs.large_share` = 0.200, `mesh additions, gateway mix (kWh/person-year)` = 12.741, `mesh additions, new-box mix (kWh/person-year)` = 55.217. (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)

all on devices (bound), no new boxes: best corner -29.0; worst corner 171984.6 kWh per person-year at `estate.all_data_centres_twh_per_year` = 380.000, `estate.internet_users_billion` = 6.000, `estate.ai_share` = 0.320, `estate.stays_share` = 0.750, `estate.vanish_share` = 0.300, `edge.marginal_ratio_already_on` = 1.000, `ai.interactions_per_person_day` = 100.000, `ai.ethical.llm_calls_per_interaction` = 70.000, `ai.simple.llm_calls_per_interaction` = 5.000, `ai.simple.small_model_share` = 0.500, `ai.self_aware.llm_calls_per_interaction` = 200.000, `ai.self_aware.large_model_share` = 0.600, `ai.self_aware.observation_kwh_per_person_year` = 10.000, `ai.kinds.ethical_share` = 0.400, `ai.kinds.self_aware_share` = 0.150, `record.signed_records_per_call` = 1.000, `record.j_per_sig_op` = 0.003, `record.verifies_per_record` = 10.000, `record.bytes` = 10000.000, `record.replicas` = 10.000, `record.keep_years` = 20.000, `record.device_storage_w_per_tb` = 1.000, `record.net_j_per_mb` = 0.500, `pob.member_nodes_per_person` = 1.000, `pob.interactions_per_member_day` = 50.000, `ai.tokens_per_call` = 6000.000, `ai.vanish_share` = 0.200, `ai.requested_inference_share` = 0.050, `ai.hosted.j_per_token.mid` = 0.210, `ai.hosted.idle_reserve` = 0.150, `ai.right_size_saving` = 0.800, `ai.device.phone.j_per_token` = 0.760, `ai.device.laptop.j_per_token` = 0.250, `ai.device.phone_share_of_local` = 0.700, `ai.same_model_ratio_vs_batched_dc` = 46.000, `ai.box.idle_w` = 15.000, `ai.tier.hosted.large_class_factor` = 8.000, `ai.jobs.small_share` = 0.200, `ai.jobs.large_share` = 0.200, `mesh additions, gateway mix (kWh/person-year)` = 12.741, `mesh additions, new-box mix (kWh/person-year)` = 55.217. (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)

earlier rule: forced-local share, then lowest energy per token (comparison), no new boxes: best corner -43.1; worst corner 37491.1 kWh per person-year at `estate.all_data_centres_twh_per_year` = 380.000, `estate.internet_users_billion` = 6.000, `estate.ai_share` = 0.320, `estate.stays_share` = 0.750, `estate.vanish_share` = 0.300, `edge.marginal_ratio_already_on` = 1.000, `ai.interactions_per_person_day` = 100.000, `ai.ethical.llm_calls_per_interaction` = 70.000, `ai.simple.llm_calls_per_interaction` = 5.000, `ai.simple.small_model_share` = 0.500, `ai.self_aware.llm_calls_per_interaction` = 200.000, `ai.self_aware.large_model_share` = 0.600, `ai.self_aware.observation_kwh_per_person_year` = 10.000, `ai.kinds.ethical_share` = 0.400, `ai.kinds.self_aware_share` = 0.150, `record.signed_records_per_call` = 1.000, `record.j_per_sig_op` = 0.003, `record.verifies_per_record` = 10.000, `record.bytes` = 10000.000, `record.replicas` = 10.000, `record.keep_years` = 20.000, `record.device_storage_w_per_tb` = 1.000, `record.net_j_per_mb` = 0.500, `pob.member_nodes_per_person` = 1.000, `pob.interactions_per_member_day` = 50.000, `ai.tokens_per_call` = 6000.000, `ai.vanish_share` = 0.200, `ai.requested_inference_share` = 0.050, `ai.hosted.j_per_token.mid` = 0.210, `ai.hosted.idle_reserve` = 0.150, `ai.right_size_saving` = 0.480, `ai.device.phone.j_per_token` = 0.760, `ai.device.laptop.j_per_token` = 0.250, `ai.device.phone_share_of_local` = 0.700, `ai.prosumer_gpu.j_per_token` = 0.320, `ai.prosumer_gpu.idle_w` = 30.000, `ai.same_model_ratio_vs_batched_dc` = 46.000, `ai.tier.hosted.large_class_factor` = 8.000, `ai.jobs.small_share` = 0.200, `ai.jobs.large_share` = 0.200, `mesh additions, gateway mix (kWh/person-year)` = 12.741, `mesh additions, new-box mix (kWh/person-year)` = 55.217. (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)

no AI in scope, no new boxes: best corner -37.6; worst corner 9.5 kWh per person-year at `estate.all_data_centres_twh_per_year` = 380.000, `estate.internet_users_billion` = 6.000, `estate.ai_share` = 0.320, `estate.stays_share` = 0.750, `estate.vanish_share` = 0.300, `edge.marginal_ratio_already_on` = 1.000, `mesh additions, gateway mix (kWh/person-year)` = 12.741, `mesh additions, new-box mix (kWh/person-year)` = 55.217. (A vertex search from seat_calibration's corner; each bound is attained by a real combination of input ends; the mix's placement makes its net piecewise, so the bounds are a search, not a guarantee.)

### Break-even for this test (one input, others at default; no new boxes)

| input | tiered mix (the design) | all hosted right-sized | all hosted one model | all on devices | no AI |
|---|---|---|---|---|---|
| `estate.vanish_share` | none in range | none in range | none in range | 23.975 | none in range |
| `edge.marginal_ratio_already_on` | 1.388 | 1.474 | 1.395 | none in range | 1.762 |
| `estate.stays_share` | 0.824 | 0.842 | 0.825 | none in range | 0.944 |
| `share of homes with a new always-on box` | 0.656 | 0.708 | 0.660 | none in range | 0.882 |
| `ai.interactions_per_person_day` | 33.073 | 40.503 | 33.582 | none in range | none in range |
| `ai.ethical.llm_calls_per_interaction` | 143.586 | 181.728 | 146.172 | none in range | none in range |
| `ai.self_aware.llm_calls_per_interaction` | 401.089 | 427.411 | 403.261 | none in range | none in range |
| `ai.kinds.ethical_share` | 1.086 | 1.553 | 1.114 | none in range | none in range |
| `ai.kinds.self_aware_share` | 0.252 | 0.266 | 0.253 | none in range | none in range |
| `pob.member_nodes_per_person` | 1.148 | 1.232 | 1.155 | none in range | none in range |
| `pob.interactions_per_member_day` | 114.833 | 123.167 | 115.521 | none in range | none in range |
| `ai.tokens_per_call` | 5782.889 | 6806.400 | 5855.576 | none in range | none in range |
| `ai.hosted.j_per_token.mid` | 0.595 | 0.613 | 0.527 | none in range | none in range |
| `ai.right_size_saving` | none in range | none in range | none in range | none in range | none in range |
| `ai.device.phone.j_per_token` | 9.492 | none in range | none in range | none in range | none in range |
| `ai.same_model_ratio_vs_batched_dc` | none in range | none in range | none in range | none in range | none in range |
| `ai.box.idle_w` | none in range | none in range | none in range | none in range | none in range |
| `ai.vanish_share` | none in range | none in range | none in range | 30.423 | none in range |
| `ai.jobs.small_share` | 7.528 | 23.331 | 7.985 | none in range | none in range |
| `ai.alloc.local_share` | 5.431 | none in range | none in range | none in range | none in range |
| `mesh additions (kWh per person-year)` | 15.404 | 16.409 | 15.487 | none in range | 19.753 |


# TEST 2, THE ADOPTION PATH (substitution 0 to 1; prior 0.005 to 0.16)

## 1. What replaced what

`FEDERATION_SCALING_MODEL.md` §3.6 compared a gross device-fleet draw with a whole data-centre
estate and printed "4.8× less". That comparison is now labelled an untested gross estimate of
unlike quantities. The typed constant `planetary_5b_power_mw = 3,082 MW PROJECTED` had nothing
computing it and is removed.

Eric's framing: "it is delta from status quo not gross total"; "people use phones regardless".
Per household-hour (per participant-hour for a call):

    delta = A + B - C
    A  added on existing devices   wake-ups and radio tails, radio bytes and signature work on
                                   phones people already run; the marginal draw of a node on a
                                   gateway or computer that is on anyway
    B  added devices               full draw of devices added or kept on FOR the mesh (a new
                                   box, a modem, a village node, a share of a hosted seat or relay)
    C  avoided upstream (net)      data-centre energy of the workload that moves, plus the
                                   network energy of the status quo's bytes, minus the network
                                   energy of the mesh's own bytes

Positive is a cost and negative a saving. The sign is not known in advance. Every result is a
point with its bounds, and the break-even table names the inputs that flip it.

## 2. Inputs (`ENERGY_SPECS`), seamed to CIRISHolon

The default table is CIRISHolon **POWER-0** (`conformance/scene/tables/power0/power_table.json`
v3, read at c6677a1). Our own rows fill only where POWER-0 has no row. Each row names the
POWER-0 row it takes and carries that row's basis. Every POWER-0 verdict is "unresolved"
(priors from documents). A row whose source says MEASURED (Arcep, the US network-equipment
report, one 2012 handset) is still MODEL here, because MEASURED in this repo means measured on
the production mesh.

The household rows come from **DEVICES-0** (US 2020, RECS): 0.813 phones per person, against
the twin's 1.0, which is refused. Its household mix is 72.8% fixed broadband, 14.6% cellular
only and 7.1% no internet. The envelope is the United States, 2020. Term C rows come from **UPSTREAM-0** (read at 1beb5f5; each row tagged average allocation, marginal
or capacity, and that tag carried in its "taken from" column). Presence rows come from
**PRESENCE-0** (US time diaries 2022-2024, ages 15+, read at 274a5df).

Rows whose interval is a single point are **levers**: what the edge does today, fixed. The
break-even table reports the value at which each lever would flip the sign.

| row | device | state | unit | lo / default / hi | class | on anyway | year | needs measurement on device | taken from (basis) | reasoning |
|---|---|---|---|---|---|---|---|---|---|---|
| `phone.deep_idle_w` | phone | deep idle (screen off, radio idle) | W | 0.008 / 0.015 / 0.03 | MODEL | yes | 2026 | - | ours | order of a modern smartphone's suspended draw (tens of mW). Listed for completeness: the mesh does not change deep idle, so no term charges it |
| `phone.wake_j.wifi` | phone | one small exchange on Wi-Fi, wake-up and tail included | J | 0.02 / 0.04 / 0.2 | MODEL | no | 2012 | phone on a power monitor: J per edge round on Wi-Fi (screen off, Doze off and on), net of the idle baseline | POWER-0 row 8 (Huang et al. 2012, MEASURED on one handset; tail 0.24 s at 119 mW; verdict unresolved) | one 2012 handset; a current phone's Wi-Fi power save is of the same order |
| `phone.cell.promotion_j` | phone | one exchange on a carrier: promotion and transfer, tail excluded | J | 0.2 / 0.46 / 1.5 | MODEL | no | 2012 | phone on a power monitor: energy of an edge round on LTE and 5G NSA up to the end of transfer | POWER-0 row 6 (Huang et al. 2012: 12.76 J per small packet incl. an 11.6 s tail at 1.06 W, so 12.76 - 11.6 x 1.06 = 0.46 J; MEASURED on one handset, verdict unresolved) | the whole-exchange figure less its tail |
| `phone.cell.tail_s` | phone | radio held in its high-power tail after the last packet | s | 2.0 / 11.6 / 12.0 | MODEL | no | 2012 | the carrier's inactivity timer and C-DRX as seen on a current phone's power trace | POWER-0 row 6 (11.6 s, 2012) and its note (two 5G phones 2021: still about 10 s); the low end is fast dormancy | a regime, not a constant: the carrier sets it |
| `phone.cell.tail_w` | phone | radio power in the tail | W | 0.5 / 1.06 / 1.5 | MODEL | no | 2012 | - | POWER-0 row 6 (1,060 mW on one 2012 LTE handset) | C-DRX in the tail lowers it on newer modems |
| `phone.j_per_mb.wifi` | phone | active radio, Wi-Fi | J/MB | 0.02 / 0.08 / 0.3 | MODEL | no | 2026 | - | ours | Wi-Fi transfer energy on a phone of the order of 0.1 J/MB at moderate rates |
| `phone.j_per_mb.cellular` | phone | active radio, cellular | J/MB | 0.2 / 0.8 / 3.0 | MODEL | no | 2026 | - | ours | cellular transfer energy several times Wi-Fi's per byte |
| `phone.j_per_sig_op` | phone | one hybrid sign or verify on the CPU | J | 0.0002 / 0.0008 / 0.003 | FRONTIER | no | 2026 | phone on a power monitor: J per hybrid verify and per sign at the edge's batch sizes | ours | hybrid verify 276 us MEASURED on an EPYC core (cewp-cli metrics); a phone core several times slower at ~1.5 W |
| `phone.j_per_chunk_sig_op` | phone | one Ed25519 sign or verify of a media chunk | J | 0.00002 / 0.0001 / 0.0005 | FRONTIER | no | 2026 | phone on a power monitor during a call, against the media engine's per-chunk sign and verify counts | ours | media chunks are Ed25519 (media.rs NodeEnergy): tens of us on a phone core at ~1.5 W |
| `lever.phone_schedule_share` | lever | share of the edge round and Pull schedule a phone runs itself | share | 1.0 / 1.0 / 1.0 | MODEL | n/a | 2026 | - | ours | seat_hooks: the edge at 8e46165 has no host-kind branch, so a phone runs the whole schedule; a phone that left its rounds to its home node would run less |
| `lever.cell_round_share` | lever | share of schedule wake-ups on a carrier network that touch the radio | share | 1.0 / 1.0 / 1.0 | MODEL | n/a | 2026 | - | ours | the twin assumes per-round: edge's round runs over every live link on any network (micro.rs changes only the budget class and relay offering with the network; capacity.rs has no metered or battery path). 0.1 is a 5-minute batch, 0 push-woken or suspended on cellular |
| `lever.round_cadence_s` | lever | seconds between Revocation rounds | s | 30.0 / 30.0 / 30.0 | MODEL | n/a | 2026 | - | ours | capacity.rs CadenceS = 30 s (DERIVED, edge #881); Pull cycles ride on top |
| `phone.rounds_while_unsuspended` | phone | share of the phone's unsuspended hours in which it runs edge rounds | share | 0.7 / 1.0 / 1.0 | MODEL | n/a | 2026 | - | PRESENCE-0 (US 2022-2024, 15+): a person is awake 0.623 of the week, against the twin's 0.85 unsuspended (refused) | 1.0 is edge today: a phone node keeps its rounds while the OS lets it run, whether or not its person is asleep (the twin's suspended share, micro mobile_offline_share 0.15, is the only pause); 0.733 = 0.623 / 0.85 is rounds only while the person is awake |
| `home.gateway_w` | home gateway | ready, Wi-Fi on (already on: not charged) | W | 3.3 / 13.77 / 25.0 | MODEL | yes | 2025 | - | POWER-0 rows 10 (13.77 W, US new integrated gateways 2025, MEASURED sales-weighted) and 0 (9.1 W, Arcep 36 boxes, range 3.3 to 25) | a 9 to 14 W baseline the mesh neither adds nor removes; listed so the scale of a home node's marginal draw is visible |
| `home_node.on_router.marginal_w` | home node | running on the household's existing gateway (marginal) | W | 0.1 / 0.4 / 1.5 | FRONTIER | yes | 2026 | gateway at the wall with and without the edge daemon at the twin's schedule | ours | the gateway is on anyway; the daemon adds CPU wake-ups on a SoC idling near 10 W; POWER-0 has no row (not_available) |
| `home_node.on_computer.marginal_w` | home node | running on a computer already on (marginal) | W | 0.2 / 0.8 / 3.0 | FRONTIER | yes | 2026 | desktop or NAS already on, with and without the edge daemon, at the wall | ours | a daemon keeps a desktop out of its deepest idle states |
| `home_node.j_per_mb` | home node | bytes through the gateway, relayed bytes included | J/MB | 0.05 / 0.16 / 0.5 | FRONTIER | yes | 2024 | home node at the wall forwarding a known byte stream for others (relay), J per MB in plus out | POWER-0 row 0 note (traffic adds 0.1 W at 5 Mbit/s: 0.16 J/MB; 0.3 to 0.8 W at 50 Mbit/s: 0.05 to 0.13 J/MB) | relayed bytes count twice (in and out) |
| `home_node.j_per_sig_op` | home node | one hybrid sign or verify | J | 0.0001 / 0.0004 / 0.002 | MODEL | yes | 2026 | - | ours | the EPYC verify (276 us MEASURED) on a slower core at a few W |
| `home_node.j_per_chunk_sig_op` | home node | one Ed25519 sign or verify of a media chunk | J | 0.00001 / 0.00005 / 0.0003 | MODEL | yes | 2026 | - | ours | Ed25519 verify of the order of 50 us on a desktop or SBC core at a few W |
| `home_node.added_box.w` | home node | a new always-on box bought or kept on for the mesh (full draw) | W | 3.3 / 6.0 / 13.77 | MODEL | no | 2025 | - | POWER-0 rows 20 (simple Ethernet router target 3.3 W), 11 (routers and APs 10.15 W), 10 (gateways 13.77 W) | a box of the gateway's own order: more than the fixed network spends on that home |
| `home_node.cell_modem.w` | home node | a cellular modem in an added box (cellular-only home) | W | 1.0 / 2.0 / 4.0 | FRONTIER | no | 2026 | the box at the wall with its modem attached and idle | ours | an LTE modem idling attached, mains-powered |
| `laptop.marginal_w` | laptop | running the edge while awake (marginal) | W | 0.1 / 0.5 / 1.5 | FRONTIER | yes | 2026 | laptop on battery with and without the daemon | POWER-0 row 29 gives 22 W in use, duty-cycled (not the marginal) | listed for CIRISHolon's table; no scenario puts the edge on a laptop |
| `hosted.seat.w` | hosted server | a serve seat, idle included, with PUE | W | 60.0 / 120.0 / 300.0 | MODEL | no | 2026 | - | POWER-0 rows 41-42 for the overhead (PUE 1.09 hyperscale, 1.54 survey average); the server draw is ours | a one-socket rented server at the wall |
| `hosted.devices_per_seat` | hosted server | devices one seat serves | devices | 1000.0 / 2000.0 / 10000.0 | MODEL | n/a | 2026 | - | ours | the page draws one seat per ~2,000 nodes (docs/globe.js buildWorld) |
| `hosted.j_per_store_read` | hosted server | one counted store read | J | 0.001 / 0.01 / 0.05 | FRONTIER | no | 2026 | seat at the wall against its store-read counter over a window | ours | a few ms of one core at ~5 to 10 W per core |
| `hosted.j_per_mb` | hosted server | relayed bytes on a hosted relay | J/MB | 0.01 / 0.05 / 0.3 | FRONTIER | no | 2026 | hosted relay at the wall against bytes forwarded | ours | server NIC, CPU and seal work per byte with PUE |
| `network.fixed.traffic_j_per_mb` | network | fixed network, the traffic-dependent part | J/MB | 0.08 / 0.24 / 0.5 | MODEL | no | 2020 | - | POWER-0 row 35 (0.03 W per Mbit/s = 0.24 J/MB; power model from operator data); the short-run saving on a fixed line is this alone | fixed networks are a per-line baseload plus this small term (POWER-0 warning) |
| `network.fixed.core_baseload_w` | network | fixed core baseload per line (scales with capacity in the long run) | W | 0.5 / 1.5 / 2.3 | MODEL | yes | 2024 | - | POWER-0 rows 34 (1.5 W core per line, 2020 model, high for fibre) and 4 (Arcep: 2.3 W per line all in, under 10 kWh a year on fibre) | the access line itself stays whether or not the mesh runs; only core capacity is avoidable, and only in the long run |
| `network.fixed.line_mb_per_hour` | network | a fixed line's own traffic, to share its core capacity | MB/h | 200.0 / 700.0 / 2000.0 | FRONTIER | n/a | 2026 | - | ours | of the order of 500 GB a month for a household line |
| `network.mobile.traffic_j_per_mb` | network | mobile network, the traffic-dependent part | J/MB | 5.0 / 12.24 / 25.0 | MODEL | no | 2020 | - | POWER-0 row 37 (1.53 W per Mbit/s, 4G, core and access summed = 12.24 J/MB) | cellular is where traffic costs energy |
| `network.mobile.baseload_w` | network | mobile baseload per subscriber (scales with capacity in the long run) | W | 0.6 / 1.2 / 4.6 | MODEL | yes | 2024 | - | POWER-0 rows 36 (1.2 W per subscriber, 4G model) and 5 (Arcep 40 kWh per SIM a year = 4.6 W all in) | - |
| `network.mobile.sim_mb_per_hour` | network | a SIM's own traffic, to share its capacity | MB/h | 10.0 / 25.0 / 60.0 | FRONTIER | n/a | 2026 | - | ours | of the order of 15 to 20 GB a month |
| `network.thin_uplink.j_per_mb` | network | a village's one thin uplink (satellite or long microwave) | J/MB | 5.0 / 50.0 / 500.0 | FRONTIER | n/a | 2026 | - | ours | a satellite terminal's 50 to 100 W over a few Mbit/s of use; capacity-bound, so the same in both runs |
| `c1.delivery_kwh_per_person_year` | data centre | C1 delivery: message relay, fan-out and serving per daily person | kWh | 0.02 / 0.2 / 1.0 | FRONTIER | n/a | 2023 | - | UPSTREAM-0 rows 26 and 8 (a serving-only messenger: servers 2.9 M USD a year for its whole user base; messages queued only until delivered): a small share of the 5.5 kWh ceiling | kept under its old identifier; now kWh per person-year |
| `c.substitution_share` | lever | share of use that MOVES from an ad-funded platform to the mesh (not kept alongside it) | share | 0.05 / 0.05 / 0.05 | FRONTIER | n/a | 2021 | - | UPSTREAM-0 rows 39-44 (survey 2021: 26% wanted to leave a messenger, about a quarter of them managed, 0.5% uninstalled; median five messaging apps; 79% overlap): PRIOR 0.005 to 0.16 | 0 = the mesh is purely additive, the honest floor; the axis runs 0 to 1, the prior is 0.005 to 0.16; it multiplies every status-quo line of C |
| `c.ceiling_kwh_per_person_year` | data centre | everything one large social operator spends per daily active person (the ceiling of the avoided term) | kWh | 5.5 / 5.5 / 5.5 | MODEL | n/a | 2024 | - | UPSTREAM-0 row 0 (AVG, operator-reported 2024: 5.5 kWh per daily active person, 0.63 W; 3.3, 3.9, 3.6, 5.5 for 2021-2024, includes generative AI) | no component sum may exceed it: C1 + C2 + C3 + C4 are clipped to it and reported as a fraction of it |
| `c.platform_use_hours_per_person_day` | workload | hours a day a person spends in the ad-funded apps the mesh would replace | h | 1.0 / 2.5 / 5.0 | FRONTIER | n/a | 2026 | - | ours | of the order of two to three hours a day of social and messaging use |
| `c2.retained_gb_per_user` | data centre | logical bytes a platform keeps per user (retention AT REST) | GB | 1.0 / 3.0 / 10.0 | FRONTIER | n/a | 2025 | - | UPSTREAM-0 row 7 (AUTHOR'S ESTIMATE 1 to 10 GB; floor 0.6 GB from 'exabytes' over 3.5 bn) | - |
| `c2.kwh_per_logical_tb_year` | data centre | storage energy per logical TB-year, replication and cooling included | kWh | 11.5 / 15.4 / 28.2 | MODEL | n/a | 2026 | - | UPSTREAM-0 row 10 (AVG/CAP: 5.09 kWh per raw TB-year x PUE 1.08-1.54 x replication 2.1-3.6) | so 1-10 GB per user is 0.015-0.15 kWh a year (row 14): under 3% of the ceiling. Retention at rest is SMALL |
| `c2.mesh_retained_gb_per_user` | home node | content the mesh keeps per member on members' own devices (bounded retention) | GB | 0.5 / 5.0 / 30.0 | FRONTIER | yes | 2026 | - | ours | the custody model holds members' content on their own devices with bounded retention; durability.rs gives bytes held per holder but no time axis |
| `c2.device_storage_w_per_tb` | home node | marginal storage power on a device already on | W/TB | 0.0 / 0.2 / 1.0 | FRONTIER | yes | 2026 | - | ours | flash or a disk that spins anyway: near zero marginal |
| `c3.harvest_rank_ads_compute_kwh_per_person_year` | data centre | harvesting, ranking and advertising COMPUTE (logging, profiling, feature stores, training, inference, ad ranking) per daily person | kWh | 0.5 / 3.0 / 5.5 | FRONTIER | n/a | 2024 | - | UPSTREAM-0 row 25 (AVG: 0.5 to 5.5 kWh per person-year; NOBODY PUBLISHES THE SPLIT; 3.0 is the middle of the SCOUT'S OWN central reading 2 to 4); rows 18-24 for shares | this is where the platform's energy is; long-run capacity only (row 13: -28% storage IO moved power by -0.04%) |
| `c4.impressions_per_use_hour` | data centre | ad impressions per user-hour of use | impressions | 40.0 / 60.0 / 80.0 | FRONTIER | n/a | 2026 | - | UPSTREAM-0 row 32 (AUTHOR'S ESTIMATE from a measured ad load of 13% of feed items, row 29) | - |
| `c4.adtech_j_per_impression` | data centre | the third-party ad-tech chain per impression: exchanges, bidders, verification | J | 0.5 / 5.0 / 50.0 | FRONTIER | n/a | 2026 | - | UPSTREAM-0 rows 27, 31: no measured energy per impression; vendor models differ eightfold; a bracket | C4 is the chain beyond the platform's own ranking (which is in C3) |
| `c5.ad_and_tracker_mb_per_use_hour` | network | ad creatives and tracker bytes per user-hour of use (network side) | MB | 5.0 / 20.0 / 100.0 | FRONTIER | n/a | 2026 | - | UPSTREAM-0 row 28 (measured: 21.1% of web page bytes are tracking) | priced on access by the network rows |
| `c5.ad_device_kwh_per_phone_year` | phone | the device side of ads and tracking: rendering, ad and tracker bytes, telemetry wake-ups | kWh | 0.0 / 0.25 / 0.3 | MODEL | no | 2015 | phone on a power monitor with the platform app with and without ads | UPSTREAM-0 row 38 (MARG: at most 0.2-0.3 kWh per phone-year; apps with ads used 16% more energy in a 2015 study, row 33; OS telemetry about every 4.5 min, row 37) | a true marginal and small: it does not offset term A's radio cost |
| `datacentre.call.wh_per_participant_hour` | data centre | SFU work per participant-hour of a video call | Wh | 0.1 / 1.0 / 5.0 | FRONTIER | n/a | 2026 | - | ours | an SFU forwards without transcoding; the high end includes recording and transcoding |
| `village.node.w` | village node | one added node with its radio and solar controller (full draw) | W | 5.0 / 10.0 / 20.0 | MODEL | no | 2026 | - | ours | a small server plus a Wi-Fi or mesh radio |
| `village.households` | workload | households behind one village node and uplink | households | 20.0 / 50.0 / 200.0 | MODEL | n/a | 2026 | - | ours | a village of a few hundred people |
| `household.persons` | workload | persons | persons | 2.0 / 2.5 / 3.0 | MODEL | n/a | 2026 | - | ours | micro.rs household rule round(2.5 + 1.1 N(0,1)) clamped to 1..6 |
| `household.phones_per_person` | workload | smartphones per person | phones | 0.8 / 0.813 / 1.0 | MODEL | n/a | 2020 | - | DEVICES-0 (US 2020, RECS): 0.813 per person (the twin's 1.0 REFUSED); 0.999 per adult | envelope: United States, 2020 |
| `phone.wifi_share` | workload | share of a phone's awake hours on Wi-Fi (home with a fixed line) | share | 0.4 / 0.65 / 0.9 | MODEL | n/a | 2026 | - | ours | micro.rs phone_wifi_share 0.65 (globe slider default); CIRISHolon attach.phone.network (168 bins) replaces it |
| `phone.wifi_share.cell_only_home` | workload | share of awake hours on Wi-Fi in a home with no fixed line | share | 0.0 / 0.1 / 0.3 | FRONTIER | n/a | 2026 | - | ours | no home Wi-Fi: only Wi-Fi met away from home |
| `workload.bytes_per_act` | workload | wire bytes of one act's content copy | bytes | 1000.0 / 5000.0 / 50000.0 | MODEL | n/a | 2026 | - | ours | a signed text act with a thumbnail; media-heavy acts run higher |
| `phone.mesh_extra_content_wakes` | workload | extra content wake-ups under the mesh, as a share of the status quo's push wake-ups | share | -0.3 / 0.0 / 0.3 | FRONTIER | n/a | 2026 | phone wake log under the mesh and under a push-notification app for one scripted day | ours | mesh content arrives on a live link, push on the OS channel; whether batching differs is unknown |
| `phone.keepalive_bytes_per_mapping_hour` | workload | bytes to hold one NAT mapping for an hour | bytes | 0.0 / 2000.0 / 20000.0 | FRONTIER | n/a | 2026 | packet capture of a phone holding punched paths | ours | keepalives ride the 30 s round wake, so they add bytes, not wake-ups |
| `mix.fixed_broadband_share` | workload | households with fixed broadband (a gateway) | share | 0.728 / 0.728 / 0.728 | MODEL | n/a | 2020 | - | DEVICES-0 (US 2020, RECS) via the lead: 72.8% | envelope: United States, 2020, a pandemic year |
| `mix.cellular_only_share` | workload | households with cellular data only | share | 0.146 / 0.146 / 0.146 | MODEL | n/a | 2020 | - | DEVICES-0 (US 2020): 14.6% | for them an always-on node has no fixed line to sit behind |
| `mix.nothing_to_host_share` | workload | households with no computer and no fixed line | share | 0.087 / 0.087 / 0.087 | MODEL | n/a | 2020 | - | DEVICEGEN-1 (CIRISHolon 33dc99f): CERTIFIED for the US 2020 (8.8% generated, 8.7% held out; clustering 2.92 against 2.85) | nothing to host a home node on |
| `mix.no_computer_share` | workload | households with no laptop, desktop or tablet | share | 0.111 / 0.111 / 0.111 | MODEL | n/a | 2020 | - | DEVICES-0 / DEVICEGEN-1 (US 2020) | - |
| `mix.desktop_share` | workload | households with a desktop (DECLARED able to host; on all day: unknown) | share | 0.408 / 0.408 / 0.408 | MODEL | n/a | 2020 | - | DEVICES-0 (US 2020); the desktop is independent of the line among homes with a computer in this stand-in (MODEL) until the generator call replaces it | - |
| `mix.no_internet_share` | workload | households with no internet | share | 0.071 / 0.071 / 0.071 | MODEL | n/a | 2020 | - | DEVICES-0 (US 2020): 7.1% | they take no part: delta 0; the undeclared 5.5% is left out and the mix renormalised over 94.5% |

Rows ours to measure on real hardware (**needs measurement on device**):

* The phone radio: J per edge round on Wi-Fi and on LTE/5G NSA up to the end of transfer, and
  the tail seconds the carrier actually holds, on a current phone, screen off, Doze on and off.
  POWER-0's figures are one 2012 handset.
* `home_node.on_router.marginal_w`, `home_node.on_computer.marginal_w`: the wall draw of the
  gateway or computer with and without the daemon at the twin's schedule.
* `home_node.j_per_mb`: J per MB a home node forwards for others, in and out.
* Signature J on a phone (hybrid, and Ed25519 per media chunk). Store-read and forwarded-byte J
  on a hosted seat.
* `phone.mesh_extra_content_wakes`, `phone.keepalive_bytes_per_mapping_hour`: a scripted day's
  wake log and packet capture under the mesh and under a push-notification app.

## 3. How the network is priced (POWER-0's warning)

Network energy is mostly a per-line baseload. Pricing traffic at an average kWh per GB would
overstate any saving by a large factor, so the two average-intensity rows (POWER-0 rows 38 and
39, marked "NOT a marginal cost") are not used. Two regimes are reported side by side:

* **Short run** (capacity fixed): only the traffic-dependent term moves. Fixed lines cost
  0.03 W per Mbit/s, which is 0.24 J/MB (row 35). Mobile costs 1.53 W per Mbit/s, which is
  12.24 J/MB (row 37).
* **Long run** (capacity not built): add the line's share of core baseload, 1.5 W ÷ the line's
  own 700 MB/h, about 7.7 J/MB (rows 34 and 4). For mobile, add the SIM's share of its
  1.2 W baseload ÷ 25 MB/h, about 173 J/MB (rows 36 and 5). The access line stays whether or
  not the mesh runs, so it is never avoided. A line's and a SIM's own traffic are FRONTIER.

netgraph's AS hops and long-haul kilometres are read and reported but not priced. The per-line
and per-Mbit/s figures already contain the core.

## 4. The phone radio term (POWER-0 finding 3)

A carrier exchange costs a promotion (0.46 J) plus a tail (11.6 s at 1.06 W). Together these
are POWER-0's 12.76 J per small LTE packet, and the test checks that sum. The tail is cut at
the gap to the next wake-up. A Wi-Fi exchange costs 0.04 J. The edge's schedule is 126.75
wake-ups per device-hour (120 Revocation rounds at the 30 s cadence plus 6.75 Pull cycles,
seat_hooks). The edge has no host-kind or metered branch. A phone node runs every round while
the OS lets it run, whether or not its person is asleep. The twin's only pause is the suspended
share (micro `mobile_offline_share` 0.15). **The twin assumes per-round**: every round on a
carrier touches the radio.

Wh per phone per day from the edge schedule alone:

| round share | cadence s | tail 2 s, Wi-Fi 0.1 | tail 2 s, Wi-Fi 0.65 | tail 2 s, Wi-Fi 0.9 | tail 5 s, Wi-Fi 0.1 | tail 5 s, Wi-Fi 0.65 | tail 5 s, Wi-Fi 0.9 | tail 11.6 s, Wi-Fi 0.1 | tail 11.6 s, Wi-Fi 0.65 | tail 11.6 s, Wi-Fi 0.9 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 30 | 1.67 | 0.67 | 0.21 | 3.73 | 1.47 | 0.44 | 8.25 | 3.23 | 0.94 |
| 1 | 120 | 0.48 | 0.19 | 0.06 | 1.08 | 0.43 | 0.13 | 2.39 | 0.94 | 0.27 |
| 1 | 300 | 0.25 | 0.10 | 0.03 | 0.55 | 0.22 | 0.07 | 1.22 | 0.48 | 0.14 |
| 0.1 | 30 | 0.17 | 0.08 | 0.04 | 0.38 | 0.16 | 0.07 | 0.83 | 0.34 | 0.12 |
| 0.1 | 120 | 0.05 | 0.02 | 0.01 | 0.11 | 0.05 | 0.02 | 0.24 | 0.10 | 0.03 |
| 0.1 | 300 | 0.03 | 0.01 | 0.01 | 0.06 | 0.02 | 0.01 | 0.12 | 0.05 | 0.02 |
| 0 | 30 | 0.00 | 0.02 | 0.03 | 0.00 | 0.02 | 0.03 | 0.00 | 0.02 | 0.03 |
| 0 | 120 | 0.00 | 0.01 | 0.01 | 0.00 | 0.01 | 0.01 | 0.00 | 0.01 | 0.01 |
| 0 | 300 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |

POWER-0's own arithmetic is reproduced by `the_phone_term_matches_powers_own_arithmetic`. All
day on LTE at 30 s it is 10.8 Wh/day, most of a battery. On Wi-Fi it is 0.03 Wh/day.

### Presence side by side (PRESENCE-0)

PRESENCE-0 refuses the twin's awake share: 0.623 (15.0 h a day), against 0.85. The pass keeps
the **person awake** and the **device running rounds** as two inputs. Edge today keeps rounds
while the device is unsuspended (`phone.rounds_while_unsuspended` = 1). The alternative is
rounds only while the person is awake (0.623 / 0.85 = 0.733). The Wi-Fi share is derived hour
by hour from where people are: at home, always on Wi-Fi; at a workplace or school, 0.8;
elsewhere, 0.2 (both FRONTIER, since PRESENCE-0 has no networks). Over device-hours that gives
0.852. Over awake hours it gives 0.783, inside PRESENCE-0's bracket [0.61, 0.79].

| scenario | stand-in (unsuspended 0.85, Wi-Fi 0.65) | PRESENCE-0, edge as is (rounds while unsuspended) | PRESENCE-0, rounds only while awake |
|---|---|---|---|
| (inputs) | Wi-Fi share 0.650, rounds while unsuspended 1.000, phone Wh/day 3.23 | Wi-Fi share 0.852, rounds while unsuspended 1.000, phone Wh/day 1.38 | Wi-Fi share 0.783, rounds while unsuspended 0.733, phone Wh/day 1.48 |
| home node on existing gateway | 3093 [370, 15718] cost (interval > 0) | 2518 [370, 15718] cost (interval > 0) | 2551 [370, 15718] cost (interval > 0) |
| home node on computer already on | 4533 [730, 21118] cost (interval > 0) | 3958 [730, 21118] cost (interval > 0) | 3991 [730, 21118] cost (interval > 0) |
| home node on a new always-on box | 23253 [11890, 59890] cost (interval > 0) | 22678 [11890, 59890] cost (interval > 0) | 22711 [11890, 59890] cost (interval > 0) |
| phones only (home with a fixed line) | 1436 [-12.0, 9223] open (point > 0) | 861 [-12.0, 9223] open (point > 0) | 893 [-12.0, 9223] open (point > 0) |
| phones only (cellular-only home) | 3008 [36.4, 13057] cost (interval > 0) | 3008 [36.4, 13057] cost (interval > 0) | 2338 [36.4, 13057] cost (interval > 0) |
| new box on cellular (cellular-only home) | 30495 [15457, 74934] cost (interval > 0) | 29930 [15457, 74934] cost (interval > 0) | 29959 [15457, 74934] cost (interval > 0) |
| village, one thin uplink | 1237 [-1149, 16926] open (point > 0) | 1237 [-1149, 16926] open (point > 0) | 1235 [-1149, 16926] open (point > 0) |
| US 2020 household mix, node on gateway | 2848 [294, 14126] cost (interval > 0) | 2405 [294, 14126] cost (interval > 0) | 2326 [294, 14126] cost (interval > 0) |
| US 2020 household mix, new box | 22625 [11548, 57714] cost (interval > 0) | 22095 [11548, 57714] cost (interval > 0) | 22124 [11548, 57714] cost (interval > 0) |
| US 2020 household mix, node on a computer already on | 2630 [253, 13189] cost (interval > 0) | 2187 [253, 13189] cost (interval > 0) | 2108 [253, 13189] cost (interval > 0) |
| US 2020 household mix, no home node | 1571 [-0.821, 9122] open (point > 0) | 1128 [-0.821, 9122] open (point > 0) | 1049 [-0.821, 9122] open (point > 0) |

The bounds are the same in all three columns. They span every row's full interval, which
contains all three settings. The points move.

## 4b. Term C as named components, sourced from CIRISHolon UPSTREAM-0

Eric's correction ("most of the data center energy we are saving is ... the storing and
harvesting for personal information and ads") made term C a sum of named components. CIRISHolon
**UPSTREAM-0** (`conformance/scene/tables/upstream0/upstream_table.json`, 45 rows, each tagged
average allocation, marginal or capacity, read at 1beb5f5, verdict unresolved) now supplies them
through the seam. Each row carries its tag and basis. Where the table showed the structure was
wrong, the structure changed:

* **Ceiling.** One large social operator reports 5.5 kWh per daily active person for 2024
  (0.63 W, generative AI included; an average allocation). C1-C4 are clipped to it on a line of
  their own and reported as a fraction of it.
* **C1 delivery**: serving and fan-out, 0.02 / 0.2 / 1.0 kWh per person-year (FRONTIER; a
  serving-only messenger's costs put it at a small share of the ceiling).
* **C2 retention at rest is small**: 1-10 GB per user × 15.4 kWh per logical TB-year
  (replication and cooling included) gives 0.015-0.15 kWh a year, under 3% of the ceiling. The
  earlier break-even of "11.6 GB retained per user-year" pointed at the wrong component.
* **C3 harvesting, ranking and advertising compute** is where the energy is: 0.5 to 5.5 kWh per
  person-year. Nobody publishes the split. The central 3.0 is the middle of the scout's own 2-4
  reading.
* **C4 the third-party ad-tech chain**: 40-80 impressions per use-hour (the author's estimate
  from a measured 13% ad load) × 0.5 / 5 / 50 J per impression. There is no measured energy per
  impression, and vendor models differ eightfold.
* **C5 the device and network side** is a true marginal and small: at most 0.2-0.3 kWh per
  phone-year, plus ad and tracker bytes on access (21.1% of web page bytes are tracking).
* **C2-C4 are long-run capacity only.** They are removed entirely from the short run: a 28% fall
  in storage traffic moved storage power by 0.04%.
* **Substitution** (`c.substitution_share`) multiplies every status-quo line. The axis runs from
  0 to 1, but the **prior is 0.005 to 0.16**: in a 2021 survey 26% wanted to leave a messenger,
  about a quarter of them managed, and 0.5% uninstalled; the median user has five messaging
  apps. The default is 0.05.

### Headline: the sign with substitution inside its prior (kWh per person-year; calls in J per participant-hour)

| scenario | run | delta at 0.01 | delta at 0.05 | delta at 0.16 | sign inside the prior | substitution zero | cadence zero (s) | carrier round share zero |
|---|---|---|---|---|---|---|---|---|
| home node on existing gateway | short run | 3.03 cost | 3.01 cost | 2.97 cost | COST across the prior and every input interval | none | none | none |
| home node on existing gateway | long run | 3.36 cost | 3.19 open | 2.75 open | positive at the point across the prior; open over the inputs | 0.844 | none | none |
| home node on computer already on | short run | 4.43 cost | 4.42 cost | 4.37 cost | COST across the prior and every input interval | none | none | none |
| home node on computer already on | long run | 4.76 cost | 4.60 open | 4.15 open | positive at the point across the prior; open over the inputs | 1.193 | none | none |
| home node on a new always-on box | short run | 22.67 cost | 22.65 cost | 22.60 cost | COST across the prior and every input interval | none | none | none |
| home node on a new always-on box | long run | 22.99 cost | 22.83 cost | 22.39 cost | COST across the prior and every input interval | none | none | none |
| phones only (home with a fixed line) | short run | 1.42 cost | 1.40 open | 1.35 open | positive at the point across the prior; open over the inputs | 3.266 | none | none |
| phones only (home with a fixed line) | long run | 1.73 cost | 1.57 open | 1.13 open | positive at the point across the prior; open over the inputs | 0.440 | none | none |
| phones only (cellular-only home) | short run | 2.95 cost | 2.93 cost | 2.88 open | positive at the point across the prior; open over the inputs | none | none | none |
| phones only (cellular-only home) | long run | 3.85 cost | 3.66 open | 3.14 open | positive at the point across the prior; open over the inputs | 0.826 | none | none |
| new box on cellular (cellular-only home) | short run | 29.72 cost | 29.70 cost | 29.65 cost | COST across the prior and every input interval | none | none | none |
| new box on cellular (cellular-only home) | long run | 30.63 cost | 30.44 cost | 29.90 cost | COST across the prior and every input interval | none | none | none |
| call relayed by a neighbour vs SFU | short run | 15438 J cost | 14904 J cost | 13436 J cost | COST across the prior and every input interval | 1.167 | none | none |
| call relayed by a neighbour vs SFU | long run | 237124 J cost | 230701 J cost | 213038 J cost | COST across the prior and every input interval | 1.487 | none | none |
| call relayed in a data centre vs SFU | short run | 13722 J cost | 13188 J cost | 11720 J cost | COST across the prior and every input interval | 1.038 | none | none |
| call relayed in a data centre vs SFU | long run | 159477 J cost | 153054 J cost | 135391 J cost | COST across the prior and every input interval | 1.003 | none | none |
| village, one thin uplink | short run | 1.24 cost | 1.21 open | 1.12 open | positive at the point across the prior; open over the inputs | 1.640 | none | none |
| village, one thin uplink | long run | 1.20 cost | 1.05 open | 0.62 open | positive at the point across the prior; open over the inputs | 0.320 | none | none |
| US 2020 household mix, node on gateway | short run | 2.79 cost | 2.77 cost | 2.73 cost | COST across the prior and every input interval | none | none | none |
| US 2020 household mix, node on gateway | long run | 3.18 cost | 3.03 open | 2.60 open | positive at the point across the prior; open over the inputs | 0.841 | none | none |
| US 2020 household mix, new box | short run | 22.05 cost | 22.04 cost | 21.99 cost | COST across the prior and every input interval | none | none | none |
| US 2020 household mix, new box | long run | 22.44 cost | 22.29 cost | 21.87 cost | COST across the prior and every input interval | none | none | none |
| US 2020 household mix, node on a computer already on | short run | 2.58 cost | 2.56 cost | 2.52 cost | COST across the prior and every input interval | none | none | none |
| US 2020 household mix, node on a computer already on | long run | 2.96 cost | 2.81 open | 2.39 open | positive at the point across the prior; open over the inputs | 0.784 | none | none |
| US 2020 household mix, no home node | short run | 1.55 cost | 1.53 open | 1.48 open | positive at the point across the prior; open over the inputs | 3.788 | none | none |
| US 2020 household mix, no home node | long run | 1.93 cost | 1.77 open | 1.35 open | positive at the point across the prior; open over the inputs | 0.514 | none | none |

The two levers together, long run, kWh per person-year, with the sign over every input interval
in brackets. "Fixed idle" means carrier rounds off and a 300 s cadence:

| scenario | today, 0.05 | today, 0.16 | fixed idle, 0.05 | fixed idle, 0.16 | fixed idle, 1.0 |
|---|---|---|---|---|---|
| home node on existing gateway | 3.19 (open) | 2.75 (open) | 2.24 (open) | 1.79 (open) | -1.58 (open) |
| phones only (home with a fixed line) | 1.57 (open) | 1.13 (open) | 0.61 (open) | 0.17 (open) | -3.21 (open) |
| phones only (cellular-only home) | 3.66 (open) | 3.14 (open) | 1.21 (open) | 0.69 (open) | -3.27 (open) |
| village, one thin uplink | 1.05 (open) | 0.62 (open) | 1.04 (open) | 0.61 (open) | -2.64 (open) |
| US 2020 household mix, node on gateway | 3.03 (open) | 2.60 (open) | 1.91 (open) | 1.49 (open) | -1.73 (open) |
| US 2020 household mix, node on a computer already on | 2.81 (open) | 2.39 (open) | 1.69 (open) | 1.27 (open) | -1.94 (open) |
| US 2020 household mix, no home node | 1.77 (open) | 1.35 (open) | 0.66 (open) | 0.24 (open) | -2.98 (open) |
| US 2020 household mix, new box | 22.29 (cost) | 21.87 (cost) | 21.40 (cost) | 20.98 (cost) | 17.75 (open) |

**Holon's arithmetic, checked against the model, per scenario.** Inside the substitution prior
the delta is positive at the point in every scenario, in both runs.

Long run at 0.05, against the 5.5 kWh ceiling (kWh per person-year):

| scenario | delta |
|---|---|
| node on gateway | +3.2 |
| phones only on a fixed line | +1.6 |
| cellular-only phones | +3.7 |
| village | +1.1 |
| US mix, gateway | +3.0 |
| US mix, new box | +22.3 |

* **COST over every input interval:** the new box, the box on cellular, both calls, and every
  short-run household case with a home node.
* **Elsewhere:** the sign is open over the inputs, with a positive point.

**A correction to "positive unless substitution is high or the idle cadence is fixed":**
* **Fixing the idle alone does not flip the sign inside the prior.** Carrier rounds off with a
  300 s cadence cuts the household delta to between a half and a quarter (phones only, fixed
  line: +1.6 → +0.6 at 0.05, and +0.17 at 0.16), but the point stays positive.
* **Substitution alone flips the point only far outside its prior.** Long-run zeros: phones only
  0.44, village 0.32, US mix with no node 0.51, US mix on a computer 0.78, US mix on the gateway
  0.84. The short run needs more than 1 almost everywhere.
* **Both together reach negative only at high substitution:** fixed idle with substitution 1.0
  gives −1.6 to −3.3 kWh per person-year. A new box stays a cost even then.
* So: positive unless substitution is high, and the idle fix matters only together with it.

**C5 does not offset term A's radio cost.** Per phone-year:

| | edge (mesh, term A) | ads and tracking (status quo, C5) |
|---|---|---|
| energy per phone-year | 1.18 kWh (stand-in presence); 0.51 kWh (PRESENCE-0, edge as is) | at most 0.30 kWh, and only times substitution |
| wake-ups an hour | 126.75 | 13.3 (OS telemetry, about every 4.5 minutes) |

### C by component (expected regime, substitution 0.05, point J; last column: C1-C4 per person as a share of the 5.5 kWh ceiling)

| scenario | run | C1 delivery | C2 retention at rest | C3 harvest/rank/ads compute | C4 ad-tech chain | C5 ad/tracker device+network | mesh upstream | C total | A + B | C1-C4 / ceiling |
|---|---|---|---|---|---|---|---|---|---|---|
| home node on existing gateway | short run | 10.7 | 0.000 | 0.000 | 0.000 | 11.6 | -25.1 | -2.8 | 3091 | 4% |
| home node on existing gateway | long run | 17.5 | 2.4 | 154 | 3.9 | 28.6 | -395 | -189 | 3091 | 60% |
| home node on computer already on | short run | 10.7 | 0.000 | 0.000 | 0.000 | 11.6 | -25.1 | -2.8 | 4531 | 4% |
| home node on computer already on | long run | 17.5 | 2.4 | 154 | 3.9 | 28.6 | -395 | -189 | 4531 | 60% |
| home node on a new always-on box | short run | 10.7 | 0.000 | 0.000 | 0.000 | 11.6 | -25.1 | -2.8 | 23251 | 4% |
| home node on a new always-on box | long run | 17.5 | 2.4 | 154 | 3.9 | 28.6 | -395 | -189 | 23251 | 60% |
| phones only (home with a fixed line) | short run | 10.7 | 0.000 | 0.000 | 0.000 | 11.6 | -24.4 | -2.1 | 1434 | 4% |
| phones only (home with a fixed line) | long run | 17.5 | 2.4 | 154 | 3.9 | 28.6 | -385 | -178 | 1434 | 60% |
| phones only (cellular-only home) | short run | 11.4 | 0.000 | 0.000 | 0.000 | 13.3 | -68.1 | -43.4 | 2965 | 4% |
| phones only (cellular-only home) | long run | 27.7 | 2.4 | 154 | 3.9 | 54.0 | -1032 | -790 | 2965 | 60% |
| new box on cellular (cellular-only home) | short run | 11.5 | 0.000 | 0.000 | 0.000 | 13.6 | -69.2 | -44.0 | 30451 | 4% |
| new box on cellular (cellular-only home) | long run | 29.5 | 2.4 | 154 | 3.9 | 58.6 | -1046 | -798 | 30451 | 60% |
| call relayed by a neighbour vs SFU | short run | 667 | 0.000 | 0.000 | 0.000 | 0.000 | -14774 | -14107 | 797 | - |
| call relayed by a neighbour vs SFU | long run | 8029 | 0.000 | 0.000 | 0.000 | 0.000 | -237932 | -229904 | 797 | - |
| call relayed in a data centre vs SFU | short run | 667 | 0.000 | 0.000 | 0.000 | 0.000 | -9747 | -9080 | 4108 | - |
| call relayed in a data centre vs SFU | long run | 8029 | 0.000 | 0.000 | 0.000 | 0.000 | -156974 | -148945 | 4108 | - |
| village, one thin uplink | short run | 15.5 | 0.000 | 0.000 | 0.000 | 23.5 | -97.4 | -58.5 | 1179 | 4% |
| village, one thin uplink | long run | 15.5 | 2.4 | 154 | 3.9 | 23.5 | -97.4 | 102 | 1179 | 60% |
| US 2020 household mix, node on gateway | short run | 10.0 | 0.000 | 0.000 | 0.000 | 11.0 | -29.9 | -8.8 | 2839 | 3% |
| US 2020 household mix, node on gateway | long run | 17.8 | 2.2 | 142 | 3.6 | 30.4 | -464 | -268 | 2839 | 56% |
| US 2020 household mix, new box | short run | 10.0 | 0.000 | 0.000 | 0.000 | 11.0 | -30.0 | -9.0 | 22616 | 3% |
| US 2020 household mix, new box | long run | 18.1 | 2.2 | 142 | 3.6 | 31.1 | -466 | -269 | 22616 | 56% |
| US 2020 household mix, node on a computer already on | short run | 10.0 | 0.000 | 0.000 | 0.000 | 11.0 | -29.6 | -8.6 | 2621 | 3% |
| US 2020 household mix, node on a computer already on | long run | 17.8 | 2.2 | 142 | 3.6 | 30.4 | -459 | -263 | 2621 | 56% |
| US 2020 household mix, no home node | short run | 10.0 | 0.000 | 0.000 | 0.000 | 11.0 | -29.3 | -8.3 | 1562 | 3% |
| US 2020 household mix, no home node | long run | 17.8 | 2.2 | 142 | 3.6 | 30.4 | -456 | -259 | 1562 | 56% |

### The substitution share side by side (delta point [bounds] sign)

| scenario | run | 0 | 0.01 | 0.05 | 0.16 | 0.25 | 0.5 | 1 |
|---|---|---|---|---|---|---|---|---|
| home node on existing gateway | short run | 3116 [429, 15738] cost | 3111 [418, 15734] cost | 3093 [370, 15718] cost | 3044 [154, 15672] cost | 3004 [-69.9, 15635] open | 2893 [-691, 15532] open | 2669 [-1971, 15325] open |
| home node on existing gateway | long run | 3486 [434, 83458] cost | 3445 [384, 83189] cost | 3280 [-1221, 82114] open | 2825 [-9105, 79156] open | 2454 [-15555, 76737] open | 1421 [-33472, 70015] open | -643 [-69307, 56571] open |
| home node on computer already on | short run | 4556 [789, 21138] cost | 4551 [778, 21134] cost | 4533 [730, 21118] cost | 4484 [514, 21072] cost | 4444 [290, 21035] cost | 4333 [-331, 20932] open | 4109 [-1611, 20725] open |
| home node on computer already on | long run | 4926 [794, 88858] cost | 4885 [744, 88589] cost | 4720 [-861, 87514] open | 4265 [-8745, 84556] open | 3894 [-15195, 82137] open | 2861 [-33112, 75415] open | 797 [-68947, 61971] open |
| home node on a new always-on box | short run | 23276 [11949, 59910] cost | 23271 [11938, 59906] cost | 23253 [11890, 59890] cost | 23204 [11674, 59844] cost | 23164 [11450, 59807] cost | 23053 [10829, 59704] cost | 22829 [9549, 59497] cost |
| home node on a new always-on box | long run | 23646 [11954, 127630] cost | 23605 [11904, 127361] cost | 23440 [10299, 126286] cost | 22985 [2415, 123328] cost | 22614 [-4035, 120909] open | 21581 [-21952, 114187] open | 19517 [-57787, 100743] open |
| phones only (home with a fixed line) | short run | 1458 [46.7, 9244] cost | 1454 [36.3, 9240] cost | 1436 [-12.0, 9223] open | 1387 [-230, 9178] open | 1346 [-454, 9140] open | 1235 [-1075, 9037] open | 1012 [-2355, 8830] open |
| phones only (home with a fixed line) | long run | 1818 [51.1, 76810] cost | 1777 [1.9, 76541] cost | 1612 [-1759, 75466] open | 1158 [-9643, 72508] open | 786 [-16093, 70088] open | -246 [-34010, 63367] open | -2311 [-69844, 49923] open |
| phones only (cellular-only home) | short run | 3033 [109, 13090] cost | 3028 [97.0, 13084] cost | 3008 [36.4, 13057] cost | 2954 [-252, 12984] open | 2910 [-531, 12924] open | 2786 [-1306, 12758] open | 2539 [-2880, 12426] open |
| phones only (cellular-only home) | long run | 3997 [137, 133460] cost | 3949 [77.6, 133024] cost | 3755 [-2495, 131277] open | 3223 [-14922, 126475] open | 2787 [-25090, 122546] open | 1577 [-53333, 111631] open | -842 [-109820, 89802] open |
| new box on cellular (cellular-only home) | short run | 30520 [15554, 74967] cost | 30515 [15541, 74961] cost | 30495 [15457, 74934] cost | 30439 [15116, 74861] cost | 30394 [14836, 74801] cost | 30268 [14043, 74635] cost | 30017 [12455, 74303] cost |
| new box on cellular (cellular-only home) | long run | 31497 [15595, 185984] cost | 31447 [15531, 185547] cost | 31248 [12691, 183801] cost | 30702 [264, 178998] cost | 30255 [-9903, 175069] open | 29013 [-38147, 164154] open | 26529 [-94633, 142325] open |
| call relayed by a neighbour vs SFU | short run | 15572 [6155, 33832] cost | 15438 [5936, 33629] cost | 14904 [5058, 32818] cost | 13436 [2645, 30586] cost | 12235 [670, 28760] cost | 8898 [-4816, 23687] open | 2224 [-15786, 13542] open |
| call relayed by a neighbour vs SFU | long run | 238730 [50263, 2062796] cost | 237124 [49753, 2049207] cost | 230701 [47711, 1994852] cost | 213038 [42096, 1845375] cost | 198586 [37503, 1723075] cost | 158443 [24742, 1383355] cost | 78156 [-778, 703914] open |
| call relayed in a data centre vs SFU | short run | 13856 [4393, 40550] cost | 13722 [4350, 40171] cost | 13188 [4178, 38653] cost | 11720 [3704, 34481] cost | 10519 [3317, 31067] cost | 7182 [2242, 21585] cost | 508 [91.1, 2620] cost |
| call relayed in a data centre vs SFU | long run | 161082 [33493, 1379142] cost | 159477 [33158, 1365377] cost | 153054 [31822, 1310316] cost | 135391 [28148, 1158898] cost | 120939 [25142, 1035011] cost | 80795 [16792, 690881] cost | 508 [91.1, 2620] cost |
| village, one thin uplink | short run | 1276 [129, 17566] cost | 1269 [117, 17438] cost | 1237 [-1149, 16926] open | 1152 [-4782, 15516] open | 1082 [-7755, 14363] open | 887 [-16012, 11159] open | 498 [-34863, 7231] open |
| village, one thin uplink | long run | 1276 [129, 17566] cost | 1236 [74.3, 17432] cost | 1077 [-1427, 16894] open | 639 [-5669, 15415] open | 280 [-9141, 14204] open | -716 [-18784, 10843] open | -2708 [-40407, 6599] open |
| US 2020 household mix, node on gateway | short run | 2869 [347, 14147] cost | 2865 [337, 14143] cost | 2848 [294, 14126] cost | 2802 [79.6, 14079] cost | 2764 [-136, 14041] open | 2659 [-734, 13936] open | 2449 [-1963, 13726] open |
| US 2020 household mix, node on gateway | long run | 3303 [355, 84913] cost | 3264 [308, 84638] cost | 3107 [-1327, 83540] open | 2674 [-9320, 80520] open | 2321 [-15860, 78049] open | 1339 [-34026, 71184] open | -626 [-70359, 57455] open |
| US 2020 household mix, new box | short run | 22646 [11608, 57735] cost | 22642 [11598, 57731] cost | 22625 [11548, 57714] cost | 22579 [11340, 57668] cost | 22541 [11125, 57630] cost | 22435 [10526, 57525] cost | 22225 [9295, 57314] cost |
| US 2020 household mix, new box | long run | 23082 [11618, 127057] cost | 23043 [11570, 126782] cost | 22885 [9943, 125684] cost | 22451 [1950, 122663] cost | 22095 [-4590, 120192] open | 21108 [-22756, 113328] open | 19134 [-59089, 99599] open |
| US 2020 household mix, node on a computer already on | short run | 2651 [306, 13210] cost | 2647 [297, 13206] cost | 2630 [253, 13189] cost | 2584 [38.1, 13143] cost | 2546 [-177, 13105] open | 2441 [-776, 13000] open | 2231 [-2005, 12789] open |
| US 2020 household mix, node on a computer already on | long run | 3081 [314, 83911] cost | 3042 [267, 83636] cost | 2884 [-1434, 82538] open | 2452 [-9427, 79517] open | 2099 [-15967, 77046] open | 1116 [-34133, 70182] open | -848 [-70466, 56453] open |
| US 2020 household mix, no home node | short run | 1592 [52.8, 9144] cost | 1588 [43.0, 9139] cost | 1571 [-0.821, 9122] open | 1525 [-216, 9076] open | 1487 [-432, 9038] open | 1382 [-1030, 8933] open | 1172 [-2259, 8722] open |
| US 2020 household mix, no home node | long run | 2018 [60.6, 79791] cost | 1979 [13.5, 79517] cost | 1822 [-1741, 78419] open | 1390 [-9734, 75398] open | 1036 [-16274, 72927] open | 53.9 [-34440, 66062] open | -1910 [-70773, 52333] open |

## 4c. Where the home node runs (US 2020 household mix)

The home node's host is an explicit scenario input: the gateway, a computer already on, a new
box, or none. It applies only to households that can host it; the rest run phones only, and
homes with no internet are at zero. Household types come from a stated joint stand-in:
DEVICES-0 marginals, CIRISHolon DEVICEGEN-1's certified 8.7% of households with nothing to host
on, and a desktop spread evenly over homes with a computer (MODEL). It sits behind the seam
`holon_scene::world::devices::generate` will fill: the single holon-scene pin is CIRISHolon f07a5ee (twin/globe 6a71c08; it carries DEVICEGEN-1, devices.rs byte-identical to 33dc99f, and PRESENCEGEN-2). The rev was agreed
with twin-globe2, who is wiring the same call into the kernel. DEVICEGEN-1 declares a gateway
present with a fixed line and a desktop able to host. Whether the gateway can host and whether
the desktop is on all day are both unknown, so the shares reached are upper bounds. Envelope:
United States, 2020. By 2024 set-top boxes are +76% and streaming devices −22%.

| host | households it reaches | delta short run | delta long run |
|---|---|---|---|
| US 2020 household mix, node on gateway | 72.8% | 2848 [294, 14126] cost (interval > 0) | 3107 [-1327, 83540] open (point > 0) |
| US 2020 household mix, node on a computer already on | 32.3% | 2630 [253, 13189] cost (interval > 0) | 2884 [-1434, 82538] open (point > 0) |
| US 2020 household mix, new box | 87.4% | 22625 [11548, 57714] cost (interval > 0) | 22885 [9943, 125684] cost (interval > 0) |
| US 2020 household mix, no home node | 0.0% | 1571 [-0.821, 9122] open (point > 0) | 1822 [-1741, 78419] open (point > 0) |

**Someone home to host (PRESENCEGEN-2).** No energy term in TEST 1 or TEST 2 needs a person to be
home. A home node runs on a gateway or box that is on regardless, and the phone terms use
per-phone awake and suspended shares. So no host-availability figure is stated as a number here.
If a future term depends on "someone is home", it takes PRESENCEGEN-2's bracket: independent
draws leave nobody home in 0.24-0.27 of weekday hours 10-15, with a ceiling of about 0.5, for the
US 2022-24 only.

## 5. Counters read (DERIVED from the packages' events)

| counter | source | value (seed 1) |
|---|---|---|
| wake-ups per device-hour: rounds + Pull cycles | `seat_hooks::device_counters(Phone, unclaimed)` | 126.75 = 120 + 6.75 |
| sync bytes per device-hour each way; seat reads | same | 76,049; 86.4 |
| acts signed / copies received per person-hour; content wake-ups per phone-hour | `people_hooks::energy`, Poisson, 2,000-person stand-in, 1 day | 7.51 / 158.9; 93.2 |
| relayed share of pairs: fixed / phone Wi-Fi / phone cellular; NAT mappings held per phone | `reach::hooks::energy_counters` | 0.254 / 0.213 / 0.517; 0.77 |
| ASes, IXPs, long-haul km (read, not priced) | `netgraph` stand-in | same-metro 3.00, 0.35, 0; peer 3.94, 0.17, 9,781; to DC 2.83, 0.72, 1,711 |
| call MB per participant-hour: own / forwarded / rx (fixed; Wi-Fi; cellular); chunk sig ops | `media::energy_by_class(scenario_call4)` | 1,680 / 1,892 / 2,137; 1,274 / 788 / 2,266; 1,362 / 0 / 2,257; ~5.5 M to 5.8 M |

**Missing counters.** None of these is invented. Each is either estimated or left unpriced, as
stated.
* **Bytes forwarded for others per device, outside a call.** The delta estimates them as
  relayed share × the household's copies × relay offering share, labelled as an inference in
  its ledger line. Owed: a per-device relayed-bytes counter in `reach/hooks.rs` or `micro.rs`,
  split by link class.
* **Storage writes and byte-seconds held.** `durability.rs` has bytes held without time, so
  storage is not priced.
* **The seat schedule's own signature operations.** Not counted. Only the act signs and
  verifies are priced.
* **Sync bytes as a function of the cadence.** The lever changes wake-ups only. The byte
  counter is per hour at 30 s.

## 6. Scenarios

| scenario | per | A | B | C |
|---|---|---|---|---|
| home node on existing gateway | household-hour | phones; gateway marginal W; node sync and relayed bytes | seat share | DC messaging; status-quo content bytes on access less the mesh's (both ends) and its sync |
| home node on computer already on | household-hour | as above with the computer's marginal W | seat share | as above |
| home node on a new always-on box | household-hour | phones; node bytes | box full W + seat share | as above |
| phones only (home with a fixed line) | household-hour | phones | seat share + relayed bytes on a hosted relay | as above |
| phones only (cellular-only home) | household-hour | phones, Wi-Fi 0.1 (FRONTIER) | as above | as above, on mobile access |
| new box on cellular (cellular-only home) | household-hour | phones on the box's Wi-Fi at home | box + modem W + seat share | as above, every byte on mobile access |
| call relayed by a neighbour vs SFU | participant-hour | mesh call bytes less the SFU's, chunk signatures | none | SFU Wh; SFU bytes less the mesh's |
| call relayed in a DC vs SFU | participant-hour | one upload + received less the SFU's, chunk signatures | the relay, priced as an SFU | the SFU it replaces |
| village, one thin uplink | household-hour | phones on village Wi-Fi | seat share + village node share | DC messaging; status-quo content over the uplink less the mesh's non-local share and sync |
| US 2020 household mix, node on gateway | household-hour | 72.8% gateway homes + 14.6% cellular-only phones-only + 7.1% no internet at 0, renormalised over 94.5% | | |
| US 2020 household mix, new box | household-hour | 72.8% new box + 14.6% box on cellular | | |

Regimes (FSD §4.4) set the suspended share, the local share of copies (it matters for the
village) and the relay offering share.

## 7. Results (seed 1, expected regime)

Each bound is the lowest or highest value found over the rows' interval ends. The search
starts at seat_calibration's corners and refines them by a vertex search, so each bound is
attained by a real combination of input ends. The interval-arithmetic enclosure is guaranteed
to contain every combination but is wider. Every result is FRONTIER.

| scenario | per | regime | run | A added on existing | B added devices | C avoided upstream (net) | delta = A + B - C | enclosure | sign | class |
|---|---|---|---|---|---|---|---|---|---|---|
| home node on existing gateway | household-hour | expected | short run | 2433 [371, 10382] | 658 [56.4, 4337] | -2.8 [-998, 99.5] | 3093 [370, 15718] | [-562, 15874] | cost (interval > 0) | FRONTIER |
| home node on existing gateway | household-hour | expected | long run | 2433 [371, 10382] | 658 [56.4, 4337] | -189 [-67395, 1746] | 3280 [-1221, 82114] | [-5547, 90503] | open (point > 0) | FRONTIER |
| home node on computer already on | household-hour | expected | short run | 3873 [731, 15782] | 658 [56.4, 4337] | -2.8 [-998, 99.5] | 4533 [730, 21118] | [-202, 21274] | cost (interval > 0) | FRONTIER |
| home node on computer already on | household-hour | expected | long run | 3873 [731, 15782] | 658 [56.4, 4337] | -189 [-67395, 1746] | 4720 [-861, 87514] | [-5187, 95903] | open (point > 0) | FRONTIER |
| home node on a new always-on box | household-hour | expected | short run | 993 [11.5, 4982] | 22258 [11936, 53909] | -2.8 [-998, 99.5] | 23253 [11890, 59890] | [10958, 60046] | cost (interval > 0) | FRONTIER |
| home node on a new always-on box | household-hour | expected | long run | 993 [11.5, 4982] | 22258 [11936, 53909] | -189 [-67395, 1746] | 23440 [10299, 126286] | [5973, 134675] | cost (interval > 0) | FRONTIER |
| phones only (home with a fixed line) | household-hour | expected | short run | 993 [11.5, 4973] | 441 [34.7, 3254] | -2.1 [-996, 102] | 1436 [-12.0, 9223] | [-944, 9378] | open (point > 0) | FRONTIER |
| phones only (home with a fixed line) | household-hour | expected | long run | 993 [11.5, 4973] | 441 [34.7, 3254] | -178 [-67239, 1902] | 1612 [-1759, 75466] | [-5929, 83850] | open (point > 0) | FRONTIER |
| phones only (cellular-only home) | household-hour | expected | short run | 2524 [70.0, 8019] | 441 [34.7, 3254] | -43.4 [-1784, 113] | 3008 [36.4, 13057] | [-1444, 13201] | cost (interval > 0) | FRONTIER |
| phones only (cellular-only home) | household-hour | expected | long run | 2524 [70.0, 8019] | 441 [34.7, 3254] | -790 [-120004, 2696] | 3755 [-2495, 131277] | [-9194, 139843] | open (point > 0) | FRONTIER |
| new box on cellular (cellular-only home) | household-hour | expected | short run | 993 [11.5, 4982] | 29458 [15536, 68309] | -44.0 [-1643, 116] | 30495 [15457, 74934] | [14521, 75135] | cost (interval > 0) | FRONTIER |
| new box on cellular (cellular-only home) | household-hour | expected | long run | 993 [11.5, 4982] | 29458 [15536, 68309] | -798 [-110509, 2879] | 31248 [12691, 183801] | [6837, 194840] | cost (interval > 0) | FRONTIER |
| call relayed by a neighbour vs SFU | participant-hour | expected | short run | 797 [181, 3623] | 0.000 [0.000, 0.000] | -14107 [-29195, -4877] | 14904 [5058, 32818] | [1404, 36472] | cost (interval > 0) | FRONTIER |
| call relayed by a neighbour vs SFU | participant-hour | expected | long run | 797 [181, 3623] | 0.000 [0.000, 0.000] | -229904 [-1991229, -47530] | 230701 [47711, 1994852] | [-21418, 2063981] | cost (interval > 0) | FRONTIER |
| call relayed in a data centre vs SFU | participant-hour | expected | short run | 508 [91.1, 2620] | 3600 [360, 18000] | -9080 [-18916, -2845] | 13188 [4178, 38653] | [-348, 43179] | cost (interval > 0) | FRONTIER |
| call relayed in a data centre vs SFU | participant-hour | expected | long run | 508 [91.1, 2620] | 3600 [360, 18000] | -148945 [-1290578, -30489] | 153054 [31822, 1310316] | [-38178, 1380316] | cost (interval > 0) | FRONTIER |
| village, one thin uplink | household-hour | expected | short run | 18.1 [1.7, 405] | 1161 [125, 6853] | -58.5 [-9668, 1294] | 1237 [-1149, 16926] | [-2151, 17565] | open (point > 0) | FRONTIER |
| village, one thin uplink | household-hour | expected | long run | 18.1 [1.7, 405] | 1161 [125, 6853] | 102 [-9636, 1571] | 1077 [-1427, 16894] | [-2632, 17861] | open (point > 0) | FRONTIER |
| US 2020 household mix, node on gateway | household-hour | expected | short run | 2264 [297, 9237] | 575 [48.8, 3844] | -8.8 [-1045, 94.1] | 2848 [294, 14126] | [-656, 14269] | cost (interval > 0) | FRONTIER |
| US 2020 household mix, node on gateway | household-hour | expected | long run | 2264 [297, 9237] | 575 [48.8, 3844] | -268 [-70459, 1761] | 3107 [-1327, 83540] | [-5693, 91327] | open (point > 0) | FRONTIER |
| US 2020 household mix, new box | household-hour | expected | short run | 918 [10.6, 4608] | 21698 [11596, 52084] | -9.0 [-1023, 94.0] | 22625 [11548, 57714] | [10685, 57866] | cost (interval > 0) | FRONTIER |
| US 2020 household mix, new box | household-hour | expected | long run | 918 [10.6, 4608] | 21698 [11596, 52084] | -269 [-68992, 1752] | 22885 [9943, 125684] | [5658, 133852] | cost (interval > 0) | FRONTIER |
| US 2020 household mix, node on a computer already on | household-hour | expected | short run | 2140 [266, 8766] | 482 [39.5, 3380] | -8.6 [-1044, 95.1] | 2630 [253, 13189] | [-697, 13331] | cost (interval > 0) | FRONTIER |
| US 2020 household mix, node on a computer already on | household-hour | expected | long run | 2140 [266, 8766] | 482 [39.5, 3380] | -263 [-70392, 1828] | 2884 [-1434, 82538] | [-5734, 90322] | open (point > 0) | FRONTIER |
| US 2020 household mix, no home node | household-hour | expected | short run | 1155 [19.7, 5070] | 408 [32.1, 3009] | -8.3 [-1043, 95.9] | 1571 [-0.821, 9122] | [-950, 9264] | open (point > 0) | FRONTIER |
| US 2020 household mix, no home node | household-hour | expected | long run | 1155 [19.7, 5070] | 408 [32.1, 3009] | -259 [-70339, 1882] | 1822 [-1741, 78419] | [-5988, 86201] | open (point > 0) | FRONTIER |

By device class (expected regime, short run, stand-in presence, J as each enters the delta):

| scenario | phone | home node | hosted | network | data centre | village node |
|---|---|---|---|---|---|---|
| home node on existing gateway | 973 | 1449 | 658 | 23.5 | -10.3 | 0.000 |
| home node on computer already on | 973 | 2889 | 658 | 23.5 | -10.3 | 0.000 |
| home node on a new always-on box | 973 | 21609 | 658 | 23.5 | -10.3 | 0.000 |
| phones only (home with a fixed line) | 973 | 9.0 | 441 | 22.8 | -10.3 | 0.000 |
| phones only (cellular-only home) | 2505 | 9.0 | 441 | 64.1 | -10.3 | 0.000 |
| new box on cellular (cellular-only home) | 973 | 28809 | 658 | 64.7 | -10.3 | 0.000 |
| call relayed by a neighbour vs SFU | 622 | 175 | 0.000 | 14287 | -180 | 0.000 |
| call relayed in a data centre vs SFU | 444 | 64.2 | 3600 | 9260 | -180 | 0.000 |
| village, one thin uplink | -1.3 | 9.0 | 441 | 79.2 | -10.3 | 720 |
| US 2020 household mix, node on gateway | 1137 | 1118 | 575 | 28.0 | -9.5 | 0.000 |
| US 2020 household mix, new box | 900 | 21098 | 608 | 28.1 | -9.5 | 0.000 |
| US 2020 household mix, node on a computer already on | 1137 | 993 | 482 | 27.7 | -9.5 | 0.000 |
| US 2020 household mix, no home node | 1137 | 8.3 | 408 | 27.5 | -9.5 | 0.000 |

## 8. Break-even: what decides the sign

The phone-radio inputs come first. Levers are fixed at what the edge does today, and their
zero is a design threshold. Then come the inputs whose zero lies inside their own interval
(these decide the sign), then the largest movers. Expected regime, stand-in presence. The full
table, including the long run and every scenario, is printed by `cewp-cli energy`.

| scenario | run | input | component | class | interval | delta at lo / hi | zero at | inside interval |
|---|---|---|---|---|---|---|---|---|
| phones only (home with a fixed line) | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 1436 / 1436 | none in range | no |
| phones only (home with a fixed line) | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 2149 / 724 | 1.1547 | no |
| phones only (home with a fixed line) | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 656 / 1468 | none in range | no |
| phones only (home with a fixed line) | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 1436 / 1436 | none in range | no |
| phones only (home with a fixed line) | short run | `phone.cell.tail_w` | A/B | MODEL | [0.5, 1.5] W | 938 / 1827 | none in range | no |
| phones only (home with a fixed line) | short run | `phone.cell.promotion_j` | A/B | MODEL | [0.2, 1.5] J | 1416 / 1515 | none in range | no |
| phones only (home with a fixed line) | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 1436 / 1436 | none in range | no |
| phones only (home with a fixed line) | short run | `hosted.seat.w` | A/B | MODEL | [60, 300] W | 1216 / 2094 | none in range | no |
| phones only (home with a fixed line) | short run | `hosted.devices_per_seat` | A/B (count) | MODEL | [1000, 10000] devices | 1875 / 1084 | none in range | no |
| phones only (home with a fixed line) | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 1436 / 1436 | 3.2663 | no |
| call relayed by a neighbour vs SFU | short run | `network.mobile.traffic_j_per_mb` | C1 delivery, mesh upstream (minus) | MODEL | [5, 25] J/MB | 6916 / 28983 | none in range | no |
| call relayed by a neighbour vs SFU | short run | `phone.j_per_chunk_sig_op` | A/B | FRONTIER | [0.00002, 0.0005] J | 14554 / 16656 | none in range | no |
| call relayed by a neighbour vs SFU | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 14904 / 14904 | 1.1667 | no |
| village, one thin uplink | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 1237 / 1237 | none in range | no |
| village, one thin uplink | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 1237 / 1237 | none in range | no |
| village, one thin uplink | short run | `village.households` | A/B (count) | MODEL | [20, 200] households | 2317 / 697 | none in range | no |
| village, one thin uplink | short run | `village.node.w` | A/B | MODEL | [5, 20] W | 877 / 1957 | none in range | no |
| village, one thin uplink | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 1237 / 1237 | 1.6396 | no |
| US 2020 household mix, node on gateway | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 2848 / 2848 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 3398 / 2299 | 1.9541 | no |
| US 2020 household mix, node on gateway | short run | `phone.wifi_share.cell_only_home` | A and C (count) | FRONTIER | [0, 0.3] share | 2892 / 2759 | 6.7516 | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 1937 / 2886 | none in range | no |
| US 2020 household mix, node on gateway | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 2848 / 2848 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.tail_w` | A/B | MODEL | [0.5, 1.5] W | 2267 / 3305 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.promotion_j` | A/B | MODEL | [0.2, 1.5] J | 2825 / 2941 | none in range | no |
| US 2020 household mix, node on gateway | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 2848 / 2848 | none in range | no |
| US 2020 household mix, node on gateway | short run | `home_node.on_router.marginal_w` | A/B | FRONTIER | [0.1, 1.5] W | 2016 / 5899 | none in range | no |
| US 2020 household mix, node on gateway | short run | `hosted.seat.w` | A/B | MODEL | [60, 300] W | 2562 / 3707 | none in range | no |
| US 2020 household mix, node on gateway | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 2848 / 2848 | none in range | no |
| US 2020 household mix, new box | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 22625 / 22625 | none in range | no |
| US 2020 household mix, new box | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 23284 / 21968 | 9.5888 | no |
| US 2020 household mix, new box | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 21904 / 22655 | none in range | no |
| US 2020 household mix, new box | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 22625 / 22625 | none in range | no |
| US 2020 household mix, new box | short run | `phone.cell.tail_w` | A/B | MODEL | [0.5, 1.5] W | 22165 / 22987 | none in range | no |
| US 2020 household mix, new box | short run | `phone.cell.promotion_j` | A/B | MODEL | [0.2, 1.5] J | 22607 / 22699 | none in range | no |
| US 2020 household mix, new box | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 22625 / 22625 | none in range | no |
| US 2020 household mix, new box | short run | `home_node.added_box.w` | A/B | MODEL | [3.3, 13.77] W | 13635 / 48495 | none in range | no |
| US 2020 household mix, new box | short run | `home_node.cell_modem.w` | A/B | FRONTIER | [1, 4] W | 22069 / 23737 | none in range | no |
| US 2020 household mix, new box | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 22625 / 22625 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 2630 / 2630 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 3180 / 2081 | 1.8541 | no |
| US 2020 household mix, node on a computer already on | short run | `phone.wifi_share.cell_only_home` | A and C (count) | FRONTIER | [0, 0.3] share | 2674 / 2541 | 6.2257 | no |
| US 2020 household mix, node on a computer already on | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 1719 / 2668 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 2630 / 2630 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `phone.cell.tail_w` | A/B | MODEL | [0.5, 1.5] W | 2049 / 3087 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `phone.cell.promotion_j` | A/B | MODEL | [0.2, 1.5] J | 2607 / 2723 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 2630 / 2630 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `home_node.on_computer.marginal_w` | A/B | FRONTIER | [0.2, 3] W | 1891 / 5338 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `hosted.seat.w` | A/B | MODEL | [60, 300] W | 2390 / 3350 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 2630 / 2630 | none in range | no |
| US 2020 household mix, no home node | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 1571 / 1571 | none in range | no |
| US 2020 household mix, no home node | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 2121 / 1022 | 1.3676 | no |
| US 2020 household mix, no home node | short run | `phone.wifi_share.cell_only_home` | A and C (count) | FRONTIER | [0, 0.3] share | 1615 / 1482 | 3.7114 | no |
| US 2020 household mix, no home node | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 660 / 1609 | none in range | no |
| US 2020 household mix, no home node | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 1571 / 1571 | none in range | no |
| US 2020 household mix, no home node | short run | `phone.cell.tail_w` | A/B | MODEL | [0.5, 1.5] W | 989 / 2028 | none in range | no |
| US 2020 household mix, no home node | short run | `phone.cell.promotion_j` | A/B | MODEL | [0.2, 1.5] J | 1548 / 1664 | none in range | no |
| US 2020 household mix, no home node | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 1571 / 1571 | none in range | no |
| US 2020 household mix, no home node | short run | `hosted.seat.w` | A/B | MODEL | [60, 300] W | 1368 / 2180 | none in range | no |
| US 2020 household mix, no home node | short run | `hosted.devices_per_seat` | A/B (count) | MODEL | [1000, 10000] devices | 1977 / 1246 | none in range | no |
| US 2020 household mix, no home node | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.05, 0.05] share | 1571 / 1571 | 3.7876 | no |

### Findings (the sign and what decides it; nothing here is a saving or a cost claim)

Sections 7 and 8 are at substitution 0.05; `cewp-cli energy` prints the long-run rows. Section 4b
carries the headline: inside the substitution prior the point is positive in every scenario,
and it is a cost over every input interval for any new box and for both calls.
* **Short run:** C2-C4 are zero. Where a single input decides, it is C1 delivery or the phone
  radio.
* **Long run:** C3 compute is the largest status-quo line. At substitution 0.05, even its high
  end does not outweigh the mesh's own upstream bytes on capacity for most households.
* **Levers:** the phone radio is the largest term the mesh adds. The two levers (substitution,
  and the idle cadence with radio touch on carriers) flip the point only together, and only at
  high substitution.

## 9. Hooks (flat `Vec<f32>`, class codes as `seat_hooks::class`)

* `view_delta(seed)`: `DELTA_STRIDE = 16` per (scenario, regime, run): `[scenario, regime, run
  (0 short, 1 long), per (0 household-hour, 1 participant-hour), A, A_lo, A_hi, B, B_lo, B_hi,
  C, C_lo, C_hi, net, net_lo, net_hi]`. Scenarios are in `Scenario::ALL` order (0 gateway,
  1 computer, 2 new box, 3 phones only (fixed line), 4 phones only (cellular only), 5 box on
  cellular, 6 call via a neighbour, 7 call via a DC, 8 village, 9 mix gateway, 10 mix box,
  11 mix computer, 12 mix none). Regimes are 0 optimistic, 1 expected, 2 pessimistic,
  3 adversarial. 104 rows. All at substitution 0.05. The sign is a
  cost if `net_lo > 0`, a saving if `net_hi < 0`, and open otherwise.
* `view_delta_class(seed)`: one class per `view_delta` row.
* `view_break_even(seed)`: `BREAK_EVEN_STRIDE = 9` per row: `[scenario, run, row index into
  ENERGY_SPECS, class, lo, hi, net_at_lo, net_at_hi, zero (NaN if none)]`, phone-radio rows
  first.
* `view_phone_wh(seed)`: `PHONE_STRIDE = 5` per row: `[round_share, cadence_s, tail_s,
  wifi_share, wh_per_day]`, 81 rows over `PHONE_GRID`.

## 10. Validity envelope

* No row is MEASURED in this repo's sense. The phone radio rests on one 2012 handset. The
  gateway rows are new US units of 2025, not the installed stock.
* Data-centre energy for messaging alone is unpublished, and it decides most household signs.
* The household rows and the mix are United States 2020. PRESENCE-0 is United States
  2022-2024, ages 15+. Neither carries to another country without saying so.
* Network capacity shares in the long run rest on a line's and a SIM's own traffic (FRONTIER).
* The workloads are the twin's: the Poisson act process on a 2,000-person stand-in, the edge
  release/45.0.0 schedule at 8e46165, and the media engine's 60 s four-person call. The result
  is a per-household or per-participant delta, not a planetary total.
