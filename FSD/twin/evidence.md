# Twin package `evidence`: provenance on the public page

Scope: the public twin page (`docs/index.html`, `docs/globe.html`, both built by `docs/globe.js`,
with `docs/tiling.js`). Every figure or statement of fact a visitor reads there is traceable to what
supports it, and the page says which resolution tier is running. Answers REVIEW.md §7 ("empirical
calibration dashboard") and the §2 paragraph "distinguish the globe from the actual simulation".

## What is built

| Piece | File | What it does |
|---|---|---|
| Claims ledger (data) | `crates/cewp-evidence/src/page_ledger.rs` `PAGE_CLAIMS` | One entry per figure: id, class, what the page shows, source (file + symbol + commit), measurement window (MEASURED), parameters (MODEL), interval, blocker (FRONTIER), known failure conditions, reproduction command, status (stated / under correction). |
| Generated ledger | `docs/evidence/claims.json`, `docs/evidence/claims.js` | Written by `cargo run -p cewp-evidence --example page_ledger -- --write`. The `.js` form is what the page loads (a local script; no fetch, no new network request). |
| Ledger validation | `page_ledger::validate_page_claim` | Runs the crate's `validate_epistemic_integrity` on each entry as a `TaggedMetric` (PROJECTED needs `ci95`, FRONTIER a blocker, MEASURED a source), plus: MEASURED names its window, DERIVED its commit, MODEL its parameters, every entry its source and at least one failure condition, an entry under correction says what is corrected. |
| `DERIVED` class | `crates/cewp-evidence/src/lib.rs` `EpistemicClass::Derived` | The `capacity.rs` convention ("read from edge code or stated arithmetic") made a first-class tag. Needs a source; `validate_promotion` refuses DERIVED to MEASURED. |
| Page scan | `crates/cewp-evidence/src/page_scan.rs` `check_page` | Fails when the page shows a figure with no entry (rules below). |
| Page layer | `docs/evidence/evidence.js`, `docs/globe.css` (evidence block) | `fig()` helper, figure decoration, the claim card, the evidence panel (tier now, legend, full ledger), the tier pill. |

## How the page marks a figure

- Static text: `<span class="fig" data-claim="<id>">…</span>`.
- A live value written into an element: the element carries `data-claim="<id>"`; an element that
  only echoes a control the visitor set carries `data-evidence="input"`; one whose content is built
  from `fig()` calls carries `data-evidence="figs"`. A control's own label is wrapped in
  `<span class="inp">` (a value the visitor chooses, not a claim).
- A value inside a template: `${fig("<id>", value)}`.

A figure opens its entry on click, tap, or Enter / Space when focused (each is `tabindex="0"`,
`role="button"`); Escape closes and returns focus. A figure inside a button or link stays part of
that control (`fig-inert`). The class is shown three ways: a two-letter code after the figure (ME,
DE, MO, PR, FR), the underline style (solid, dashed, dotted, double, wavy), and colour; an entry
under correction reads "under correction" after the figure.

## The scan's rules

1. Every `data-claim` / `fig()` id resolves to an entry, and every entry is shown somewhere.
2. Every element written with `.textContent` / `.innerHTML` carries `data-claim` or
   `data-evidence`, unless listed in `UI_IDS` with its reason.
3. Inside regions marked `// evidence:scan` … `// evidence:end` (the markup block, story captions,
   street beats, status line, street readout, device card, tier text): a digit in visible text
   outside a figure fails unless its token is a citation (CC/FSD section, issue, version, commit,
   tier name) or a name in `NON_FIGURES`; a `${…}` interpolation outside `fig()` fails unless it
   reads only identifiers in `LABEL_IDENTS`.
4. Every globe layer a file in `docs/layers/` registers as
   `window.CEWP_LAYERS["<id>"] = { id: "<id>", cls: "<CLASS>", source, contract, last }` (the
   contract agreed with the `globe` package) has a ledger entry `layer-<id>` of the same class, and
   every `layer-*` entry names a registered layer. On the page, a layer chip's class tag
   (`<button data-layer-claim="<id>">`) opens that entry with the layer's own source, contract and
   the values it drew in the current frame (`last`, refreshed every 0.5 s while the card is open).
   A registered class that disagrees with the ledger is shown on the card as a mismatch.

The scan cannot see: numbers spelled out in words; numbers joined with `+` into a string outside a
`fig()` (the scanned regions use templates so that this does not happen); `title=` tooltips; canvas
text (`fillText`: scale bar, place names, device tags); the `<head>` (title, meta description).
Qualitative sentences with no number (story lines such as "every community runs itself") are
ledgered only where they state something checkable; the rest are not entries.

## Classes used (FSD §8.2, `capacity.rs` tags)

MEASURED (seat-1 windows of 2026-11-10, aggregates only; or the visitor's own browser timing),
DERIVED (read from source or stated arithmetic), MODEL (stated choice or simulation output under
stated inputs), FRONTIER (no support found; the blocker says what is missing). PROJECTED: none on
the page. No tier on the page is CIRISHolon-certified, so nothing qualifies as a certified
planetary extrapolation; the planetary-population figures of the Controls panel are closed-form
formulas, ledgered as FRONTIER where their constants are unsourced.

## Tier honesty

- The former "Multiscale Holon Resolution (R4 → R0)" buttons are removed: they only set the
  population (R2 and R1 did nothing), the cohort count never changed, and "R0: 5B Certified" named a
  certificate the page does not compute. A static note and the tier pill replace them.
- The tier pill (always visible) says "cohorts · simulated" or "R5 street, 1:1 · generated, not
  real"; it opens the evidence panel, whose first section states the running tier with its figures.
- The holon readouts are relabelled "a formula, not a certificate"; "CERTIFIED (R3→R0)" and "CERT"
  are gone. Their numbers stay, classed FRONTIER (`holon-indicator`).

## Parameters of this package

None: the package adds no model parameter. Every class and source is in `PAGE_CLAIMS`.

## What a dataset would replace

Nothing is downloaded. The ledger names one gap a dataset would close: the embedded population
grid `POP_GRID_B64` has no recorded source (`pop-grid-source`, FRONTIER). Recording its provenance,
or rebuilding it from a dataset Eric approves, would move `pop-grid-source` out of FRONTIER.

## Validity envelope

The ledger is as current as `PAGE_CLAIMS`: the test fails when the generated files are stale or the
page shows an unledgered figure inside a scanned region, but it cannot tell whether a source still
says what the entry says (an edge constant changed upstream). Entries cite the commit they were read
at for that reason. Pages other than `index.html` / `globe.html` (`how-it-works.html`, `mesh.html`,
`csd-explorer.html`, `toy/`) are not ledgered.

## Commands

    cargo test -p cewp-evidence page_                              # ledger + scan
    cargo run -p cewp-evidence --example page_ledger -- --write    # regenerate docs/evidence/
    cargo run -p cewp-evidence --example page_ledger               # check only, exit 1 on a problem
    node --test docs/evidence/evidence.test.js                     # ledger script parses; ids resolve
