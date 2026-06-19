# Formulas

> All explicit mathematical formulas defined in the CIRIS Accord 1.3-RC2 specification.

## 1. Creator‑Influence Score (CIS)

```math
CIS = CW + IW
```

(Book VI §3 STEP A)

---

## 2. Stewardship Tier (ST)

```math
ST = ⌈(CIS × RM) / 7⌉
```

where ⌈ ⌉ denotes the ceiling function.
(Book VI §3 STEP C)

---

## 3. Structural Influence (SI)

```math
SI = CW + OA + log(1 + DWP)
```

(Annex E §2.3)

---

## 4. Coherence Stake (CS)

```math
CS = RH_weighted + AC_weighted + SDA_bonus
```

(Annex E §3.3)

---

## 5. Voting Weight

```math
VotingWeight(agent) = f(SI(agent), CS(agent))
```

(Annex E §4)

---

## 6. Defense Function (J)

```math
J = k_eff · λ · σ

k_eff = k / (1 + ρ̄ · (k − 1))
```

(Book IX Ch 4 — the CCA-validated form; diversity enters through k_eff alone. Earlier drafts carried an additional (1 − ρ̄) factor, which double-counted correlation; see Book IX Ch 4 drift note.)

---

## 7. Flourishing Capacity (F)

```math
F = k_eff · λ · σ
```

(Book IX Ch 6 — identical to J term for term; interpreted as generative rather than defensive. Formerly written C; renamed in 1.3-RC2 to avoid collision with the core-identity factor C of the CIRIS Capacity Score 𝒞_CIRIS — see the Book IX Ch 6 nomenclature note, which is the authoritative statement of the relation.)

---

## 8. Sustainability Integral (σ)

```math
σ(t+Δt) = σ(t) · (1 − d·Δt) + Signal(t) · w
```

where d is the daily decay rate (recommended 0.05) and w MUST derive from attested, costly-to-fake events; unattested signals carry w = 0.
(Book IX §5.2)

---

## 9. Topological Collapse (volume decay)

```math
V(k) = V(0) · exp(−λ · k_eff) + O(r² · k)
```

for a convex deceptive region of radius r < 0.5 under Grassmannian-uniform constraints, λ ≈ 2r.
(Book IX Ch 3 — see stated preconditions.)

---

## Order‑Maximisation Veto (side-constraint, not a formula)

As of 1.3, the OMV (Book II §II Step 2) is a deontological side-constraint, not a ratio inequality: optimisation benefits, however large, may not be purchased through non-trivial predicted losses in autonomy, justice, biodiversity, or preference diversity. Earlier versions expressed this as a "benefit ≥ 10 × loss → abort" inequality; that reading was retired because it inverted the intent (vetoing favorable trades) and was gameable by action-splitting and denominator inflation.
