# Twin package `theory`, part 3: three localities, never merged

Code: `crates/cewp-sim/src/locality2.rs`, built on `social.rs` (the society), `activity.rs` (who
acts when), `netgraph.rs` (AS paths) and `reach/hooks.rs` (relayed pairs). CLI:
`cewp-cli locality [--persons N] [--days D] [--seed S] [--geo docs/globe.js|stand-in] [--sweep]`,
run from the repo root. View: `view_localities(seed)`.

## 1. Why

README §2 claimed "~92% of signed acts never leave the signer's locality". That figure is
`0.65 + 0.30 × 0.90`, arithmetic on hand-set inputs: families 100% local, community rooms
90/9/1 (`docs/globe.js` `buildWorld`), and an act mix of 65/30/4/1. It counts only where
recipients sit. By delivery it is 0.58 at 100 k nodes and 0.21 at 1 M (MODEL: computed by the page's engine on its generated world, people.md). It reads no byte path and no presence. Part 1 relabelled the claim as an
input. This module measures three different things that the one word "local" had been carrying.

* **(a) Audience locality**: where the recipients of an act sit, by act and by delivery, per
  kind of group, in nested member-distance bands.
* **(b) Path locality**: where the delivered bytes go: the highest network segment a delivery
  reaches, the relayed share, and where the relay sits.
* **(c) Presence-weighted locality**: (a) and (b) weighted by who is awake, at home and on which
  network at the hour of the act.

The three are reported side by side. None is the headline of another, and each carries the
verdict of its inputs.

## 2. Kinds of group

social.rs's seven room kinds (Household, Kin, Work, School, Neighbourhood, Interest, Public)
come with their placement unchanged. Their acts come from `activity::Generator` (the Poisson law,
today's). Three kinds are added here, all with FRONTIER parameters (`LOC2_SPECS`):

| kind | members | acts |
|---|---|---|
| close friends | `close_friends.per_person` (4) drawn by band: tile 0.30, locality 0.35, country 0.15, abroad in the world region 0.10, elsewhere 0.10 | `close_friends.acts_per_person_day` (15) |
| own devices | the person's other devices (1.25, from 11.25 B devices over 5 B people, less the phone), all in the household | 24 / day |
| own agents | `own_agents.per_person` (1): hosted 0.6 (host in country 0.5, world region 0.3, elsewhere 0.2), else on the home node | 10 / day |

The added kinds' acts follow the diurnal shape as expected counts per local hour. Only the
friend draw is random.

## 3. Bands (audience)

Nested: **household ⊂ building ⊂ tile ~1 km ⊂ locality ⊂ country ⊂ abroad in the same world
region ⊂ abroad elsewhere**. The twin has no building, tile or country layer, so each is a stated
grouping:

* building: households of one grid cell, in index order, chunked by
  `band.households_per_building` (4, FRONTIER; `building.dwellings` replaces it);
* tile: the same chunked by `band.households_per_tile` (400, FRONTIER; `cell.households` from
  GHS-POP replaces it);
* country: localities of one world region chunked in index order by
  `band.localities_per_country` (3, MODEL).

The brief's order put "same region" between locality and country. A twin region is one of the
nine GSMA/UN world regions, larger than a country, so it nests outside: "abroad, same world
region". The building and tile bands are only as meaningful as social.rs's placement inside a
locality. It places neighbourhood and school rooms "neighbours first" in cell order, and other
rooms at random within the locality. At a twin locality of ~16 grid cells, the inner bands are
thin for every kind except household and own devices.

## 4. Path (bytes)

`PathModel::build` reads, once per world:

* same-locality cross-network paths from netgraph, 6 routed pairs of fixed ASes per metro: the
  share that stays in the metro, crosses an IXP, or leaves the metro ("trombones");
* the one-network share (sum of market shares squared): the chance two fixed lines in one metro
  share an ISP;
* cross-locality paths from `sample_fixed_pairs`: same country (same region, under 1,500 km),
  same world region, other region;
* the path to the nearest data centre (hosted relays and hosted agents);
* relayed share of pairs by class (reach hooks) and the regime's relay offering share.

A delivery's segment depends on its band and on whether each end is at home on Wi-Fi. A
household delivery is on the LAN only if both ends are home. A relayed delivery is at least
metro (a neighbour's node) or at least the data-centre path (a hosted relay). Bytes per
delivery are equal across kinds (no size model), so the byte share is the delivery share.

## 4b. Against the status quo

Path locality is stated as a difference from the status-quo path for the same deliveries. In
the status quo every delivery goes up to the platform's data centre and down again (the
netgraph path to the nearest content network). Public-room deliveries are served from a CDN
inside the ISP 21% of the time (Arcep 2024, 14 to 48% by ISP, via CIRISHolon;
`baseline.cdn_inside_isp_share`). Arcep also reports public IXP peering at 1.5% of inbound. The
stand-in's data-centre paths cross an IXP far more often, so the IXP column is the stand-in's
and overstates that segment.

## 4c. The reference at district grain (CIRISHolon TIES-0, Germany)

TIES-0 is one count of friendship links between German districts on one platform. Per band:
own district 0.35, within 100 km 0.27, 100 to 300 km 0.13, farther in Germany 0.10, foreign
Europe 0.15. By density fifth, the sparsest has 0.44 own and 0.09 foreign, and the densest has
0.23 own and 0.23 foreign. The report sets the twin's acquaintance and kin ties beside it at the
twin's own grain. It says plainly that these are not the same quantity, and it uses TIES-0's
0.75-within-300-km only as an outer envelope at locality grain. Administrative membership is
its own key, and no single distance exponent holds, so no distance kernel is used. Ties are
not messages: the tie-to-message weighting is a separate labelled input (`tie_weight`, k = 0
flat, k = 1 strong ties talk more, MODEL), and the comparison is shown under both. Below a
district TIES-0 gives no licence (own-district links within 1 km anywhere in 0.4% to 48.2%
for the median district). The sub-kilometre placements (`band.households_per_building`,
`band.households_per_tile`, `close_friends.tile_share`) stay FRONTIER and are swept over that
whole width.

## 5. Presence (seam)

`trait Presence` gives awake, at home and on Wi-Fi for 168 local hour-of-week bins (bin 0 =
Monday 00:00, as `docs/geo/couplings.json`). `Presence0Presence` reads CIRISHolon PRESENCE-0 (US time diaries 2022-2024, ages 15+,
`presence0.rs`, pinned) as `presence.people`. The table gives awake and at-home shares by day
type and clock hour, and it refuses the twin's awake share (0.623 against 0.85). Wi-Fi at a
workplace or school (0.8) and elsewhere (0.2) are FRONTIER inputs, because the table has no
networks. The report shows (c) with the stand-in and with PRESENCE-0 side by side. Outside the
US, PRESENCE-0 is a stand-in for another country's week. `StandInPresence` (MODEL, unresolved)
derives awake
from the twin's diurnal curve (`model.diurnal.v1`), and at home and on Wi-Fi from a stated
weekday/weekend day shape. CIRISHolon's `presence.people` (awake, place) and
`attach.phone.network` (Wi-Fi vs carrier) rows replace it through this seam, one bin per row.
(c) weights each delivery by the recipient being awake at the act's local hour. It prices the
path with that hour's home and Wi-Fi shares.

## 6. Registry rows each measure will read (CIRISHolon part A grows by these)

* (a): `household.kin`, `person.work_team`, `person.school_class`,
  `household.neighbourhood_room`, `person.interest_group`, `person.public_follow`, `person.tie`,
  `act.recipient`, `act.scope`, `cell.households`, `building.dwellings`, `locality.region`.
* (b): `household.access_network`, `phone.carrier`, `as.population`, `as.upstream`,
  `as.peering`, `ixp.membership`, `metro.nearest_ixp`, `cable.links`,
  `household.reach_marginals`, `household.lan`.
* TIES-0 at district grain (a): `person.tie` and a new `district_pair.ties` row.
* (c): `presence.people`, `attach.phone.network`, `joint.presence_attach_activity`,
  `locality.time_zone`, `region.week_class`, `person.commute`.

Every measure returns a `Verdict` (class, rows, note). Today (a) is FRONTIER: "computed from a
generated, uncertified tie table", and the note names the FRONTIER placement rows. (b) is
FRONTIER (netgraph stand-in, reach CGNAT), and (c) is FRONTIER (stand-in presence on top of
(a) and (b)).

## 7. Results

Run: `cargo run --release -p cewp-cli -- locality --persons 5000 --days 1 --seed 1 --sweep`
(rc 0) on the page's population grid. Header: Three localities (5000 persons, 1 day(s), seed 1, geography: the page's population grid (docs/globe.js)).

**Headline range (the sweep, all MODEL/FRONTIER inputs):** act locality at the locality band
(the page's definition, social.rs's seven kinds) runs **65.8% to 83.5%** over the
room-placement sweep (74.3% at the defaults). Delivery locality runs **29.5% to 88.5%** (51.0%
at the defaults). The placeless null gives 32.5% / 4.0%. Over all ten kinds the act locality
runs 61.3% to 78.9% and the delivery locality 30.6% to 88.0%. The share of delivered bytes that
leave the metro runs 63.7% to 86.8% (78.2% at the defaults, 97.7% placeless). Against the status
quo (every delivery up to a platform data centre and down; public rooms served from a CDN
inside the ISP 21% of the time), all kinds together keep +5.5 pp more bytes in one network or on
the LAN and send 13.0 pp fewer out of the metro. Public rooms go the other way (+20.7 pp leave
the metro, because the status quo's CDN is closer). PRESENCE-0 against the stand-in moves the
presence-weighted figures by at most a few points: all kinds leave the metro 76.8% against
79.3%.

**At district grain against TIES-0 (Germany), not the same quantity:** 74.9% of TIES-0 links lie
within 300 km (61.5% in the densest fifth, 82.2% in the sparsest). The twin's acquaintance and
kin ties at its own grain put 30.4% within 300 km when weighted flat, and 38.9% under "strong
ties talk more" (k = 1, MODEL). 57.6% / 52.9% lie farther in the same world region, against
TIES-0's 10%. So the twin's ties are far less local than the TIES-0 outer envelope. Part of
that is the twin's grain: its "region" is a continent, and a grid cell is about 100 km across,
so the "within 100 km" band is nearly empty. Part is the FRONTIER placement.

**Below a district:** the building and tile sweeps move nothing. At this world size a grid cell
holds fewer households than one building or tile chunk, and TIES-0 gives no licence below a
district (own-district links within 1 km anywhere in 0.4% to 48.2%). **The model cannot state a
sub-district locality yet.**

### (a) Audience locality: where recipients sit

Verdict: **FRONTIER**. computed from a generated, uncertified tie table: room placement rows interest_groups_per_person, interest_local_share, interest_regional_share, public_follows_per_person, public_local_share, public_regional_share are FRONTIER, and close friends, own agents and the building and tile bands are FRONTIER here; no CIRISHolon tie row is certified yet. Will read: household.kin, person.work_team, person.school_class, household.neighbourhood_room, person.interest_group, person.public_follow, person.tie, act.recipient, act.scope, cell.households, building.dwellings, locality.region.

Share of deliveries per band, and act locality (every recipient within the band) at the locality band.

| kind | acts | deliveries | household | building | tile ~1 km | locality | country | abroad, same world region | abroad, other world region | acts all within locality |
|---|---|---|---|---|---|---|---|---|---|---|
| household | 281412 | 610613 | 100.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | 100.0% |
| kin | 309395 | 3158969 | 18.4% | 3.8% | 1.7% | 60.3% | 1.2% | 9.9% | 4.8% | 64.2% |
| work | 30929 | 396185 | 5.4% | 9.9% | 8.2% | 68.3% | 1.3% | 5.4% | 1.6% | 68.6% |
| school | 12590 | 161175 | 3.2% | 4.9% | 6.5% | 85.5% | 0.0% | 0.0% | 0.0% | 100.0% |
| neighbourhood | 36602 | 1356032 | 5.5% | 4.5% | 4.4% | 85.6% | 0.0% | 0.0% | 0.0% | 100.0% |
| interest | 183061 | 5847456 | 1.3% | 1.8% | 1.6% | 35.9% | 5.9% | 32.5% | 20.9% | 47.8% |
| public | 8526 | 3918326 | 0.2% | 0.3% | 0.3% | 8.2% | 3.9% | 20.7% | 66.5% | 36.2% |
| close friends | 75000 | 300000 | 16.7% | 12.2% | 5.2% | 36.6% | 8.5% | 9.5% | 11.3% | 26.8% |
| own devices | 120000 | 150000 | 100.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | 100.0% |
| own agents | 50000 | 50000 | 40.0% | 0.0% | 0.0% | 0.0% | 30.0% | 18.0% | 12.0% | 0.0% |
| all kinds | 1107515 | 15948756 | 10.0% | 2.4% | 1.8% | 37.6% | 3.7% | 19.3% | 25.2% | 70.5% |

### (b) Path locality: where the bytes go

Verdict: **FRONTIER**. netgraph stand-in AS graph (MODEL) and reach outcomes (MODEL marginals, FRONTIER CGNAT and carrier punching); no country layer, so 'same country' is a same-region pair under 1,500 km; equal bytes per delivery. Will read: household.access_network, phone.carrier, as.population, as.upstream, as.peering, ixp.membership, metro.nearest_ixp, cable.links, household.reach_marginals, household.lan.

Path model: same-locality cross-network segments [0.0, 0.0, 0.238, 0.241, 0.521], one-network share 0.36; cross-locality (same country / same world region / other) [[0.0, 0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 0.0, 1.0]]; relayed share of pairs fixed / Wi-Fi / cellular [0.254, 0.213, 0.517]; path to the nearest data centre [0.0, 0.0, 0.039, 0.11, 0.851]; relay offering 0.90 (expected regime). Path locality is stated against the status quo (every delivery up to the platform's data centre and down, public rooms served from a CDN inside the ISP 21% of the time, Arcep 2024); Arcep also reports public IXP peering at 1.5% of inbound, far below the stand-in's IXP share on data-centre paths, so the IXP column is the stand-in's.

| kind | home LAN | one access network | metro (cross-network) | IXP | long-haul | relayed | relayed by a neighbour | status quo: in one network / leaves the metro | mesh minus status quo: in one network or LAN / leaves the metro |
|---|---|---|---|---|---|---|---|---|---|
| household | 47.7% | 0.0% | 12.1% | 12.3% | 27.9% | 15.4% | 90.0% | 0.0% / 96.1% | +47.7 pp / -55.9 pp |
| kin | 8.8% | 8.9% | 17.9% | 14.8% | 49.6% | 26.9% | 90.0% | 0.0% / 96.1% | +17.7 pp / -31.7 pp |
| work | 2.6% | 11.7% | 21.2% | 17.2% | 47.4% | 28.8% | 90.0% | 0.0% / 96.1% | +14.3 pp / -31.6 pp |
| school | 1.5% | 13.1% | 23.5% | 18.9% | 43.0% | 29.1% | 90.0% | 0.0% / 96.1% | +14.6 pp / -34.2 pp |
| neighbourhood | 2.6% | 12.8% | 23.2% | 18.7% | 42.6% | 28.8% | 90.0% | 0.0% / 96.1% | +15.4 pp / -34.8 pp |
| interest | 0.6% | 5.3% | 9.5% | 7.7% | 76.8% | 29.4% | 90.0% | 0.0% / 96.1% | +5.9 pp / -11.6 pp |
| public | 0.1% | 1.2% | 2.1% | 1.7% | 94.9% | 29.5% | 90.0% | 21.0% / 75.9% | -19.7 pp / +20.7 pp |
| close friends | 8.0% | 7.3% | 14.9% | 12.4% | 57.5% | 27.2% | 90.0% | 0.0% / 96.1% | +15.3 pp / -26.3 pp |
| own devices | 69.1% | 0.0% | 7.2% | 7.3% | 16.5% | 9.1% | 90.0% | 0.0% / 96.1% | +69.1 pp / -72.4 pp |
| own agents | 27.6% | 0.0% | 14.7% | 6.6% | 51.0% | 0.0% | 0.0% | 0.0% / 96.1% | +27.6 pp / -38.5 pp |
| all kinds | 5.0% | 5.7% | 11.2% | 9.2% | 69.0% | 28.0% | 90.0% | 5.2% / 91.2% | +5.5 pp / -13.0 pp |

### (c) Presence-weighted locality

Verdict: **FRONTIER**. stand-in presence (diurnal model.diurnal.v1, stated day shape): unresolved, no reference; CIRISHolon presence.people and attach.phone.network replace it; on top of (a) and (b). Will read: presence.people, attach.phone.network, joint.presence_attach_activity, locality.time_zone, region.week_class, person.commute.

Beside it, PRESENCE-0: verdict **FRONTIER**. PRESENCE-0 as presence.people (US time diaries 2022-2024, ages 15+; awake 0.623 of the week, the twin's 0.85 refused); outside the US it is a stand-in of another country's week; Wi-Fi at work 0.8 and elsewhere 0.2 are FRONTIER; nothing for under-15s or for whether anyone is home in a multi-person household; on top of (a) and (b).

Deliveries weighted by the recipient being awake at the act's local hour; the path by who is at home and on Wi-Fi at that hour. (b) uses the presence's daily means; (c) the hour's.

| kind | within locality (a) | within locality (c, stand-in) | within locality (c, PRESENCE-0) | home LAN (b, stand-in) | home LAN (c, stand-in) | home LAN (c, PRESENCE-0) | leaves the metro (b, stand-in) | leaves the metro (c, stand-in) | leaves the metro (c, PRESENCE-0) |
|---|---|---|---|---|---|---|---|---|---|
| household | 100.0% | 100.0% | 100.0% | 47.7% | 42.5% | 44.2% | 40.2% | 44.2% | 42.8% |
| kin | 84.1% | 84.2% | 84.1% | 8.8% | 7.8% | 8.1% | 64.4% | 66.1% | 62.3% |
| work | 91.7% | 91.7% | 91.8% | 2.6% | 1.8% | 2.0% | 64.5% | 68.3% | 62.3% |
| school | 100.0% | 100.0% | 100.0% | 1.5% | 1.1% | 1.2% | 61.9% | 65.7% | 59.2% |
| neighbourhood | 100.0% | 100.0% | 100.0% | 2.6% | 2.5% | 2.6% | 61.4% | 62.3% | 57.4% |
| interest | 40.6% | 40.6% | 40.6% | 0.6% | 0.6% | 0.6% | 84.5% | 85.0% | 82.9% |
| public | 9.0% | 8.9% | 9.0% | 0.1% | 0.1% | 0.1% | 96.6% | 96.8% | 96.3% |
| close friends | 70.6% | 70.6% | 70.6% | 8.0% | 7.2% | 7.4% | 69.9% | 71.3% | 68.2% |
| own devices | 100.0% | 100.0% | 100.0% | 69.1% | 62.7% | 64.7% | 23.8% | 28.7% | 27.1% |
| own agents | 40.0% | 40.0% | 40.0% | 27.6% | 25.1% | 25.9% | 57.7% | 57.7% | 57.7% |
| all kinds | 51.8% | 51.6% | 51.8% | 5.0% | 4.4% | 4.7% | 78.2% | 79.3% | 76.8% |

### (a) at district grain: the reference, CIRISHolon TIES-0 (Germany)

CIRISHolon 8477c00a7d1f4d92f086a44ade4d1c8889109932 conformance/scene/tables/ties0_de/ties_table.json v1 sha256 3ca880e83577204bfd9ede906381b4307f1bb980bed7b16dac0289f67d77a06f. One undifferentiated count of friendship links between German districts on one platform: not kinds of tie, not households, not messages; links beyond Europe are not in the file. The twin's column is its acquaintance and kin ties (member pairs of kin, work, school, neighbourhood and interest rooms) in the same bands at its own grain (own area = one grid cell; 'farther' = over 300 km in the same world region; 'foreign' = another world region). **Not the same quantity**: TIES-0 is links on one platform in one country; the twin is room memberships drawn from FRONTIER placement inputs (the page's 'community rooms 90% local', social.rs diaspora 0.12, interest groups 0.5 in the signer's locality). TIES-0's 0.75 within 300 km is an outer envelope for acquaintance and kin links at the twin's locality grain (a locality's half-width is of the order of 300 km), not a target for message locality.

| ties | own area | within 100 km | 100 to 300 km | farther, same country or region | foreign | within 300 km |
|---|---|---|---|---|---|---|
| TIES-0, all districts (population-weighted) | 34.6% | 27.4% | 12.9% | 10.0% | 15.2% | 74.9% |
| TIES-0, sparsest fifth | 43.5% | 25.8% | 12.9% | 8.7% | 9.0% | 82.2% |
| TIES-0, densest fifth | 23.0% | 24.3% | 14.2% | 15.2% | 23.3% | 61.5% |
| twin, ties weighted flat (k = 0) | 13.6% | 0.1% | 16.7% | 57.6% | 12.0% | 30.4% |
| twin, strong ties talk more (k = 1, MODEL) | 20.6% | 0.2% | 18.1% | 52.9% | 8.2% | 38.9% |

Below a district TIES-0 gives no licence: the share of own-district links within 1 km lies anywhere in [0.4%, 48.2%] for the median district (a bracket, not a measurement). The household, building, tile and close-friends placements stay FRONTIER and are swept below; a distance law carried down one scale failed at Holon (2.8% of the population within 0.05, against 53.5% for a density-fifth mean), so no kernel is used.

### Sweep: the headline as a function of the room-placement inputs

Act and delivery locality at the locality band (the page's definition) over social.rs's seven kinds, then over all ten, and the share of delivered bytes that leave the metro. Inputs one at a time from the defaults, then jointly, then the placeless null.

Across the sub-kilometre FRONTIER rows the delivery share within the tile runs 13.7% to 14.1% (the building and tile inputs do not move it at all: at this world size a grid cell holds fewer households than one building or tile chunk, so those bands are not resolved and what lies 'within the tile' is the household and same-cell placement. **The model cannot state a sub-district locality yet**; it needs a world placed at street grain (the micro tier) and a sub-district tie reference, which TIES-0 says does not exist); delivery locality at the locality band runs 51.8% to 51.8% over the same rows.

| setting | act locality (7 kinds) | delivery locality (7) | act locality (10) | delivery locality (10) | delivery within the tile (10) | bytes leaving the metro |
|---|---|---|---|---|---|---|
| defaults | 74.3% | 51.0% | 70.5% | 51.8% | 14.1% | 78.2% |
| interest_local_share = 0.1 | 66.0% | 31.5% | 64.1% | 32.6% | 10.2% | 85.9% |
| interest_local_share = 0.3 | 70.1% | 37.5% | 67.2% | 38.5% | 11.2% | 83.6% |
| interest_local_share = 0.7 | 78.5% | 60.8% | 73.8% | 61.3% | 15.7% | 74.4% |
| interest_local_share = 0.9 | 83.0% | 68.6% | 77.3% | 68.8% | 16.2% | 71.6% |
| public_local_share = 0.1 | 74.1% | 45.0% | 70.3% | 45.9% | 12.7% | 80.6% |
| public_local_share = 0.7 | 74.6% | 63.2% | 70.8% | 63.7% | 17.2% | 73.2% |
| diaspora_share = 0.02 | 85.1% | 54.0% | 78.9% | 54.7% | 14.6% | 77.1% |
| diaspora_share = 0.3 | 62.5% | 47.2% | 61.3% | 48.1% | 13.9% | 79.6% |
| work_cross_locality_share = 0 | 75.5% | 51.2% | 71.4% | 52.0% | 14.1% | 78.1% |
| work_cross_locality_share = 0.2 | 73.2% | 50.4% | 69.6% | 51.2% | 14.0% | 78.4% |
| public_follows_per_person = 0 | 74.7% | 65.2% | 70.8% | 65.6% | 18.5% | 72.2% |
| public_follows_per_person = 10 | 74.2% | 41.0% | 70.4% | 41.9% | 11.3% | 82.4% |
| joint: interest and public local share = 0.1 | 65.8% | 29.5% | 63.9% | 30.6% | 9.4% | 86.8% |
| joint: interest and public local share = 0.5 | 74.4% | 52.5% | 70.6% | 53.3% | 14.5% | 77.6% |
| joint: interest and public local share = 0.9 | 83.5% | 88.5% | 77.7% | 88.0% | 20.6% | 63.7% |
| close friends 30% local (tile + locality) | 74.3% | 51.0% | 68.9% | 51.2% | 13.8% | 78.5% |
| band.households_per_building = 1 (sub-kilometre, FRONTIER) | 74.3% | 51.0% | 70.5% | 51.8% | 14.1% | 78.2% |
| band.households_per_building = 20 (sub-kilometre, FRONTIER) | 74.3% | 51.0% | 70.5% | 51.8% | 14.1% | 78.2% |
| band.households_per_tile = 100 (sub-kilometre, FRONTIER) | 74.3% | 51.0% | 70.5% | 51.8% | 14.1% | 78.2% |
| band.households_per_tile = 2000 (sub-kilometre, FRONTIER) | 74.3% | 51.0% | 70.5% | 51.8% | 14.1% | 78.2% |
| close_friends.tile_share = 0.0013 (TIES-0 below-district bracket, FRONTIER) | 74.3% | 51.0% | 70.7% | 51.8% | 13.7% | 78.2% |
| close_friends.tile_share = 0.1688 (TIES-0 below-district bracket, FRONTIER) | 74.3% | 51.0% | 70.6% | 51.8% | 14.0% | 78.2% |
| placeless null model (every non-household room ignores place) | 32.5% | 4.0% | 37.9% | 4.6% | 2.5% | 97.7% |


## 8. Validity envelope

* Every input that places a room is MODEL or FRONTIER. The sweep is the result: the headline is
  a function of those inputs, not a property of the mesh.
* The twin locality is ~16 grid cells, a large sub-national area. "Local" at that grain says
  little about network distance, which is why (b) exists.
* No country layer, no building or tile layer: stated groupings (section 3). At the page's grid
  and 5,000 persons a cell holds fewer households than a building or tile chunk, so the
  sub-kilometre bands are not resolved. The model cannot state a sub-district locality yet.
* TIES-0 is Germany, one platform's links. The twin's comparison uses continents as "region" and
  ~100 km cells as "own area", so the band comparison is an envelope check, not a fit.
* (b) uses the netgraph stand-in (MODEL), with no real AS, IXP or cable data. The trombone
  share is the stand-in's.
* (c) uses a stand-in presence (MODEL). Phones away from home are on Wi-Fi a fifth of the time
  by assumption.
* Equal bytes per delivery: a call or a media share would weigh differently.
* Consent-free rows (to seats) are not a group and are left out of all three measures.
