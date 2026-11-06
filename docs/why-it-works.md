# What this solves, and how

Decentralized networks have failed on a short list of problems for twenty years. None of the pieces below is new. The architecture is: the same technologies, arranged so that each problem is answered by a rule that every node computes identically and a proof that the rule holds. Where a problem is only narrowed, the residual is written down. *Pre-release: the shapes are CIRISConstitution `rc17`; where this page and the Constitution disagree, the Constitution wins.*

## 1. Two copies of the rules

**The problem.** Every networked application keeps its rules twice: once in the code that draws the buttons and once in the code that accepts or refuses the request. They drift, and the drift is where authorization bugs live. Hypermedia as the engine of application state was proposed in 2000 as the cure and almost never built.

**What this does.** There is one table, the family model: what a row is, who may sign it, under which policy, and what it does to the object's state. The admission door reads it. The queue a person sees is the same predicate run forwards: the transitions of the object's family enabled for that person's standing in the object's current state. Proved sound and complete against the door; a stranger sees nothing; an answered item leaves the next view.

**With.** A generated manifest from a census of the code, CDDL schemas for the view and the item, Lean for the proof, and plain JSON over a node's API. *Peers:* persist's schema-plus-relationship-graph lineage (Zanzibar, SpiceDB, OpenFGA), which decides but does not render; this closes the loop to the client.

## 2. Moderation without a trust and safety department

**The problem.** Federated networks either centralize moderation in an operator or leave it to whoever runs a server, and neither scales or answers to the people moderated.

**What this does.** Moderation is a delegable duty held by appointment, never by platform. A room's moderator acts alone on content, never on membership. Removal is a roster change under a floor no single vote can reach. Anyone may raise a flag; the room's live majority decides when no moderator acts. A room that loses its moderator locks and elects one in a 24-hour ranked vote whose ballots are the appointment's own signatures. Readers ignore, block and mute on their own side. Delegation to agents is a row the person signs.

**With.** Signed delegation edges, order-free folds, instant-runoff over signed ballots, and the same lifecycle table as everything else.

## 3. Identity and Sybil resistance without a registrar

**The problem.** Either a central registry decides who exists, or anyone can mint unlimited identities and outvote the real ones.

**What this does.** Nobody is admitted to anything without their own signed acceptance, and no single vote admits. Trust is each reader's own set of accepted roots, not a registry's entry. A person's devices are one roster, so counts are per person however many keys they hold. Roots of trust are founded by small quorums that count founders, not crowds. Relays on the anonymous tier are bounded per sender by the sender's own contacts, so minted identities buy no route.

**With.** Hybrid Ed25519 and ML-DSA signatures, a self roster per person, delegation edges with scope and term (the UCAN, Biscuit, ZCAP-LD and macaroon lineage), and a conferral ceremony for roots. *Peers:* Verify's trust-anchor store beside Veraison and CoRIM.

## 4. Consent and retraction in a replicated store

**The problem.** Once a statement replicates it cannot be unsaid, and systems either pretend it can or give up on consent.

**What this does.** Every row is forward-only. A withdrawal is a boundary, not a deletion: what came before stays provable and stops applying. Nothing is written on anyone's behalf; a consent is a choice the person signs, kept alive only as the same row, never widened silently, and never brought back after a withdrawal. Folds over held rows are order-free, so two devices that disagree offline resolve the same way on every node.

**With.** Append-only signed rows, permutation-invariant folds, and per-item durability chosen by the owner.

## 5. Who talks to whom

**The problem.** Encryption hides content and leaks the graph. Onion routing hides the graph from most observers and breaks under a global one.

**What this does.** Transport separates into three meshes (CC 5.4.3 / Edge v44): low-bandwidth shaped links (`L ≤ 8`, `≤ 64 KiB`), high-bandwidth 33 KB/s shaped seat links, and unshaped live/ALM relay trees. Links between peers that choose shaping carry a constant stream of identical `16,272 B` frames (`ρ = 0.5`), so timing and volume inside a link are hidden; what is hidden and what is not (`+` benefit / `−` downside) is stated to the person before they choose. Group addresses are derived, never announced. Strangers may carry only what no one must consent to carry. The anonymous tier (CC 5.4.6) is a separate dedicated identity, reached only at a rendezvous, in private rooms held on members' own always-on nodes, its first hop a sticky guard from the person's own contacts, walking `15,296 B` X-Wing onion frames across shaped links by replacing scheduled cover frames at **zero extra wire bytes**. What remains against a global observer and a compromised relay is written in the text with edge's numbers.

**With.** Poisson-timed cover traffic in the mixnet lineage (`Tiers.lean`: `each_home_adds_seven`), per-hop X-Wing post-quantum encapsulation in fixed-length `CONP` packets, Reticulum IPv6/NAT hole-punching + $k=2$ homed leaves, and HKDF-derived addresses.

## 6. Post-quantum from the first key

**The problem.** Retrofitting post-quantum cryptography means a migration no network has finished.

**What this does.** Every signature is hybrid and every key exchange is hybrid from the first key, with one composite construction and one hybrid KEM, and the next edition's signed object is CBOR with COSE so the rows are statements any transparency log can register.

**With.** ML-DSA-65 with Ed25519 as the IETF composite, X-Wing, COSE and SCITT, COSE receipts over RFC 9162 trees. *Peers:* Verify's comparison names the field as mostly classical with migration underway; the residual it names, no external audit yet, stands.

## 7. Twenty-nine languages without a release cycle

**The problem.** Localization lives in each client's binary and ships at app-store speed.

**What this does.** Every string a client shows is served by the node for the requested locale by a stable id. Clients hold no copy, so a new language or a corrected sentence reaches every surface, including a browser or a firmware node, with no release.

## 8. Governance that still works at ten thousand members

**The problem.** Fractional quorums become unreachable as a group grows, or someone picks the electorate.

**What this does.** Votes count against the group's expected-active base, a member-only count from a secret-shared census under a ratchet that cannot be gamed by silencing people. Every door runs one decision shape, five numbers published as data with its decision function and vectors. One person, one vote, however many devices.

**With.** Secret sharing to two drawn members, integer folds, a published policy table, and a model-checked decision machine.

## 9. Transparency without a transparency service

**The problem.** Audit logs are run by someone, and that someone can be compelled or captured.

**What this does.** Every build, licence and stream has its own Merkle log, witnessed by cosignatures; receipts are standard; and every rule in the constitution is tied in a public register to the code, test or proof that backs it, or says plainly that nothing does yet. There is no registrar, because a trust decides what a node serves. *Peers:* Trillian, Sigstore Rekor and immudb run the log as a service; SCITT names the registrar a Transparency Service; this keeps the log shape and drops the service.

## 10. Coordination without a coordinator

**The problem.** Fan-out trees, relay selection and leader choice need a coordinator, and the coordinator is the point of capture.

**What this does.** Every node computes the same answer from the same signed inputs: the relay tree, the layer deal, the vote count, the moderator seat. Determinism replaces coordination, and the inputs are hardened so one lie cannot become a universal one.

## 11. The protocol itself

**The problem.** Whoever controls the specification controls the network.

**What this does.** The constitution is forkable, its amendment process is entrenched behind the quorum it describes, authority rests with a named steward until a measured scale and tightens after, and the whole thing is checked by tooling that refuses to build when text and evidence disagree.


## Where it stands beside its peers

The three substrate repositories keep their own comparisons against the field, written to say where each is ahead, at parity and behind. They are the references for the claims above; read them before repeating any of this.

- **[CIRISVerify, "Where it stands vs. peers"](https://github.com/CIRISAI/CIRISVerify#where-it-stands-vs-peers)** and its full [standards comparison](https://github.com/CIRISAI/CIRISVerify/blob/main/docs/STANDARDS_COMPARISON.md): concise trust-anchor stores (Veraison, CoRIM), classical versus hybrid signature crypto (FIPS 204 from day one, the classical half bound by the post-quantum one), device attestation across four platforms in one store, RFC 6962 transparency logs beside Trillian and Sigstore Rekor, selective disclosure beside SD-JWT, ISO mdoc and BBS+, supply-chain provenance beside Sigstore, in-toto and SLSA, boot-state attestation (Keylime, absent here). Its own honest read: ahead on post-quantum posture, at parity on attestation, behind on ecosystem, scale and external review, with no third-party audit to date.
- **[CIRISPersist, "Standards it builds on" and "What is different"](https://github.com/CIRISAI/CIRISPersist#standards-it-builds-on)**: JCS (RFC 8785), RFC 6962 trees from Verify, delegation chains in the lineage of UCAN, Biscuit, ZCAP-LD and macaroons, authorization walks in the lineage of Zanzibar, SpiceDB and OpenFGA, RFC 6330 erasure coding, FIPS 204, and verifiable append-only state as immudb, Trillian, Rekor and IETF SCITT have it. Its own placement: the union of immudb, a SCITT transparency service whose registration policy decides what may be logged, and SpiceDB, with four differences it names: a hash-pinned, versioned registration policy whose refusals name the rule; conferrals that declare where they are enforced and a build that checks it; consent as a projection over the edge graph; commons operations under a reverse quorum. And its trade-offs, stated: the policy engine cannot be swapped, the vocabulary version is the crate version, and a gate is only as good as its witnesses.
- **[CIRISEdge, the protocol tour and the two-plane hybrid](https://github.com/CIRISAI/CIRISEdge#the-shape-of-the-protocol-one-paragraph-tour)**, its [standards comparison](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/STANDARDS_COMPARISON.md) and [lightnet/darknet](https://github.com/CIRISAI/CIRISEdge/blob/main/docs/LIGHTNET_DARKNET.md): fifteen signed envelope kinds over Reticulum, anti-entropy per peer and kind, attribution at ingest before any serve gate, consent gating that cannot exceed the transmission principle by construction, and the derived group plane that makes a group's existence structurally invisible to outsiders, with the measurement status of each claim and a multi-container acceptance bench that runs real video and blobs through real relays in CI.

The constitution's own placement against vendor constitutions, agent-governance work and the anonymity literature is in its register and its formal directory; the eleven problems above cite the standards each mechanism rests on, and the residuals are the parts those comparisons say are not yet won.

---

Every one of these mechanisms is on the page beside this one in miniature: six people, one mesh of rows, their views, and the queues their views generate as they act. The proofs and the tables are in CIRISConstitution `rc17` under `formal/` and `manifests/`.
