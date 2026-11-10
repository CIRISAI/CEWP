#!/usr/bin/env python3
"""Build the page's compact Natural Earth layers (docs/geo/) from the raw GeoJSON.

Natural Earth (naturalearthdata.com) is public domain. The raw GeoJSON is fetched separately
(it is never committed) and read here with the standard json module only; nothing that ships
with the data is run.

Usage:  python3 -I tools/geo/build_ne.py <raw_dir> <docs_geo_out_dir>

Output, one binary format ("CG1") for every file:
  magic "CG1\\0", u32 quantum (units per degree), then sections until EOF:
    u8 kind, varint n_polylines, then per polyline: varint n_points, then n_points pairs of
    zigzag-varint deltas of (round(lat*q), round(lon*q)), the first pair relative to (0, 0).
  kind 1 = fill ring (land outer and island rings, and lakes not already holes in the land; the
           page fills them even-odd in the stencil, so a lake inside land is a hole)
  kind 2 = shoreline (coast and lake shores; the land polygons' cut edges on the antimeridian and
           along the south pole are left out, and a tile's clip edges are never shoreline)
  kind 3 = country border (admin-0 boundary lines on land)
  kind 4 = river (rivers and lake centrelines)

  ne110.js   the 110m layers as base64 in one script (the planet view; replaces the old coast)
  ne50.bin   the 50m layers (continent view, loaded lazily)
  ne10/<la>_<lo>.bin   the 10m layers clipped to 10-degree tiles (region view, loaded per tile);
             <la>, <lo> are the tile's south-west corner; ne10/index.json lists the tiles present
  places.bin populated places: magic "CP1\\0", u32 count, then per place: i32 lat*1e4, i32 lon*1e4,
             u8 scalerank, u32 pop_max, u8 name length, name UTF-8; sorted by scalerank then pop
"""
import json, math, os, struct, sys, base64

def load(raw, name):
    with open(os.path.join(raw, name + ".geojson"), encoding="utf-8") as f:
        return json.load(f)["features"]

def polys(features):
    """Every ring of every polygon, as [(lat, lon)] (GeoJSON is lon, lat)."""
    out = []
    for f in features:
        g = f.get("geometry") or {}
        cs = g.get("coordinates") or []
        if g.get("type") == "Polygon":
            cs = [cs]
        elif g.get("type") != "MultiPolygon":
            continue
        for poly in cs:
            for ring in poly:
                out.append([(float(p[1]), float(p[0])) for p in ring])
    return out

def lines(features):
    out = []
    for f in features:
        g = f.get("geometry") or {}
        cs = g.get("coordinates") or []
        if g.get("type") == "LineString":
            cs = [cs]
        elif g.get("type") != "MultiLineString":
            continue
        for ln in cs:
            out.append([(float(p[1]), float(p[0])) for p in ln])
    return out

def artificial(a, b):
    """An edge the polygon was cut along: the antimeridian, or the south pole's edge."""
    if abs(a[1]) >= 179.999 and abs(b[1]) >= 179.999 and (a[1] > 0) == (b[1] > 0):
        return True
    return a[0] <= -89.99 and b[0] <= -89.99

def shorelines(rings):
    """Each ring's boundary as polylines, broken wherever an edge is artificial."""
    out = []
    for r in rings:
        cur = [r[0]]
        for a, b in zip(r, r[1:]):
            if artificial(a, b):
                if len(cur) > 1:
                    out.append(cur)
                cur = [b]
            else:
                cur.append(b)
        if len(cur) > 1:
            out.append(cur)
    return out

def inside(rings_binned, la, lo):
    n = 0
    for (a1, o1, a2, o2) in rings_binned.get(int(math.floor(lo)), ()):
        if (o1 <= lo) != (o2 <= lo):
            y = a1 + (a2 - a1) * (lo - o1) / (o2 - o1)
            if y > la:
                n += 1
    return n & 1 == 1

def bin_edges(rings):
    b = {}
    for r in rings:
        for (a1, o1), (a2, o2) in zip(r, r[1:]):
            for k in range(int(math.floor(min(o1, o2))), int(math.floor(max(o1, o2))) + 1):
                b.setdefault(k, []).append((a1, o1, a2, o2))
    return b

def ring_point(r):
    """A point strictly inside a simple ring: the midpoint of a short chord across one vertex."""
    n = len(r) - 1
    for i in range(n):
        a, b, c = r[i - 1], r[i], r[i + 1]
        m = ((a[0] + c[0]) / 2, (a[1] + c[1]) / 2)
        p = (b[0] + (m[0] - b[0]) * 0.01, b[1] + (m[1] - b[1]) * 0.01)
        if inside(bin_edges([r]), p[0], p[1]):
            return p
    return r[0]

def fill_rings(land, lakes):
    """Land rings, plus each lake ring that sits on land (a lake already cut out as a hole is skipped)."""
    lb = bin_edges(land)
    out = list(land)
    for r in lakes:
        p = ring_point(r)
        if inside(lb, p[0], p[1]):
            out.append(r)
    return out

# ---- encoding
def varint(n, out):
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            return

def zz(n):
    return (n << 1) ^ (n >> 63)

def section(kind, polylines, q, out):
    kept = []
    for pl in polylines:
        pts = []
        for la, lo in pl:
            p = (round(la * q), round(lo * q))
            if not pts or pts[-1] != p:
                pts.append(p)
        if len(pts) >= 2:
            kept.append(pts)
    if not kept:
        return 0
    out.append(kind)
    varint(len(kept), out)
    for pts in kept:
        varint(len(pts), out)
        pa = po = 0
        for a, o in pts:
            varint(zz(a - pa), out)
            varint(zz(o - po), out)
            pa, po = a, o
    return sum(len(p) for p in kept)

def encode(q, layers):
    out = bytearray(b"CG1\0") + struct.pack("<I", q)
    n = 0
    for kind, pls in layers:
        n += section(kind, pls, q, out)
    return bytes(out), n

# ---- clipping to a tile
def clip_ring(r, y0, x0, y1, x1):
    """Sutherland-Hodgman against the box; even-odd fill of the result equals the ring's fill in the box."""
    pts = r[:-1] if r[0] == r[-1] else r
    for axis, val, keep_ge in ((1, x0, True), (1, x1, False), (0, y0, True), (0, y1, False)):
        if not pts:
            break
        res = []
        for i in range(len(pts)):
            a, b = pts[i - 1], pts[i]
            ia = a[axis] >= val if keep_ge else a[axis] <= val
            ib = b[axis] >= val if keep_ge else b[axis] <= val
            if ib:
                if not ia:
                    res.append(cross(a, b, axis, val))
                res.append(b)
            elif ia:
                res.append(cross(a, b, axis, val))
        pts = res
    if len(pts) < 3:
        return None
    return pts + [pts[0]]

def cross(a, b, axis, val):
    t = (val - a[axis]) / (b[axis] - a[axis])
    p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    p[axis] = val
    return (p[0], p[1])

def clip_line(pl, y0, x0, y1, x1):
    """Liang-Barsky per segment; consecutive kept segments join into one polyline."""
    out, cur = [], []
    for a, b in zip(pl, pl[1:]):
        t0, t1 = 0.0, 1.0
        dy, dx = b[0] - a[0], b[1] - a[1]
        ok = True
        for p, qq in ((-dx, a[1] - x0), (dx, x1 - a[1]), (-dy, a[0] - y0), (dy, y1 - a[0])):
            if p == 0:
                if qq < 0:
                    ok = False
                    break
            else:
                t = qq / p
                if p < 0:
                    if t > t1:
                        ok = False
                        break
                    t0 = max(t0, t)
                else:
                    if t < t0:
                        ok = False
                        break
                    t1 = min(t1, t)
        if not ok:
            if len(cur) > 1:
                out.append(cur)
            cur = []
            continue
        pa = (a[0] + dy * t0, a[1] + dx * t0)
        pb = (a[0] + dy * t1, a[1] + dx * t1)
        if cur and cur[-1] == pa:
            cur.append(pb)
        else:
            if len(cur) > 1:
                out.append(cur)
            cur = [pa, pb]
    if len(cur) > 1:
        out.append(cur)
    return out

def area(r):
    return 0.5 * sum(a[1] * b[0] - b[1] * a[0] for a, b in zip(r, r[1:]))

def bbox(pl):
    las = [p[0] for p in pl]
    los = [p[1] for p in pl]
    return min(las), min(los), max(las), max(los)

def layers_for(raw, res):
    land = polys(load(raw, f"ne_{res}_land"))
    lakes = polys(load(raw, f"ne_{res}_lakes"))
    fill = fill_rings(land, lakes)
    shore = shorelines(land) + shorelines(lakes)
    border = lines(load(raw, f"ne_{res}_admin_0_boundary_lines_land"))
    river = lines(load(raw, f"ne_{res}_rivers_lake_centerlines"))
    return fill, shore, border, river

def main():
    raw, out = sys.argv[1], sys.argv[2]
    os.makedirs(os.path.join(out, "ne10"), exist_ok=True)
    report = []
    # 110m: embedded as a script so the planet view and the land test need no request
    f, s, b, r = layers_for(raw, "110m")
    data, n = encode(100, [(1, f), (2, s), (3, b), (4, r)])
    js = "// Natural Earth 110m (public domain), built by tools/geo/build_ne.py; format in that script\nconst NE110_B64=\"" + base64.b64encode(data).decode() + "\";\n"
    with open(os.path.join(out, "ne110.js"), "w") as fh:
        fh.write(js)
    report.append(("ne110.js", len(js), n))
    # 50m
    f, s, b, r = layers_for(raw, "50m")
    data, n = encode(1000, [(1, f), (2, s), (3, b), (4, r)])
    with open(os.path.join(out, "ne50.bin"), "wb") as fh:
        fh.write(data)
    report.append(("ne50.bin", len(data), n))
    # 10m, per 10-degree tile
    f, s, b, r = layers_for(raw, "10m")
    boxes = {"f": [(bbox(x), x) for x in f], "s": [(bbox(x), x) for x in s], "b": [(bbox(x), x) for x in b], "r": [(bbox(x), x) for x in r]}
    tiles, tot, totn = [], 0, 0
    for la in range(-90, 90, 10):
        for lo in range(-180, 180, 10):
            y0, x0, y1, x1 = la, lo, la + 10, lo + 10
            hit = lambda bb: not (bb[2] < y0 or bb[0] > y1 or bb[3] < x0 or bb[1] > x1)
            fill = [c for c in (clip_ring(x, y0, x0, y1, x1) for bb, x in boxes["f"] if hit(bb)) if c]
            lays = [(1, fill)]
            for kind, key in ((2, "s"), (3, "b"), (4, "r")):
                lays.append((kind, [p for bb, x in boxes[key] if hit(bb) for p in clip_line(x, y0, x0, y1, x1)]))
            lays[0] = (1, [c for c in fill if abs(area(c)) > 1e-10])
            data, n = encode(10000, lays)
            if n == 0:
                continue  # the tile only touches a feature at its edge
            name = f"{la}_{lo}.bin"
            with open(os.path.join(out, "ne10", name), "wb") as fh:
                fh.write(data)
            tiles.append([la, lo])
            tot += len(data)
            totn += n
    with open(os.path.join(out, "ne10", "index.json"), "w") as fh:
        json.dump(tiles, fh, separators=(",", ":"))
    report.append((f"ne10/ ({len(tiles)} tiles)", tot, totn))
    # populated places
    pl = []
    for ft in load(raw, "ne_10m_populated_places_simple"):
        p = ft.get("properties") or {}
        g = (ft.get("geometry") or {}).get("coordinates") or [0, 0]
        name = str(p.get("name") or "").encode("utf-8")[:255]
        if not name:
            continue
        pl.append((int(p.get("scalerank") or 10), -int(p.get("pop_max") or 0), float(g[1]), float(g[0]), name))
    pl.sort()
    out_b = bytearray(b"CP1\0") + struct.pack("<I", len(pl))
    for sr, npop, la, lo, name in pl:
        out_b += struct.pack("<iiBIB", round(la * 1e4), round(lo * 1e4), min(255, sr), max(0, min(2**32 - 1, -npop)), len(name)) + name
    with open(os.path.join(out, "places.bin"), "wb") as fh:
        fh.write(out_b)
    report.append(("places.bin", len(out_b), len(pl)))
    for name, size, n in report:
        print(f"{name}\t{size}\t{n}")

if __name__ == "__main__":
    main()
