# FSD (twin package `media`): link contention and call quality

Status: package draft on branch `twin/media`; integration folds pointers into
`FSD/SIMULATION_ENGINE.md` §3.1/§3.2. Code: `crates/cewp-sim/src/link.rs` (access links),
`media.rs` (call traffic, controller, scenarios, report), `qoe.rs` (metrics). CLI:
`cewp-cli media [SEEDS]`.

Answers outside-review items 3 ("model real bandwidth contention") and 8 ("model real-time
media quality"). Every number carries an FSD §8.2 class: `DERIVED` (read off CIRISEdge
`8e46165` = v45, or its leviculum pin `v0.34.0+ciris.3`, symbol named), `MODEL` (a stated
choice), `FRONTIER` (an open number; placeholder). Every result of the engine is `MODEL`
output.

## 1. Why

`transport.rs` enforces the constitutional budget classes (CC 5.4.3) and `alm.rs` builds the
deterministic tree, but both treat a node that offers a rate as having it. A household's
uplink is shared with its own traffic, sits behind a buffer sized by the ISP, crosses Wi-Fi
airtime or a cellular scheduler, and loses packets in bursts. This package puts a physical
link under the budgets and measures what a person in a room call experiences on it.

## 2. The link model (`link.rs`)

One `LinkDir` per direction per participant (uplink, downlink). Integer microseconds, integer
bytes, splitmix64 seeded per node.

| element | model | class |
|---|---|---|
| capacity | nominal rate × Wi-Fi airtime share (permille) × cellular level | MODEL |
| cellular | Markov chain over {100, 60, 30, 10} % of nominal, a step every 500 ms, stay 0.9 | MODEL |
| queue | one bottleneck FIFO per direction, buffer = `buffer_ms` at the nominal rate | MODEL |
| discipline | tail-drop FIFO; AQM variant CoDel (RFC 8289 control law, 5 ms / 100 ms) | MODEL (RFC defaults) |
| medium loss | Gilbert-Elliott, stepped per packet: (p_gb, p_bg, loss_good, loss_bad) in ppm | MODEL |
| path MTU | an RNS packet over `path_mtu − 28` rides several IP fragments; each is a GE step | MODEL |
| household traffic | transfers arrive Poisson (mean gap), small or large; downlink adds a periodic 2 MB fetch every 4 s (a video player); each transfer is a Reno AIMD flow (IW 10, slow start, halve once per RTT, lost bytes resent) | MODEL |
| core | one-way 5 ms same region, 40 ms across, plus each end's last mile | MODEL |

Conservation, per link direction, at every instant:
`offered = tail_drop + aqm_drop + air_loss + delivered + flushed + queued + in_service`
(`LinkDir::conservation_residual() == 0`), and per packet over a whole run every created
packet ends in exactly one bucket of `media::Ledger` or is still live.

Access profiles (all MODEL; the rates are typical of the class, not of any measured network):

| profile | up / down | buffer up / down | last mile | airtime | loss | relays |
|---|---|---|---|---|---|---|
| fibre-home (wired desktop) | 50 / 300 Mbit/s | 50 / 50 ms | 2 ms | 1.0 | wired GE | yes |
| cable-home-wifi (laptop) | 10 / 150 Mbit/s | 300 / 100 ms | 8 ms | 0.6 | Wi-Fi GE | yes |
| dsl-home (wired) | 1 / 16 Mbit/s | 500 / 200 ms | 12 ms | 1.0 | wired GE | yes |
| phone-home-wifi | 10 / 150 Mbit/s | 300 / 100 ms | 8 ms | 0.5 | Wi-Fi GE | no (not charging) |
| phone-lte | 6 / 25 Mbit/s, varying | 1000 / 500 ms | 25 ms | 1.0 | LTE GE | no (metered) |

GE parameters (ppm: p_gb, p_bg, loss_good, loss_bad; stationary loss in brackets):
wired 100, 500k, 10, 300k [~0.007 %]; Wi-Fi 3k, 250k, 500, 250k [~0.35 %];
LTE 1k, 100k, 1k, 300k [~0.40 %]. Relay policy per CC 5.4.6 item 1 (edge
`onion/policy.rs` `device_relays`): homes yes, phones only while charging and unmetered.

## 3. The call, derived from edge v45

### 3.1 Wire (all DERIVED)

| quantity | value | source |
|---|---|---|
| RNS MTU | 500 B | leviculum `constants.rs` `MTU` |
| link MDU | 431 B | leviculum `link/mod.rs` `compute_link_mdu`: ⌊(500 − 1 − 19 − 48)/16⌋·16 − 1 |
| datagram payload | 427 B | leviculum `link_datagram_mdu` = MDU − `DATAGRAM_HEADER_LEN` 4 |
| media limit | 402 B | edge `reticulum.rs` `link_datagram_message_limit` = 427 − `SEAL_OVERHEAD` 25 (`link_seal.rs`: 9 + 16) |
| frame budget | 246 B | edge `av_call.rs` `LegInfo::frame_budget` = 402 − `AV_CHUNK_WIRE_OVERHEAD` 84 (`realtime_av.rs`: 48 + 4 + 2·16) − `CHUNK_TRAILER_LEN` 72 (`av_chunk_auth.rs`: 8 + 64) |
| fragment payload | 238 B | edge `av_call/frame.rs` `split`: budget − `FRAGMENT_MARKER_LEN` 8 |
| RNS packet of a full chunk | 499 B | header 19 + token (IV 16 + PKCS7 ⌈(431+1)/16⌉·16 + HMAC 32) |
| on the wire with IPv4/UDP | 527 B | + 28 (MODEL: IPv4) |
| fragment cap | 4096 fragments, 1 MiB | `frame.rs` `MAX_FRAME_FRAGMENTS`, `MAX_FRAME_BYTES` (at 238 B a frame over 974,848 B is refused) |

So at the base MTU a full video datagram carries 238 B of codec payload in 527 B on the wire
(45 %); a 60 B Opus frame (24 kbit/s, 20 ms) rides a 351 B datagram (17 %): 24 kbit/s of
audio is ~140 kbit/s per copy on the uplink.

### 3.2 What edge does with a chunk (DERIVED)

- Datagrams: leviculum#102 `send_unordered`: "no window, no proof, no retransmit". The only
  local refusal is the interface gate (`PacingDelay` from `next_slot_ms_for_interface`), whose
  turnaround defaults to 0 (`traits.rs` `frame_turnaround_ms`) and no leviculum-std interface
  overrides it, so on a UDP interface a datagram is never refused locally.
- One Ed25519 signature per chunk (`av_chunk_auth.rs` trailer), verified by every relay before
  it forwards and by every receiver (`av_call.rs` module doc). The engine runs one CPU queue
  per node for signing and verifying; media older than `MEDIA_STALE_BUDGET` (800 ms) in that
  queue is dropped.
- Audio and video share one `chunk_seq` per stream (`frame.rs` module doc); the receiver's
  `ReorderWindow` releases in `chunk_seq` order, waits for a gap up to
  `clamp(2·gap_ewma, 40, 400) ms` (prior 50 ms, fast attack / slow decay from duplicate copies),
  then skips it. A frame is released only whole (`FrameRefusal::Incomplete`).
- Video class `0x04` (`realtime_av.rs` `CODEC_TEMPORAL_LAYERED`, H.264): one spatial layer,
  temporal layers t0 (base) and higher. There is no simulcast and no keyframe request in
  edge v45; a lost base frame is "a freeze until the next keyframe" (`av_call.rs`
  `frames_lost_by_layer`).
- Layer caps ride SUBSCRIBE (`Call::request_layers`), "the host's lever, off by default".
- Relays forward a stream once per chunk (`FORWARD_DEDUPE`); a standby child gets base-layer
  and audio chunks only (`av_call/alm.rs` module doc).

### 3.3 The controller (DERIVED, simplified)

From `av_call/alm.rs` and `realtime_av_alm/controller/{admission,selector,peer_state}.rs`
with `ControllerConfig::interactive()` and the call's overrides
(`idle_aware_silence = true`, `direct_first = true`):

- Each receiver asks for a stream after its legs are up (MODEL: `SETUP_RTTS` = 4 round trips
  for link request, transit KEX, hello, binding parts) plus `U(0, join_jitter 1 s)`.
- Candidates: the publisher first when it can reach the receiver (direct first, R1), then
  rooted member relays by (depth, RTT, id). Max depth 3, no cycles.
- Admission: `committed ≤ floor + probe` in stream units (a primary costs one stream, a
  standby its audio + base share). The publisher's probe starts at `clamp(members − 1, 1, 8)`
  (`PUBLISHER_INITIAL_FANOUT`), a relay's at one stream (trial slot). After two clean windows
  with demand (a refusal) or with the allowance 80 % used, the probe doubles, to at most 16
  streams; `floor` rises to what the forwarder sends. Refused receivers back off
  `0.5 s · 2^k · U(0.5, 1.5)`, at most 8 s.
- **No ceiling is ever learned on datagram legs.** `path_meter.rs` `EgressMeter` ("Limit:
  datagram legs"): a datagram leg counts what it accepted as delivered, and the verdict needs
  refused frames, which a UDP interface never produces (§3.2). So `floor + probe` grows on
  demand without bound; the engine reproduces this and offers the counterfactual
  `delivery_report` (CIRISEdge#975: receivers report loss; three windows ≥ 5 % loss set a
  ceiling of 0.75 × delivered and shed LIFO).
- One warm standby per receiver per stream (audio + base), not in the primary's subtree.
- Failover: a parent leaving with a signed end closes its links; children see it on their next
  `WATCH_TICK` (≤ 250 ms). A parent that vanishes is seen after
  `suspect 1.5 s × (2 + depth)/2` of silence when another path still delivers the stream
  (selector's silence scaling); with no other path, `idle_aware_silence` reads the silence as
  an idle stream, and only the link's stale timeout ends it (leviculum: keepalive
  `clamp(RTT × 360/1.75, 5 s, 360 s)`, stale after 2 × keepalive + 5 s grace; the stale path's
  exact trigger was not traced: FRONTIER). The standby is promoted first.
- A voluntary move off a degraded primary (window loss ≥ 5 % or mean latency > 400 ms) needs
  three consecutive degraded windows and the dwell `10 s × 2^k` (k = voluntary moves in the
  last 120 s); a relay child returns to the publisher when it has room and the child did not
  leave it for degradation within 120 s. Moves are make-before-break.

What is simplified relative to edge (MODEL): no score weights (delivery/headroom/latency/
stability) beyond the candidate order; no C5 per-leg shed (it needs a local refusal, which
UDP does not give); no provisional sending before SUBSCRIBE; no epoch rolls; a joiner's
first window waits from `chunk_seq` 0 (as edge does) and skips.

### 3.4 Media (MODEL, edge sets none of it)

Opus 24 kbit/s in 20 ms frames (60 B, one chunk, no marker). H.264 500 kbit/s at 15 fps,
two temporal layers alternating (t1 non-reference), IDR every 2 s; IDR = 8 × a t0 P-frame,
t1 = 0.7 × t0, ±20 % per frame. At these settings a t0 P-frame is ~3.8 KB (17 fragments), a
t1 ~2.7 KB (12), an IDR ~31 KB (130); the base layer (t0 incl. IDRs) is 69 % of the video
bytes, so a warm standby (audio + base) costs 0.69 of a full stream. Capture + encode 50 ms, decode + render 30 ms. Ed25519
verify 60 µs desktop / 150 µs phone, sign 25 / 60 µs (FRONTIER: a release-build measurement
on a phone core is requested).

## 4. Quality of experience (`qoe.rs`)

Per (stream, receiver) session; distributions are p50/p95/p99 over sessions pooled over
seeds. Definitions are in the `qoe.rs` module doc: join time to the first shown frame; glass-
to-glass by depth; frame loss by type (a frame is lost if any fragment is); freezes by the
W3C webrtc-stats rule `gap > max(3·interval, interval + 150 ms)`; audio missing (lost or
later than 400 ms) and audible gaps (runs ≥ 2 frames); share of one-second windows at the
requested layer; relay recovery (audio gap, video gap, time to the full layer).

## 5. Stated bounds the tests hold

- Ample, lossless links: no frame lost, no freeze, every window at the requested layer; each
  frame's glass-to-glass equals `capture_encode + sign + tx_up(chunk₀) + prop + tx_down(chunk₀)
  + fragments × verify + decode_render` exactly (verify paces the receiver because
  verify > sign + serialisation). Audio: the same with one verify.
- Independent loss p: frames of n fragments are lost at `1 − (1 − p)^n` (per bucket within
  four standard deviations).
- A relay leaving with a signed end, child with a warm standby: audio gap ≤ 20 ms + 400 ms
  (reorder max); base video gap ≤ 2 frames + 400 ms; full layer back within
  `WATCH_TICK + RTT + 2 frames + 400 ms + decode_render`.
- The same without a standby: audio gap ≤ `WATCH_TICK + RTT + 20 ms + 400 ms`; video gap
  ≤ `WATCH_TICK + RTT + GOP + 2 frames + 400 ms + decode_render`, and longer than two frames
  (the broken chain waits for the next IDR).

## 6. What this model cannot say (validity envelope)

- No real codec: frame sizes are a parametric GOP, not rate-distortion output; no encoder rate
  control reacts to loss or to the network (edge has none; the host's encoder may).
- No real congestion controller for media: edge v45 has none on datagram legs beyond the
  admission probe; the engine's counterfactual #975 is a sketch, not edge code.
- Household TCP is Reno-like: no CUBIC curve, no SACK, no RTO, no pacing, no delayed ACKs.
- No measured Wi-Fi: airtime share and GE loss are constants; no rate adaptation, no
  contention among the household's own stations, no uplink/downlink airtime coupling.
- No cellular scheduler: a Markov capacity level stands in for proportional-fair scheduling.
- No core congestion, no peering bottleneck, no shared upstream (each household is its own
  link; two participants in one home are modelled as two homes).
- No NAT model: reachability is a per-node flag (`unreachable`); the joint distribution is the
  `nat` package's.
- Controller simplifications listed in §3.3. The live tree is the closed-loop controller, not
  `alm.rs`'s deterministic topology (see §8).
- A returning phone is a new participant (no rejoin of the same node).

## 7. Data that would replace MODEL parameters (approval requests, nothing downloaded)

See the package report for names, URLs, licences and sizes. In short: FCC Measuring
Broadband America raw data (access rates, latency under load, loss), the MONROE / 5Gophers
style cellular traces (capacity over time), and published Wi-Fi airtime/loss traces. Each
would drop in behind `AccessProfile` / `CellModel` / `GeParams`.

## 8. Findings (`cewp-cli media 5`: MODEL output, 5 seeds, p50/p95/p99 over sessions)

Conditions: `quiet` = no household traffic, `busy` = household traffic on; `fifo` / `codel`
= queue discipline; `#975` = the delivery-report counterfactual. Every run conserved bytes per
link and packets per ledger.

1. **Contention on vs off is second-order here.** In call4, busy vs quiet moves audio-missing
   p50 from 333 to 434 permille and g2g p95 from 866 to 1175 ms; the DSL+backup call moves
   audio latency p95 from 1.5 s to 2.7 s; broadcast and room20 do not move. The sensitivity
   block (call4, busy) shows why: with medium loss and cellular dips off, the busy call is
   near perfect (audio missing p50 0, every window at the requested layer, freeze p50
   4.3 s/min from IDR serialisation), so household traffic alone (at these MODEL rates) is not
   what breaks the call. What breaks it is how edge's media structure meets ordinary loss.
2. **No keyframe recovery turns small chunk loss into long freezes.** With no keyframe
   request and no resend, a lost IDR (130 fragments) or any lost t0 P-frame (17) freezes the
   base chain until the next IDR. At ~0.4 % chunk loss (a Wi-Fi receiver of a fibre
   publisher) a session lost 8 of 28 IDRs and could not decode 350 complete frames: half the
   call frozen. The closed form `1 − (1 − p)^n` holds under independent loss (tested); in the
   scenarios loss is burst- and size-correlated, so measured frame loss by bucket sits well
   below the pooled closed form while IDRs still lose 30–97 %.
3. **IDR fan-out bursts overflow a shallow buffer (broadcast).** The publisher signs a
   130-fragment IDR in ~3 ms and copies it to every child at once: 24 × 130 × 527 B = 1.6 MB
   into a 50 ms (312 KB) fibre buffer. IDR loss 965 permille; 114 of 120 viewer sessions
   never show a frame, while P-frame loss stays under 4 %. Edge has no media pacing on a UDP
   interface (§3.2), so this is the model's reading of edge, conditional on the MODEL buffer.
4. **Admission never learns the uplink on datagram legs; fan-out follows demand.** Broadcast
   fan-out reaches all 24 viewers direct; room20 publishers reach 18–19 children each (full
   mesh), the room collapses (1787 of 1900 sessions never show video, audio missing 965
   permille). The #975 counterfactual caps broadcast fan-out at 4 (no-video sessions 114 →
   24 of 120) and room20's at 4 (no-video 1800 → 1258), at the price of long joins.
5. **The warm standby is expensive.** Standby = audio + base = 0.69 of a stream at this GOP;
   relaying standbys overloads 6 Mbit/s Wi-Fi uplinks. call4 without standbys: audio missing
   p50 434 → 17 permille, chunk loss p95 435 → 118.
6. **MTU is the largest single lever.** RNS MTU 1400 (frame budget 1142 B): IDR 28 fragments,
   call4 freeze p50 39 → 9 s/min, IDR loss p50 357 → 35, layer windows at request p50 428 →
   910 permille.
7. **CoDel trades IDRs for latency.** call4: g2g p95 1175 → 563 ms, audio latency p95
   1717 → 613 ms, but IDR loss p50 357 → 535 and join p95 4.4 → 16 s (the AQM's drops land on
   the large bursts).
8. **Relay recovery under load is far from the unloaded bound.** Churn, busy: 36 relay
   departures, 33 with a standby; audio gap p50 1.2 s, p95 20 s; video gap p50 10.6 s, p95
   46 s; none never resumed. (Regenerated after the ended-stream fix in 564d756: before it, a
   departed publisher's relays kept stale legs and their later departures were counted as
   recoveries of a dead stream, which gave 47 departures and 10 never resumed.) Unloaded,
   the tested bound holds (§5).
9. **Wire efficiency at the base MTU**: video 238/527 B (45 %), Opus 60/351 B (17 %); a
   24 kbit/s voice copy is ~140 kbit/s on the uplink. A frame over 974,848 B (4096 × 238) is
   refused at the base MTU although `MAX_FRAME_BYTES` is 1 MiB.

Inferences from code reading the engine reproduces but that edge tests should confirm: the
reorder window waits a full window at every `chunk_seq` gap a layer cap or a standby leg
creates (filtered t1 chunks are gaps, and audio shares the sequence); a vanished parent with
no other active path is detected only by the link's stale timeout under `idle_aware_silence`.

## 9. Energy counters (`HOOKS.md` §1)

`RunResult::energy` holds one `NodeEnergy` per participant and `RunResult::devices` its
(device, medium); `energy_by_class` sums them per (`micro.rs` `HostKind`, `LinkClass`). Device
classes used: fibre and DSL homes are `HomeNode`, the cable-home laptop is `Laptop`, both
phones are `Phone`; media `Fixed` (fibre, DSL), `WiFi` (cable-home laptop, phone at home),
`Cellular` (phone on LTE). Every count is DERIVED from the engine's own events; no power
figure is attached (the energy package owns device wattage). `cewp-cli media` prints them per
participant-hour (`energy_lines`).

| Count | Where the engine takes it |
|---|---|
| own / relay tx bytes and copies | a media copy leaving the uplink (lost on the medium or not); relay = another publisher's stream |
| media rx bytes, copies, duplicates | a copy arriving whole over the downlink; duplicate = a chunk already held (primary and standby both sending) |
| household tx / rx bytes | background TCP segments on the same links (contention on only) |
| signs | one Ed25519 per published chunk (`av_chunk_auth.rs`) |
| verifies | one per copy reaching the CPU, at every relay and receiver (copies dropped stale are not verified) |
| inner seals / opens | epoch-key AEAD once per chunk at the publisher; opened once per unique chunk at a node that plays the stream (`realtime_av.rs` "inner-once / outer-N"; that a non-playing relay never opens the inner layer is an inference) |
| hop seal / open copies and bytes | per copy, three encryptions: the per-link outer AEAD (`seal_av_outer`), the E1 link seal (`link_seal.rs`), the RNS link token; bytes = each layer's output (plain + 84; + 72 + 25; + 4) |
| cpu busy | sign + verify time |
| up / down busy, wakes, idle-gap histogram | link transmit time counted when a packet goes on the wire; a wake = idle to busy; the gap before each wake in buckets < 10 ms, < 100 ms, < 1 s, < 10 s, >= 10 s (or first), so a radio tail can be priced later without a re-run |
| present | join to leave or the run's end |

Not counted, because the engine has no such event: control traffic (SUBSCRIBE, C3 frames,
bindings, RNS keepalives, link setup), storage (a call writes nothing), AS hops and long-haul
kilometres (the media engine has two regions, not routes; `twin-topology`'s
`NetPath::counters` is the type to join on once members are placed on its graph). Bytes
transmitted from a node's queue when it leaves are flushed, not counted as sent; the copy on
the wire at that moment had its seal counted but not its transmission. The energy test
(`energy_counts_balance_on_a_relayed_lossless_call`) holds: copies sent = copies received =
verifies = hop opens, hop seals = sent + flushed, inner opens + duplicates = verifies per
player, class sums = node sums.

## 10. The globe's view (`media_view.rs`, `HOOKS.md` §2)

Plain functions returning flat `Vec<f32>` / `Vec<u32>` for `kernel-wasm` (owned by
the globe package); layouts are in each function's doc comment. Scenario: `scenario_view` = the
churn room (4 homes, 6 phones, 2 phones behind hard NAT through a home relay that leaves with
a signed end at 30 s), household traffic off, 60 one-second steps. `media_view(seed)` runs it
once and returns every buffer; the per-buffer functions each run it again.

- members, stride 4: lat, lon, access profile code, nominal uplink bytes/s (MODEL; places are
  two coarse metro anchors with about a degree of seeded scatter, never an address);
- tree, stride 3: stream, parent, child for every edge that existed at any step (one tree per
  publishing member; root = publisher);
- edges per step, stride 4, aligned to the tree: rate bytes/s, loss permille (copies sent in
  the step never verified at the child), queue ms (parent uplink + child downlink backlog at
  the step's end), layer (0 detached, 1 audio + base, 2 full) (DERIVED);
- rx per step, stride 3, index `stream * n + member`: layer shown (-1 no session, 0 audio
  only, 1 base, 2 both), stall, mean glass-to-glass ms (DERIVED);
- events, stride 5: time ms, kind (parent left, audio resumed, video resumed, not resumed),
  stream, child, departed parent (DERIVED).

The tree is the closed-loop controller's (§3), not `alm.rs` `compute_alm_topology_verified`:
member ids are the scenario's node indices, so a layer drawn from `alm.rs` for another member
set does not line up with this one.

## 11. Two people (`media_call2.rs`, `cewp-cli media-call2 5`)

Asked when v45.0.0 was weighed to ship calls limited to two people. One publisher each way,
audio and video, no relay tree, no warm standby, household traffic on (and off, for scale).
Four conditions: a laptop and a phone on home Wi-Fi, or a laptop on Wi-Fi and a phone on LTE;
each either punched direct at a probed MTU of 1,400 (frame budget 1,142 B; MODEL that the path
probes that high) or through a hosted relay at the base MTU (246 B). The relay is a third
node (`SERVER_RELAY`, 1 Gbit/s, 20 ms buffers, no loss; MODEL) that both members must use.
The model's relay verifies each chunk where an RNS transport hop would not; at 60 us per
verify on that host it is never the bottleneck.

MODEL output, 5 seeds, 10 sessions per condition (p95 is therefore the worst session), p50/p95:

| busy household | audio missing ‰ | audio lat p95 ms | freeze s/min | keyframe loss ‰ | g2g p95 ms | join s |
|---|---|---|---|---|---|---|
| Wi-Fi, direct 1400 | 7 / 19 | 76 / 173 | 14.4 / 21.9 | 107 / 214 | 157 / 258 | 2.1 / 4.0 |
| Wi-Fi, relayed 500 | 2 / 18 | 125 / 192 | 40.0 / 47.7 | 428 / 535 | 209 / 344 | 2.1 / 8.1 |
| LTE, direct 1400 | 13 / 375 | 156 / 2,782 | 15.9 / 22.0 | 107 / 214 | 249 / 2,542 | 2.1 / 4.1 |
| LTE, relayed 500 | 4 / 403 | 145 / 8,028 | 37.7 / 48.3 | 360 / 560 | 244 / 7,709 | 2.5 / 8.1 |

Quiet household: audio missing ≤ 13 ‰ at p95 except LTE relayed (288 ‰); freeze unchanged
(14–15 s/min direct, 37–39 s/min relayed). Chunk loss is 3–9 ‰ everywhere.

Reading (inference from the model's structure): audio is usable in all four conditions at the
median; its tail on LTE is video sharing a 1 s FIFO uplink through capacity dips with no
bitrate adaptation and no audio priority. Video is not smooth in any condition. A lost
fragment of a reference frame freezes the picture until the next forced keyframe (2 s; no
keyframe request, no retransmit), and at 7 ‰ chunk loss most two-second groups lose one. At
the base MTU an IDR is ~4.6× as many fragments, keyframe loss rises from ~10 % to ~43 %, and the
freeze nearly triples. Freeze scales with the MODEL Wi-Fi loss (Gilbert-Elliott, §2): by
arithmetic alone, not run, 1 ‰ chunk loss would freeze a direct call about 3–4 s/min.

## 12. Audio-only full-mesh room (`media_call2.rs`, `cewp-cli media-audio 5`)

The server's relay-off full mesh (≤ 9 members): Opus only, every member `relay: false`, no
standby, base MTU (one 20 ms Opus frame is one datagram at either MTU). N = 4 and 9; mixes
(MODEL): `wifi` alternates laptop and phone on home Wi-Fi; `wifi+lte` alternates a Wi-Fi
laptop and an LTE phone; `wifi+1dsl` is the Wi-Fi mix with member 0 on the 1 Mbit/s DSL line.

Each member's uplink carries N − 1 copies: 413–416 kbit/s at N = 4, 1,104–1,112 kbit/s at
N = 9, ~139 kbit/s per 24 kbit/s voice copy (§8 item 9). MODEL output, 5 seeds, p50/p95 over
sessions; worst publisher = highest median missing over one publisher's streams:

| busy household | audio missing ‰ | worst publisher ‰ | audio lat p50 ms | audio lat p95 ms |
|---|---|---|---|---|
| N=4 wifi | 6 / 10 | 7 | 22 / 22 | 39 / 134 |
| N=4 wifi+lte | 8 / 38 | 11 | 39 / 58 | 59 / 257 |
| N=4 wifi+1dsl | 8 / 176 | 58 | 22 / 53 | 57 / 475 |
| N=9 wifi | 7 / 17 | 7 | 23 / 25 | 41 / 143 |
| N=9 wifi+lte | 9 / 584 | 411 | 40 / 874 | 81 / 7,739 |
| N=9 wifi+1dsl | 7 / 944 | 943 | 23 / 520 | 43 / 584 |

Quiet household: N = 4 is ≤ 10 ‰ at p95 on every mix; N = 9 wifi 6/9; N = 9 wifi+lte 7/346
(worst publisher 65); N = 9 wifi+1dsl still 943 ‰ for the DSL publisher.

Reading: a member whose sustained uplink falls under (N − 1) × 139 kbit/s loses its voice to
everyone, since there is no adaptation and no relay offload. A 1 Mbit/s line holds 7 copies
(N ≤ 8, quiet only); an LTE phone in a 10 % dip (600 kbit/s, MODEL) holds 4 (N ≤ 5). N = 9 is
also the publisher's initial fan-out of 8 (edge `PUBLISHER_INITIAL_FANOUT`, DERIVED); a tenth
member would not be admitted without a relay. Not run: packing several Opus frames per chunk,
which would cut the per-copy overhead.

## 13. One thin publisher in a five-person call (`media_thin.rs`, `cewp-cli media-thin 5`)

The design case for edge v45.1. Eric's rule: ALM engages by the publisher's measured uplink,
not by room size. Member 0 publishes on a thin uplink (DSL buffer depth and last mile, uplink
swept 128 / 256 / 512 / 1,000 kbit/s, plus 2,000 to bracket video; MODEL). Member 1 is the
natural next hop (fibre, nearest, joins first). Members 2–4 are a fibre home, a cable-Wi-Fi
laptop and a Wi-Fi phone. Everyone publishes; the lines report member 0's stream only (4
receivers × 5 seeds = 20 sessions). Modes:
`a` full mesh, relay off (v45.0.0 as the server ships it); `b` relay on, edge's admission
(initial fan-out clamp(N−1, 1, 8) = 4, probe growth, nothing refused locally on a datagram
leg); `b975` as `b` plus the CIRISEdge#975 delivery report (not in v45); `c` counterfactual
(`CallParams::publisher_cap = 1`, `standby_spares_publisher`): exactly one copy to the next
hop, which relays to the other three, no standby charged to the publisher; `c1400` as `c` at
MTU 1,400. Thresholds (MODEL): audio is carried when the p95 session misses ≤ 50 ‰; video is
carried when every session shows video and the p95 freeze is ≤ 25 s/min (just above the
two-person direct Wi-Fi baseline of §11).

Lowest swept uplink that carries each, quiet household (MODEL output):

| mode | audio only | audio + video |
|---|---|---|
| a (mesh, relay off) | 1,000 kbit/s | none up to 2,000 |
| b (relay on, v45 admission) | 1,000 kbit/s | none up to 2,000 |
| b975 (loss-driven shedding) | 256 kbit/s (p95 exactly 50 ‰) | none up to 2,000 |
| c (one copy, base MTU) | 256 kbit/s | none up to 2,000 (2,000: freeze p95 39 s/min) |
| c1400 (one copy, MTU 1,400) | 256 kbit/s | 1,000 kbit/s (freeze p50/p95 11/22 s/min) |

Busy household (the thin line's own household uploads on, §2 `BG_UP_HOME`): no mode carries
audio at any swept uplink. Even `c` at 2,000 kbit/s misses 409 ‰ at p95, because a household
backup fills the line's 500 ms FIFO ahead of the voice.

Uplink use (publisher media bytes on the wire over capacity): one Opus copy is ~138 kbit/s
(`c`: 538 ‰ of 256, 269 ‰ of 512). Four copies are ~540 kbit/s (`a`: 539 ‰ of 1,000). One
audio + video copy is ~770 kbit/s at MTU 1,400 (770 ‰ of 1,000), and more than 1,000 kbit/s at
the base MTU, where `c` saturates the line at 988 ‰ and no session shows video.

Reading (inference from the model's structure):
1. At 128 kbit/s nothing carries even audio, in any mode, because one voice copy is ~139 kbit/s
   on the wire: a 60 B Opus frame in a ~351 B datagram, 50 per second. The relay tree cannot
   fix that. Only the per-copy overhead can: packing several 20 ms frames per chunk, or a
   lighter per-chunk signature.
2. With relay on, edge's admission alone (`b`) does not move a thin publisher off full mesh by
   itself: fan-out starts at 4 and nothing is refused on a datagram leg. Receivers' degraded-
   parent moves do shift some of them to the next hop (fan-out ends at 1 in the 256 and 512
   runs), but only after long loss; audio p95 stays 349–985 ‰.
3. Loss-driven shedding (`b975`) gets audio to the one-copy floor (256 kbit/s), which is what
   "engage by measured uplink" asks for. `c` is the same floor reached at once, with no loss
   period.
4. Video from a thin publisher needs both the one-copy rule and the probed MTU; at the base MTU
   one copy does not fit 1 Mbit/s.
5. The rule must measure the uplink the call actually gets, not the nominal rate: with the
   household busy, the free share of the line is what decides.
