# Twin package `nat`: joint reachability model

Code: `crates/cewp-sim/src/reach.rs` (`cewp_sim::reach`). CLI: `cewp-cli nat [SEED]`.
Review item answered: REVIEW.md section 3 item 2, "Model actual residential NAT distributions".

## 1. What the micro tier has today (checked against the code)

`crates/cewp-sim/src/micro.rs` `PARAM_SPECS` rows 0 to 3, used in `MicroWorld::build`
(household router draw), `go_out_with` (cellular), `punch_possible` and `settle_punches`:

| parameter | default | class in code | what the code does with it |
|---|---|---|---|
| `reach_home_share` | 0.40 | FRONTIER | a household router is `Public` or `Punchable` with this share, else `Hard` |
| `public_listener_share` | 0.30 | MODEL | of those, `Public` (12 % of routers); the rest `Punchable` (28 %) |
| `cellular_punchable_share` | 0.15 | FRONTIER | a phone on cellular is punchable, drawn per network move; never public |
| `punch_success_share` | 0.90 | FRONTIER | a punch between two punchable ends works, drawn once per pair and network epoch |

The review's four numbers are right. Two things it does not say:

- Every draw is independent: each dwelling draws its own router from its own stream, each
  cellular attachment draws on its own, so two neighbours on one ISP or two phones on one carrier
  are unrelated. A `Hard` end never gets a direct path except LAN or a public listener on the
  other side.
- `reach_home_share` means three different things in the repo. In `micro.rs` it is "public or
  punchable IPv4 router". In `workload.rs` (`RegimeParameters`) its doc says "IPv6 + PCP/UPnP".
  In `examples/scale_model.rs` it is "IPv6 or punched UDP path", anchored to IPv6 capability
  statistics. The planetary kernel (`kernel-wasm` `direct_nat_pm`) uses it as a per-pair direct
  probability. This package keeps the micro meaning (an IPv4 class of the router) and models
  IPv6 as a separate, pairwise mechanism.

## 2. The model

### 2.1 Structure (correlation is structural)

1. **Access network** (`AccessNet { id, tech, region }`, the resolved form of the `topology`
   package's `NetLabel`; adapter in `reach/topo.rs`, section 9.1). `NetBehaviour` is drawn once from the network id: CGN or not, and the CGN's
   RFC 4787 mapping, filtering and hairpinning; the router the ISP ships (mapping, filtering,
   hairpinning, UPnP); IPv6 deployed or not and its firewall policy. Every home on the ISP and
   every phone on the carrier inherits it.
2. **Home** (`ReachModel::home`): its own router instead of the ISP's (`own_router_share`), a
   second NAT (`double_nat_share`), a hand-forwarded port (only on a public IPv4 address: no CGN,
   no double NAT), IPv6 on or off. NAT layers compose: the more restrictive mapping and filtering
   win; hairpinning is the outer layer's.
3. **Phone**: on home Wi-Fi it is behind the home's router (client-only); on cellular behind the
   carrier's CGN (a carrier with no CGN gives a public address behind a stateful firewall).
4. **Region** (V4Rich / V4Scarce) moves the fixed-line CGN share; **technology** moves CGN share
   (cable lower, fibre higher) and IPv6 deployment.

The micro class of an endpoint is derived, not drawn: `Public` = forwarded port on a public IPv4
address and not client-only; `Punchable` = the composed NAT stack maps endpoint-independently;
`Hard` = neither.

### 2.2 The pair rule (`direct_path`)

Inputs: both endpoints, whether they share a LAN, and whether they may punch (a facilitator
answers, both on edge v44+, and they are room peers or consented contacts). Output: mechanism
and probability. Order and derivation:

| mechanism | when | p | source |
|---|---|---|---|
| LAN | same household network | 1 | micro `unpunched_direct` (AutoInterface) |
| public listener | either end has a forwarded IPv4 port and is not client-only | 1 | edge `NodeMode::binds_listener` (`src/edge.rs`); one hop: `link_is_forwarded` (`src/transport/reticulum.rs:534`) |
| port mapped | counterfactual only (`port_mapping_client` = 1) | 1 | no UPnP/PCP/NAT-PMP client in edge `src/transport` at 8e46165 or leviculum `src` at v0.34.0+ciris.3 (git grep: none) |
| IPv6 punch | both ends global IPv6 behind open or stateful firewalls | `punch_success_share` | leviculum `bind_session_socket` (dual-stack), `host_candidates`, `same_nat_alternates` (global v6 always a candidate) |
| hole punch | both composed NATs map endpoint-independently | `punch_success_share` | leviculum `direct_link::punch`: punches the reflexive address learned from one facilitator over the same socket; edge `DirectLinksConfig::leviculum_settings` (`ProbeProtocol::Rnsp`) |
| peer-reflexive punch | one end EIM with EIF or ADF filtering, the other not EIM | `punch_success_share` | leviculum `punch`: a punch heard from an unadvertised address is acked and punched (`PEER_REFLEXIVE_MAX`); two-sided check: an address must be acked and heard |
| relay | anything else | 0 | |

Further derived rules:
- **Two non-EIM ends never punch.** Neither learns an address the other sends from, and
  leviculum does no port prediction (it punches advertised and peer-reflexive addresses only).
- **Two ends behind one CGN** need the CGN to hairpin for an IPv4 punch.
- **No facilitator, or not room peers: no punch.** Edge `DirectLinksConfig::proposes`
  (proposing needs a facilitator) and `accepts_direct` (`DirectAccept::RoomPeers`).
- **IPv6 listener** is off: edge's documented default `listen_addr` is `0.0.0.0:4242`
  (`src/ffi/pyo3.rs:7002`), IPv4. `ipv6_listener` = 1 is the counterfactual.
- **"One-sided punch" is not a mechanism.** In edge's own history it names a failure (one side
  moved the link, leviculum#89, the 2026-10-31 incident). Since the two-sided commit it ends as a
  failed attempt, inside `punch_success_share`.

### 2.3 Calibration (DERIVED)

`ReachModel::new` solves three shares so the class marginals at the defaults equal the micro
defaults exactly (tested to 1e-9 by exact enumeration, and by sampling):

- `port_forward_share` = public share / P(no CGN, no double NAT) (closed form);
- `router_eim_share` by bisection on P(not public, composed stack EIM) = 0.28;
- `cell_cgn_eim_share` = (0.15 - (1 - `cell_cgnat_share`)) / `cell_cgnat_share`.

`punch_success_share` keeps its meaning as a per-attempt success, applied to every punch the NAT
pair allows. Two punchable ends on different networks with IPv6 off punch at exactly 0.90 (tested).
A target the structure cannot reach (for example a cellular punchable share below the share of
carriers without CGN) is refused, not clamped.

## 3. Parameters

| parameter | default | class | source / why |
|---|---|---|---|
| `reach_home_share` | 0.40 | FRONTIER | micro P0 / FSD 4.4 Expected regime: home routers that are public or punchable over IPv4. Calibration target |
| `public_listener_share` | 0.30 | MODEL | micro P1: of reachable home routers, the share with an inbound IPv4 listener. Calibration target |
| `cellular_punchable_share` | 0.15 | FRONTIER | micro P2: cellular attachments whose NAT maps endpoint-independently. Calibration target |
| `punch_success_share` | 0.90 | FRONTIER | micro P3: per-attempt success of a punch the NAT pair allows (loss, timing, the 10 s PUNCH_WINDOW, the two-sided commit of leviculum#89); no fleet number |
| `fixed_cable_share` | 0.30 | MODEL | home-weighted share of fixed access on cable; kind of source: national / OECD broadband subscription statistics by technology (varies widely by country) |
| `fixed_fibre_share` | 0.40 | MODEL | home-weighted share on fibre (DSL takes the rest); same kind of source |
| `cell_5g_share` | 0.40 | MODEL | share of cellular attachments on 5G (rest 4G); kind of source: operator and industry subscription reports |
| `region_v4_scarce_share` | 0.50 | MODEL | home-weighted share in regions whose ISPs got little IPv4 before registry exhaustion; reasoning, no dataset |
| `fixed_cgnat_rich` | 0.10 | MODEL | fixed-line CGN share where IPv4 is plentiful; kind of source: academic CGN measurement studies (P2P DHT and Netalyzr-style vantage points) found fixed-line CGN in a minority of networks, growing |
| `fixed_cgnat_scarce` | 0.40 | MODEL | fixed-line CGN share where IPv4 is scarce; same kind of source, which found CGN far more common in late-allocation regions |
| `cable_cgnat_mult` | 0.5 | MODEL | cable incumbents hold older, larger IPv4 allocations (reasoning, not a measurement) |
| `fibre_cgnat_mult` | 1.5 | MODEL | new fibre entrants hold little IPv4 (reasoning, not a measurement) |
| `cgn_eim_share` | 0.70 | MODEL | fixed-line CGNs with endpoint-independent mapping: RFC 6888 REQ-1 requires RFC 4787 REQ-1; CGN measurement studies found most but not all comply |
| `cgn_filter_eif` | 0.10 | MODEL | CGN filtering endpoint-independent (RFC 4787 sec. 5) |
| `cgn_filter_adf` | 0.30 | MODEL | CGN filtering address-dependent (the rest address-and-port-dependent) |
| `cgn_hairpin_share` | 0.70 | MODEL | CGNs that hairpin (RFC 6888 REQ-9 requires it); compliance incomplete, no number |
| `router_filter_eif` | 0.15 | MODEL | home-router filtering endpoint-independent; kind of source: NAT-type classification in P2P / VoIP / game-console STUN surveys, where port-restricted cone is the most common |
| `router_filter_adf` | 0.30 | MODEL | home-router filtering address-dependent (rest address-and-port-dependent); same kind of source |
| `router_hairpin_share` | 0.60 | MODEL | home routers that hairpin (leviculum session.rs: 'many home routers do not') |
| `router_upnp_share` | 0.60 | MODEL | home routers with UPnP-IGD or PCP on (moot while edge has no client) |
| `own_router_share` | 0.30 | MODEL | homes running their own router instead of the ISP's; reasoning |
| `double_nat_share` | 0.10 | MODEL | homes behind two NATs (ISP gateway in router mode plus their own); kind of source: home-network measurement (Netalyzr-era) reported a minority |
| `fixed_v6_cable` | 0.65 | MODEL | cable ISPs with IPv6 deployed; kind of source: public IPv6 adoption dashboards (search-engine and RIR measurement) put user IPv6 capability near 40-50 % globally, uneven by network |
| `fixed_v6_fibre` | 0.55 | MODEL | fibre ISPs with IPv6 deployed; same kind of source |
| `fixed_v6_dsl` | 0.30 | MODEL | DSL ISPs with IPv6 deployed; same kind of source |
| `home_v6_enabled_share` | 0.85 | MODEL | on a v6 ISP, homes whose router hands out global IPv6; reasoning |
| `fixed_v6_fw_open` | 0.05 | FRONTIER | home IPv6 with no inbound firewall; RFC 6092 recommends a stateful default; no number |
| `fixed_v6_fw_blocked` | 0.10 | FRONTIER | home IPv6 firewall that defeats a simultaneous UDP send; no number |
| `cell_cgnat_share` | 0.95 | MODEL | cellular attachments behind CGN; kind of source: CGN measurement studies found CGN in nearly every cellular network |
| `cell_filter_eif` | 0.05 | MODEL | carrier CGN filtering endpoint-independent; reasoning |
| `cell_filter_adf` | 0.15 | MODEL | carrier CGN filtering address-dependent (rest address-and-port-dependent); reasoning |
| `cell_hairpin_share` | 0.50 | FRONTIER | carrier CGNs that hairpin between two subscribers; no number |
| `cell_v6_4g` | 0.55 | MODEL | 4G carriers with IPv6 deployed; kind of source: operator IPv6 deployment statistics, where large mobile networks are among the most IPv6-heavy |
| `cell_v6_5g` | 0.75 | MODEL | 5G carriers with IPv6 deployed; same kind of source |
| `cell_v6_fw_blocked` | 0.30 | FRONTIER | carriers whose IPv6 firewall drops subscriber-to-subscriber UDP even on a simultaneous send; no number |
| `family_same_carrier_share` | 0.80 | MODEL | a household's phones on one carrier (family plans); reasoning |
| `phone_wifi_share` | 0.65 | MODEL | micro P7: phone on Wi-Fi vs cellular |
| `port_mapping_client` | 0.0 | DERIVED | 0: no UPnP / PCP / NAT-PMP client in edge 8e46165 src/transport or leviculum v0.34.0+ciris.3 src (git grep: absent; leviculum#101). 1 = counterfactual |
| `ipv6_candidates` | 1.0 | DERIVED | 1: leviculum same_nat_alternates always punches the peer's global IPv6 host candidate on the dual-stack session socket (bind_session_socket, host_candidates). 0 = IPv4 only |
| `peer_reflexive` | 1.0 | DERIVED | 1: leviculum direct_link::punch acks and punches the address a punch came from (PEER_REFLEXIVE_MAX = 4). 0 = advertised addresses only |
| `ipv6_listener` | 0.0 | DERIVED | 0: edge's documented listen_addr default is 0.0.0.0:4242 (src/ffi/pyo3.rs:7002), IPv4 only. 1 = counterfactual: a home with no IPv6 firewall is dialled over IPv6 |

Solved at the defaults (DERIVED, `ReachModel::new`): `port_forward_share` 0.1808,
`router_eim_share` 0.3764, `cell_cgn_eim_share` 0.1053.

Fixed in code, MODEL: a non-EIM mapping is address-dependent 30 % / address-and-port-dependent
70 % (the pair rule treats both alike); world shape (`WorldShape::default`: 8 streets of 60
households per metro, 6 ISPs per metro, 1 to 3 serving a street, 3 carriers, skewed market
shares); 1 to 4 phones per household; pair mix family 0.65 / community 0.30 / other 0.05
(micro `family_share`, `community_share`).

RESULTS_PLACEHOLDER

## 4. Results (`cewp-cli nat 1`; every number MODEL: computed from MODEL/FRONTIER inputs)

**Direct-path probability by access pair**, room peers, facilitator up, ends on different
networks (4,000 pairs a cell; a fixed end is a home whose node listens if it can, a cellular end
a client-only phone):

| | cable | fibre | dsl | 4g | 5g |
|---|---|---|---|---|---|
| cable | 0.58 | 0.51 | 0.49 | 0.40 | 0.46 |
| fibre | 0.52 | 0.46 | 0.44 | 0.34 | 0.37 |
| dsl | 0.51 | 0.44 | 0.45 | 0.31 | 0.33 |
| 4g | 0.40 | 0.33 | 0.32 | 0.16 | 0.22 |
| 5g | 0.44 | 0.39 | 0.34 | 0.22 | 0.30 |

Same network (one ISP or one carrier): cable 0.66, fibre 0.54, DSL 0.51, 4G 0.39, 5G 0.52.
Two phones on ONE carrier are more than twice as likely to go direct as two on different 4G
carriers (0.39 vs 0.16): they share the carrier's IPv6 deployment and firewall, which outweighs
the hairpin they need for an IPv4 punch.

Mechanism mix over the matrix cells: public listener 0.14, IPv6 punch 0.15, hole punch 0.04,
peer-reflexive punch 0.09, relay 0.58.

**Against the micro rule, by class pair** (same endpoints): public x any 1.00 (same);
punchable x punchable 0.90 (same, by calibration); punchable x hard **0.37** (micro 0); hard x
hard **0.16** (micro 0). The difference is the peer-reflexive punch (an EIM end whose filter
admits the new port) and IPv6.

**Relay share** (share of pairs needing a forwarder; family 0.65 / community 0.30 / other 0.05;
256 metros of 8 streets x 60 households; mean (standard error) [p10, p90] across metros):

| regime (FSD 4.4) | correlated | independent, same marginals | delta (se) |
|---|---|---|---|
| Optimistic (reach 0.65) | 0.288 (0.006) [0.17, 0.43] | 0.290 (0.001) [0.27, 0.31] | -0.002 (0.006) |
| Expected (0.40) | 0.437 (0.006) [0.31, 0.55] | 0.444 (0.001) [0.42, 0.46] | -0.007 (0.006) |
| Pessimistic (0.15) | 0.585 (0.003) [0.52, 0.64] | 0.591 (0.001) [0.58, 0.61] | -0.006 (0.003) |
| Adversarial (0) | 0.653 (0.001) [0.64, 0.67] | 0.652 (0.001) [0.64, 0.67] | +0.001 (0.001) |

(Adversarial is not 1: family pairs on one household LAN stay direct.)

Decomposition at Expected, correlated: the micro rule on the same worlds gives 0.573; switching
off IPv6 candidates gives 0.527; switching off IPv6 and peer-reflexive gives 0.575, i.e. the
joint model with both off reproduces the micro rule (the comparability check).

**Per street** (community pairs, every cross-household pair of a street, 256 streets):
correlated mean 0.60, p10 0.31, p50 0.60, p90 0.88; independent mean 0.65, p10 0.58, p50 0.65,
p90 0.73.

**The finding (MODEL).** At the same per-endpoint marginals, correlation barely moves the mean
relay share (Expected: -0.007, about one standard error; no regime moves it by more than 0.007),
but it widens the spread: the p10-p90 band across metros grows from 0.04 to 0.24 wide, and across
streets from 0.15 to 0.57. A street served by an EIM ISP with IPv6 relays a third of its pairs;
one on a symmetric CGN relays nearly nine in ten. The mean is the wrong summary for the R5
street and for seat or relay sizing: provisioning for the mean street under-provisions the
streets that need relays most. This depends on the MODEL world shape (1 to 3 ISPs a street,
skewed market shares) and should be re-run on the `topology` package's AS graph.

**The second finding (MODEL).** The joint model's Expected relay share (0.44) is 0.14 below the
micro rule's on the same endpoints (0.57). That gap is not correlation; it is two mechanisms the
micro rule leaves out and edge has: IPv6 host candidates (0.09 of it: 0.527 -> 0.437) and the
peer-reflexive punch (0.05: 0.575 -> 0.527). If those are right, the micro tier over-counts relayed pairs at its own defaults.

**Calibration tension (inference).** Hitting 28 % punchable homes with this structure needs only
38 % of home routers to map endpoint-independently (`router_eim_share` 0.376). NAT-type surveys
of consumer routers generally report a majority as cone (EIM) NATs; if that holds for CIRIS
homes, today's `reach_home_share` 0.40 is pessimistic. Only B1 with F1 of section 5 can settle it.

## 5. Fleet-measurement request (for the lead to put to the server session)

Goal: turn the MODEL shares above into MEASURED ones without any per-device row or identifier
leaving production. Every item is a **count summed over the fleet per day**, keyed only by the
coarse buckets named, reported only where a bucket holds at least k nodes (k chosen by the
server session; buckets under k are merged or dropped). No node id, key id, IP address, port,
ASN, carrier name, place or timestamp finer than the day. Each node computes its buckets about
itself, locally, from what it already sees.

Coarse buckets a node can compute about itself:
- `access`: `fixed` (Wi-Fi / Ethernet) or `cellular` (OS network type). Cable / fibre / DSL and
  4G / 5G are NOT knowable on the device reliably and stay MODEL unless the OS reports them.
- `v6`: has a global IPv6 address (leviculum `host_candidates` already computes this).
- `nat_public`: its reflexive IPv4 address (from the facilitator probe) equals its interface
  address (no NAT).
- `cgn_hint`: its interface or (where visible) router WAN address is in 100.64.0.0/10 (RFC 6598)
  or the reflexive address differs from a UPnP-reported external address. Only a hint.
- `mapping`: `eim` / `non_eim` / `unknown` - knowable only if the probe reaches TWO facilitator
  addresses and compares the reflexive ports (RFC 5780 style). Edge probes one facilitator today
  (`DirectLinksConfig::facilitator`), so this needs a second facilitator address on the seat:
  a feature request, not a counter (F1 below).

The request comes in two lists. List A is what edge ALREADY counts at 8e46165 (every symbol
read with `git show 8e46165:src/observability.rs`); it can be summed over the fleet today with
no edge change, only no bucket key. List B is new edge work, to file as an edge issue after the
v45 tag. Two items of B are features rather than counters and are marked **FEATURE**.

### List A: counters edge already exports at 8e46165 (fleet sums, no bucket key)

| # | what | symbol (`src/observability.rs` at 8e46165) | replaces / checks |
|---|---|---|---|
| A1 | direct-link proposals made | `EdgeMetrics::direct_link` label `DIRECT_LINK_PROPOSED` | denominator of every rate below |
| A2 | proposals that ended with a direct path | `DIRECT_LINK_ESTABLISHED` (same map) | fleet direct share of room-peer pairs: this model's output (`1 - relay share` over proposing pairs) |
| A3 | punches that never got through (symmetric NAT, firewall, no route) | `DIRECT_LINK_FAILED_PUNCH` | with A2, the fleet punch failure rate; checks `punch_success_share` x the share of non-EIM x non-EIM pairs |
| A4 | the facilitator did not answer this node's probe | `DIRECT_LINK_FAILED_PROBE` | UDP-blocked networks (not modelled, section 7) |
| A5 | other endings | `DIRECT_LINK_FAILED_TIMEOUT`, `_FAILED_REJECTED`, `_FAILED_GLARE`, `_FAILED_HELD`, `_FAILED_INTERFACE_LOST`, `_FAILED_PEER_FELL_BACK`, `DIRECT_LINK_LOST` | separates NAT failures (A3) from timing and mobility ones the model does not have |
| A6 | proposals not made for want of a facilitator | `DIRECT_LINK_OFF_NO_FACILITATOR` (same map) | `PairCtx::may_punch` false share |
| A7 | peers with a direct path now | `EdgeMetrics::direct_links_active` (gauge, read as `direct_links_active()`) | held mappings (energy hook) at the fleet level |
| A8 | links left unshaped because a forwarder carries them | `EdgeMetrics::link_shaping` label `LINK_SHAPING_FORWARDED` (`link_shaping_unshaped:forwarded`) | a lower bound on forwarded identity links (shaped tier only; not A/V legs) |
| A9 | dead punched links moved back to the relayed path | `EdgeMetrics::link_stall` label `LINK_STALL_RETIRED_DIRECT` | punched paths that do not survive (NAT mapping timeout; not modelled) |

A1 to A6 are labels of one map, `EdgeMetrics::direct_link`; A1 to A3 alone give a fleet-level
check of this model's direct share for proposing pairs. They cannot split it by access class,
NAT behaviour or mechanism: that needs List B.

### List B: new edge work

Each is a daily fleet sum, keyed by the self-computed buckets above, under the k-threshold.

| # | counter | key | replaces |
|---|---|---|---|
| B1 | nodes seen | access x v6 x nat_public x cgn_hint (x mapping once F1 exists) | `cell_cgnat_share`, `fixed_cgnat_*` (as a hint), `fixed_v6_*` / `cell_v6_*` (by access only), the class marginals (`reach_home_share`, `cellular_punchable_share`) once `mapping` exists |
| B2 | nodes with any unsolicited inbound dial in the day | access x v6 | `public_listener_share` (and so `port_forward_share`); no inbound-dial counter exists at 8e46165 |
| B3 | the List A `direct_link` labels A1 to A5, bucketed | own access x own v6 x peer v6 x label | the access-pair matrix of section 4 (own side only without F2) |
| B4 | established direct links by winning candidate | own access x candidate (`v4_advertised`, `v4_peer_reflexive`, `v6`, `same_nat_host`) | the mechanism mix: IPv6 punch, hole punch and peer-reflexive punch apart (leviculum `Punched.peer` vs the target list) |
| B5 | proposals between two ends reporting the same public IPv4 | own access x outcome | `cgn_hairpin_share`, `cell_hairpin_share`, `router_hairpin_share` |
| B6 | proposals where both ends had global IPv6, by whether the v6 candidate was acked | own access x acked | `fixed_v6_fw_blocked`, `cell_v6_fw_blocked` |
| B7 | room-peer sends by link kind | own access x (`one_hop`, `forwarded`) via `link_is_forwarded` (`src/transport/reticulum.rs:534`) | the relay share itself, for A/V legs too (A8 covers shaped identity links only) |
| B8 | facilitator probes answered / not, per node rather than per proposal | own access | UDP-blocked share by access (A4 counts per proposal, unbucketed) |
| F1 | **FEATURE**: a second facilitator address on the seat, so a node compares its reflexive port across two destinations (RFC 5780 style) | gives the `mapping` bucket | `router_eim_share`, `cgn_eim_share`, `cell_cgn_eim_share`: the calibrated core of this model |
| F2 | **FEATURE**: 2 bits of the peer's access class in the direct-link REQUEST/ACCEPT (a wire change; the rns-rs DirectLink protocol is shared) | peer access on B3/B4 | the access-pair matrix itself; if refused, B3/B4 on own class still identify the marginals and the pair matrix stays MODEL |

## 6. What a real dataset would replace

No dataset was downloaded. Candidates, each needing Eric's approval (name, what it would
replace; licence and size to be checked on the provider's page before asking):
- A CGN / NAT-behaviour measurement corpus (academic CGN detection studies, STUN-survey data) -
  would replace `fixed_cgnat_*`, `cgn_*`, `router_filter_*`, `cell_*` filter shares.
- Per-network IPv6 deployment statistics (public IPv6 adoption measurement by network) - would
  replace `fixed_v6_*`, `cell_v6_*` and, with the `topology` package's ASes, give each access
  network its own IPv6 share instead of a per-technology draw.
- Fixed-broadband subscriptions by technology per country (OECD-style statistics) - would
  replace `fixed_cable_share`, `fixed_fibre_share` per region.
The loader seam is `ReachModel::network`: a dataset row per access network (CGN yes/no, mapping,
filtering, IPv6 share) would replace that one draw; nothing downstream changes.

## 7. Validity envelope: what the model cannot say

- It is calibrated, not measured: the class marginals are the micro defaults by construction.
  Every pair-level number is MODEL (or FRONTIER where a FRONTIER input dominates).
- Port prediction, TCP punching, UPnP/PCP and TURN-like relaying are absent because edge does not
  do them; if edge adds one, the rule table must change.
- Time is absent: a punch's outcome is a probability per attempt; NAT mapping timeouts, retries
  (`DIRECT_PUNCH_FAILED_BACKOFF` 1 h), network moves and the link's later death are micro's job.
- UDP-blocked networks (the facilitator probe never answers) and enterprise / campus networks
  are not modelled.
- Hairpinning is only modelled for two ends behind one CGN; two devices of one household off
  the LAN, and carrier-internal host candidates, are not.
- The world shape (ISPs per street, market skew) drives the dispersion result directly; it is
  MODEL and the `topology` package's real-AS shape should replace it.
- Tech below fixed/cellular (cable/fibre/DSL, 4G/5G) moves only CGN and IPv6 shares here; no
  source supports a per-technology NAT-behaviour difference beyond that.

## 8. Integration into the micro tier (not done; for the owner of `micro.rs`)

`reach.rs` does not touch `micro.rs`. The seam, smallest hunks:
1. `MicroWorld::build`, household router draw (the `u_reach` / `u_public` block): give each
   household its endpoint from topology's label, `ReachModel::endpoint_of(seed, &node_net[i],
   hi, phone, unmetered)` (section 9.1), store it on the household, and set `router`
   from `Endpoint::class()`. Monotone in the targets: the calibrated shares are thresholds on
   fixed per-network / per-home uniforms, so raising reach still adds routers without
   reshuffling.
2. `go_out_with`: replace `u_punch < cellular_punchable_share` with the node's carrier endpoint
   (`phone_on_carrier`), the same carrier for a household's phones with
   `family_same_carrier_share`.
3. `punch_possible` / `punch_would_succeed`: replace with
   `direct_path(a, b, ctx, &params).mechanism.is_punch()` and `punch_roll < p`.
4. `unpunched_direct`'s listener rule is unchanged (it is the same rule).
The page's `reach_home_share` slider keeps working: it is the calibration target.


## 9. Hooks (HOOKS.md) and the topology adapter

### 9.1 Topology adapter (`crates/cewp-sim/src/reach/topo.rs`)
`topology` (branch `twin/topology`, `crate::netgraph`) labels each micro node with
`NodeNet { home: NetLabel, cellular: Option<NetLabel> }`, `NetLabel { asn, role, metro, region }`.
It carries no access technology and no NAT attribute, by design. `ReachModel::resolve(seed,
&NetLabel)` turns a label into this model's `AccessNet`:

| label | resolves to | class |
|---|---|---|
| `asn` | `AccessNet::id`: one ASN is one network, so every household on it shares its CGN, ISP router, IPv6 and firewall draw | DERIVED |
| `AccessFixed` | Cable / Fibre / Dsl, drawn per ASN by the fixed-technology mix | MODEL |
| `AccessMobile` | Cell4g / Cell5g, drawn per ASN by `cell_5g_share` | MODEL |
| `Content` | `Resolved::Hosted`: a public listener, outside this model | DERIVED (micro's rule) |
| `Tier1`, `RegionalTransit`, `asn == 0` | `Resolved::Unknown`: the caller keeps micro's rule | - |
| `region` (9 page regions) | `RegionClass` V4Rich / V4Scarce, drawn once per region | MODEL |

`ReachModel::endpoint_of(seed, &NodeNet, household, phone, unmetered)` gives a node's endpoint
(home router; phone on Wi-Fi behind it; phone on its carrier off Wi-Fi). The netgraph types are
MIRRORED in `topo.rs` until `twin/topology` merges; on merge, the mirror block becomes one
`pub use crate::netgraph::{AsRole, Asn, MetroId, NetLabel, NodeNet};`. `World::generate` keeps its
own generated ISPs (ids 1..) for the metro experiments of section 4; those results are not yet
re-run on topology's graph. topology's `market_share_ppm` / `market_rank` are not used: CGN
likelihood does not follow carrier size in this model (an open question, not a finding).

### 9.2 Energy counters (`reach::hooks`, DERIVED counts, no time, no watts)
`realise` draws each pair's outcome once (seeded); `count` / `energy_counters` give per node
and per class (micro `HostKind::HomeNode`/`Laptop` = fixed endpoint, `HostKind::Phone` split by
Wi-Fi / carrier): pair-ends, direct, relayed (no direct path formed), punch attempts and
failures (a room-peer pair with no LAN and no listener attempts a punch whatever its NATs, as
edge proposes without knowing the NAT type), and held mappings (direct paths for which the node
keeps a NAT mapping or IPv6 pinhole open: both ends of a punched path, the NATed dialler of a
listener path). The model has no time: it counts held paths, not keepalives; attempts, not
probes (leviculum: at most `PUNCH_WINDOW` / `PUNCH_INTERVAL` = 10 s / 100 ms = 100 probes per
attempt; edge backs a failed pair off `DIRECT_PUNCH_FAILED_BACKOFF` 1 h). It does not choose the
forwarder of a relayed pair (micro's routing does), so relay work per forwarder is micro's to
count. No bytes, radio wake, signatures, storage or AS-hop events exist in this model.

One realised Expected metro (`cewp-cli nat 1`, seed 1, default shape, 400 community pairs; MODEL
world, DERIVED counts):

| class | nodes | pair-ends | direct | relayed | punch attempts (failed) | held mappings |
|---|---|---|---|---|---|---|
| fixed | 480 | 1671 | 1089 | 582 | 617 (392) | 239 |
| phone on Wi-Fi | 771 | 3036 | 2073 | 963 | 886 (587) | 314 |
| phone on carrier | 439 | 1717 | 754 | 963 | 1467 (763) | 754 |

Pairs by outcome: direct IPv6 584, direct listener 79, punched 30, relayed without attempt 383,
punch failed 871, LAN 1265. A phone on a carrier is an end of a relayed pair 56% of the time
(963 / 1717) and holds a mapping for every direct path it has (it never listens): that is the
radio-awake cost the energy package will price.

### 9.3 Globe view (`reach::hooks`, layout agreed with `twin-globe`)
Scenario: 3 streets x 12 households (about 130 devices), 4 ISPs (at most 2 per street), 3
carriers, correlated; every family pair, 120 community pairs, 30 others. All MODEL.
- `nat_view_nodes(seed) -> Vec<u32>`, stride 4: access class (0 fixed, 1 phone on Wi-Fi, 2 phone
  on carrier); NAT behaviour (0 public, 1/2/3 EIM / address-dependent / symmetric mapping, 4/5/6
  the same behind a CGN; to micro `RouterNat`: 0 = Public, 1 and 4 = Punchable, the rest Hard);
  IPv6 (0 none, 1 open, 2 stateful, 3 blocked); household index.
- `nat_view_node_pos(seed) -> Vec<f32>`, stride 2: x, y metres in a generated street frame.
- `nat_view_pairs(seed) -> Vec<u32>`, stride 3: a, b, outcome (0 direct IPv6, 1 direct listener,
  2 hole-punched, 3 relayed without attempt, 4 punch failed then relayed, 5 same LAN).
- `nat_view_joint(seed) -> Vec<f32>`: `[3, 7, 21 shares]`, access class x NAT behaviour over 16
  default-shape metros, summing to 1.
