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

## 5. Presence (seam)

`trait Presence` gives awake, at home and on Wi-Fi for 168 local hour-of-week bins (bin 0 =
Monday 00:00, as `docs/geo/couplings.json`). `StandInPresence` (MODEL, unresolved) derives awake
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
* (c): `presence.people`, `attach.phone.network`, `joint.presence_attach_activity`,
  `locality.time_zone`, `region.week_class`, `person.commute`.

Every measure returns a `Verdict` (class, rows, note). Today (a) is FRONTIER: "computed from a
generated, uncertified tie table", and the note names the FRONTIER placement rows. (b) is
FRONTIER (netgraph stand-in, reach CGNAT), and (c) is FRONTIER (stand-in presence on top of
(a) and (b)).

## 7. Results

RESULTS

## 8. Validity envelope

* Every input that places a room is MODEL or FRONTIER. The sweep is the result: the headline is
  a function of those inputs, not a property of the mesh.
* The twin locality is ~16 grid cells, a large sub-national area. "Local" at that grain says
  little about network distance, which is why (b) exists.
* No country layer, no building or tile layer: stated groupings (section 3).
* (b) uses the netgraph stand-in (MODEL), with no real AS, IXP or cable data. The trombone
  share is the stand-in's.
* (c) uses a stand-in presence (MODEL). Phones away from home are on Wi-Fi a fifth of the time
  by assumption.
* Equal bytes per delivery: a call or a media share would weigh differently.
* Consent-free rows (to seats) are not a group and are left out of all three measures.
