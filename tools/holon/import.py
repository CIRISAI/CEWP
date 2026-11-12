#!/usr/bin/env python3
"""Import one CIRISHolon statistics table into the twin (FSD/twin/holon-intake.md section 1).

Usage: python3 -I tools/holon/import.py <CIRISHolon checkout> <commit> <ref>
       ref: uneven0 | household0_us | household0_de | power0 | devices0_us | presence0_us | grid0

Reads the table and its sidecar from the checkout at the pinned commit (`git show`; nothing is
fetched from the network), validates it (integers where the table promises integers; no key that
could carry a real feature's coordinates, id, name or tags; share vectors within the sidecar's
tolerance), applies the fixes the sidecar states (UNEVEN-0: renormalise each share vector to
1,000,000 by largest remainder, recording every vector that moved), writes the compact page asset
docs/geo/holon/<ref>.json and its entry in docs/geo/holon/index.json. The ledger entry
(crates/cewp-evidence/src/page_ledger.rs, id "holon-<ref>") carries the sidecar's notice;
docs/geo/holon/intake.test.js ties the two together.
"""
import hashlib, json, os, subprocess, sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
OUT = os.path.join(ROOT, "docs", "geo", "holon")
FORBIDDEN = {"lat", "lon", "latitude", "longitude", "coordinates", "geometry", "bbox", "osm_id", "feature_id",
             "tags", "wikidata", "geoid", "fips", "ags", "address", "x", "y"}
SOURCES = {
    "power0": ("conformance/scene/tables/power0", ["power_table.json"]),
    "devices0_us": ("conformance/scene/tables/devices0_us", ["device_table.json"]),
    "presence0_us": ("conformance/scene/tables/presence0_us", ["presence_table.json"]),
    "uneven0": ("conformance/scene/tables/uneven0", ["transition_table.json", "transition_table_by_region.json"]),
    "household0_us": ("conformance/scene/tables/household0_us", ["household_table.json"]),
    "household0_de": ("conformance/scene/tables/household0_de", ["household_table.json"]),
    "grid0": ("conformance/scene/tables/grid0", ["grid_table.json"]),
}


def show(holon, commit, path):
    return subprocess.run(["git", "-C", holon, "show", f"{commit}:{path}"], check=True, capture_output=True).stdout


def no_real_features(obj, where="table"):
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k.lower() in FORBIDDEN:
                raise SystemExit(f"refused: key {k!r} in {where} could carry a real feature")
            no_real_features(v, f"{where}.{k}")
    elif isinstance(obj, list):
        for v in obj:
            no_real_features(v, where)


def apportion(total, weights):
    """Largest remainder, ties to the lower index (the rule popchain and holon-scene share)."""
    s = sum(weights)
    if s == 0:
        return [0] * len(weights)
    base = [total * w // s for w in weights]
    rem = [(total * w % s, -i) for i, w in enumerate(weights)]
    for _, ni in sorted(rem, reverse=True)[: total - sum(base)]:
        base[-ni] += 1
    return base


def ppm_vector(v, tol, label, moved):
    if not all(isinstance(x, int) and x >= 0 for x in v):
        raise SystemExit(f"refused: {label} is not non-negative integers")
    if abs(sum(v) - 1_000_000) > tol:
        raise SystemExit(f"refused: {label} sums to {sum(v)}, outside the sidecar's tolerance {tol}")
    out = apportion(1_000_000, v)
    if out != v:
        moved.append({"vector": label, "sum_before": sum(v), "largest_move": max(abs(a - b) for a, b in zip(out, v))})
    return out


def uneven0(t, reg, moved):
    world = {"tiles": [], "people": [], "cells": [], "people_total": []}
    for row in t["rows"]:
        if row["grain"] != 1:
            continue  # the generator reads grain 1, each cell once (holon-scene incell::divide)
        c = row["cell_class"]
        world["tiles"].append(ppm_vector(row["tiles_each_cell_once"], 300, f"world class {c} tiles", moved))
        world["people"].append(ppm_vector(row["people_each_cell_once"], 300, f"world class {c} people", moved))
        world["cells"].append(row["cells"])
        world["people_total"].append(row["people"])
    regional = [[None] * 5 for _ in reg["regions"]]
    for row in reg["rows"]:
        r, c = row["region"], row["cell_class"]
        e = {"cells": row["cells"], "verdict": row["verdict"], "reason": row["reason"]}
        if row.get("tiles_each_cell_once") is not None:
            tol = 2000  # the regional cut's stated precision: three significant digits, within 2,000 parts
            e["tiles"] = ppm_vector(row["tiles_each_cell_once"], tol, f"region {r} class {c} tiles", moved)
            e["people"] = ppm_vector(row["people_each_cell_once"], tol, f"region {r} class {c} people", moved)
        regional[r][c - 1] = e
    return {"unit": "ppm", "tile_classes": t["tile_classes"], "cell_classes": t["cell_classes"], "grain": "1 pixel (about 0.9 km), each cell once",
            "world": world, "regions": reg["regions"], "regional": regional,
            "unlisted_parents": reg["unlisted_parents"], "precision": reg["precision"]}


def household(t, country):
    classes = ["nation", "village", "suburban", "terraced", "apartment blocks", "dense fabric"]
    out = {"classes": classes, "rows": {}}
    if country == "us":
        rows, verd = t["keys"]["tract"]["rows"], t["verdicts"]["tract"]
        bands = ["1", "2", "3", "4", "5", "6", "7+"]
    else:
        rows, verd = t["rows"], t["verdicts"]
        bands = None
    for c in classes:
        if c not in rows:
            continue
        r, v = rows[c], verd.get(c, {})
        e = {"bands": r.get("bands", bands), "shares_ppm": r["shares_ppm"],
             "persons_per_household": round(r["persons_per_household"], 4), "one_person_share": round(r["one_person_share"], 4)}
        for k in ("mean", "p1"):
            if k in v:
                e[k] = {"verdict": v[k]["verdict"], "reason": v[k]["reason"]}
                for f in ("generator", "reference", "budget"):
                    if f in v[k]:
                        e[k][f] = round(v[k][f], 4)
        out["rows"][c] = e
    g = t["generator"]
    out["generator"] = {"mean": g["final"]["mean"] if "final" in g else g["mean"], "p1": round((g["final"]["p1"] if "final" in g else g["p1"]), 4)}
    if "final" in g:
        out["generator"]["shares_by_size_0_to_7"] = [round(x, 6) for x in g["final"]["shares"]]
    return out


RUST = os.path.join(ROOT, "crates", "cewp-sim", "src", "holon_tables.rs")


def rust_tables():
    """crates/cewp-sim/src/holon_tables.rs from docs/geo/holon/uneven0.json: the tables the kernel's
    in-cell seam reads, compiled in so native and wasm read the same integers."""
    a = json.load(open(os.path.join(OUT, "uneven0.json")))
    d = a["data"]
    arr = lambda v: "[" + ", ".join(str(x) for x in v) + "]"
    rows = lambda m: "[\n" + "".join("    " + arr(v) + ",\n" for v in m) + "]"
    reg = []
    for r in d["regional"]:
        cells = []
        for e in r:
            cells.append(f"Some(({arr(e['tiles'])}, {arr(e['people'])}))" if "tiles" in e else "None")
        reg.append("    [" + ", ".join(cells) + "],\n")
    return ("// Generated by tools/holon/import.py from docs/geo/holon/uneven0.json; do not edit.\n"
            f"//! CIRISHolon UNEVEN-0 (commit {a['holon_commit']}), grain 1 pixel, each cell once, parts per million,\n"
            "//! renormalised to 1,000,000 by largest remainder as the sidecar says. Row = cell class 1..=5 (index 0..5),\n"
            "//! column = tile class 0 (empty) and 1..=5. MODEL via CIRISHolon; every row unresolved.\n\n"
            "/// World rows: share of a cell's tiles at each tile class.\n"
            f"pub const UNEVEN0_TILES: [[u64; 6]; 5] = {rows(d['world']['tiles'])};\n"
            "/// World rows: share of a cell's people at each tile class.\n"
            f"pub const UNEVEN0_PEOPLE: [[u64; 6]; 5] = {rows(d['world']['people'])};\n"
            "/// Regional rows by the twin's nine regions (docs/geo/regions.json order): (tiles, people), or None\n"
            "/// where the region holds fewer than 30 cells of the class (the world row stands in).\n"
            "#[allow(clippy::type_complexity)]\n"
            f"pub const UNEVEN0_REGIONAL: [[Option<([u64; 6], [u64; 6])>; 5]; 9] = [\n{''.join(reg)}];\n"
            f"/// The Holon commit the tables come from.\npub const UNEVEN0_COMMIT: &str = \"{a['holon_commit']}\";\n")


def power(t):
    keep = ("device_class", "state", "value", "unit", "would_be_on_anyway", "year", "basis")
    return {"rows": [{k: r.get(k) for k in keep} for r in t["rows"]], "finding": t.get("finding"), "not_available": t.get("not_available"),
            "corrections": t.get("corrections")}


def devices(t):
    kinds = {k: {"groups": v["groups"], "mean": [round(x, 4) for x in v["mean"]], "none": [round(x, 4) for x in v["none"]]} for k, v in t["kinds"].items()}
    pp = {k: {f: (round(x, 4) if isinstance(x, float) else x) for f, x in v.items()} for k, v in t["per_person"].items()}
    return {"per_person": pp, "kinds": kinds, "three_rates": t.get("three_rates"), "have_nots_cluster": t.get("have_nots_cluster")}


def presence(t):
    a = t["all_persons"]
    r4 = lambda m: [[[round(x, 4) for x in h] for h in d] for d in m]
    tn = t["three_numbers"]
    return {"states": t["states"], "day_types": t["day_types"], "table": r4(a["table"]), "standard_error": r4(a["standard_error"]), "diaries": a["diaries"],
            "by_day_of_week": {"days": t["by_day_of_week"]["days"], "table": r4(t["by_day_of_week"]["table"])}, "three_numbers": tn, "years": t["years"]}


def grid(t):
    """GRID-0: every row kept as published (value, unit, year, kind, trust, source, licence, how it was read),
    with the ISO 3166-1 numeric code of a country row from docs/geo/country_iso.json (the Natural Earth names the
    country mask is built from); a country row whose name is not there is refused, never guessed."""
    names = json.load(open(os.path.join(ROOT, "docs", "geo", "country_iso.json")))["names"]
    keep = ("id", "place", "place_kind", "parent", "year", "value", "unit", "kind", "trust", "source", "licence", "read", "original_value")
    rows = []
    for r in t["rows"]:
        e = {k: r.get(k) for k in keep}
        if r["place_kind"] == "country":
            if r["place"] not in names:
                raise SystemExit(f"refused: country {r['place']!r} has no ISO code in docs/geo/country_iso.json")
            e["iso"] = names[r["place"]]
        rows.append(e)
    return {"conventions": t.get("conventions"), "rows": rows}


def main():
    if sys.argv[1] == "--check-rust":
        same = os.path.exists(RUST) and open(RUST).read() == rust_tables()
        print("holon_tables.rs current" if same else "holon_tables.rs stale: run tools/holon/import.py ... uneven0")
        sys.exit(0 if same else 1)
    holon, commit, ref = sys.argv[1], sys.argv[2], sys.argv[3]
    base, files = SOURCES[ref]
    side_b = show(holon, commit, f"{base}/sidecar.json")
    side = json.loads(side_b)
    raw = {f: show(holon, commit, f"{base}/{f}") for f in files}
    tables = {f: json.loads(b) for f, b in raw.items()}
    for f, t in tables.items():
        no_real_features(t, f)
    if "holds_no_real_feature" not in side:
        raise SystemExit("refused: the sidecar does not state that the table holds no real feature")
    shas = {f: hashlib.sha256(b).hexdigest() for f, b in raw.items()}
    if side.get("sha256") and side.get("table") in shas and shas[side["table"]] != side["sha256"]:
        raise SystemExit(f"refused: {side['table']} sha256 differs from the sidecar")
    if False:
        raise SystemExit(f"refused: {side['table']} sha256 differs from the sidecar")
    moved = []
    if ref == "uneven0":
        data = uneven0(tables["transition_table.json"], tables["transition_table_by_region.json"], moved)
    elif ref == "power0":
        data = power(tables["power_table.json"])
    elif ref == "devices0_us":
        data = devices(tables["device_table.json"])
    elif ref == "presence0_us":
        data = presence(tables["presence_table.json"])
    elif ref == "grid0":
        data = grid(tables["grid_table.json"])
    else:
        data = household(tables["household_table.json"], ref[-2:])
    full = side.get("commit") or subprocess.run(["git", "-C", holon, "rev-parse", commit], check=True, capture_output=True, text=True).stdout.strip()
    notice = " ".join(s["notice"] for s in side.get("sources", []) if s.get("notice")) or str(side.get("licences", ""))
    asset = {"ref": side.get("reference", ref), "version": side.get("version"), "holon_commit": full, "notice": notice,
             "class": "MODEL", "dataset": (" + ".join(s["name"] for s in side.get("sources", [])) or "documents gathered by CIRISHolon's scout") + " via CIRISHolon",
             "envelope": side.get("envelope"), "verdict": side.get("verdict"), "limits": side.get("limits"),
             "renormalised": moved, "data": data}
    os.makedirs(OUT, exist_ok=True)
    text = json.dumps(asset, separators=(",", ":"), sort_keys=True) + "\n"
    path = os.path.join(OUT, f"{ref}.json")
    with open(path, "w") as fh:
        fh.write(text)
    idx_path = os.path.join(OUT, "index.json")
    idx = json.load(open(idx_path)) if os.path.exists(idx_path) else {"schema": "cewp-twin-holon-index/v1", "tables": {}}
    idx["tables"][ref] = {
        "reference": asset["ref"], "version": asset["version"], "holon_commit": full,
        "source_files": {f: {"path": f"{base}/{f}", "sha256": shas[f], "bytes": len(raw[f])} for f in files},
        "asset": f"docs/geo/holon/{ref}.json", "asset_sha256": hashlib.sha256(text.encode()).hexdigest(), "asset_bytes": len(text.encode()),
        "serves_registry_rows": side.get("serves_registry_rows", []), "class": "MODEL", "dataset": asset["dataset"],
        "notice": notice, "envelope": side.get("envelope"), "verdict": side.get("verdict"), "ledger": "holon-" + ref.replace("_", "-"),
        "renormalised_vectors": len(moved),
    }
    with open(idx_path, "w") as fh:
        json.dump(idx, fh, indent=1, sort_keys=True)
        fh.write("\n")
    if ref == "uneven0":
        with open(RUST, "w") as fh:
            fh.write(rust_tables())
    print(f"{ref}: {len(text)} bytes -> {path}; {len(moved)} vectors renormalised")


if __name__ == "__main__":
    main()
