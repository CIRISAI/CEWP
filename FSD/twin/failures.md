# Twin package: correlated failure domains and durability

Code: `crates/cewp-sim/src/failure.rs`, `crates/cewp-sim/src/durability.rs`, example
`crates/cewp-sim/examples/failure_domains.rs`
(`cargo run --release -p cewp-sim --example failure_domains -- seeds=100 review_seeds=200`).
It answers REVIEW.md §3 items 6 (correlated infrastructure failures) and 7 (durability by
failure domain). Every output of this package is **MODEL**: the twin's result under the tagged
inputs below. Nothing here is measured on the live mesh.

## 1. What was there before

- FSD §4.4 names a "correlated regional ASN cut" in the adversarial regime's `p_offline`
  cell. No code modelled it. Churn in `micro.rs` is per node and independent
  (`mobile_offline_share`). The review's "a good start" credits a table cell, not a model.
- `cewp-adversary` carries correlated holder loss only as text thresholds ("correlated holder
  loss >= 0.70"). Nothing draws a correlated event.
- `chunk_custody.rs` counts holders. It does not look at whose they are. Its `N_EFF_MIN = 3`
  and `R9_HOLDER_FLOOR = 9` are not chunk-holder rules of CC 6.1.2.1.2. That section gates
  aggregation-tier *sources*, and "R9" names a CC 8.3.1 residual. This package uses neither
  constant as a custody rule. It only sweeps 9 as one holder count (see the `durability.rs`
  module doc).

## 2. What is modelled

**World** (`DomainWorld`, generated, MODEL). Countries, metros (Zipf weights), power-grid
regions (1..2 consecutive metros), fixed access ISPs and cellular carriers per country (Zipf
shares), international cables (some land in two countries), cloud providers and regions.
Also households, persons and devices: phone, home node, laptop, hosted server, one serve seat.
Rooms (calls) are drawn with the FSD 4.4 locality share. `NodeKind` discriminants 0..4 equal
`micro.rs` `HostKind` 0..4. Ids are indices. There are no real places, networks or people.

**Failure-domain labels** (`FailureDomainSource`, the seam). Every node has a label on each
of household, operator, access network, metro, power region, country, cloud region and cloud
provider. The topology package, and once approved, real AS/cable data, plug in behind this
trait.

**Nine correlated scenarios** (`ScenarioKind`, `sample_event`). Each has a lognormal duration
and a recovery shape:

| scenario | what fails | recovery |
|---|---|---|
| regional_power | one grid region dark. Homes and routers go off unless on a UPS. Phones on battery fall to cellular until the cell sites' batteries run out (`power.tower_backup_h`). A data centre fails its generator transfer with `power.dc_fail`. | linear over the last `ramp_share` |
| isp_outage | one fixed ISP, nationwide, picked by load | step |
| submarine_cable | `cable.cuts` cables on the route into one country. A country whose every cable is cut becomes an island. | step |
| cloud_region | one cloud region (hosted nodes, and the seat if it lives there) | linear |
| dns_bootstrap | the seat name does not resolve. Running sessions continue. A rejoining node without a cached public peer cannot join. | step |
| bgp_leak | one country's access prefixes are unreachable from the ASes that accept the leak | step |
| cellular_outage | one carrier, nationwide | step |
| national_shutdown | the country's international links go. Domestic cellular also goes with probability `shutdown.mobile_also`. | step |
| relay_withholding | colluding operators of `withhold.relay_share` of relays forward nothing and serve no chunk | each operator noticed on its own exponential clock, mean D/3 |

**Reachability** (`evaluate`, MODEL rule). A node is up if it is awake, powered, attached to
a live access network, not removed, and (if it must rejoin now) able to rejoin. Two up nodes
reach each other when four things hold: their access networks connect through country cores
and live cables; no route leak separates them; and either end has a public listener or a
non-withholding relay sits in their component (CC 5.4.3). Phones are client-only. Outputs are
reachable pairs, the largest component, rooms losing a member or half their members, and the
durability probe.

**Domain vs random harness** (`paired_run`, `compare`). Each seed draws one event. The
harness measures it, then removes the same number of baseline-up devices uniformly at random
(never the seat) on the same world, and measures that. Outcomes are reported as distributions
(p01, p05, p50, p95, p99, mean) over seeds. Route leaks and withholding remove no nodes, so
their random arm removes nothing.

**Durability** (`durability.rs`). A chunk is lost when every holder fails `serves_mainland`
(up, in the largest component, not withholding, reachable by a NAT'd requester). The
probability is exact over placement (hypergeometric, checked against brute-force enumeration)
and Monte Carlo over failure states (checked against exhaustive enumeration on a tiny world).
There are three placements: `Uniform` (the replica-count reading), `AudienceLocal(0.65)` (the
audience that pulled it holds it), and `DomainSpread` (round-robin over access networks). The
CC requires none of them and forbids none. There are two granularities: `WholeObject` and
`PerChunk` (256 KiB chunks, CIRISEdge `CHUNK_BYTES`, so 38,147 chunks for 10^10 bytes). The
effective replica count applies CC 6.1.2.1.2's `k / (1 + rho_bar (k - 1))` to holders, with
MODEL per-axis weights (`RhoWeights`). It is reported beside the per-axis Kish `n_eff`.

**The review's named question** (`review_question`): a popular 10 GB object; two of the four
largest fixed ISPs down at once; 30% of home-class nodes gone for good. It is evaluated
during the outage and after the ISPs return.

## 3. Parameters

Every value below is printed by the example from `FAILURE_SPECS`. That array is the source of
truth, and `specs_are_tagged_and_unique` asserts that no input is MEASURED. The "anchor"
figures in sources are public post-incident numbers recalled from memory. They were **not
re-read for this work**. They bound an order of magnitude and do not make a parameter
measured. Durability-side constants: `CHUNK_BYTES` = 256 KiB (CIRISEdge
`group_content/store.rs` at edge 310aa7e, a producer choice, not CC). The review's 10 GB is
read as 10^10 bytes (MODEL). `RhoWeights` = household 0.6, operator 0.3, asn 0.25, metro 0.1,
power 0.2, country 0.1, cloud region 0.4, cloud provider 0.1 (MODEL, judgement, to be
replaced by co-failure rates). Holder counts swept: 9, 30 (`DEFAULT_TARGET_HOLDERS`), 100.

| name | value | class | source |
|---|---|---|---|
| name | value | class | source |
|---|---|---|---|
| world.nodes | 10000 | MODEL | REVIEW 'concrete first milestone': a 10,000-node benchmark |
| world.countries | 12 | MODEL | enough jurisdictions for national events to be a minority of the world |
| world.metros_per_country_max | 5 | MODEL | 1..=max metros per country, Zipf population weights |
| world.metro_zipf | 1 | MODEL | metro population ~ 1/rank^s inside a country (city-size law shape, not fitted) |
| world.power_region_metros_max | 2 | MODEL | a grid region spans 1..=max consecutive metros of one country |
| world.fixed_isps_per_country | 4 | MODEL | access ISPs per country; share ~ 1/rank^s |
| world.isp_zipf | 1 | MODEL | market concentration of access ISPs (s = 1: the largest of four holds 48%) |
| world.cell_carriers_per_country | 3 | MODEL | cellular carriers per country; share ~ 1/rank^s with the ISP exponent |
| world.cables_per_country_max | 4 | MODEL | 1..=max international links (cables) per country; a country with one is an island |
| world.cable_shared_share | 0.5 | MODEL | share of cables that also land in a second country (one cut, two countries) |
| world.cloud_providers | 3 | MODEL | hosting providers, share ~ 1/rank |
| world.cloud_regions_per_provider | 4 | MODEL | regions per provider, placed in metros by population |
| world.persons_per_household_max | 4 | MODEL | 1..=max persons per household |
| world.home_node_share | 0.35 | MODEL | households with an always-on home node (NAS / mini-PC) |
| world.laptop_share | 0.6 | MODEL | persons with a laptop running CIRIS |
| world.hosted_share | 0.03 | MODEL | persons who rent a hosted server |
| world.phone_wifi_share | 0.6 | MODEL | phones on home Wi-Fi at the instant (the rest on cellular) |
| world.reach_home_share | 0.4 | FRONTIER | FSD 4.4 Expected regime; micro.rs reach_home_share; real NAT mix awaits leviculum#100 |
| world.public_listener_share | 0.3 | MODEL | micro.rs public_listener_share: of reachable home routers, the share with an inbound listener |
| world.relay_pref | 0.9 | MODEL | FSD 4.4 Expected relay offer opt-in |
| world.mobile_offline_share | 0.15 | MODEL | FSD 4.4 Expected p_offline; micro.rs mobile_offline_share |
| world.laptop_awake_share | 0.5 | MODEL | laptops with the lid open at the instant |
| world.ups_share | 0.05 | MODEL | home nodes (and their router) on a UPS that outlasts the evaluation hour |
| world.seats | 1 | MODEL | serve seats; production shape is one (micro.rs default M = 1) |
| world.rooms_per_person | 0.3 | MODEL | calls/rooms in the instant per person |
| world.room_size_max | 8 | MODEL | room sizes 2..=max, uniform |
| world.room_locality | 0.65 | MODEL | FSD 4.4 Expected locality share: a member after the first is from the first member's metro |
| eval.hour | 1 | MODEL | hours after onset at which the state is measured |
| eval.rejoin_share | 0.3 | MODEL | share of nodes that must re-establish a session during an event (wake, network change) |
| eval.peer_cache_share | 0.5 | MODEL | rejoining nodes that hold a cached public peer and need no bootstrap seat |
| power.median_h | 6 | MODEL | lognormal median; anchor: 2003 Northeast blackout, most load back within ~2 days (US-Canada Task Force final report, 2004) |
| power.sigma | 1 | MODEL | lognormal shape (heavy tail: storm outages run days) |
| power.ramp_share | 0.6 | MODEL | last share of the duration over which supply returns linearly (rolling restoration) |
| power.tower_backup_h | 4 | FRONTIER | cell-site battery autonomy during a grid outage; no figure trusted here |
| power.dc_fail | 0.02 | MODEL | a data-centre node in the region whose generator transfer fails |
| isp.median_h | 4 | MODEL | lognormal median; anchor for the tail: Rogers, Canada, 2022-07-08, about a day nationwide (CRTC inquiry) |
| isp.sigma | 1.2 | MODEL | lognormal shape |
| cable.cuts | 2 | MODEL | cables cut on the route into one country (uniform over countries); anchor: 2008 Mediterranean cuts (SEA-ME-WE 4 and FLAG, two cables). Connectivity only: capacity loss on surviving cables is not modelled |
| cable.median_h | 336 | MODEL | lognormal median (14 d); anchor: Tonga 2022 single cable, ~5 weeks to repair |
| cable.sigma | 0.7 | MODEL | lognormal shape |
| cloud.median_h | 5 | MODEL | lognormal median; anchor: AWS us-east-1, 2021-12-07, ~7 h (AWS post-event summary) |
| cloud.sigma | 0.8 | MODEL | lognormal shape |
| cloud.ramp_share | 0.5 | MODEL | share of the duration over which services return |
| dns.median_h | 3 | MODEL | lognormal median; anchors: Dyn DDoS 2016-10-21 (hours, in waves); Facebook 2021-10-04 ~6 h (Meta engineering post) |
| dns.sigma | 0.8 | MODEL | lognormal shape |
| bgp.acceptor_share | 0.3 | MODEL | share of other ASes that accept the leaked route and lose the victims |
| bgp.median_h | 1.5 | MODEL | lognormal median; anchor: Pakistan Telecom / YouTube 2008-02-24, ~2 h (RIPE NCC analysis) |
| bgp.sigma | 0.7 | MODEL | lognormal shape |
| cell.median_h | 6 | MODEL | lognormal median; anchors: AT&T 2024-02-22 ~12 h (FCC report); Optus 2023-11-08 ~12 h |
| cell.sigma | 0.8 | MODEL | lognormal shape |
| shutdown.median_h | 48 | MODEL | lognormal median; anchor: Egypt 2011-01-28, ~5 days |
| shutdown.sigma | 1 | MODEL | lognormal shape |
| shutdown.mobile_also | 1 | MODEL | probability the shutdown also cuts domestic cellular (1 = full shutdown) |
| withhold.relay_share | 0.2 | MODEL | share of relays whose operators collude to withhold (forward nothing, serve no chunk) |
| withhold.median_h | 24 | FRONTIER | time until withholding is noticed and routed around; no instrument exists |
| withhold.sigma | 1 | FRONTIER | lognormal shape |

## 4. Findings (all MODEL; world seed 2026, 10,002 nodes, 100 seeds per scenario, hour 1)

The numbers come from `failure_domains -- seeds=100 review_seeds=200` at commit d121842.
Claims marked *test* are asserted by a unit test on a different 3,000-node world, so they hold
beyond this one seed.

- **C1, a partition is not the loss of its nodes** (*test*
  `island_keeps_internal_pairs_that_random_loss_removes`). A national shutdown and the same
  count removed at random leave the same largest component. The island keeps its internal
  pairs, and random loss does not. Mean reach_pairs: 0.835 for the domain arm against 0.832
  for the random arm. Rooms losing a member: 0.188 against 0.313, because local rooms survive
  inside the island.
- **C2, domain loss costs chunks that random loss does not** (*test*
  `domain_loss_costs_chunks_that_random_loss_does_not`, at least 100x on the mean per-chunk
  loss). Mean per-chunk loss, audience-local placement, 9 holders:

  | scenario | domain arm | random arm |
  |---|---|---|
  | regional_power | 7.6e-4 | 1.4e-15 |
  | isp_outage | 2.2e-6 | 1.5e-16 |
  | national_shutdown | 2.1e-3 | 1.5e-8 |
  | submarine_cable | 7.0e-4 | 3.7e-10 |

  Removing 10% of nodes at random is not the event of losing a domain, as the review says.
  The model puts that gap at many orders of magnitude for holders that share the domain.
- **C3, multihoming hides an outage until the towers fail** (*test*
  `multihomed_phones_hide_an_outage_until_the_towers_fail`). At hour 1 of a power or ISP
  outage no room loses a member (mean 0 against 0.072 and 0.041 for random), because phones
  fall back to cellular. Once the cell-site batteries are out (`power.tower_backup_h`,
  FRONTIER), the grid outage loses more half-rooms than random loss does. The crossover hour
  is the FRONTIER parameter, not a finding.
- **C4, for node loss alone, connectivity depends only on the count.** For power, ISP, cloud,
  cellular and DNS events, reach_pairs and lcc are equal in both arms to 4 digits. One network
  component with fungible relays means pairs depend only on how many nodes are up. Domain
  structure shows up in rooms, in chunks, and in partitions (cables, shutdowns, leaks), not
  in pair counts.
- **C5, withholding is invisible to connectivity** (*test*, exact, every seed). Twenty per
  cent of relays withholding loses no node and no pair. It costs only unserved chunks: mean
  per-chunk loss 2.4e-13 against 0. A connectivity metric cannot detect it.
- **C6, BGP leak**: no node is lost, yet 3.4% of pairs are lost on average and 6.3% of rooms
  lose a member, against 0 for random.
- **C7, the review's question** (two of the four largest ISPs down, 30% of home-class nodes
  gone, a 10 GB object as 38,147 chunks, P(NOT fully recoverable), mean over 200 seeds, during
  the outage):

  | placement | 9 holders | 30 holders | 100 holders | k_eff p50 at 9 / 30 |
  |---|---|---|---|---|
  | uniform | 0.98 | 3.5e-9 | <1e-38 | 7.2 / 15.7 |
  | audience_local(0.65) | 0.62 (1.00 if the publisher is a customer of a lost ISP) | 1.7e-4 (9.1e-4) | <1e-22 | 3.4 / 4.3 |
  | domain_spread | 0.16 | 3.3e-15 | 0 | 7.9 / 19.1 |

  Whole-object holding at 9 holders: uniform 1.2e-4, audience_local 9.1e-4, domain_spread
  4.6e-6. Per-chunk placement multiplies exposure by the chunk count. At 9 holders a 10 GB
  object is more likely lost than whole under uniform and audience-local placement. At 30
  holders, audience-local placement is 10^4 to 10^5 times worse than uniform, because its
  holders share a metro and ISP. Its k_eff stays near 4 however many holders are added. The
  holder count is not the durability number. The domain mix of the holders is.

## 5. What real datasets would replace (approval requests; none fetched)

The seam is `FailureDomainSource`, plus the `sample_event` duration parameters. Each request
needs approval as a new dataset. Licence and size are to be confirmed from each project's own page
before any request is granted. They were not read for this package.

| dataset | would replace | form |
|---|---|---|
| CAIDA AS Relationships / PeeringDB (shared with the topology package) | generated access ISPs, country cores, market shares (`world.fixed_isps_per_country`, `world.isp_zipf`) | AS graph + IXP membership, coarse grain |
| TeleGeography Submarine Cable Map (public GitHub data) | generated cables and landings (`world.cables_per_country_max`, `world.cable_shared_share`) | cable to landing-country list |
| IODA (Georgia Tech) outage records | duration distributions for ISP, national and cable events (`isp.*`, `shutdown.*`, `cable.*`) | per-event start/end, aggregate only |
| Access Now #KeepItOn STOP dataset | shutdown frequency, duration, mobile-only share (`shutdown.*`) | per-event rows, aggregate only |
| US DOE OE-417 electric disturbance reports | power outage durations and restoration shape (`power.*`) | per-event rows, aggregate only |
| Cloudflare Radar outage center | cross-check of ISP/cloud/DNS durations | per-event annotations |

No dataset exists for cell-site battery autonomy, relay withholding detection time, or per-axis
co-failure weights. Those stay FRONTIER/MODEL until field telemetry (FSD Level 5) exists.

## 6. Validity envelope (what the model cannot say)

- **One instant, one event.** The state is evaluated at `eval.hour`. Recovery shapes only
  set who is back by then. There is no traffic, no repair flux (the swarm re-replicating
  during an outage is not modelled, so "during" overstates loss for long events), and no
  compound events except the hand-built review question.
- **Connectivity, not capacity.** A surviving cable carries everything. Congestion on the
  remaining paths, rate-limited seats and media quality belong to the media and topology
  packages.
- **Tails below about 1e-6 are arithmetic, not forecasts.** They assume holders fail
  independently given the drawn state. The probability of an unmodelled event (a bug, a
  coordinated deletion, a correlated disk-firmware fault) is far larger than 1e-15 and
  dominates. Quote such cells as "below the model's resolution".
- **The generated world is not the Internet.** Twelve countries, four ISPs each, Zipf shares.
  The ordering of placements (C7) and the existence of the gaps (C1 to C6) are the findings.
  The magnitudes depend on these stand-ins.
- Not modelled from REVIEW item 7: disk wear, storage exhaustion, voluntary departure as a
  rate, jurisdictional coercion beyond shutdowns, correlated deletion. Withholding is the only
  malicious holder behaviour.

## 7. Hooks (HOOKS.md)

- **Globe** (`failure.rs`). `view_nodes`: 6 u32 per node (kind, country, metro, power region,
  access network, public). `view_event_steps`: per step `[t f32 bits, up, lcc]`, then per node
  0 down / 1 mainland / 2 cut off / 3 withholding. `view_event_components`: per step, per node,
  a dense network-component label, `u32::MAX` when down. `view_demo`: a deterministic
  2,000-node power-region scenario. Ids are indices, and the topology package owns
  coordinates.
- **Energy.** `ReviewRow::bytes_held_by_kind` gives bytes held per `NodeKind` (= `micro.rs`
  `HostKind` 0..4) by a drawn holder set (DERIVED). The model has no time axis for storage,
  so byte-seconds are not counted. It has no traffic, radio, signature or path-length events,
  so those counters are not exposed.
