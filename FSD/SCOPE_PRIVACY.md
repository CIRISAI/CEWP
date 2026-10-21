# FSD: Scope-Native Privacy

**Status:** Design proposal, reconciled to the CIRIS Constitution
1.0-rc9 (tag `v1.0-rc9`). The wire profile of this construction is
absorbed into CC 5.4 (CC 5.4.1–5.4.6: `record_id`, symbol AEAD,
fragmentation, Welcome wrap, witness-chain content, announce
suppression); where this document and CC differ, CC wins. The
construction realizes CC 1.13.3.4: anonymity toward outsiders
defaults to the smallest cohort scope, and federation
(public-commons) scope is the opt-in. It runs on the derived group
plane; the public identity plane makes no anonymity claim (CC 5.4.6
*Position*). The companion [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) is the
planned opt-in tier for GPA resistance at federation scope, named by
CC 1.13.3.1, not yet specified normatively in CC and not yet shipped.

**Companion docs:**
[CEWP.md](CEWP.md) (substrate framing),
[FEDERATION_SCALING_MODEL.md §9](FEDERATION_SCALING_MODEL.md)
(identity-aware-storage trade),
[MEDIA_SHARING.md](MEDIA_SHARING.md) (per-stream MLS exporter
precedent). CC 4.4.3.2.1 (crypto tiers), CC 5.2 (structural
invisibility), CC 5.4 (scope-privacy wire profile), CC 6.1.6
(deterministic ALM). CIRISEdge `docs/LIGHTNET_DARKNET.md` (the
two-plane split as edge ships and measures it).

**Threat model.** State-level adversary with legal coercion
authority, full forensic access to seized hardware in the
cold-state condition, subpoena power over the federation
directory, and ability to compel cooperation under duress.
Out of scope: live-seizure RAM extraction, endpoint compromise
of the publisher's device, TCN-class compulsion of future
architectural cooperation, statistical disclosure over months
of observation below the anonymity trilemma. CC 1.13.3.1 bounds
what may be claimed against this adversary: no communication-graph
privacy, no traffic-analysis resistance, and no unobservability
against a global passive adversary.

---

## 1. Thesis

In CC, "privacy" means exactly two things: content-holding
confidentiality and cohort-scoped visibility. It does not mean
metadata privacy, communication-graph privacy, or unobservability
(CC 1.13.3). Inside that bound, CC 1.13.3.4 sets the default:
**anonymity defaults to the smallest cohort scope consistent with a
publication's intent; federation (public-commons) scope is the
opt-in.** "Anonymity" here means anonymity toward outsiders. Content
born and living at `self` / `family` / `community` is never
advertised to outsiders (CC 1.13.3.4, CC 5.2).

The system runs on two planes (CC 5.4.6 *Position*; CIRISEdge
`docs/LIGHTNET_DARKNET.md` §1):

- **Identity plane (lightnet), public.** Key records, announced
  transport destinations, trust roots. It is attributable and makes
  no anonymity claim: the federation directory announces normally,
  and every inbound byte is attributed to a rooted key it owns before
  any handler sees it (edge doc §1). Even a device joining for the
  first time is "unowned but never anonymous": the per-link seal
  binds its self-signed node key (CC 3.2, adding a device, (e);
  CC 5.3.3.5 E1).
- **Group plane (darknet), derived.** Group destinations are derived
  from the cached directory plus a per-group HKDF and are never
  announced, so a group's existence is invisible to non-members
  (CC 5.4.6). Members relay for members (edge doc §1).

This FSD specifies the group-plane construction as a configuration of
primitives the substrate already ships.

What it does **not** provide, at any scope: unobservability against a
global passive adversary (CC 1.13.3.1) and multi-hop anonymity (edge
doc §5), both not provided today; the planned anonymous tier
([ANONYMOUS_TIER.md](ANONYMOUS_TIER.md)) is the opt-in path, and must
meet the CC 5.4.6 bar (the Tor v3 / I2P b33 blinded-retained-state
family as the nearest admissible relaxation, rotation clocks global).
Nor does it provide ballot secrecy among members (edge doc
§5); and protection against an on-path forwarder that correlates link
traffic across a clique of members to infer that a group exists
(CC 5.4.6, CIRISConstitution#132 ruling).

Why the default sits at the smallest scope: CC 1.13.3.4 calls it a
deliberate correction of the opt-in-anonymity design, under which the
non-savvy vulnerable fail to obtain a protection that a motivated
adversary obtains trivially.

Why GPA-unobservability stays outside the default: it is the one
protection that is genuinely costly (onion-routing latency, mixing,
dedicated cover) and is only reached by an operator already
publishing to the public commons (CC 1.13.3.4). **This boundary is
provisional** (CC 1.13.3.4): as the overhead of GPA-resistance is
measured, the line MAY move toward making more of it default, for
example for individual federation-scope publishers.

The addressing primitive at federation scope is identity-typed
(`author_id`, `attesting_key_id`). Below federation it is
**scope-typed**. `cohort_scope` takes seven values grouped into three
crypto tiers (CC 4.4.3.2.1; §2.1 below). The four that matter for the
reading here:

- `self`: one owner's own devices. For `self`, an outsider is any node
  not owner-bound to the same owner; the boundary is well-defined
  because ownership is single-valued (CC 1.13.3.4; CC 3.2
  single-owner invariant).
- `family`: an intimate roster, never listed (CC 5.4.2).
- `community`: a bounded roster that may be larger, listed or open
  (CC 5.4.2), encrypted under a per-community DEK (CC 4.4.3.2.1).
- `federation`: Commons plaintext; full identity-aware substrate.

The identity-aware property of FEDERATION_SCALING_MODEL.md §9 is a
property of the **federation tier**, not the substrate. At every
smaller tier the full identity-aware substrate runs *inside the
encryption boundary*; outsiders cannot adjudicate what they cannot
see. "Trust-enforceability to insiders and anonymity to outsiders are
the same property viewed from opposite sides of the cohort encryption
boundary" (CC 5.4.6).

---

## 2. Substrate primitives the construction composes

### 2.1 Scope-typed addressing — CC 4.4.3.2.1

`cohort_scope` is wire-format. CC 4.4.3.2.1 groups its seven values
into three crypto tiers, plus the `public` room subkind. The persist
enum names (`InvisibleEncrypted`, `CommunityDek`) are the ones CC 5.4.2
uses:

| Crypto tier | `cohort_scope` values | At-rest | Wire discovery | Reader |
|---|---|---|---|---|
| **self / family** (`InvisibleEncrypted`) | `self`, `family` | encrypted, per-write DEK | none outside the cohort (CC 5.2 structural invisibility) | occurrences / family members |
| **Community** (`CommunityDek`) | `community`, `affiliations` | encrypted under the community DEK | none outside the roster: `custody:ack:v1` within the cohort (CC 3.1.3.3) | community members (DEK cascade) |
| **Commons** | `species`, `biosphere`, `federation` | plaintext | `custody:ack:v1` at federation scope, which is the holder directory | anyone |
| **Public room** (`cohort_subkind: public`) | `community` | plaintext (Commons tier, no DEK) | `custody:ack:v1` at federation scope; a `community:listing:v1` row | anyone who joins |

No holding claim leaves a bounded cohort (CC 5.2). `custody:ack:v1` is
the one holding claim at every tier, placed at the content's own scope
and replicated only to that cohort's members (CC 3.1.3.3, the
CIRISConstitution#141 ruling). The retired `holds_bytes` carrier and
the withdrawn rc6 clause under which a non-member host "inspects the
provenance and chooses to hold opaque ciphertext for a community it
trusts" no longer exist
(CC 3.1.3.3; CC 4.4.3.2.1 holder-inspectability principle: members
hold what they can open, strangers hold nothing). An earlier version
of this table advertised `holds_bytes:*` with cleartext provenance for
the Community tier; that is withdrawn.

The community DEK is mandatory, not opt-in: it is the CC 5.1 epoch-DEK
cascade applied to `cohort_scope: community`, one DEK shared across
emissions, wrapped to each member on admission, re-wrapped on
membership change, `wrap_algorithm: v2` (hybrid PQC) mandatory
(CC 4.4.3.2.1).

Two `cohort_subkind` values opt OUT of the DEK cascade and are
Commons-tier: `infrastructure` (`ciris-canonical` and any
governance/trust root) and `public` (a room anyone may join, with no
confidentiality claim against the public) (CC 4.4.3.2.1).

`affiliations` semantic handling beyond the crypto-tier mapping
remains deferred to a later candidate round (CC 4.5.8.4); the
construction binds on the crypto-tier resolution.

This FSD's construction applies to the **self/family and Community
tiers** (`self`, `family`, `community`, `affiliations` with a subkind
other than `infrastructure` or `public`). Commons-tier content is
plaintext federation-scope material outside this FSD's scope.

The 65% locality dividend
([FEDERATION_SCALING_MODEL.md §1.3](FEDERATION_SCALING_MODEL.md))
re-reads as the structural privacy property of the self/family
tier: typical activity is self/family-scope and structurally
undiscoverable to the federation by wire format alone.
<!-- rc9-sync: unverified, see CC ? (the 65% figure is a CEWP model output, not stated in CC rc9) -->

**Implementation note (persist).** Persist implements this grouping
as `crypto_tier(cohort_scope, cohort_subkind) -> CryptoTier`, the
function CC 5.4.2 names. Edge consumers MUST resolve through
`crypto_tier()` rather than matching on `cohort_scope` strings
directly (CC 5.4.2), so that future `affiliations` semantic changes
flow without wire-format churn.

### 2.2 MLS group state — RFC 9420 ciphersuite 0x004D

Every group has openmls MLS group state under ciphersuite `0x004D`:
the X-Wing KEM (X25519 + ML-KEM-768) and ML-DSA-65 signatures, both
halves of the suite used (CC 5.4.4). Two group-scoped subkeys are
derived per epoch by **bare HKDF-Expand** over the group's raw
`exporter_secret` (CC 5.4.1, CC 5.4.2):

    K_record_id = HKDF-SHA256-Expand(
        PRK  = raw group exporter_secret,
        info = "ciris-edge/scope-privacy/record-id/v1"  (ASCII),
        L    = 32
    )
    K_symbol    = HKDF-SHA256-Expand(
        PRK  = raw group exporter_secret,
        info = "ciris-edge/scope-privacy/symbol/v1"  (ASCII),
        L    = 32
    )

This is **NOT** RFC 9420 `MLS_Exporter()` / `ExpandWithLabel()`:
there is no HKDF-Extract step and no MLS KDF-label framing (CC 5.4.1).
Implementations MUST pass the raw `exporter_secret` to the
`ciris-crypto` `scope_privacy::k_record_id` helper and MUST NOT route
the label through openmls's `export_secret()` (CC 5.4.1). The same
`symbol/v1` label derives `K_symbol` and prefixes the per-symbol
`info` (CC 5.4.2).

The exporter secret is group-and-epoch-bound; distinct ASCII labels
separate `K_record_id` from `K_symbol`. Both rebind on MLS Add/Remove
(CC 5.4.2). Per-record and per-symbol diversification is downstream
(§2.4).

The HKDF here is **SHA-256**, matching the ciphersuite's key-schedule
hash (CC 5.4.1). §2.4's per-symbol derivation uses **HKDF-SHA3-256**
for hash-family separation between layers, not for
harvest-now-decrypt-later: the HNDL exposure of this plane is the
KEM's, which is why the KEM is hybrid (CC 5.4.3, rationale corrected
2026-10-18). An earlier version of this section gave HNDL as the
reason; that is withdrawn.

**Group destinations.** A group-scoped destination is resolved
deterministically from a cached directory entry plus a per-group HKDF;
every member derives the same destination, and outsiders, lacking the
group key, cannot (CC 5.4.6). CC states that this derivation reuses
"the same group-and-epoch-bound key schedule the substrate already
uses to diversify record-ids and symbols", introducing no new key
material (CC 5.4.6). CC rc9 does not pin a label, byte layout or test
vector for the destination derivation, as it does for `K_record_id`
(CC 5.4.1) and `symbol_key` (CC 5.4.2). Edge states that the
derivation is owned by `ciris-verify`, "one derivation, no second
expectation" (edge doc §1). This FSD therefore specifies no
destination formula of its own.

### 2.3 Deterministic ALM topology — CC 6.1.6

`compute_alm_topology(snapshot) → topology` is a pure, deterministic,
integer-only function over `(capacity_ads, trust_grants,
reachability_observations, locality)`, such that byte-equal inputs
yield byte-equal output across implementations (CC 6.1.6). Every
observer of the snapshot derives the same topology. Capacity
advertisements MUST be hybrid-verified before scoring (CC 6.1.6 N8);
reachability observations are ephemeral planner inputs, never
attested, replicated or witness-leafed (CC 6.1.6 D6). There is no
directory authority: the "directory" is a function whose addressing
argument is HMAC-opaque to non-members (CC 5.4.1; §2.4).

### 2.4 Group-keyed fountain-coded distribution — CC 5.4.1–5.4.3

RaptorQ defaults `(N=20, K=6, target_holders=30)` (CC 5.4.3). Each
record (CC 5.4.1):

    record_id_input = CBOR_dCE({
        "v":   1,
        "epc": mls_group_epoch,
        "iid": internal_id,
        "typ": record_type,    // integer per RecordType table below
    })
    record_id       = HMAC-SHA3-256(K_record_id, record_id_input)

`CBOR_dCE` is RFC 8949 §4.2.1 Core Deterministic Encoding
(shortest-form integer encoding, definite lengths only,
no floating-point unless required), with map keys ordered
**by encoded-length first, then lexicographically by encoded
key bytes**. The four-entry map at v=1 therefore serializes
keys in the canonical order `v, epc, iid, typ`. Keys are CBOR text
strings; `v`, `epc` and `typ` are unsigned integers; `iid` is a CBOR
**byte string** (major type 2), never a text string (CC 5.4.1). CC
5.4.1 carries three normative conformance vectors.

`RecordType` integer encoding (CC 5.4.1; CIRISConformance asserts
byte-for-byte reproducibility):

| Type             | int |
|------------------|-----|
| reserved         | 0   |
| SelfRecord       | 1   |
| FamilyRecord     | 2   |
| CommunityRecord  | 3   |
| FederationRecord | 4   |

Additional record types extend this table via CC 5.4.1.

For each RaptorQ symbol (CC 5.4.2):

    symbol_key = HKDF-SHA3-256(salt=record_id, ikm=K_symbol,
                               info="ciris-edge/scope-privacy/symbol/v1"
                                    || u16_be(symbol_index), len=32)
    symbol_envelope = (record_id, u16_be(symbol_index), nonce,
                       XChaCha20-Poly1305(symbol_key, nonce, fragment), tag)

Without `K_symbol` and `record_id`, the holder has indistinguishable-
from-random bytes (Tahoe-LAFS least-authority by reduction to
XChaCha20-Poly1305 IND-CCA + HKDF-SHA3 extract-then-expand; CC 5.4.2).
The same suite seals real and cover fragments, with no cover marker
and no length distinguisher (CC 5.4.2). The fragment set rebinds on
MLS Add/Remove (CC 5.4.3); archive policy is §3.5.

### 2.5 Reticulum (RNS) link layer and the per-link seal

Reticulum carries both planes. On the identity plane, RNS announces
propagate normally (edge doc §1). The RNS link layer itself is
classical and is not relied on for post-quantum confidentiality;
every byte an edge sends over an RNS link MUST be under a hybrid
transit seal (X25519 + ML-KEM-768) bound to the link, with no
classical fallback and no public classical lane (CC 5.3.3.5 E1).

This is not an initiator-anonymity property. An earlier version of
this section stated that "initiator anonymity is structural" because
RNS packets carry no source address field. CC rc9 does not support
that reading: the seal binds the initiator's key into the handshake,
and on the first-contact plane it binds the initiator's self-signed
node key, "never an anonymous one" (CC 5.3.3.5 E1; CC 3.2, adding a
device, (e)); every inbound byte is attributed before handling (edge
doc §1); and Reticulum announce and path request expose who is
reachable where (CC 1.13.3.1). What the group plane hides is the
group, not the link endpoints.

Group-scoped destinations are excluded from announces (§3.3). Where
two members have no direct path, a scoped body MAY ride their
identity-plane link through a non-member transport node, end-to-end
encrypted with the room discriminated inside the link encryption; the
intermediary's only role is reachability, and the link MUST be brought
direct where it can be (CC 5.4.6, CIRISConstitution#132 ruling).
Members relay for members on the group plane (edge doc §1).

### 2.6 Maintenance traffic budget

L1 steady-state maintenance (anti-entropy, RaptorQ lazy repair, ALM
topology snapshots, witness chains) is the budget anchor for cover,
~10–60 KB/s/peer (CC 5.4.3). This traffic is emitted for substrate
liveness regardless of any anonymity goal. §3.1 shapes it into the
cover budget.

---

## 3. The construction

### 3.1 Fixed envelopes and sender-side Poisson cover

> **Placement (steward, 2026-10-21):** the Poisson cover discipline is option (1) of the planned anonymous tier, opt-in per node and negotiated per link, not a default at every scope. See [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) §2–§3 and the measured costs in [benchmarks/anonymous-tier-shaping-2026-10-21.md](../benchmarks/anonymous-tier-shaping-2026-10-21.md). Whether CC 5.4.3's fixed-envelope MUST stays universal is an open question there.

Normative at CC 5.4.3:

- **Fixed envelope size: 1.4 KB**, one MTU. Larger payloads chunk
  per CC 5.4.3; smaller ones are padded to the fixed size.
- **Uniform XChaCha20-Poly1305 AEAD framing** for both real and
  synthetic envelopes, with a synthetic cover envelope marked
  `type=cover` inside the AEAD-protected header.

Proposed by this FSD, not pinned in CC rc9 (CC 5.4.3 refers to it only
as "the companion Poisson-cover emission discipline"):

- **Per-scope Poisson inter-emission intervals.** Each peer
  samples `t_next ~ Exp(λ_scope)` from a peer-local CSPRNG. On
  timer fire, emit the next-queued envelope at that scope; if
  empty, emit a synthetic cover envelope.
- **λ_scope tuned so cover emission dominates real publication on
  a lifetime-average inequality** across the measurement window;
  the budget anchor is §2.6 maintenance throughput.

This is the Loopix Poisson discipline applied at the sender-side
emission boundary. The substrate's existing maintenance traffic
**is** the cover budget. RNS forwarders do not perform per-hop
re-mixing; cover is sender + recipient only. It is not a claim
against traffic analysis: CC treats the record layer's diversified
record ids and cover framing as the wire-privacy answer for the
payload, "not a claim against" an on-path observer's inference that a
group exists (CC 5.4.6), and traffic-analysis resistance remains a
CC 1.13.3.1 non-goal.

### 3.2 Default cohort_scope

`cohort_scope` defaults to the smallest scope consistent with the
publication's intent; federation scope is the opt-in (CC 1.13.3.4).
Concretely: community when a group context is active, family when in
a family context, self when no group context exists. Proposed: the
operator API returns the chosen scope on every publication
("published at scope=X to audience=Y") so silent demotions cannot
occur.

Widening is the one seam where content is checked: a
perceptual-hash tripwire runs at every scope-widening promotion, a
promotion into the Commons or a post into a `public` room; never on
the device and never inside a private cohort (CC 1.13.3.4).

### 3.3 MLS handshake discipline

- **PrivateMessage-only at non-federation scope.** PublicMessage
  framing is banned at community/family/self.
- **Welcome wrap (CC 5.4.4).** MLS Welcome messages MUST be wrapped by
  HPKE `mode_base` (RFC 9180 §5.1.1) to the invitee's static X-Wing
  public key. X-Wing has no `AuthEncap` (per
  draft-connolly-cfrg-xwing-kem; confirmed in draft-ietf-hpke-pq
  §7.2), so HPKE `mode_auth` is structurally unavailable; sender
  authentication is provided out-of-band by an ML-DSA-65 signature
  from the inviter.

  HPKE parameters (cross-impl pinned; private-use suite-id since
  X-Wing has no IANA KEM-id):

        HPKE_SUITE_ID = b"HPKE-xwing-hkdf-sha256-aes256gcm-v1"
        KDF           = HKDF-SHA256
        AEAD          = AES-256-GCM

  The suite-id string is used byte-for-byte in every RFC 9180
  `LabeledExtract` / `LabeledExpand` call; it is a domain-separation
  byte literal, not a wire-vocabulary identifier (CC 5.4.4). The AEAD
  is AES-256-GCM at the HPKE layer, distinct from the §2.4 / §3.1
  XChaCha20-Poly1305 layers.

  The inviter signs the encapsulation bytes with ML-DSA-65:

        encap_signing_bytes
            = x25519_ephemeral_pub      (32 bytes)
            || u32_be(len(mlkem768_ct))
            || mlkem768_ciphertext

  The encoding is length-delimited; no `algorithm` field is
  included. Verification is a precondition to unwrapping the Welcome;
  an implementation that omits it forfeits sender authentication, and
  the forwarder-side membership-opacity guarantee no longer holds
  (CC 5.4.4).
- **Invitee key discovery via cached federation directory
  (CC 5.4.6).** Every participating L1 peer maintains a complete
  local copy of the federation directory's X-Wing public keys,
  refreshed via `federation_keys` anti-entropy. **No per-invitation or
  per-resolution directory query is emitted.** Phone-class peers
  resolve through their L1-relay parent over a relay-blinded path. The
  directory remains federation-public but exposes no
  `querier → invitee` edge under subpoena.
- **No announce for group-scoped destinations (CC 5.4.6).** A
  destination below federation scope MUST NOT emit a Reticulum
  announce, directed or broadcast; the prohibition binds the emission,
  not the addressing mode (CIRISConstitution#91 ruling). Members
  resolve each other's destinations from the cached directory +
  per-group HKDF (§2.2). A peer with a stale cache reports an honest
  miss and MUST NOT fall back to an announce or a per-destination
  query. Per-destination announce control is a small Leviculum
  extension (informative); its absence fails toward suppression
  (CC 5.4.6). Multi-hop reach for a group-scoped destination is an
  amendment-plane question, with the bar: no outsider-observable
  emission, no outsider-retained path state, no epoch-correlated wave
  (CC 5.4.6).

### 3.4 Witness chain content discipline (CC 5.4.5)

This is the privacy witness chain. It is distinct from the CC 6.1.1
WholenessWitness and from the CC 5.3.1 transparency-log witness, and
the three MUST NOT be cross-verified or substituted (CC 5.4.5).

- **Federation witness chain** commits only to federation-scope
  record_ids and a single rate-smoothed counter for non-federation
  activity. The counter ticks on a federation-wide constant schedule
  (1 tick / federation epoch). At each tick the witness commits to
  `count mod target_rate`: real leaves below target are padded by
  cover leaves to the target; real leaves above target back-pressure
  into the next tick. Cover leaves commit to
  `HMAC-SHA3(witness_signing_key, leaf_position || epoch_id)`,
  cryptographically indistinguishable from real Merkle roots under
  the IND of HMAC-SHA3. Deviation from the constant tick is a
  federation-tier slashing condition (CC 5.4.5).
- **Per-community witness chain** commits to the community's
  record_id set with member-only visibility, signed inside the
  community MLS encryption (CC 5.4.5).

### 3.5 Per-community archive policy (CC 3.2 `archive_mode`)

MLS epoch advance bounds the lifetime of `K_record_id` and
`K_symbol`. Each community configures one of (CC 3.2):

- **`rotate-forward`** (default, 30-day window): honest holders
  delete past-epoch keys after the window. Forward-secrecy is
  honest-holder discipline, not cryptographic; a holder under
  coercion or running modified software can retain keys, and this is
  structurally undetectable (§7.8).
- **`retain`**: holders retain past-epoch keys indefinitely for
  archive readability; an adversary compromising a member at
  epoch N+k recovers everything back to their join epoch.

The choice MUST be surfaced at community creation. `archive_mode` is
operator-local config, not federated trust state (CC 3.2; §7.14).

---

## 4. Properties

| Property | Source | Notes |
|---|---|---|
| Operator-cryptographic-opacity for stored ciphertext (cold-state) | §2.4 symbol AEAD (CC 5.4.2); proposed §6 at-rest-encrypted openmls StorageProvider with hardware-bound boot key | A holder lacking `K_symbol` and `record_id` holds bytes indistinguishable from random (CC 5.4.3). Warm-state seizure is §7.8. |
| No directory authority to subpoena for routing | §2.3 deterministic ALM (CC 6.1.6) + §2.4 HMAC'd record_id (CC 5.4.1) + §3.3 cached directory (CC 5.4.6) | The routing "directory" is a pure function; addressing argument is HMAC-opaque; key discovery is cached, not queried per-invitation. |
| Group existence hidden from outsiders | §3.3 no group-scoped announce; §2.2 derived destinations (CC 5.4.6) | Edge labels this Normative + Shipped; relay holds no group address and non-member cannot fetch are Measured (edge doc §3). This is not link-layer anonymity: link endpoints are attributable identity-plane keys (§2.5), and an on-path forwarder correlating traffic across members can infer that a group exists (CC 5.4.6, #132). |
| In-group safety + moderation | §2.2 MLS + full substrate inside the encryption boundary | Trust graph, attestation, eviction and the per-community witness chain run inside the encryption that delivers outsider opacity (CC 5.4.5). |
| Uniform envelopes; proposed sender-side Poisson cover | §2.6 maintenance budget; §3.1 | Fixed 1.4 KB envelopes and uniform AEAD are normative (CC 5.4.3). The Poisson discipline is this FSD's proposal. Neither is a CC claim against traffic analysis or a global passive adversary (CC 1.13.3.1). |
| PQC group secrecy | §2.2 X-Wing (CC 5.4.4); community DEK `wrap_algorithm: v2` (CC 4.4.3.2.1); per-link hybrid seal (CC 5.3.3.5 E1) | PCS posture depends on §3.5; CC 1.13.3.1 states streams have no PCS. |
| Forwarder-side membership opacity | §2.4 + §3.3 PrivateMessage-only + signed-wrapped Welcome (CC 5.4.4) | Forwarders see uniform envelopes and cannot read handshakes. Welcome-emission timing on freshly-instantiated paths remains observable (§7.13), as does link-traffic correlation (CC 5.4.6). |
| Past-epoch forward-secrecy (honest-holder) | §3.5 rotate-forward (CC 3.2) | Honest-holder assumption; coerced or modified holders are §7.8. |
| Locality dividend | §2.1 wire format | Self/family content never emits a holding claim outside the cohort (CC 5.2). |

**Accepted in-scope.** Group members can confirm any record_id
their group has published: the anonymity target is outsiders, not
insiders (CC 5.4.6). In-group adversary tolerance is in-group
governance, not anonymity (§7.1). Cross-fragment timing correlation
across K=6 holders would be dampened by the proposed independent
Poisson timers but not eliminated below the trilemma (§7.10).
Federation-scope content is public by design.

---

## 5. Threat-model mapping

| Adversary capability | Defense |
|---|---|
| Subpoena federation directory | Non-federation records emit no holding claim outside their cohort (CC 5.2); record_id is HMAC-opaque (CC 5.4.1); the federation witness commits to a constant-rate counter (CC 5.4.5). The directory exposes federation-public keys, but no `querier → invitee` edge (CC 5.4.6). |
| Compel cold-state disk + keys | §2.4 symbol AEAD; proposed: openmls StorageProvider encrypts persisted state at rest under a hardware-bound key (SQLCipher with TPM-sealed boot passphrase). Seized cold-state node yields ciphertext only. |
| Coerce reveal whose data the operator holds | Opaque fragments under HMAC'd record_ids, held only by members (CC 4.4.3.2.1 holder-inspectability). The operator can truthfully claim cold-state ignorance (CC 5.4.3). |
| Compel active cooperation (cold-state) | Scope-segregated deployment (§6.3): the federation-tier node identity and the community-tier node identity are different processes. A seized community-tier node lacks federation-tier signing authority; a seized federation-tier node lacks community MLS state. |
| Global passive adversary | **Not defended.** Unobservability against a global passive adversary is a CC 1.13.3.1 non-goal at every scope. The proposed §3.1 cover is not a GPA claim. Not provided today; the planned anonymous tier ([ANONYMOUS_TIER.md](ANONYMOUS_TIER.md)) is the opt-in path. |
| K=6 of 20 holders compromised | Recovery still requires `K_symbol`; that requires either breaking X-Wing or compromising a member who holds the group key. |
| DS-role adversary (RFC 9750 §5) | Substrate AEAD covers handshake content; deterministic ALM lets any holder verify topology. Residual: drop/reorder extends Remove-propagation latency (§7.1). |

**Acknowledged residual.** TCN-class compulsion (§7.7),
live-seizure RAM extraction (§7.8), long-term SDA (§7.10),
targeted in-community Sybil (§7.9), Welcome-emission timing on
freshly-instantiated paths (§7.13), operator-local-config metadata
including `archive_mode` and same-node detection telemetry
(§7.14), and the CC 1.13.3.1 non-goals (relationship-existence,
communication-graph, traffic-analysis, GPA).

---

## 6. Implementation

### 6.1 Cross-repo distribution

| Repo | What ships | LOC |
|---|---|---|
| **CIRISVerify (ciris-crypto)** | HKDF-SHA3 + HMAC-SHA3 + XChaCha20-Poly1305 + HPKE mode_base (RFC 9180 §5.1.1) over X-Wing KEM + ML-DSA-65 sign/verify wrapper for §3.3 Welcome authentication; §2.2 / §2.4 derivation helpers (`scope_privacy::derive_record_id` is the byte-exact source of truth CC 5.4.1 names); the group-destination derivation (edge doc §1). | ~700–1000 |
| **CIRISPersist** | `BlobStorage::put_scope_blob` for symbol-AEAD admission; schema migration on SQLite + Postgres for `federation_scope_blobs` keyed on `(record_id, symbol_index)` with per-symbol nonce. The 7-value `cohort_scope` lattice + `crypto_tier()` grouping is already in tree and matches CC 4.4.3.2.1 (see [CEWP#4](https://github.com/CIRISAI/CEWP/issues/4)). | ~600 + migration |
| **CIRISEdge** | `CohortScope` resolving through persist's `crypto_tier()`; openmls StorageProvider backed by SQLCipher with TPM-sealed boot passphrase; emission layer between every `Edge::send_*` and `Transport::send` running per-scope Poisson + synthetic cover + per-holder CSPRNG jitter; uniform-envelope chunking/reassembly across ReplicationCoordinator, durable outbound, realtime-AV dispatcher, ALM relay fan-out, blob_swarm, MDC sub-streams; §3.2 default + per-publication scope echo; §3.3 Welcome wrap, directory caching, announce suppression, relay-blinded query for phone-class; §3.4 witness chains; §3.5 archive_mode. Already shipped per edge doc §3: derived destinations with announce suppression (`ScopeAddressTable`, arrival-scope admission), MLS cohorts under X-Wing `0x004D` with the CC 5.4.4 Welcome wrap, address retirement on epoch advance. | ~3500–4500 |
| **CIRIS Constitution (CC 5.4)** | Absorbed at CC 1.0-rc9: deterministic CBOR `record_id` with vectors (CC 5.4.1), symbol AEAD (CC 5.4.2), fixed-envelope fragmentation (CC 5.4.3), Welcome wrap (CC 5.4.4), witness-chain content (CC 5.4.5), announce suppression and cached-directory discovery (CC 5.4.6); `archive_mode` vocabulary (CC 3.2). Not pinned in CC: the Poisson emission discipline, the group-destination derivation bytes. | spec |
| **CIRISAgent / CIRISServer** | PyO3 surface audit for default-scope flip + migration helper. | ~200 |
| **CIRISConformance** | Cross-implementation `record_id` reproducibility on the deterministic CBOR profile; forensic disk-inspection test; Poisson emission KS-test (per-scope λ_real ≤ λ_cover, p > 0.01); 20-holder cross-fragment cluster-detection KS-test. | ~600 |

**Total: ~6000–7500 LOC** across the substrate triple plus a
multi-week cross-cdylib release cascade (estimate from the original
proposal; parts have since shipped, see the edge row).

### 6.2 What this does not require

No `AnonymousContribution` parallel record class. No new
MessageType. No Sphinx packet format. No new rendezvous protocol.
No new transport class: wraps existing transport-http /
transport-reticulum / store-and-forward. No new row type: the row-type
slot is closed to the five (CC 2.4).

### 6.3 Operator deployment guidance — scope segregation

For high-sensitivity communities, operators SHOULD run a
scope-segregated deployment: separate process / VM / box for the
community-tier role, addressing the federation tier through a
different operator identity. Same-node multi-tier participation
leaks community membership via timing correlation. This is
operator-facing deployment discipline; the substrate emits a
warning when a community group's holder set includes peers
running the same node identity at federation scope. CC 3.2 gives the
same advice for `archive_mode`: a community requiring
high-sensitivity participation segregates deployment accordingly.

---

## 7. Non-goals

### 7.1 In-group adversary, Remove latency
Encryption boundary IS moderation boundary; a member-adversary
holds the keys. Defense is MLS Remove + epoch rotation; PCS
depends on §3.5. Remove-propagation under §3.1 Poisson and
LXMF store-and-forward can be hours; communities requiring
fast Remove SHOULD trigger an explicit epoch flush. DS-role
adversaries can deliberately delay handshake propagation.

### 7.2 Publisher OPSEC failures
Device compromise, key theft, biometric coercion: out of
substrate scope.

### 7.3 Federation-scope content
Commons plaintext is intentionally public. Authors who want
anonymity toward outsiders stay at community scope or below.

### 7.4 Same-node bleeding without §6.3 scope segregation
Without segregation, timing correlation between federation-tier
and community-tier activity leaks community membership.
Deployment discipline, not cryptographic property.

### 7.5 Cross-fragment timing correlation
K=6 holders receive symbols under per-holder Poisson; complete
elimination requires breaking the trilemma.

### 7.6 LoRa / cellular L0 peers
Bytes-per-minute regime cannot afford the §3.1 cover budget.
Per-publication warning when the holder set includes
degraded-mode peers; high-sensitivity communities restrict
membership.

### 7.7 TCN-class compulsion
A Technical Capability Notice compelling future architectural
cooperation is beyond cryptographic mitigation. The construction
holds against cold-state subpoena, not against compelled
architectural change.

### 7.8 Warm-state seizure / hot-key extraction / dishonest holder
RAM acquisition of a running node recovers the openmls in-memory
state including derived keys. A holder retaining past-epoch keys
against §3.5 policy is structurally undetectable (CC 3.2). Universal
across systems; mitigation is deployment discipline (hot-key
zeroization on suspicious-event detection, full-disk encryption with
at-boot passphrase).

### 7.9 ALM-Sybil
Adversary controlling capacity-ads + trust-grants can engineer
into target holder sets. Decryption threshold (K=6 of 20 + group
key) ~15–20% global Sybil. **Observation threshold** (holding
opaque symbols, correlating across many groups) is lower because
it requires only being a holder. **Targeted in-community Sybil**
engineering trust-grant + locality-ad adjacency to a single
community's posture wins K=6 holders below the global threshold;
defense is community-level holder-admission diversity. Defended
by hybrid-verified capacity advertisements (CC 6.1.6 N8), trust-grant
discipline, and the CC 4.1.1 5-hop default cap on `delegates_to`
traversal. Since only members hold community bytes (CC 4.4.3.2.1),
a Sybil holder must first be admitted to the community.
<!-- rc9-sync: unverified, see CC ? (the ~15–20% global-Sybil figure is not stated in CC rc9) -->

### 7.10 Long-term statistical disclosure
SDA over months of observation recovers persistent patterns even
under continuous Poisson cover. Trilemma lower bound; not
eliminable.

### 7.11 Group lifecycle — split / merge / death
Split: forks retain pre-split exporter_secret and operate
independently from that epoch forward. Merge: no automatic
mechanism. Death: last member departure destroys the only copy of
exporter_secret; records become permanent ciphertext, eventually
GC'd as unowned. Surfaced at group creation.

### 7.12 Compromise recovery
Group-key compromise recovery is **rotate-forward**, not mass
re-publication. Old records are compromised; new records under
new exporter_secret are not.

### 7.13 Welcome-emission timing
The §3.3 HPKE-Base + signature wrap protects content but not the
*event*. A link-adjacent forwarder observing a fresh path
instantiation between two peers observes the invitation
signature. §3.1 cover camouflages volume but not new-path
emergence. Mitigation guidance: batch Welcomes at scheduled
epoch transitions.

### 7.14 Operator-local config metadata
`archive_mode` per community + same-node-detection telemetry
(§6.3 warning state) are operator-local config, observable under
compelled disclosure on the same terms (CC 3.2). Operators with
high-sensitivity participation run §6.3 scope-segregated
deployments per archive_mode class or treat the metadata as
known-leak.

### 7.15 DS-role adversary residual (RFC 9750 §5)
Substrate AEAD covers handshake content; deterministic ALM
prevents misroute. Residual: drop/reorder extends Remove latency
(§7.1) at adversary choice.

### 7.16 What CC 1.13.3.1 rules out
Relationship-existence privacy, communication-graph privacy,
traffic-analysis resistance and GPA-unobservability are not provided
by this construction, and implementers and operators MUST NOT
represent them as provided (CC 1.13.3). For GPA-unobservability and
multi-hop anonymity: not provided today; the planned anonymous tier
([ANONYMOUS_TIER.md](ANONYMOUS_TIER.md)) is the opt-in path.

---

## 8. The harmful-community trade

The construction protects the dissident under a hostile government.
It also protects communities whose content the federation would
refuse to host. That is structurally true, and CC states it plainly:
CEG declines client-side scanning, which would be a surveillance
backdoor, and does not claim to detect harmful content inside private
cohorts, "the same wall every end-to-end-encrypted system hits"
(CC 1.13.3.4).

What CC adds is stated in the same section (CC 1.13.3.4):

- **Fails-secure governance.** Every community has a named moderator
  (the named-moderator existence invariant with merit auto-promotion,
  CC 4.5.4).
- **A perceptual-hash tripwire at every scope-widening promotion.** A
  promotion into the Commons, or a post into a `public` room, creates
  the Contribution that is hash-matched at that seam; never on the
  device, never inside a private cohort. This catches the
  spreading-correlated child-safety cases without scanning content
  that stays within its cohort.

CC's own summary of the line: "fails-secure governance plus
accountable, censorship-resistant moderation; never private-content
detection" (CC 1.13.3.4). The default is set at the smallest scope
because opt-in anonymity would weaken protection only for those least
able to navigate it, while motivated adversaries clear any toggle
(CC 1.13.3.4).

---

## 9. Acceptance criteria

Status at CC 1.0-rc9. A box is checked only where CC or the CIRISEdge
`docs/LIGHTNET_DARKNET.md` states the item as met.

- [ ] CIRISVerify ships HKDF-SHA3 + HMAC-SHA3 + XChaCha20-Poly1305
      + HPKE mode_base over X-Wing + ML-DSA-65 wrap + §2.2 / §2.4
      derivation helpers (with cross-cdylib lockstep cascade).
      *Partly evidenced: CC 5.4.1 / 5.4.2 name `ciris-crypto`
      `scope_privacy::derive_record_id` / `derive_symbol_key` as the
      byte-exact source of truth.*
- [ ] CIRISPersist ships `BlobStorage::put_scope_blob` + schema
      migration. (Lattice contraction is moot: persist's
      `crypto_tier()` already implements CC 4.4.3.2.1's 7-value
      grouping; see [CEWP#4](https://github.com/CIRISAI/CEWP/issues/4).)
- [ ] CIRISEdge ships `CohortScope` resolving through persist's
      `crypto_tier()`; SQLCipher-backed openmls StorageProvider with
      TPM-sealed boot passphrase; Poisson emission layer on all
      enumerated emission paths; §3.2 default + scope echo; §3.3
      Welcome wrap + cached federation directory + announce
      suppression; §3.4 per-community witness + single federation
      counter; §3.5 archive_mode config.
      *Partly evidenced (edge doc §3): derived destinations and
      announce suppression Normative + Shipped; MLS cohorts with the
      CC 5.4.4 Welcome wrap Shipped; non-member cannot fetch, relay
      holds no group address, zero-loss rotation and address
      retirement Measured on bench-mesh. Poisson emission,
      StorageProvider and witness chains are not stated as shipped.*
- [x] The Constitution normatively absorbs the wire profile: CC 5.4.1
      (`record_id` on the RFC 8949 §4.2.1 profile, with vectors),
      CC 5.4.2 (symbol AEAD), CC 5.4.3 (fragmentation), CC 5.4.4
      (Welcome wrap), CC 5.4.5 (witness-chain content), CC 5.4.6
      (announce suppression), CC 3.2 (`archive_mode`), with the
      constitutional alignment at CC 1.13.3.4 and CC 5.4.6
      *Position*. *Not pinned in CC: the Poisson emission discipline
      and the group-destination derivation bytes (§2.2).*
- [ ] CIRISConformance ships cross-artifact integration tests:
      forensic cold-state disk inspection recovers no publisher
      identity / no group identity / no record content;
      cross-implementation `record_id` reproducibility on the
      deterministic CBOR profile; per-scope Poisson discipline
      lifetime-average inequality verified by KS-test at p > 0.01
      over ≥24h; 20-holder cross-fragment cluster-detection KS-test
      at p > 0.01.
- [ ] Bench: per-tier steady-state maintenance budget measured
      under realistic federation conditions; ~10–60 KB/s/peer
      prediction validated; per-scope cover ratio reported.

---

## 10. Prior art and positioning

The construction recombines well-known primitives:
encrypt-then-erasure-code with operator opacity
(Tahoe-LAFS, Storj, Hyphanet); MLS group key state with X-Wing
PQC (RFC 9420, openmls, MIMI-MLS draft); group-shaped wire
addressing (ssb-tribes GroupId, libp2p GossipSub topics, Waku v2
topics, MIMI `mimi://domain/g/id` URIs); encryption boundary =
moderation boundary (Briar, Session closed groups, Cwtch,
SimpleX); directory-free routing (mainline DHT, Iroh, Briar
contact graph); Loopix Poisson cover (Loopix, Nym, Katzenpost);
the Reticulum link layer; AEAD with subcredential KDF (Tor v3 §B.2
HSDir).

The positioning CC records (CC 5.4.6 *Position*, the
CIRISConstitution#91 prior-art record): the design sits in the
zero-emission, membership-concealment corner of the anonymity
trilemma, with the relay floor ("whoever relays for you must know how
to reach you"; Vasserman et al., CCS 2009) satisfied entirely by
members. Two of that corner's known prices are bought back. First,
the two-plane split: the group plane is derived from the public
identity plane, so derivation replaces discovery and no bootstrap
emission exists. Second, determinism replaces coordination: the
member-relay ALM tree is a pure function every member computes
(CC 6.1.6), recovering ⌈log_k N⌉ fan-out without a coordinator that
would have to learn the group exists. An earlier version of this
section made its own novelty claim about the scope lattice and reused
maintenance cover; CC does not state that claim, and it is withdrawn
in favour of CC's.

---

## 11. Relationship to ANONYMOUS_TIER.md

CC 1.13.3.1 names unobservability against a global passive adversary
at federation scope as the residual non-goal, and refers to a
"separate, opt-in mechanism (the CIRISNodeCore Anonymous Tier, Sphinx
onion routing)". CC rc9 specifies no wire, record class or transport
for that tier, and CC 5.4.6 treats multi-hop reach as an
amendment-plane question whose nearest admissible relaxation is the
Tor v3 / I2P b33 blinded-retained-state family, with rotation clocks
global. GPA-unobservability and multi-hop anonymity are not provided
today; the planned anonymous tier ([ANONYMOUS_TIER.md](ANONYMOUS_TIER.md))
is the opt-in path. It is the plan of record for that tier, with
implementation expected soon (steward ruling, 2026-10-21); it is not
yet specified normatively in CC and not yet shipped, and it must meet
the CC 5.4.6 bar.

This FSD is the construction for scopes below federation, and every
deployment runs it: content defaults to the smallest scope, and
widening to federation (public-commons) scope is a per-publication
opt-in (CC 1.13.3.4). Nothing in this FSD depends on the anonymous
tier: deployments that need GPA resistance layer it on top
(CC 1.13.3.3).

The default/opt-in boundary at GPA-unobservability is provisional
(CC 1.13.3.4; §1).

---

## 12. References

- CIRIS Constitution 1.0-rc9 (tag `v1.0-rc9`): CC 1.13.3,
  CC 1.13.3.1, CC 1.13.3.4, CC 3.1.3.3, CC 3.2, CC 4.4.3.2.1,
  CC 5.2, CC 5.3.3.5, CC 5.4.1–5.4.6, CC 6.1.6
- CIRISEdge `docs/LIGHTNET_DARKNET.md` (two-plane claims, mechanisms
  and measurements)
- [FSD/ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) (planned opt-in tier,
  not yet specified in CC or shipped)
- [FSD/FEDERATION_SCALING_MODEL.md §9](FEDERATION_SCALING_MODEL.md)
  (identity-aware-storage trade)
- [FSD/CEWP.md §7.1, §8](CEWP.md) (substrate + scaling/privacy)
- [RFC 9420 MLS](https://datatracker.ietf.org/doc/rfc9420/),
  [RFC 9750 MLS Architecture](https://datatracker.ietf.org/doc/rfc9750/),
  [RFC 9180 HPKE](https://datatracker.ietf.org/doc/rfc9180/),
  [RFC 8949 §4.2.1 CBOR Deterministic Encoding](https://datatracker.ietf.org/doc/rfc8949/)
- [draft-connolly-cfrg-xwing-kem](https://datatracker.ietf.org/doc/draft-connolly-cfrg-xwing-kem/),
  [draft-ietf-hpke-pq](https://datatracker.ietf.org/doc/draft-ietf-hpke-pq/),
  [draft-ietf-mimi-protocol](https://datatracker.ietf.org/doc/draft-ietf-mimi-protocol/)
- [Loopix (Piotrowska et al., USENIX Security 2017)](https://arxiv.org/abs/1703.00536)
- [Anonymity Trilemma (Das–Meiser–Mohammadi–Kate, IEEE S&P 2018)](https://eprint.iacr.org/2017/954)
- [Tor v3 onion services](https://spec.torproject.org/rend-spec/),
  [Briar](https://briarproject.org), [Session](https://getsession.org),
  [Cwtch](https://cwtch.im), [SimpleX](https://simplex.chat),
  [Hyphanet](https://www.hyphanet.org/)
