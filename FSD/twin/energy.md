# Twin package `theory`, part 2: the energy delta from the status quo

Code: `crates/cewp-sim/src/energy.rs`. CLI: `cewp-cli energy [seed]` (prints the report below).
Views: `view_delta`, `view_delta_class`, `view_break_even` (layouts in their doc comments).

## 1. What replaced what

`FEDERATION_SCALING_MODEL.md` §3.6 compared a gross device-fleet draw
(`examples/scale_model.rs`: users × device shares × idle watts × an assumed marginal share)
with a whole data-centre estate (10,000 facilities × 5 MW), and printed "4.8× less". It reads no
traffic, no locality and no presence. A typed constant `planetary_5b_power_mw = 3,082 MW
PROJECTED` sat in the evidence bundle with nothing computing it, and it contradicted the 10.3 GW
table. Both are now labelled as untested gross estimates. The constant is removed, and the CLI,
the evidence sample, `evidence_bundle.json` and the FSD example no longer carry it.

Eric's framing: "it is delta from status quo not gross total", "people use phones regardless".
The quantity is therefore, per household-hour (per participant-hour for a call):

    delta = A + B - C
    A  added on existing devices   extra draw on devices people already run (wake-ups and
                                   radio tails, radio bytes, signature work, the marginal draw
                                   of a node on a router or computer that is on anyway), net of
                                   the device work the status quo does for the same workload
    B  added devices               full draw of devices added or kept on FOR the mesh (a new
                                   always-on box, a village node, the household's share of a
                                   hosted seat or relay)
    C  avoided upstream (net)      data-centre and network energy of the workload that moves,
                                   minus the network energy of the mesh's own bytes

Positive is a cost, negative a saving. The model does not know the sign in advance. Every
result is reported as a point with its interval, and the break-even table names the inputs
that flip it.

## 2. Inputs (`ENERGY_SPECS`)

Every row has a device, a state, a unit, `[lo, mid, hi]`, a class, `would_be_on_anyway`, a year,
the measurement that would replace it, and its reasoning. No dataset stands behind any of them.
Each is MODEL or FRONTIER from stated reasoning, and the literature orders of magnitude are
quoted from memory, not re-read. The table is laid out so CIRISHolon's sourced table (per
class, per state, with the same flag and an "always-on devices a household already has" census
column) replaces it row for row by `name`.

| row | device | state | unit | lo / default / hi | class | on anyway | year | needs measurement on device | reasoning |
|---|---|---|---|---|---|---|---|---|---|
| `phone.deep_idle_w` | phone | deep idle (screen off, radio idle) | W | 0.008 / 0.015 / 0.03 | MODEL | yes | 2026 | - | order of a modern smartphone's suspended draw (tens of mW). Listed for completeness: the mesh does not change deep idle, so no term charges it |
| `phone.wake_j.wifi` | phone | one wake-up burst on Wi-Fi, radio tail included | J | 0.03 / 0.15 / 0.6 | FRONTIER | yes | 2026 | phone on a power monitor: J per edge round wake on Wi-Fi (screen off, Doze off and on), net of the idle baseline | Wi-Fi power-save tail of ~0.1 to 0.3 s at ~0.5 to 1 W plus an application-processor wake; no CIRIS device measured |
| `phone.wake_j.cellular` | phone | one wake-up burst on cellular, RRC tail included | J | 0.5 / 3.0 / 12.0 | FRONTIER | yes | 2026 | phone on a power monitor: J per edge round wake on LTE and 5G NSA, with the carrier's own inactivity timer and C-DRX | an LTE RRC inactivity tail of ~10 s at ~1 W (people_hooks RADIO_TAIL_S = 10 s, MODEL) is the high end; C-DRX and fast dormancy bring it down; no CIRIS device measured |
| `phone.j_per_mb.wifi` | phone | active radio, Wi-Fi | J/MB | 0.02 / 0.08 / 0.3 | MODEL | yes | 2026 | - | Wi-Fi transfer energy on a phone of the order of 0.1 J/MB at moderate rates, falling at high rates |
| `phone.j_per_mb.cellular` | phone | active radio, cellular | J/MB | 0.2 / 0.8 / 3.0 | MODEL | yes | 2026 | - | cellular transfer energy several times Wi-Fi's per byte (higher transmit power, scheduling) |
| `phone.j_per_sig_op` | phone | one hybrid sign or verify on the CPU | J | 0.0002 / 0.0008 / 0.003 | FRONTIER | yes | 2026 | phone on a power monitor: J per hybrid verify and per sign at the edge's batch sizes | hybrid verify 276 us MEASURED on an EPYC core (cewp-cli metrics); a phone core several times slower at ~1.5 W gives ~0.5 to 3 mJ |
| `phone.j_per_chunk_sig_op` | phone | one Ed25519 sign or verify of a media chunk | J | 0.00002 / 0.0001 / 0.0005 | FRONTIER | yes | 2026 | phone on a power monitor during a call, against the media engine's per-chunk sign and verify counts | media chunks are Ed25519 (media.rs NodeEnergy signs and verifies): tens of us on a phone core at ~1.5 W |
| `home_node.on_router.marginal_w` | home node | running on the household's existing router (marginal) | W | 0.1 / 0.4 / 1.5 | FRONTIER | yes | 2026 | router with and without the edge daemon at the twin's schedule, at the wall | a router is on anyway; the daemon adds CPU wake-ups and a little RAM traffic on a SoC that idles at a few W |
| `home_node.on_computer.marginal_w` | home node | running on a computer already on (marginal) | W | 0.2 / 0.8 / 3.0 | FRONTIER | yes | 2026 | desktop or NAS already on, with and without the edge daemon, at the wall | a desktop's package power rises by tenths of a W to a few W when a daemon keeps it out of its deepest idle states |
| `home_node.j_per_mb` | home node | bytes sent or received, relayed bytes included | J/MB | 0.005 / 0.02 / 0.1 | FRONTIER | yes | 2026 | home node at the wall forwarding a known byte stream for others (relay), J per MB in plus out | wired NIC and CPU per byte on small hardware; relayed bytes count twice (in and out) |
| `home_node.j_per_sig_op` | home node | one hybrid sign or verify | J | 0.0001 / 0.0004 / 0.002 | MODEL | yes | 2026 | - | the EPYC verify (276 us MEASURED) on a slower core at a few W |
| `home_node.j_per_chunk_sig_op` | home node | one Ed25519 sign or verify of a media chunk | J | 0.00001 / 0.00005 / 0.0003 | MODEL | yes | 2026 | - | Ed25519 verify of the order of 50 us on a desktop or SBC core at a few W |
| `home_node.added_box.w` | home node | a new always-on box bought or kept on for the mesh (full draw) | W | 2.5 / 5.0 / 12.0 | MODEL | no | 2026 | - | single-board computer at 2.5 to 5 W, mini-PC idle 6 to 12 W |
| `laptop.marginal_w` | laptop | running the edge while awake (marginal) | W | 0.1 / 0.5 / 1.5 | FRONTIER | yes | 2026 | laptop on battery with and without the daemon | listed for CIRISHolon's table; no scenario below puts the edge on a laptop |
| `hosted.seat.w` | hosted server | a serve seat, idle included | W | 60.0 / 120.0 / 300.0 | MODEL | no | 2026 | - | a one-socket rented server at the wall including its share of data-centre overhead (PUE) |
| `hosted.devices_per_seat` | hosted server | devices one seat serves | devices | 1000.0 / 2000.0 / 10000.0 | MODEL | n/a | 2026 | - | the page draws one seat per ~2,000 nodes (docs/globe.js buildWorld); the seat knee interval runs higher |
| `hosted.j_per_store_read` | hosted server | one counted store read | J | 0.001 / 0.01 / 0.05 | FRONTIER | no | 2026 | seat at the wall against its store-read counter over a window | a few ms of one core at ~5 to 10 W per core |
| `hosted.j_per_mb` | hosted server | relayed bytes on a hosted relay | J/MB | 0.01 / 0.05 / 0.3 | FRONTIER | no | 2026 | hosted relay at the wall against bytes forwarded | server NIC, CPU and seal work per byte with PUE |
| `network.access_fixed.j_per_mb` | network | fixed access (one end, one direction) | J/MB | 0.0 / 5.0 / 100.0 | FRONTIER | n/a | 2026 | - | the decisive unknown: an attributional average of ~0.03 kWh/GB (~100 J/MB) in studies of the late 2010s, against a marginal near zero because fixed access power barely varies with traffic; 5 J/MB is a placeholder between them |
| `network.access_mobile.j_per_mb` | network | mobile access (one end, one direction) | J/MB | 5.0 / 50.0 / 500.0 | FRONTIER | n/a | 2026 | - | attributional averages of ~0.1 to 0.2 kWh/GB for mobile networks around 2020 (360 to 720 J/MB), marginal much lower |
| `network.core.j_per_mb_per_as` | network | per AS crossed (routers and metro transport) | J/MB | 0.005 / 0.05 / 0.3 | FRONTIER | n/a | 2026 | - | a few routers per AS at ~1 to 10 nJ/bit each |
| `network.long_haul.j_per_mb_per_1000km` | network | per 1,000 km of long-haul | J/MB | 0.002 / 0.02 / 0.1 | FRONTIER | n/a | 2026 | - | optical amplifiers and regenerators per km per byte, small against routers |
| `network.thin_uplink.j_per_mb` | network | a village's one thin uplink (satellite or long microwave) | J/MB | 5.0 / 50.0 / 500.0 | FRONTIER | n/a | 2026 | - | a satellite terminal's 50 to 100 W over a few Mbit/s of use, or a solar microwave hop; per byte far above urban access |
| `datacentre.messaging.wh_per_person_hour` | data centre | messaging and social workload one person drives, per hour of the day | Wh | 0.02 / 0.1 / 0.5 | FRONTIER | n/a | 2026 | - | public operator data-centre electricity over monthly users is of the order of a few Wh per user-day (quoted from memory, not re-read); the share that is messaging the mesh replaces is unknown |
| `datacentre.call.wh_per_participant_hour` | data centre | SFU work per participant-hour of a video call | Wh | 0.1 / 1.0 / 5.0 | FRONTIER | n/a | 2026 | - | an SFU forwards without transcoding; server share per participant small, the high end includes recording and transcoding |
| `village.node.w` | village node | one added node with its radio and solar controller (full draw) | W | 5.0 / 10.0 / 20.0 | MODEL | no | 2026 | - | a small server plus a Wi-Fi or mesh radio |
| `village.households` | workload | households behind one village node and uplink | households | 20.0 / 50.0 / 200.0 | MODEL | n/a | 2026 | - | a village of a few hundred people |
| `household.persons` | workload | persons (one phone each) | persons | 2.0 / 2.5 / 3.0 | MODEL | n/a | 2026 | - | micro.rs household rule round(2.5 + 1.1 N(0,1)) clamped to 1..6 |
| `phone.wifi_share` | workload | share of a phone's online hours on Wi-Fi | share | 0.5 / 0.7 / 0.85 | FRONTIER | n/a | 2026 | - | CIRISHolon's attach.phone.network rhythm (168 bins) replaces it |
| `workload.bytes_per_act` | workload | wire bytes of one act's content copy | bytes | 1000.0 / 5000.0 / 50000.0 | MODEL | n/a | 2026 | - | a signed text act with a thumbnail; media-heavy acts run higher |
| `phone.mesh_extra_content_wakes` | workload | extra content wake-ups under the mesh, as a share of the status quo's push wake-ups | share | -0.3 / 0.0 / 0.3 | FRONTIER | n/a | 2026 | phone wake log under the mesh and under a push-notification app for one scripted day | mesh content arrives on a live link, push on the OS channel; whether batching differs is unknown, so zero with a symmetric band |
| `phone.schedule_share` | workload | share of the edge round and Pull schedule a phone runs itself | share | 1.0 / 1.0 / 1.0 | MODEL | n/a | 2026 | - | the edge at 8e46165 has no host-kind branch (seat_hooks), so a phone runs the whole schedule; a phone that left its rounds to its home node would run less (a design lever: see the break-even table) |
| `phone.keepalive_bytes_per_mapping_hour` | workload | bytes to hold one NAT mapping for an hour | bytes | 0.0 / 2000.0 / 20000.0 | FRONTIER | n/a | 2026 | packet capture of a phone holding punched paths | keepalives ride the 30 s round wake (shorter than any NAT timeout), so they add bytes, not wake-ups |

Rows marked **needs measurement on device** are ours to measure on real hardware:

* `phone.wake_j.wifi` and `phone.wake_j.cellular`: put a phone on a power monitor and measure
  J per edge round wake, net of the idle baseline, with the screen off, Doze on and off, on
  Wi-Fi, on LTE and on 5G NSA, under the carrier's own inactivity timer and C-DRX.
* `home_node.j_per_mb`: J per MB a home node forwards for others, in and out, at the wall,
  against a known byte stream.
* `phone.j_per_sig_op` and `phone.j_per_chunk_sig_op`: J per hybrid sign/verify and per
  Ed25519 chunk sign/verify on a phone.
* `home_node.on_router.marginal_w` and `home_node.on_computer.marginal_w`: the wall draw of the
  router or computer with and without the daemon at the twin's schedule.
* `hosted.j_per_store_read` and `hosted.j_per_mb`: the seat at the wall against its store-read
  and forwarded-byte counters.
* `phone.mesh_extra_content_wakes` and `phone.keepalive_bytes_per_mapping_hour`: a scripted
  day's wake log and packet capture under the mesh and under a push-notification app.

## 3. Counters read (DERIVED from the packages' events)

| counter | source | value (seed 1) |
|---|---|---|
| radio wake-ups per device-hour (edge rounds and Pull cycles) | `seat_hooks::device_counters(Phone, unclaimed)` | 126.75 (the same for every host kind: the edge at 8e46165 has no host-kind branch) |
| sync bytes per device-hour, each way | same | 76,049 |
| seat store reads per device-hour | same | 86.4 |
| acts signed / copies received per person-hour | `people_hooks::energy`, Poisson, 2,000-person stand-in world, 1 day | 7.51 / 158.9 |
| content wake-ups per phone-hour (radio tail 10 s MODEL) | same | 93.2 |
| relayed share of pairs: fixed / phone Wi-Fi / phone cellular | `reach::hooks::energy_counters`, default model, correlated | 0.254 / 0.213 / 0.517 |
| NAT mappings held per phone | same | 0.77 |
| ASes, IXPs, long-haul km: same-metro peer / any peer / to the nearest data centre | `netgraph` stand-in (seed 1), `sample_fixed_pairs` and routed paths to the nearest content network | 3.00, 0.35, 0 / 3.94, 0.17, 9,781 / 2.83, 0.72, 1,711 |
| call MB per participant-hour: own tx / forwarded / rx (fixed, Wi-Fi, cellular) | `media::energy_by_class` of `scenario_call4` (busy, tail-drop) | 1,680 / 1,892 / 2,137; 1,274 / 788 / 2,266; 1,362 / 0 / 2,257 |
| chunk signs and verifies per participant-hour | same | ~5.5 M to 5.8 M |

**Missing counters.** Nothing invented. Each gap is listed with what stands in for it.

* **Bytes forwarded for others, per device.** No package counts it outside a call. `reach`
  counts relayed pairs, not bytes, and media counts forwarded bytes only inside a call. The
  delta estimates a node's relayed bytes as relayed share × the household's content copies ×
  the relay offering share. That is an inference, marked as such in its ledger line. Owed: a
  per-device relayed-bytes counter in `reach/hooks.rs` (or `micro.rs`), split by link class.
* **Storage writes and byte-seconds held.** `durability.rs` gives bytes held per node kind for
  one drawn holder set, with no time axis. Storage is not priced, because no counter gives
  writes per device-hour.
* **Signature operations per device outside calls.** `people_hooks` counts one sign per act and
  one verify per copy received. The seat schedule's own signature work (round Summaries, Pull
  replies) is not counted. The delta prices only the people_hooks signs and verifies.
* **The status quo's own device wake-ups for the same workload.** Content wake-ups are assumed
  equal under the mesh and under push, and the difference is a FRONTIER row in [-0.3, 0.3]. A
  push app's heartbeat is not counted on either side, because the phone keeps it for other apps.
* **Seat egress per device on the path.** The delta prices the device's own sync bytes over
  access and the data-centre path. It does not price the seat's internal reads beyond
  `hosted.j_per_store_read`.

## 4. Scenarios

| scenario | per | A | B | C |
|---|---|---|---|---|
| home node on existing router | household-hour | phones' schedule wake-ups, sync and keepalive bytes, act signatures; router marginal W; node sync and relayed bytes | household's share of a hosted seat (W and store reads) | DC messaging per person-hour; status-quo content over access + DC path, less mesh content over access (both ends) + peer paths (local share by regime) and mesh sync to the seat |
| home node on computer already on | same | computer marginal W instead of the router's | same | same |
| home node on a new always-on box | same | phones and node bytes | the box's full draw + seat share | same |
| phones only | same | phones | seat share + relayed bytes on a hosted relay (the part neighbours do not carry) | same |
| call relayed by a neighbour vs SFU | participant-hour | mesh call bytes (own + forwarded + received) less the SFU call's (1 up + N-1 down) per class; chunk signatures | none | SFU Wh; SFU bytes over access + DC path, less mesh bytes over both access ends + peer paths |
| call relayed in a data centre vs SFU | participant-hour | one upload + received per class less the SFU's; chunk signatures | the hosted relay, priced as the SFU | the SFU it replaces; bytes cancel (same shape) |
| village, one thin uplink | household-hour | phones on village Wi-Fi | seat share + the household's share of one added village node | DC messaging; status-quo content over the uplink, less the mesh's non-local share over the uplink and its sync |

Regimes (FSD §4.4, `workload.rs`) set the phone online share (1 - `mobile_offline_churn`), the local share of copies (`locality_share`: the same metro or, in the village, the village LAN) and the share of relayed copies a neighbour carries (`relay_offering_share`).

## 5. Results

The full report is `cargo run -p cewp-cli -- energy 1` (also `energy::tests::print_report`).
The expected regime is shown. Every result is FRONTIER, because it is no stronger than its
weakest row.

RESULTS_TABLE

Each bound is the lowest or highest value found over every row's interval ends. The search
starts from seat_calibration's corners and refines them by a vertex search, so each bound is
attained by a real combination of input ends. The interval-arithmetic enclosure, guaranteed to
contain every combination, is wider and is printed beside each bound.

By device class (expected regime, J per unit, as each enters the delta):

BYDEV_TABLE

## 6. Break-even: what decides the sign

Each row holds the other inputs at their defaults and reports the value of one input where the
delta is zero. A zero inside the input's own interval means that input decides the sign.

BREAK_EVEN_TABLE

FINDINGS_TEXT

## 7. Validity envelope: what this model cannot say

* No figure is MEASURED, and no CIRIS device has been on a power monitor. The sign is open in
  every scenario except the added box. The interval says how open, and the break-even table says
  which measurement would close it.
* The network rows are attributional averages and marginal near-zero figures in one interval.
  Which one applies is a modelling choice the literature has not settled, and it moves the call
  scenarios by an order of magnitude.
* Workloads are the twin's: the Poisson act process of the 2,000-person stand-in world, the
  seat schedule of edge release/45.0.0 at 8e46165, and the media engine's 60 s four-person call.
  The delta is per household-hour or per participant-hour, not a planetary total. Nothing here
  multiplies up to watts per country.
* "Avoided" data-centre energy assumes the mesh replaces the whole comparable workload
  (messaging, or an SFU call). A household that keeps its old apps beside the mesh avoids
  nothing, and `datacentre.*` at its low end is that case.
* The village is one generic thin uplink. No village dataset was read (registry row
  `settlement.thin_uplink`, wanted).
* Paths come from the netgraph stand-in (MODEL), reach outcomes from the reach model (MODEL
  marginals, FRONTIER CGNAT).

## 8. Hooks

Views (flat `Vec<f32>`, class codes as `seat_hooks::class`):

* `view_delta(seed)`: `DELTA_STRIDE = 15` f32 per (scenario, regime): `[scenario, regime, per,
  A, A_lo, A_hi, B, B_lo, B_hi, C, C_lo, C_hi, net, net_lo, net_hi]`. Scenarios are in
  `Scenario::ALL` order (0 router node, 1 computer node, 2 added box, 3 phones only, 4 call via a
  neighbour, 5 call via a data centre, 6 village). Regimes are in `REGIMES` order (0 optimistic,
  1 expected, 2 pessimistic, 3 adversarial). `per` is 0 for a household-hour and 1 for a
  participant-hour. The sign is a cost if `net_lo > 0`, a saving if `net_hi < 0`, and open
  otherwise.
* `view_delta_class(seed)`: one class code per row of `view_delta`.
* `view_break_even(seed)`: `BREAK_EVEN_STRIDE = 8` f32 per (scenario, input) at the expected
  regime: `[scenario, row index into ENERGY_SPECS, class, lo, hi, net_at_lo, net_at_hi, zero]`.
  `zero` is NaN when no zero lies in [0, 100 × hi]. A row is deciding when `lo <= zero <= hi`.
