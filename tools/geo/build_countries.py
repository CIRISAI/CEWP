#!/usr/bin/env python3
"""docs/geo/countries.bin: Natural Earth 1:50m admin-0 countries (public domain, v5.1.2) as a compact
point-in-country mask, for choosing a statistical reference's envelope only (never drawn as a border).

Usage: python3 -I tools/geo/build_countries.py <raw dir holding ne_50m_admin_0_countries.geojson> docs/geo

Each country's rings (outer rings and holes alike; the test is even-odd over all of a country's rings)
are simplified by Douglas-Peucker at 0.02 degree and quantised to 0.01 degree. A country is coded by
ISO 3166-1 numeric (ISO_N3_EH, which carries France 250 and Norway 578 where ISO_N3 has -99); the
entities with no ISO numeric code (Somaliland, Kosovo, Northern Cyprus, Siachen Glacier) are coded 0:
no country envelope.

Format (little-endian): "CC1\\0", u32 rings; per ring: u16 iso, u32 points, i32 lat0, i32 lon0 (0.01
degree units), then (points - 1) steps: i8 dlat, i8 dlon, or the byte -128 followed by i16 dlat, i16 dlon.
A ring left under four points by the simplification keeps its four extreme points, so no code is lost.
"""
import json, os, struct, sys

TOL = 0.05


BORDER_TOL = 0.05  # land borders as coasts: at 1:50m the source's own border line is the limit (FSD/twin/geodata.md)


def dp_w(pts, tols):
    """Douglas-Peucker where each point carries its own tolerance: a point is kept when its distance
    over its tolerance exceeds 1 (land-border points 0.01 degree, coast 0.05)."""
    if len(pts) < 4:
        return pts
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        a, b = stack.pop()
        (ax, ay), (bx, by) = pts[a], pts[b]
        dx, dy = bx - ax, by - ay
        L = (dx * dx + dy * dy) ** 0.5
        best, bi = -1.0, -1
        for i in range(a + 1, b):
            px, py = pts[i]
            d = abs(dy * (px - ax) - dx * (py - ay)) / L if L > 0 else ((px - ax) ** 2 + (py - ay) ** 2) ** 0.5
            d /= tols[i]
            if d > best:
                best, bi = d, i
        if best > 1.0 and bi > 0:
            keep[bi] = True
            stack += [(a, bi), (bi, b)]
    return [p for p, k in zip(pts, keep) if k]


def dp(pts, tol):
    if len(pts) < 4:
        return pts
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        a, b = stack.pop()
        (ax, ay), (bx, by) = pts[a], pts[b]
        dx, dy = bx - ax, by - ay
        L = (dx * dx + dy * dy) ** 0.5
        best, bi = -1.0, -1
        for i in range(a + 1, b):
            px, py = pts[i]
            d = abs(dy * (px - ax) - dx * (py - ay)) / L if L > 0 else ((px - ax) ** 2 + (py - ay) ** 2) ** 0.5
            if d > best:
                best, bi = d, i
        if best > tol and bi > 0:
            keep[bi] = True
            stack += [(a, bi), (bi, b)]
    return [p for p, k in zip(pts, keep) if k]


def main():
    raw, out = sys.argv[1], sys.argv[2]
    feats = json.load(open(os.path.join(raw, "ne_50m_admin_0_countries.geojson"), encoding="utf-8"))["features"]
    # a point on two countries' rings is on a land border: kept finer, so border towns land on their side
    owners = {}
    for f in feats:
        g = f["geometry"]
        polys = g["coordinates"] if g["type"] == "MultiPolygon" else [g["coordinates"]]
        for poly in polys:
            for ring in poly:
                for x, y in ring:
                    owners.setdefault((round(x, 6), round(y, 6)), set()).add(f["properties"].get("ADMIN"))
    border = {k for k, v in owners.items() if len(v) > 1}
    rings = []
    for f in feats:
        p = f["properties"]
        iso = int(p.get("ISO_N3_EH") or "-99")
        iso = iso if iso > 0 else 0
        g = f["geometry"]
        polys = g["coordinates"] if g["type"] == "MultiPolygon" else [g["coordinates"]]
        for poly in polys:
            for ring in poly:
                q = []
                src = [(float(x), float(y)) for x, y in ring]
                simp = dp_w(src, [BORDER_TOL if (round(x, 6), round(y, 6)) in border else TOL for x, y in src])
                if len(simp) < 4:  # a small island or state: keep its four extreme points so its code survives
                    ext = [min(src, key=lambda v: v[1]), max(src, key=lambda v: v[0]), max(src, key=lambda v: v[1]), min(src, key=lambda v: v[0])]
                    simp = ext + ext[:1]
                for lo, la in simp:
                    t = (round(la * 100), round(lo * 100))
                    if not q or q[-1] != t:
                        q.append(t)
                # a step too long for i16 (rings along the antimeridian) is cut into shorter steps on the same edge
                r2 = q[:1]
                for t in q[1:]:
                    (a1, o1) = r2[-1]
                    k = max(1, -(-max(abs(t[0] - a1), abs(t[1] - o1)) // 30000))
                    for j in range(1, k + 1):
                        r2.append((a1 + (t[0] - a1) * j // k, o1 + (t[1] - o1) * j // k))
                if len(r2) >= 3:
                    rings.append((iso, r2))
    b = bytearray(b"CC1\0") + struct.pack("<I", len(rings))
    npts = 0
    for iso, q in rings:
        b += struct.pack("<HIii", iso, len(q), q[0][0], q[0][1])
        for (a1, o1), (a2, o2) in zip(q, q[1:]):
            da, do = a2 - a1, o2 - o1
            if -127 <= da <= 127 and -127 <= do <= 127:
                b += struct.pack("<bb", da, do)
            else:  # escape -128, then the step as two i16
                b += struct.pack("<bhh", -128, da, do)
        npts += len(q)
    with open(os.path.join(out, "countries.bin"), "wb") as fh:
        fh.write(b)
    print(f"countries.bin: {len(b)} bytes, {len(rings)} rings, {npts} points, {len({r[0] for r in rings})} codes")


if __name__ == "__main__":
    main()
