# FSD: Media Sharing — Multimedia, Cinema, Content Discipline, Takedowns

**Status:** v0.1 design. Reconciled with the CIRIS Constitution
1.0-rc9 (tag `v1.0-rc9`); where this document and CC disagree, CC
wins. CC 3.3.2 locks the `takedown_notice` and `key_grant` field
shapes this FSD first sketched.
**Goal:** Extend `external_content` with multimedia (image / audio /
video / film / 3D) sub_kinds; codify the per-cohort content
discipline that makes CEWP a TikTok/YouTube replacement *where the
user controls the algorithm*; specify takedown notices as CEG-native
wire artifacts; ground the design in international standards (DMCA /
DSA / OSA / AVMSD / TVEC / NCMEC / Christchurch / EU AI Act) without
adding a row type: the row-type slot is closed to the five
primitives (CC 2.4).
**Companion docs:**
* [CEWP.md](CEWP.md) — platform identity + premise/bet
* [FEDERATION_SCALING_MODEL.md](FEDERATION_SCALING_MODEL.md) — substrate scaling
* [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) — the planned opt-in
  anonymous tier for GPA unobservability (named by CC 1.13.3.1; not
  yet specified in CC, not yet shipped)
* [SCHEMA.md §4.29](../SCHEMA.md) — existing external_content sub_kinds
* [CIRIS Constitution 1.0-rc9](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9/constitution)
  — wire-format authority (the former CEG specification is now Parts 2–5
  of CC)

---

## 0. Premise

> "We are replacing TikTok / YouTube with an option where you control
> the algorithm."

The algorithm IS the trust × popularity × freshness math already in
the substrate. The user controls it by setting trust depth, cohort
filter, quality threshold, and freshness decay — every parameter
backed by a CEG-native wire artifact, every recommendation traceable
to (this content + this trust path + this quality signal).

This FSD extends that machinery to **multimedia at any scale**,
codifies the **per-cohort content discipline** that prevents the
substrate from becoming a BitChute-class cesspool while preserving
the **CEG locality dividend** (private speech stays private), and
specifies **takedown notices** as CEG-native wire artifacts so the
substrate is internationally compliant without inventing a separate
moderation tower.

The thesis: **the substrate refuses to amplify, but doesn't surveil.**
A defensible Signal-like line at the federation propagation event,
not at the user's device.

## 1. The content discipline

The discipline is structural at the wire-format level — refusal to
carry is enforced at the scope seams (admission into a cohort, and
the scope-widening seam: a promotion into the Commons or a post into
a `public` room, CC 1.13.3.4, CC 3.2), not by per-instance volunteer
admins. This is the central difference from Mastodon /
PeerTube prior art, which the [Stanford 2023 fediverse CSAM study](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf)
and the [IFTAS 2024 Needs Assessment](https://about.iftas.org/2024/12/17/the-2024-iftas-needs-assessment-report-is-here/)
empirically showed cannot scale with volunteer per-instance
moderation alone.

### 1.1 The per-cohort matrix

| `cohort_scope` | Default content rule | Mechanism |
|---|---|---|
| `self` | Anything. Never advertised or delivered beyond the owner's own devices; the substrate doesn't help distribute it. | CEG locality dividend (§1.3 scaling model). No holding claim leaves the cohort (CC 5.2): the one holding claim, `custody:ack:v1`, is placed at `self` scope and replicated only to the owner's devices (CC 3.1.3.3). Same defensible position as [Signal](https://signal.org/bigbrother/) on E2EE messaging — two parties' private exchange is their concern; the substrate isn't their accomplice because it isn't their distribution channel. |
| `family` | Anything. Same as `self`. | Same — bytes and custody reports go to admitted family members only (CC 5.2, CC 3.1.3.3). |
| `community` (general) | **PG-13 + community-decency**. Refused at intake if `content_rating > PG-13` (any declared scheme) or if no rating declared on multimedia content. | Trust × capacity intake gate, with content_rating consulted. |
| `community` (CW) | **The CW community's declared content classification**. Membership = consent. Warning is wire-format-mandatory. | A community-creation attestation declares `cw_class:{horror,art_cinema,erotic,political,...}` ; joining the community requires explicit `delegates_to:cw_community` attestation acknowledging the warning. |
| `affiliations` | Same as `community` (general). | Federation-publishable but PG-13 default. |
| `species` / `planet` / `federation` | **PG-13 + R/X allowed for declared cinema/art with consumer opt-in; R/X via news editorial framing otherwise.** | `content_class ∈ {film, short_film, documentary, art_piece, theatre, performance}` + declared `content_rating` admits R/X at federation scope, conditional on consumer trust depth + quality threshold + age-gate clearance. |

### 1.2 What this enables (and what it deliberately doesn't)

**Enabled:**

* Tarantino's Pulp Fiction (R, cinema, federation-publishable to
  consenting adults with consumer opt-in) circulates as
  `external_content` `sub_kind: film` `content_rating:mpaa:R`
* A horror short by a small filmmaker carries
  `content_rating:bbfc:18` and circulates via a CW horror community
  where members consented to graphic content
* PornHub-as-trusted-publisher: you emit `delegates_to:pornhub` from
  your federation key; their content reaches your personal feed via
  the trust-graph admission gate; never enters general community/
  global feeds
* OnlyFans creator-as-trusted-publisher: same mechanism, finer
  grain — you trust specific creators individually
* A current-events news segment that includes adult-relevance
  material rides `sub_kind: news_article` with content warnings;
  federation-publishable with consumer opt-in
* Two people sharing private content at `self`/`family` scope:
  the substrate never advertises it outside the cohort (CC 5.2) and
  never scans it (CC 1.13.3.4). Same "not our distribution channel"
  position [Signal](https://signal.org/bigbrother/) takes; same threat
  model the [Apple NeuralHash failure](https://arxiv.org/abs/2111.06628)
  showed cannot be solved by client-side scanning without breaking
  the rest

**Deliberately not enabled at federation scope:**

* Adult content without cinematic/artistic context that doesn't
  ride the trusted-publisher path
* "Free speech" pure-amplification feeds — the [BitChute trajectory](https://dl.acm.org/doi/10.1145/3710950)
  the prior-art research shows happens when "we don't moderate" is
  the differentiator
* Engagement-optimized algorithmic feed promoting whatever
  maximizes time-on-platform — the YouTube/TikTok pathology being
  replaced

### 1.2.1 The trusted-publisher path — adult content, in detail

The cohort-scope matrix in §1.1 carries "trusted-publisher path" as
the route by which content the federation refuses to amplify at
community/global scope nonetheless reaches subscribers who've opted
in. This is the same mechanism Netflix-class streaming uses (§4 of
[the scaling model](FEDERATION_SCALING_MODEL.md), `netflix_replacement`
scenario), and **the same mechanism a CEWP-native adult content
publisher (call it `AdultHUB`, or any deployment of OnlyFans / PornHub
shape) uses**. The substrate doesn't need a separate mechanism for
adult content; the publisher-trust path is the canonical way any
content provider with a subscriber model operates on CEWP.

**The mechanics, end-to-end:**

1. **The publisher operates an L1 (or hyperscale-equivalent) server**
   carrying their content catalog. Their content envelopes carry:
   * `author_id` = the publisher's federation key (verified-business
     identity; via `delegates_to:business_verifier:adulthub_inc` and
     equivalent for OnlyFans, PornHub, Netflix, a community film
     festival, etc.)
   * `content_rating` attestations per the established schemes
     (`content_rating:mpaa:NC-17`, `content_rating:bbfc:R18`, etc.)
   * `content_class:adult` (or `content_class:film` for cinema, etc.)
   * For UGC-class publishers (OnlyFans, PornHub): per-creator
     identity attestations via `delegates_to:creator:{creator_key_id}`
     where each creator went through Ondato/Persona-class
     identity-verification, mirroring the
     [OnlyFans Ondato pattern](https://ondato.com/case-studies/onlyfans-case-study/)
     and [PornHub's December 2020 verified-uploader purge](https://www.eff.org/deeplinks/2020/12/visa-and-mastercard-are-trying-dictate-what-you-can-watch-pornhub)
   * For large videos: `BlobBody::External(ExternalRef)` pointing to
     the publisher's own S3-class store; federation carries metadata
     + ACL only (§2.6)

2. **The subscriber opts in** by emitting `delegates_to:publisher:
   adulthub` from their federation key, where the attestation
   declares:
   * a witness-emitted `age_assurance:provider:adult:v1` (or the
     `government` rung) is on record for the subscriber's key. This is
     not an operator option: exiting child protection to reach
     `content_class:adult` requires at least the `provider` rung, and
     a self-declaration never satisfies it (CC 3.4.11 protective gate,
     CC 3.4.13 Q3; §4 of this FSD; UK OSA / French SREN compliance)
   * Subscriber consents to the publisher's content classification
     (the `cw_class:adult` warning is wire-format-mandatory; §3.4)

3. **The substrate routes content** via the standard trust × capacity
   intake gate. Publisher's content reaches subscribers whose trust
   graphs admit it; never propagates into community / global feeds
   because the cohort-scope-content-discipline (§1.1) refuses
   non-CW community content above PG-13. Subscribers' substrates
   admit it because the *trust path* qualifies, not because the
   *cohort scope* would otherwise admit it. This is the same
   bilateral mechanism Signal uses for sealed-sender messaging —
   content delivery is permissioned by the recipient's intake gate,
   not by federation-wide propagation.

4. **Per-creator eviction is native**: if a specific creator violates
   terms (NCII, copyright, takedown_notice from third party), the
   substrate evicts via `evict_actor(creator_key)`
   ([CIRISPersist#125](https://github.com/CIRISAI/CIRISPersist/issues/125)). The §9 identity-aware-storage thesis is what makes
   this trivial — the publisher's substrate knows whose bytes it's
   holding; can drop one creator's entire content set as a single
   operation; doesn't have to discover-then-purge.

5. **Children never see it.** This does not rest on the publisher
   delegation alone. CC sets a non-derogable floor: anyone not
   age-assured to be 18 or over is treated as a child, and no
   deployment may switch that off (CC 3.4.13 Q2). The protective gate
   blocks every viewer below `adult` assurance from `adult`
   `content_class`, and a `minor` band, "declined to state" and an
   absent assurance all resolve to blocked (CC 3.4.11). Leaving child
   protection needs at least `age_assurance:provider:adult`;
   self-declaration is insufficient (CC 3.4.13 Q3). A minor operates
   under a guardian bound by a live guardianship `delegates_to`, and
   no guardian consent, alone or stacked with the child's, can open
   `content_class:adult` to a minor (CC 3.4.13 Q3, Q5). The publisher
   delegation is refused for such a key in this design, and the CC
   gate holds even where a deployment did not refuse it.

**What this means structurally:** the substrate is not a CSAM-
distribution platform because (a) `self`/`family`-scope content is
never federation-discoverable (§1.1 self/family row — the Signal
position), and (b) federation-discoverable content at community/
global scope is gated by content_rating + content_class + trust path,
none of which CSAM can pass. **Adult content with proper publisher
identity + per-creator verification + age-gated subscribers + content
rating + content class IS distributable on the substrate — without
any of the moderation pathologies BitChute / DTube / LBRY hit by
positioning as "free speech" amplification engines**.

The publisher operates inside the substrate's discipline. The
substrate's discipline is what makes the publisher's compliance
posture legible to consumers (subscribers know what they're getting
because the cryptographic provenance + attestation graph proves it).
This is OnlyFans-class identity-aware-publishing operationalized in
CEG-native terms.

**Operator-config knobs for adult publishers:**

* Required `age_assurance` level (`provider` / `government`; the
  `self` rung never satisfies adult content, CC 3.4.11, CC 3.4.13 Q3)
* Required `delegates_to:business_verifier` chain (compliance
  certifications: 2257 records, KYC creator verification, etc.)
* Geographic restrictions (per-jurisdiction admission rules; e.g.,
  refuse subscribers whose `key_id` claims a jurisdiction where
  the content is illegal)
* Per-content-class CW requirements (members must explicitly opt
  into specific CW classes; granular consent)
* Takedown response SLA (DSA Art. 16 timeliness; substrate-mandatory
  for NCII / CSAM-hash-match / TVEC bases)

### 1.3 The "cinema is art" exception (and why it doesn't generalize to porn)

R/X-rated cinema circulates at federation scope because it carries
two things the substrate can verify:

1. **A declared `content_rating`** from an established scheme (MPAA /
   BBFC / PEGI / IFCO / etc.) — auditable claim about how the
   content has been classified
2. **A declared `content_class`** of art-bearing form (film,
   short_film, documentary, art_piece, theatre, performance) —
   auditable claim about *what kind of content* it is

The substrate doesn't adjudicate "is this art" — the federation's
attestation graph does. A film with a major distributor's
attestation (Disney, A24, Criterion, Studio Ghibli,
international cinema houses, registered film festivals) is
unambiguously cinema. An "art piece" claim from a low-trust source
won't pass quality thresholds set by typical consumers, even if
nominally permitted at the substrate level.

This is why the discipline holds for "porn-disguised-as-art" attacks:
the trust graph + quality attestations adjudicate, not the
substrate. A consumer's algorithm config (trust depth + quality
threshold) refuses content whose art-claim doesn't have substantial
attestation backing. The substrate provides the wire-format surface
the attestation graph operates on; the federation does the work.

## 2. New external_content sub_kinds

Five additions to SCHEMA §4.29:

### 2.1 `image`

Photo, illustration, graphic, screenshot, meme, infographic.

**Source struct** (`crate::ingest::ImageSource`):
```
pub struct ImageSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    pub body_media_type: String,        // image/jpeg, image/png, image/webp, image/avif, image/svg+xml
    pub cohort_scope: String,
    pub content_rating: Vec<ContentRating>,  // multi-scheme; see §3.1
    pub content_class: Option<String>,  // photograph / illustration / screenshot / meme / infographic / generated
    pub width_px: u32,
    pub height_px: u32,
    pub orientation: Option<String>,    // for camera-EXIF respect
    pub alt_text: String,               // accessibility; mandatory
    pub captured_at: Option<DateTime<Utc>>,
    pub creator_key_id: Option<String>, // if different from federation submitter
    pub generated_by: Option<String>,   // for AI-generated: model name per EU AI Act Art. 50 disclosure
    pub topical_relations: Vec<TopicalRelation>,
    pub citations: Vec<Citation>,
}
```

**Dimensions** the federation can attest on this content:
* `image:accuracy:{topic}` — factual accuracy when image makes claims
* `image:authenticity:provenance_chain` — chain-of-custody attestations (cf. C2PA)
* `image:authenticity:ai_generated` — explicit signal per EU AI Act Art. 50
* `image:content_warning:{class}` — gore / nudity / strobe / etc.
* `image:license:{class}` — Creative Commons / proprietary / public_domain

### 2.2 `audio`

Music, podcast episode, lecture, audiobook chapter, sound
sample, generative music piece.

**Source struct** carries: codec (opus / mp3 / flac / aac), duration,
sample rate, bit rate, transcript (mandatory for `cohort_scope ≥
community` per accessibility), `content_class ∈ {music, podcast,
lecture, audiobook, soundscape, generated}`, `creator_key_id`,
`license` info.

**Dimensions**:
* `audio:fidelity:{topic}` — audio quality attestations
* `audio:source_quality:{publisher}` — publisher reputation
* `audio:authenticity:ai_generated`
* `audio:accuracy:{topic}` — for podcasts / lectures
* `audio:content_warning:{class}` — explicit language / mature themes

### 2.3 `video`

General video. Short clips, social videos, talking heads, screen
recordings, vlogs, gameplay, tutorials.

**Source struct** carries: codec (av1 / h264 / h265 / vp9), duration,
resolution (width × height), frame rate, captions (transcript +
SRT/WebVTT mandatory for `cohort_scope ≥ community`),
`content_class`, `creator_key_id`, `license`, **`is_ai_generated:
bool` (per EU AI Act Art. 50 — mandatory disclosure)**, `thumbnail_sha256`
(separate blob).

**Dimensions** include all the image+audio dimension families plus:
* `video:source_quality:{publisher}` — publisher reputation
* `video:accuracy:{topic}` — factual accuracy
* `video:content_warning:{class}` — graphic content / flashing / etc.

### 2.4 `film` — cinematic / art-bearing video

Full-length cinema, short films, documentaries, theatre recordings,
performance art. Distinguished from general `video` by the
`content_class` claim + the trust-graph attestations that adjudicate
the art-bearing nature of the content.

**Source struct** carries everything `video` does plus:
* `content_class: String` (mandatory — film / short_film / documentary
  / art_piece / theatre / performance / animation / experimental)
* `production_credits: ProductionCredits` (director / writer /
  cinematographer / etc.)
* `distributor_key_id: Option<String>` (Disney / A24 / Criterion /
  Studio Ghibli / etc. — the distributor's own key, under which it
  asserts its "this is legitimate film" claim; there is no roster of
  permitted attesters, and a reader admits the claim only if it trusts
  that emitter's root or has consented to it, CC 2.4.1.2.1)
* `imdb_id` / `tmdb_id` / equivalent external canonical IDs
  (optional but reduces trust-graph friction)
* `release_year`
* `runtime_seconds`
* `languages: Vec<String>` (audio + subtitle availability)

**Dimensions** (in addition to `video:*`):
* `film:critical_reception:{aggregator}` — Rotten Tomatoes / Metacritic / Letterboxd-equivalent
* `film:director_reputation:{director_key_id}`
* `film:distributor_reputation:{distributor_key_id}`
* `film:cultural_significance:{measure}` — preservation society
  attestations, archive nominations, etc.

### 2.5 `model_3d` — three-dimensional content

Static models, scenes, volumetric video, scanned environments,
character rigs.

**Source struct** carries: codec (`gltf` / `usdz` / `fbx` / `obj` /
`ply` / `gaussian_splat` / `nerf`), MIME type, vertex_count,
triangle_count (when applicable), bounding_box, animations
(yes/no), texture_resolution, license, content_class (static_object
/ scene / character / volumetric_capture / generated / etc.),
intended_renderer (web_gl / web_gpu / vr / ar / mobile / desktop).

**Dimensions**:
* `model_3d:fidelity:{measure}` — geometric / texture fidelity
* `model_3d:authenticity:provenance_chain` (cf. C2PA-3D)
* `model_3d:authenticity:ai_generated`
* `model_3d:rendering_class:{class}` — what hardware can handle it
* `model_3d:license:{class}`

3D content is treated identically to other multimedia under the
content discipline — declared `content_rating` + cohort_scope gate
both apply.

### 2.6 Inline vs external — NO new chunking primitive

The substrate's `BlobBody` enum (already shipped in CIRISPersist
`federation/blobs.rs`) has two variants — `Inline(Vec<u8>)` and
`External(ExternalRef)`. **The multimedia tier uses both; no new
wire primitive is introduced**, no chunking layer added, no streaming
manifest format invented at the substrate layer.

**Inline (`BlobBody::Inline`, bounded by edge's `MAX_BODY_BYTES`
= 16 MiB):**

| Content class | Typical size | Inline? |
|---|---|---|
| Photo (full-resolution smartphone, JPEG) | 2-5 MiB | ✓ |
| Image (illustration, infographic, screenshot) | < 1 MiB | ✓ |
| Audio clip (≤ 5 min at 96-128 kbps) | < 5 MiB | ✓ |
| Audio podcast episode (≤ 25 min at 96 kbps) | ≤ 15 MiB | ✓ (borderline) |
| Short-form video (TikTok-class, 15-60 sec @ 1080p H.264) | 3-15 MiB | ✓ |
| YouTube Shorts (60 sec @ 1080p H.264) | ~10-30 MiB | sometimes ✗ |
| Animated GIF / short clip (≤ 15 MiB) | 2-15 MiB | ✓ |
| 3D model (small static, ≤ 15 MiB) | varies | mostly ✓ |

The substrate carries inline content end-to-end: bytes ride
federation transport, get admitted at every recipient's intake gate,
contribute to each holder's `bytes_held` accounting, fan out via
demand-driven replication (§2.7).

**External (`BlobBody::External(ExternalRef)`, no size limit):**

| Content class | Typical size | External? |
|---|---|---|
| Audio podcast episode (1-3 hour at high bitrate) | 50-300 MiB | ✓ |
| Long-form video (YouTube general, 10-60 min) | 100 MiB - 2 GiB | ✓ |
| Films (90-180 min cinema) | 2-10 GiB | ✓ |
| Episodes (TV streaming class) | 1-5 GiB | ✓ |
| Volumetric video / scanned environments | 100 MiB - 10 GiB | ✓ |
| Large 3D scenes / character rigs | 50 MiB - 5 GiB | ✓ |

For external content, **the federation carries (SHA, the holding
claim — `custody:ack:v1` at the content's own scope, CC 3.1.3.3 — the
ExternalRef pointing to the publisher's S3-class
store, content access control). The bytes themselves ride that
external store directly**, federation-mediated by trust + ACL but
not federation-stored. This is structurally how Netflix Open Connect
+ CloudFront work today — we just expose the discoverability +
trust-graph + ACL as CEG-native instead of as proprietary AWS APIs.

The S3-class store can be:
- The publisher's own infrastructure (Netflix runs Open Connect; a
  small film co-op runs MinIO on a home server)
- A community-shared object store (e.g., a film festival cooperatively
  operating storage for member submissions)
- An IPFS / Filecoin / Sia gateway exposing content via standard
  protocols (the substrate doesn't care; ExternalRef is just a URI)
- A CEWP-aware distributed object store (follow-up; not v1)

The substrate's "no datacenters required" claim still holds:
publishers' existing storage IS their substrate. CEWP doesn't acquire
a new infrastructure dependency; it composes with what publishers
already run.

### 2.7 Demand-driven replication — substrate's existing pattern, applied to multimedia

Replication for multimedia content follows the same CEG-organic
pattern as all other CEWP content
([FEDERATION_SCALING_MODEL.md §1.1](FEDERATION_SCALING_MODEL.md)).
**Nothing new is required for multimedia.**

**For inline content:**

```
T=0     Publisher emits the Contribution + put_blob_signing.
        Publisher's L1 is the sole holder reporting `custody:ack:v1`
        `here` for X, at X's own cohort_scope (CC 3.1.3.3).
T=ε     A user (a friend in the publisher's trust set, or a
        delegates_to:publisher subscriber) fetches via ContentFetch.
        Receiver's intake gate: trust(publisher) ≥ threshold AND
        capacity_available. On pass: bytes admitted to receiver's
        federation_blobs; receiver signs ITS OWN custody:ack:v1
        `here` for X at X's scope. Receiver is now a holder.
T=...   Every subsequent successful fetch creates a new holder.
        The holder set grows as demand spreads through trust graph
        layers (at federation scope the `here` rows are the holder
        directory, CC 3.1.3.3 rule 2; at a bounded scope they are
        seen only inside the cohort, CC 5.2); popular content fan-outs to many holders;
        unpopular content stays at the publisher.
T=...   Eviction sweeper (popularity × freshness) drops content
        from holders whose users have lost interest. Cold-metro
        copies evict; hot-metro copies are kept. The substrate
        produces CDN-shape topology without a CDN — it's just the
        intake/eviction discipline applied to demand.
```

**For external content:**

```
T=0     Publisher emits the Contribution + put_blob_signing where
        BlobBody::External points to the publisher's S3 / CDN.
        Publisher's L1 reports custody:ack:v1 `here` at the
        content's scope.
T=ε     A user fetches the metadata via ContentFetch (small;
        federation-native). The metadata includes the ExternalRef.
        User's edge fetches the bytes directly from the publisher's
        S3 (NOT through the substrate). No federation byte cost.
T=...   Optional: an L1 server CAN opt in to becoming a CDN edge —
        it full-fetches the external bytes, stores locally (now
        BlobBody::Inline if ≤ 16 MiB OR its own BlobBody::External
        pointing to its own S3), and reports its own custody:ack:v1
        `here`. Now there are two effective holders; other
        peers fetching can use either. This is the same
        demand-driven replication pattern applied to external
        content; it's opt-in by the L1 operator, gated by the
        operator's disk budget + the operator's policy.
```

**Implication: no preemptive replication.** The substrate doesn't
push content to L1s in advance. Replication is the side effect of
demand passing through the trust graph; popular content ends up
where demand is, cold content stays at origin. This is exactly the
CDN topology Netflix / YouTube / Cloudflare maintain via centralized
control, produced naturally from CEG primitives + trust topology +
the popularity × freshness eviction sweeper.

**Holonomic refinement (CC 6.1).** Durability is a per-item choice
the owner makes on publish: an item marked durable at family or
community scope carries a target holder count, and the blob swarm
replicates its sealed chunks to that count across the cohort's live
nodes, rarest chunk first; repair runs when the holonomic view shows
the item's holders falling toward its floor; recovery pulls the chunks
from any holder and decrypts them with the cohort key; an un-marked or
withdrawn item descends (CC 6.1.5). Below the minimum-viable floor
only the signed envelope survives (CC 6.1.5.1). Nodes never encode
stored content; the rateless codec serves the realtime A/V plane only
(CC 5.4.2, CC 5.4.3). Where an item is layered, each `ChunkLayer` is
its own chunk set and can be shed whole, so losing the high-detail
layers yields a coarse version of the same item; degradation is a
property of the layering (CC 6.1). Items age by the noise-floor
descent (CC 6.1.2), where a revoked item must fall below the
individual-recoverability boundary (privacy) while the collective
contribution persists below it (durability). The demand-driven
fan-out above is the cache path; the durable path is the owner's
per-item target. The toy benches neither. The room at row level and
the aggregation producer are live; holding claims, rarity, repair and
descent are built and not yet installed for any item (CC 6.1.5).

**Implication for the toy:** the only new model knob needed is the
fraction of fetch volume that's external_ref vs inline. External
fetch contributes to bandwidth but NOT to bytes_held; inline fetch
contributes to both. Storage gates are determined by inline content
(own + admitted-trust + hot cache); bandwidth gates have an external
multiplier (the publisher's own S3 capacity becomes the substrate's
effective storage tier for that content).

### 2.8 `live_stream` — real-time broadcast

A live broadcast is a streaming sub_kind that emits chunks as it
goes. Wire format: each chunk is a Contribution; the stream
manifest (mpeg-dash / hls / similar) is the parent Contribution
referencing chunks via `topical_relation:has_chunk`.

**Source struct** for the manifest: codec, anticipated_runtime,
chunks_per_minute, latency_class (low/standard/archival),
content_class (news / sports / event / gameplay / vlog /
performance), age_gate_requirement (operator config; §4), takedown
escalation contacts (mandatory for federation scope per EU DSA
Art. 16).

**Realtime A/V is specified in CC (CC 5.3.3).** Earlier drafts
deferred the live-streaming wire format as net-new complexity; CC now
carries it. Realtime media — call / voice / screen-share / chat — is
modeled as composition (CC 5.3.3.2): per-chunk sealing conforms to
**SFrame**, and the epoch-DEK rekey on member change is the **MLS
TreeKEM** construction ([RFC 9420](https://datatracker.ietf.org/doc/html/rfc9420)),
and Edge ships the realtime A/V chunk wire (`SealedAvChunk` /
`ChunkLayer`; CIRISEdge, see its repository for the version). This
builds on the **delivery axis** — the third orthogonal envelope
concern alongside visibility (`cohort_scope`) and revocability
(`subject_key_ids`) — with three optional envelope fields
(`delivery_mode: pull | push`, `listed: public`, `history_on_join:
full | from_join`) plus the streaming-transport surface (CC 5.3.3)
and the `delivery_receipt:{stream_id}` prefix (registered at CC 3.1.4;
emitter rule at CC 3.4.6).
The load-bearing insight: **observer-share (N=1) and streaming
multicast (N>1) are the same primitive at different cardinality** —
subscriber-set = `community` (Policy M, CC 4.4.3.2); E2E directed
delivery = one `key_grant:{axis}:v1` row per recipient carrying the
per-`(stream_id, epoch)` stream-epoch DEK (CC 3.3.2). So a live
broadcast is just the `SealedAvChunk` chunk-DAG
(`topical_relation:has_chunk`) carried over a `delivery_mode: push`
envelope to a `cohort_scope: community` subscriber-set, with a
per-stream log and stream-root for accountability (CC 5.3.3.3) and
`delivery_receipt:{stream_id}` for opt-in delivery acknowledgement.
**Still zero new structural primitives** — the row-type slot stays
closed to the five (CC 2.4); live broadcast and 1:1 media attestation
compose from the same set. This is what makes CEWP a
TikTok/YouTube-*live* replacement, not just a VOD one.

**Status at CC 1.0-rc9.** CC 8.3.3 records the observer-share (N=1)
half as normative-ready with no remaining blockers, and the
streaming-multicast (N>1) half as specified with its implementation
substrate-pending (the streaming substrate step is unscheduled; the
accountable tier is additionally pending). The issue trail this FSD
tracked:
[CIRISPersist#142](https://github.com/CIRISAI/CIRISPersist/issues/142)
(chunk-DAG / per-`stream_id` log storage) and
[CIRISRegistry#34](https://github.com/CIRISAI/CIRISRegistry/issues/34)
(consistency-proof enforcement). The recommended operational cap
`MAX_CHUNKS_PER_EPOCH = 2²⁴` is from CC 5.3.3.1. Lowest v1 deployment
priority.

## 3. Content classification dimension family

Multi-scheme coexistence per the mechanism-descriptive-name gate
(CC 1.13.5).

### 3.1 The `content_rating:*` family

Same content can carry multiple rating attestations from different
schemes. Consumers' algorithms pick which scheme to honor.

| Scheme | Dimension prefix | Ratings |
|---|---|---|
| MPAA (US film) | `content_rating:mpaa:*` | G, PG, PG-13, R, NC-17, X |
| BBFC (UK film) | `content_rating:bbfc:*` | U, PG, 12, 12A, 15, 18, R18 |
| PEGI (EU games) | `content_rating:pegi:*` | 3, 7, 12, 16, 18 |
| ESRB (US games) | `content_rating:esrb:*` | E, E10+, T, M, AO |
| IFCO (Ireland) | `content_rating:ifco:*` | G, PG, 12A, 15A, 16, 18 |
| [Common Sense Media](https://www.commonsensemedia.org/about-us/our-mission/about-our-ratings) | `content_rating:csm:age_appropriate` | 2-18 (age) |
| Operator-defined community CW | `content_rating:cw:{community}:*` | per community policy |

The federation does not pick a canonical scheme. Each rating is a
`scores` attestation on the content; consumer algorithms select
which schemes to honor.

### 3.2 Mapping ratings to cohort_scope gate

The substrate enforces this default rule at federation scope
(`cohort_scope ≥ community` general):

```
admits_to_general_community(content) =
  (content_rating:mpaa <= PG-13 across all attestations) OR
  (content_rating:bbfc <= 12A across all attestations) OR
  (content_rating:pegi <= 12 across all attestations) OR
  (any scheme's equivalent <= PG-13)
```

For `cohort_scope = community (CW)`: the CW community's declared
class determines admission. A `cw_class:art_cinema` community
admits up to NC-17/18; a `cw_class:horror` community admits up to
the horror-tolerance threshold the community declares.

For `cohort_scope ≥ species`: `content_class ∈ {film, short_film,
documentary, art_piece, theatre, performance, animation,
experimental}` AND consumer opt-in lifts the PG-13 ceiling. News
framing remains a path for current-events.

### 3.3 The `content_class:*` family

Mechanism-descriptive declarations of *what kind of content this is*.
Mandatory for federation-scope multimedia. Multi-class allowed (a
documentary-art-piece can carry both).

| Class | Federation scope allowed (R/X)? |
|---|---|
| `film`, `short_film`, `documentary`, `art_piece`, `theatre`, `performance`, `animation`, `experimental` | Yes, with consumer opt-in |
| `educational`, `tutorial`, `lecture`, `talk` | Yes, content_rating gate applies |
| `news`, `current_events`, `journalism` | Yes (editorial-framing exception) |
| `entertainment`, `vlog`, `social_video`, `gameplay`, `commentary` | PG-13 gate strict |
| `adult` | No (publisher-trust-path only) |
| `generated` (AI-generated) | Always carries `is_ai_generated: true`; EU AI Act Art. 50 disclosure |

### 3.4 The `cw_class:*` family (CW community declarations)

A CW community declares its content class at creation; members
explicitly opt in via `delegates_to:cw_community:{community_id}`
attestation.

| CW class | What members opt into |
|---|---|
| `cw_class:art_cinema` | NC-17/18 cinema, art-bearing form |
| `cw_class:horror` | Graphic horror content |
| `cw_class:political` | Politically charged / partisan content |
| `cw_class:erotic` | Erotic content (still distinct from porn-as-utility) |
| `cw_class:violence` | Graphic violence |
| `cw_class:medical` | Medical content (surgical / autopsy / etc.) |
| `cw_class:nsfw_text` | Text-only NSFW (sexual themes in writing) |

Operators can define additional CW classes; established schemes
(MPAA / BBFC / PEGI / etc.) cover the canonical descriptors.

## 4. The age gate — a CC floor, with operator choices above it

The first draft of this section made the age gate operator-managed,
not federation-mandated. CC 1.0-rc9 does not allow that. The
protective gate is normative (CC 3.4.11), and the child-protection
floor is non-derogable: anyone not age-assured to be 18 or over is
treated as a child, and no deployment, operator or declared
jurisdiction may switch that off (CC 3.4.13 Q2). What remains
operator-managed is everything *above* the floor: which verifiers a
deployment accepts, and whether it raises protection further (the
tuning direction is one-way, CC 3.4.13 Q1, Q2).

### 4.1 The wire-format surface

Age assurance is two dimension families (CC 3.4.11):

| Dimension | Who emits it | What it attests | Standard alignment |
|---|---|---|---|
| `age_self_declared:band:{band}:v1` (e.g. `age_self_declared:band:adult:v1`) | The subject, about itself; not reserved | Self-declared band. Lowest assurance and unfalsifiable from inside the fabric. | Not OSA-"highly effective"; never satisfies a sensitive space or `content_class:adult` (CC 3.4.11, CC 3.4.13 Q3) |
| `age_assurance:provider:{band}:v1` (the level MAY be qualified with the verifier's key) | A registered age-assurance verifier; witness-reserved, a subject cannot emit it | Third-party AV provider attests the band. Provider's reputation is in the trust graph. | UK OSA "highly effective" if provider meets [Ofcom criteria](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/); French [SREN](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs) "double anonymity" if provider implements that protocol |
| `age_assurance:government:{band}:v1` (the level MAY be qualified with a credential class) | A witness, as above | Government-issued credential (eIDAS / national ID / passport) | OSA-"highly effective"; strongest assurance |

`{band}` ∈ `minor` | `adult`, with the CC 3.4.13 Q1 developmental
bands (under-13, 13–15, 16–17, adult) as finer qualifiers above the
binary wire predicate. `age_assurance:self:*` is malformed and MUST be
rejected (CC 3.4.11). A consumer unions both families and takes the
highest level on record; absence resolves to the protective default
(CC 3.4.11).

Where a verified rung is required, CC prefers a zero-leak proof (an
mDL `age_over_18` selective disclosure) ranked above full-ID-scan
providers (CC 3.4.13 Q1).

### 4.2 Provider integration pattern

Providers (Yoti / Persona / Onfido / Open Banking-rooted AV /
national identity systems) hold a witness key and emit an
`age_assurance:{level}:{band}:v1` `scores` row about the user's
federation key (CC 3.4.11). The federation can then see "this
provider attested this user's band."

[French SREN's "double anonymity"](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs)
is satisfied because the provider attests against a federation key,
not against the user's real-world identity directly; the platform
sees only the attestation, not the underlying identity proof.

### 4.3 The platform gate, and minors

The gate itself is fixed by CC 3.4.11: `general`-class content is
visible to everyone; `adult`-class content only to a viewer whose
band is `adult`; a `minor` band, "declined to state" and an absent
assurance are all blocked. The gate decides visibility only and never
fires `slashing:*`; a misdeclared age is adjudicated under
`moderation:age_assurance_misdeclaration` (CC 3.4.11).

Entering protection takes only `age_self_declared:band:minor:v1`; leaving it
to reach `content_class:adult` takes at least
`age_assurance:provider:adult` (CC 3.4.13 Q3). A minor below the
solo-autonomy threshold of 16 acts at community scope only with
guardian co-sign; the guardian holds a live guardianship
`delegates_to`, must be at least `provider`-assured adult, and must
have the relationship to that child verified (CC 3.4.13 Q1, Q3). No
guardian, and no stacked consent of guardian and child, may breach
the hard floor: no unilateral federation- or Commons-scope publish
below the teen threshold, not discoverable by unconnected adults,
profile private by default, and no entry to `content_class:adult`
(CC 3.4.13 Q5). The help/report channel stays available to every
minor and no guardian may disable or surveil it (CC 3.4.13 Q4, Q5).

A deployment chooses which providers it accepts and may raise any of
these protections; it may not lower them (CC 3.4.13 Q1, Q2). A
private home deployment (CIRISHome) is still bound by the floor.

The substrate ships the WIRE FORMAT for the attestation and CC fixes
the gate's rule. Each deployment ships the gate's UI and its choice
of accepted verifiers.

## 5. Takedown notices — CEG-native, internationally-aligned

The architectural foundation: a takedown notice is a **signed
Contribution by a claimant against a content SHA** — a `scores` row on
`takedown_notice:{basis}:v1` with `subject_kind: takedown_notice`
(CC 3.3.2). Removal propagates as a **tombstone on the bytes plane**,
not as a `withdraws` of anyone's holding claim: nobody withdraws
another device's custody report, and holders that see the tombstone
re-acknowledge `none` (CC 4.5.3, CC 3.1.3.3 rule 4). The retired
`holds_bytes` carrier this section was first written against no
longer exists (CC 2.4). **No new wire primitives needed.**

This composes cleanly with [Bluesky / AT Protocol's labeler
architecture](https://docs.bsky.app/blog/blueskys-moderation-architecture)
(detection / accusation / propagation / user-side-action as
composable signed artifacts) — the validated prior-art design.

### 5.1 The takedown notice envelope

A `Contribution` `subject_kind: takedown_notice` with payload (the
field shape is now locked at CC 3.3.2, which follows this sketch):

```
{
  "content_sha256": "<hex>",                  // the content being challenged
  "content_holder_key_ids": ["..."],          // optional: specific holders to notify
  "claimant_key_id": "<key_id>",              // submitter's federation identity
  "legal_basis": "...",                       // see §5.2
  "jurisdiction": "...",                      // ISO 3166-1 country / region
  "good_faith_statement": "...",              // text per DMCA §512(c)(3) requirement
  "claim_text": "...",                        // human-readable description
  "evidence_refs": ["..."],                    // optional supporting attestations
  "perceptual_hash": "...",                    // for PhotoDNA/PDQ/hash-match takedowns
  "counter_notice_channel": "...",            // contact for DMCA §512(g) restoration
  "asserted_at": "<datetime>",
  "expires_at": "<datetime?>"                 // optional auto-expiry
}
```

### 5.2 Legal basis enumeration

The legal_basis field is mechanism-descriptive (CC 1.13.5) and maps
to the international standards landscape. The wire strings are the
closed set at CC 3.3.2, byte-compared; a misspelled basis is dropped
by a fail-secure peer, so the spelling matters. CC adds an eleventh,
`steward_purge`, for a trust root's cosigned purge of its own
canonical (CC 2.4.1.1), which is not a media takedown.

| Legal basis | Source | Action discipline |
|---|---|---|
| `dmca512` | [US DMCA §512(c)(3)](https://www.copyright.gov/512/) | Notice → expeditious removal → 10-14 day counter-notice window → restoration if uncontested |
| `dsa_article16` | [EU DSA Art. 16](https://www.cms-digitallaws.com/en/dsa/article-16/) | Notice + receipt confirmation "without undue delay" → timely / diligent / non-arbitrary decision → internal complaint mechanism (Art. 20) for restoration disputes |
| `tvec_terrorist` | [EU TVEC 2021/784](https://www.europarl.europa.eu/news/en/press-room/20210422IPR02621/new-rules-adopted-for-quick-and-smooth-removal-of-terrorist-content-online) | 1-hour removal from competent-authority order |
| `ncmec_csam` | [US NCMEC CyberTipline](https://www.missingkids.org/gethelpnow/cybertipline) | Mandatory reporting (US-based providers); emergency takedown, no counter-notice path |
| `gifct_cip` | [GIFCT Content Incident Protocol](https://gifct.org/2022/06/23/debrief-cip-activation-buffalo/) | Fast-path takedown during active incidents (e.g., Buffalo CIP) |
| `community_standards` | Per-cohort decency / CW community rules | Cohort-trust admission gate; eviction sweeper handles propagation |
| `perceptual_hash_csam` | [PhotoDNA](https://technologycoalition.org/resources/update-on-voluntary-detection-of-csam/) / [PDQ](https://github.com/facebook/ThreatExchange/tree/main/pdq) | Hash match at the scope-widening seam (§5.6) or at an enabled per-group watchlist (CC 4.5.7) |
| `osa_illegal_content` | [UK OSA 2023](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/) | Illegal content removal per Ofcom timelines |
| `avmsd_age_inappropriate` | [EU AVMSD](https://digital-strategy.ec.europa.eu/en/policies/audiovisual-and-media-services) | Composes with the `age_assurance:*` gate; not immediate removal (CC 3.3.2) |
| `court_order` | Jurisdiction-specific court order | Per-jurisdiction handling |

### 5.3 Who may act: the holder's own copy, and the `takedown` duty

The first draft split this into "owner-side" and "third-party"
paths, with clearinghouses pre-trusted by operator configuration. CC
1.0-rc9 draws the line differently, and this section follows it.

**A holder over its own copy.** A device can always stop holding: it
ends its own custody report by re-acknowledging `none` or by
`withdraws`, and only the signer can retract its report (CC 3.1.3.3
rule 4). That needs no one's leave, but it removes one copy, not the
content.

**A takedown over a community's content.** A `takedown_notice` acts
on content only when its `claimant_key_id` holds the `takedown` duty
over the content's community, as itself or delegated from a
steward-bound root (CC 4.5.3, CC 4.5.5). This holds for every basis.
The immediate-removal fast path waives the quorum and the
counter-notice, never the duty-holder gate: an immediate-removal
basis asserted by a key with no `takedown` duty over the content is
refused like any other unauthorised action (CC 4.5.3). A
clearinghouse (NCMEC, GIFCT-CIP, IWF, Project Arachnid, INHOPE
hotlines) therefore acts where a community or root has delegated it
that duty, not because an operator listed it.

**Open labeling stays open.** Anyone may file a `scores` row against
anything they see; it is an observation, and consumers filter on it
as their own policy (CC 4.5.5). DMCA §512 notices, DSA Article 16
reports and community-standards reports from arbitrary keys enter
this way and reach a duty-holder, who acts or does not.

**Detection at the seam.** A perceptual-hash match at the
scope-widening seam (§5.6), or at a per-group watchlist the group's
`moderate`-holder enabled, auto-fires a `takedown_notice` with basis
`perceptual_hash_csam` (CC 4.5.7, CC 4.5.5). The watchlist splits
powers: the node runs the matcher, the operator provisions the
licensed hash database and carries the NCMEC report obligation, and
the `moderate`-holder opts the group in (CC 4.5.7).

**Why this split:** operators still carry legal duties for bytes they
hold (NCMEC reporting; TVEC 1-hour deadline; OSA illegal-content
duty), and they can always drop their own copy at once. What CC adds
is that acting on *the content* — for everyone who holds it — belongs
to whoever holds the `takedown` duty for that community, so no key
gains takedown power over other people's communities by being named
in a configuration file.

### 5.4 The propagation mechanism

1. **Notice admission.** The `takedown_notice` is admitted under the
   duty-holder gate (§5.3; CC 4.5.3 step 1). Immediate-removal bases
   need no quorum.
2. **Tombstone.** The substrate tombstones the content on the bytes
   plane, and the tombstone replicates: it is the only thing that
   tells a peer holding a replica to descend (CC 4.5.3 step 2).
   Holders see it, SHOULD cease serving, and re-acknowledge `none`.
3. **Per-basis discipline** (CC 4.5.3 step 3):
   * Immediate-removal bases (`ncmec_csam`, `perceptual_hash_csam`,
     `tvec_terrorist`, `gifct_cip`, `court_order`): a reachable holder
     MUST destroy the bytes on receipt; the CSAM bases keep only hash
     and minimal metadata for the legal retention window and the
     operator files the NCMEC CyberTipline report; no counter-notice
     path.
   * Counter-notice bases (`dmca512`, `dsa_article16`,
     `osa_illegal_content`, `community_standards`): expeditious
     removal, with the counter-notice routed as
     `reconsideration:new_evidence` (CC 4.5.3 step 5).
     <!-- rc9-sync: unverified, see CC ? — the "conditional removal
     with scheduled restoration if uncontested" mechanism of the
     first draft has no counterpart in CC 4.5.3; how restored bytes
     come back after a reversed tombstone is not stated. -->
4. **Audit.** Every fast-path takedown enters a
   `hard_case:fast_path_takedown` row; a misclassified basis can be
   contested by `reconsideration:procedural_error` (CC 4.5.3 step 4).
5. **Per-actor eviction.** A holder can still drop one actor's whole
   content set from its own store with `evict_actor`
   ([CIRISPersist#125](https://github.com/CIRISAI/CIRISPersist/issues/125));
   that is local storage housekeeping, not a takedown of anyone
   else's copies.

### 5.5 Counter-notice / reconsideration + hold-window schedule

Counter-notices ride `reconsideration:{grounds}`, a `scores`
dimension (CC 3.1.9.2), not a structural primitive. Every notice has
an `attestation_id` a reconsideration can target (CC 3.3.2), and the
subject of the target holds standing to file it (CC 4.5.5). Filing and
ruling on an appeal is the `review` duty (CC 4.5.5); outcomes are
`reversed`, `partial` or `upheld` (CC 3.1.9.2). Holders that again
hold restored bytes report `custody:ack:v1` `here` as for any blob
(CC 3.1.3.3).

Per-LegalBasis hold window (configurable; defaults shipped):

| LegalBasis | Default hold | Counter-notice path | Discipline source |
|---|---|---|---|
| `NcmecCsam` / `PerceptualHashCsam` | Immediate (no hold) | None (mandatory reporting) | NCMEC CyberTipline |
| `TvecTerrorist` / `GifctCip` | Immediate (no hold) | None (1-hour regulatory deadline) | TVEC + Christchurch Call |
| `Dmca512` | 14 calendar days | `reconsideration:new_evidence` (§512(g)) | DMCA §512 safe harbor |
| `DsaArticle16` | 7 calendar days | `reconsideration:new_evidence` (Art. 20) | DSA |
| `OsaIllegalContent` | 14 calendar days | `reconsideration:new_evidence` | UK OSA Ofcom guidance |
| `AvmsdAgeInappropriate` | 7 calendar days | `reconsideration:new_evidence` | EU AVMSD |
| `CommunityStandards` | Per-cohort policy (default immediate) | `reconsideration:new_evidence` | Per-cohort discipline |
| `CourtOrder` | Immediate (default) | Jurisdiction-specific appeal | Per-jurisdiction |

**Business-day mapping note:** DMCA §512(g) specifies "10-14 *business*
days" between counter-notice and restoration. The substrate ships
"14 *calendar* days" as the default DMCA hold to provide safe-harbor
coverage in all jurisdictions without requiring per-jurisdiction
business-calendar tracking (an operator concern, not a substrate
concern). Operators with strict business-day requirements MAY lower
their hold window to 10 calendar days; with strict counter-notice
windows MAY raise to 21 calendar days. The substrate enforces the
operator's configured value uniformly; operators choose strictness
appropriate to their jurisdiction.

This is structurally identical to the DMCA §512(g) restoration
protocol but with federation-cryptographic provenance throughout.

### 5.6 Substrate-level hash-matching

Private content is never scanned. For CSAM, PhotoDNA-class
perceptual hash matching runs only at the **scope-widening seam** —
never on the user's device, never inside a private cohort (CC
1.13.3.4). That seam has two forms:

1. **Promotion into the Commons** — a scope-widening promotion
   (a `supersedes` at the wider scope, CC 8.1.5) creates the
   Contribution that is hash-matched at that seam (CC 1.13.3.4).
   Promotion is the federation-emit moment (CC 5.3.2.4.2), and a
   directory promotion runs as one promotion per file, each with its
   own tripwire, so no collection-level crossing bypasses it (CC
   3.1.3.4).
2. **A post into a `public` room** — posting into a `public`
   community is publication, and the tripwire runs on the post
   exactly as on a promotion into the Commons, because a room anyone
   may join would otherwise be the way around the seam (CC 3.2).

This is the seam where *spreading* happens — content leaving the
cohort it was born in and reaching anyone who asks — and it is the
harm vector that creates new victims. Matching there recovers the
spreading-correlated catches **without** scanning content that stays
within its cohort (CC 1.13.3.4).

In both cases the match fires at the publication seam, NOT on the
user's device. This is the line that
[Apple's NeuralHash 2021 proposal](https://arxiv.org/abs/2111.06628)
failed to draw and that [Signal](https://signal.org/bigbrother/)
refuses on the user's device.

Implementation:
* Operator-config: which licensed hash databases to provision ([NCMEC's
  PhotoDNA](https://technologycoalition.org/resources/update-on-voluntary-detection-of-csam/),
  [Project Arachnid](https://protectchildren.ca/en/programs-and-initiatives/project-arachnid/),
  [Meta's PDQ-open-source](https://github.com/facebook/ThreatExchange/tree/main/pdq);
  CC 4.5.7, CC 4.5.10.1)
* On match at the seam: refuse the widening; emit a `takedown_notice`
  with `legal_basis: perceptual_hash_csam`; the operator files the
  NCMEC report; existing copies follow the tombstone path of §5.4

Content that stays within its `self` or `family` cohort is never
scanned at all; inside a bounded `community`, matching happens only
where the group's `moderate`-holder enabled a per-group watchlist,
which runs at the group's publish/share seam, is audited on every
enable and match, and is never global (CC 4.5.7). This is what CC
1.13.3.4 means by anonymity defaulting to the smallest cohort scope:
content born at `self` / `family` / `community` is never advertised
to outsiders. It is invisibility toward outsiders, not unobservability
against a global passive adversary, which is not provided today;
the planned anonymous tier (FSD/ANONYMOUS_TIER.md) is the opt-in path
(CC 1.13.3.1, CC 1.13.3.4). CC also does not claim to detect harmful
content inside private cohorts; the honest line is fails-secure
governance (every community has a named moderator, CC 4.5.4) plus the
tripwire at the seam, never private-content detection (CC 1.13.3.4).

## 6. Content encryption + key distribution

The substrate is intentionally agnostic about content encryption — it
provides **identity** (federation keys) + **atomic delivery**
(`put_blob_signing` + Contributions). Content-level encryption is an
application-layer pattern that rides on top of the substrate's
identity surface, parameterized by *who's authorized to read*. Two CC
rules bound it: the substrate's own at-rest cascade for `self` /
`family` content is operator policy and MAY be off, but when on it
MUST wrap with `wrap_algorithm` v2, never v1 (CC 5.2); and every
`key_grant` row carries v2 only (CC 3.3.2).

### 6.1 Three encryption postures, by content class

| Content class | Encryption model | Substrate surface used |
|---|---|---|
| **Public** (encyclopedia, news, generally-published, PG-13 community/global) | **None at content level.** Bytes are hash-addressed cleartext. Substrate encrypts at rest via persist's AES-GCM (5.45 GiB/s; protects against disk theft). Access control = discovery + admission, not decryption. | put_blob_signing + `custody:ack:v1` at federation scope (CC 3.1.3.3); persist's at-rest encryption. A `public` room is this tier too: plaintext, no DEK (CC 3.2) |
| **Restricted-group** (CW community, family chat, members-only research) | **Group DEK + per-member-wrapped DEK**. Content body encrypted with random DEK; DEK wrapped under each member's registered encryption keys (X25519 + ML-KEM-768 from its current occurrence, not its signing keys; CC 3.3.2). DEK rotates on membership change. | put_blob_signing for ciphertext + one `key_grant:{axis}:v1` row per recipient (§6.2) for wrapped DEKs |
| **Subscription / publisher-routed** (AdultHUB, OnlyFans, Netflix, paid community) | **Per-content DEK + per-subscriber-wrapped DEK**. Publisher generates DEK per content; wraps for each subscriber's pubkey when they're authorized; revocation = stop wrapping for revoked subscribers. | Same as restricted-group; usually with `BlobBody::External` for the ciphertext + key_grants for the DEKs |
| **Person-to-person** (direct messages, self-scope addressed delivery) | **Hybrid KEM (X25519 + ML-KEM-768) + AES-256-GCM.** Single recipient, no DEK distribution needed beyond the sealed-envelope wrap; the link beneath carries the hybrid transit seal (CC 5.3.3.5 E1). | Edge's `inline_text_pipeline` (Classify + Scrub + AES-GCM) extended for content carriage |

### 6.2 The `key_grant` row (1+4 stays locked)

The payload this section first sketched is now locked at CC 3.3.2.
**No new structural attestation primitive**: a key wrap is one
`scores` row **per recipient** on `key_grant:{axis}:v1`, `{axis}` ∈
`content` | `epoch` | `stream` (closed), `attesting_key_id` the
sealer and `attested_key_id` the recipient occurrence (CC 3.3.2). The
rc6 "key-grant set" (N wraps in one row) is retired (CC 2.4).

**Payload fields** (CC 3.3.2; see there for the normative shape):

```
{
  "subject_kind": "key_grant",
  "recipient_key_id": "<the federation_keys row receiving the DEK>",
  "content_sha256": "<sha256 of the content this DEK decrypts>",
  "wrapped_dek": "<base64url — the DEK under the recipient's ENCRYPTION keys>",
  "wrap_algorithm": "x25519_mlkem768_aes256_gcm_hkdf_sha256",
  "ratchet_version": 1,
  "key_validity_window": {
    "start": "<RFC 3339 datetime>",
    "end":   "<RFC 3339 datetime or null>"
  },
  "scope": "single_content" | "group_member" | "subscription_tier",
  "rotation_chain": ["<prior key_grant id in the grant-supersession lineage>"],
  "asserted_at": "<RFC 3339 datetime>"
}
```

The row is signed by the grantor (publisher / group admin / sender)
with their hybrid federation key. The recipient's application unwraps
the DEK locally and decrypts the referenced content. A recipient with
no registered ML-KEM key is excluded, fail-secure, never sent a
classical-only wrap (CC 3.3.2, CC 5.2).

### 6.3 The wrap algorithm

**Superseded at CC 1.0-rc9.** The first draft used an X25519-only
wrap (`x25519-aes256-gcm-hkdf-sha256`) as v1 and left hybrid ML-KEM
for later, arguing that the ciphertext-stealing horizon was further
out than the signature-forging one. CC does not accept that: the DEK
protects content that may persist indefinitely, so a classical-only
KEM is a harvest-now-decrypt-later exposure (CC 5.1), and a
`key_grant` row carries `wrap_algorithm` v2 only (CC 3.3.2).

The v2 wire string is `x25519_mlkem768_aes256_gcm_hkdf_sha256`:
hybrid X25519 + ML-KEM-768 (FIPS 203) KEM, HKDF-SHA-256, AES-256-GCM
(CC 3.3.2). The wire string is byte-compared and an unknown string is
rejected, so a separator or case variant (including this draft's
hyphenated form) does not decode (CC 3.3.2). A future v3 (ML-KEM-1024
anticipated) is an additive row (CC 3.3.2).

The construction is the HPKE-style pattern (ephemeral key agreement,
KDF, AEAD over the DEK) with the KEM made hybrid. The signature on
the row stays hybrid (Ed25519 + ML-DSA-65).

### 6.4 Group key rotation for CW communities

CW community uses a **shared symmetric group DEK** rotated on
membership change. The rotation pattern:

```
T=0   Community admin generates DEK_0; emits key_grants to all
      initial members under DEK_0. Past content (none) uses DEK_0.
T=k   New member M joins. Admin emits a key_grant to M for the
      CURRENT DEK_0 (admitting them to existing content) and
      simultaneously generates DEK_1, emits key_grants to ALL
      members (including M) under DEK_1. All NEW content from T=k+ε
      onward uses DEK_1.
T=l   Member X leaves / is removed. Admin generates DEK_2, emits
      key_grants to remaining members under DEK_2. New content uses
      DEK_2. X retains DEK_0 and DEK_1 for content they were
      authorized for; cannot decrypt DEK_2 content.
```

**Forward-secrecy posture:** X (the leaver) can still decrypt
content they had legitimate access to under DEK_0 / DEK_1. This is
the same posture as Signal group messaging (departed members keep
past message keys). For **full forward secrecy** (X cannot decrypt
ANY past content after leaving), [MLS (RFC 9420)](https://datatracker.ietf.org/doc/html/rfc9420)
tree-based ratcheting is the application-layer composition —
implementable on top of the `key_grant` primitive but not required
at the substrate level for stored community content. Operators can
choose: simple rotation (this FSD's v1) or MLS (a community-attestation
extension layer). For streams the choice is already made: the
epoch-DEK rekey on member change is MLS TreeKEM, member removal forces
a rotation, and each wrap is v2 (CC 5.1).

### 6.5 Per-subscriber wrap for publisher routes (AdultHUB / OnlyFans / Netflix)

Subscription-routed content uses **per-content DEK with per-
subscriber wrap**:

```
For each content C:
  Publisher generates DEK_C; encrypts content with DEK_C; stores
  ciphertext at BlobBody::External (publisher's S3) OR BlobBody::Inline
  if ≤ 16 MiB.
  Publisher emits the content Contribution with content_sha256 = SHA(ciphertext).

For each active subscriber S (at content publish time):
  Publisher emits a key_grant:content:v1 row to S under DEK_C with
  scope = "subscription_tier", scope_id = "<their subscription tier>".

For each NEW subscriber S' (joining after content publish):
  Publisher's substrate processes their delegates_to:publisher attestation;
  enumerates content they're now authorized for (per their tier);
  emits key_grants for accessible back-catalog content.

For revoked subscriber R:
  Publisher's substrate stops emitting NEW key_grants for R.
  R retains key_grants already emitted (can decrypt content authorized
  during their active subscription window — same as Netflix
  downloaded-while-active titles remaining playable; deliberate UX
  posture for revocation).
```

**At AdultHUB scale (100M subscribers, 1K new videos/day, ~1 KB per
key_grant):** ~100B key_grants/day federation-aggregate = ~100 GB/day
federation-aggregate. Spread across the publisher's L1 server fleet,
this is bounded by the scaling-model toy's existing federation
bandwidth budget. (`adulthub_replacement` scenario in
[FEDERATION_SCALING_MODEL.md §5](FEDERATION_SCALING_MODEL.md) shows
the publisher-side load fits per-server.)

### 6.6 Revocation, per-actor eviction, and the substrate's identity-aware-storage thesis

The §9 identity-aware-storage property of CEWP
([scaling model §9](FEDERATION_SCALING_MODEL.md))
makes per-actor revocation of key_grants tractable — the substrate
knows which key_grants were issued to which grantee key. When a
publisher mass-retires grants tied to a compromised recipient, CC
has it emit a fresh `key_grant` whose `rotation_chain` supersedes the
prior grant, NOT a `withdraws` against it (CC 3.3.2). Persist's
`evict_actor(key_id)`
([CIRISPersist#125](https://github.com/CIRISAI/CIRISPersist/issues/125))
handles the local storage side of the same operation.

This is the cryptographic enforcement complement to the trust-graph
admission gate. Together:
* Trust gate refuses new admission of content from / to revoked
  parties
* superseding grants (CC 3.3.2) retire key_grants to revoked
  parties, and `evict_actor` drops their local rows
* a bytes-plane tombstone propagates a content-side takedown (CC
  4.5.3); a holder ends its own custody report itself (CC 3.1.3.3)
* `reconsideration:{grounds}` reverses if the revocation was wrong
  (CC 3.1.9.2)

The substrate provides the complete revocation surface — at the
discoverability layer, the admission layer, the eviction layer, and
the reconsideration layer — using primitives that already exist.

## 7. The user-controlled algorithm — operationalized

The user's "for you" surface is computed locally from these
already-shipped + this-FSD-extended primitives:

| Knob | Wire-format primitive | Already shipped? |
|---|---|---|
| Trust depth (0/1/2/3) | `delegates_to` graph traversal via NodeCore trust-depth oracle | ✅ NodeCore#21 (this session) |
| Cohort filter (local/community/global) | `cohort_scope` field at admission + UI feed selector | ✅ NodeCore Phase 2B compose surfaces |
| Quality threshold | `compose_article_quality` aggregation thresholding | ✅ NodeCore #19 Phase 3 sub-1 |
| Freshness decay | persist EvictionSweeper decay curve config | ✅ CIRISPersist |
| Content_rating filter | `content_rating:*` dimensions admitted at consumer threshold | This FSD |
| Content_class filter | `content_class:*` dimension at consumer policy | This FSD |
| CW community subscriptions | `delegates_to:cw_community:*` attestations | This FSD |
| Trusted publisher subscriptions | `delegates_to:publisher:*` attestations | This FSD |
| Age gate | `age_assurance:*` / `age_self_declared:*` rows + the CC 3.4.11 protective gate (operators choose accepted verifiers) | CC 3.4.11, CC 3.4.13 |

The user's algorithm is the tuple of these settings. Default
presets (PG-13 family / standard adult / news-only / academic-only)
ship with the unified-client (CIRISAgent). Custom configs are
exportable as signed attestations the user can carry across
deployments.

This is the "composable moderation" pattern Bluesky / Ozone
pioneered ([Bluesky's Moderation Architecture](https://docs.bsky.app/blog/blueskys-moderation-architecture)),
generalized to the full algorithm rather than just labeling.

## 8. AI-generated content disclosure

[EU AI Act Article 50](https://artificialintelligenceact.eu/article/50/)
requires AI-generated media to be marked as such. This is a
mandatory dimension on AI-generated multimedia:

* `image:authenticity:ai_generated`
* `audio:authenticity:ai_generated`
* `video:authenticity:ai_generated`
* `model_3d:authenticity:ai_generated`

The substrate refuses to carry undisclosed AI-generated content at
`cohort_scope ≥ community` for any sub_kind where `generated_by`
or equivalent metadata indicates AI generation but no
`authenticity:ai_generated` attestation is present. This is
substrate-level enforcement of the EU AI Act disclosure
requirement.

### 8.1 The C2PA interop profile

CC 8.4.1 ("speak CEG inside, standards at the edge") and CC 8.4.2
give the boundary profile for media provenance:
**CEG cites [C2PA](https://c2pa.org/) manifests via `evidence_refs[]`
— it does not re-encode them into the JCS interior.** The C2PA
manifest stays in its own COSE-signed container; the CEG
`authenticity:provenance_chain` attestation references it by digest
through `evidence_refs[]`, layering CEG's federation judgment *atop*
C2PA's capture/edit provenance. No second interior canonicalization;
the 1+4 surface is untouched. This is the EU AI Act Art. 50 export
path — a C2PA-bearing manifest satisfies the disclosure obligation,
and the CEG attestation carries the federation's scoring of it.

**Provenance says where the bytes came from; CEG says what a
community of signers makes of them.** C2PA answers "this image was
captured on device D and edited in tool T"; the CEG attestation
graph answers "and the federation scores this source / this art-claim
/ this accuracy at scope S." The two compose without either
swallowing the other — C2PA at the byte boundary, CEG over the
federation. This resolves the §13 open question on C2PA integration:
reference-via-`evidence_refs[]`, do not define a competing
CEG-native provenance encoding.

## 9. International standards mapping (full coverage)

| Regulation | What it requires | CEG-native mechanism |
|---|---|---|
| [US DMCA §512(c)(3)](https://www.copyright.gov/512/) | Notice/counter-notice, designated agent, expeditious removal | `takedown_notice` Contribution + `reconsideration:new_evidence` |
| [EU DSA Art. 16](https://www.cms-digitallaws.com/en/dsa/article-16/) | Notice mechanism, receipt confirmation, timely decision | `takedown_notice` + receipt-attestation timestamp + decision audit log |
| [EU DSA Art. 20](https://eur-lex.europa.eu/eli/reg/2022/2065/oj/eng) | Internal complaint mechanism | `reconsideration:{grounds}` pathway (CC 4.5.5) |
| [EU TVEC 2021/784](https://www.europarl.europa.eu/news/en/press-room/20210422IPR02621/new-rules-adopted-for-quick-and-smooth-removal-of-terrorist-content-online) | 1-hour terrorist content removal | `takedown_notice` with `legal_basis: tvec_terrorist`; substrate-mandatory immediate eviction |
| [UK OSA 2023](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/) | Highly effective age assurance for harmful content | `age_assurance:provider:adult:v1` from an Ofcom-compliant provider (CC 3.4.11) |
| [France SREN](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs) | Double-anonymity age verification | `age_assurance:provider:*` pattern; provider attests against federation key, not real identity |
| [EU AVMSD](https://digital-strategy.ec.europa.eu/en/policies/audiovisual-and-media-services) | Age verification proportionate to risk | Cohort-scope + content_rating + age_assurance composition |
| [US COPPA](https://www.ftc.gov/legal-library/browse/rules/childrens-online-privacy-protection-rule-coppa) | Children's privacy under 13 | Substrate has no behavioral tracking; CEG locality dividend; the CC 3.4.13 floor (treat-as-child-unless-assured, guardian co-sign under 16, no profiling of minors) |
| [EU GDPR Art. 8](https://gdpr-info.eu/art-8-gdpr/) | Child consent age (13-16 per member state) | Same as COPPA — substrate doesn't process personal data for advertising |
| [EU AI Act Art. 50](https://artificialintelligenceact.eu/article/50/) | AI-generated content disclosure | `authenticity:ai_generated` mandatory dimension |
| [US KOSA (pending)](https://www.congress.gov/bill/118th-congress/senate-bill/1409) | Duty of care for minors | Substrate refuses engagement-optimization; user-controlled algorithm; PG-13 default at federation scope; CC 3.4.13 under-18 baseline (no engagement-maximizing nudges, no profiling-based targeting) |
| [Christchurch Call](https://www.christchurchcall.org/) / [GIFCT](https://gifct.org/) | Terrorist content removal coordination | `legal_basis: gifct_cip` substrate-mandatory takedown |
| [NCMEC CyberTipline](https://www.missingkids.org/gethelpnow/cybertipline) | Mandatory CSAM reporting (US providers) | `legal_basis: ncmec_csam` substrate-mandatory takedown + report emission |

The substrate doesn't replace any regulation. It provides the
wire-format substrate the regulatory regime can plug into.

## 10. What this requires across the stack

### 10.1 NodeCore (this repo)

* Five new `external_content` sub_kind ingest paths
  (`ingest_image`, `ingest_audio`, `ingest_video`, `ingest_film`,
  `ingest_model_3d`) — same `finalize_external_content_ingest`
  shared tail as existing six sub_kinds
* New source structs + `build_*_payload` builders for each
* Extension to `compose_article_quality` to handle multimedia
  dimension families
* New `takedown_notice` subject_kind + ingest path
* CW community membership tracking via `delegates_to:cw_community:*`
* Age gate consumer policy in compose layer (filter content per
  user's age_assurance + content_rating thresholds)
* SCHEMA.md §4.29 extension (five new sub_kinds)
* SCHEMA.md new section for `takedown_notice` + `age_assurance:*`
  attestation classes

### 10.2 Persist

* Perceptual-hash check at the scope-widening seam (promotion into
  the Commons, post into a `public` room; §5.6) and at enabled
  per-group watchlists (operator provisions which hash databases;
  PhotoDNA / PDQ / Project Arachnid; CC 4.5.7)
* `takedown_notice` substrate enforcement on receive, under the
  duty-holder gate (CC 4.5.3)
* the bytes-plane tombstone at the takedown_notice handler (CC 4.5.3)

### 10.3 Edge

* MessageType for takedown_notice dispatch + counter-notice
* No new transport work — rides existing Contribution MessageType
  family

### 10.4 LensCore

* AI-generation detection model (helps enforce `authenticity:
  ai_generated` even when content lacks self-disclosure)
* Detection events for content-class-misclassification (porn
  claiming to be "art")

### 10.5 Registry

* Done in CC: `takedown_notice` and `key_grant` (CC 3.3.2), the
  multimedia dimension families (CC 3.3.12), age assurance (CC
  3.4.11) and minor protection (CC 3.4.13)
* 1+4 wire-format lockdown survives — this is all dimension
  additions + new subject_kinds, NOT new structural primitives

### 10.6 Agent runtime

* Default user-algorithm presets (PG-13 family / standard adult /
  news-only / academic-only / horror-CW / art-cinema-CW)
* Age-gate UI implementation (operator-config which providers; the
  gate's rule is CC 3.4.11 and the minor floor CC 3.4.13)
* CW community discovery + opt-in UX
* Trusted publisher discovery + opt-in UX (PornHub / OnlyFans / etc.
  appear as discoverable publishers; user opts in via
  `delegates_to:publisher:*` attestation)

## 11. Moderation as first-class API surface

Moderation isn't a feature bolted onto the media tier — it's the
shape of the fabric. This section makes the architecture explicit so
the NodeCore + LensCore + Agent runtime API surfaces all expose it
the same way.

The architecture has **four structural layers**. They all already
exist across CEG + MISSION + the federation FSDs; §11 here unifies
how they appear in the media-tier API.

### 11.1 The four-layer moderation architecture

#### Layer 1 — You are your own moderator (default, structural)

This is the **CEG locality dividend** + the **per-consumer admission
gate** doing the work.

* 65% of typical activity never leaves your device (`cohort_scope`
  wire field, CC 2.1)
* Every consumer applies their own admission gate: **trust × capacity
  at intake, popularity × freshness at eviction** (per
  [FEDERATION_SCALING_MODEL §3](FEDERATION_SCALING_MODEL.md))
* Per-feed thresholds: you set what gets surfaced; nothing reaches
  your eyeballs without passing your gate
* The substrate doesn't prejudge "art" or "harm" — your gate decides

**No one else needs to moderate for you.** The substrate's default
is "your judgment, on your hardware, against your trust set." This
is what makes CEWP a *moderator-of-record-not-required* substrate.

#### Layer 2 — People you trust are moderators (structural, by graph)

**The trust graph IS the moderation graph.** Whoever you
`delegates_to` is a moderator from your perspective at the depth
you've configured.

* `delegates_to:{role}:{key_id}` (CC 2.4.1.2) — `moderator`, `wa`,
  `expert_in:{cell}`, `publisher`, or any role
* Trust recursion depth (operator-side config — see §4 above and
  [TRUST_HIERARCHY.md](TRUST_HIERARCHY.md)): depth 0 = self only;
  depth 1 = your direct delegates; depth 2 = their delegates; etc.
* A `moderation_event` filed by someone at depth 1 in your graph
  weighs more than one at depth 5 — the same intake-gate trust
  computation that controls content admission controls moderation-
  signal admission
* A WA you trust in a domain is automatically a moderator for that
  domain — same primitive
* Your personal moderators are emergent: no platform-level
  appointment, no instance-owner bottleneck. A `community` is
  different: it federates only while it has a named moderator
  holding the `moderate` duty, named by its founder at creation and
  replaced by merit auto-promotion if that moderator lapses; with no
  eligible member it fails secure (CC 4.5.4)

**This works whether or not you run an agent.** If you trust your
kid's school's tech-coordinator at depth 1, their reports filter
your kid's feed. No agent involved.

#### Layer 2.5 — Optionally, delegate to your agent (one flavor of layer 2)

This is the user's option, not the architecture's default. Same
primitive — `delegates_to:moderator:{agent_key_id}` — only the key
belongs to the user's own agent. The agent then:

* Receives reports + moderation queues routed by the trust graph
* Acts per the user's stated values (encoded in their `Goal:*`
  projector, [P12](../MISSION.md#primitive-12-goal))
* Signs `moderation_event` Contributions on the user's behalf
* Every decision lands in the user's H3ERE trace — auditable,
  reviewable, reversible via P11 Reconsideration

If the user doesn't delegate, their agent doesn't moderate.
**The substrate has zero requirement that anyone delegate to an
agent.** Layer 2.5 is a UX convenience for users who want it; the
substrate is moderation-shaped without it.

#### Layer 3 — WA quorum adjudication (formal, CEG-codified)

When a `moderation_event` warrants formal action against another
participant beyond what the trust graph already accomplishes:

* P8 ModerationEvent — in CC, a `moderation:{allegation_type}` row
  (file with witness diversity per P10)
* P9 SlashingAttestation — `slashing:{outcome}` (`proven_rogue` /
  `not_proven`), emitted under the `slash` duty
* P11 Reconsideration — `reconsideration:{grounds}` (universal
  appeal — `new_evidence` / `procedural_error` /
  `quorum_compromise`), under the `review` duty
* (all three are `scores` dimensions, CC 3.1.9.2; the duties are CC
  4.5.5)
* Locality-scaled-quorum per [FSD-002 §6.1.5](https://github.com/CIRISAI/CIRISRegistry/blob/main/FSD-002.md)
  — N scales with `cohort_scope`
* RATCHET provides anti-Sybil advisory flags
  ([MISSION §2.16](../MISSION.md#216-ratchet-integration-contract))

Layer 3 fires when persistent failure at layer 2 has happened —
a participant has accrued enough across-trust-graphs evidence of
rogue action that the federation, not just individuals, takes
notice.

#### Layer 4 — Substrate-protective fast-path

For TVEC / NCMEC / GIFCT / PerceptualHashCsam / CourtOrder —
immediate removal, no counter-notice, still under the duty-holder
gate. DMCA / DSA Art 16 / OSA / community-standards go through
counter-notice via P11. See §5.3 above + CC 4.5.3 for the codified
protocol.

### 11.2 Reporting IS first-class

**Reporting a piece of content is filing a P8 ModerationEvent.**
There is no parallel "report API" — there is only the universal
Contribution envelope, applied to moderation as it's applied to
everything else.

```rust
// crate::ingest — first-class on the public surface

pub async fn report_content<S: BlobStorage, C: ContributionStore>(
    ctx: &IngestContext<'_, S, C>,
    target_contribution_id: ContributionId,
    allegation_type: AllegationType,       // P8 variants
    rationale: String,
    evidence_refs: Vec<EvidenceRef>,
    witness_set: Option<Vec<WitnessId>>,    // P10; opt-in at filing time
) -> Result<ContributionId, IngestError> {
    // Builds a Contribution(MODERATION_EVENT) per MISSION §2.8 P8
    //   (a moderation:{allegation_type} row, CC 3.1.9.2)
    // Witness-diversity gate applies (P10) — high-stakes if
    //   target_contribution holds Tier-2 dimensional claims
    // (The first draft's stake_credits parameter is removed: CC
    //   carries no conserved balance as substrate state, CC 3.3.10.)
    // Returns the filed Contribution ID; WA-quorum adjudication
    //   proceeds via standard P8/P9 flow
}
```

```rust
pub async fn report_actor<S: BlobStorage, C: ContributionStore>(
    ctx: &IngestContext<'_, S, C>,
    target_actor_key: FederationKey,
    allegation_type: AllegationType,
    rationale: String,
    evidence_refs: Vec<EvidenceRef>,
    witness_set: Option<Vec<WitnessId>>,
) -> Result<ContributionId, IngestError>;
```

Both surfaces are pyfunction-exported. The Agent runtime UI hooks
them from any "report" affordance — `report_content` on a media
item; `report_actor` on a publisher / community / participant.

### 11.3 Reporting affordances on every read surface

Every read surface MUST expose moderation actions on every item it
returns. This is a NodeCore API discipline, not a UI suggestion.

| Read surface | Per-item actions |
|---|---|
| `compose_local_feed` | `report` (file P8); `block_actor` (locally lower trust); `mute_actor` (filter from your views without trust impact) |
| `compose_community_feed` | All of above + `report_to_cohort_moderators` (route via `delegates_to:moderator` at your cohort scope) |
| `compose_global_feed` | All of above + `submit_for_wa_review` (escalate to layer 3) |
| `compose_article_quality` | `dispute_quality` (file `Proposal` with subject_kind=quality_dispute) |
| `compose_witness_view` | `flag_for_diversity_audit` (file `hard_case:witness_diversity_violation`) |

The return type of every compose function carries an `actions`
block per item:

```rust
pub struct FeedItem {
    pub contribution_id: ContributionId,
    pub blob_sha: Option<Sha256>,
    pub payload_preview: PayloadPreview,
    pub actions: ItemActions,   // <- first-class
}

pub struct ItemActions {
    pub can_report: bool,                     // always true for non-self items
    pub can_block_actor: bool,                // always true for non-self
    pub can_mute_actor: bool,                 // always true for non-self
    pub can_dispute_quality: bool,            // when item carries quality scores
    pub can_escalate_to_wa: bool,             // when item ≥ federation cohort_scope
    pub default_legal_basis_hints: Vec<LegalBasis>,  // auto-suggested for media
}
```

LensCore + Agent-runtime client UIs read `ItemActions` directly and
render the affordances. **There is no path through the API that
returns a Contribution without surfacing how to moderate it.**

### 11.4 Cohort-scoped reporting routes

A `moderation_event` carries the same `cohort_scope` shape as the
content it targets. A report filed at:

* `cohort_scope: self` — local-only; never leaves the device.
  Useful for "remind me to revisit this" personal flagging that
  doesn't touch the federation
* `cohort_scope: family` — visible to family delegates only;
  routes to `delegates_to:moderator` keys at family scope
* `cohort_scope: community` — visible to community moderators
  (anyone the cohort has delegated to as moderator); standard P10
  witness-diversity gate at community-scaled N
* `cohort_scope: federation` / `global` — visible federation-wide;
  full P10 + WA-quorum adjudication path

This is the same locality dividend applied to moderation as to
content. A community handles community problems at community scale
without the federation needing to know.

### 11.5 The agent-delegation path, explicit

A user opts into agent-delegated moderation by emitting:

```
delegates_to:moderator:{my_agent_federation_key}
  scope: cohort_scope ∈ {self, family, community, federation, global}
  policy_ref: my_user_values_attestation_id
  active: true
```

The agent then receives a routed moderation queue for the scopes
the user delegated. Each queued item carries:

* The target Contribution + content (or hash-only if content
  unsuitable for agent review)
* The reporting party (if any) — agent moderation often originates
  from agent's own intake-gate refusals rather than third-party
  reports
* The user's stated values (from their P12 Goal projector + any
  attached policy attestations)

The agent's decision is a signed Contribution per usual. The H3ERE
pipeline trace is the audit record. **Every moderation decision by
the agent is reviewable by the user, reversible via P11
Reconsideration, and subject to the user's ongoing oversight.**

The user can revoke the delegation at any time by emitting
`withdraws` against the original `delegates_to`. Decisions made
under the now-withdrawn delegation remain in the audit chain but
the agent no longer receives the queue.

### 11.6 Why this avoids the failure modes of prior moderation systems

| Prior failure mode | What CEWP does instead |
|---|---|
| **Mastodon volunteer-admin bottleneck**: per-instance admins overworked, instance-owner is single point of failure ([Stanford 2023](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf), [IFTAS 2024](https://about.iftas.org/2024/12/17/the-2024-iftas-needs-assessment-report-is-here/)) | No platform-level admin role. Layer 1 (self) + Layer 2 (trust graph) handle most cases at consumer-scale; Layer 3 (WA quorum) handles federation-scale. Each community has a named moderator, but the seat is merit-replaced on lapse and every action is gated and appealable, so no instance owner is a single point of failure (CC 4.5.4). |
| **Bluesky labeler centralization**: a few labelers do most of the work; labeler keys become trust-monopolies | Labelers are just `delegates_to:moderator` participants; users compose multiple sources at any depth; no labeler is structurally privileged. |
| **Reddit reputational ranking vs moderation conflation**: subreddit moderators are the people with the most time, not the most judgment | The trust graph is depth-bounded and the substrate doesn't reward time-spent; moderators are those the cohort actually delegates to. |
| **YouTube/TikTok centralized policy + appeal-into-the-void**: Big Tech ToS as the law; appeals decided by the company that runs the platform | No central policy. Per-cohort discipline; P11 Reconsideration with fresh-quorum recusal as the universal appeal path. |
| **Twitter/X owner-policy-shifts**: every change of ownership rewrites the rules | Substrate has no owner. Rule-layer changes route through the amendment process (CC 4.5.1: federation Contribution + WA quorum + 1-of-n sign-off), and the process itself is entrenched (CC 4.5.1.2). |
| **Discord per-server-owner power**: server owners can do anything | No server-owner role exists at the substrate level; an operator hosts substrate but doesn't preside over its content discipline. |

### 11.7 Cross-references

This section unifies pieces specified elsewhere:

* CC 2.1 / CC 2.4 — `cohort_scope` + `delegates_to`
* CC 4.5.3 + CC 4.5.10.1 — fast-path takedown coordination + hash-database access
* CC 4.5.4 / CC 4.5.5 — named-moderator invariant; moderation as a delegable duty
* [MISSION §2.8 / §2.9 / §2.10 / §2.11](../MISSION.md#primitive-8-moderation-event) — P8/P9/P10/P11 primitives
* [FSD-002 §6.1.5](https://github.com/CIRISAI/CIRISRegistry/blob/main/FSD-002.md) — locality-scaled-quorum
* [TRUST_HIERARCHY.md](TRUST_HIERARCHY.md) — recursion depth + transitive trust
* §5 above — `takedown_notice` envelope
* §4 above — the age gate (CC 3.4.11 floor, CC 3.4.13 minors)

The `moderation_event` Contribution kind is the universal moderation
envelope; `takedown_notice` is the substrate-protective fast-path
variant. Both ride the standard P5 Contribution shape; both surface
through the same API discipline §11.3 establishes.

## 12. What CEWP takes from prior art (and rejects)

### 12.1 Validates / adopts

* [Bluesky / Ozone composable moderation](https://docs.bsky.app/blog/blueskys-moderation-architecture) — labels-as-signed-artifacts pattern → CEWP's open labeling + takedown_notice + tombstone structure (CC 4.5.5, CC 4.5.3)
* [Nostr NIP-11 relay policy declaration](https://nips.nostr.com/11) — declare substrate discipline at protocol level → CEWP's per-cohort content discipline
* [PornHub Dec 2020 verified-uploader purge](https://www.eff.org/deeplinks/2020/12/visa-and-mastercard-are-trying-dictate-what-you-can-watch-pornhub) → identity-aware uploads make per-actor eviction trivial
* [OnlyFans Ondato-based verification](https://ondato.com/case-studies/onlyfans-case-study/) → provider-attestation pattern works
* [Signal's defensible E2EE line](https://signal.org/bigbrother/) → self/family scope is the substrate's "we don't see, we don't carry" position
* [PhotoDNA + PDQ at the publication seam](https://github.com/facebook/ThreatExchange/tree/main/pdq) → substrate-protective hash-matching at scope-widening promotion or a `public`-room post (NOT user device; CC 1.13.3.4)
* [French SREN double-anonymity](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs) → provider attests against federation key, not real identity
* [DMCA §512(g) counter-notice](https://www.copyright.gov/512/) → maps to `reconsideration:new_evidence` (CC 4.5.3)

### 12.2 Rejects / avoids

* [Vidme ad-funded death](https://variety.com/2017/digital/news/vidme-shuts-down-video-app-1202628367/) → no ad-funded UGC substrate model; mission-locked AGPL
* [DTube / Steem / LBRY token rewards → farming + SEC risk](https://knightcolumbia.org/blog/mapping-social-media-crypto-logic-platforms-and-the-cautionary-tale-of-steemit) → no monetization primitives at substrate layer
* [BitChute "free speech" → extremist concentration](https://dl.acm.org/doi/10.1145/3710950) → positioning is "we don't centralize," not "we don't have rules"
* [Stanford Mastodon CSAM finding](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf) — volunteer per-instance moderation can't scale → substrate-level structural refusal + named-moderator invariant (CC 4.5.4) + perceptual hash match at the publication seam
* [Apple NeuralHash failure](https://arxiv.org/abs/2111.06628) — adversarial collisions on client-side scanning → hash-match at the publication seam, not user device
* TikTok/YouTube engagement-optimization → no centralized recommender; user controls the algorithm

## 13. Open questions

* **Live-streaming wire format details** — RESOLVED: realtime A/V is specified in CC 5.3.3 (SFrame + MLS RFC 9420; Edge `SealedAvChunk`/`ChunkLayer`, §2.8). CC 8.3.3 records the N>1 multicast half as substrate-pending (CIRISPersist#142 / CIRISRegistry#34), deployment deferred to Phase 2.
* **C2PA integration for image/video provenance** — RESOLVED by CC 8.4.2 (§8.1 above): `authenticity:provenance_chain` cites [C2PA](https://c2pa.org/) manifests via `evidence_refs[]`, no second interior canonicalization; CEG judgment layers atop C2PA provenance (EU AI Act Art. 50 path).
* **Perceptual hash database access** — PhotoDNA is gated to vetted orgs; PDQ is open-source. CC 4.5.7 makes the licensed hash database operator-provisioned and unshippable, and CC 4.5.10.1 records the access landscape; shared access at federation scale is still open.
* **CW community vs trusted-publisher tradeoff** — both paths can route adult content; do they coexist (yes per this FSD) or should one be the canonical pattern? Current answer: both coexist; CW communities for community-of-interest; publishers for one-to-many.
* **Operator coordination for fast-path takedowns** (TVEC 1-hour, GIFCT CIP) — specified at CC 4.5.3 (notice admission, tombstone, per-basis dispatch, audit).

## 14. References

### Internal

* [CEWP.md](CEWP.md) — platform identity
* [FEDERATION_SCALING_MODEL.md](FEDERATION_SCALING_MODEL.md) — substrate scaling
* [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) — the planned opt-in anonymous tier (CC 1.13.3.1; not yet shipped)
* [SCHEMA.md §4.29](../SCHEMA.md) — existing external_content
* [CIRIS Constitution 1.0-rc9](https://github.com/CIRISAI/CIRISConstitution/tree/v1.0-rc9/constitution) — wire-format authority (tag `v1.0-rc9`)

### External — moderation architecture prior art

* [Bluesky's Moderation Architecture](https://docs.bsky.app/blog/blueskys-moderation-architecture)
* [Stackable Approach to Moderation (Bluesky)](https://bsky.social/about/blog/03-12-2024-stackable-moderation)
* [Ozone — GitHub](https://github.com/bluesky-social/ozone)
* [Nostr NIP-11 — Relay Information Document](https://nips.nostr.com/11)
* [Mastodon — Moderation actions documentation](https://docs.joinmastodon.org/admin/moderation/)
* [Stanford — Child Safety on Federated Social Media (2023)](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf)
* [IFTAS 2024 Needs Assessment](https://about.iftas.org/2024/12/17/the-2024-iftas-needs-assessment-report-is-here/)
* [Collaborative Content Moderation in the Fediverse (arXiv)](https://arxiv.org/html/2501.05871v1)
* [Will Admins Cope? Decentralized Moderation in the Fediverse (CHI 2023)](https://dl.acm.org/doi/fullHtml/10.1145/3543507.3583487)

### External — content rating + age verification

* [MPA Film Ratings](https://www.filmratings.com/ratings-guide/)
* [BBFC Classification Guidelines](https://www.bbfc.co.uk/what-we-do/classification-guidelines)
* [PEGI](https://pegi.info/)
* [Common Sense Media](https://www.commonsensemedia.org/about-us/our-mission/about-our-ratings)
* [UK Online Safety Act age assurance (Cooley)](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/)
* [Arcom — French SREN technical guidelines](https://www.arcom.fr/en/find-out-more/legal-area/legal-resources/technical-guidelines-age-verification-protection-persons-under-18-online-pornography)
* [France SREN child protection (Bird & Bird)](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs)
* [Yoti — Age verification privacy](https://www.yoti.com/privacy/age-verification/)

### External — CSAM / takedown / standards

* [US DMCA §512](https://www.copyright.gov/512/)
* [EU DSA Article 16 (CMS)](https://www.cms-digitallaws.com/en/dsa/article-16/)
* [EU TVEC Regulation 2021/784](https://www.europarl.europa.eu/news/en/press-room/20210422IPR02621/new-rules-adopted-for-quick-and-smooth-removal-of-terrorist-content-online)
* [EU AVMSD](https://digital-strategy.ec.europa.eu/en/policies/audiovisual-and-media-services)
* [PhotoDNA — Technology Coalition](https://technologycoalition.org/resources/update-on-voluntary-detection-of-csam/)
* [PhotoDNA Analysis (ACM 2023)](https://dl.acm.org/doi/fullHtml/10.1145/3600160.3605048)
* [Meta PDQ — GitHub](https://github.com/facebook/ThreatExchange/tree/main/pdq)
* [Project Arachnid](https://protectchildren.ca/en/programs-and-initiatives/project-arachnid/)
* [NeuralHash collision attacks (arXiv)](https://arxiv.org/abs/2111.06628)
* [Signal — Government Communication](https://signal.org/bigbrother/)
* [GIFCT Hash-Sharing Database](https://gifct.org/hsdb/)
* [GIFCT CIP Buffalo debrief](https://gifct.org/2022/06/23/debrief-cip-activation-buffalo/)
* [EU AI Act Article 50](https://artificialintelligenceact.eu/article/50/)
* [US KOSA (S.1409)](https://www.congress.gov/bill/118th-congress/senate-bill/1409)

### External — prior art (failures + cautionary tales)

* [Variety — Vidme shutdown](https://variety.com/2017/digital/news/vidme-shuts-down-video-app-1202628367/)
* [BitChute Community Guidelines](https://support.bitchute.com/policy/guidelines/)
* [Content Moderation and Hate Speech on Alternative Platforms (PACM HCI)](https://dl.acm.org/doi/10.1145/3710950)
* [Pew Research on BitChute](https://www.pewresearch.org/short-reads/2023/02/17/key-facts-about-bitchute/)
* [Knight Institute — Steemit cautionary tale](https://knightcolumbia.org/blog/mapping-social-media-crypto-logic-platforms-and-the-cautionary-tale-of-steemit)
* [LBRY — Wikipedia](https://en.wikipedia.org/wiki/LBRY)
* [EFF — Visa/Mastercard vs PornHub](https://www.eff.org/deeplinks/2020/12/visa-and-mastercard-are-trying-dictate-what-you-can-watch-pornhub)
* [Ondato — OnlyFans Case Study](https://ondato.com/case-studies/onlyfans-case-study/)
