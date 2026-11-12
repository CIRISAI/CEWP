#!/usr/bin/env python3
"""Import one CIRISHolon statistics table into the twin (FSD/twin/holon-intake.md section 1).

Usage: python3 -I tools/holon/import.py <CIRISHolon checkout> <commit> <ref>
       ref: uneven0 | household0_us | household0_de

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
    "uneven0": ("conformance/scene/tables/uneven0", ["transition_table.json", "transition_table_by_region.json"]),
    "household0_us": ("conformance/scene/tables/household0_us", ["household_table.json"]),
    "household0_de": ("conformance/scene/tables/household0_de", ["household_table.json"]),
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


def main():
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
    moved = []
    if ref == "uneven0":
        data = uneven0(tables["transition_table.json"], tables["transition_table_by_region.json"], moved)
    else:
        data = household(tables["household_table.json"], ref[-2:])
    full = side.get("commit") or subprocess.run(["git", "-C", holon, "rev-parse", commit], check=True, capture_output=True, text=True).stdout.strip()
    notice = " ".join(s["notice"] for s in side.get("sources", []) if s.get("notice"))
    asset = {"ref": side.get("reference", ref), "version": side.get("version"), "holon_commit": full, "notice": notice,
             "class": "MODEL", "dataset": " + ".join(s["name"] for s in side.get("sources", [])) + " via CIRISHolon",
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
    print(f"{ref}: {len(text)} bytes -> {path}; {len(moved)} vectors renormalised")


if __name__ == "__main__":
    main()
