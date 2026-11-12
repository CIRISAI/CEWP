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
| `datacentre.messaging.wh_per_person_hour` | data centre | C1 delivery: message relay and fan-out one person drives, per hour of the day | Wh | 0.02 / 0.1 / 0.5 | FRONTIER | n/a | 2026 | - | POWER-0 not_available: no published figure for messaging alone; row 43 (415 TWh, every workload) is not this | a stated bound: a few Wh per user-day for a whole platform, of which messaging is an unknown share |
| `c.substitution_share` | lever | share of use that MOVES from an ad-funded platform to the mesh (not kept alongside it) | share | 0.5 / 0.5 / 0.5 | FRONTIER | n/a | 2026 | - | ours | 0 = the mesh is purely additive, the honest floor ('people use phones regardless' applies to platforms too); the report shows 0, 0.25, 0.5 and 1 side by side; it multiplies every status-quo line of C |
| `c.short_run_share_c2_c4` | data centre | share of retention, harvesting and ad-system energy that moves with one user's use while capacity is fixed | share | 0.0 / 0.1 / 0.3 | FRONTIER | n/a | 2026 | - | ours | storage and training capacity are provisioned per user and per product, so C2-C4 live in the long run; in the short run only marginal inference and request handling move |
| `c.platform_use_hours_per_person_day` | workload | hours a day a person spends in the ad-funded apps the mesh would replace | h | 1.0 / 2.5 / 5.0 | FRONTIER | n/a | 2026 | - | ours | of the order of two to three hours a day of social and messaging use |
| `c2.retained_gb_per_user_year` | data centre | messages, media, contacts and metadata a platform keeps per user per year | GB | 0.5 / 3.0 / 20.0 | FRONTIER | n/a | 2026 | - | ours | media-heavy users far above text-only ones |
| `c2.years_kept` | data centre | years a platform keeps it | years | 2.0 / 8.0 / 20.0 | FRONTIER | n/a | 2026 | - | ours | indefinite retention in practice; the stock per user is GB a year times years kept |
| `c2.replication_factor` | data centre | copies across regions and backups | copies | 2.0 / 3.0 / 6.0 | FRONTIER | n/a | 2026 | - | ours | three-way replication plus backups and caches |
| `c2.storage_w_per_tb` | data centre | storage power per TB held, servers and PUE included | W/TB | 0.5 / 2.0 / 8.0 | FRONTIER | n/a | 2026 | - | ours | dense disk at a fraction of a W per TB raw, several W per TB with servers, network and overhead |
| `c2.mesh_retained_gb_per_user` | home node | content the mesh keeps per member on members' own devices (bounded retention) | GB | 0.5 / 5.0 / 30.0 | FRONTIER | yes | 2026 | - | ours | the custody model holds members' content on their own devices with bounded retention; durability.rs gives bytes held per holder but no time axis |
| `c2.device_storage_w_per_tb` | home node | marginal storage power on a device already on | W/TB | 0.0 / 0.2 / 1.0 | FRONTIER | yes | 2026 | - | ours | flash or a disk that spins anyway: near zero marginal |
| `c3.harvest_wh_per_use_hour` | data centre | event logging, tracking pipelines, profiling, feature stores, ranking and ad-targeting training and inference, per user-hour of use | Wh | 0.05 / 0.5 / 5.0 | FRONTIER | n/a | 2026 | - | ours | the machinery behind the feed and the ads; no published per-user figure |
| `c4.impressions_per_use_hour` | data centre | ad impressions per user-hour of use | impressions | 20.0 / 60.0 / 200.0 | FRONTIER | n/a | 2026 | - | ours | an ad every few posts in a scrolling feed |
| `c4.j_per_impression` | data centre | the ad-tech chain per impression: auction, exchanges, bidders, verification | J | 0.5 / 5.0 / 50.0 | FRONTIER | n/a | 2026 | - | ours | dozens of bid requests per impression across exchanges and bidders |
| `c5.ad_and_tracker_mb_per_use_hour` | phone | ad creatives and tracker bytes downloaded per user-hour of use | MB | 5.0 / 20.0 / 100.0 | FRONTIER | no | 2026 | - | ours | video ads dominate; trackers are small but many |
| `c5.render_j_per_impression` | phone | phone CPU and GPU to fetch scripts and render one ad | J | 0.05 / 0.2 / 1.0 | FRONTIER | no | 2026 | phone on a power monitor scrolling a feed with and without an ad blocker | ours | script execution and decoding per ad |
| `c5.tracker_wakes_per_phone_hour` | phone | telemetry and tracker requests that wake the radio, per unsuspended phone-hour | wakes | 0.0 / 6.0 / 60.0 | FRONTIER | no | 2026 | a phone's wake log with the platform app installed and idle | ours | a platform app that phones home pays the same promotion and tail term A charges the mesh's idle round |
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
| home node on existing gateway | 2532 [-36523, 15435] open (point > 0) | 1978 [-36523, 15435] open (point > 0) | 2003 [-36523, 15435] open (point > 0) |
| home node on computer already on | 3972 [-36163, 20835] open (point > 0) | 3418 [-36163, 20835] open (point > 0) | 3443 [-36163, 20835] open (point > 0) |
| home node on a new always-on box | 22692 [-25003, 59607] open (point > 0) | 22138 [-25003, 59607] open (point > 0) | 22163 [-25003, 59607] open (point > 0) |
| phones only (home with a fixed line) | 874 [-36907, 8941] open (point > 0) | 321 [-36907, 8941] open (point > 0) | 346 [-36907, 8941] open (point > 0) |
| phones only (cellular-only home) | 2388 [-37201, 12662] open (point > 0) | 2388 [-37201, 12662] open (point > 0) | 1717 [-37201, 12662] open (point > 0) |
| new box on cellular (cellular-only home) | 29907 [-21694, 74539] open (point > 0) | 29356 [-21694, 74539] open (point > 0) | 29381 [-21694, 74539] open (point > 0) |
| village, one thin uplink | 550 [-51750, 11063] open (point > 0) | 550 [-51750, 11063] open (point > 0) | 548 [-51750, 11063] open (point > 0) |
| US 2020 household mix, node on gateway | 2319 [-33884, 13847] open (point > 0) | 1893 [-33884, 13847] open (point > 0) | 1809 [-33884, 13847] open (point > 0) |
| US 2020 household mix, new box | 22102 [-22613, 57436] open (point > 0) | 21590 [-22613, 57436] open (point > 0) | 21613 [-22613, 57436] open (point > 0) |
| US 2020 household mix, node on a computer already on | 2101 [-33925, 12911] open (point > 0) | 1675 [-33925, 12911] open (point > 0) | 1591 [-33925, 12911] open (point > 0) |
| US 2020 household mix, no home node | 1042 [-34180, 8844] open (point > 0) | 616 [-34180, 8844] open (point > 0) | 532 [-34180, 8844] open (point > 0) |
| host | households it reaches | delta short run | delta long run |
|---|---|---|---|
| US 2020 household mix, node on gateway | 72.8% | 2319 [-33884, 13847] open (point > 0) | 1715 [-137400, 71371] open (point > 0) |
| US 2020 household mix, node on a computer already on | 32.3% | 2101 [-33925, 12911] open (point > 0) | 1492 [-137507, 70369] open (point > 0) |
| US 2020 household mix, new box | 87.4% | 22102 [-22613, 57436] open (point > 0) | 21490 [-126120, 113515] open (point > 0) |
| US 2020 household mix, no home node | 0.0% | 1042 [-34180, 8844] open (point > 0) | 430 [-137814, 66250] open (point > 0) |

The bounds are the same in all three columns. They span every row's full interval, which
contains all three settings. The points move.

## 4b. Term C rebuilt as named components (Eric, 2026-11-11)

Eric's correction: "most of the data center energy we are saving is not from the pure
processing, it is the storing and harvesting for personal information and ads". Term C is now
a sum of named components. Each has its own rows, class, interval and a substitution share, so
the claim can be tested rather than assumed:

* **C1 delivery**: message relay and fan-out (`datacentre.messaging.wh_per_person_hour`), the
  SFU for calls, and the status quo's delivery bytes on access.
* **C2 retention**: every user's content kept, replicated and held:
  GB/user-year × years kept × replication × W/TB. The mesh side (content on members' own
  devices with bounded retention, a marginal draw on devices that are on anyway) goes in A.
* **C3 harvesting**: event logging, tracking, profiling, feature stores, and ranking and
  targeting training and inference, per user-hour of use.
* **C4 the ad system**: impressions per user-hour × J per impression across the ad-tech chain.
* **C5 the device and network side**: ad and tracker bytes on the phone's radio and on access,
  ads rendered on the phone, and tracker and telemetry wake-ups. These pay the same promotion
  and tail that term A charges the mesh's idle rounds.

Every C2-C5 row is FRONTIER with stated reasoning, laid out for CIRISHolon to source row for row.
**The substitution share** (`c.substitution_share`) multiplies every status-quo line of C. It
is the share of use that actually moves off an ad-funded platform rather than being kept
alongside it. At 0 the mesh is purely additive, which is the honest floor. **C2-C4 live in the
long run**: storage and training capacity are provisioned per user and per product. In the short
run only `c.short_run_share_c2_c4` (0.1, FRONTIER) of them moves with one user's use.

### C by component (expected regime, substitution 0.5, point J per household-hour or participant-hour)

| scenario | run | C1 delivery | C2 retention | C3 harvesting | C4 ad system | C5 ad/tracker device+network | mesh upstream | C total | A + B |
|---|---|---|---|---|---|---|---|---|---|
| home node on existing gateway | short run | 455 | 64.8 | 23.4 | 3.9 | 37.3 | -25.1 | 559 | 3091 |
| home node on existing gateway | long run | 523 | 648 | 234 | 39.1 | 208 | -395 | 1257 | 3091 |
| home node on computer already on | short run | 455 | 64.8 | 23.4 | 3.9 | 37.3 | -25.1 | 559 | 4531 |
| home node on computer already on | long run | 523 | 648 | 234 | 39.1 | 208 | -395 | 1257 | 4531 |
| home node on a new always-on box | short run | 455 | 64.8 | 23.4 | 3.9 | 37.3 | -25.1 | 559 | 23251 |
| home node on a new always-on box | long run | 523 | 648 | 234 | 39.1 | 208 | -395 | 1257 | 23251 |
| phones only (home with a fixed line) | short run | 455 | 64.8 | 23.4 | 3.9 | 37.3 | -24.4 | 560 | 1434 |
| phones only (home with a fixed line) | long run | 523 | 648 | 234 | 39.1 | 208 | -385 | 1267 | 1434 |
| phones only (cellular-only home) | short run | 461 | 64.8 | 23.4 | 3.9 | 91.7 | -68.1 | 577 | 2965 |
| phones only (cellular-only home) | long run | 624 | 648 | 234 | 39.1 | 499 | -1032 | 1012 | 2965 |
| new box on cellular (cellular-only home) | short run | 463 | 64.8 | 23.4 | 3.9 | 57.6 | -69.2 | 543 | 30451 |
| new box on cellular (cellular-only home) | long run | 642 | 648 | 234 | 39.1 | 508 | -1046 | 1025 | 30451 |
| call relayed by a neighbour vs SFU | short run | 6674 | 0.000 | 0.000 | 0.000 | 0.000 | -14774 | -8101 | 797 |
| call relayed by a neighbour vs SFU | long run | 80287 | 0.000 | 0.000 | 0.000 | 0.000 | -237932 | -157645 | 797 |
| call relayed in a data centre vs SFU | short run | 6674 | 0.000 | 0.000 | 0.000 | 0.000 | -9747 | -3074 | 4108 |
| call relayed in a data centre vs SFU | long run | 80287 | 0.000 | 0.000 | 0.000 | 0.000 | -156974 | -76687 | 4108 |
| village, one thin uplink | short run | 502 | 64.8 | 23.4 | 3.9 | 132 | -97.4 | 629 | 1179 |
| village, one thin uplink | long run | 502 | 648 | 234 | 39.1 | 132 | -97.4 | 1458 | 1179 |
| US 2020 household mix, node on gateway | short run | 422 | 59.9 | 21.7 | 3.6 | 42.9 | -29.9 | 520 | 2839 |
| US 2020 household mix, node on gateway | long run | 499 | 599 | 217 | 36.1 | 237 | -464 | 1124 | 2839 |
| US 2020 household mix, new box | short run | 422 | 59.9 | 21.7 | 3.6 | 37.6 | -30.0 | 515 | 22616 |
| US 2020 household mix, new box | long run | 502 | 599 | 217 | 36.1 | 239 | -466 | 1126 | 22616 |
| US 2020 household mix, node on a computer already on | short run | 422 | 59.9 | 21.7 | 3.6 | 42.9 | -29.6 | 520 | 2621 |
| US 2020 household mix, node on a computer already on | long run | 499 | 599 | 217 | 36.1 | 237 | -459 | 1129 | 2621 |
| US 2020 household mix, no home node | short run | 422 | 59.9 | 21.7 | 3.6 | 42.9 | -29.3 | 520 | 1562 |
| US 2020 household mix, no home node | long run | 499 | 599 | 217 | 36.1 | 237 | -456 | 1133 | 1562 |

### The substitution share side by side (delta point [bounds] sign)

| scenario | run | 0 | 0.25 | 0.5 | 1 |
|---|---|---|---|---|---|
| home node on existing gateway | short run | 3116 [429, 15738] cost | 2824 [-17986, 15587] open | 2532 [-36523, 15435] open | 1948 [-73598, 15133] open |
| home node on existing gateway | long run | 3486 [434, 83458] cost | 2660 [-71435, 76838] open | 1834 [-145233, 70218] open | 182 [-292829, 56977] open |
| home node on computer already on | short run | 4556 [789, 21138] cost | 4264 [-17626, 20987] open | 3972 [-36163, 20835] open | 3388 [-73238, 20533] open |
| home node on computer already on | long run | 4926 [794, 88858] cost | 4100 [-71075, 82238] open | 3274 [-144873, 75618] open | 1622 [-292469, 62377] open |
| home node on a new always-on box | short run | 23276 [11949, 59910] cost | 22984 [-6466, 59759] open | 22692 [-25003, 59607] open | 22108 [-62078, 59305] open |
| home node on a new always-on box | long run | 23646 [11954, 127630] cost | 22820 [-59915, 121010] open | 21994 [-133713, 114390] open | 20342 [-281309, 101149] open |
| phones only (home with a fixed line) | short run | 1458 [46.7, 9244] cost | 1166 [-18370, 9092] open | 874 [-36907, 8941] open | 290 [-73982, 8638] open |
| phones only (home with a fixed line) | long run | 1818 [51.1, 76810] cost | 992 [-71973, 70190] open | 166 [-145771, 63569] open | -1486 [-293366, 50329] open |
| phones only (cellular-only home) | short run | 3033 [109, 13090] cost | 2711 [-18478, 12876] open | 2388 [-37201, 12662] open | 1742 [-74647, 12233] open |
| phones only (cellular-only home) | long run | 3997 [137, 133460] cost | 2975 [-81001, 122647] open | 1953 [-165157, 111833] open | -91.3 [-333467, 90207] open |
| new box on cellular (cellular-only home) | short run | 30520 [15554, 74967] cost | 30214 [-3040, 74753] open | 29907 [-21694, 74539] open | 29295 [-59078, 74111] open |
| new box on cellular (cellular-only home) | long run | 31497 [15595, 185984] cost | 30461 [-65744, 175170] open | 29425 [-149829, 164357] open | 27354 [-317999, 142731] open |
| call relayed by a neighbour vs SFU | short run | 15572 [6155, 33832] cost | 12235 [670, 28760] cost | 8898 [-4816, 23687] open | 2224 [-15786, 13542] open |
| call relayed by a neighbour vs SFU | long run | 238730 [50263, 2062796] cost | 198586 [37503, 1723075] cost | 158443 [24742, 1383355] cost | 78156 [-778, 703914] open |
| call relayed in a data centre vs SFU | short run | 13856 [4393, 40550] cost | 10519 [3317, 31067] cost | 7182 [2242, 21585] cost | 508 [91.1, 2620] cost |
| call relayed in a data centre vs SFU | long run | 161082 [33493, 1379142] cost | 120939 [25142, 1035011] cost | 80795 [16792, 690881] cost | 508 [91.1, 2620] cost |
| village, one thin uplink | short run | 1276 [129, 17566] cost | 913 [-25624, 14315] open | 550 [-51750, 11063] open | -176 [-106318, 7040] open |
| village, one thin uplink | long run | 1276 [129, 17566] cost | 499 [-64974, 14306] open | -279 [-130451, 11046] open | -1835 [-263720, 7005] open |
| US 2020 household mix, node on gateway | short run | 2869 [347, 14147] cost | 2594 [-16710, 13997] open | 2319 [-33884, 13847] open | 1770 [-68230, 13548] open |
| US 2020 household mix, node on gateway | long run | 3303 [355, 84913] cost | 2509 [-67546, 78142] open | 1715 [-137400, 71371] open | 126 [-277106, 57830] open |
| US 2020 household mix, new box | short run | 22646 [11608, 57735] cost | 22374 [-5445, 57586] open | 22102 [-22613, 57436] open | 21557 [-56950, 57136] open |
| US 2020 household mix, new box | long run | 23082 [11618, 127057] cost | 22286 [-56271, 120286] open | 21490 [-126120, 113515] open | 19897 [-265817, 99974] open |
| US 2020 household mix, node on a computer already on | short run | 2651 [306, 13210] cost | 2376 [-16752, 13060] open | 2101 [-33925, 12911] open | 1552 [-68272, 12611] open |
| US 2020 household mix, node on a computer already on | long run | 3081 [314, 83911] cost | 2287 [-67653, 77140] open | 1492 [-137507, 70369] open | -96.1 [-277214, 56828] open |
| US 2020 household mix, no home node | short run | 1592 [52.8, 9144] cost | 1317 [-17006, 8994] open | 1042 [-34180, 8844] open | 493 [-68526, 8544] open |
| US 2020 household mix, no home node | long run | 2018 [60.6, 79791] cost | 1224 [-67960, 73021] open | 430 [-137814, 66250] open | -1159 [-277520, 52709] open |

Break-even on the substitution share, other inputs at their defaults (`cewp-cli energy`), is
the share of use that would have to move before the point delta reaches zero. The short run
needs more than 1 everywhere except the village (0.88), so it cannot be reached. In the long
run: phones only on a fixed line 0.55, the village 0.41, US mix with no home node 0.64, US mix
with the node on a computer 0.97, and US mix with the node on the gateway 1.04, just out of
reach. The two new-box mixes need 7 or more, so no possible share reaches them.

## 4c. Where the home node runs (US 2020 household mix)

The home node's host is an explicit scenario input: the gateway, a computer already on, a new
box, or none. It applies only to households that can host it; the rest run phones only, and
homes with no internet are at zero. Household types come from a stated joint stand-in:
DEVICES-0 marginals, CIRISHolon DEVICEGEN-1's certified 8.7% of households with nothing to host
on, and a desktop spread evenly over homes with a computer (MODEL). It sits behind the seam
`holon_scene::world::devices::generate` (CIRISHolon 33dc99f) will fill. The rev is to be agreed
with twin-globe2, who is wiring the same call into the kernel. DEVICEGEN-1 declares a gateway
present with a fixed line and a desktop able to host. Whether the gateway can host and whether
the desktop is on all day are both unknown, so the shares reached are upper bounds. Envelope:
United States, 2020. By 2024 set-top boxes are +76% and streaming devices −22%.

| host | households it reaches | delta short run | delta long run |
|---|---|---|---|
| US 2020 household mix, node on gateway | 72.8% | 2319 [-33884, 13847] open (point > 0) | 1715 [-137400, 71371] open (point > 0) |
| US 2020 household mix, node on a computer already on | 32.3% | 2101 [-33925, 12911] open (point > 0) | 1492 [-137507, 70369] open (point > 0) |
| US 2020 household mix, new box | 87.4% | 22102 [-22613, 57436] open (point > 0) | 21490 [-126120, 113515] open (point > 0) |
| US 2020 household mix, no home node | 0.0% | 1042 [-34180, 8844] open (point > 0) | 430 [-137814, 66250] open (point > 0) |

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
| home node on existing gateway | household-hour | expected | short run | 2433 [371, 10382] | 658 [56.4, 4337] | 559 [-716, 37903] | 2532 [-36523, 15435] | [-38555, 15803] | open (point > 0) | FRONTIER |
| home node on existing gateway | household-hour | expected | long run | 2433 [371, 10382] | 658 [56.4, 4337] | 1257 [-55499, 146535] | 1834 [-145233, 70218] | [-162329, 90123] | open (point > 0) | FRONTIER |
| home node on computer already on | household-hour | expected | short run | 3873 [731, 15782] | 658 [56.4, 4337] | 559 [-716, 37903] | 3972 [-36163, 20835] | [-38195, 21203] | open (point > 0) | FRONTIER |
| home node on computer already on | household-hour | expected | long run | 3873 [731, 15782] | 658 [56.4, 4337] | 1257 [-55499, 146535] | 3274 [-144873, 75618] | [-161969, 95523] | open (point > 0) | FRONTIER |
| home node on a new always-on box | household-hour | expected | short run | 993 [11.5, 4982] | 22258 [11936, 53909] | 559 [-716, 37903] | 22692 [-25003, 59607] | [-27035, 59975] | open (point > 0) | FRONTIER |
| home node on a new always-on box | household-hour | expected | long run | 993 [11.5, 4982] | 22258 [11936, 53909] | 1257 [-55499, 146535] | 21994 [-133713, 114390] | [-150809, 134295] | open (point > 0) | FRONTIER |
| phones only (home with a fixed line) | household-hour | expected | short run | 993 [11.5, 4973] | 441 [34.7, 3254] | 560 [-714, 37905] | 874 [-36907, 8941] | [-38937, 9306] | open (point > 0) | FRONTIER |
| phones only (home with a fixed line) | household-hour | expected | long run | 993 [11.5, 4973] | 441 [34.7, 3254] | 1267 [-55343, 146691] | 166 [-145771, 63569] | [-162711, 83470] | open (point > 0) | FRONTIER |
| phones only (cellular-only home) | household-hour | expected | short run | 2524 [70.0, 8019] | 441 [34.7, 3254] | 577 [-1389, 38819] | 2388 [-37201, 12662] | [-40439, 13128] | open (point > 0) | FRONTIER |
| phones only (cellular-only home) | household-hour | expected | long run | 2524 [70.0, 8019] | 441 [34.7, 3254] | 1012 [-100561, 166646] | 1953 [-165157, 111833] | [-192069, 139455] | open (point > 0) | FRONTIER |
| new box on cellular (cellular-only home) | household-hour | expected | short run | 993 [11.5, 4982] | 29458 [15536, 68309] | 543 [-1248, 38193] | 29907 [-21694, 74539] | [-23848, 75062] | open (point > 0) | FRONTIER |
| new box on cellular (cellular-only home) | household-hour | expected | long run | 993 [11.5, 4982] | 29458 [15536, 68309] | 1025 [-91066, 165961] | 29425 [-149829, 164357] | [-174923, 194448] | open (point > 0) | FRONTIER |
| call relayed by a neighbour vs SFU | participant-hour | expected | short run | 797 [181, 3623] | 0.000 [0.000, 0.000] | -8101 [-20064, 4996] | 8898 [-4816, 23687] | [-15665, 34536] | open (point > 0) | FRONTIER |
| call relayed by a neighbour vs SFU | participant-hour | expected | long run | 797 [181, 3623] | 0.000 [0.000, 0.000] | -157645 [-1379732, -24562] | 158443 [24742, 1383355] | [-640853, 2048950] | cost (interval > 0) | FRONTIER |
| call relayed in a data centre vs SFU | participant-hour | expected | short run | 508 [91.1, 2620] | 3600 [360, 18000] | -3074 [-9785, 7029] | 7182 [2242, 21585] | [-17417, 41243] | cost (interval > 0) | FRONTIER |
| call relayed in a data centre vs SFU | participant-hour | expected | long run | 508 [91.1, 2620] | 3600 [360, 18000] | -76687 [-679081, -7521] | 80795 [16792, 690881] | [-657613, 1365285] | cost (interval > 0) | FRONTIER |
| village, one thin uplink | household-hour | expected | short run | 18.1 [1.7, 405] | 1161 [125, 6853] | 629 [-3805, 51905] | 550 [-51750, 11063] | [-58265, 17492] | open (point > 0) | FRONTIER |
| village, one thin uplink | household-hour | expected | long run | 18.1 [1.7, 405] | 1161 [125, 6853] | 1458 [-3788, 130606] | -279 [-130451, 11046] | [-136966, 17481] | open (point < 0) | FRONTIER |
| US 2020 household mix, node on gateway | household-hour | expected | short run | 2264 [297, 9237] | 575 [48.8, 3844] | 520 [-767, 35197] | 2319 [-33884, 13847] | [-35949, 14202] | open (point > 0) | FRONTIER |
| US 2020 household mix, node on gateway | household-hour | expected | long run | 2264 [297, 9237] | 575 [48.8, 3844] | 1124 [-58291, 138632] | 1715 [-137400, 71371] | [-154728, 90974] | open (point > 0) | FRONTIER |
| US 2020 household mix, new box | household-hour | expected | short run | 918 [10.6, 4608] | 21698 [11596, 52084] | 515 [-745, 35100] | 22102 [-22613, 57436] | [-24511, 57800] | open (point > 0) | FRONTIER |
| US 2020 household mix, new box | household-hour | expected | long run | 918 [10.6, 4608] | 21698 [11596, 52084] | 1126 [-56824, 138527] | 21490 [-126120, 113515] | [-143204, 133499] | open (point > 0) | FRONTIER |
| US 2020 household mix, node on a computer already on | household-hour | expected | short run | 2140 [266, 8766] | 482 [39.5, 3380] | 520 [-766, 35198] | 2101 [-33925, 12911] | [-35990, 13265] | open (point > 0) | FRONTIER |
| US 2020 household mix, node on a computer already on | household-hour | expected | long run | 2140 [266, 8766] | 482 [39.5, 3380] | 1129 [-58224, 138699] | 1492 [-137507, 70369] | [-154768, 89969] | open (point > 0) | FRONTIER |
| US 2020 household mix, no home node | household-hour | expected | short run | 1155 [19.7, 5070] | 408 [32.1, 3009] | 520 [-765, 35199] | 1042 [-34180, 8844] | [-36243, 9197] | open (point > 0) | FRONTIER |
| US 2020 household mix, no home node | household-hour | expected | long run | 1155 [19.7, 5070] | 408 [32.1, 3009] | 1133 [-58171, 138753] | 430 [-137814, 66250] | [-155022, 85848] | open (point > 0) | FRONTIER |

By device class (expected regime, short run, stand-in presence, J as each enters the delta):

| scenario | phone | home node | hosted | network | data centre | village node |
|---|---|---|---|---|---|---|
| home node on existing gateway | 958 | 1449 | 658 | 8.9 | -542 | 0.000 |
| home node on computer already on | 958 | 2889 | 658 | 8.9 | -542 | 0.000 |
| home node on a new always-on box | 958 | 21609 | 658 | 8.9 | -542 | 0.000 |
| phones only (home with a fixed line) | 958 | 9.0 | 441 | 8.3 | -542 | 0.000 |
| phones only (cellular-only home) | 2452 | 9.0 | 441 | 27.9 | -542 | 0.000 |
| new box on cellular (cellular-only home) | 958 | 28809 | 658 | 24.6 | -542 | 0.000 |
| call relayed by a neighbour vs SFU | 622 | 175 | 0.000 | 9901 | -1800 | 0.000 |
| call relayed in a data centre vs SFU | 444 | 64.2 | 3600 | 4874 | -1800 | 0.000 |
| village, one thin uplink | 7.1 | 9.0 | 441 | -84.8 | -542 | 720 |
| US 2020 household mix, node on gateway | 1117 | 1118 | 575 | 11.2 | -501 | 0.000 |
| US 2020 household mix, new box | 886 | 21098 | 608 | 10.7 | -501 | 0.000 |
| US 2020 household mix, node on a computer already on | 1117 | 993 | 482 | 10.9 | -501 | 0.000 |
| US 2020 household mix, no home node | 1117 | 8.3 | 408 | 10.7 | -501 | 0.000 |

## 8. Break-even: what decides the sign

The phone-radio inputs come first. Levers are fixed at what the edge does today, and their
zero is a design threshold. Then come the inputs whose zero lies inside their own interval
(these decide the sign), then the largest movers. Expected regime, stand-in presence. The full
table, including the long run and every scenario, is printed by `cewp-cli energy`.

| scenario | run | input | component | class | interval | delta at lo / hi | zero at | inside interval |
|---|---|---|---|---|---|---|---|---|
| phones only (home with a fixed line) | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 874 / 874 | 0.1061 | no |
| phones only (home with a fixed line) | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 1561 / 189 | 0.9690 | no |
| phones only (home with a fixed line) | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 112 / 906 | 0.5817 | no |
| phones only (home with a fixed line) | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 874 / 874 | 489.7785 | no |
| phones only (home with a fixed line) | short run | `phone.cell.tail_w` | A/B, C5 ad/tracker device+network | MODEL | [0.5, 1.5] W | 388 / 1256 | 0.0532 | no |
| phones only (home with a fixed line) | short run | `phone.cell.promotion_j` | A/B, C5 ad/tracker device+network | MODEL | [0.2, 1.5] J | 854 / 952 | none in range | no |
| phones only (home with a fixed line) | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 874 / 874 | 0.1124 | no |
| phones only (home with a fixed line) | short run | `datacentre.messaging.wh_per_person_hour` | C1 delivery | FRONTIER | [0.02, 0.5] Wh | 1234 / -926 | 0.2942 | yes |
| phones only (home with a fixed line) | short run | `hosted.seat.w` | A/B | MODEL | [60, 300] W | 654 / 1532 | none in range | no |
| phones only (home with a fixed line) | short run | `hosted.devices_per_seat` | A/B (count) | MODEL | [1000, 10000] devices | 1313 / 523 | none in range | no |
| phones only (home with a fixed line) | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 874 / 874 | 1.2482 | no |
| call relayed by a neighbour vs SFU | short run | `network.mobile.traffic_j_per_mb` | C1 delivery, mesh upstream (minus) | MODEL | [5, 25] J/MB | 3362 / 18654 | 0.6023 | no |
| call relayed by a neighbour vs SFU | short run | `datacentre.call.wh_per_participant_hour` | C1 delivery | FRONTIER | [0.1, 5] Wh | 10518 / 1698 | 5.9434 | no |
| call relayed by a neighbour vs SFU | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 8898 / 8898 | 1.1667 | no |
| village, one thin uplink | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 550 / 550 | none in range | no |
| village, one thin uplink | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 550 / 550 | none in range | no |
| village, one thin uplink | short run | `datacentre.messaging.wh_per_person_hour` | C1 delivery | FRONTIER | [0.02, 0.5] Wh | 910 / -1250 | 0.2222 | yes |
| village, one thin uplink | short run | `village.households` | A/B (count) | MODEL | [20, 200] households | 1630 / 10.0 | 211.7640 | no |
| village, one thin uplink | short run | `village.node.w` | A/B | MODEL | [5, 20] W | 190 / 1270 | 2.3611 | no |
| village, one thin uplink | short run | `network.thin_uplink.j_per_mb` | C1 delivery, C5 ad/tracker device+network, mesh upstream (minus) | FRONTIER | [5, 500] J/MB | 626 / -213 | 374.3978 | yes |
| village, one thin uplink | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 550 / 550 | 0.8786 | no |
| US 2020 household mix, node on gateway | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 2319 / 2319 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 2849 / 1791 | 1.7525 | no |
| US 2020 household mix, node on gateway | short run | `phone.wifi_share.cell_only_home` | A and C (count) | FRONTIER | [0, 0.3] share | 2362 / 2234 | 5.7028 | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 1430 / 2356 | none in range | no |
| US 2020 household mix, node on gateway | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 2319 / 2319 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.tail_w` | A/B, C5 ad/tracker device+network | MODEL | [0.5, 1.5] W | 1752 / 2765 | none in range | no |
| US 2020 household mix, node on gateway | short run | `phone.cell.promotion_j` | A/B, C5 ad/tracker device+network | MODEL | [0.2, 1.5] J | 2297 / 2410 | none in range | no |
| US 2020 household mix, node on gateway | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 2319 / 2319 | none in range | no |
| US 2020 household mix, node on gateway | short run | `home_node.on_router.marginal_w` | A/B | FRONTIER | [0.1, 1.5] W | 1487 / 5370 | none in range | no |
| US 2020 household mix, node on gateway | short run | `datacentre.messaging.wh_per_person_hour` | C1 delivery | FRONTIER | [0.02, 0.5] Wh | 2652 / 654 | 0.6573 | no |
| US 2020 household mix, node on gateway | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 2319 / 2319 | 2.6099 | no |
| US 2020 household mix, new box | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 22102 / 22102 | none in range | no |
| US 2020 household mix, new box | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 22737 / 21468 | 9.7206 | no |
| US 2020 household mix, new box | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 21397 / 22131 | none in range | no |
| US 2020 household mix, new box | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 22102 / 22102 | none in range | no |
| US 2020 household mix, new box | short run | `phone.cell.tail_w` | A/B, C5 ad/tracker device+network | MODEL | [0.5, 1.5] W | 21652 / 22455 | none in range | no |
| US 2020 household mix, new box | short run | `phone.cell.promotion_j` | A/B, C5 ad/tracker device+network | MODEL | [0.2, 1.5] J | 22084 / 22174 | none in range | no |
| US 2020 household mix, new box | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 22102 / 22102 | none in range | no |
| US 2020 household mix, new box | short run | `home_node.added_box.w` | A/B | MODEL | [3.3, 13.77] W | 13112 / 47972 | none in range | no |
| US 2020 household mix, new box | short run | `datacentre.messaging.wh_per_person_hour` | C1 delivery | FRONTIER | [0.02, 0.5] Wh | 22435 / 20437 | 5.4105 | no |
| US 2020 household mix, new box | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 22102 / 22102 | 20.7940 | no |
| US 2020 household mix, node on a computer already on | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 2101 / 2101 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 2631 / 1573 | 1.6488 | no |
| US 2020 household mix, node on a computer already on | short run | `phone.wifi_share.cell_only_home` | A and C (count) | FRONTIER | [0, 0.3] share | 2144 / 2016 | 5.1615 | no |
| US 2020 household mix, node on a computer already on | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 1212 / 2138 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 2101 / 2101 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `phone.cell.tail_w` | A/B, C5 ad/tracker device+network | MODEL | [0.5, 1.5] W | 1534 / 2547 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `phone.cell.promotion_j` | A/B, C5 ad/tracker device+network | MODEL | [0.2, 1.5] J | 2079 / 2192 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 2101 / 2101 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `home_node.on_computer.marginal_w` | A/B | FRONTIER | [0.2, 3] W | 1363 / 4809 | none in range | no |
| US 2020 household mix, node on a computer already on | short run | `datacentre.messaging.wh_per_person_hour` | C1 delivery | FRONTIER | [0.02, 0.5] Wh | 2434 / 437 | 0.6049 | no |
| US 2020 household mix, node on a computer already on | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 2101 / 2101 | 2.4116 | no |
| US 2020 household mix, no home node | short run | `lever.cell_round_share` | A and C (count) | MODEL | [1, 1] share | 1042 / 1042 | 0.0870 | no |
| US 2020 household mix, no home node | short run | `phone.wifi_share` | A and C (count) | MODEL | [0.4, 0.9] share | 1571 / 514 | 1.1442 | no |
| US 2020 household mix, no home node | short run | `phone.wifi_share.cell_only_home` | A and C (count) | FRONTIER | [0, 0.3] share | 1085 / 957 | 2.5758 | no |
| US 2020 household mix, no home node | short run | `phone.cell.tail_s` | A and C (count) | MODEL | [2, 12] s | 153 / 1079 | 0.3472 | no |
| US 2020 household mix, no home node | short run | `lever.round_cadence_s` | A and C (count) | MODEL | [30, 30] s | 1042 / 1042 | 759.5890 | no |
| US 2020 household mix, no home node | short run | `phone.cell.tail_w` | A/B, C5 ad/tracker device+network | MODEL | [0.5, 1.5] W | 475 / 1488 | 0.0317 | no |
| US 2020 household mix, no home node | short run | `phone.cell.promotion_j` | A/B, C5 ad/tracker device+network | MODEL | [0.2, 1.5] J | 1019 / 1133 | none in range | no |
| US 2020 household mix, no home node | short run | `lever.phone_schedule_share` | A and C (count) | MODEL | [1, 1] share | 1042 / 1042 | 0.0918 | no |
| US 2020 household mix, no home node | short run | `datacentre.messaging.wh_per_person_hour` | C1 delivery | FRONTIER | [0.02, 0.5] Wh | 1375 / -623 | 0.3504 | yes |
| US 2020 household mix, no home node | short run | `hosted.seat.w` | A/B | MODEL | [60, 300] W | 839 / 1651 | none in range | no |
| US 2020 household mix, no home node | short run | `hosted.devices_per_seat` | A/B (count) | MODEL | [1000, 10000] devices | 1448 / 717 | none in range | no |
| US 2020 household mix, no home node | short run | `c.substitution_share` | C2-C5 (count) | FRONTIER | [0.5, 0.5] share | 1042 / 1042 | 1.4481 | no |

### Findings (the sign and what decides it; nothing here is a saving or a cost claim)

Sections 7 and 8 are at substitution 0.5. The long-run break-even rows are printed by
`cewp-cli energy`.

* **At substitution 0 every household case is a cost** across its interval (section 4b). The
  mesh is purely additive there, and nothing upstream is avoided.
* **Short run (capacity fixed):** at substitution 0.5 no household case reaches a saving at the
  point. C1 delivery still decides wherever any single input decides (phones only:
  `datacentre.messaging` 0.29 Wh/person-hour; village: 0.22).
* **Long run:** **C2 retention** and **C3 harvesting** now decide the gateway and computer
  mixes. Break-evens: storage 7.7 W/TB, retained 11.6 GB per user-year, harvesting 4.5 Wh per
  user-hour (gateway mix). For phones-only and no-node cases many inputs decide: retained GB
  3.8, storage 2.5 W/TB, harvesting 0.85 Wh, ad J per impression 26, tracker wake-ups 49 an
  hour, use 3.4 h a day, and the phone radio (Wi-Fi share 0.70, tail 9.5 s).
* **A new always-on box** (on a line or on cellular) needs a substitution share of 7 or more to
  reach zero at the defaults. No possible share does that. It turns open only through the
  far ends of the C2-C4 intervals.
* **What would have to be true** for the gateway mix to be a saving in the long run: nearly all
  use moves (substitution ≥ 1.04 at the defaults, so not quite), or retention, harvesting or
  the ad chain cost more than their defaults. Examples: more than 7.7 W per TB held, more than
  11.6 GB per user-year, more than 4.5 Wh of harvesting per user-hour. None of these has a
  published figure (all FRONTIER).
* **The phone radio** is still the largest device term. The tracker wake-ups a platform app
  makes are now priced on the status-quo side at the same radio cost. They decide the sign at
  49 wake-ups an hour (phones only, long run).

## 9. Hooks (flat `Vec<f32>`, class codes as `seat_hooks::class`)

* `view_delta(seed)`: `DELTA_STRIDE = 16` per (scenario, regime, run): `[scenario, regime, run
  (0 short, 1 long), per (0 household-hour, 1 participant-hour), A, A_lo, A_hi, B, B_lo, B_hi,
  C, C_lo, C_hi, net, net_lo, net_hi]`. Scenarios are in `Scenario::ALL` order (0 gateway,
  1 computer, 2 new box, 3 phones only (fixed line), 4 phones only (cellular only), 5 box on
  cellular, 6 call via a neighbour, 7 call via a DC, 8 village, 9 mix gateway, 10 mix box,
  11 mix computer, 12 mix none). Regimes are 0 optimistic, 1 expected, 2 pessimistic,
  3 adversarial. 104 rows. All at substitution 0.5. The sign is a
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
