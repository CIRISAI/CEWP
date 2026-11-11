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

1. **Access network** (`AccessNet { id, tech, region }`, stand-in for the `topology` package's
   access-AS label). `NetBehaviour` is drawn once from the network id: CGN or not, and the CGN's
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
  a feature request, not a counter.

Counters requested:

| # | counter | key | replaces |
|---|---|---|---|
| C1 | nodes seen | access x v6 x nat_public x cgn_hint x mapping | `cell_cgnat_share`, `fixed_cgnat_*` (as a hint), `fixed_v6_*` / `cell_v6_*` (by access only), the class marginals (`reach_home_share`, `cellular_punchable_share`) once `mapping` exists |
| C2 | nodes with any unsolicited inbound dial in the day | access x v6 | `public_listener_share` (and so `port_forward_share`) |
| C3 | direct-link proposals made, and their outcome | own access x own v6 x peer v6 x outcome (`established`, `failed_punch`, `failed_probe`, `failed_timeout`, `rejected`, `glare`, `held`, `lost`) | edge already counts proposals and outcomes (`direct_link_proposed`, `direct_link_established`, `direct_link_failed_*` in `src/observability.rs`); only the bucket key is new |
| C4 | established direct links by winning candidate | own access x candidate (`v4_advertised`, `v4_peer_reflexive`, `v6`, `same_nat_host`) | the mechanism mix: tells IPv6 punch, hole punch and peer-reflexive punch apart (leviculum `Punched.peer` vs the target list says which won) |
| C5 | proposals between two ends reporting the same public IPv4 | own access x outcome | `cgn_hairpin_share`, `cell_hairpin_share`, `router_hairpin_share` |
| C6 | proposals where both ends had global IPv6, by whether the v6 candidate was acked | own access x acked | `fixed_v6_fw_blocked`, `cell_v6_fw_blocked` |
| C7 | room-peer sends by link kind | own access x (`one_hop`, `forwarded`) via `link_is_forwarded` | the relay share itself (the output of this model) |
| C8 | facilitator probe outcomes | own access x (`answered`, `no_answer`) | UDP-blocked networks (not modelled; see 6) |

With C1 to C4 bucketed by **own and peer** access class the table becomes the access-pair matrix
of section 4. The peer's `access` is not on the wire today; carrying 2 bits of it in the
direct-link REQUEST/ACCEPT is a wire change (and the rns-rs DirectLink protocol is shared) - if
that is refused, C3/C4 keyed on own class only still identify the marginals, and the pair matrix
stays MODEL.

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
   household an `AccessNet` (from `topology` once it lands), store
   `ReachModel::home(seed, &model.network(seed, net), hi)` on the household, and set `router`
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

