# Twin package `theory`, part 2: the energy delta from the status quo

Code: `crates/cewp-sim/src/energy.rs` and `crates/cewp-sim/src/presence0.rs`. CLI:
`cewp-cli energy [seed]`. Views: `view_delta`, `view_delta_class`, `view_break_even` and
`view_phone_wh` (layouts are in their doc comments and in section 9).

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
only and 7.1% no internet. The envelope is the United States, 2020. Presence rows come from
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
| `datacentre.messaging.wh_per_person_hour` | data centre | messaging and social workload one person drives, per hour of the day | Wh | 0.02 / 0.1 / 0.5 | FRONTIER | n/a | 2026 | - | POWER-0 not_available: no published figure for messaging alone; row 43 (415 TWh, every workload) is not this | a stated bound: a few Wh per user-day for a whole platform, of which messaging is an unknown share |
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
| home node on existing gateway | 2197 [-4948, 14819] open (point > 0) | 1627 [-4948, 14819] open (point > 0) | 1658 [-4948, 14819] open (point > 0) |
| home node on computer already on | 3637 [-4588, 20219] open (point > 0) | 3067 [-4588, 20219] open (point > 0) | 3098 [-4588, 20219] open (point > 0) |
| home node on a new always-on box | 22357 [6572, 58991] cost (interval > 0) | 21787 [6572, 58991] cost (interval > 0) | 21818 [6572, 58991] cost (interval > 0) |
| phones only (home with a fixed line) | 540 [-5330, 8324] open (point > 0) | -30.7 [-5330, 8324] open (point < 0) | 0.152 [-5330, 8324] open (point > 0) |
| phones only (cellular-only home) | 2101 [-5239, 11926] open (point > 0) | 2101 [-5239, 11926] open (point > 0) | 1430 [-5239, 11926] open (point > 0) |
| new box on cellular (cellular-only home) | 29585 [10177, 73803] cost (interval > 0) | 29021 [10177, 73803] cost (interval > 0) | 29050 [10177, 73803] cost (interval > 0) |
| village, one thin uplink | 263 [-7463, 6864] open (point > 0) | 263 [-7463, 6864] open (point > 0) | 261 [-7463, 6864] open (point > 0) |
| US 2020 household mix, node on gateway | 2018 [-4621, 13259] open (point > 0) | 1578 [-4621, 13259] open (point > 0) | 1498 [-4621, 13259] open (point > 0) |
| US 2020 household mix, new box | 21794 [6635, 56847] cost (interval > 0) | 21267 [6635, 56847] cost (interval > 0) | 21296 [6635, 56847] cost (interval > 0) |

The bounds are the same in all three columns. They span every row's full interval, which
contains all three settings. The points move.

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
| home node on existing gateway | household-hour | expected | short run | 2424 [371, 10058] | 658 [56.4, 4337] | 884 [-424, 5399] | 2197 [-4948, 14819] | [-6200, 15407] | open (point > 0) | FRONTIER |
| home node on existing gateway | household-hour | expected | long run | 2424 [371, 10058] | 658 [56.4, 4337] | 650 [-42934, 5394] | 2432 [-4943, 57330] | [-31922, 89739] | open (point > 0) | FRONTIER |
| home node on computer already on | household-hour | expected | short run | 3864 [731, 15458] | 658 [56.4, 4337] | 884 [-424, 5399] | 3637 [-4588, 20219] | [-5840, 20807] | open (point > 0) | FRONTIER |
| home node on computer already on | household-hour | expected | long run | 3864 [731, 15458] | 658 [56.4, 4337] | 650 [-42934, 5394] | 3872 [-4583, 62730] | [-31562, 95139] | open (point > 0) | FRONTIER |
| home node on a new always-on box | household-hour | expected | short run | 984 [11.5, 4658] | 22258 [11936, 53909] | 884 [-424, 5399] | 22357 [6572, 58991] | [5320, 59579] | cost (interval > 0) | FRONTIER |
| home node on a new always-on box | household-hour | expected | long run | 984 [11.5, 4658] | 22258 [11936, 53909] | 650 [-42934, 5394] | 22592 [6577, 101502] | [-20402, 133911] | cost (interval > 0) | FRONTIER |
| phones only (home with a fixed line) | household-hour | expected | short run | 984 [11.5, 4649] | 441 [34.7, 3254] | 885 [-421, 5399] | 540 [-5330, 8324] | [-6582, 8910] | open (point > 0) | FRONTIER |
| phones only (home with a fixed line) | household-hour | expected | long run | 984 [11.5, 4649] | 441 [34.7, 3254] | 661 [-42778, 5395] | 764 [-5326, 50681] | [-32304, 83086] | open (point > 0) | FRONTIER |
| phones only (cellular-only home) | household-hour | expected | short run | 2515 [70.0, 7695] | 441 [34.7, 3254] | 855 [-977, 5396] | 2101 [-5239, 11926] | [-7282, 12732] | open (point > 0) | FRONTIER |
| phones only (cellular-only home) | household-hour | expected | long run | 2515 [70.0, 7695] | 441 [34.7, 3254] | 216 [-80020, 5366] | 2740 [-5209, 90970] | [-48892, 139072] | open (point > 0) | FRONTIER |
| new box on cellular (cellular-only home) | household-hour | expected | short run | 984 [11.5, 4658] | 29458 [15536, 68309] | 856 [-836, 5394] | 29585 [10177, 73803] | [8687, 74666] | cost (interval > 0) | FRONTIER |
| new box on cellular (cellular-only home) | household-hour | expected | long run | 984 [11.5, 4658] | 29458 [15536, 68309] | 239 [-70525, 5353] | 30203 [10218, 143493] | [-32601, 194065] | cost (interval > 0) | FRONTIER |
| call relayed by a neighbour vs SFU | participant-hour | expected | short run | 797 [181, 3623] | 0.000 [0.000, 0.000] | -1427 [-9919, 15967] | 2224 [-15786, 13542] | [-34630, 32385] | open (point > 0) | FRONTIER |
| call relayed by a neighbour vs SFU | participant-hour | expected | long run | 797 [181, 3623] | 0.000 [0.000, 0.000] | -77358 [-700291, 959] | 78156 [-778, 703914] | [-1329114, 2032249] | open (point > 0) | FRONTIER |
| call relayed in a data centre vs SFU | participant-hour | expected | short run | 508 [91.1, 2620] | 3600 [360, 18000] | 3600 [360, 18000] | 508 [91.1, 2620] | [-36382, 39092] | cost (interval > 0) | FRONTIER |
| call relayed in a data centre vs SFU | participant-hour | expected | long run | 508 [91.1, 2620] | 3600 [360, 18000] | 3600 [360, 18000] | 508 [91.1, 2620] | [-1345874, 1348585] | cost (interval > 0) | FRONTIER |
| village, one thin uplink | household-hour | expected | short run | 9.1 [1.7, 80.5] | 1161 [125, 6853] | 907 [46.2, 7608] | 263 [-7463, 6864] | [-17763, 17097] | open (point > 0) | FRONTIER |
| village, one thin uplink | household-hour | expected | long run | 9.1 [1.7, 80.5] | 1161 [125, 6853] | 907 [46.2, 7608] | 263 [-7463, 6864] | [-17763, 17097] | open (point > 0) | FRONTIER |
| US 2020 household mix, node on gateway | household-hour | expected | short run | 2256 [297, 8937] | 575 [48.8, 3844] | 813 [-477, 4993] | 2018 [-4621, 13259] | [-5901, 13836] | open (point > 0) | FRONTIER |
| US 2020 household mix, node on gateway | household-hour | expected | long run | 2256 [297, 8937] | 575 [48.8, 3844] | 534 [-45438, 4985] | 2297 [-4613, 58220] | [-32145, 90619] | open (point > 0) | FRONTIER |
| US 2020 household mix, new box | household-hour | expected | short run | 910 [10.6, 4308] | 21698 [11596, 52084] | 813 [-456, 4993] | 21794 [6635, 56847] | [5440, 57434] | cost (interval > 0) | FRONTIER |
| US 2020 household mix, new box | household-hour | expected | long run | 910 [10.6, 4308] | 21698 [11596, 52084] | 538 [-43971, 4983] | 22070 [6645, 100363] | [-20754, 133144] | cost (interval > 0) | FRONTIER |

By device class (expected regime, short run, stand-in presence, J as each enters the delta):

| scenario | phone | home node | hosted | network | data centre | village node |
|---|---|---|---|---|---|---|
| home node on existing gateway | 984 | 1440 | 658 | 15.9 | -900 | 0.000 |
| home node on computer already on | 984 | 2880 | 658 | 15.9 | -900 | 0.000 |
| home node on a new always-on box | 984 | 21600 | 658 | 15.9 | -900 | 0.000 |
| phones only (home with a fixed line) | 984 | 0.000 | 441 | 15.2 | -900 | 0.000 |
| phones only (cellular-only home) | 2515 | 0.000 | 441 | 45.1 | -900 | 0.000 |
| new box on cellular (cellular-only home) | 984 | 28800 | 658 | 43.8 | -900 | 0.000 |
| call relayed by a neighbour vs SFU | 622 | 175 | 0.000 | 5027 | -3600 | 0.000 |
| call relayed in a data centre vs SFU | 444 | 64.2 | 3600 | 0.000 | -3600 | 0.000 |
| village, one thin uplink | 9.1 | 0.000 | 441 | -6.6 | -900 | 720 |
| US 2020 household mix, node on gateway | 1146 | 1109 | 575 | 19.2 | -832 | 0.000 |
| US 2020 household mix, new box | 910 | 21090 | 608 | 19.0 | -832 | 0.000 |

## 8. Break-even: what decides the sign

The phone-radio inputs come first. Levers are fixed at what the edge does today, and their
zero is a design threshold. Then come the inputs whose zero lies inside their own interval
(these decide the sign), then the largest movers. Expected regime, stand-in presence. The full
table, including the long run and every scenario, is printed by `cewp-cli energy`.

| scenario | run | input | class | interval | delta at lo / hi | zero at | inside interval |
|---|---|---|---|---|---|---|---|
| home node on existing gateway | short run | `lever.cell_round_share` | MODEL | [1, 1] share | 2197 / 2197 | none in range | no |
| home node on existing gateway | short run | `phone.wifi_share` | MODEL | [0.4, 0.9] share | 2907 / 1490 | 1.4287 | no |
| home node on existing gateway | short run | `phone.cell.tail_s` | MODEL | [2, 12] s | 1418 / 2230 | none in range | no |
| home node on existing gateway | short run | `lever.round_cadence_s` | MODEL | [30, 30] s | 2197 / 2197 | none in range | no |
| home node on existing gateway | short run | `phone.cell.tail_w` | MODEL | [0.5, 1.5] W | 1700 / 2589 | none in range | no |
| home node on existing gateway | short run | `phone.cell.promotion_j` | MODEL | [0.2, 1.5] J | 2178 / 2277 | none in range | no |
| home node on existing gateway | short run | `lever.phone_schedule_share` | MODEL | [1, 1] share | 2197 / 2197 | none in range | no |
| home node on existing gateway | short run | `home_node.on_router.marginal_w` | FRONTIER | [0.1, 1.5] W | 1117 / 6157 | none in range | no |
| home node on existing gateway | short run | `datacentre.messaging.wh_per_person_hour` | FRONTIER | [0.02, 0.5] Wh | 2917 / -1403 | 0.3442 | yes |
| home node on existing gateway | short run | `hosted.seat.w` | MODEL | [60, 300] W | 1870 / 3180 | none in range | no |
| home node on a new always-on box | short run | `lever.cell_round_share` | MODEL | [1, 1] share | 22357 / 22357 | none in range | no |
| home node on a new always-on box | short run | `phone.wifi_share` | MODEL | [0.4, 0.9] share | 23067 / 21650 | 8.8889 | no |
| home node on a new always-on box | short run | `phone.cell.tail_s` | MODEL | [2, 12] s | 21578 / 22390 | none in range | no |
| home node on a new always-on box | short run | `lever.round_cadence_s` | MODEL | [30, 30] s | 22357 / 22357 | none in range | no |
| home node on a new always-on box | short run | `phone.cell.tail_w` | MODEL | [0.5, 1.5] W | 21860 / 22749 | none in range | no |
| home node on a new always-on box | short run | `phone.cell.promotion_j` | MODEL | [0.2, 1.5] J | 22338 / 22437 | none in range | no |
| home node on a new always-on box | short run | `lever.phone_schedule_share` | MODEL | [1, 1] share | 22357 / 22357 | none in range | no |
| home node on a new always-on box | short run | `home_node.added_box.w` | MODEL | [3.3, 13.77] W | 12637 / 50329 | none in range | no |
| home node on a new always-on box | short run | `datacentre.messaging.wh_per_person_hour` | FRONTIER | [0.02, 0.5] Wh | 23077 / 18757 | 2.5842 | no |
| phones only (home with a fixed line) | short run | `lever.cell_round_share` | MODEL | [1, 1] share | 540 / 540 | 0.4479 | no |
| phones only (home with a fixed line) | short run | `phone.wifi_share` | MODEL | [0.4, 0.9] share | 1248 / -167 | 0.8408 | yes |
| phones only (home with a fixed line) | short run | `phone.cell.tail_s` | MODEL | [2, 12] s | -240 / 572 | 4.9563 | yes |
| phones only (home with a fixed line) | short run | `lever.round_cadence_s` | MODEL | [30, 30] s | 540 / 540 | 71.3873 | no |
| phones only (home with a fixed line) | short run | `phone.cell.tail_w` | MODEL | [0.5, 1.5] W | 41.9 / 931 | 0.4529 | no |
| phones only (home with a fixed line) | short run | `phone.cell.promotion_j` | MODEL | [0.2, 1.5] J | 520 / 619 | none in range | no |
| phones only (home with a fixed line) | short run | `lever.phone_schedule_share` | MODEL | [1, 1] share | 540 / 540 | 0.4518 | no |
| phones only (home with a fixed line) | short run | `datacentre.messaging.wh_per_person_hour` | FRONTIER | [0.02, 0.5] Wh | 1260 / -3060 | 0.1600 | yes |
| phones only (home with a fixed line) | short run | `hosted.seat.w` | MODEL | [60, 300] W | 320 / 1198 | none in range | no |
| phones only (home with a fixed line) | short run | `hosted.devices_per_seat` | MODEL | [1000, 10000] devices | 979 / 189 | none in range | no |
| phones only (cellular-only home) | short run | `lever.cell_round_share` | MODEL | [1, 1] share | 2101 / 2101 | 0.1642 | no |
| phones only (cellular-only home) | short run | `phone.wifi_share.cell_only_home` | FRONTIER | [0, 0.3] share | 2386 / 1532 | 0.8408 | no |
| phones only (cellular-only home) | short run | `phone.cell.tail_s` | MODEL | [2, 12] s | 95.8 / 2185 | 1.5415 | no |
| phones only (cellular-only home) | short run | `lever.round_cadence_s` | MODEL | [30, 30] s | 2101 / 2101 | 255.4235 | no |
| phones only (cellular-only home) | short run | `phone.cell.tail_w` | MODEL | [0.5, 1.5] W | 821 / 3107 | 0.1409 | no |
| phones only (cellular-only home) | short run | `phone.cell.promotion_j` | MODEL | [0.2, 1.5] J | 2050 / 2306 | none in range | no |
| phones only (cellular-only home) | short run | `lever.phone_schedule_share` | MODEL | [1, 1] share | 2101 / 2101 | 0.1655 | no |
| phones only (cellular-only home) | short run | `datacentre.messaging.wh_per_person_hour` | FRONTIER | [0.02, 0.5] Wh | 2821 / -1499 | 0.3335 | yes |
| phones only (cellular-only home) | short run | `phone.mesh_extra_content_wakes` | FRONTIER | [-0.3, 0.3] share | 1547 / 2656 | none in range | no |
| phones only (cellular-only home) | short run | `hosted.seat.w` | MODEL | [60, 300] W | 1882 / 2760 | none in range | no |
| call relayed by a neighbour vs SFU | short run | `datacentre.call.wh_per_participant_hour` | FRONTIER | [0.1, 5] Wh | 5464 / -12176 | 1.6179 | yes |
| call relayed by a neighbour vs SFU | short run | `network.mobile.traffic_j_per_mb` | MODEL | [5, 25] J/MB | -586 / 7178 | 6.5101 | yes |
| call relayed by a neighbour vs SFU | short run | `phone.j_per_chunk_sig_op` | FRONTIER | [0.00002, 0.0005] J | 1874 / 3976 | none in range | no |
| call relayed by a neighbour vs SFU | short run | `network.fixed.traffic_j_per_mb` | MODEL | [0.08, 0.5] J/MB | 2041 / 2523 | none in range | no |
| call relayed in a data centre vs SFU | short run | `phone.j_per_chunk_sig_op` | FRONTIER | [0.00002, 0.0005] J | 158 / 2260 | none in range | no |
| call relayed in a data centre vs SFU | short run | `home_node.j_per_chunk_sig_op` | MODEL | [0.00001, 0.0003] J | 454 / 848 | none in range | no |
| village, one thin uplink | short run | `lever.round_cadence_s` | MODEL | [30, 30] s | 263 / 263 | none in range | no |
| village, one thin uplink | short run | `lever.phone_schedule_share` | MODEL | [1, 1] share | 263 / 263 | none in range | no |
| village, one thin uplink | short run | `datacentre.messaging.wh_per_person_hour` | FRONTIER | [0.02, 0.5] Wh | 983 / -3337 | 0.1293 | yes |
| village, one thin uplink | short run | `village.households` | MODEL | [20, 200] households | 1343 / -277 | 78.8330 | yes |
| village, one thin uplink | short run | `village.node.w` | MODEL | [5, 20] W | -96.7 / 983 | 6.3425 | yes |
| village, one thin uplink | short run | `hosted.seat.w` | MODEL | [60, 300] W | 43.8 / 922 | 48.0201 | no |
| village, one thin uplink | short run | `hosted.devices_per_seat` | MODEL | [1000, 10000] devices | 702 / -87.9 | 4997.9044 | yes |
| village, one thin uplink | short run | `workload.bytes_per_act` | MODEL | [1000, 50000] bytes | 279 / 86.1 | 71872.7552 | no |
| US 2020 household mix, node on gateway | short run | `lever.cell_round_share` | MODEL | [1, 1] share | 2018 / 2018 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.wifi_share` | MODEL | [0.4, 0.9] share | 2564 / 1473 | 1.5788 | no |
| US 2020 household mix, node on gateway | short run | `phone.wifi_share.cell_only_home` | FRONTIER | [0, 0.3] share | 2062 / 1930 | 4.7991 | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.tail_s` | MODEL | [2, 12] s | 1107 / 2055 | none in range | no |
| US 2020 household mix, node on gateway | short run | `lever.round_cadence_s` | MODEL | [30, 30] s | 2018 / 2018 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.tail_w` | MODEL | [0.5, 1.5] W | 1436 / 2474 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.promotion_j` | MODEL | [0.2, 1.5] J | 1994 / 2111 | none in range | no |
| US 2020 household mix, node on gateway | short run | `lever.phone_schedule_share` | MODEL | [1, 1] share | 2018 / 2018 | none in range | no |
| US 2020 household mix, node on gateway | short run | `datacentre.messaging.wh_per_person_hour` | FRONTIER | [0.02, 0.5] Wh | 2683 / -1312 | 0.3424 | yes |
| US 2020 household mix, node on gateway | short run | `home_node.on_router.marginal_w` | FRONTIER | [0.1, 1.5] W | 1186 / 5068 | none in range | no |
| US 2020 household mix, node on gateway | short run | `hosted.seat.w` | MODEL | [60, 300] W | 1731 / 2876 | none in range | no |

### Findings (the sign and what decides it; nothing here is a saving or a cost claim)

* **A new always-on box is a cost** across its whole interval: +22.4 kJ per household-hour, in
  [+6.6, +59.0] kJ (6.2 W at the point). On cellular with a modem it is +29.6 kJ, in
  [+10.2, +73.8]. In the US 2020 mix it is +21.8 kJ, in [+6.6, +56.8]. The box's full draw is
  of the gateway's own order and dominates. No avoided-upstream input inside its row reaches it.
* **A node on the existing gateway, phones only, the village and the gateway mix are open.**
  The sign is decided first by `datacentre.messaging.wh_per_person_hour`, with break-even
  0.34 Wh (gateway), 0.16 (phones only, fixed line), 0.33 (cellular-only phones), 0.13
  (village) and 0.34 (mix). POWER-0 lists that quantity as **not available**: no published
  figure exists for messaging alone. The decisive input is one nobody publishes.
* **The phone radio decides phones-only cases.** For phones only with a fixed line (stand-in),
  the Wi-Fi share flips the sign at 0.84 and the carrier tail at 4.96 s. The lever
  `cell_round_share` flips it at 0.45 (fixed-line home) and 0.16 (cellular-only home). The
  round cadence flips it at 71 s (fixed line) and 255 s (cellular only). Under PRESENCE-0 the
  phones-only point drops from +540 to −31 (edge as is) or +0.2 (rounds only while awake). The
  sign sits on the presence input.
* **A call relayed by a neighbour** is open in the short run: +2.2 kJ per participant-hour, in
  [−15.8, +13.5]. It is decided by SFU energy (break-even 1.62 Wh per participant-hour) and by
  mobile traffic J/MB (6.5). In the long run the mesh's extra bytes on mobile capacity make it
  +78 kJ, in [−0.8, +704]. **A call relayed in a data centre** is a cost of +0.5 kJ, in
  [+0.1, +2.6]: the relay is an SFU, and the delta is chunk signing on the devices.
* **Network.** In the short run the traffic term barely moves anything on fixed lines. Mobile
  capacity in the long run is where the mesh's extra bytes cost most.

## 9. Hooks (flat `Vec<f32>`, class codes as `seat_hooks::class`)

* `view_delta(seed)`: `DELTA_STRIDE = 16` per (scenario, regime, run): `[scenario, regime, run
  (0 short, 1 long), per (0 household-hour, 1 participant-hour), A, A_lo, A_hi, B, B_lo, B_hi,
  C, C_lo, C_hi, net, net_lo, net_hi]`. Scenarios are in `Scenario::ALL` order (0 gateway,
  1 computer, 2 new box, 3 phones only (fixed line), 4 phones only (cellular only), 5 box on
  cellular, 6 call via a neighbour, 7 call via a DC, 8 village, 9 mix gateway, 10 mix box).
  Regimes are 0 optimistic, 1 expected, 2 pessimistic, 3 adversarial. 88 rows. The sign is a
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
