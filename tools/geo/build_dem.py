#!/usr/bin/env python3
"""Build the page's terrain layers (docs/geo/relief/, docs/geo/lakes.bin) from two public-domain rasters.

Sources (fetched by tools/geo/fetch_dem.sh into a directory outside every repo; never committed):
  - NOAA NCEI ETOPO 2022, 60 arc-second surface elevation (netCDF-4), read with h5py;
  - Natural Earth 1:10m shaded relief (SR_HR.tif, 8-bit greyscale), read with tifffile.
Both are public domain (credit NOAA NCEI and Natural Earth). Both grids are 21600 x 10800 cells
of 1/60 degree, cell centres at -179.99167 + k/60 east and 89.99167 - k/60 north. Nothing that
ships with the data is run; the files are only read as arrays.

Usage:  python3 -E -P tools/geo/build_dem.py <raw_dir> <docs_geo_dir>
  (-E -P instead of -I, because h5py and tifffile live in the user's site-packages; -P keeps the
  current and script directories off the import path.)
  <raw_dir> holds ETOPO_2022_v1_60s_N90W180_surface.nc and sr/SR_HR.tif.
  <docs_geo_dir> already holds the Natural Earth layers from build_ne.py (ne50.bin, ne10/),
  which this script reads for the land mask.

Output:
  relief/0/0_0.jpg            the whole globe, 2048 x 1024 (z0)
  relief/1/<r>_<c>.jpg        45-degree tiles, 1024 x 1024 (z1), r = 0..3 from the north, c = 0..7 from 180 W
  relief/2/<r>_<c>.jpg        15-degree tiles, 900 x 900 (z2, the source's own 1/60 degree)
  relief/index.json           {"1": [[r, c], ...], "2": [...]}: the tiles present (any land in them)
    Each pixel: R = shaded relief (Natural Earth; 206 is flat ground), G = elevation code
    round(255 * sqrt(clamp(z, 0, 6500) / 6500)) from ETOPO, B = 0. JPEG, 4:4:4, quality 78.
  lakes.bin                   CG1 (format in build_ne.py): kind 1 fill rings and kind 2 shores of
    the lakes ETOPO shows as flat water surfaces that Natural Earth 10m and 50m both have as land
    (Lake Zurich, Lake Neuchatel and the like); and kind 5, the Natural Earth 10m lake rings of
    lakes that 50m has as land (Lake Garda, Lake Maggiore). The page adds kinds 1 and 2 to the 10m
    and 50m land sets and kind 5 to the 50m set (and so to the land test the tiling reads), where
    even-odd fill makes each a hole in the land.
"""
import json, math, os, sys

import numpy as np
import h5py
import tifffile
from PIL import Image
from scipy import ndimage
from skimage import measure
from shapely.geometry import Polygon

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_ne  # noqa: E402  (this repository's own encoder)

N = 60  # cells per degree
H, W = 180 * N, 360 * N

# ---- lake rule (MODEL: a detection rule for this twin, checked by eye on known lakes)
SEED_DZ_M = 0.25     # a seed cell has at least SEED_NB of its 8 neighbours within this of it
SEED_NB = 2          # (two, so a lake one or two cells wide still seeds along its length)
LEVEL_TOL_M = 1.5    # a lake cell lies within this of its seed's level
MIN_CELLS = 5        # a lake, its pieces within two cells of each other taken together, covers at least
                     # this many cells (about 12-17 km^2 at mid latitudes); a lake narrower than a cell
                     # reads as a chain of pieces, since its cells mix water with the shore
MAX_KM2 = 600.0      # Natural Earth 10m has every lake larger than this; a bigger flat is a pan or a floodplain
SHORE_RISE_M = 8.0   # a lake sits in a basin: the median shore cell (1-2 cells out) is this much higher
SHORE_SHARE = 0.6    # and this share of shore cells is at least 3 m higher
FLAT_EXACT = 3       # ETOPO carries a mapped water surface as one exact value: at least this many cells
                     # of the region hold its most common value exactly (a desert flat varies in the
                     # last digits)
MIN_LEVEL_M = 1.5    # levels within this of sea level are coastal flats, not lakes
MARGIN = 60          # cells of margin around each 10-degree window (lakes wider than 1 degree are in Natural Earth)


def decode_cg1(path):
    """Fill rings (kind 1) of a CG1 file, as lists of (lat, lon)."""
    b = open(path, "rb").read()
    assert b[:3] == b"CG1", path
    q = int.from_bytes(b[4:8], "little")
    i = 8
    rings = []

    def vi():
        nonlocal i
        r, s = 0, 0
        while True:
            c = b[i]
            i += 1
            r |= (c & 0x7F) << s
            s += 7
            if not c & 0x80:
                return r

    def zz(n):
        return (n >> 1) ^ -(n & 1)

    while i < len(b):
        k = b[i]
        i += 1
        n = vi()
        for _ in range(n):
            m = vi()
            a = o = 0
            pts = []
            for _ in range(m):
                a += zz(vi())
                o += zz(vi())
                pts.append((a / q, o / q))
            if k == 1:
                rings.append(pts)
    return rings


def raster_evenodd(rings, mask, r0=0, c0=0):
    """XOR every ring's interior into a boolean mask of cell centres (even-odd fill); the mask's
    cell (0, 0) is global cell (r0, c0)."""
    Hm, Wm = mask.shape
    tog = np.zeros((Hm, Wm + 1), dtype=np.uint8)
    for r in rings:
        a = np.asarray(r, dtype=np.float64)
        if len(a) < 3:
            continue
        if (a[0] != a[-1]).any():
            a = np.vstack([a, a[:1]])
        y = (90.0 - a[:, 0]) * N - 0.5 - r0   # row coordinate of the point
        x = (a[:, 1] + 180.0) * N - 0.5 - c0  # column coordinate
        y0, y1, x0, x1 = y[:-1], y[1:], x[:-1], x[1:]
        lo = np.ceil(np.minimum(y0, y1)).astype(np.int64)
        hi = np.ceil(np.maximum(y0, y1)).astype(np.int64)  # rows lo..hi-1 have centres in [min, max)
        cnt = np.clip(hi, 0, Hm) - np.clip(lo, 0, Hm)
        keep = cnt > 0
        if not keep.any():
            continue
        lo, cnt = np.clip(lo[keep], 0, Hm), cnt[keep]
        y0, y1, x0, x1 = y0[keep], y1[keep], x0[keep], x1[keep]
        rep = np.repeat(np.arange(len(cnt)), cnt)
        rows = lo[rep] + (np.arange(cnt.sum()) - np.repeat(np.cumsum(cnt) - cnt, cnt))
        t = (rows - y0[rep]) / (y1[rep] - y0[rep])
        xs = x0[rep] + t * (x1[rep] - x0[rep])
        cols = np.clip(np.ceil(xs).astype(np.int64), 0, Wm)
        np.bitwise_xor.at(tog, (rows, cols), 1)
    mask ^= np.bitwise_xor.accumulate(tog, axis=1)[:, :Wm].astype(bool)


def land_masks(geo):
    l50 = np.zeros((H, W), dtype=bool)
    raster_evenodd(decode_cg1(os.path.join(geo, "ne50.bin")), l50)
    l10 = np.zeros((H, W), dtype=bool)
    idx = json.load(open(os.path.join(geo, "ne10", "index.json")))
    for la, lo in idx:
        # a tile's rings are clipped to its box: fill them into the box alone
        rings = decode_cg1(os.path.join(geo, "ne10", f"{la}_{lo}.bin"))
        r0, c0 = (90 - (la + 10)) * N, (lo + 180) * N
        box = np.zeros((10 * N, 10 * N), dtype=bool)
        raster_evenodd(rings, box, r0, c0)
        l10[r0:r0 + 10 * N, c0:c0 + 10 * N] |= box
    return l50, l10


def load_rasters(raw):
    with h5py.File(os.path.join(raw, "ETOPO_2022_v1_60s_N90W180_surface.nc"), "r") as f:
        lat = f["lat"][:]
        assert len(lat) == H and lat[0] < lat[-1], "ETOPO rows must run south to north"
        z = np.flipud(f["z"][:].astype(np.float32))  # north-up, like SR_HR
    z[z < -20000] = np.nan
    sr = tifffile.imread(os.path.join(raw, "sr", "SR_HR.tif"))
    assert sr.shape == (H, W) and sr.dtype == np.uint8
    return z, sr


def find_lakes(z, l10, l50):
    """Rings of flat water surfaces on land both Natural Earth sets have as land."""
    rings = []
    cand_land = l10 & l50
    lat_c = 90.0 - (np.arange(H) + 0.5) / N
    lon_c = (np.arange(W) + 0.5) / N - 180.0
    stats = {"lakes": 0, "cells": 0, "edge": 0, "big": 0, "flat_shore": 0, "small": 0, "inexact": 0}
    for wr in range(0, H, 10 * N):
        for wc in range(0, W, 10 * N):
            r0, r1 = max(0, wr - MARGIN), min(H, wr + 10 * N + MARGIN)
            c0, c1 = max(0, wc - MARGIN), min(W, wc + 10 * N + MARGIN)
            zz = z[r0:r1, c0:c1]
            ok = cand_land[r0:r1, c0:c1] & np.isfinite(zz)
            if not ok.any():
                continue
            zf = np.where(np.isfinite(zz), zz, 0.0)
            pad = np.pad(zf, 1, mode="edge")
            close = np.zeros(zf.shape, dtype=np.int8)
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    if dy or dx:
                        nb = pad[1 + dy:1 + dy + zf.shape[0], 1 + dx:1 + dx + zf.shape[1]]
                        close += np.abs(nb - zf) < SEED_DZ_M
            seed = ok & (close >= SEED_NB) & (np.abs(zf) > MIN_LEVEL_M)
            # the ice sheets are smooth, not water
            la = lat_c[r0:r1][:, None]
            lo = lon_c[c0:c1][None, :]
            seed &= ~((la < -60) | ((la > 59) & (lo > -75) & (lo < -10)))
            if not seed.any():
                continue
            lab, n = ndimage.label(seed, structure=np.ones((3, 3)))
            field = np.full(zz.shape, -LEVEL_TOL_M, dtype=np.float64)
            lake = np.zeros(zz.shape, dtype=bool)
            seen = np.zeros(zz.shape, dtype=bool)  # cells of a region already judged
            sizes = ndimage.sum_labels(seed, lab, index=np.arange(1, n + 1))
            objs = ndimage.find_objects(lab)
            for k in (np.argsort(-sizes, kind="stable") + 1):
                sl = objs[k - 1]
                s = lab[sl] == k
                if seen[sl][s].any():
                    continue
                cy = (sl[0].start + sl[0].stop) / 2
                cx = (sl[1].start + sl[1].stop) / 2
                # each lake is made once, by the window whose core holds its seed's centre
                if not (wr <= r0 + cy < wr + 10 * N and wc <= c0 + cx < wc + 10 * N):
                    continue
                level = float(np.median(zf[sl][s]))
                g0, g1 = max(0, sl[0].start - MARGIN), min(zz.shape[0], sl[0].stop + MARGIN)
                h0, h1 = max(0, sl[1].start - MARGIN), min(zz.shape[1], sl[1].stop + MARGIN)
                near = ok[g0:g1, h0:h1] & (np.abs(zf[g0:g1, h0:h1] - level) < LEVEL_TOL_M)
                nl, _ = ndimage.label(near, structure=np.ones((3, 3)))
                hit = np.unique(nl[(lab[g0:g1, h0:h1] == k) & near])
                reg = np.isin(nl, hit[hit > 0])
                seen[g0:g1, h0:h1] |= reg
                if not reg.any():
                    continue
                if reg[0, :].any() or reg[-1, :].any() or reg[:, 0].any() or reg[:, -1].any():
                    stats["edge"] += 1
                    continue
                vals, cnts = np.unique(zf[g0:g1, h0:h1][reg], return_counts=True)
                if cnts.max() < FLAT_EXACT:
                    stats["inexact"] += 1
                    continue
                coslat = math.cos(math.radians(float(lat_c[r0 + int(cy)])))
                km2 = reg.sum() * (111.32 / N) ** 2 * coslat
                if km2 > MAX_KM2:
                    stats["big"] += 1
                    continue
                shore = ndimage.binary_dilation(reg, iterations=2) & ~reg & np.isfinite(zz[g0:g1, h0:h1])
                rise = zf[g0:g1, h0:h1][shore] - level
                if len(rise) == 0 or np.median(rise) < SHORE_RISE_M or np.mean(rise >= 3.0) < SHORE_SHARE:
                    stats["flat_shore"] += 1
                    continue
                lake[g0:g1, h0:h1] |= reg
                f = LEVEL_TOL_M - np.abs(zf[g0:g1, h0:h1] - level)
                fg = field[g0:g1, h0:h1]
                np.maximum(fg, np.where(ndimage.binary_dilation(reg, iterations=1) & ok[g0:g1, h0:h1], f, -LEVEL_TOL_M), out=fg)
                stats["lakes"] += 1
                stats["cells"] += int(reg.sum())
            if not lake.any():
                continue
            grp, ng = ndimage.label(ndimage.binary_dilation(lake, iterations=2), structure=np.ones((3, 3)))
            cnt = ndimage.sum_labels(lake, grp, index=np.arange(1, ng + 1))
            small = np.isin(grp, np.nonzero(cnt < MIN_CELLS)[0] + 1) & lake
            stats["small"] += int(small.sum())
            lake &= ~small
            if not lake.any():
                continue
            # the shore: the level set of the field (a sub-cell line), only next to a found lake
            field[~ndimage.binary_dilation(lake, iterations=1)] = -LEVEL_TOL_M
            padded = np.pad(field, 1, constant_values=-LEVEL_TOL_M)
            for c in measure.find_contours(padded, 0.0):
                if len(c) < 4:
                    continue
                rows = c[:, 0] - 1 + r0
                cols = c[:, 1] - 1 + c0
                pts = [(90.0 - (rr + 0.5) / N, (cc + 0.5) / N - 180.0) for rr, cc in zip(rows, cols)]
                poly = Polygon([(p[1], p[0]) for p in pts])
                if not poly.is_valid or poly.area < 1e-5:
                    continue
                simp = poly.simplify(0.15 / N, preserve_topology=True)
                ring = [(y, x) for x, y in simp.exterior.coords]
                if len(ring) >= 4:
                    rings.append(ring)
    return rings, stats


def ne10_only_lakes(geo, l10, l50):
    """Natural Earth 10m lake rings whose lake 50m has as land (Lake Garda, Lake Maggiore): the
    cells water in 10m and land in 50m that touch no 50m water, and the 10m rings around them."""
    w = l50 & ~l10
    lab, n = ndimage.label(w, structure=np.ones((3, 3)))
    touch = np.unique(lab[ndimage.binary_dilation(~l50, structure=np.ones((3, 3))) & w])
    good = np.ones(n + 1, dtype=bool)
    good[0] = False
    good[touch] = False
    out = []
    idx = json.load(open(os.path.join(geo, "ne10", "index.json")))
    for la, lo in idx:
        for r in decode_cg1(os.path.join(geo, "ne10", f"{la}_{lo}.bin")):
            if len(r) < 4:
                continue
            poly = Polygon([(p[1], p[0]) for p in r])
            if not poly.is_valid:
                poly = poly.buffer(0)
            if poly.is_empty or poly.area > 3.0:
                continue
            pt = poly.representative_point()
            rr, cc = int((90.0 - pt.y) * N), int((pt.x + 180.0) * N)
            if 0 <= rr < H and 0 <= cc < W and good[lab[rr, cc]]:
                out.append(r)
    return out


def elev_code(z):
    e = np.where(np.isfinite(z), np.clip(z, 0, 6500), 0)
    return np.round(255 * np.sqrt(e / 6500)).astype(np.uint8)


def save_jpg(rgb, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    Image.fromarray(rgb, "RGB").save(path, "JPEG", quality=78, subsampling=0, optimize=True)
    return os.path.getsize(path)


def relief_tiles(z, sr, land, out):
    total = 0
    idx = {"1": [], "2": []}
    ec = elev_code(z)
    def rgb(r0, r1, c0, c1, size):
        a = sr[r0:r1, c0:c1]
        b = ec[r0:r1, c0:c1]
        if a.shape[0] != size:
            a = np.asarray(Image.fromarray(a).resize((size * (c1 - c0) // (r1 - r0), size), Image.BOX))
            b = np.asarray(Image.fromarray(b).resize((size * (c1 - c0) // (r1 - r0), size), Image.BOX))
        return np.dstack([a, b, np.zeros_like(a)])
    total += save_jpg(rgb(0, H, 0, W, 1024), os.path.join(out, "0", "0_0.jpg"))
    for z_, deg, px in (("1", 45, 1024), ("2", 15, 900)):
        s = deg * N
        for r in range(H // s):
            for c in range(W // s):
                if not land[r * s:(r + 1) * s, c * s:(c + 1) * s].any():
                    continue
                total += save_jpg(rgb(r * s, (r + 1) * s, c * s, (c + 1) * s, px), os.path.join(out, z_, f"{r}_{c}.jpg"))
                idx[z_].append([r, c])
    with open(os.path.join(out, "index.json"), "w") as f:
        json.dump(idx, f, separators=(",", ":"))
    return total, idx


def main():
    raw, geo = sys.argv[1], sys.argv[2]
    l50, l10 = land_masks(geo)
    print("land cells: 50m", int(l50.sum()), "10m", int(l10.sum()), flush=True)
    z, sr = load_rasters(raw)
    rings, st = find_lakes(z, l10, l50)
    only10 = ne10_only_lakes(geo, l10, l50)
    data, n = build_ne.encode(10000, [(1, rings), (2, rings), (5, only10)])
    with open(os.path.join(geo, "lakes.bin"), "wb") as f:
        f.write(data)
    print("lakes.bin", len(data), "bytes;", len(rings), "ETOPO rings,", len(only10), "10m-only rings,", n, "points;", st, flush=True)
    total, idx = relief_tiles(z, sr, l50 | l10, os.path.join(geo, "relief"))
    print("relief", total, "bytes;", "z1", len(idx["1"]), "tiles, z2", len(idx["2"]), "tiles", flush=True)


if __name__ == "__main__":
    main()
