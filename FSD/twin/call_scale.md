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
| capacity.rs:378 | `room_att_rounds_per_cadence` | (none) | 1 (MODEL) | a member of a room hubbed at the seat rounds Attestation with it, where the room's rows ride (v45.2.1 wrap_courier.rs header, runtime.rs set_peers) |
| capacity.rs:379 | `room_rows_per_join` | (none) | 4, interval 2-6 (MODEL) | call row + its share (av_call.rs PersistCallRecord::record_join), roster row, MLS commit |
| capacity.rs:380 | `room_delivery_reads_per_row` | (none) | 1 (MODEL) | the hub's row read to deliver a row to one member |
| capacity.rs:1362 | `Scenario::room_members`, `room_joins_per_min`, `room_size` | (none) | 0 by default (FRONTIER: the server chooses whether the canonical hubs a room) | |
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

## 5. What a 1000-person call puts on the seat

- **The canonical is not the room's hub: nothing per member joined (DERIVED).**
  - Call rows, the roster and MLS rows ride the Attestation plane.
  - A v45 device keeps one ordinary round with a seat, on Revocation (`BUNDLE_ROUND_PLANES`,
    runtime.rs :1207), and Attestation-plane writes kick no Pull (`kick_wrote` returns for planes
    outside `BUNDLE_PULL_PLANES`, runtime.rs :1491-1494).
  - A member's keys already sit on the seat. Its 10-minute Pull and Revocation round are the
    same in a call or out of one.
- **The canonical hubs the room (MODEL; the server's choice).**
  - Each member becomes a consent peer of the seat: whole-plane rounds on the five bootstrap
    planes plus an Attestation round every 30 s.
  - Per member at today's fleet, steady: 1.3e-3 to 2.3e-3 cores, 9.3e-4 to 6.9e-3 sweep-permit
    utilisation and 14 to 20 counted reads/s (the low ends at 1000 members, the high ends at 12,
    where the 10 s send-set read is shared by fewer).
  - While joins arrive at one a second into a 1000-member room, the rows and their 999 deliveries
    each add 0.63 cores.
  - The largest room the seat can hub at today's fleet:

    | key_publish_set growth g | members (point) | members (interval) | binds at the point |
    |---|---|---|---|
    | 0 | 640 | 237 to 1,909 | sweep permits |
    | 1 | 640 | 235 to 1,860 | sweep permits |

    The binding term is the two sweep permits held per Attestation and route listing page
    (`att_listing_permits` 2-4 DERIVED, `td_listing_permits` MODEL, page hold = `page_cpu_s`,
    FITTED CPU per byte × MODEL bytes per hash). A 1000-member room hubbed at the seat exceeds
    the permit ceiling at the point estimate.
  - The fleet knee with such a room: 259 to 24,169 devices at g = 0 and 257 to 7,920 at g = 1,
    sweep permits at both ends.
- **MLS.** The call's group is the room's (av_call.rs :10), so commits ride the room's rows
  through the hub when there is one. Commit and ratchet-tree bytes for suite 0x004D are not
  measured. The scale memo's estimates (~24 KB a commit, ~2.6 MB a tree at 1000) are not used here.

## 6. The canonical-to-node ratio

One v45 seat serves 4,882 to 75,879 devices (MODEL; outer interval over every input interval, one
seat of 4 cores at a 70 % ceiling). Where in that range it falls is set by the server's
`key_publish_set` growth g:

| g | devices | binds at the low end | binds at the high end |
|---|---|---|---|
| 0 | 14,253 to 75,879 | sweep permits | CPU |
| 0.25 | 7,961 to 24,430 | CPU | CPU |
| 0.5 | 6,316 to 18,438 | CPU | CPU |
| 0.75 | 5,440 to 15,534 | CPU | CPU |
| 1 | 4,882 to 13,723 | CPU | CPU |

This is unchanged from v45.0.0: no seat-side constant moved between 8e46165 and v45.2.1. Calls add
nothing to it unless the canonical hubs a room (section 5).

**What the twin needs from the server.**
- The number is g in `subjects(N) = subjects_today × (1 + g (N / 1,349 - 1))`, for the
  Key-plane publish set (8,211 today), the occurrence publish set (5,650) and the Global set
  (own ∪ cohort, every key held, 8,698).
- Either the rule, which does or does not list every key the seat holds as the fleet grows,
  or two readings at different fleet sizes:
  - the seat's `announced_nodes`;
  - subjects re-read per key plane over one 10-minute `MEMO_TTL` window. v45.2.1 counts these
    as `memo_reads` `key_plane.{key_record,occurrence_binding,revocation}.ttl`; with
    stale-while-revalidate every subject is re-read once a TTL, so the 10-minute sum is the
    walked set.
- Then g = (S2 / S1 - 1) / (N2 / N1 - 1).

## 7. What this model cannot say

- It is closed form, a mean member, and a uniform room. No access link, queue, loss or churn
  is in it; those are the packet engine's (media.rs), which stops near twenty members.
- The fixed shape's legs, presence transport, watch count and standby are MODEL. The 45.3
  fixes are not built.
- The proof term misses by about a fifth either way, and the measured classes carry no
  per-class byte split (bindings and identity-plane replication are not separated).
- The bench's copy (30 fps of one-chunk frames) and the matrix's copy (larger codec frames)
  bracket the rate. Neither is a phone camera.
- The hub-room cost assumes a room member's Attestation round costs what today's peers' rounds
  cost the seat (composer lookups 386.7 a round, MEASURED on today's peers).
- Admission has no model, only the measurement divided by twelve.

## 8. Owed

- Re-run the packet engine's seeded findings on v45.2.1's publisher slow start
  (`PUBLISHER_INITIAL_PROBE` 2, `STALL_PROBE_HOLD_MS` 10 s, `STALL_NONCHILD_OFFER_MS` 2 s).
- Line numbers in capacity.rs still cite 8e46165. The texts hold at v45.2.1.
- From the bench: per-class byte counters (offers, proofs, bindings, announces, identity-plane
  replication) at N = 4 and 6, to replace the two FITTED terms and the subtraction.
- From the server: g (section 6), and whether the canonical hubs any call room.
