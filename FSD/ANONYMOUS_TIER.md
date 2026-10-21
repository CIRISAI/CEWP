# FSD: Anonymous Tier (the opt-in GPA-unobservability path)

**Status at CC 1.0-rc9: planned opt-in tier; implementation expected
soon (steward ruling, 2026-10-21).** CC 1.13.3.1 names it ("the
CIRISNodeCore Anonymous Tier, Sphinx onion routing") as the separate,
opt-in mechanism for deployments that need resistance to a global
passive adversary at federation scope, and CC 1.13.3.3 says such a
deployment MUST layer it. It is not yet specified normatively in CC
and not yet shipped. It is opt-in, not a default. This document is the
plan of record. It must meet the bar CC 5.4.6 sets for multi-hop
reach: no outsider-observable emission, no outsider-retained path
state, no epoch-correlated wave, and rotation clocks global, never
group-event-driven (CC 5.4.6 names the Tor v3 / I2P b33
blinded-retained-state family as the nearest admissible relaxation).
Reconciled to the CIRIS Constitution 1.0-rc9 (tag `v1.0-rc9`); where
this document and CC differ, CC wins.

**What the tier is for.** The CC 1.13.3.1 residual:
unobservability against a global passive adversary at federation
(public-commons) scope. As planned (3-hop onion routing, blinded keys,
AEAD, rendezvous) it hides the publisher's network origin and identity
from holders, the directory and a subpoena; it does **not** by itself
defeat a global passive adversary, which §1 and §2.4 below exclude.
Closing that residual would need the costs CC 1.13.3.4 names for it:
onion-routing latency, mixing and dedicated cover. Earlier versions of
this header claimed full GPA-unobservability while §2.4 disclaimed it;
the header is corrected to match §2.4.

**What CC provides without this tier.** Anonymity toward outsiders
defaults to the smallest cohort scope; federation scope is the opt-in
(CC 1.13.3.4). Content born and living at `self` / `family` /
`community` emits no holding claim outside its cohort (CC 5.2), and
group destinations are derived, never announced, so a group's
existence is invisible to non-members (CC 5.4.6). The identity plane
is public and makes no anonymity claim (CC 5.4.6 *Position*); a
joining device is "unowned but never anonymous" (CC 3.2, adding a
device, (e)). See [SCOPE_PRIVACY.md](SCOPE_PRIVACY.md). Earlier
versions of this header described "structural initiator-anonymity" as
on by default; CC rc9 binds the initiator's key into every sealed
link (CC 5.3.3.5 E1), so that phrase is withdrawn here.

**Companion:** [FEDERATION_SCALING_MODEL.md §9.9](FEDERATION_SCALING_MODEL.md)
**Threat model:** totalitarian government coercing a node operator
  or subpoenaing the federation directory to identify dissident
  publication.

**The provisional line.** CC 1.13.3.4 marks the default/opt-in
boundary as provisional: as the overhead of GPA-resistance is
measured, the line MAY move toward making more of it default, for
example for individual federation-scope publishers. CC rc9 has not
moved it. An earlier CEWP model estimate of the latency and bandwidth
cost of a 3-hop default is in the [interactive toy](../toy/index.html);
it is a model, not a measurement, and CC does not cite it.

The mental model: cohort scopes are the **rooms of your house**
(private toward outsiders by default); federation scope is the
**public square**; this tier is the **anonymous
remailer** you reach the square through when the square itself is
watched. The default builds the private house; this tier adds the
remailer, opt-in.

---

## 1. Threat model

The adversary is a **state-level actor with legal coercion authority
and full forensic access to a CIRIS node operator's hardware**. The
adversary can:

* Subpoena the federation directory (any peer can query the public
  `federation_attestations` table)
* Compel a node operator to produce their disk, keys, and logs
* Coerce a node operator to reveal whose data they're holding
* Compel a node operator to actively cooperate (e.g. emit
  attestations under duress)

The protected party is the **publisher** (a dissident, journalist,
whistleblower) and the **operator** (who must be able to truthfully
claim: "I don't know whose content is on my disk because the
substrate itself prevents me from knowing").

Out of scope for this FSD: traffic analysis attacks beyond the
substrate (e.g. global passive adversaries correlating onion-route
entry/exit timing); compromised onion-route relays (handled by the
multi-hop routing assumption); endpoint compromise of the
publisher's own device.

## 2. Non-goals

The anonymous tier is **NOT** intended to:

1. **Replace the identity-aware substrate.** The substrate's
   trust-enforceability, abuse-handling, and per-actor eviction
   properties are load-bearing for federation-scale trust contexts.
   The anonymous tier does not weaken any of those.
2. **Provide identity-aware properties for anonymous content.** By
   construction the anonymous tier admits content under blinded
   keys; trust-weighted admission is structurally impossible. The
   only admission gate is capacity (LRU eviction).
3. **Hide the existence of the substrate itself.** Anonymous-tier
   participation is publicly observable at the protocol level; what's
   hidden is *which records belong to which publication*, not *that
   the operator runs the substrate*. This is the Tor-relay posture:
   you publicly run Tor; you privately have no idea which onion
   services use your relay.
4. **Defeat global passive adversaries.** Traffic analysis at
   network-monitor scale is outside this design's threat model (it's
   outside Tor's too). This is why the header above does not claim
   GPA-unobservability for the tier as currently planned.

## 3. Wire format additions

The original draft made the tier additive: parallel record classes
beside the existing ones. **At CC 1.0-rc9 that shape is closed.** The row-type
slot admits exactly the five primitives (`scores`, `delegates_to`,
`supersedes`, `withdraws`, `recants`), a substrate MUST refuse any
other row type at admission, and the rc6 "carrier" class is gone
(CC 2.4). Implementing the records below therefore needs an amendment to
CC 2.4 (or an expression inside the five), not an additive extension.

### 3.1 `AnonymousContribution`

Parallel to the `Contribution` envelope, with the identity-
attribution surface replaced by cryptographic blinding:

```
AnonymousContribution {
  // Same as Contribution
  contribution_id: ULID,
  contribution_type: ContributionType,
  subject: Cell,
  payload: JSON,
  submitted_at: DateTime,

  // Different from Contribution
  blinded_signing_key: Ed25519PubKey,   // ← per-publication, NOT author_id
  signature: HybridSignature,           // ← signed by blinded key
  rendezvous_token: Bytes,              // ← Signal-style delivery token OR
                                        //   Tor-style time-rotated index
  payload_aead_envelope: Bytes,         // ← XChaCha20-Poly1305 keyed by
                                        //   KDF(blinded_signing_key, period)
}
```

There is **no `author_id` field**. The publisher signs with a
freshly-derived blinded Ed25519 key (Tor-style key blinding from a
long-term anonymity identity the substrate never sees). Different
records from the same anonymity identity use different blinded
keys, unlinkable without the underlying anonymity identity.

### 3.2 `AnonymousHoldsBytes`

Drafted as a parallel to the `holds_bytes:sha256:{prefix}`
attestation, with `attesting_key_id` replaced by a node-blinded key.
**`holds_bytes` is retired** (rc7): the one holding claim at every
tier is `custody:ack:v1`, placed at the content's own scope, with a
72-hour freshness window as the row's `expires_at` (CC 3.1.3.3; CC 2.4
records why the carrier class is not to be reinvented). The
implementation has to restate it against `custody:ack:v1`.

```
AnonymousHoldsBytes {
  blob_sha256: [u8; 32],
  // attesting_key_id REMOVED — replaced by:
  node_blinded_key: Ed25519PubKey,   // ← per-blob, NOT operator's identity
  signature: HybridSignature,        // ← signed by node-blinded key
  asserted_at: DateTime,
  expires_at: DateTime,              // ← drafted as a 24h TTL; the rc9
                                     //   custody:ack:v1 window is 72h
}
```

The operator's federation identity is NOT in the attestation. Each
held-blob advertisement uses a different blinded key, derived per-
blob from the operator's anonymity identity. The federation can see
that *somebody* is holding blob X (necessary for discoverability)
but cannot link that holder to any other holder or to a federation
identity.

### 3.3 No new structural primitives

`withdraws` / `supersedes` / `delegates_to` / `recants` do NOT have
anonymous counterparts; those primitives are intrinsic to the trust
graph, which the anonymous tier explicitly does not have. Anonymous
content is evicted by LRU; there is no anonymous-tier governance.

## 4. Cryptographic primitive set

Four primitives, all standard:

### 4.1 Ed25519 key blinding (Tor v3 model)

Per-publication signing keys derived from an anonymity identity +
a time period or per-publication nonce. Different records from
the same anonymity identity use unlinkable blinded keys.

* Anchor: [Tor rend-spec-v3 §A.2 Key Derivation](https://spec.torproject.org/rend-spec/deriving-keys.html)
* Already present: CIRISVerify ships Ed25519 sign/verify
* New work: the blinding derivation function + period-rotation policy
* rc9 constraint: every federation record is signed Ed25519 +
  ML-DSA-65 (CC 5.3.3.5 E1, citing CC 2.6.1.2). An Ed25519-only
  blinded key would not meet that; see §9 question 8.

### 4.2 AEAD payload encryption (Veilid model)

XChaCha20-Poly1305 over the payload, keyed by `KDF(blinded_pk,
period)` so the storage node never sees the underlying key. The
storage node holds opaque ciphertext.

* Anchor: [Veilid: Cryptography](https://veilid.com/how-it-works/cryptography/)
* Already present: CIRISVerify ships AES-256-GCM, and
  XChaCha20-Poly1305 is the CC 5.4.2 symbol AEAD. Either works
  (AES-GCM is faster on hardware AES; ChaCha is faster on
  embedded). Decision deferred to §9.
* New work: the subcredential-derivation KDF

### 4.3 Onion-routed write path (Sphinx or Veilid private routes)

The publishing node's IP is not the record's network origin. Either
Sphinx (the cryptographic packet format Tor uses) or Veilid-style
private routes (3-hop onion-encrypted route blobs).

* Anchor: [Sphinx packet format paper (Danezis & Goldberg)](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf),
  [Veilid: Private Routing](https://veilid.com/how-it-works/private-routing/)
* Already present: none; this is the most substantial addition
* New work: a new transport class alongside Reticulum + HTTPS in
  edge. Sphinx is well-specified and has reference implementations
  (Lightning's onion routing is a direct adaptation). 3-hop default.
* rc9 constraint: every byte an edge sends over an RNS link is under
  the hybrid per-link transit seal, all or nothing (CC 5.3.3.5 E1).
  CC 5.4.6 sets the bar any multi-hop amendment must meet for
  group-scoped destinations: no outsider-observable emission, no
  outsider-retained path state, no epoch-correlated wave.

### 4.4 Discoverability rendezvous

Either model is workable:

* **Signal model**: recipient-derived 96-bit delivery token from
  the recipient's profile key. Targeted publication (publisher
  knows who they're sending to).
* **Tor model**: time-rotated blinded index. Open publication
  (anyone deriving the right blinded key from public material +
  time can find the record).

Both serve different use cases (targeted vs broadcast); a design
could support both. CC 5.4.6 adds a rule for any rotation in this
family: rotation clocks must be global, never group-event-driven.

* Anchors: [Signal: Sealed Sender](https://signal.org/blog/sealed-sender/),
  [Tor rend-spec-v3 §B.2 HSDir indexing](https://spec.torproject.org/rend-spec/deriving-keys.html)
* New work: the rendezvous protocol + how the substrate routes
  rendezvous queries (probably a parallel directory namespace,
  `federation_anonymous_rendezvous` table or similar)

## 5. Substrate impact (planned)

### 5.1 Persist

A parallel storage surface for opaque-blinded records:

* **New table:** `federation_anonymous_blobs`: opaque blob storage
  indexed by SHA256, no signer attribution. Same `BlobBody` type as
  `federation_blobs`.
* **New trait method on `BlobStorage`:** `put_anonymous_blob(sha,
  body, media_type, blinded_signature)`: no trust-score lookup,
  capacity-only admission. Eviction is LRU-only since there's no
  attestation chain to score popularity against.
* **New table:** `federation_anonymous_attestations`: holds
  `AnonymousHoldsBytes` records. Indexed by `blob_sha256` (so
  `list_holders` works) but with `node_blinded_key` instead of
  `attesting_key_id`. Each entry is independently unlinkable from
  every other.
* **No `TrustScoring` consultation.** Admission is purely capacity.
  Eviction is purely LRU + the holder advertisement's TTL.

At rc9 this conflicts with the holder-inspectability principle: a
node never holds bytes it does not know what they are, and "nothing is
ever an opaque blob held by a stranger" (CC 4.4.3.2.1). Capacity-only
admission of opaque federation-scope ciphertext is exactly the case
that principle excludes; the implementation has to answer it (§9 question 9).

### 5.2 Edge

* **New transport class**: Sphinx-onion-routed delivery alongside
  Reticulum + HTTPS. Likely a separate cargo feature.
* **New MessageType**: `AnonymousContributionSubmit` carrying the
  onion-routed payload + rendezvous token.
* **New handler**: receives `AnonymousContributionSubmit`,
  verifies the blinded-key signature, calls
  `persist::put_anonymous_blob`. No trust-score gate.
* **Cohort scope:** anonymous content is conceptually `federation`
  scope (publishable to anyone who knows the rendezvous), but the
  scope field is meaningful only for the substrate's
  discoverability gate. The anonymous tier's discoverability is
  out-of-band.

### 5.3 NodeCore

* **New `#[pyfunction]`**: `publish_anonymously(content_bytes,
  rendezvous_recipient_pubkey | rendezvous_period, signer)`.
  Internally:
  1. Derive blinded signing key from `signer` + nonce
  2. Encrypt payload via XChaCha20-Poly1305 keyed by KDF(blinded_pk)
  3. Construct `AnonymousContribution` envelope
  4. Sign with blinded key
  5. Hand to edge for onion-routed delivery
* **No replication policy**: node-core produces wire artifacts; the
  substrate decides where they live.

## 6. Deployment posture (planned)

The plan is that, once specified in CC and shipped, the anonymous
tier is substrate-level infrastructure that every L0/L1 server runs,
indistinguishably from operators who happen to be holding more
anonymous records vs. fewer. Publishing through it stays opt-in per
publication; running it is what the plan makes universal. CC rc9 does
not yet require any node to run it.

The reasoning: if only some
operators ran the tier, those operators would become identification
targets (raid the nodes that run the deniable storage). The
operator's deniability ("I'm running the standard substrate; I have
no way to know which records on my disk belong to which publication")
only works if running the standard substrate necessarily includes the
tier. This is the Tor relay posture: every Tor relay runs the same
code regardless of which onion services it happens to be holding
descriptors for.

**Operator-side capacity budgeting** is part of the plan:
operators configure how much of their disk goes to
anonymous-tier storage vs. identity-aware storage, with the protocol
running both.

### 6.1 Interplay with fountain storage and the noise floor

The substrate's fountain storage (RaptorQ symbols under the CC 5.4.2
group-keyed AEAD; CC 6.1.5) and the noise floor (CC 6.1.2) operate on
the identity-aware tier. As planned, the anonymous tier sits outside
both:

* **Exempt from swarm / fountain retention.** Anonymous-tier blobs
  are not RaptorQ-coded into the swarm. Fountain addressing is
  group-keyed (CC 5.4.1, CC 5.4.2) and holding claims are
  `custody:ack:v1` rows signed by the holding device (CC 3.1.3.3),
  which the anonymous tier deliberately lacks. An anonymous blob
  lives only where its (capacity-only, LRU) holders put it. This is
  intentional: any-K reconstruction across holders would manufacture
  exactly the cross-holder linkage the blinding severs.
* **Outside the noise-floor descent.** The CC 6.1.2 retirement model
  operates over the identity-aware corpus. Anonymous records retire by
  LRU + holder-TTL only (§5.1), because there is no scoreable identity
  to drive descent.

### 6.2 The noise floor and the threat model

The CC 6.1.2 **noise floor** is the individual-recoverability
boundary: content that is revoked or withdrawn descends below it and
becomes individually unrecoverable while only the collective gist
persists. That boundary is itself a privacy primitive for the
identity-aware tier. For the totalitarian threat model this matters
two ways:

* For identity-aware content, revocation is durable against a
  later-coercing adversary once an item has descended.
* For genuinely deniable publication (dissident / journalist /
  whistleblower under §1's state-level adversary), the noise floor
  is **not** the mechanism, because it still admits forensic recovery
  of an item *above* it by a holder who is coerced. The anonymous
  tier's compelled-disclosure resistance (operator cryptographically
  cannot decrypt) would be the stronger guarantee; the noise floor
  complements it but does not substitute for blinding.

## 7. What the anonymous tier CAN and CANNOT do (planned)

| Property | Identity-aware (main path) | Anonymous tier (planned) |
|---|---|---|
| Trust-weighted admission | Yes | **No**: no identity to score |
| Per-actor eviction | Yes | **No**: operator can't selectively evict; LRU only |
| Operator-side governance | Yes | **No**: operator holds opaque ciphertext |
| Trust graph observable | Yes (by design) | **No**: blinded keys are unlinkable |
| Compelled-disclosure resistance | Limited (operator can be compelled to query their index) | **Strong**: operator cryptographically cannot decrypt |
| Abuse handling | Trust + adjudication | LRU + capacity gating only |
| Discoverability | Federation directory | Out-of-band rendezvous (token or blinded index) |
| Wire format | `Contribution` + `custody:ack:v1` holding claims (CC 3.1.3.3) | `AnonymousContribution` + `AnonymousHoldsBytes` |
| CC 2.4 row-type slot | The five | Would require amending CC 2.4 (the slot is closed) |

## 8. Prior art mapping

Each anonymity primitive maps to an existing well-specified design:

| Primitive | Reference design | Status in CIRIS today |
|---|---|---|
| Per-publication blinded keys | [Tor rend-spec-v3](https://spec.torproject.org/rend-spec/deriving-keys.html) | Ed25519 sign/verify present; blinding derivation new |
| AEAD with subcredential | [Veilid](https://veilid.com/how-it-works/cryptography/), [Tor §B.2](https://spec.torproject.org/rend-spec/hsdesc-encrypt.html) | AES-GCM and XChaCha20-Poly1305 present; subcredential KDF new |
| Onion-routed transport | [Sphinx packet format](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf), [Veilid private routes](https://veilid.com/how-it-works/private-routing/) | New transport class |
| Sender deniability | [Signal sealed sender](https://signal.org/blog/sealed-sender/) | New rendezvous protocol |
| Time-rotated index | [Tor rend-spec-v3 §B.2](https://spec.torproject.org/rend-spec/deriving-keys.html) | New rendezvous protocol |

**No primitive needs to be invented.** Each is standard, well-
specified, and has multiple reference implementations.

The composition (parallel paths on a single substrate, opt-in per
publication) has its own prior art: Signal runs sealed-sender +
identified delivery on one server; Tor runs onion services +
identified relays on the same network; Veilid records can be both
signed-by-known-key and routed-via-private-route on the same node.
CC 5.4.6 names the Tor v3 / I2P b33 blinded-retained-state family as
the nearest admissible relaxation for a future multi-hop amendment.

## 9. Open design questions

These get resolved as implementation starts:

1. **AEAD cipher choice**: XChaCha20-Poly1305 (Veilid; the CC 5.4.2
   symbol AEAD) or AES-256-GCM? Both work; trade-off is hardware-
   acceleration (AES) vs embedded-friendly (ChaCha).
2. **Onion-routing transport**: Sphinx (Tor + Lightning,
   well-specified) or Veilid private routes (newer, less peer-
   reviewed but explicitly designed for opaque storage)?
3. **Rendezvous model**: both (Signal model for targeted; Tor
   model for broadcast) or pick one?
4. **Onion-route hop count**: 3 (Tor / Sphinx standard) or
   different? Trade-off is latency vs adversary coverage.
5. **Anonymous-tier disk budget default**: what fraction of an
   L0/L1 server's disk goes to anonymous storage by default? Too
   low and the tier is starved; too high and identity-aware admission
   is crowded.
6. **Rendezvous lookup transport**: does rendezvous query ride
   the same onion-routed transport as anonymous-publish, or a
   parallel discoverability surface? If the latter, what protects
   query patterns from analysis?
7. **Wire-format integration**: at rc9 the row-type slot is closed to
   the five (CC 2.4). Can the tier be expressed inside the five, or
   does it need an amendment?
8. **Post-quantum blinding**: every federation record is signed
   Ed25519 + ML-DSA-65 (CC 5.3.3.5 E1, citing CC 2.6.1.2). How is a
   blinded per-publication key made hybrid?
9. **Holder-inspectability**: how does capacity-only storage of opaque
   ciphertext square with CC 4.4.3.2.1 ("nothing is ever an opaque
   blob held by a stranger")?
10. **The CC 5.4.6 bar**: no outsider-observable emission, no
    outsider-retained path state, no epoch-correlated wave; rotation
    clocks global.

## 10. Migration path (planned)

The anonymous tier is planned to be additive to deployed nodes:

* **Today's substrate** is the identity-aware substrate (cohabitation
  install, serving install, ingest path). The federation operates
  under trust+capacity discipline.
* **Introduction** ships the tier as an additional
  substrate-level feature. Nodes that do not upgrade can continue
  to serve identity-aware traffic while ignoring anonymous records.
  At rc9 they refuse them at admission, since an unregistered
  row type is refused with `attestation_type_unregistered` (CC 2.4).
* **Adoption gate (planned)**: the deniability argument of §6
  requires every operator to run the tier, so introduction aligns
  with a federation-wide upgrade window. CC rc9 does not yet state
  this gate.

## 11. Acceptance criteria

None of these is met at CC 1.0-rc9.

- [ ] CIRISVerify ships a key-blinding derivation primitive (hybrid,
  per §9 question 8)
- [ ] CIRISVerify ships the AEAD-with-subcredential primitive
  (XChaCha20-Poly1305 or AES-256-GCM keyed by `KDF(blinded_pk,
  period)`)
- [ ] CIRISPersist ships `federation_anonymous_blobs` +
  `federation_anonymous_attestations` tables + `BlobStorage::
  put_anonymous_blob` trait method
- [ ] CIRISEdge ships Sphinx-style onion-routed transport class
  + `AnonymousContributionSubmit` MessageType + handler
- [ ] CIRISNodeCore ships `publish_anonymously()` pyfunction +
  rendezvous query helpers
- [ ] A CC amendment specifies the tier's records and transport,
  against CC 2.4 (closed row-type slot), CC 3.1.3.3 (one holding
  claim), CC 4.4.3.2.1 (holder-inspectability) and the CC 5.4.6 bar
- [ ] Cross-repo integration test: dissident publishes anonymously
  → operator-side forensic inspection cannot recover the publisher
  identity, the publication content (without the recipient's
  rendezvous key), or the trust relationship (because there is
  no recorded trust relationship)
- [ ] Per the §6 plan: a deployment flag such
  that tier-compatible deployments cannot opt out of running the
  tier

## 12. Cross-references

* CIRIS Constitution 1.0-rc9 (tag `v1.0-rc9`): CC 1.13.3.1,
  CC 1.13.3.4, CC 2.4, CC 3.1.3.3, CC 4.4.3.2.1, CC 5.2,
  CC 5.3.3.5, CC 5.4.6, CC 6.1.2
* [SCOPE_PRIVACY.md](SCOPE_PRIVACY.md): the default construction for
  scopes below federation
* [CIRISNodeCore FSD/FEDERATION_SCALING_MODEL.md §9.9](FEDERATION_SCALING_MODEL.md):
  scaling-model derivation that motivated this design
* [CIRISNodeCore MISSION.md](../MISSION.md): the "serve all of
  humanity" goal that includes totalitarian-context users
* [Veilid](https://veilid.com/): closest active reference design
* [Tor v3 onion services rend-spec](https://spec.torproject.org/rend-spec/)
* [Signal sealed sender](https://signal.org/blog/sealed-sender/)
* [Sphinx packet format](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf)
