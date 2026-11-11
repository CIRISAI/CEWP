# Owed corrections and checks, collected from the twin packages

Collected verbatim by the integrator from each merged package's `FSD/twin/<package>.md`, for the
lead to review. Nothing here has been applied to `FSD/SIMULATION_ENGINE.md` or the README.
A package with no entry listed nothing as owed in its spec at the merged head.

## globe (`FSD/twin/globe.md`, at twin/globe 77a5874)

> Cross-platform check owed when media lands: the page's media buffers must match a native run bit for bit.
> twin-media's basis (a source read, not a run): integer µs/bytes engine, exact `Rng::f`, no hash-order
> iteration; the one transcendental (`f64::ln` in `BackgroundModel::next_gap_us`) runs only with household
> contention on, which the view scenario leaves off. A view with contention on may differ by an ulp in wasm.

**Done:** `views_media_bits_match_the_wasm_build` (kernel-wasm views.rs) checks the FNV-1a digest of
slots 20-25 natively against the built wasm's.

## nat, topology, people, holon, evidence, media

No item marked owed in their specs at the merged heads (3115c11's nat, 0f0aef4, 77c0bc2, 13682fa,
140d068, 716379e). The globe item above names media: it is now merged, and the check is still owed.

## calibration (`FSD/twin/calibration.md` §10, at twin/calibration c13cee5)

**Applied on twin/globe (the commit that adds this line):** §4.7.2 to §4.7.4 regenerated from
`cargo run --release -p cewp-sim --example capacity_sweep` on the merged calibrated model (every
knee an interval against g), the prose corrections below made in §4.7.1, the reading bullets and
§4.7.5, and README :13. The `docs/` items were done by twin-globe (seat card on `view_seat_fleet`,
knee from `calibration_view_knee`).

> For the lead (this package does not edit the main FSD, README or `docs/`). Line numbers at
> f73b918.
>
> - `FSD/SIMULATION_ENGINE.md` :438 `bundle_bootstrap` source: "CIRISEdge#971 c580298" →
>   "release/45.0.0 at 8e46165 (runtime.rs `BUNDLE_ROUND_PLANES` :1140)".
> - :553, :569, :577–579, :600, :616, :627, :671, :794: "v45.0.0 (in release)" and the c580298
>   cites → "release/45.0.0 at 8e46165, untagged when read"; c580298 → e3c20ec, bb6ca83 →
>   3bffbdc, 4897789 → 7fa7105, 95a85ca → 5f86c2d, 9a04428 → 6660677 (none of the old shas is an
>   ancestor of 8e46165).
> - :579, :674, :757: Revocation and occurrence-withdrawal walk own ∪ cohort, which on a
>   canonical is every key held: 8,211 to 8,698 at seat-1, not "tens"/30.
> - :627: "the listing cache and read budget are MODEL until measured" → both BUILT and on by
>   default at 8e46165; only their timings are MODEL.
> - :584–585, :620, :714: residuals 684.5 / 4,703 / 4,407 → 678.5 / 4,697 / 4,401 (send-set
>   floor added); add that the TTL burst cannot occur at 8e46165 (stale-while-revalidate, ≤256
>   reads a listing).
> - :636–637, :646, :674–675, :802: drop 18,229 / 18,200 / "about 18,000", 47,709 / 47,700 and
>   7,355 / 7,400 → the growth table in section 4 (knee interval against g, binding term, never
>   one count).
> - :600–760 generated tables and parameter rows: regenerate from `capacity_sweep` on this branch.
> - :727, :736, :754: c580298 line cites → 8e46165 (runtime.rs :1113, :2637; bridge.rs :7478,
>   :13152).
> - `README.md` :13: "CIRISEdge v45.0.0 (in release), read at commit c580298" → "release/45.0.0
>   at 8e46165 (untagged)"; any knee figure → the interval against growth.
>
> `docs/` (owner `twin-evidence`): globe.js :291 (version/commit line), :1909, :1914
> `BS_TITLE[1]`, :1915–1917, :2485 (single `k45.knee` → interval from
> `calibration_view_knee`), :2486 (cache and budget built), :2487 (Global planes walk every key;
> TTL revalidates ≤256 reads a listing). No figure in `docs/*.html`.

## Not yet merged

- failures: no commits on twin/failures yet.
