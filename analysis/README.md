# CIRIS consolidated analysis set

This directory gathers the **load-bearing documents** of the CIRIS system,
pulled from their home repositories into one place so the whole stack can be
reviewed as a single philosophical–technical system rather than a pile of
independent software projects. The architecture's correctness depends less on
the Rust code than on whether the constitutional, epistemic, and governance
layers stay mutually consistent — these are the documents that let you check
that.

Layout follows the four-layer / eight-tier review frame:
**philosophy → constitution → governance → implementation.**

- Provenance (source repo + pinned commit for every file): [`SOURCES.md`](./SOURCES.md)
- Threat models are taken from each repo's **canonical** `docs/THREAT_MODEL.md`,
  not the stale vendored copies in RATCHET — see `SOURCES.md` for the staleness table.

> Everything here is a verbatim copy. The home repo stays canonical; re-pull
> before relying on any document for a decision.

---

## Tier 0 — Foundational philosophy (why the system exists)
*Answers: what is adaptive coherence; what is the Coherence Ratchet
mathematically; why coherence over other objectives; what constitutes benefit;
why accept M-1; what limits the system from becoming coercive.*

| Document | Source |
|---|---|
| [`CIRISAgent__ACCORD.md`](./tier0-philosophy/CIRISAgent__ACCORD.md) | CIRISAgent/ACCORD.md |
| [`CIRISAgent__MISSION.md`](./tier0-philosophy/CIRISAgent__MISSION.md) | CIRISAgent/MISSION.md |
| [`coherence-ratchet/`](./tier0-philosophy/coherence-ratchet/) | README, RESEARCH_PROGRAM, `papers/Corridor Dynamics.tex`, **Lean formal lake** (`formal/`) |
| [`RATCHET__ratchet-paper/`](./tier0-philosophy/RATCHET__ratchet-paper/) | RATCHET — *Constrained Reasoning Chains* (the RATCHET paper) |
| [`RATCHET__CIRISAGENT_PAPER/`](./tier0-philosophy/RATCHET__CIRISAGENT_PAPER/) | RATCHET — k_eff correlation paper |
| [`RATCHET__coherence_substrate_synthesis/`](./tier0-philosophy/RATCHET__coherence_substrate_synthesis/) | RATCHET — substrate synthesis paper |
| [`CIRISAgent__PROOF_OF_BENEFIT_FEDERATION.md`](./tier0-philosophy/CIRISAgent__PROOF_OF_BENEFIT_FEDERATION.md) | Proof-of-Benefit spec |
| [`RATCHET__MISSION_DRIVEN_DEVELOPMENT.md`](./tier0-philosophy/RATCHET__MISSION_DRIVEN_DEVELOPMENT.md) | M-1 / mission derivation |

## Tier 1 — Constitutional layer
*Answers: who is sovereign; who can change the rules; who can halt the system;
how constitutional legitimacy is maintained; can authority be revoked; what
happens if accord holders disagree.*

| Document | Source |
|---|---|
| [`CIRIS_Constitution/`](./tier1-constitution/CIRIS_Constitution/) | CIRISRegistry — Constitution 0.1.5 (`.tex` + `.pdf`) |
| [`CIRISAccord/`](./tier1-constitution/CIRISAccord/) | CIRISAccord — the Accord books + annexes + PDF |
| [`CEG__09_humanity_accord.md`](./tier1-constitution/CEG__09_humanity_accord.md) | CEG §9 humanity_accord (anti-capture) |
| [`CEG__11_governance.md`](./tier1-constitution/CEG__11_governance.md) | CEG §11 governance |

## Tier 2 — Federation governance
*Answers: how consensus forms; how expertise is recognized; how disputes are
resolved; how moderation works; how trust accumulates and decays.*

| Document | Source |
|---|---|
| [`CIRISNodeCore__MISSION.md`](./tier2-federation-governance/CIRISNodeCore__MISSION.md) | CIRISNodeCore/MISSION.md |
| [`CIRISNodeCore__CIRIS_FEDERATION.md`](./tier2-federation-governance/CIRISNodeCore__CIRIS_FEDERATION.md) | federation model |
| [`CIRISNodeCore__FEDERATION_ANNOUNCEMENT.md`](./tier2-federation-governance/CIRISNodeCore__FEDERATION_ANNOUNCEMENT.md) | announcement protocol |
| [`CIRISNodeCore__FEDERATION_SCALING_MODEL.md`](./tier2-federation-governance/CIRISNodeCore__FEDERATION_SCALING_MODEL.md) | scaling model |
| [`CIRISNodeCore__FEDERATION_TAB.md`](./tier2-federation-governance/CIRISNodeCore__FEDERATION_TAB.md) | federation tab/UI |
| [`CIRISNodeCore__README.md`](./tier2-federation-governance/CIRISNodeCore__README.md) | consensus primitives overview |

> Voting / expertise / moderation / deferral protocols live inside CIRISNodeCore
> and CEG §11; the deferral/voting wire forms are specified in the CEG spec
> under Tier 4.

## Tier 3 — Observation & epistemology
*Answers: what counts as evidence; how coherence is measured; what an epistemic
failure is; how the system separates truth / consensus / popularity / authority.*

| Document | Source |
|---|---|
| [`CIRISLensCore__MISSION.md`](./tier3-observation-epistemology/CIRISLensCore__MISSION.md) | CIRISLensCore/MISSION.md |
| [`CIRISLensCore__README.md`](./tier3-observation-epistemology/CIRISLensCore__README.md) | lens core overview |
| [`server-lens-core__MISSION.md`](./tier3-observation-epistemology/server-lens-core__MISSION.md) | in-tree lens core (CIRISServer) |
| [`RATCHET__CCA_PAPER/`](./tier3-observation-epistemology/RATCHET__CCA_PAPER/) | Coherence Collapse Analysis |
| [`RATCHET__coherence_detector_paper.tex/.pdf`](./tier3-observation-epistemology/) | coherence detector paper |

## Tier 4 — Registry / trust layer
*Answers: what a credential means; sovereign vs registered member; how licensure
interacts with consensus; how revocations are enforced.*

| Document | Source |
|---|---|
| [`CEG-1.0-RC29-spec/`](./tier4-registry-trust/CEG-1.0-RC29-spec/) | **CEG 1.0-RC29 wire grammar** §0–§19 (the spec authority) |
| [`CIRISRegistry__MISSION.md`](./tier4-registry-trust/CIRISRegistry__MISSION.md) | CIRISRegistry/MISSION.md |
| [`CIRISRegistry__FEDERATION_CLIENT.md`](./tier4-registry-trust/CIRISRegistry__FEDERATION_CLIENT.md) | federation client |
| [`CIRISRegistry__FSD-002_FEDERATION_SURFACE.md`](./tier4-registry-trust/CIRISRegistry__FSD-002_FEDERATION_SURFACE.md) | federation surface |
| [`CIRISPersist__FEDERATION_TRUST_INTERFACE.md`](./tier4-registry-trust/CIRISPersist__FEDERATION_TRUST_INTERFACE.md) | trust contract |

## Tier 5 — Security model
*For every repo: threat model, security audits. Key surfaces: key compromise,
signature forgery, attestation spoofing (Verify); history rewriting (Persist);
eclipse/Sybil (Edge); steward compromise, forged licenses, revocation
suppression (Registry); metric gaming, score poisoning (Lens); governance
capture, vote buying, expertise laundering (NodeCore); prompt injection,
jailbreaks, self-modification (Agent).*

[`threat-models/`](./tier5-security/threat-models/) — **canonical** per-repo
threat models (Verify, Persist, Edge, LensCore, Registry, Billing, Lens, Proxy,
Ossicle) + the cross-repo `FEDERATION_THREAT_MODEL.md` aggregate. Plus:
[`CIRISAgent__SECURITY.md`](./tier5-security/CIRISAgent__SECURITY.md),
[`CIRISAgent__SECURITY_AUDIT_PROHIBITED_CAPABILITIES.md`](./tier5-security/CIRISAgent__SECURITY_AUDIT_PROHIBITED_CAPABILITIES.md),
[`CIRISPersist__SECURITY_AUDIT_v0.1.4.md`](./tier5-security/CIRISPersist__SECURITY_AUDIT_v0.1.4.md).

## Tier 6 — Substrate analysis
*Answers: can the federation survive partition / steward loss / registry loss /
malicious nodes.*

Verify (`MISSION`, `README`, `FEDERATION_IDENTITY`, `FSD-003`,
`IMPLEMENTATION_ROADMAP`), Persist (`MISSION`, `README`,
`PLATFORM_ARCHITECTURE`, `FEDERATION_DIRECTORY`, `ROADMAP`), Edge (`MISSION`,
`README`, `FEDERATION_SCALING_MODEL`, `ROADMAP_TO_V4`) —
see [`tier6-substrate/`](./tier6-substrate/).

## Tier 7 — Integration layer
*Answers: do the pieces actually compose; are assumptions preserved across
layers; does decentralization survive implementation.*

[`CIRISServer__MISSION.md`](./tier7-integration/CIRISServer__MISSION.md),
[`CIRISServer__README.md`](./tier7-integration/CIRISServer__README.md),
[`CIRISAgent__ARCHITECTURE_OVERVIEW.md`](./tier7-integration/CIRISAgent__ARCHITECTURE_OVERVIEW.md),
[`CIRISAgent__MISSION.md`](./tier7-integration/CIRISAgent__MISSION.md). CEWP's own
[`FSD/`](../FSD/) and [`docs/`](../docs/) are the platform-level integration
documents.

---

## The contradictions worth testing once it's all in one place

1. **Constitutional** — can humanity-accord authority coexist with federation sovereignty? (T1 + CEG §9)
2. **Epistemic** — can Lens stay observational if it is grounded in Accord-defined values? (T3 + T1)
3. **Governance** — can NodeCore avoid becoming a de facto ruling class? (T2 + CEG §11)
4. **Economic** — can Proof-of-Benefit resist gaming? (T0 PoB + T5)
5. **Security** — can sovereign membership resist Sybil without becoming permissioned? (T4 + Edge T5)
6. **Decentralization** — can Registry stay authoritative while Edge claims no node is special? (T4 + T6)
7. **Ethical** — can M-1 stay adaptive without collapsing into moral relativism? (T0 + T1)
8. **Mathematical** — does the Coherence Ratchet actually imply what later documents rely on? (T0 papers + `coherence-ratchet/formal/` Lean)
