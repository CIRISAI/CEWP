# Call scale: what one live call costs each member and the seat

Module `crates/cewp-sim/src/call_scale.rs`; run `cargo run --release -p cewp-sim --example call_scale`.
Edge read: v45.2.1 (tag, d175963). Measurements: edge v45.2.1's own mesh witness with a socket-level
FIFO shaper ("bench"), and the server's 4-6-node container matrix with a `tc netem` cap on one
member's whole uplink ("matrix"), both 2026-11-25. Aggregates only.

The goal this answers: one live call of 1000 people (later millions) on a mesh that runs ONE
canonical seat on purpose, so that every per-seat cost is driven down. More seats are not an
answer to load here.

## 1. What changed in the twin

Base: `twin/holon-draw` 38e51ea. Its `crates/` is byte-identical to `twin/door` 4479258,
`twin/wrapup` and `twin/uppm-precision` (capacity.rs blob 917e1b4, media.rs 752bbaa); the branches
differ only in `FSD/twin/globe.md` and `texture.md`.

The 2026-11-11 audit's seat-model items were already on this base (calibration commit 859250b):
the read budget and the listing cache are booked BUILT (`listing_budget_k` = 256, `warm_listing_s`),
the Global planes walk every key on the seat (`global_subjects_at_seat` = 8,698, from 30), the reach
rule has its term (`reach_unowned_share`, `reach_reads_per_unowned_subject`), the seat-to-device
initiator term exists (`seat_initiator_device_share`, FRONTIER, 0), and the source test's needle
reads `from_mins(10)`, not `from_secs(600)`. This change re-checks them against v45.2.1: all 25
needles and the three plane lists hold there (`edge_971::VERIFIED_AT`), and
`edge_971_constants_match_the_source` passes with the v45.2.1 tree as `CEWP_EDGE_SRC`.

| file:line | parameter | before (class) | now (class) | source |
|---|---|---|---|---|
| capacity.rs:131 | `edge_971::VERIFIED_AT` | (none) | d175963, v45.2.1 | every needle compared against the v45.2.1 tree |
| capacity.rs:379 | `room_att_rounds_per_cadence` | (none) | 1 (MODEL) | a member of a room hubbed at the seat rounds Attestation with it, where the room's rows ride; not reachable on v45.2.x (section 5) |
| capacity.rs:380 | `room_rows_per_join` | (none) | 4, interval 2-6 (MODEL) | call row + its share (av_call.rs PersistCallRecord::record_join), roster row, MLS commit |
| capacity.rs:381 | `room_delivery_reads_per_row` | (none) | 1 (MODEL) | the hub's row read to deliver a row to one member |
| capacity.rs:1362 | `Scenario::room_members`, `room_joins_per_min`, `room_size` | (none) | 0 by default; not reachable on v45.2.x (section 5) | |
| capacity.rs:334 | `global_subjects_at_seat` | 8,698, interval 8,211-8,698, scaled by g (DERIVED) | 8,698, interval 4-8,698, constant in the fleet (FRONTIER) | self_provider ∪ start-time cohort (v45.2.1 bridge.rs :8385, runtime.rs :2069); server self_provider = own keys (server compose.rs :5187) |
| capacity.rs:335 | `announced_fraction` f (was `key_publish_set_growth` g) | g (FRONTIER): every set × (1 + g(N/N0 - 1)) | f (MODEL, swept 0-1): Key + f(1 + r) per new device, IO + f per new device, Global constant | server announced_relay.rs :164 `selection_for` |
| capacity.rs:336 | `new_owners_per_announced` r | (none) | 0.5, interval 0-1 (MODEL) | owners in the Key set per new announced node |
| media.rs:51 | `MTag::Measured` | (none) | the media table can say MEASURED | |
| media.rs:237 | `MEASURED_CHUNK_RNS_BYTES` | (none; DERIVED `full_chunk_wire` 527 B) | 504 B RNS, 532 B with IPv4/UDP (MEASURED, bench) | 246 B frame; header 52, tags 32, trailer 72, E1 seal 25, token ~53, RNS header 19, framing ~5 |
| media.rs:241 | `MEASURED_CHUNK_RESIDUAL` | (none) | 5 B (MEASURED minus DERIVED) | the derived packet is 499 B, 1 B under the 500 B RNS MTU, so the 5 B lie outside the RNS packet (untraced framing on the measured path) |
| media.rs:244 | `MEASURED_COPY_BPS_BENCH` | (none) | 127 kbit/s (MEASURED, bench) | 30 fps, 30.3 chunks/s, ~122 kbit/s of frames |
| media.rs:247 | `MEASURED_COPY_BPS_MATRIX` | (none) | 190 kbit/s (MEASURED, matrix) | larger codec frames; also v45.2.1 alm.rs :57 |
| media.rs:128 | `PUBLISHER_INITIAL_FANOUT` | 8 (DERIVED at 8e46165) | 8, marked OWED | v45.2.1 slow-starts at `PUBLISHER_INITIAL_PROBE` 2 with a 10 s hold after a stall (alm.rs :59, :62; 45.2.0 6c67851). The packet engine keeps the 8e46165 rule so its seeded findings stay comparable; re-running them is owed |
| media.rs (video_bitrate) | `video_bitrate` | 500 kbit/s (MODEL) | unchanged | the engine's phone camera is a target codec, not the measured stream; call_scale uses the measured copy |
| micro.rs:168 | `bundle_bootstrap` source | cites c580298 | cites e3c20ec in 8e46165 | a stale cite the audit named |
| call_scale.rs:113 | offers per leg | (none) | 1 + (N - 2) (DERIVED) | v45.2.1 alm.rs :2098-2170, `OFFER_EVERY_MS` 1000 (:53), `ensure_relay` per received stream (:2015) |
| call_scale.rs:126 | `CTRL_BYTES_PER_MSG` | (none) | 190.2 B (FITTED at N = 4) | 14 kbit/s / 9.2 msg/s; the source gives 184 B for an offer with no ancestry (64 + `OFFER_WIRE_OVERHEAD` 120, alm.rs :71) |
| call_scale.rs:64 | `PROOF_BYTES` | (none) | 110 B (MEASURED) | one proof upstream per received Channel message |
| call_scale.rs:129 | `ANN_BPS_PER_PEER` | (none) | 833 bit/s per peer (FITTED at N = 4) | 2.5 kbit/s over 3 peers |
| call_scale.rs:70 | `NONMEDIA_CAPPED_N6_KBPS` | (none) | 130-150 kbit/s (MEASURED, matrix) | the capped member's non-media at N = 6/300 |
| call_scale.rs:100 | `CAPPED_RUNS` | (none) | latency p50/p95 and heard share for N = 4/250, N = 4/300, N = 6/300 (MEASURED, matrix) | |
| call_scale.rs:82 | `ADMIT_12` | (none) | 12th seated 93 / 127 s, read-all 81 / 85 s, takedown 68 / 71 s (MEASURED, matrix) | |
| call_scale.rs:132 | `CALL_SPECS` | (none) | presence blob 176 B, 306-462 B on the wire, 0.2-2 per s, 256 visible (MODEL: locked design, not shipped); watch 9 (MODEL) | |

## 2. The per-member model

Mean member, kbit/s each way. In a relay tree every copy one member receives another sends, so
the mean uplink equals the mean downlink; who carries it moves with the tree.

**Today (edge v45.2.1, as measured).**
- Legs: 2(N - 1) links (each publisher dials every member, v45.2.1 av_call.rs :21-24).
- Offers: (N - 1)(1 + (N - 2)) = (N - 1)² sent a second, as many received (DERIVED). Checks: 9 and
  25 against the matrix's 9.2 and 24.7 messages a second at N = 4 and 6.
- Control: offers × 190.2 B (FITTED at N = 4; held out: 38.0 kbit/s at N = 6 against 26-38
  MEASURED, at the upper end).
- Proofs: offers received × 110 B (not fitted): 7.9 against 10 MEASURED at N = 4, 22 against
  18-20 at N = 6. The miss is about a fifth either way, and the per-class byte split was not
  measured.
- Announces: (N - 1) × 833 bit/s (FITTED at N = 4; held out: 4.17 against 4-5 at N = 6).
- Media: (N - 1) × (1 + 0.69) copies, a primary and a warm audio-plus-base standby for every
  stream (alm.rs :31-32). The standby share is MODEL output of the packet engine (media.md :146).
- Bindings are not a separate term. The source re-sends 3,504 B on every leg every 10 s
  (av_chunk_auth.rs :493), which would be 2.8 kbit/s a leg. That is more than the measured
  announce class, so the measured classes absorb whatever bindings actually flowed.

**Fixed (MODEL): the 45.3 fixes plus per-stream subscription.**
- A member watches `watch` streams: 9 here, 25 shown too.
- Each watched stream has a parent and as many children on average. A warm standby adds a leg
  at the high end only.
- The presence tier adds 4 neighbours, and only those legs carry a liveness offer, its proof and
  announces.
- Presence: min(N - 1, 256) - watch visible members × 0.2-2 blobs a second × 306-462 B.
- The 45.3 fixes do not change the means. They change what a capped member sends.

## 3. Results (run output `call_scale` at this commit)

### Fitted terms against the held-out N = 6

| term | N = 4 model | N = 4 MEASURED | N = 6 model | N = 6 MEASURED (held out) |
|---|---|---|---|---|
| offers sent/s (DERIVED count) | 9.00 | 9.2 | 25.0 | 24.7 |
| control kbit/s (bytes FITTED at N = 4) | 13.7 | 14 | 38.0 | 26 to 38 |
| proofs kbit/s (one 110 B proof per offer received; not fitted) | 7.92 | 10 | 22.0 | 18 to 20 |
| announces kbit/s (FITTED at N = 4) | 2.50 | 2.5 | 4.17 | 4 to 5 |
| capped member offered / cap, N = 6/300 (one copy + MEASURED non-media) | | | 0.86 to 1.13 | 0.96 to 1.11 |

Identity-plane replication on the capped member at N = 6/300 is 94 to 115 kbit/s. This is
DERIVED by subtraction from MEASURED totals: non-media, less C3 sent at the stall cadence (10.6/s ×
190 B), less proofs for C3 received (16.7/s × 110 B), less announces.

Today's control, proofs and announces alone, before any media, fill a 300 kbit/s uplink at
N = 13 and a 1 Mbit/s uplink at N = 22 (DERIVED + FITTED; at N = 12 they are 299.8 kbit/s).

### Per member, one live call, kbit/s each way

| N | today: legs | today: offers sent/s | today: control + proofs + announces | today: media (copy 127-190) | today: total | fixed, watch 9: non-media | fixed: media (9 copies, cold to warm standby) | fixed: presence | fixed: total | fixed, watch 25: total |
|---|---|---|---|---|---|---|---|---|---|---|
| 4 | 6 | 9 | 24.1 | 644 to 963 | 668 to 987 | 29.1 to 38.8 | 381 to 963 | 0 | 410 to 1002 | 410 to 1002 |
| 12 | 22 | 121 | 300 | 2361 to 3532 | 2661 to 3832 | 71.2 to 100 | 1143 to 2890 | 0.98 to 14.8 | 1215 to 3005 | 1481 to 3652 |
| 50 | 98 | 2401 | 5807 | 10517 to 15734 | 16324 to 21541 | 71.2 to 100 | 1143 to 2890 | 19.6 to 296 | 1234 to 3286 | 3361 to 8460 |
| 100 | 198 | 9801 | 23622 | 21248 to 31789 | 44870 to 55411 | 71.2 to 100 | 1143 to 2890 | 44.1 to 665 | 1258 to 3655 | 3386 to 8830 |
| 1000 | 1998 | 998001 | 2397771 | 214415 to 320779 | 2612186 to 2718549 | 71.2 to 100 | 1143 to 2890 | 121 to 1826 | 1335 to 4816 | 3463 to 9991 |

Today's 1000-person call is about 2.6 Gbit/s each way per member. About nine tenths of it is offers
and their proofs (O(N²) per member), and the rest is the full-mesh subscription (O(N)). Under
per-stream subscription a member's cost stops growing once N - 1 passes 256 (the presence window).
At 1000 it is 1.3 to 4.8 Mbit/s each way at watch 9, and 3.5 to 10 Mbit/s at watch 25, all MODEL.
The low end is a cold standby, presence at its floor and a 127 kbit/s copy. The high end is a warm
standby, presence at its sustained cap and a 190 kbit/s copy.

### A capped uplink

- **Today (MEASURED).** The kernel FIFO fills before the verdict, and nothing drops on age:
  - The first capped second queues 260 to 440 full chunks, 4.4 to 7.5 s at 250 kbit/s, before
    the verdict at +0.9 s.
  - After the shed, one copy plus non-media is 96 to 111 % of the cap, so the queue never drains.
  - At N = 4/250: p50 6.0 to 9.9 s and p95 11.1 to 19.9 s. At N = 6/300: p50 11.9 s and p95
    15.9 s.
  - The twin brackets the after-shed load with one copy and the measured non-media: 0.86 to 1.13.
- **Fixed (MODEL).**
  - The sender withholds video while it is backlogged, and the receiver drops media older than
    the end-to-end budget (0.8 s assumed). Latency is then bounded by the budget, and the loss
    shows as frames not shown.
  - A capped member keeps sending its own video only while its non-media, its identity-plane
    replication and one copy fit the cap.
  - At N = 1000 with subscription, 300 kbit/s carries the member's 71 kbit/s non-media plus a
    127 kbit/s copy. It does not carry that plus ~100 kbit/s of identity-plane replication and a
    190 kbit/s copy (`fixed_capped_sends_video`). Today's replication of 94 to 115 kbit/s on the
    capped member is therefore what decides a thin member's video, once control is scoped.

### Per-Add stream roll today (DERIVED)

Every Add or Remove rolls every live stream (av_call.rs :10-13). Each member then re-sends its
3,504 B binding on every leg (:60-62): 3,504 × (N - 1) B.

| N | bytes each member sends per Add | seconds on a 1 Mbit/s uplink |
|---|---|---|
| 4 | 10,512 | 0.08 |
| 12 | 38,544 | 0.31 |
| 50 | 171,696 | 1.37 |
| 100 | 346,896 | 2.78 |
| 1000 | 3,500,496 | 28.0 |

## 4. Admission (12 seats in 93 to 127 s) and what it implies at 1000

The twin has no admission model, so only arithmetic on the measurement is possible:
- 93 to 127 s for 12 seats is 7.8 to 10.6 s per seat if they are serial (DERIVED: MEASURED / 12).
- Each seat is a commit, then the joiner's Welcome, wraps and roster, then each member's next
  round that carries the rows. The hub's kicks ("the room seat kicks after it grants",
  wrap_courier.rs) cut that wait, and the wrap courier settles a row only on the recipient's next
  answer (`CONFIRM_DELAY`).
- At the measured per-seat time, 1000 serial seats take 7,800 to 10,600 s. This is a lower bound
  (MODEL: constant per-seat time), because the per-Add roll grows with N. At 1000 members one
  Add's roll alone is 28 s of a 1 Mbit/s uplink per member, longer than the measured time to seat
  one member.
- Seating 1000 needs either batched Adds (one commit seats many) or a stream epoch decoupled from
  room churn.

**SLOT (pending, from the server's per-run timeline): the 12-person seating steps.** Per seat, the
time of each step (request, commit, Welcome, wraps, roster, first row read by every member), for
both runs. Until it lands, the per-seat time above is the measurement divided by twelve and the
1000-seat figure is a lower bound; no step is modelled.

## 5. What a 1000-person call puts on the seat

**Nothing per member joined, on v45.2.x (DERIVED).**
- The canonical is never a call-room member, and edge 45.2.x refuses a serve seat any call
  relay (server 0.5.254).
- A room's rows (the call row is shared `With::Community`, av_call.rs :1131-1140; the roster and
  the MLS rows ride the same room scope) are served to a peer only past persist's `may_receive`
  audience gate (`audience_withholds` on the fetch and advertise paths, bridge.rs :6445-6468).
  A canonical outside the room is never sent them, holds none, and so cannot relay them in a
  round.
- A v45 device keeps one ordinary round with a seat, on Revocation (`BUNDLE_ROUND_PLANES`,
  runtime.rs :1207), and Attestation-plane writes kick no Pull (`kick_wrote` returns for planes
  outside `BUNDLE_PULL_PLANES`, runtime.rs :1491-1494).
- A member's keys already sit on the seat. Its 10-minute Pull and Revocation round are the same
  in a call or out of one.
- The one path not costed: RNS packets of a member-to-member round may transit the canonical as
  a Reticulum transport node. That is packet forwarding with no store read, outside the seat
  ledger; this week's measurements do not show it.

**Not reachable on v45.2.x, kept as a term (MODEL):** a room hubbed at the seat, for a server
that seats its canonical in a room (`Scenario::room_members`, 0 by default).
- Each member would cost 1.3e-3 to 2.3e-3 cores, 9.3e-4 to 6.9e-3 sweep-permit utilisation and
  14 to 20 reads/s.
- The largest such room at today's fleet is 640 members at the point, 237 to 2,009 over the
  intervals, bound by sweep permits.

**MLS.** The call's group is the room's (av_call.rs :10), so commits ride the room's rows among
its members. Commit and ratchet-tree bytes for suite 0x004D are not measured.

## 6. The canonical-to-node ratio

**The rule (server 0.5.254, unchanged since CIRISServer#655, read from its code).**
- `selection_for` (server announced_relay.rs :164), on a relaying canonical:
  - Key publish set = own keys ∪ every ANNOUNCED node ∪ each announced node's owner;
  - IdentityOccurrence publish set = own ∪ every announced node.
  - Unannounced nodes are in neither.
- The self_provider (server compose.rs :5187) is the server's own keys.
- Revocation and occurrence withdrawal are not selected by the server. They walk edge's
  Global set, `self_provider() ∪ cohort()` (v45.2.1 bridge.rs :8385-8390, `list_revocation_rows`
  :14381). The cohort is the runtime's start-time snapshot of the host-named peers
  (runtime.rs :2069-2070), never refreshed.
- So Global does not grow with the fleet. Its size is set by the peers the server names at a
  start: FRONTIER, 4 (own keys alone) to 8,698 (every key held: the key_plane_memo.rs header's
  claim at #930, when the server's self_provider was its whole publish set). The point is 8,698,
  the costlier end.

**The twin now.**
- Key(N) = 8,211 + f (1 + r)(N - 1,349) and IO(N) = 5,650 + f (N - 1,349).
- f = `announced_fraction` (MODEL, the swept axis, 0 to 1): the share of new devices the
  server lists as announced.
- r = `new_owners_per_announced` (MODEL, 0.5, interval 0 to 1).
- Global = `global_subjects_at_seat` (FRONTIER, interval 4 to 8,698), constant in N.
- Under the server's rule, the twin's 5,650 occurrence subjects imply ~5,646 announced nodes in
  the selector's set, against the relay report's 1,349. Which count the selector reads is for
  the canonical reading to settle; the slope per new node is what matters for the knee.

**One v45 seat serves 11,820 to 77,382 devices** (MODEL; outer interval over every input
interval, 4 cores at a 70 % ceiling):

| announced fraction f | devices | binds at the low end | binds at the high end |
|---|---|---|---|
| 0 | 14,253 to 77,382 | sweep permits | sweep permits |
| 0.25 | 13,550 to 74,124 | sweep permits | sweep permits |
| 0.5 | 12,914 to 71,004 | sweep permits | sweep permits |
| 0.75 | 12,355 to 68,102 | sweep permits | sweep permits |
| 1 | 11,820 to 65,483 | sweep permits | sweep permits |

**What binds, at the calibrated point.**
- Point knee: 30,813 devices at f = 0 and 25,167 at f = 1 (MODEL, one input point; the
  interval above is the claim).
- The two sweep permits reach 0.70 first; CPU is at 0.50 to 0.57 and the busiest key-plane
  lock at 0.21 to 0.25.
- 59 to 73 % of the held permit time is the devices' 10-minute subject Pulls: 8 permits a cycle
  (MEASURED on a debug build, contact-4413), each held one store read (~2 ms, DERIVED, interval
  1 to 3 ms). The rest is listing pages and, at f = 1, key-plane refills over the larger Key
  set (43,938 subjects).
- **What it hinges on.** It hinges on the permit hold per Pull read on a release build
  (unmeasured), not on the publish sets. Under the server's rule the sets grow by one or two
  subjects per announced node, so f moves the knee by about a fifth. Before this rule was read
  the twin grew them by 4.2 to 6.1 subjects per device (g = 1), and CPU on the Global walk bound
  at 4,882 to 13,723.

The previous table (g, 4,882 to 75,879, CPU binding from g = 0.25) is superseded.

**Check against canonical-1 (not a fit).** Sample: edge v45.2.1, server 0.5.254, 38 min just
after a roll, so not steady state (MEASURED, with that caveat). It shows 131 to 174 % of a core,
replication out ~8.4 Mbit/s and in ~2.0 Mbit/s. `call_scale::seat_check`:

| devices | model seat cores (point, interval) | model egress, devices' term | today's peers' whole-plane offers (DERIVED, not in the term) |
|---|---|---|---|
| 1,349 | 0.26 (0.11 to 0.56) | 228 kbit/s, 169 bit/s a device | 557 kbit/s |
| 5,646 (the announced count the occurrence set implies) | 0.58 (0.21 to 1.35) | 954 kbit/s | 2,324 kbit/s |

- **CPU.**
  - The model is 5 to 7 times low at 1,349 and 2.3 to 3.0 times low at 5,646. Only the top of
    the 5,646 interval touches the sample's low end.
  - No single term closes it plausibly. `today_rounds_scale` would have to be 27 to 38 (19 to
    30 at 5,646), or the fitted `cpu_per_byte_s` 12 to 16 times larger (4.5 to 6.5).
  - What the model lacks:
    - a CPU term for serving bytes: Deliver bodies, the #927 proactive push of the self-publish
      set (runtime.rs :2072-2077), cursor-plane pages, and framing;
    - the post-roll re-sweep;
    - a bias in its fit window. Window B is a quiet minute at 0.652 cores, while the same seat
      averaged 1.28 cores over the 45-minute pre-roll window (MEASURED, a different meter),
      so the fit alone is about 2 times low.
- **Egress.**
  - The model's egress term counts only the devices' idle Revocation round and amortised Pull
    replies.
  - With today's peers' whole-plane offers added, it is about 0.8 Mbit/s at 1,349 (11 times
    low) and 3.3 Mbit/s at 5,646 (2.5 times low).
  - Measured per node: 6.2 kbit/s per relay-reported announced node, or 1.5 kbit/s per node of
    the selector's implied set (DERIVED from MEASURED).
  - The model has no Deliver-body, proactive-push or cursor-page bytes.
- **What it means for the ratio.**
  - The knee binds on permits with CPU at 0.50 to 0.57 of the ceiling. If the missing CPU
    scales with devices, CPU binds first and the knee falls. If it is the re-sweep or a fixed
    cost, it does not.
  - The like-for-like steady reading decides it, together with v45.2.1's seat-load counters
    (replication I/O by plane, persist reads by caller label).

**SLOT (pending, from the server): sizes read on canonical-1.** The Key and occurrence publish
sets and the announced-node count the selector reads, and the Global set (own ∪ start-time
peers). The Global size replaces the 4 to 8,698 interval. A second reading at a different fleet
size checks the slope f (1 + r).

**SLOT (pending, from the server's matrix): the canonical's share of a call member's non-media**
(one canonical node per cell, split approximate). Until it lands, the 94 to 115 kbit/s of
identity-plane replication on the capped member (section 3) is not split between the canonical
and the other members.

## 7. What this model cannot say

- It is closed form, a mean member, and a uniform room. No access link, queue, loss or churn
  is in it; those are the packet engine's (media.rs), which stops near twenty members.
- The fixed shape's legs, presence transport, watch count and standby are MODEL. The 45.3
  fixes are not built.
- The proof term misses by about a fifth either way, and the measured classes carry no
  per-class byte split (bindings and identity-plane replication are not separated).
- The bench's copy (30 fps of one-chunk frames) and the matrix's copy (larger codec frames)
  bracket the rate. Neither is a phone camera.
- The hub-room term (not reachable on v45.2.x) assumes a room member's Attestation round costs
  what today's peers' rounds cost the seat (composer lookups 386.7 a round, MEASURED on today's
  peers).
- Admission has no model, only the measurement divided by twelve.

## 8. Owed

- Re-run the packet engine's seeded findings on v45.2.1's publisher slow start
  (`PUBLISHER_INITIAL_PROBE` 2, `STALL_PROBE_HOLD_MS` 10 s, `STALL_NONCHILD_OFFER_MS` 2 s).
- Line numbers in capacity.rs still cite 8e46165. The texts hold at v45.2.1.
- From the bench: per-class byte counters (offers, proofs, bindings, announces, identity-plane
  replication) at N = 4 and 6, to replace the two FITTED terms and the subtraction.
- From the server: the set sizes read on canonical-1 (section 6).
- The public page's seat-knee claim (cewp-evidence page_ledger.rs :983, 4,882 to 75,879 and the
  "publish set grows or not" text) and the globe's g slider labels are stale under this rule. The
  ledger, `docs/` and the wasm are other packages', so they are left unchanged.
- FSD/SIMULATION_ENGINE.md's growth table (the g rows) is superseded by section 6.
