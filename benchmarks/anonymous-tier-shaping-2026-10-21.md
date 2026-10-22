<!-- Vendored 2026-10-21 from the CIRISEdge session's report. Harness: CIRISEdge tests/shaping_measure_e1.rs and shaping_anonymity_model.py on local branch local/shaping-measure (dee9036, not pushed at vendoring). Edge v43.1.1 (d44e1c57). MEASURED = run with edge's production emission primitives; MODELLED = arithmetic from stated assumptions. Re-pull from CIRISEdge before relying on a number. -->

# Traffic shaping, relays and onion routing — measured vs modelled

Edge v43.1.1 (d44e1c57), 2026-10-21. For the steward's questions A–D on the three wizard opt-ins.
MEASURED = run on this box with edge's production emission primitives. MODELLED = arithmetic from
stated assumptions. No implementation was done.

Harness (measured): `tests/shaping_measure_e1.rs` on local branch `local/shaping-measure`
(not pushed), release build, using `seal_envelope`/`unseal_envelope`, `fragment_payload`,
`PoissonScheduler`, `BudgetMeter` on a virtual clock — the same loop the wheel-driven
`EmissionScheduler` simulator runs. Row sizes are the REAL rows from CIRISServer's kept T2 mesh
(`/tmp/ciris-native-mesh-t2d/{A,B,canonical}` databases, 50 rows: envelope + hybrid signatures + ~200 B
framing). Model: `anonymity_model.py` (scratchpad).

## A. Shaping cost for option (1)

**MEASURED, CPU (this server, one core, release):** seal 2.88 µs and unseal 2.05 µs per 1,400-byte
envelope (XChaCha20-Poly1305, header inside the AEAD). At λ = 1 envelope/s per scope that is
~0.0003 % of a core. **MODELLED for a phone:** ×3–5 on a mid ARM core, still negligible. The phone
cost is not CPU: a Poisson timer at λ ≥ 0.2/s keeps the radio awake permanently (no idle sleep),
which is the dominant battery cost of cover traffic on mobile.

**MEASURED, bandwidth overhead (padding + fragmentation), real rows:** 50 rows, 423,759 payload
bytes → 505,400 wire bytes, **overhead ratio 1.19** overall. Per family: large hybrid-signed rows
(key:record 16.8 KB → 1.13×, identity:occurrence 13.7 KB → 1.12×, chat:welcome 12.7 KB → 1.10×)
pad little; small rows pad most (trust:accepts 5.2 KB → 1.34×; a chat message 5.7 KB → 1.23×).
Maintenance frames: a 50-ref Summary (2 KB) → 2 envelopes, 1.40×; a 200-ref Summary (8 KB) → 7
envelopes, 1.23×; a small Diff (300 B) → 1 envelope, **4.67×**. Cover on top of real is set by the
BudgetMeter: at load ρ of the scope's rate, cover = (1−ρ) of all envelopes (see M3 below), so the
total line rate is fixed at λ × 1,400 B/s regardless of real volume.

**MEASURED, latency added by the Poisson timer** (virtual clock, 4 h per cell, real row-size mix,
rows arriving Poisson at ρ·λ envelope-equivalents/s, 10-min budget window; delay = row arrival →
its last fragment sent):

| λ (env/s) | line rate | ρ=0.25 mean / p95 (s) | ρ=0.5 mean / p95 (s) | ρ=0.8 mean / p95 (s) |
|---|---|---|---|---|
| 0.1 | 140 B/s | 857 / 2551 | 638 / 1355 | 409 / 926 |
| 0.2 | 280 B/s | 482 / 1165 | 211 / 577 | 195 / 430 |
| 0.5 | 700 B/s | 118 / 449 | 53 / 187 | 63 / 168 |
| 1 | 1.4 KB/s | 25 / 128 | 29 / 129 | 49 / 158 |
| 2 | 2.8 KB/s | 16 / 99 | 12 / 56 | 18 / 52 |
| 5 | 7 KB/s | 5 / 21 | 5 / 17 | 14 / 66 |

Reading it: a multi-fragment row waits for several timer fires, so delay ≈ (fragments × 1/λ) plus
queueing; p95 is 4–5× the mean. For maintenance (anti-entropy Summary/Diff/Deliver, witness
chains, repair) a server-class node at λ = 1–2/s (1.4–2.8 KB/s per shaped scope, ~10–20 % of a
typical home uplink's idle) adds tens of seconds mean, ~1–2 min p95: acceptable for anti-entropy
(today's cadence is 30 s rounds). For ordinary chat rows the same applies: a message becomes a
10–60 s delivery, not sub-second. A phone cannot sustain λ ≥ 1/s on battery; at λ = 0.2/s the
delay is 3–8 min mean. So the wizard copy "may increase latency" is right, and the honest number
for a phone is minutes.

**A/V:** stays outside shaping, and must. Media chunks ride ~400 B link frames at media rate; in
1,400-byte Poisson envelopes they would cost 3.5× bandwidth and seconds of jitter. A call is a
burst a GPA can see regardless (modelled, not measured; CC 5.3.3.2 real-time path).

**Link-frame and link-setup level:** no. Envelope shaping runs INSIDE established links. What a GPA
sees today (answer 5): the RNS link request to a 16-byte derived destination in the clear, the
link proof, then sealed frames. Padding the frames does not hide which node dialled which derived
address, nor when. Hiding that needs one of: long-lived links with cover on every link to every
peer (edge already pools idle links, #531, so setup is rare; constant-rate cover per link then
hides when a link is "used", but not which links exist), or routing every dial through a relay
(option 2/3), or a uniform "dial budget" of dummy link opens. None exists; all are modelled only.

## B. Thresholds (MODELLED)

Publisher anonymity under (1): an unshaped publisher's burst identifies it (anonymity set 1).
A shaped publisher is hidden among the shaped nodes whose timers could have fired. With a GPA able
to localise the ingress region (20 regions, uniform), the set is the shaped nodes in that region:

| N | shaped fraction | regional anonymity set | long-term intersection breaks after (publication epochs) |
|---|---|---|---|
| 10³ | 5 % / 20 % / 50 % | 2.5 / 10 / 25 | 6 / 100 / 625 |
| 10⁴ | 5 % / 20 % / 50 % | 25 / 100 / 250 | 625 / 10⁴ / 6×10⁴ |
| 10⁵ | 5 % / 20 % / 50 % | 250 / 1,000 / 2,500 | 6×10⁴ / 10⁶ / 6×10⁶ |

(Statistical-disclosure order estimate: epochs ≈ set² / messages-per-epoch; a publisher that
publishes to a fixed audience once per epoch.) Timing correlation across a single sealed link is
not resisted by (1) at all (same link, same two endpoints); it is resisted only once the flow is
spread over relays with per-hop Poisson delay (Loopix model): the expected crowd per relay per
mixing window must be ≫ 1, i.e. users × rate × window / relays ≥ ~5.

Relays for (2)/(3): with users sending 0.5 env/s and a 2 s per-hop window, 3 relays serve 100 users
(crowd 33), 10 relays serve 100 (crowd 10), 30 relays need ≥ 1,000 users (crowd 33); 100 relays
with 100 users are too thin (crowd 1). More relays than traffic HURTS mixing. Rule of thumb:
relays ≈ users/30 at that rate, never below 3.

Where it stops: long-term intersection wins once the attacker's observation epochs exceed the
numbers above; with N = 10³ and < 20 % shaped, that is days. Regional clustering divides the set
by the number of distinguishable regions; it is the dominant term at small N.

## C. Mixing with partial adoption (MODELLED)

- A shaped node is distinguishable from an unshaped one (a constant 1,400-byte Poisson stream vs
  bursty variable frames). Partial shaping therefore publishes the SET of shaped nodes exactly, and
  hides content/recipients only within that set.
- A shaped node talking to an unshaped peer is exposed again on that pair: the peer's bursty,
  timed replies correlate with the shaped node's real envelopes. Shaping must be on at both ends of
  any link that should be private, so (1) is pairwise, not per node.
- (2) without (1): relays and their clients are visible by traffic shape. (3) without (1): the
  path is hidden but the FACT of using onion routing is not (bursts to known relays).
- Minimum sensible bundle: (1) on every link that matters; (2) and (3) add value only on top of (1).
  The wizard should present (3) as implying (1).

## D. Local estimate of adoption (MODELLED)

- (1) and (2) are observable classes: any node can recognise a shaped peer by its envelope stream
  and a relay by its relayed load, so a node can estimate |shaped| and |relays| in its region by
  observation, and a GPA can at least as well. That count is honest to show the user.
- (3) is NOT countable by observation once (1) is on; that is the property. A signed "I use (3)"
  declaration row would publish exactly what (3) hides, so no.
- A privacy-preserving count of (3) needs aggregation: k-anonymous coarse-region buckets published
  only when ≥ k participants, over the existing community/family planes, or secure aggregation /
  PIR (out of scope). Recommendation: show |shaped_region| and |relays_region| (observable),
  and for (3) show only the k-anonymous bucket or nothing.

## What to build first, if the steward proceeds

1. Wire `emission::Scheduler` into the replication send path for maintenance kinds only, per scope,
   default off; expose λ and the budget window in config; measure on bench-mesh before any default.
2. Treat shaping as a link property negotiated in the CTKX handshake (both ends or neither), so a
   shaped node never pairs with an unshaped one silently.
3. Relays (2) are "ordinary sealed, attributed identity-plane links" today already; what is missing
   is the onion packet format. For (3) the FSD's Sphinx needs a KEM variant (Outfox-class over
   ML-KEM/X-Wing): no implementation exists in any CIRIS repo; it is a design + crypto review task.
