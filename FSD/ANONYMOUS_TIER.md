# FSD: Anonymous Tier (opt-in shaping, relaying and onion routing)

**Status at CC 1.0-rc9: planned opt-in tier; implementation expected
soon (steward ruling, 2026-10-21).** CC 1.13.3.1 names it ("the
CIRISNodeCore Anonymous Tier, Sphinx onion routing") as the separate,
opt-in mechanism for deployments that need unobservability against a
global passive adversary (GPA) at federation scope, and the CC 1.13.3.3
operator guidance says such a deployment MUST layer it. The shaping
study was run on 2026-10-21 (§10). The tier is not yet specified
normatively in CC and not yet shipped; nothing in it is a default or
mandatory today. CC still needs three things before it ships (§13): a
CC text change admitting the tier as the 1.13.3.1 mechanism, a ruling
on the row expression of §5, and the CC 5.4.3 cover reference pointed
here. Reconciled to the CIRIS Constitution 1.0-rc9 (tag `v1.0-rc9`);
text marked *rc10* cites later steward rulings on the CC `rc10`
branch. Where this document and CC differ, CC wins.

This revision rebuilds the tier on CEWP as shipped. It replaces the
earlier draft's parallel record classes (`AnonymousContribution`,
`AnonymousHoldsBytes`), parallel persist tables and Ed25519 key
blinding; §11 records why.

**What CC provides without this tier.** Anonymity toward outsiders
defaults to the smallest cohort scope; federation scope is the opt-in
(CC 1.13.3.4). Anonymity there means outsiders cannot see a cohort's
content or map its addresses to it, never an unattributed sender:
every link is attributed to a key, and a first contact is unowned but
never anonymous (CC 1.13.3.4 *rc10*; CC 3.2 adding a device (e); CC
5.3.3.5 E1). Group destinations are derived, never announced
(CC 5.4.6). The identity plane is public and makes no anonymity claim
(CC 5.4.6 *Position*). See [SCOPE_PRIVACY.md](SCOPE_PRIVACY.md).

**Companion:** [FEDERATION_SCALING_MODEL.md §9.9](FEDERATION_SCALING_MODEL.md);
evidence: [benchmarks/anonymous-tier-shaping-2026-10-21.md](../benchmarks/anonymous-tier-shaping-2026-10-21.md).

---

## 1. Threat model

The adversary is a state-level actor with legal coercion authority and
full forensic access to a CIRIS node operator's hardware. It can:

* subpoena the federation directory (the identity plane is public by
  design, CC 5.4.6 *Position*);
* compel an operator to produce disk, keys and logs;
* coerce an operator to reveal whose data they hold;
* compel an operator to cooperate actively (for example, emit rows
  under duress);
* observe links in its own jurisdiction, and at the limit every link
  (the GPA of CC 1.13.3.1).

The protected parties are the **publisher** (a dissident, journalist,
whistleblower) and the **operator**, who must be able to say truthfully:
"I cannot tell who published what my node holds or relays, because
the substrate gives me no way to know."

What each opt-in buys against this adversary is in §1.1. Out of scope
for every option: compromise of the publisher's own device, and
publisher operational-security failures (SCOPE_PRIVACY §7.2).

### 1.1 What the tier does and does not claim

| Option | Hides | Does not hide |
|---|---|---|
| (1) shaping | which of the shaped nodes in a region emitted a given burst; real vs cover on a shaped link | that a node shapes (the shaped set is visible); which links exist and when they were set up (§10.4); anything on a link to an unshaped peer |
| (2) relaying, on top of (1) | which shaped client a relay's traffic came from, within the relay's crowd | that a node relays; the client↔relay link |
| (3) onion routing, on top of (1) | the publisher's network origin from the storage node and every single relay; timing correlation once per-hop delay mixes a crowd (§10.3) | long-term intersection over many publication epochs (§10.3); that the user reaches known relays, if (1) is off |

No option defeats long-term intersection attacks by a GPA. The CC
1.13.3.4 boundary at GPA-unobservability is provisional and MAY move
toward more default protection as the overhead is measured; §10 is the
first such measurement. CC rc9 has not moved it.

## 2. Posture: shaping for every node, anonymous publishing opt-in

**Steward ruling, 2026-10-21 (option "b").** Every node may shape its
existing traffic; publishing anonymously stays opt-in. The client
setup wizard offers three opt-ins:

1. "Shape your traffic to make the network more private (may increase
   latency)."
2. "Allow anonymous peers to route through you."
3. "Turn on onion routing (increases latency 3x or more)."

Rules that follow from the evidence (§10.5):

* **(3) implies (1).** Without shaping, a user's bursts to known
  relays reveal that they use onion routing even though the path is
  hidden. The wizard turns (1) on when (3) is chosen.
* **Shaping is a link property, not a node property.** It is
  negotiated in the CTKX handshake, both ends or neither. (CTKX is the
  study's name; this document reads it as the per-link transit KEX that
  runs as each link's first frame, CC 5.3.3.5 E1 and CC 3.3.6.2.) A
  shaped node talking to an unshaped peer is exposed on that pair,
  because the peer's bursty replies correlate with the shaped node's
  real envelopes; a shaped node never pairs silently with an unshaped
  one.
  (CTKX is edge's transit-key handshake frame, CIRISEdge
  `src/transport/av_transit_handshake.rs` and `link_seal.rs`; the E1
  seal refuses a first frame that is not CTKX.)
* **(2) and (3) add value only on top of (1).** A relay without
  shaping, and its clients, are visible by traffic shape.

Why shaping is offered to every node rather than only to publishers:
a shaped publisher hides only among the other shaped nodes in its
region (§10.3), and the set of shaped nodes is itself visible (§10.5).
Shaping by ordinary users is what makes a publisher's shaping worth
anything, and it gives operators the cover the deniability argument
needs ("I shape because many nodes do").

## 3. Cover: the Poisson emission discipline (option 1)

CC 5.4.3 makes two things normative for substrate emission: a fixed
1.4 KB envelope, and uniform XChaCha20-Poly1305 framing for real and
cover envelopes with `type=cover` inside the AEAD-protected header. Its
size rule names "the companion Poisson-cover emission discipline",
with substrate-maintenance throughput as the budget anchor, and does
not define it. That discipline is this tier's option (1), not a
default at every scope.

The discipline is built: CIRISEdge `src/emission/` (CIRISEdge#175,
introduced in v6.1.0) has the 1,400-byte envelope, fragmentation and
reassembly, per-scope `Exp(λ)` timers from a peer-local CSPRNG, cover
on empty timer fire, and the `BudgetMeter` that holds the
lifetime-average rate per scope. Maintenance traffic (anti-entropy,
repair, witness chains) is the cover budget. The module is
unit-tested and exported, and is not wired into any production send
path (CIRISEdge v43.1.1); SCOPE_PRIVACY §3.1 describes the same
discipline.

**Audio and video stay outside shaping.** Media chunks ride small link
frames at media rate; in Poisson envelopes they would cost about 3.5×
bandwidth and seconds of jitter, and a call is a burst a GPA sees
regardless (§10.1; CC 5.3.3.2 is the real-time path).

## 4. Transport: onion routing inside the per-link seal (options 2, 3)

**Every hop is an ordinary link.** Onion routing runs inside the
per-link hybrid seal: each hop is a sealed, attributed identity-plane
link between two relays, keyed by an X25519 + ML-KEM-768 exchange
bound to the link (CC 5.3.3.5 E1). A relay is bound by the transit KEX
pair on its signed `transport_destination` binding, and a node with no
valid transit pair cannot relay (CC 5.3.3.5 E1, ruling 2026-10-13). So
"every link attributed to a key" holds at every hop; what the onion
layer hides is the end-to-end path, not any link's endpoints. The
storage node sees the last relay, never the publisher.

**The packet format MUST be post-quantum.** The per-link seal is all
or nothing, with no classical lane (CC 5.3.3.5 E1, steward
2026-10-19). A classical X25519 Sphinx header inside it would make the
path-hiding layer the one classical layer, open to
harvest-now-decrypt-later of routes. The onion packet is therefore a
KEM-based Sphinx variant of the Outfox class ([Rial, Piotrowska,
Halpin, arXiv:2412.19937](https://arxiv.org/abs/2412.19937)) over
ML-KEM-768 or X-Wing, not classical Sphinx.

**No implementation exists in any CIRIS repository.** Relaying in
the sense of (2) is ordinary sealed links today; the onion packet
format is a design and crypto-review task.

**Mixing.** Shaping alone does not resist timing correlation across
one sealed link. That needs the flow spread over relays with per-hop
Poisson delay, in the Loopix model ([Piotrowska et al., USENIX
Security 2017](https://www.usenix.org/conference/usenixsecurity17/technical-sessions/presentation/piotrowska)),
with enough crowd per relay (§10.3).

**The CC 5.4.6 bar.** Multi-hop reach is an amendment-plane question
whose bar is no outsider-observable emission, no outsider-retained
path state and no epoch-correlated wave; the nearest admissible
relaxation is the Tor v3 / I2P b33 blinded-retained-state family,
with rotation clocks global (CC 5.4.6). That bar is stated for
group-scoped destinations; this tier publishes at federation scope,
but it adopts the rotation rule (§9) and the no-epoch-wave rule.

## 5. Identity and rows: inside the five, no new type

**Planned expression, pending ruling.** The steward deferred a formal
ruling on this until after the persist refactor. What follows is the
planned expression, not a ruled one.

### 5.1 A fresh hybrid key per publication

Each publication is signed by a fresh hybrid key pair (Ed25519 +
ML-DSA-65), derived deterministically from a secret seed only the
publisher holds and a global time period or a per-publication nonce.
Both halves derive from 32-byte seeds: Ed25519 natively ([RFC 8032
§5.1.5](https://www.rfc-editor.org/rfc/rfc8032#section-5.1.5)) and
ML-DSA by its seeded key generation ([FIPS 204
§5.1](https://csrc.nist.gov/pubs/fips/204/final)). The KDF that splits
the publisher's seed into the two is a design choice still to make.

This replaces Tor-style Ed25519 key blinding, which has no ML-DSA
counterpart. A classical-only signature is not admitted at the
federation tier: every federation record is signed Ed25519 + ML-DSA-65
(CC 5.3.3.5 E1, citing CC 2.6.1.2).

**Consequence for rendezvous.** With blinding, anyone could derive a
publication key from a public identity and a time period. With seeded
derivation nobody but the publisher can, so rendezvous uses a token or
a published index secret, never a derived key (§7).

### 5.2 The rows

A publication is two rows of existing types:

1. a `key:record:v1` for the fresh key, admitted by proof of
   possession against the keys it carries. This is the one stated
   exception to resolving the attester through a held row (CC 2.4,
   cosigned-rows paragraph; CC 3.1 `key:record`), and proof of
   possession checks the `key_id` fingerprint against the public key
   (CC 3.2 adding a device (e); CC 2.6.8);
2. an ordinary `scores` row carrying the content, signed by that key.
   CC 2.4 admits a row only once its signer's key-claim row is held,
   so (1) travels with or before (2).

No row type is added; the CC 2.4 slot stays closed to the five. The
earlier draft's `AnonymousContribution` and `AnonymousHoldsBytes` are
retired. Blobs stay on the existing blob plane, addressed by content
hash; persist needs no parallel table.

`withdraws`, `supersedes`, `delegates_to` and `recants` need no
anonymous counterparts. A publisher who keeps the seed can supersede or
withdraw under the same fresh key.

## 6. Reach: consent-bounded, never pushed

The fresh key belongs to no trust root. A row reaches a reader only
when the reader trusts the emitter's root or has consented to receive
from it, and reaches nobody else (CC 2.4.1.2.1, citing CC 4.4.3.8).
Anonymous-tier rows therefore reach only readers who consented to
receive the anonymous tier. Nobody receives anonymous content without
opting in, and a fresh key carries no weight in any reader's trust
set (CC 4.4.3.8 Policy A).

## 7. Publication modes, scanning and takedown

**Open publication** (anyone holding the public rendezvous material
can find and read it) is public content. It is admitted as a Commons
post, the holder inspects it (CC 4.4.3.2.1: Commons is plaintext and a
node never holds bytes it does not know what they are), and the
perceptual-hash tripwire runs at its admission, the scope-widening
seam (CC 1.13.3.4). The publisher is hidden; the content is not.

**Targeted delivery** (a token known only to the recipient, after the
Signal sealed-sender model) is a private message to that recipient and
is never scanned (CC 1.13.3.4).

**Takedown by digest still works.** The takedown and purge tombstone
names plane, target digest and the authorising notice, never content
and never the author (CC 4.5.3), so it applies to an anonymous
publication unchanged. On *rc10* (steward ruling 2026-10-21), an
expeditious-with-counter-notice basis withholds the bytes for its
window (`Dmca512` 10 days, `DsaArticle16` and `OsaIllegalContent` 14,
`CommunityStandards` 30, counted from each holder's execution of the
tombstone) before deletion, and an admitted `reconsideration` with
outcome `reversed` restores standing (CC 4.5.3).

The old draft's "LRU and capacity only" abuse posture is withdrawn:
the tier has the tripwire, takedown and consent-bounded reach.

## 8. Holding claims (open)

The one holding claim is `custody:ack:v1`, signed by the holder's own
device key (CC 3.1.3.3); at Commons scope it is the holder directory
(CC 4.4.3.2.1). For anonymous blobs that names, by node key, who holds
them, which the threat model forbids. Two options, not yet chosen:

* **per-blob fresh holder keys**, each its own `key:record:v1` by proof
  of possession, signing that blob's `custody:ack:v1`; or
* **no advertisement**: holders emit no custody row for anonymous
  blobs, and fetch goes only through the rendezvous.

## 9. Rotation

Publication keys (when derived per period) and rendezvous indexes
rotate on global time periods, never on group events (CC 5.4.6
*Position*: "rotation clocks must be global, never group-event-driven").

## 10. Evidence: the shaping study (2026-10-21)

Source: [benchmarks/anonymous-tier-shaping-2026-10-21.md](../benchmarks/anonymous-tier-shaping-2026-10-21.md),
run by CIRISEdge at v43.1.1 with edge's emission primitives. MEASURED
means run with those primitives on a virtual clock over real rows
from a CIRISServer test mesh; MODELLED means arithmetic from stated
assumptions. No implementation was done.

### 10.1 Cost of (1)

* **CPU (MEASURED):** seal 2.88 µs, unseal 2.05 µs per envelope; about
  3 µs, negligible at any λ. On a phone (MODELLED) ×3–5, still
  negligible. The phone cost is the radio: a timer at λ ≥ 0.2/s keeps
  it awake.
* **Padding (MEASURED):** 1.19× overall on 50 real rows. Large
  hybrid-signed rows pad 1.10–1.13×, small rows up to 1.34×, a small
  Diff frame 4.67×. Total line rate is fixed at λ × 1,400 B/s whatever
  the real volume.

### 10.2 Latency added by the Poisson timer (MEASURED)

| Node | λ (env/s) | line rate | mean delay | p95 delay |
|---|---|---|---|---|
| server | 1 | 1.4 KB/s | 25–49 s | 128–158 s |
| server | 2 | 2.8 KB/s | 12–18 s | 52–99 s |
| phone | 0.2 | 280 B/s | 195–482 s (3–8 min) | 430–1,165 s |

Ranges span load ρ = 0.25–0.8. A multi-fragment row waits several
timer fires, so p95 is 4–5× the mean. For a server this is acceptable
for anti-entropy (today's rounds are 30 s); a chat message becomes a
10–60 s delivery. A phone cannot sustain λ ≥ 1/s on battery.

### 10.3 Thresholds (MODELLED)

A shaped publisher hides among the shaped nodes in the region a GPA
can localise (20 regions assumed):

| N | shaped | regional anonymity set | intersection breaks after (epochs) |
|---|---|---|---|
| 10³ | 5 / 20 / 50 % | 2.5 / 10 / 25 | 6 / 100 / 625 |
| 10⁴ | 5 / 20 / 50 % | 25 / 100 / 250 | 625 / 10⁴ / 6×10⁴ |
| 10⁵ | 5 / 20 / 50 % | 250 / 1,000 / 2,500 | 6×10⁴ / 10⁶ / 6×10⁶ |

At N = 10³ with under 20 % shaped, intersection wins in days. Regional
clustering is the dominant term at small N.

**Relay crowd rule.** Per-hop mixing needs an expected crowd per relay
per window of about 5 or more. At 0.5 env/s per user and a 2 s window:
relays ≈ users/30, never fewer than 3. Too many relays for the traffic
thins every crowd and hurts mixing (100 relays for 100 users gives a
crowd of 1).

### 10.4 What shaping does not hide (MODELLED)

Shaping runs inside established links. A GPA still sees the RNS link
request to a 16-byte derived destination, the link proof, and when it
happened. Hiding link setup needs relays, or long-lived links with
per-link cover (edge pools idle links, CIRISEdge#531, so setup is
rare), or a dummy-dial budget. None is built.

### 10.5 Partial adoption and local estimates (MODELLED)

* A shaped node is distinguishable from an unshaped one, so partial
  shaping publishes the shaped set and hides content and recipients
  only within it.
* A shaped↔unshaped pair is exposed (§2).
* (1) and (2) are observable classes: a node can count shaped peers
  and relays in its region, and a GPA can count them at least as well,
  so showing that count to the user is honest.
* (3) is not countable by observation once (1) is on. A signed "I use
  (3)" declaration row would publish what (3) hides, so there is none.
  A count of (3) is shown only as a k-anonymous coarse-region
  aggregate, published when at least k participants contribute, or not
  at all.

## 11. Wizard copy guidance

* On a phone, option (1) says **minutes**, not "may increase latency"
  (§10.2).
* Show the user their region's observed counts of shaped nodes and
  relays (§10.5).
* Say that protection grows as more people turn shaping on (§10.3).
* Option (3) says it turns (1) on (§2).

## 12. Build order (edge, if the steward proceeds)

1. Wire `emission::Scheduler` into the replication send path for
   maintenance kinds only, per scope, default off; expose λ and the
   budget window in config.
2. Negotiate shaping in the CTKX handshake, both ends or neither.
3. Measure on bench-mesh before any default.
4. Then the post-quantum onion packet format (§4): design and crypto
   review first.

## 13. What CC still needs, and acceptance

None of these is met at CC 1.0-rc9.

- [ ] A CC text change admitting the tier as the CC 1.13.3.1
  mechanism: the three opt-ins, shaping as a CTKX link property, the
  post-quantum onion packet over the E1 seal.
- [ ] A ruling on the row expression of §5 (fresh `key:record:v1` by
  proof of possession plus `scores`), deferred by the steward until
  after the persist refactor.
- [ ] The CC 5.4.3 "companion Poisson-cover emission discipline"
  pointed at this tier's option (1).
- [ ] Edge ships §12 steps 1–3, measured on bench-mesh.
- [ ] A post-quantum onion packet format is specified, reviewed and
  implemented.
- [ ] Cross-repo test: an anonymous publication, then forensic
  inspection of the storage node and of each relay recovers neither
  the publisher's identity nor its network origin.

## 14. Open questions

1. **Holding claims** (§8): per-blob fresh holder keys, or no
   advertisement with fetch only through the rendezvous?
2. **Consent row**: which row expresses a reader's consent to receive
   the anonymous tier (§6)? CC states the reach rule, not this
   consent's family.
3. **Takedown authority**: CC 4.5.3 requires the claimant to hold the
   `takedown` duty over the content's community. Whose duty covers an
   open anonymous publication that belongs to no community, beyond a
   trust root's `steward_purge`?
4. **Key record contents**: what `identity_type` and roles does a
   publication key's `key:record:v1` carry, so that it links to
   nothing and trips no steward-binding gate (CC 3.2)?
5. **Fixed envelopes everywhere?** CC 5.4.3 already says all outbound
   substrate traffic MUST use the 1.4 KB envelope, while edge does not
   wire it. Does that MUST stay universal, with only the Poisson timing
   in option (1), or move with it?
6. **Onion packet**: Outfox over ML-KEM-768 or over X-Wing; hop count
   (3 is the Tor and Sphinx convention; the wizard's "3x or more"
   assumes it); per-hop delay parameters.
7. **Rendezvous**: token (targeted) and published index secret (open),
   or one of them? Does a rendezvous lookup ride the onion transport,
   and what protects lookup patterns?
8. **Link-setup leak** (§10.4): relays only, per-link cover, or a
   dummy-dial budget?
9. **Phone posture**: what λ does a phone run, given that the radio,
   not the CPU, is the cost?

## 15. Prior art

| Primitive | Reference design | Status in CIRIS |
|---|---|---|
| Sender-side Poisson cover, per-hop mixing | [Loopix](https://www.usenix.org/conference/usenixsecurity17/technical-sessions/presentation/piotrowska) | Cover built in edge `src/emission/`, not wired; mixing not built |
| Onion packet format | [Sphinx (Danezis, Goldberg)](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf); post-quantum: [Outfox](https://arxiv.org/abs/2412.19937) | None |
| Private routes for opaque storage | [Veilid private routing](https://veilid.com/how-it-works/private-routing/) | None |
| Per-publication keys | [Tor rend-spec-v3 key blinding](https://spec.torproject.org/rend-spec/deriving-keys.html) | Replaced by seeded hybrid keys (§5.1) |
| Sender deniability, targeted delivery | [Signal sealed sender](https://signal.org/blog/sealed-sender/) | New rendezvous protocol |
| Time-rotated index | [Tor rend-spec-v3 HSDir indexing](https://spec.torproject.org/rend-spec/deriving-keys.html) | New rendezvous protocol |

CC 5.4.6 names the Tor v3 / I2P b33 blinded-retained-state family as
the nearest admissible relaxation for multi-hop reach. Signal runs
sealed-sender and identified delivery on one server, and Tor runs
onion services beside identified relays; this tier follows the same
pattern of an opt-in path on one substrate.

## 16. Cross-references

* CIRIS Constitution 1.0-rc9 (tag `v1.0-rc9`): CC 1.13.3.1,
  CC 1.13.3.3, CC 1.13.3.4, CC 2.4, CC 2.4.1.2.1, CC 2.6.8,
  CC 3.1.3, CC 3.1.3.3, CC 3.2, CC 3.3.6.2, CC 4.4.3.2.1,
  CC 4.4.3.8, CC 4.5.3,
  CC 5.3.3.2, CC 5.3.3.5, CC 5.4.3, CC 5.4.6; `rc10` branch for the
  2026-10-21 rulings in CC 1.13.3.4 and CC 4.5.3
* [SCOPE_PRIVACY.md](SCOPE_PRIVACY.md): the default construction below
  federation scope; §3.1 describes the cover discipline
* [FEDERATION_SCALING_MODEL.md §9.9](FEDERATION_SCALING_MODEL.md): the
  scaling-model derivation that motivated the tier
* [benchmarks/anonymous-tier-shaping-2026-10-21.md](../benchmarks/anonymous-tier-shaping-2026-10-21.md):
  the shaping study
* CIRISEdge `src/emission/` (CIRISEdge#175) and `docs/LIGHTNET_DARKNET.md`
