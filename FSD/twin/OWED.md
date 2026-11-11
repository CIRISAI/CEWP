# Owed corrections and checks, collected from the twin packages

Collected verbatim by the integrator from each merged package's `FSD/twin/<package>.md`, for the
lead to review. Nothing here has been applied to `FSD/SIMULATION_ENGINE.md` or the README.
A package with no entry listed nothing as owed in its spec at the merged head.

## globe (`FSD/twin/globe.md`, at twin/globe 77a5874)

> Cross-platform check owed when media lands: the page's media buffers must match a native run bit for bit.
> twin-media's basis (a source read, not a run): integer µs/bytes engine, exact `Rng::f`, no hash-order
> iteration; the one transcendental (`f64::ln` in `BackgroundModel::next_gap_us`) runs only with household
> contention on, which the view scenario leaves off. A view with contention on may differ by an ulp in wasm.

## nat, topology, people, holon, evidence

No item marked owed in their specs at the merged heads (3115c11's nat, 0f0aef4, 77c0bc2, 13682fa,
140d068).

## Not yet merged

- calibration (twin/calibration 859250b): its spec has a section "10. Corrections owed in
  `FSD/SIMULATION_ENGINE.md` §4.7 and README", still a placeholder (`RESULTS_CORRECTIONS`) at that
  head; to be collected when the branch merges.
- media, failures: not merged.
