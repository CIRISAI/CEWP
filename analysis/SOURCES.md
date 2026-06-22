# Provenance manifest

All documents under `analysis/` were pulled from their **home repositories**
in the `CIRISAI` GitHub organization and copied here verbatim so the whole
philosophical–technical system can be reviewed in one place. Nothing here is
authored in CEWP; every file is a vendored copy. The home repo remains
canonical — re-pull before relying on any document for a decision.

**Pulled:** 2026-06-19 (shallow `git clone --depth 1` of each `main`/`master`).

| Source repo | Commit pinned | Role |
|---|---|---|
| [CIRISAccord](https://github.com/CIRISAI/CIRISAccord) | `6302d62` | Constitutional/ethics layer — the Accord books |
| [coherence-ratchet](https://github.com/CIRISAI/coherence-ratchet) | `f6f6b41` | Integration paper + Lean formal lake (Zenodo 20300774) |
| [RATCHET](https://github.com/CIRISAI/RATCHET) | `c266982` | Coherence/PoB papers, CCA paper, federation threat-model index |
| [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) | `2fb7a2c` | CEG 1.0-RC29 spec + CIRIS Constitution 0.4 |
| [CIRISNodeCore](https://github.com/CIRISAI/CIRISNodeCore) | `b4dbe49` | Consensus core — federation governance |
| [CIRISLensCore](https://github.com/CIRISAI/CIRISLensCore) | `3f6c0d8` | Observation/epistemology core |
| [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) | `dbbfb70` | Crypto + identity substrate |
| [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) | `d7a419a` | Storage substrate + federation directory |
| [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) | `77b64e5` | Mesh transport substrate |
| [CIRISServer](https://github.com/CIRISAI/CIRISServer) | `fb16ed7` | Fabric-node runtime (integration layer) |
| [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) | `7f2369b` | `fabric node + brain` — Accord/PoB/security |
| [CIRISBilling](https://github.com/CIRISAI/CIRISBilling) | `efeceee` | Threat model (peripheral) |
| [CIRISLens](https://github.com/CIRISAI/CIRISLens) | `be73e8f` | Threat model (retired deployment) |
| [CIRISProxy](https://github.com/CIRISAI/CIRISProxy) | `44ae015` | Threat model (peripheral) |
| [CIRISOssicle](https://github.com/CIRISAI/CIRISOssicle) | `1bc2f09` | Threat model (peripheral) |

## Threat models: canonical, not vendored

`RATCHET/FEDERATION_THREAT_MODELS/` carries **vendored copies** of every
repo's threat model. Several were stale at pull time, so the threat models in
`tier5-security/threat-models/` were taken from **each repo's own canonical
`docs/THREAT_MODEL.md`**, not from RATCHET's copies. Staleness measured at pull:

| Repo | RATCHET vendored | Canonical parent | Used |
|---|---|---|---|
| CIRISEdge | 893 lines | **1817 lines** | canonical (parent) |
| CIRISPersist | 980 lines | **2673 lines** | canonical (parent) |
| CIRISVerify | 540 lines | **819 lines** | canonical (parent) |
| CIRISLensCore | 939 lines | 939 lines (identical) | canonical (parent) |
| CIRISRegistry | 1896 lines | 1896 lines (identical) | canonical (parent) |
| CIRISBilling | 620 lines | 620 lines (identical) | canonical (parent) |
| CIRISLens | 945 lines | 945 lines (identical) | canonical (parent) |
| CIRISProxy | 532 lines | 532 lines (identical) | canonical (parent) |
| CIRISOssicle | 190 lines | 190 lines (identical) | canonical (parent) |

Only `FEDERATION_THREAT_MODEL.md` (the cross-repo aggregate) and the index
README have no standalone parent; those two are taken from RATCHET.
