#!/usr/bin/env python3
"""docs/geo/country_iso.json: country names and alpha-3 codes to ISO 3166-1 numeric codes, from the same Natural Earth 1:50m
admin-0 file docs/geo/countries.bin is built from (public domain, v5.1.2), so a statistics table that names a
country (CIRISHolon GRID-0) finds it in the mask. Names are each feature's NAME, ADMIN and NAME_LONG; the code is
ISO_N3_EH as in build_countries.py. A name that two features carry with different codes is dropped (ambiguous).

Usage: python3 -I tools/geo/country_iso.py <raw dir holding ne_50m_admin_0_countries.geojson> docs/geo
"""
import json, os, sys


def main():
    raw, out = sys.argv[1], sys.argv[2]
    ne = json.load(open(os.path.join(raw, "ne_50m_admin_0_countries.geojson")))
    seen = {}
    for f in ne["features"]:
        p = f["properties"]
        iso = int(p.get("ISO_N3_EH") or -99)
        if iso <= 0:
            continue
        for k in ("NAME", "ADMIN", "NAME_LONG"):
            v = p.get(k)
            if v:
                seen.setdefault(v, set()).add(iso)
    names = {k: next(iter(v)) for k, v in sorted(seen.items()) if len(v) == 1}
    # ISO 3166-1 alpha-3 (ISO_A3_EH, the same feature's code as ISO_N3_EH) -> numeric, for tables that join on alpha-3
    a3 = {}
    for f in ne["features"]:
        p = f["properties"]
        iso, code = int(p.get("ISO_N3_EH") or -99), p.get("ISO_A3_EH")
        if iso > 0 and code and code != "-99":
            a3.setdefault(code, set()).add(iso)
    iso3 = {k: next(iter(v)) for k, v in sorted(a3.items()) if len(v) == 1}
    doc = {"what": "country name and ISO 3166-1 alpha-3 -> ISO 3166-1 numeric (Natural Earth 1:50m admin-0 v5.1.2, NAME / ADMIN / NAME_LONG and ISO_A3_EH; ISO_N3_EH); public domain",
           "command": "python3 -I tools/geo/country_iso.py <raw dir> docs/geo", "names": names, "iso3": iso3}
    with open(os.path.join(out, "country_iso.json"), "w") as fh:
        json.dump(doc, fh, indent=0, sort_keys=True, ensure_ascii=False)
        fh.write("\n")
    print(len(names), "names", len(iso3), "alpha-3 codes")


if __name__ == "__main__":
    main()
