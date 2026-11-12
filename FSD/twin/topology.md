# Twin package: AS-aware network topology (`crates/cewp-sim/src/netgraph.rs`)

Status: built on branch `twin/topology`. The default graph is a **MODEL stand-in**. It is generated, it is NOT real Internet data, and every printed report opens by saying so (`Provenance::banner`). Real data drops in behind one loader seam once a dataset is approved (§6).

## 1. What changed and why

Before this package the twin used geography as network distance. Every device sat on the population grid, and its distance to a peer was the great circle between them. Real paths do not follow geography. Two homes in one city on different ISPs can hand traffic over hundreds of kilometres away. Two homes on different continents whose ISPs peer at a shared exchange can be close.

`netgraph` gives the twin a network layer:

- A path between two devices is an **AS path**: access ISP, then transit and/or an IXP, then access ISP.
- **Latency** follows the metros and cables that path actually crosses.
- **Shared fate** is the set of ASes, IXPs, cables and metros the path depends on.

## 2. Data model

All quantities are integers: kilometres, microseconds, ppm and permille. The great circle (`gc_km`) is the one float computation. It uses only correctly rounded IEEE-754 operations (+ − × ÷ √) with its own sine and arcsine series and never calls libm, so native and wasm builds give the same kilometre.

| type | meaning |
|---|---|
| `Metro` | A place where networks meet: `lat_cdeg`, `lon_cdeg` (0.01°), population `weight`, `region`. The stand-in uses the page's 181 localities (§3.1). |
| `AutonomousSystem` | `asn`, `role`, `region`, `home_metro` (the headquarters; a mobile AS's packet gateway), `pops` (metros with a point of presence), `market_weight`, and its `market_share_ppm` and `market_rank` in its home metro. |
| `AsRole` | `Tier1`, `RegionalTransit`, `AccessFixed`, `AccessMobile`, `Content` (cloud/CDN/hosting). |
| `Ixp` | `metro`, `name`, `members`. |
| `AsLink` | `a`, `b`, `rel` (`ProviderCustomer`, with a = provider, or `Peer`), and `at`: every `Interconnect { metro, ixp: Option<IxpId> }` where the two hand traffic over. |
| `Cable` | `kind` (`Terrestrial` / `Submarine`), end metros `a`, `b`, route `km`. |
| `NetLabel` | A node's network position: `asn`, `role`, `metro`, `region`. |
| `NodeNet` | Per micro node: `home` (household fixed AS, or the data centre's content AS) and `cellular` (a phone's carrier). `current(unmetered)` picks one. |
| `SharedFate` | Sorted, deduplicated `ases`, `ixps`, `cables`, `metros`. Has `contains(FateElement)`, `overlap`, `elements`. |
| `NetPath` | `ases`, `dirs` (Up / Across / Down), `handoffs`, `metro_trail`, `km`, `latency: LatencyParts`, `latency_us`, `fate`. |

### 2.1 Interconnect rule (one rule for the stand-in and for loaded data)

`NetGraph::assemble` is the only constructor, and the generator and the loader both end in it.

- **Peering pair:** the two meet at every IXP both belong to, and privately at every other PoP they share. If they share nothing, they meet at the lower ASN's PoP nearest the other's home.
- **Customer and provider:** they meet at every PoP they share. Otherwise the customer backhauls to the provider's PoP nearest its home. This is the rule that makes traffic from a small metro trombone through a larger one.

### 2.2 Routing

1. **AS path.** `as_path` does a breadth-first search over (AS, phase) states. Phase 0 may still climb; phase 1 only descends. The search returns the valley-free path with the fewest AS hops: up customer→provider any number of times, across at most one peering link, then down provider→customer. Ties go to the path found first in ascending-ASN neighbour order. *MODEL:* real BGP chooses by local preference (customer > peer > provider) before path length. Measured paths (RIPE RIS / RouteViews, §6) are what would check this rule.
2. **Geography (hot potato).** Starting at the source device's metro, each AS hop leaves at the interconnect nearest to where the packet is now, measured by backbone distance (early exit). A mobile source first hauls to its AS's gateway metro, and a mobile destination is reached through its gateway (`mobile_central_gateway`, MODEL). The last leg goes to the destination device's metro.
3. **Backbone.** The backbone is shortest paths in kilometres over the cable graph: Dijkstra from every metro, computed once at assembly.
4. **Latency.** One-way latency is the sum of four terms (`LatencyParts`):
   - propagation: km × `fibre_ns_per_km`;
   - AS hops: ASes crossed × `as_hop_us`;
   - IXP crossings: IXP fabrics crossed × `ixp_crossing_us`;
   - access: the access term of each end (`access_fixed_us`, `access_mobile_us` or `access_content_us`).
5. **Shared fate.** Every AS on the path, every IXP fabric crossed at a handoff, every cable on a backbone leg, and every metro visited.

## 3. The stand-in generator (`generate`, `generate_raw`)

Each stage draws from its own splitmix64 stream, seeded from the graph seed (default 7). The draws are integer only. ASNs come from the RFC 6996 private-use range starting at 4,200,000,000, so no stand-in AS can be mistaken for a real network.

### 3.1 Metros: DERIVED

`LOCALITIES` holds the 181 localities of `POP_GRID_B64` in `docs/globe.js`. For each locality it keeps the coordinates of its most populous grid cell, the sum of its cell weights and its region. The test `localities_match_the_page_grid` decodes the page's grid itself and checks every row. These are real places at coarse grain. No other geography is used.

### 3.2 Stages

1. **Tier-1s.** `tier1_count` networks. Each is present in a region with probability `tier1_region_presence`, and in each region it is present in, it has PoPs in the `tier1_pops_per_region` largest metros. At least 2 tier-1s are forced into every region. All tier-1s peer with each other (full mesh).
2. **Content (cloud/CDN/hosting).** `content_count` networks, each in a region with probability `content_region_presence` and in its `content_pops_per_region` largest metros there. At least one is forced into every region. Each buys transit from `content_upstreams` tier-1s.
3. **Regional transit.** `transit_per_region` per region. The home metro is drawn by population weight. PoPs are the largest `transit_pop_share` of the region's metros plus the home. Each buys from 1..=`transit_upstreams_max` tier-1s present in its region. Each pair of transits in a region peers privately with probability `transit_peer`.
4. **Incumbent markets.** These stand in for countries, because the grid has none. The generator walks each region's metros from the largest down. Every metro no incumbent covers yet founds one, which covers all of the region's metros within `incumbent_reach_km`. An incumbent's first upstream is a tier-1 with probability `incumbent_tier1`, otherwise a transit; its second is a transit. Each market gets `mobile_per_market` carriers covering the same metros, with weights `mobile_weight_top/(k+1)`. Each carrier buys from 1–2 of the region's transits and tier-1s.
5. **Local fixed ISPs.** A metro gets `local_fixed_base` local ISPs, plus one per doubling of its weight above `local_fixed_weight_step`, capped at `local_fixed_max`. Weights are `local_weight_top/(k+1)`. Each buys from one regional transit, and with probability `access_multihome` also from a second transit or an incumbent covering the metro.
6. **IXPs.** A metro with weight ≥ `ixp_min_weight` hosts one IXP, and one with weight ≥ `ixp_second_weight` hosts two. Every AS present at the metro joins with a probability set by its kind: `ixp_join_content`, `ixp_join_transit`, `ixp_join_access`, `ixp_join_incumbent` (incumbents and carriers), `ixp_join_tier1`.
   - Two members peer with probability `ixp_peer`, or `content_peer` if either is a cloud network.
   - That probability is multiplied by `restrictive_peering` if either is an incumbent or a carrier.
   - Tier-1s never peer with non-tier-1s, and a pair already in a transit relationship never peers.
7. **Cables.**
   - Inside each region: a minimum spanning tree over its metros by great circle, plus each metro's `terrestrial_extra_links` nearest neighbours. All of these are `Terrestrial`.
   - Between regions: for each adjacent pair in `REGION_LINKS`, the `cables_per_region_pair` closest disjoint pairs among each region's `gateway_candidates` largest metros. Each pair in that table is marked land or sea (MODEL).
   - Route km = great circle × `terrestrial_inflation` or `submarine_inflation`.
8. **Markets.** In every metro, the fixed, mobile and content networks present split 1,000,000 ppm by `market_weight`, using largest-remainder apportionment (`tiling::apportion`).

### 3.3 Parameters

Every number in this table is printed by `cewp-cli netgraph` with its tag.

| name | default | unit | class | reasoning / source |
|---|---:|---|---|---|
| `tier1_count` | 12 | ASes | MODEL | transit-free networks; commonly 12-20 are called tier-1, the low end keeps the stand-in small |
| `tier1_region_presence` | 700 | permille | MODEL | chance a tier-1 has PoPs in a given region; global carriers skip some regions (at least 2 per region are forced) |
| `tier1_pops_per_region` | 3 | metros | MODEL | a tier-1 present in a region sits in its largest metros |
| `content_count` | 8 | ASes | MODEL | hyperscale cloud / CDN / hosting networks the off-street nodes live in |
| `content_region_presence` | 800 | permille | MODEL | chance a cloud network has PoPs in a region (at least one per region is forced) |
| `content_pops_per_region` | 3 | metros | MODEL | cloud regions sit in a region's largest metros |
| `content_upstreams` | 2 | ASes | MODEL | tier-1 transit a cloud network buys besides peering |
| `transit_per_region` | 6 | ASes | MODEL | regional transit providers per region |
| `transit_pop_share` | 400 | permille | MODEL | share of a region's metros (largest first) where a regional transit has a PoP |
| `transit_upstreams_max` | 3 | ASes | MODEL | a regional transit buys from 1..=this many tier-1s |
| `transit_peer` | 400 | permille | MODEL | chance two transits of a region peer privately at their common PoPs |
| `incumbent_reach_km` | 700 | km | MODEL | a national fixed incumbent covers its region's metros within this distance of its home; stands in for a country (the grid has no countries) |
| `incumbent_tier1` | 500 | permille | MODEL | chance an incumbent's first upstream is a tier-1 rather than a regional transit |
| `mobile_per_market` | 3 | ASes | MODEL | mobile network operators per incumbent market (3-4 per country is the common structure) |
| `mobile_central_gateway` | 1 | bool | MODEL | 1: a mobile AS hauls all traffic to its home metro (packet gateway) before leaving its network |
| `local_fixed_base` | 1 | ASes | MODEL | local fixed ISPs every metro has besides incumbents |
| `local_fixed_weight_step` | 2,000 | grid weight | MODEL | one more local ISP per doubling of a metro's population weight above this |
| `local_fixed_max` | 5 | ASes | MODEL | cap on local fixed ISPs per metro |
| `access_multihome` | 300 | permille | MODEL | chance a local ISP buys a second upstream |
| `incumbent_weight` | 4,500 | market weight | MODEL | an incumbent's subscriber weight in each metro it covers |
| `local_weight_top` | 2,000 | market weight | MODEL | weight of a metro's largest local ISP; the k-th has top/(k+1) (Zipf, exponent 1) |
| `mobile_weight_top` | 4,000 | market weight | MODEL | weight of a market's largest carrier; the k-th has top/(k+1) |
| `ixp_min_weight` | 3,000 | grid weight | MODEL | a metro at least this populous hosts an IXP |
| `ixp_second_weight` | 20,000 | grid weight | MODEL | a metro at least this populous hosts a second IXP |
| `ixp_join_content` | 950 | permille | MODEL | cloud networks join nearly every IXP where they have a PoP (open peering) |
| `ixp_join_transit` | 600 | permille | MODEL | regional transit joins an IXP at a PoP |
| `ixp_join_access` | 500 | permille | MODEL | local fixed ISP joins its metro's IXP |
| `ixp_join_incumbent` | 300 | permille | MODEL | incumbent fixed / mobile networks join less (restrictive peering) |
| `ixp_join_tier1` | 50 | permille | MODEL | tier-1s rarely peer publicly |
| `ixp_peer` | 600 | permille | MODEL | chance two IXP members peer (route servers make multilateral peering common) |
| `content_peer` | 900 | permille | MODEL | chance a cloud network peers with an IXP co-member |
| `restrictive_peering` | 250 | permille | MODEL | multiplier on ixp_peer when either side is an incumbent or mobile operator |
| `terrestrial_extra_links` | 2 | links | MODEL | nearest-neighbour backbone links per metro beyond the region's spanning tree |
| `cables_per_region_pair` | 2 | cables | MODEL | long-haul cables per adjacent region pair (closest gateway metro pairs) |
| `gateway_candidates` | 8 | metros | MODEL | a region's largest metros that may land an inter-region cable |
| `terrestrial_inflation` | 1,500 | permille | MODEL | fibre route length over great circle on land (roads, rail, rights of way) |
| `submarine_inflation` | 1,250 | permille | MODEL | cable route length over great circle at sea |
| `fibre_ns_per_km` | 4,897 | ns/km | DERIVED | c / n with silica group index n = 1.468: 299,792 km/s / 1.468 = 204,218 km/s |
| `as_hop_us` | 300 | us one-way | MODEL | per AS crossed: border routers, queueing at an uncongested hop |
| `ixp_crossing_us` | 200 | us one-way | MODEL | an IXP fabric crossing (switch hops inside the exchange) |
| `access_fixed_us` | 4,000 | us one-way | MODEL | fixed last mile + aggregation (FTTH / DOCSIS / DSL mix, idle line) |
| `access_mobile_us` | 15,000 | us one-way | MODEL | mobile radio + core one-way (4G/5G mix, idle cell) |
| `access_content_us` | 200 | us one-way | MODEL | a data-centre host to its network edge |

## 4. Micro-tier assignment (`assign_micro`)

`MicroWorld` is read only; `micro.rs` is unchanged. The patch's metro is the locality nearest `(lat0, lon0)`.

- **Household devices** (home node, laptop, and a phone on Wi-Fi) share one fixed AS. It is drawn from the metro's fixed market on the household's own stream, so adding households never reshuffles the existing ones.
- **A phone** also gets a carrier, drawn per (household, person) from the metro's mobile market. `NodeNet::current(unmetered)` gives the carrier when the phone is off Wi-Fi.
- **Seats, hosted servers and CI runners** get the content AS at the metro nearest their site that has one. The draw is per site, so all the nodes of one site share a network.

The `nat` package conditions NAT behaviour on `NetLabel` (role, asn, market rank). The `failures` package consumes `SharedFate` and the per-node `asn`/`metro`.

## 5. Findings (MODEL outputs of the stand-in, seed 7, default parameters)

All numbers below are **MODEL outputs** of the stand-in at seed 7 with default parameters
(`cargo run -p cewp-cli -- netgraph`, and the `netgraph` unit tests, which pin them). They
describe the generator, not the Internet. Nothing here is MEASURED.

| quantity | value | class |
|---|---:|---|
| metros (page localities) | 181 | DERIVED (grid) |
| ASes | 839: tier-1 12, regional transit 54, fixed access 501, mobile 264, content 8 | MODEL |
| AS links | 2,614 (view: 1,265 provider-customer, 1,349 peering) | MODEL |
| IXPs / memberships | 95 / 833 | MODEL |
| long-haul cables | 285 (24 submarine) | MODEL |
| AS degree | median 2, p90 12, p99 93, max 143; Hill alpha over the top 83 = 1.024 (test band [0.7, 2.0], max/median >= 20) | MODEL |
| Spearman(great-circle km, one-way latency), 4,000 fixed-fixed pairs | 954 permille (pure geography = 1,000 by construction; 964 on the old unsourced grid, before GHS-POP 2020 moved the metros) | MODEL |
| same, the pairs under 1,000 km | 887 permille (915 on the old grid) | MODEL |
| same-metro pairs on different ISPs | 253; 7 leave the metro, 0 leave the region | MODEL |
| same-metro one-way latency | median 9,100 us, p90 9,400 us, max 65,251 us | MODEL |
| pairs >= 1,500 km apart faster than the slowest same-metro pair | 1,511 of 2,749 (fastest 20,471 us) | MODEL |
| slowest same-metro pair | fixed -> tier-1 -> transit -> fixed, 11,446 km of fibre inside Sub-Saharan Africa (tromboning: the tier-1 hands over far from the metro) | MODEL |
| globe view (seed 7) | 1,622 points (181 metros, 1,198 ASes, 243 IXPs), 6,161 links on the GHS-POP 2020 grid (1,115 points, 839 ASes, 95 IXPs, 3,732 links on the old unsourced grid, whose weights were about 6.5 times smaller in sum; the generator sizes ASes by metro weight) | MODEL |

Reading. The decorrelation is real but sits in the tail, not the bulk: most pairs still
follow geography (rank correlation 0.96), while a minority of same-city pairs trombone
through a distant hand-over and become slower than half the pairs 1,500 km+ apart. A first
guess that the stand-in would sit below 950 / 800 permille failed (964 / 915); the test now
pins the measured numbers so a generator change must restate them here. Whether the real
Internet is more or less decorrelated cannot be said without RIPE Atlas (or similar)
latency data, which needs approval (§6).

Energy hook (HOOKS.md §1): every `NetPath` carries `PathCounters { as_hops, ases_crossed,
ixp_crossings, cable_segments, long_haul_km, submarine_km }`, DERIVED from the route (MODEL
inputs with the stand-in). This model has no bytes, radio, signature or storage events; it
counts none.

Globe hook (HOOKS.md §2): `netgraph::view` (`topology_view_points`, `topology_view_links`,
`topology_view_path`, `view_classes`, `TopologyView`); layouts in that module's docs.

## 6. Dataset approval brief

Nothing has been fetched. URLs and terms come from each project's own licence or terms pages, read on 2026-11-10. Where a page could not be read, the brief says so.

**Bottom line.** Only RouteViews (CC BY 4.0) and OpenStreetMap (ODbL) clearly let raw data sit in a public repository.
- PeeringDB and the RIPE Data Repository forbid redistribution outright.
- CAIDA's Public AUA grants a "non-transferable" licence and says nothing about redistribution.

The default for every dataset below is therefore: **keep the raw files outside every repo, as `tools/geo/fetch_dem.sh` does for ETOPO** (a manifest of URL, bytes and sha256), and commit only derived aggregates where the terms allow it. The loader seam (`TopologyFiles`) reads converted local files, so no raw byte needs to enter the repo.

One question decides several rows: **is CEWP's use research / non-commercial?** PeeringDB excludes "any other commercial application". The RIPE Data Repository forbids "any commercial purposes". RIS and RouteViews add logo and boilerplate conditions for commercial use.

| # | dataset | URL | licence (page read) | raw in a public repo? | non-research use | derived aggregate committable? | size (as stated) | what it replaces in the stand-in |
|---|---|---|---|---|---|---|---|---|
| 1 | CAIDA AS Relationships (serial-2) | https://www.caida.org/catalog/datasets/as-relationships/ | CAIDA **Public** AUA (https://www.caida.org/about/legal/aua/public_aua/): "limited, non-exclusive, non-transferable … license to copy, modify, and use"; "No license is granted for any other purpose" | **Not granted** (redistribution not mentioned; the licence is non-transferable). Keep local. | No research-only clause in the Public AUA (the stricter Master AUA does not apply to this set) | Yes, with the citation "The CAIDA UCSD AS Relationships Dataset, <dates>", and the publication reported to CAIDA | Not stated; monthly files | Every generated relationship (`rels`): tier-1 mesh, transit purchases, IXP peering decisions. `as_rel` takes the file unchanged. |
| 2 | CAIDA AS Rank (+ API) | https://asrank.caida.org/ | **No terms found** on the site or in the API docs; only a BibTeX entry | Unknown: ask CAIDA | Unknown | Unknown | n/a (API) | `role` in `as_meta` (customer-cone size separates tier-1 / transit / access), replacing the five generated role stages |
| 3 | PeeringDB | https://www.peeringdb.com/ ; AUP https://www.peeringdb.com/aup | PeeringDB AUP, no open licence: "The PeeringDB data may not be passed on in bulk to any other person or organization unless approved by PeeringDB" | **Forbidden** without PeeringDB's approval | "Internet research and analysis" is an allowed operational use; "compiling marketing lists, demographic mapping, or any other commercial application" is excluded | AUP is **silent**: ask PeeringDB | Not stated (live database) | `ixps`, `ixp_members`, AS `pops` (`netfac` facility points) and `role` (`net.info_type`): the IXP stage, the `ixp_join_*` and `ixp_peer` parameters, the PoP rules |
| 3b | CAIDA PeeringDB archive | https://www.caida.org/catalog/datasets/peeringdb/ | CAIDA Public AUA plus citation "The CAIDA UCSD PeeringDB Dataset" | Same as 1. PeeringDB's own AUP presumably still binds the content, but neither side says so. | as 1 | Yes, with citation | Not stated; daily snapshots | as 3, with dated snapshots |
| 4 | RouteViews | https://www.routeviews.org/ ; https://www.routeviews.org/routeviews/licenses/ | **CC BY 4.0** ("licensed under a Creative Commons Attribution 4.0 International License") | **Allowed** with attribution. The homepage links the CC BY-SA deed, the licences page plain BY; treat it as BY-SA to be safe. | Allowed; commercial products must show the boilerplate and logo, and the licence is revocable without attribution | Yes | Not stated; RIB dumps are large (unverified) | Checks route choice: real AS paths against the shortest-valley-free rule (§2.2). Also a second relationship inference. |
| 5 | RIPE RIS | https://www.ripe.net/ris | RIPE NCC Terms of Service §3.5 ("shall not re-package, download, compile, re-distribute or re-Use … unless permitted"), plus a separate, revocable RIS Commercial Use T&C | **Ambiguous**: keep local; prefer RouteViews | Commercial use permitted with logo, link and boilerplate | Yes, with attribution | Not stated; bview every 8 h, updates every 5 min | as 4 |
| 6 | RIPE Atlas built-in measurements | https://atlas.ripe.net/ ; RIPE Data Repository T&C https://www.ripe.net/analyse/raw-data-sets/terms-conditions/ | Repository T&C: "The User must not re-distribute the RIPE Data Repository or the Data to anybody else"; "any commercial purposes … is not allowed". The Atlas-specific terms page **could not be read** (JavaScript-only). | **Forbidden** | **Forbidden** under the Repository T&C | Yes for research results, with attribution to the RIPE NCC and notice of the publication | Not stated; daily dumps | Calibrates the latency terms: `as_hop_us`, `ixp_crossing_us`, `access_*_us`, both route inflations. Also the first real test of the §5 decorrelation numbers. |
| 7 | Submarine cables: TeleGeography Submarine Cable Map | https://www.submarinecablemap.com/ | **Not read**: the site rendered only its title, and the GitHub repo once cited for its data returns 404. CC BY-NC-SA is often quoted but **unconfirmed**. | unknown | unknown (NC would forbid commercial use) | unknown | unknown | `REGION_LINKS`, `cables_per_region_pair`, `gateway_candidates`, `submarine_inflation`; also fixes the "every intra-region link is terrestrial" limit (§7) |
| 7b | OpenStreetMap (submarine cable ways) | https://www.openstreetmap.org/copyright | **ODbL 1.0**: "free to copy, distribute, transmit and adapt … as long as you credit OpenStreetMap"; altered data only under ODbL | **Allowed**, share-alike: a derived cable file must itself be ODbL | Allowed | Yes, as ODbL | n/a | as 7. Coverage of submarine cables in OSM was **not checked**. |
| 8 | RIR delegation statistics (`delegated-<rir>-extended-latest`) | https://www.nro.net/about/rirs/statistics/ ; https://ftp.ripe.net/pub/stats/ripencc/ ; https://ftp.arin.net/pub/stats/arin/ | No licence in the files or READMEs read. RIPE NCC ToS §3.5 applies generally (ambiguous for these public files). No ARIN data-use clause found. | **Ambiguous**: keep local | Not stated | Not stated | RIPE file 17 MB, ARIN about 12.9 MB; daily | ASN → registration country, which would replace the `incumbent_reach_km` distance clusters with real country markets. Address-family figures belong to the `nat` package, not here. |

### 6.1 Recommended order

Each step is independent of the ones after it, and each lands as a local file read through `TopologyFiles`.

1. **CAIDA AS Relationships.**
   - Why first: it alone turns the relationships from MODEL into data, and its terms are the clearest of the measured AS sets.
   - Commit: degree distribution, per-region AS counts, tier-1 clique size, with the CAIDA citation.
   - Blocks nothing.
2. **PeeringDB via the CAIDA archive** (or PeeringDB directly, with its approval). IXPs, members and facility points.
   - Depends on 1 for the AS list.
   - Commit: per-metro IXP and member counts, only if PeeringDB agrees (its AUP is silent on aggregates).
3. **RouteViews RIBs.** These check the route-selection rule against real AS paths. It is the only measured source here that is plainly CC BY.
4. **RIPE Atlas built-in measurements.** These calibrate the latency terms and give real decorrelation numbers. Atlas is non-commercial and no-redistribution, so only published aggregates can enter the repo.
5. **A cable dataset.** OSM if its coverage is adequate. TeleGeography only after its licence is confirmed.
6. **RIR delegation statistics**, for country markets.
7. **AS Rank**, after CAIDA states its terms. Until then, roles can come from PeeringDB `info_type` and cone size computed locally from 1.

## 7. Validity envelope: what this model cannot say

- **Not real routes.** The stand-in is not the Internet. No AS, IXP, relationship or cable in it exists. A latency or a shared-fate set it produces is a statement about the stand-in only. It cannot be compared with a real measurement until §6's data replaces the generated pieces, and no number from it may be presented as MEASURED.
- **Route selection is shortest valley-free, not BGP policy.** Local preference, MED, prepending, traffic engineering and anycast are absent. A real path can be longer in AS hops than the stand-in's.
- **Intact graph only.** `path` does not reroute around a failed element. Rerouting after a failure is a separate question for the `failures` package (an avoid-list variant is offered).
- **No congestion.** Latency is an idle-path estimate: no queueing beyond the per-hop constant, no diurnal load, no peering-link saturation. Link contention belongs to the `media` package.
- **Coarse grain.** 181 metros cover the world. A metro is a locality of the population grid, not a city. A device's metro is the nearest locality, so a suburb or small town is folded into its locality.
- **Countries are approximated.** Incumbent markets are distance clusters (`incumbent_reach_km`) inside a region, not countries, so regulatory borders are absent.
- **Cable kinds inside a region are all terrestrial.** Rust has no land mask, so a sea crossing inside a region (Japan, the UK, Australia–Indonesia inside "SE Asia & Oceania") is labelled terrestrial and inflated as land. Only the inter-region table marks submarine links.
- **Mobile is one gateway per carrier.** Real carriers run several regional packet gateways, and local breakout is growing with 5G.
- **IPv4/IPv6 is not modelled.** The address family does not enter the path. The RIR statistics in §6 would inform the `nat` package, not this one.
