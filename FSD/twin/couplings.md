# Twin couplings: what the twin reads from a scene, models on invented inputs, and wants

The registry the twin hands to CIRISHolon (`engine/FSD_WORLD_MODEL.md` section 10). One source:
`crates/cewp-sim/src/couplings.rs`. It writes `docs/geo/couplings.json` and the tables at the end
of this file:

```
cargo run -p cewp-sim --example couplings -- --write
```

`cargo test -p cewp-sim couplings` fails if either file is stale, if a MODEL or FRONTIER parameter
of any spec array (`PARAM_SPECS`, `SOCIAL_PARAM_SPECS`, `ACTIVITY_PARAM_SPECS`, `NET_PARAM_SPECS`,
`REACH_PARAM_SPECS`, `FAILURE_SPECS`, `MEDIA_PARAM_SPECS`) has no row, if a parameter sits in two
rows, or if an attribute is read with a name that is not a row. `COST_SPECS` describes the seat's
software; only its device-behaviour parameters are world rows (`device.daily_events`).

## 1. Rows

Every row is a registry NAME with a type, a tier, a unit, a consumer and a status.

| status | part | meaning |
|---|---|---|
| `read_today` | A | a twin model reads it from the generated scene today (the holon tree, the leaf layout and the focal world built on it, or the scale engine's cohort grid). The consumer is `file:function`, or `decoration` when only the drawing uses it, so no certificate is asked for a picture. |
| `invented_today` | B | a package models it on MODEL or FRONTIER inputs that a certified scene should supply. |
| `wanted` | C | nothing models it yet; the consumer is `future: ...`. |

Types are Holon's four: a **population** lists every parent kind (a household's parents are its
cell and, once item 6 (g) lands, its access network; a phone adds a carrier; a building adds a
power region); a **relationship** names the two kinds it joins and whether it is directed; an
**attribute** names its object and the rows any model reads it **jointly** with (each such pair
needs a joint certificate); a **rhythm** is a population or relationship per bin. `class` is the
class of today's value; `params` are the spec parameters standing behind it, each with its class.
`reference` is a first guess from the open list only, or `none known`.

## 2. Bins

168 bins, hour of week. Bin 0 is Monday 00:00-01:00 in the holon's local zone time. Where the twin
knows only longitude (all of it today), the zone is round(lon / 15) hours, stated in the row. The
working week is a class coordinate `week` = `mon-fri` | `sun-thu` | `sat-wed`, keyed by region.
Background protocol traffic is not registered: it stays in the twin, flat.

## 3. What part A found, against the questions asked

Read from the code, not from memory.

- **People and households per holon, per leaf, per building** are read: popchain apportions both
  exactly down the tree, the leaf layout turns households into dwelling units, and
  `micro.rs:MicroWorld::populate` puts one household in each unit. **People per household** is read
  (it sets phones and laptops per person and the hosted-server chance), and it now sums exactly to
  the leaf's people.
- **Devices per household** is read by nothing downstream except the focal world itself; the leaf's
  device total is recorded as its device population, and the 1,000-node cap (`R5_MAX_NODES`) is a
  display sample of it.
- **Neighbour distance and same-street relations are read by no model.** The only neighbour
  relation any model reads is the household LAN (`unpunched_direct`: two devices of one household,
  a phone only on Wi-Fi). The community room is the whole street's, not a distance. Household
  positions are drawing (and the Story's choice of its four actors).
- **Distance to the serve seat** is drawn and shown, but no micro model reads a latency from it;
  the netgraph latency is metro to metro. Decoration today.
- **Household to access network** is now attached to every household of the focal world
  (`netgraph.rs:attach_access`, called by kernel-wasm on every build), and the households summed by
  street tile and by access network agree for every seed. No model reads it yet: micro's reach is
  still its own per-household router draw, so the row stays in part B until the NAT seam of
  `FSD/twin/nat.md` section 8 makes the router class follow the network.
- **Building storeys and arterial crossings** are decoration: storeys set glyph height and shadow,
  crossings join neighbouring tiles' arterials, and nothing reads either. Streets are now a graph
  (item 6 (f): street ids, junctions by kind, building and household links), which the tiled street
  drawing reads for its end caps; no model reads it, so it stays decoration. A small leaf whose
  ancestors' lines all pass outside it has no street at all (the Cairo and Paris probe leaves):
  its households are on no street.
- **Street betweenness.** CIRISHolon's street reference includes the Gini coefficient of
  betweenness over a tile's street segments (generated networks tend to match orientation and degree
  while missing how unevenly traffic concentrates). The twin computes it on its generated graph:
  `StreetGraph::segment_betweenness` (Brandes, shortest paths by length, each street cut at its
  junctions) and `betweenness_gini`, exported as kernel-wasm `holon_street_stats` (never per frame).
  On the probe leaves it reads 0 to 0.12 (Zurich 0.119 over 6 segments, the Cher village 0.110 over
  22, Kowloon 0.117 over 12, Dhaka, Delhi, Tokyo and the Maryland suburb 0 over 3 or 4): a leaf of
  at most 60 households is too small a network for the statistic. A comparison with the reference
  needs the network assembled across leaves at the reference's tile size (about 1 km), with the
  ports joining neighbouring leaves; owed.
- **Day and night by longitude** is read by the scale engine's signer draw (`diurnal_accept`) and
  by the focal world's act rate (`MicroWorld::generate`).
- **Land cover and relief** are decoration (ground tint and shading).

## 4. Generators and their verdicts

`crates/cewp-sim/src/workload.rs:diurnal_multiplier` was registered on Holon's side as generator
**model.diurnal.v0** (trough 0.20, peak 2.28 at 20:30). It is now **model.diurnal.v1** (trough
0.50, peak 1.60 at 19:30, a daytime plateau of 0.89 to 1.06 from 09:00 to 17:00, mean 1.0 over the
day), class MODEL, verdict **unresolved: no reference**, attached to every
`activity.<kind>.events` row. With a US time-use survey as the only reference, a peak hour shown
anywhere else is a model's. The page must show any activity curve with that verdict beside it.

## 5. Rhythm rows obey a continuity law

CIRISHolon (main 7b36c65; `lean/CIRISHolon/SceneRhythm.lean`, `engine/crates/holon-scene/src/rhythm.rs`)
gives rhythm rows a law: per cell per bin, next = now - what leaves + what arrives + source, as a
`Series { cells, counts[bin][cell], flows[bin] = [(from, to, amount)], source[bin][cell] }` of
integers.

- `presence.people` is emitted per bin as people per place kind AND the flows between place kinds
  in that bin, with declared source 0: a person who appears at work without leaving home is a named
  violation.
- `attach.phone.network` is the same with network kinds as the cells.
- `activity.<kind>.events` rows are events, not conserved; they carry only the weekly-volume sum
  (`activity.<kind>.weekly`).
- `check_sums` accepts an optional `"bin": k` on a line, so the twin's sums file can carry rhythm
  lines once the seam exists.

## 6. Owed

- The per-kind seam: the twin's act generators read `activity.<kind>.events`,
  `presence.people` and `attach.phone.network` instead of `diurnal_multiplier` and the flat dwell
  shares (`phone.attach`, `device.awake`, `presence.work_hours`). Not built in this round.
- The globe renders a Series directly (cells filling and emptying, arrows along flows), not
  converted to the twin's own shape.
- The page shows each activity curve with its verdict.
- The NAT seam of `FSD/twin/nat.md` section 8 (router class from the household's access
  network), which moves `household.access_network` to a model that reads it. Streets (item 6 f)
  and arterial crossings (item 6 h) are built and summed but stay decoration: no model reads them.
- A lane for a populated leaf that no line reaches, so every household is on a street.
- Street betweenness over an assembled 1 km network (ports joining leaves), to compare with CIRISHolon's Gini.

## 7. Gaps with no free reference, and what the fleet could measure instead

Holon's licence check found these with no reference the twin may use. For each, what the seat's
counters or edge's fleet gauges could measure. Every fleet figure is a count summed over the fleet,
keyed only by coarse self-computed buckets, under a k-threshold (`FSD/twin/nat.md` section 5).

| gap | rows | what the fleet could measure |
|---|---|---|
| persons per household | `household.persons`, `cell.households` | not persons: no gauge should count a home's people. Devices per owner is measured already (seat-1 relay report: 1,349 announced nodes, 1,338 owners, MEASURED, `capacity.rs` `announced_nodes`, `announced_owners`); devices per household is not observable without a household identity, which edge does not have. Stays MODEL. |
| measured access rates | `access_link.rate`, `access_link.loss` | the A/V relay controller already learns an uplink ceiling from delivered bytes (edge v45); a daily fleet histogram of that ceiling and of chunk loss by `access` bucket (fixed / cellular) would replace the rates by class. New edge work (proposed). |
| NAT and IPv6 behaviour | `household.router_class`, `household.router_behaviour`, `access_network.cgn`, `access_network.ipv6`, `carrier.nat` | List A in `nat.md` section 5: the `direct_link` labels (proposed, established, failed punch, failed probe, other endings, no facilitator), `direct_links_active`, `link_shaping_unshaped:forwarded`, `link_stall` retired direct; all exported at 8e46165 and summable today without a bucket key. List B (B1-B8) buckets them by access, IPv6, public IPv4 and CGN hint; F1 (a second facilitator address) gives the mapping class. |
| exchanges and cables | `household.nearest_exchange`, `cable.links`, `path.latency` | a seat-side RTT histogram per `access` bucket would bound the access-to-metro delay (`access_fixed_us`, `access_mobile_us`); cables and exchanges themselves are not observable from the fleet. Stays MODEL. |
| towers | `household.nearest_tower`, `tower.backhaul`, `tower.backup_hours` | the fleet sees cellular attachments (B1 `access` = cellular) but nothing per tower; tower backup during a grid outage could be read only from devices going offline during a known outage, which no gauge records. Stays FRONTIER. |

Not in any reference tranche by design: anything with non-commercial, no-derivatives or
no-redistribution terms.

## 8. The registry

<!-- couplings:begin (generated by crates/cewp-sim/examples/couplings.rs; do not edit) -->
### Part A: read today (33 rows)

| name | type | tier | unit | joins / parents / read with / bins | consumer | class | reference | params |
|---|---|---|---|---|---|---|---|---|
| `cell.people` | population | cell (every holon, root to street tile) | persons | parents: region holon (1.5 degree root), parent holon | crates/cewp-sim/src/popchain.rs:World::new, split (kernel-wasm pop_load/pop_split; docs/tiling.js tlRoot/tlSplit) -> tiling.rs LeafContext.people -> micro.rs:MicroWorld::populate (the street tier apportions the cohort tier's GHS total (7,830,639,988) over the 355,434 floored 1/8-degree cells; exact at every split (docs/geo/popcond.test.js)) | DERIVED | GHS-POP R2023A (shipped: docs/geo/gd/pop0, pop1) |  |
| `cell.households` | population | cell (every holon, root to street tile) | households | parents: parent holon | crates/cewp-sim/src/popchain.rs:World::new, split -> tiling.rs:HolonTree::budget -> tiling.rs leaf layout (dwelling units) -> micro.rs:MicroWorld::from_leaf, populate (round(people / 2.5) at the root, apportioned by the children's people below it; persons per household is a named gap) | MODEL | none known | `micro.persons_per_household` |
| `cell.density` | attribute | street tile | persons per km2 | of street tile; with cell.settlement_class | crates/cewp-sim/src/micro.rs:DensityClass::of; tiling.rs:place_dense_fabric (storeys) (GHS-POP 1/8-degree mean; the focal world without a leaf (MicroWorld::build) spreads it lognormally by density_sigma) | DERIVED | GHS-POP R2023A (shipped: docs/geo/gd/pop0, pop1) | `micro.density_sigma` |
| `cell.settlement_class` | attribute | street tile | class (village \| suburban \| terraced \| apartment blocks \| dense fabric) | of street tile; with cell.density | crates/cewp-sim/src/micro.rs:DensityClass::of -> tiling.rs leaf layout (street grid, building kind, units per building) (thresholds in docs/geo/classes.json; the class decides dwellings per building, which the focal world turns into households) | MODEL | GHSL settlement classes (R2023A) | `micro.suburban_density` `micro.urban_density` `micro.urban_core_density` |
| `cell.land_cover` | attribute | cell | class (LC100 majority) | of cell | decoration (ground tint only (docs/tiling.js mosaic); no model reads it. In-cell weighting needs a fractional built-up layer (blocked: the shipped asset has one class per cell)) | DERIVED | Copernicus LC100 (shipped: majority class per 1/8 degree) |  |
| `cell.relief` | attribute | cell | m | of cell | decoration (shading only; no model reads elevation or line of sight) | DERIVED | ETOPO 2022, Natural Earth shaded relief (shipped) |  |
| `locality.people` | population | locality (cohort cell, 1.5 degree, at least 20,000 persons) | persons | parents: region | docs/globe.js:buildWorld -> kernel-wasm/src/lib.rs:tick (nodes per locality, lat, lon, metro); crates/cewp-sim/src/social.rs:Geography::from_globe_js; netgraph.rs metros (4,559 cells holding 7,830,639,988 persons (POP_GRID_B64); the scale engine samples nodes by these weights) | DERIVED | GHS-POP R2023A (shipped: docs/geo/gd/pop0, pop1) |  |
| `locality.region` | relationship | locality | membership (one region per locality) | locality -> region | kernel-wasm/src/lib.rs:tick (region_of_metro: cross-region deliveries, region rooms); social.rs placement rules (the page's region rule; the twin has no country layer) | MODEL | Natural Earth admin-0 (approved set; not used for this today) |  |
| `locality.local_hour` | attribute | locality | hours from UTC (solar, lon / 15) | of locality; with act.volume | kernel-wasm/src/lib.rs:tick (globe::local_sec, diurnal_accept); micro.rs:MicroWorld::local_hour (start_local_hour from the page) (day and night by longitude is what spreads acts over the planet) | DERIVED | Natural Earth time zones (approved set; solar time from longitude today) | `micro.start_local_hour` |
| `node_pair.distance_km` | relationship | locality | km (great circle) | node -- node | kernel-wasm/src/lib.rs:tick (out slot 8 km summed; locality and region crossings) (between node positions drawn inside their locality) | DERIVED | derived from positions; no reference needed |  |
| `node.room` | relationship | locality | membership | node -> room | docs/globe.js:buildWorld -> kernel-wasm/src/lib.rs:tick (node_rooms: family, community, region and global rooms) (the scale page draws every family in the signer's locality and community rooms 90% local; act mix 65/30/4/1) | MODEL | none known | `micro.family_share` `micro.community_share` |
| `leaf.streets` | relationship | street tile | frontage (building on street) | building -> street | decoration (a graph since item 6 f: tiling.rs:LeafLayout::street_graph (street id = leaf + road index, junctions cross \| tee \| corner \| exit \| dead end), Building.street; the tiled street drawing takes its end caps from it (docs/tiling.js). No model reads it. A small leaf whose ancestors' lines all pass outside it has no street (Cairo and Paris probe leaves)) | MODEL | STREETREF-1 via CIRISHolon (OpenStreetMap statistics) |  |
| `household.street` | relationship | household | membership (the building's frontage street) | household -> street | decoration (micro.rs:MicroWorld::household_street, households_by_street; summed by street they are the leaf's households (popcond.test.js). No model reads it) | MODEL | STREETREF-1 via CIRISHolon |  |
| `leaf.arterial_crossings` | relationship | street tile | arterials crossing a side | street tile -- street tile | decoration (tiling.rs LeafContext.edges and l2_nb place arterials so neighbouring tiles join; no model reads them. Counted per side of every split from L2 down (tiling::arterial_crossings, kernel pop_crossings) and written to the sums file with side and peer: the fine crossings under a side sum to the coarse one and both sides agree) | MODEL | STREETREF-1 via CIRISHolon |  |
| `building.dwellings` | population | building | dwelling units (one household each) | parents: building, street tile | crates/cewp-sim/src/micro.rs:MicroWorld::populate (households in building order) (units per building follow the class and the leaf's households (tiling.rs). Household.building is stored, not read by any model) | MODEL | GHSL built-up surface and volume; building footprints (Microsoft, CDLA) |  |
| `building.storeys` | attribute | building | storeys | of building; with building.dwellings | decoration (glyph height and shadow length; tiling.rs:place_dense_fabric sets storeys from units; nothing reads them) | MODEL | GHSL building height |  |
| `household.position` | attribute | household | m from the tile centre | of household | decoration (drawing; micro.rs:story_actors picks the Story's four households by distance (presentation only). No model reads neighbour distance or same-street) | MODEL | building footprints (Microsoft, CDLA) |  |
| `household.persons` | population | household | persons | parents: household | crates/cewp-sim/src/micro.rs:MicroWorld::populate (phones and laptops per person; hosted-server chance 2.25 x persons x server_tier_share) (drawn round(2.5 + 1.1 N) clamped 1..6, then the leaf's people apportioned over the households with those weights (popchain::persons_per_household): exact per leaf) | MODEL | none known (the UN household-size table forbids derivatives) | `social.household_size_sd` |
| `household.adopting` | population | household | households running CIRIS | parents: street tile | crates/cewp-sim/src/micro.rs:MicroWorld::populate (adoption is a threshold on the household's own stream, so raising it adds households) | MODEL | none known (fleet count by locality would be the measure) | `micro.adoption` |
| `household.devices` | population | household | devices by kind (phone, laptop, home node, hosted server) | parents: household, person | crates/cewp-sim/src/micro.rs:MicroWorld::populate (device_population, household_devices; R5_MAX_NODES samples it for display) (the leaf's device total is recorded as its device population; the 1,000-node cap is a display sample of it) | MODEL | World Bank indicators mirroring ITU (phones per 100, country grain) | `micro.phones_per_person` `micro.laptops_per_person` `micro.home_node_household_share` `micro.server_tier_share` |
| `household.router_class` | attribute | household | class (public \| punchable \| hard) | of household; with household.access_network, access_link.technology, phone.attach | crates/cewp-sim/src/micro.rs:MicroWorld::unpunched_direct, punch_possible, relay_offering (Node.public, Node.punchable) (a phone off Wi-Fi takes the carrier's class instead (go_out_with)) | FRONTIER | none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5 | `micro.reach_home_share` `micro.public_listener_share` |
| `household.community` | relationship | household | membership | household -> street community room | crates/cewp-sim/src/micro.rs:MicroWorld::community, pick_destination, room_peers (direct-path proposals go to room peers) (half the street's adopting households in one room) | MODEL | none known | `micro.community_join_share` |
| `household.lan` | relationship | household | same home LAN (pair) | device -- device | crates/cewp-sim/src/micro.rs:MicroWorld::unpunched_direct (on_home_lan: same household, phone on Wi-Fi) (the only neighbour relation any model reads; there is no cross-household radio or LAN relation) | MODEL | none known |  |
| `phone.attach` | rhythm | device | phones on each network kind | relationship x 168 bins by phone x network kind (home Wi-Fi \| cellular) | crates/cewp-sim/src/micro.rs:MicroWorld::churn, change_network, on_home_lan (flat today: a stationary share and a mean dwell, no hour of week; becomes attach.phone.network) | MODEL | none known (ATUS time at home is the nearest) | `micro.phone_wifi_share` `micro.wifi_dwell_mean_s` |
| `device.awake` | rhythm | device | devices awake | population x 168 bins by device kind | crates/cewp-sim/src/micro.rs:Node::online, MicroWorld::churn, suspend, resume (flat today: stationary shares and mean dwells) | MODEL | none known | `micro.mobile_offline_share` `micro.phone_asleep_mean_s` `micro.laptop_awake_share` `micro.laptop_dwell_mean_s` |
| `phone.charging` | rhythm | device | phones on external power | population x 168 bins by phone | crates/cewp-sim/src/micro.rs:Node::device_relays, relay_offering (a phone relays only charging and unmetered) (flat today) | MODEL | none known | `micro.phone_charging_share` `micro.charge_dwell_mean_s` |
| `device.relay_preference` | attribute | device | share offering relay | of device; with household.router_class, phone.charging | crates/cewp-sim/src/micro.rs:Node::relay_offering (owners who leave the relay default on) | MODEL | none known (fleet gauge: announced relays per device class) | `micro.relay_offering_share` |
| `device.shaping_opt_in` | attribute | device | share opted in | of device; with household.devices | crates/cewp-sim/src/micro.rs:Node::budget_class, MicroWorld::negotiate (shaping is off by default on home nodes and phones) | MODEL | none known (fleet gauge) | `micro.home_shaping_opt_in` `micro.phone_shaping_opt_in` |
| `device.claimed` | attribute | device | share with an owner | of device; with household.devices | crates/cewp-sim/src/micro.rs:MicroWorld::assign_claims, seat_may_send | MODEL | none known (fleet gauge) | `micro.claimed_share` |
| `device.old_version` | attribute | device | share on an older edge | of device; with household.devices | crates/cewp-sim/src/micro.rs:MicroWorld::punch_possible (long-soak servers kept old on purpose) | MODEL | none known (fleet version census) | `micro.server_old_version_share` |
| `act.volume` | rhythm | device | acts per awake device per bin | population x 168 bins by device kind | crates/cewp-sim/src/micro.rs:MicroWorld::generate (msg_mean_interval_s / diurnal_multiplier); kernel-wasm/src/lib.rs:tick (diurnal_accept); workload.rs:diurnal_multiplier (generator model.diurnal.v1 (was v0): unresolved, no reference; becomes activity.<kind>.events) | MODEL | American Time Use Survey (one country) | `micro.msg_mean_interval_s` `activity.acts_per_person_day` `activity.activity_sigma` |
| `act.scope` | relationship | device | acts per scope (family \| community \| consent-free \| public) | act -> room kind | crates/cewp-sim/src/micro.rs:MicroWorld::pick_destination; kernel-wasm/src/lib.rs:tick (mix_self, mix_room, mix_fed) (the page's act mix 65/30/4/1) | MODEL | none known | `activity.mix_family` `activity.mix_community` `activity.mix_consent_free` `activity.mix_public` |
| `seat.distance_km` | attribute | street tile | km from the tile | of serve seat | decoration (drawn on the chip; no micro model reads a latency from it (netgraph latency is metro to metro)) | MODEL | none needed (a deployment choice) | `micro.seat_km_min` `micro.seat_km_max` |

### Part B: modelled today on invented inputs (43 rows)

| name | type | tier | unit | joins / parents / read with / bins | consumer | class | reference | params |
|---|---|---|---|---|---|---|---|---|
| `household.access_network` | relationship | household | membership (one fixed AS per household) | household -> access network | crates/cewp-sim/src/netgraph.rs:attach_access -> micro.rs Household.access, households_by_access (attached in kernel-wasm micro_build and micro_build_leaf; summed both ways in tests, read by no model yet); failure.rs:DomainWorld::build (the household's second parent: households summed by street tile and by access network agree for every seed (netgraph test, popcond.test.js). Micro's router class is still its own draw; reading the network's NAT behaviour is FSD/twin/nat.md section 8) | MODEL | none known (ISP market shares per metro) | `netgraph.incumbent_weight` `netgraph.local_weight_top` `failure.world.fixed_isps_per_country` `failure.world.isp_zipf` |
| `phone.carrier` | relationship | device | membership | phone -> carrier | crates/cewp-sim/src/netgraph.rs:assign_micro; failure.rs:DomainWorld::build (a phone's third parent) | MODEL | World Bank indicators mirroring ITU (subscriptions, country grain); no market shares | `netgraph.mobile_weight_top` `failure.world.cell_carriers_per_country` |
| `household.carrier_shared` | relationship | household | pairs on one carrier | phone -- phone | crates/cewp-sim/src/reach.rs:World::generate (family plans) | MODEL | none known | `reach.family_same_carrier_share` |
| `access_link.technology` | attribute | access link | class (cable \| fibre \| DSL \| 4G \| 5G) | of access network; with household.access_network, region.ipv4_scarcity | crates/cewp-sim/src/reach.rs:ReachModel::draw_tech, network; reach/topo.rs (drawn per ASN, so one network is one technology) | MODEL | World Bank indicators mirroring ITU (fixed broadband per 100); M-Lab NDT for rates | `reach.fixed_cable_share` `reach.fixed_fibre_share` `reach.cell_5g_share` |
| `access_link.rate` | attribute | access link | bit/s up and down, buffer ms | of access link; with access_link.technology, household.devices, access_link.cross_traffic | crates/cewp-sim/src/media.rs:AccessProfile (FIBER_HOME, CABLE_HOME_WIFI, DSL_HOME, PHONE_HOME_WIFI, PHONE_LTE) -> media_call2.rs scenarios (uplink read jointly with the household's device count and its other traffic) | MODEL | M-Lab NDT (measured access rates; the open source needs a cloud account) |  |
| `access_link.loss` | attribute | access link | Gilbert-Elliott ppm (good/bad transitions and loss) | of access link; with access_link.technology | crates/cewp-sim/src/media.rs:GE_WIRED, GE_WIFI, GE_LTE, CELL_LTE -> link.rs:Link::step, cell_step (Wi-Fi collisions, LTE handover bursts, LTE capacity steps) | MODEL | M-Lab NDT (loss, RTT) |  |
| `access_link.cross_traffic` | attribute | household | transfers per minute and bytes | of household; with access_link.rate, household.devices | crates/cewp-sim/src/media.rs:BG_UP_HOME, BG_DOWN_HOME, BG_UP_PHONE, BG_DOWN_PHONE -> link.rs household transfer (AIMD) (household traffic competing for the uplink) | MODEL | none known |  |
| `access_link.path` | attribute | access link | bytes (path MTU, IP and UDP header) | of access link; with access_link.technology | crates/cewp-sim/src/link.rs:ip_fragments; media.rs (IPv6 would raise the header to 48) | MODEL | none known | `media.path_mtu` `media.ip_udp_header` |
| `region.ipv4_scarcity` | attribute | region | class (rich \| scarce) | of region; with access_link.technology | crates/cewp-sim/src/reach.rs:ReachModel::draw_region, network (drawn once per page region) | MODEL | none known (registry allocation history) | `reach.region_v4_scarce_share` |
| `access_network.cgn` | attribute | access network | share behind CGN; mapping and filtering behaviour | of access network; with access_link.technology, region.ipv4_scarcity | crates/cewp-sim/src/reach.rs:ReachModel::network, direct_path | MODEL | none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5 | `reach.fixed_cgnat_rich` `reach.fixed_cgnat_scarce` `reach.cable_cgnat_mult` `reach.fibre_cgnat_mult` `reach.cgn_eim_share` `reach.cgn_filter_eif` `reach.cgn_filter_adf` `reach.cgn_hairpin_share` |
| `household.router_behaviour` | attribute | household | filtering, hairpin, UPnP/PCP, own router, double NAT | of household router; with access_network.cgn, access_link.technology | crates/cewp-sim/src/reach.rs:ReachModel::home, direct_path (the joint reach model; micro's router class (part A) is its calibrated marginal) | MODEL | none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5 | `reach.router_filter_eif` `reach.router_filter_adf` `reach.router_hairpin_share` `reach.router_upnp_share` `reach.own_router_share` `reach.double_nat_share` |
| `household.reach_marginals` | attribute | household | shares (public, punchable, punch success) | of household router; with household.router_behaviour | crates/cewp-sim/src/reach.rs:ReachModel calibration; failure.rs:DomainWorld::build (the reach and failure packages restate micro's marginals; micro's are on household.router_class) | FRONTIER | none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5 | `reach.reach_home_share` `reach.public_listener_share` `reach.punch_success_share` `micro.punch_success_share` `failure.world.reach_home_share` `failure.world.public_listener_share` |
| `access_network.ipv6` | attribute | access network | share with IPv6; firewall open \| stateful \| blocking | of access network; with access_link.technology, household.router_behaviour | crates/cewp-sim/src/reach.rs:ReachModel::network, home, direct_path | MODEL | none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5 | `reach.fixed_v6_cable` `reach.fixed_v6_fibre` `reach.fixed_v6_dsl` `reach.home_v6_enabled_share` `reach.fixed_v6_fw_open` `reach.fixed_v6_fw_blocked` `reach.cell_v6_4g` `reach.cell_v6_5g` `reach.cell_v6_fw_blocked` |
| `carrier.nat` | attribute | access network | CGN share, filtering, hairpin, punchable share | of carrier; with access_link.technology | crates/cewp-sim/src/reach.rs:ReachModel::phone_on_carrier; micro.rs:MicroWorld::go_out_with | FRONTIER | none known (NAT and IPv6 behaviour: no stated licence on either public source); fleet gauges in FSD/twin/nat.md section 5 | `micro.cellular_punchable_share` `reach.cellular_punchable_share` `reach.cell_cgnat_share` `reach.cell_filter_eif` `reach.cell_filter_adf` `reach.cell_hairpin_share` |
| `phone.attach_marginal` | attribute | device | share on Wi-Fi at an instant | of phone; with phone.attach | crates/cewp-sim/src/reach.rs:World::generate; failure.rs:DomainWorld::build (restated by reach and failure; micro's own is on phone.attach) | MODEL | none known | `reach.phone_wifi_share` `failure.world.phone_wifi_share` |
| `as.population` | population | network | ASes by role (tier-1, content, transit, incumbent, mobile, local fixed) | parents: metro, region | crates/cewp-sim/src/netgraph.rs:generate (stand-in ASNs from the private-use range) | MODEL | RouteViews (AS graph; relationships inferred, not imported) | `netgraph.tier1_count` `netgraph.tier1_region_presence` `netgraph.tier1_pops_per_region` `netgraph.content_count` `netgraph.content_region_presence` `netgraph.content_pops_per_region` `netgraph.transit_per_region` `netgraph.transit_pop_share` `netgraph.incumbent_reach_km` `netgraph.mobile_per_market` `netgraph.local_fixed_base` `netgraph.local_fixed_weight_step` `netgraph.local_fixed_max` |
| `as.upstream` | relationship | network | customer to provider | AS -> AS | crates/cewp-sim/src/netgraph.rs:generate, route | MODEL | RouteViews | `netgraph.content_upstreams` `netgraph.transit_upstreams_max` `netgraph.incumbent_tier1` `netgraph.access_multihome` `netgraph.mobile_central_gateway` |
| `as.peering` | relationship | network | peer links | AS -- AS | crates/cewp-sim/src/netgraph.rs:generate, route | MODEL | RouteViews | `netgraph.transit_peer` `netgraph.ixp_peer` `netgraph.content_peer` `netgraph.restrictive_peering` |
| `ixp.population` | population | network | IXPs | parents: metro | crates/cewp-sim/src/netgraph.rs:generate | MODEL | none known (IXP lists are restricted or unlicensed) | `netgraph.ixp_min_weight` `netgraph.ixp_second_weight` |
| `ixp.membership` | relationship | network | membership | AS -> IXP | crates/cewp-sim/src/netgraph.rs:generate | MODEL | none known | `netgraph.ixp_join_content` `netgraph.ixp_join_transit` `netgraph.ixp_join_access` `netgraph.ixp_join_incumbent` `netgraph.ixp_join_tier1` |
| `cable.links` | relationship | network | cables and backbone links (km) | metro -- metro | crates/cewp-sim/src/netgraph.rs:generate; failure.rs:DomainWorld::build | MODEL | none known (cable maps are restricted or paid) | `netgraph.terrestrial_extra_links` `netgraph.cables_per_region_pair` `netgraph.gateway_candidates` `netgraph.terrestrial_inflation` `netgraph.submarine_inflation` `failure.world.cables_per_country_max` `failure.world.cable_shared_share` |
| `path.latency` | attribute | network | us one way per element | of network path; with access_link.technology, as.upstream, cable.links | crates/cewp-sim/src/netgraph.rs:route; media.rs core delays -> media_call2.rs | MODEL | none known (RIPE Atlas needs approval) | `netgraph.as_hop_us` `netgraph.ixp_crossing_us` `netgraph.access_fixed_us` `netgraph.access_mobile_us` `netgraph.access_content_us` `media.core_same_region` `media.core_cross_region` `media.server_core` |
| `failure.geography` | population | region | countries, metros, grid regions, cloud regions | parents: country | crates/cewp-sim/src/failure.rs:DomainWorld::build (a generated stand-in; no real place) | MODEL | Gridfinder (power regions); Natural Earth admin-0 (countries, approved set) | `failure.world.countries` `failure.world.metros_per_country_max` `failure.world.metro_zipf` `failure.world.power_region_metros_max` `failure.world.cloud_providers` `failure.world.cloud_regions_per_provider` |
| `device.power_region` | relationship | device | membership | device -> power region | crates/cewp-sim/src/failure.rs:DomainWorld::build, evaluate (a home node on a UPS outlasts the evaluation hour) | MODEL | Gridfinder | `failure.world.ups_share` |
| `failure.domain_world_devices` | population | household | devices by kind in the failure world | parents: household, person | crates/cewp-sim/src/failure.rs:DomainWorld::build (the failure package's own household and device draw (its own shares, not micro's)) | MODEL | World Bank indicators mirroring ITU | `failure.world.persons_per_household_max` `failure.world.home_node_share` `failure.world.laptop_share` `failure.world.hosted_share` `failure.world.relay_pref` `failure.world.mobile_offline_share` `failure.world.laptop_awake_share` |
| `failure.room_membership` | relationship | room | rooms per person, size, locality | person -> room | crates/cewp-sim/src/failure.rs:DomainWorld::build, room_losses | MODEL | none known | `failure.world.rooms_per_person` `failure.world.room_size_max` `failure.world.room_locality` |
| `failure.outage` | attribute | region | lognormal duration (h) and reach per kind | of failure domain; with device.power_region, household.access_network, cable.links | crates/cewp-sim/src/failure.rs:sample_event, Event::restore_h (power and network failure domains) | MODEL | anchored on post-incident reports (recalled, not re-read) | `failure.power.median_h` `failure.power.sigma` `failure.power.ramp_share` `failure.power.dc_fail` `failure.isp.median_h` `failure.isp.sigma` `failure.cable.cuts` `failure.cable.median_h` `failure.cable.sigma` `failure.cloud.median_h` `failure.cloud.sigma` `failure.cloud.ramp_share` `failure.dns.median_h` `failure.dns.sigma` `failure.bgp.acceptor_share` `failure.bgp.median_h` `failure.bgp.sigma` `failure.cell.median_h` `failure.cell.sigma` `failure.shutdown.median_h` `failure.shutdown.sigma` `failure.shutdown.mobile_also` |
| `tower.backup_hours` | attribute | access network | h of battery during a grid outage | of cell tower; with device.power_region | crates/cewp-sim/src/failure.rs:evaluate | FRONTIER | none known (tower data is share-alike) | `failure.power.tower_backup_h` |
| `household.kin` | relationship | household | kin room membership (households per room, diaspora) | household -- household | crates/cewp-sim/src/social.rs:SocialWorld::build -> locality.rs (social.persons_per_household restates micro's) | MODEL | none known (UN DESA migrant stock is the kind of source) | `social.persons_per_household` `social.kin_households_mean` `social.diaspora_share` `social.diaspora_abroad_share` |
| `person.work_team` | relationship | person | membership | person -> workplace room | crates/cewp-sim/src/social.rs:SocialWorld::build -> locality.rs | MODEL | none known (census commuting flows are the kind of source) | `social.worker_share` `social.work_team_median` `social.work_team_sigma` `social.work_team_max` `social.work_cross_locality_share` `social.work_cross_region_share` |
| `person.school_class` | relationship | person | membership | person -> school room | crates/cewp-sim/src/social.rs:SocialWorld::build -> locality.rs | MODEL | none known (enrolment and class-size tables) | `social.student_share` `social.class_size` |
| `household.neighbourhood_room` | relationship | household | membership | household -> neighbourhood room | crates/cewp-sim/src/social.rs:SocialWorld::build (the same share as micro's street room) | MODEL | none known | `social.community_join_share` `social.neighbourhood_room_households` |
| `person.interest_group` | relationship | person | membership | person -> interest room | crates/cewp-sim/src/social.rs:SocialWorld::build | FRONTIER | none known | `social.interest_groups_per_person` `social.interest_group_mean_size` `social.interest_local_share` `social.interest_regional_share` `social.interest_pareto_alpha` |
| `person.public_follow` | relationship | person | follows | person -> public room | crates/cewp-sim/src/social.rs:SocialWorld::build | FRONTIER | none known | `social.public_follows_per_person` `social.public_room_mean_size` `social.public_local_share` `social.public_regional_share` `social.public_pareto_alpha` |
| `person.tie` | relationship | person | ties inside a room | person -- person | crates/cewp-sim/src/social.rs:SocialWorld::build -> activity.rs | MODEL | none known | `social.clique_cap` `social.large_group_ties` |
| `act.recipient` | relationship | act | share of a person's acts per room kind | act -> room | crates/cewp-sim/src/activity.rs:Activity::choose (the 90/9/1 locality mix is measured on the result (locality.rs), not set) | FRONTIER | none known | `activity.family_household_share` `activity.community_weight_work` `activity.community_weight_school` `activity.community_weight_neighbourhood` `activity.community_weight_interest` `activity.tie_weight_household` `activity.tie_weight_kin` `activity.tie_weight_work` `activity.tie_weight_school` `activity.tie_weight_neighbourhood` `activity.tie_weight_interest` `activity.tie_weight_public` `activity.tie_weight_consent_free` |
| `presence.work_hours` | rhythm | person | share active | population x 168 bins by room kind (work, school) active | crates/cewp-sim/src/activity.rs:rhythm, Activity::choose (becomes presence.people) | MODEL | American Time Use Survey (one country) | `activity.work_hours_start` `activity.work_hours_end` `activity.off_hours_work_affinity` |
| `act.weekly_shape` | rhythm | person | multiplier | population x 168 bins by day class | crates/cewp-sim/src/activity.rs:rhythm (with model.diurnal.v1 inside each day) | MODEL | American Time Use Survey (one country) | `activity.weekly_weekday` `activity.weekly_saturday` `activity.weekly_sunday` `activity.weekend_shift_h` |
| `act.cascade` | attribute | act | replies per act; reply delay (s) | of act; with act.recipient | crates/cewp-sim/src/activity.rs:Activity::run (Hawkes branching) | MODEL | none known | `activity.branching` `activity.reply_scale_s` `activity.reply_alpha` `activity.reply_max_s` |
| `event.synchronized` | rhythm | region | events and participants | population x 168 bins by event reach (locality \| region \| world) | crates/cewp-sim/src/activity.rs:Activity::events, run (matches, news, a viral post) | FRONTIER | none known | `activity.event_share` `activity.event_share_locality` `activity.event_share_region` `activity.event_share_world` `activity.event_participation_locality` `activity.event_participation_region` `activity.event_participation_world` `activity.event_acts_per_participant` `activity.event_decay_s` `activity.event_mix_family` `activity.event_mix_community` `activity.event_mix_consent_free` `activity.event_mix_public` |
| `device.capability` | attribute | device | us per sign / verify; capture and render delay | of device; with household.devices | crates/cewp-sim/src/media.rs:AccessProfile verify_us, sign_us; media_call2.rs (the device-capability class) | FRONTIER | none known (a measured number is requested) | `media.verify_phone` `media.verify_desktop` `media.sign_phone` `media.sign_desktop` `media.capture_encode` `media.decode_render` |
| `device.crash` | attribute | device | ppm of departures with no signed end | of phone | crates/cewp-sim/src/media.rs (call legs) | MODEL | none known | `media.phone_crash_share` |
| `device.daily_events` | rhythm | device | starts, writes and new devices per device per day | population x 168 bins by device | crates/cewp-sim/src/capacity.rs:CostParams (seat load) (flat per day today) | MODEL | none known (fleet gauges) | `cost.runtime_starts_per_device_day` `cost.pull_plane_writes_per_device_day` `cost.new_rows_per_device_day` `cost.device_starts_per_day` `cost.new_device_share_day` `cost.key_record_writes_per_device_day` `cost.phone_awake_share` `cost.claimed_share` |

### Part C: wanted, not modelled (40 rows)

| name | type | tier | unit | joins / parents / read with / bins | consumer | class | reference | params |
|---|---|---|---|---|---|---|---|---|
| `presence.people` | rhythm | cell | persons per bin | population x 168 bins by place_kind x person_kind; place_kind home.<dwelling class incl. dense fabric> \| work \| school \| venue \| transit \| elsewhere; person_kind working_age \| student_child \| retired \| shift | future: micro.rs churn and the scale engine's signer draw (who is home when) (in every bin people across place kinds sum to the cell's people; emitted per bin with the flows between place kinds, declared source 0) | - | American Time Use Survey (one country) |  |
| `activity.message.direct.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | relationship x 168 bins by place_kind x person_kind | future: per-kind act generator (seam owed) (generator model.diurnal.v1 attached until a reference stands) | - | American Time Use Survey (one country) |  |
| `activity.message.room.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | relationship x 168 bins by place_kind x person_kind | future: per-kind act generator (seam owed) (model.diurnal.v1) | - | none known |  |
| `activity.call.voice.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | relationship x 168 bins by place_kind x person_kind | future: per-kind act generator; media call scenarios (model.diurnal.v1) | - | none known |  |
| `activity.call.video.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | relationship x 168 bins by place_kind x person_kind | future: per-kind act generator; media call scenarios (model.diurnal.v1) | - | none known |  |
| `activity.media.share.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | relationship x 168 bins by place_kind x person_kind | future: per-kind act generator; chunk custody (model.diurnal.v1) | - | none known |  |
| `activity.agent.follow.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | population x 168 bins by place_kind x person_kind | future: per-kind act generator (model.diurnal.v1; one-ended) | - | none known |  |
| `activity.agent.scheduled.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | population x 168 bins by place_kind x person_kind | future: per-kind act generator (model.diurnal.v1; one-ended) | - | none known |  |
| `activity.presence.change.events` | rhythm | cell | events per bin per 1,000,000 persons of the class | population x 168 bins by place_kind x person_kind | future: per-kind act generator (model.diurnal.v1; one-ended) | - | none known |  |
| `activity.message.direct.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator (volume, separate from the shape) | - | none known |  |
| `activity.message.room.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `activity.call.voice.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `activity.call.video.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `activity.media.share.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `activity.agent.follow.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `activity.agent.scheduled.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `activity.presence.change.weekly` | population | person | events per person per week | parents: person | future: per-kind act generator | - | none known |  |
| `attach.phone.network` | rhythm | device | phones per network kind | relationship x 168 bins by place_kind; network kind home_wifi \| work_wifi \| cellular | future: micro.rs churn and change_network (replaces phone.attach) (in every bin a phone is on exactly one; emitted with flows between network kinds, source 0) | - | none known |  |
| `joint.presence_attach_activity` | rhythm | cell | persons | population x 168 bins by place_kind x person_kind x network kind x activity kind | future: micro.rs focal world (a phone's network, place and acts drawn together) (joint table whose marginals are presence.people, attach.phone.network and activity.<kind>.events) | - | none known |  |
| `joint.activity_devices` | population | household | persons by activity kind x device kind | parents: household, person | future: micro.rs populate and generate (joint table whose marginals are household.devices and activity.<kind>.weekly) | - | none known |  |
| `building.radio_neighbourhood` | relationship | building | pairs within Wi-Fi / BLE / LoRa range | device -- device | future: a local-link model for edge's packet-radio and LAN transports (walls and storeys between them) | - | building footprints (Microsoft, CDLA); STREETREF-1 via CIRISHolon |  |
| `building.line_of_sight` | relationship | building | visible pairs (packet radio) | building -- building | future: packet-radio link model | - | ETOPO 2022 (shipped) with GHSL building height |  |
| `building.material_class` | attribute | building | class | of building; with building.radio_neighbourhood | future: indoor radio attenuation (indoor versus outdoor) | - | none known |  |
| `building.units_per_storey` | population | building | dwellings per storey | parents: building | future: vertical neighbours for local links | - | GHSL building height and volume |  |
| `household.nearest_exchange` | relationship | household | membership and km | household -> exchange | future: access latency and shared fate | - | none known (exchanges are restricted or paid) |  |
| `household.nearest_tower` | relationship | household | membership and km | household -> cell tower | future: cellular capacity and shared fate | - | none known (towers are share-alike) |  |
| `tower.backhaul` | attribute | access network | class (fibre \| microwave \| satellite) and resilience | of cell tower; with tower.backup_hours | future: failure.rs tower domain | - | none known |  |
| `metro.nearest_ixp` | relationship | network | membership and km | metro -> IXP | future: netgraph.rs route | - | none known |  |
| `person.commute` | rhythm | cell | persons moving | relationship x 168 bins by home cell x work cell | future: micro.rs mobility (phones changing networks and neighbours through the day) (row sums are the origin's residents) | - | none known (census commuting flows) |  |
| `gathering.population` | population | cell | gathering places by kind (school, workplace, market, venue, transit) | parents: cell | future: presence.people place kinds; local links where devices meet | - | STREETREF-1 via CIRISHolon (OpenStreetMap statistics) |  |
| `person.language` | attribute | person | language | of person; with locality.local_hour | future: call scheduling, room membership | - | none known |  |
| `locality.time_zone` | attribute | locality | zone offset and DST | of locality | future: every rhythm's bin 0 (today round(lon / 15)) | - | Natural Earth time zones (approved set) |  |
| `region.week_class` | attribute | region | mon-fri \| sun-thu \| sat-wed | of region | future: every rhythm row's week coordinate | - | none known |  |
| `household.capability_class` | attribute | household | class (income / device capability) | of household; with household.devices, access_link.rate | future: codec and uplink choice in media | - | none known |  |
| `device.energy` | attribute | device | battery behaviour and energy cost per class | of device; with household.devices, phone.charging | future: reach::hooks energy counters | - | none known |  |
| `region.seasonal_load` | rhythm | region | multiplier | population x 53 bins by ISO week | future: workload volume (seasonal and event rhythms) | - | none known |  |
| `region.disaster_exposure` | attribute | region | hazard class | of region; with failure.outage | future: failure.rs event library | - | none known |  |
| `region.censorship_exposure` | attribute | region | class | of region; with failure.outage | future: failure.rs shutdown scenario | - | none known |  |
| `settlement.thin_uplink` | attribute | street tile | villages behind one thin uplink | of street tile; with cell.settlement_class, access_link.rate | future: the 45.1 village case (one shared uplink) (rural settlement pattern) | - | GHSL settlement classes; M-Lab NDT |  |
| `household.power_region` | relationship | building | membership | building -> power region | future: failure.rs, the building's second parent (the building's second parent) | - | Gridfinder |  |

### Parameters that do not describe the world

| parameter | why |
|---|---|
| `micro.homing_k` | protocol: top-k homing |
| `micro.bundle_bootstrap` | protocol: edge #881 switch |
| `micro.ci_arrivals_per_hour` | deployment: the project's own CI fleet |
| `micro.ci_job_mean_s` | deployment: the project's own CI fleet |
| `micro.ci_old_version_share` | deployment: the project's own CI fleet |
| `micro.ci_msg_interval_s` | deployment: the project's own CI fleet |
| `micro.lan_autointerface` | host configuration: LAN discovery on |
| `micro.target_households` | display: the patch is sized for the R5 visual budget |
| `failure.world.nodes` | evaluation: world size |
| `failure.world.seats` | deployment: serve seats |
| `failure.eval.hour` | evaluation |
| `failure.eval.rejoin_share` | evaluation |
| `failure.eval.peer_cache_share` | evaluation |
| `failure.withhold.relay_share` | adversary model, not a scene |
| `failure.withhold.median_h` | adversary model, not a scene |
| `failure.withhold.sigma` | adversary model, not a scene |
| `media.setup_rtts` | protocol |
| `media.audio_bitrate` | host codec configuration |
| `media.audio_frame` | host codec configuration |
| `media.video_bitrate` | host codec configuration |
| `media.video_fps` | host codec configuration |
| `media.temporal_layers` | host codec configuration |
| `media.gop` | host codec configuration |
| `media.keyframe_ratio` | host codec configuration |
| `media.t1_ratio` | host codec configuration |
| `media.frame_size_jitter` | host codec configuration |
<!-- couplings:end -->
