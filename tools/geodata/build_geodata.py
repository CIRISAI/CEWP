#!/usr/bin/env python3
"""Build the page's real population and land-cover rasters (docs/geo/gd/) and the GHS-POP drop-in
for globe.js's POP_GRID_B64 (docs/geo/popgrid_ghs.js) from two CC BY 4.0 rasters.

Sources (fetched by tools/geodata/fetch_geodata.sh into directories outside every repo; never
committed; read as arrays only, nothing that ships with them is run):
  - GHS-POP R2023A, epoch 2020, WGS84 30 arc-second (GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif):
    people per cell, float64, NoData -200, 43202 x 21384 cells of 1/120 degree, tiled 256 x 256, LZW.
    Schiavina, Freire, Carioli, MacManus (2023), European Commission JRC, doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE.
  - Copernicus Global Land Service Land Cover 100 m, collection 3, epoch 2019, V3.0.1, discrete
    classification map (PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif):
    uint8 class codes (UN LCCS; 0 = no input data, 200 = open sea), 362880 x 141120 cells of 1/1008
    degree from 180 W, 80 N to 60 S, tiled 256 x 256, DEFLATE.
    Buchhorn et al. (2020), Zenodo, doi:10.5281/zenodo.3939050.

GHS-POP is LZW-compressed and this box has no LZW codec for tifffile, so each 256 x 256 tile is
handed to Pillow's libtiff decoder wrapped in a one-strip 8-bit TIFF built here (LZW works on
bytes, and the file uses predictor 1, so the decoded bytes are the tile's raw little-endian
samples). LC100 is DEFLATE (256 x 256 tiles, predictor 1), decoded with Python's zlib. tifffile
only parses the header and the tile offsets.

Usage:  python3 -E -P tools/geodata/build_geodata.py pop  <ghs_pop.tif> <docs_dir> <globe.js>
        python3 -E -P tools/geodata/build_geodata.py lc   <lc100.tif>   <docs_dir>
  (-E -P instead of -I, because tifffile lives in the user's site-packages; -P keeps the current
  and script directories off the import path.)  <globe.js> is read only, for the old POP_GRID_B64
  (its locality and region partition is inherited; see popgrid below).

Output grid (both rasters): equal-angle, cell-centred, row 0 at the north. Cell (r, c) covers
latitudes 90 - (r+1)s .. 90 - r s and longitudes -180 + c s .. -180 + (c+1) s.
  level 0: s = 1/2 degree, 720 x 360       level 1: s = 1/8 degree, 2880 x 1440
A source pixel goes, whole, to the output cell holding its centre (no splitting); so every person
in the source lands in exactly one cell and the totals are conserved to float64 rounding.

docs/geo/gd/pop0.bin.gz   level 0 population: float32 little-endian people per cell (gzip).
docs/geo/gd/pop1.bin.gz   level 1 population shape: uint8 code per cell (gzip). 0 = under 0.1
    person (or no data); code k in 1..255 stands for 10^(-1 + (k - 0.5) * 8 / 255) people, i.e.
    log-spaced from 0.1 to 1e7 (each code spans x1.075). The loader rescales the 16 level-1
    cells of each level-0 cell so they sum to the level-0 value: population is conserved at
    level 0 exactly (float32) and level 1 carries the within-cell shape to about +-3.7 %.
docs/geo/gd/lc0.bin.gz, lc1.bin.gz   land cover: uint8 per cell (gzip), the Copernicus LC100 class
    code of the cell's majority LAND class: the most frequent code among its 100 m pixels that are
    neither 0 (no input) nor 200 (open sea); ties go to the lower code. A cell with no land pixel
    is 200 if it has any sea pixel, else 0. 255 = outside the product (the LC100 map covers
    about 80 N to 60 S; no Antarctica).
docs/geo/gd/index.json    the grid, levels, files, encodings, legend and totals (written last).
docs/geo/popgrid_ghs.js   POP_GRID_GHS_B64: the same binary layout as globe.js POP_GRID_B64
    (initPopGrid): u16 nCells, u16 nLocs, u8 region per locality, pad to even, then per cell
    i16 lat*100, i16 lon*100, u16 weight, u8 locality, u8 meta = region << 4 | 12
    (cellRad 12/25 = 0.48 degree, as every cell of the old grid). Cells are the 1.5-degree cells
    (centres -179.25 + 1.5 k, -89.25 + 1.5 k, as the old grid's) holding at least POPGRID_MIN
    people; weight = people / POPGRID_UNIT, rounded. Each cell's locality is the locality of the
    nearest old cell (great-circle), so the 181 localities and 9 regions stay as they were (a
    MODEL partition); only where people are and how many is replaced by GHS-POP.
"""
import base64, gzip, io, json, math, multiprocessing, os, re, struct, sys, zlib

import numpy as np
import tifffile
from PIL import Image

S0, S1 = 2, 8                      # cells per degree, levels 0 and 1
NX0, NY0, NX1, NY1 = 360 * S0, 180 * S0, 360 * S1, 180 * S1
POP_LO, POP_DEC = -1.0, 8.0        # level-1 code range: 10^-1 .. 10^7 people
POPGRID_STEP = 1.5
POPGRID_MIN = 20000.0              # people; MODEL threshold, see FSD/twin/geodata.md
POPGRID_UNIT = 1000.0              # people per weight unit


def tile_bytes(raw, nbytes_row, rows, predictor=1):
    """Decode one LZW segment to raw bytes via Pillow's libtiff (predictor 2 only for 8-bit samples)."""
    n = 10
    ifd_off = 8
    data_off = ifd_off + 2 + n * 12 + 4
    ent = [(256, 4, nbytes_row), (257, 4, rows), (258, 3, 8), (259, 3, 5), (262, 3, 1),
           (273, 4, data_off), (277, 3, 1), (278, 4, rows), (279, 4, len(raw)), (317, 3, predictor)]
    b = bytearray(b"II*\x00" + struct.pack("<I", ifd_off) + struct.pack("<H", n))
    for tag, typ, val in ent:
        b += struct.pack("<HHI", tag, typ, 1) + (struct.pack("<HH", val, 0) if typ == 3 else struct.pack("<I", val))
    b += struct.pack("<I", 0) + raw
    im = Image.open(io.BytesIO(bytes(b)))
    im.load()
    out = im.tobytes()
    assert len(out) == nbytes_row * rows, (len(out), nbytes_row, rows)
    return out


class Tiled:
    """Header and tile offsets of a tiled single-band GeoTIFF, LZW or DEFLATE."""

    def __init__(self, path):
        self.f = open(path, "rb")
        t = tifffile.TiffFile(self.f)
        p = t.pages[0]
        self.compression = int(p.compression)
        assert p.is_tiled and self.compression in (5, 8) and p.samplesperpixel == 1, (p.compression, p.is_tiled)
        self.predictor = int(p.predictor)
        assert self.predictor == 1 or (self.predictor == 2 and np.dtype(p.dtype).itemsize == 1), self.predictor
        self.h, self.w = p.shape
        self.dtype = np.dtype(p.dtype).newbyteorder("<")
        self.tw, self.th = p.tilewidth, p.tilelength
        self.off, self.cnt = p.dataoffsets, p.databytecounts
        sx, sy, _ = p.tags["ModelPixelScaleTag"].value
        tp = p.tags["ModelTiepointTag"].value
        assert tp[0] == 0 and tp[1] == 0
        self.x0, self.y0, self.dx, self.dy = tp[3], tp[4], sx, sy
        self.ntx = (self.w + self.tw - 1) // self.tw
        self.nty = (self.h + self.th - 1) // self.th

    def tile(self, ty, tx):
        i = ty * self.ntx + tx
        if self.cnt[i] == 0:
            return None
        self.f.seek(self.off[i])
        raw = self.f.read(self.cnt[i])
        if self.compression == 8:   # DEFLATE: Python's own zlib
            b = zlib.decompress(raw)
            assert len(b) == self.tw * self.th * self.dtype.itemsize
            if self.predictor == 2:
                b = np.cumsum(np.frombuffer(b, np.uint8).reshape(self.th, self.tw), axis=1, dtype=np.uint8).tobytes()
        else:
            b = tile_bytes(raw, self.tw * self.dtype.itemsize, self.th, self.predictor)
        return np.frombuffer(b, dtype=self.dtype).reshape(self.th, self.tw)

    def cells(self, ty, tx, per_deg):
        """Output row and column of each pixel centre of a tile, at per_deg cells per degree."""
        j = tx * self.tw + np.arange(self.tw)
        i = ty * self.th + np.arange(self.th)
        lon = self.x0 + (j + 0.5) * self.dx
        lat = self.y0 - (i + 0.5) * self.dy
        nx, ny = int(round(360 * per_deg)), int(round(180 * per_deg))
        col = np.floor((lon + 180.0) * per_deg).astype(np.int64) % nx
        row = np.clip(np.floor((90.0 - lat) * per_deg).astype(np.int64), 0, ny - 1)
        valid_j = j < self.w
        valid_i = i < self.h
        return row, col, valid_i, valid_j


def accumulate(grid, rows, cols, v):
    """grid[rows[i], cols[j]] += v[i, j], through a bincount over the tile's own small window."""
    r0, c0 = int(rows.min()), int(cols.min())
    nr, nc = int(rows.max()) - r0 + 1, int(cols.max()) - c0 + 1
    if nc > grid.shape[1] // 2:   # the tile straddles the antimeridian: each side on its own
        east = cols >= grid.shape[1] // 2
        accumulate(grid, rows, cols[east], v[:, east])
        accumulate(grid, rows, cols[~east], v[:, ~east])
        return
    loc = ((rows - r0)[:, None] * nc + (cols - c0)[None, :]).ravel()
    grid[r0:r0 + nr, c0:c0 + nc] += np.bincount(loc, weights=v.ravel(), minlength=nr * nc).reshape(nr, nc)


def gz_write(path, arr):
    raw = np.ascontiguousarray(arr).tobytes()
    with open(path, "wb") as f:
        # mtime 0 and no file name: the bytes depend on the data only
        with gzip.GzipFile(filename="", mode="wb", fileobj=f, compresslevel=9, mtime=0) as g:
            g.write(raw)
    return os.path.getsize(path), len(raw)


def write_index(gd, key, entry):
    p = os.path.join(gd, "index.json")
    idx = json.load(open(p)) if os.path.exists(p) else {}
    idx["grid"] = ("equal-angle, cell-centred, row 0 at the north; cell (r, c) covers lat 90-(r+1)s..90-r*s, "
                   "lon -180+c*s..-180+(c+1)*s; level 0 s = 1/2 degree (720 x 360), level 1 s = 1/8 degree (2880 x 1440); "
                   "row-major, gzip")
    idx[key] = entry
    with open(p, "w") as f:
        json.dump(idx, f, indent=1, sort_keys=True)
        f.write("\n")


# ------------------------------------------------------------------ population
def build_pop(src, docs, globe_js):
    T = Tiled(src)
    assert T.dtype == np.dtype("<f8"), T.dtype
    g0 = np.zeros((NY0, NX0))
    g1 = np.zeros((NY1, NX1))
    NXg, NYg = int(360 / POPGRID_STEP), int(180 / POPGRID_STEP)
    gg = np.zeros((NYg, NXg))
    total = 0.0
    nodata = 0
    neg = 0
    for ty in range(T.nty):
        for tx in range(T.ntx):
            a = T.tile(ty, tx)
            if a is None:
                continue
            _, _, vi, vj = T.cells(ty, tx, S1)
            a = a[vi][:, vj]
            nodata += int(np.count_nonzero(a == -200))
            neg += int(np.count_nonzero((a < 0) & (a != -200)))
            v = np.where(a > 0, a, 0.0)
            if not v.any():
                continue
            total += float(v.sum())
            for per, grid in ((S0, g0), (S1, g1), (1 / POPGRID_STEP, gg)):
                row, col, _, _ = T.cells(ty, tx, per)
                accumulate(grid, row[vi], col[vj], v)
        print(f"pop row {ty + 1}/{T.nty} total {total:,.0f}", file=sys.stderr, flush=True)
    g1 = g1.ravel()
    gd = os.path.join(docs, "geo", "gd")
    os.makedirs(gd, exist_ok=True)
    p0 = g0.reshape(NY0, NX0).astype("<f4")
    k = np.zeros(NY1 * NX1, dtype=np.uint8)
    pos = g1 >= 10 ** POP_LO
    code = np.floor((np.log10(np.where(pos, g1, 1.0)) - POP_LO) * 255 / POP_DEC).astype(np.int64) + 1
    k[pos] = np.clip(code[pos], 1, 255)
    sz0 = gz_write(os.path.join(gd, "pop0.bin.gz"), p0)
    sz1 = gz_write(os.path.join(gd, "pop1.bin.gz"), k.reshape(NY1, NX1))
    # what the loader reconstructs at level 1, and its error against the exact level-1 sums
    rep = np.where(k > 0, 10 ** (POP_LO + (k.astype(np.float64) - 0.5) * POP_DEC / 255), 0.0).reshape(NY1, NX1)
    blk = rep.reshape(NY0, 4, NX0, 4).sum(axis=(1, 3))
    scale = np.where(blk > 0, p0.astype(np.float64) / np.where(blk > 0, blk, 1), 0.0)
    rec = rep * np.repeat(np.repeat(scale, 4, axis=0), 4, axis=1)
    lost = float(p0.astype(np.float64)[blk == 0].sum())     # level-0 people whose 16 cells all coded 0
    exact1 = g1.reshape(NY1, NX1)
    big = exact1 >= 1000
    relerr = np.abs(rec[big] - exact1[big]) / exact1[big]
    stats = {
        "source_total_float64": total,
        "level0_total_float32": float(p0.astype(np.float64).sum()),
        "level1_total_exact": float(g1.sum()),
        "level1_reconstructed_total": float(rec.sum()) + lost,
        "level0_people_in_cells_with_all_level1_codes_zero": lost,
        "nodata_pixels": nodata,
        "negative_non_nodata_pixels": neg,
        "level1_rel_error_cells_ge_1000_people": {"median": float(np.median(relerr)), "p99": float(np.quantile(relerr, 0.99)), "max": float(relerr.max())},
        "jrc_published_total_2020_1km_mollweide": 7840952947,
    }
    write_index(gd, "population", {
        "source": "GHS-POP R2023A epoch 2020, 30 arc-second WGS84 (GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif)",
        "citation": "Schiavina, Marcello; Freire, Sergio; Carioli, Alessandra; MacManus, Kytt (2023): GHS-POP R2023A - GHS population grid multitemporal (1975-2030). European Commission, Joint Research Centre (JRC). doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE",
        "licence": "CC BY 4.0 (c) European Union, 1995-2026",
        "levels": [
            {"file": "pop0.bin.gz", "nx": NX0, "ny": NY0, "step_deg": 1 / S0, "encoding": "float32 LE people per cell", "bytes_gz": sz0[0], "bytes_raw": sz0[1]},
            {"file": "pop1.bin.gz", "nx": NX1, "ny": NY1, "step_deg": 1 / S1, "encoding": "uint8 code k: 0 = under 0.1 person; k>=1 -> 10^(-1 + (k-0.5)*8/255) people; rescale each 4x4 block to its level-0 value", "bytes_gz": sz1[0], "bytes_raw": sz1[1]},
        ],
        "totals": stats,
    })
    print(json.dumps(stats, indent=1))
    build_popgrid(gg, total, docs, globe_js)


# ------------------------------------------------------------------ the POP_GRID_B64 drop-in
def old_grid(globe_js):
    s = open(globe_js, encoding="utf-8").read()
    b = base64.b64decode(re.search(r'const POP_GRID_B64="([^"]+)"', s).group(1))
    nC, nL = struct.unpack_from("<HH", b, 0)
    reg = list(b[4:4 + nL])
    off = 4 + nL + ((4 + nL) % 2)
    cells = []
    for _ in range(nC):
        la, lo, w, loc, meta = struct.unpack_from("<hhHBB", b, off)
        off += 8
        cells.append((la / 100, lo / 100, w, loc, meta))
    return nC, nL, reg, cells


def build_popgrid(gg, total, docs, globe_js):
    NYg, NXg = gg.shape
    nC, nL, reg, old = old_grid(globe_js)
    olat = np.radians([c[0] for c in old])
    olon = np.radians([c[1] for c in old])
    oloc = np.array([c[3] for c in old])
    keep = np.argwhere(gg >= POPGRID_MIN)
    cells = []
    for r, c in keep:
        lat = 90 - (r + 0.5) * POPGRID_STEP
        lon = -180 + (c + 0.5) * POPGRID_STEP
        la, lo = math.radians(lat), math.radians(lon)
        d = np.sin((olat - la) / 2) ** 2 + np.cos(la) * np.cos(olat) * np.sin((olon - lo) / 2) ** 2
        k = int(oloc[int(np.argmin(d))])
        w = int(round(gg[r, c] / POPGRID_UNIT))
        assert 1 <= w <= 65535, (lat, lon, gg[r, c])
        cells.append((lat, lon, w, k, gg[r, c]))
    # old grid order: south to north, west to east
    cells.sort(key=lambda t: (t[0], t[1]))
    assert len(cells) < 65536 and nL < 256
    b = bytearray(struct.pack("<HH", len(cells), nL) + bytes(reg))
    if len(b) % 2:
        b += b"\x00"
    for lat, lon, w, k, _ in cells:
        b += struct.pack("<hhHBB", int(round(lat * 100)), int(round(lon * 100)), w, k, (reg[k] << 4) | 12)
    b64 = base64.b64encode(bytes(b)).decode()
    kept = sum(t[4] for t in cells)
    out = os.path.join(docs, "geo", "popgrid_ghs.js")
    with open(out, "w") as f:
        f.write("// GHS-POP R2023A (epoch 2020) on globe.js's POP_GRID_B64 layout; built by tools/geodata/build_geodata.py.\n")
        f.write("// Source: Schiavina, Freire, Carioli, MacManus (2023), GHS-POP R2023A, European Commission JRC,\n")
        f.write("// doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE, CC BY 4.0 (c) European Union; aggregated to 1.5-degree cells\n")
        f.write(f"// of at least {POPGRID_MIN:,.0f} people (weight = people / {POPGRID_UNIT:,.0f}). Localities and regions are inherited\n")
        f.write("// from the old grid (nearest old cell): a MODEL partition, not data. See FSD/twin/geodata.md.\n")
        f.write(f"// {len(cells)} cells, {nL} localities; people in kept cells {kept:,.0f} of {total:,.0f} in the source.\n")
        f.write(f'const POP_GRID_GHS_B64="{b64}";\n')
    compare(old, cells, gg, total, kept, nL)


def compare(old, cells, gg, total, kept, nL):
    NYg, NXg = gg.shape
    ow = np.zeros_like(gg)
    for la, lo, w, _, _ in old:
        r = min(NYg - 1, int(math.floor((90 - la) / POPGRID_STEP)))
        c = int(math.floor((lo + 180) / POPGRID_STEP)) % NXg
        ow[r, c] += w
    nw = np.zeros_like(gg)
    for la, lo, w, _, _ in cells:
        nw[int(round((90 - la) / POPGRID_STEP - 0.5)), int(round((lo + 180) / POPGRID_STEP - 0.5))] = w
    union = (ow > 0) | (nw > 0)
    a, b = ow[union] / ow.sum(), nw[union] / nw.sum()
    pear = float(np.corrcoef(a, b)[0, 1])
    ra = np.argsort(np.argsort(a)).astype(float)
    rb = np.argsort(np.argsort(b)).astype(float)
    spear = float(np.corrcoef(ra, rb)[0, 1])
    both = (ow > 0) & (nw > 0)
    # where the old grid put its weight that GHS-POP does not (as shares of each grid's total)
    only_old = float(ow[(ow > 0) & ~(nw > 0)].sum() / ow.sum())
    only_new = float(nw[(nw > 0) & ~(ow > 0)].sum() / nw.sum())
    tv = 0.5 * float(np.abs(a - b).sum())
    def top(gw, n=10):
        f = gw.ravel()
        o = np.argsort(-f)[:n]
        return [(round(90 - (i // NXg + 0.5) * POPGRID_STEP, 2), round(-180 + (i % NXg + 0.5) * POPGRID_STEP, 2), round(float(f[i] / f.sum()) * 100, 3)) for i in o]
    rep = {
        "old_cells": len(old), "new_cells": len(cells), "localities": nL,
        "old_weight_sum": int(ow.sum()), "new_weight_sum_x1000_people": int(nw.sum()),
        "new_people_in_kept_cells": kept, "source_total": total,
        "cells_in_both": int(both.sum()), "old_only_cells": int(((ow > 0) & ~(nw > 0)).sum()), "new_only_cells": int(((nw > 0) & ~(ow > 0)).sum()),
        "old_share_on_cells_ghs_drops": only_old, "new_share_on_cells_old_lacked": only_new,
        "pearson_shares_on_union": pear, "spearman_shares_on_union": spear,
        "total_variation_distance": tv,
        "top10_old_lat_lon_pct": top(ow), "top10_new_lat_lon_pct": top(nw),
    }
    print(json.dumps(rep, indent=1))
    return rep


# ------------------------------------------------------------------ land cover
LC_CODES = [0, 20, 30, 40, 50, 60, 70, 80, 90, 100, 111, 112, 113, 114, 115, 116, 121, 122, 123, 124, 125, 126, 200]


LC_WORKERS = 4                     # processes; the box runs other builds
_LC_T = {}


def lc_tile_row(job):
    """Class counts of one row of tiles: (first level-1 row offset, uint32 [rows, NX1, K], unknown codes)."""
    src, ty, r_top = job
    T = _LC_T.get(src) or _LC_T.setdefault(src, Tiled(src))
    K = len(LC_CODES)
    lut = np.full(256, 255, dtype=np.uint8)
    for i, c in enumerate(LC_CODES):
        lut[c] = i
    row, _, vi, _ = T.cells(ty, 0, S1)
    rr = row[vi] - r_top
    r0, nrl = int(rr.min()), int(rr.max()) - int(rr.min()) + 1
    slab = np.zeros((nrl, NX1, K), dtype=np.uint32)
    unknown = 0
    any_tile = False
    for tx in range(T.ntx):
        a = T.tile(ty, tx)
        if a is None:
            continue
        any_tile = True
        _, col, vi, vj = T.cells(ty, tx, S1)
        a = a[vi][:, vj]
        ki = lut[a]
        unknown += int(np.count_nonzero(ki == 255))
        ki = np.where(ki == 255, 0, ki)
        cc = col[vj]
        east = cc >= NX1 // 2
        parts = [east, ~east] if (east.any() and (~east).any() and cc.max() - cc.min() > NX1 // 2) else [np.ones_like(east)]
        for m in parts:   # a tile across the antimeridian is counted side by side
            cm, km = cc[m], ki[:, m]
            c0, ncl = int(cm.min()), int(cm.max()) - int(cm.min()) + 1
            loc = (((rr - r0)[:, None] * ncl + (cm - c0)[None, :]).ravel() * K + km.ravel())
            slab[:, c0:c0 + ncl] += np.bincount(loc, minlength=nrl * ncl * K).reshape(nrl, ncl, K).astype(np.uint32)
    return r0, (slab if any_tile else None), unknown


def build_lc(src, docs):
    T = Tiled(src)
    assert T.dtype == np.dtype("u1"), T.dtype
    K = len(LC_CODES)
    lut = np.full(256, 255, dtype=np.uint8)
    for i, c in enumerate(LC_CODES):
        lut[c] = i
    # the product's own rows: counts only where it has data
    lat_top = T.y0
    lat_bot = T.y0 - T.h * T.dy
    r_top = int(math.floor((90 - lat_top) * S1 + 1e-9))
    r_bot = int(math.ceil((90 - lat_bot) * S1 - 1e-9))
    nr = r_bot - r_top
    cnt = np.zeros((nr, NX1, K), dtype=np.uint32)
    unknown = 0
    jobs = [(src, ty, r_top) for ty in range(T.nty)]
    with multiprocessing.get_context("fork").Pool(LC_WORKERS) as pool:
        for n, (r0, slab, unk) in enumerate(pool.imap(lc_tile_row, jobs, chunksize=4)):
            unknown += unk
            if slab is not None:
                cnt[r0:r0 + slab.shape[0]] += slab
            if n % 20 == 0:
                print(f"lc row {n + 1}/{T.nty}", file=sys.stderr, flush=True)
    view = cnt
    gd = os.path.join(docs, "geo", "gd")
    os.makedirs(gd, exist_ok=True)
    codes = np.array(LC_CODES, dtype=np.uint8)
    i_sea, i_nd = LC_CODES.index(200), 0

    def majority(v):
        land = v.copy()
        land[..., i_sea] = 0
        land[..., i_nd] = 0
        best = np.argmax(land, axis=-1)          # first max: the lower code wins a tie
        has_land = land.max(axis=-1) > 0
        out = np.where(has_land, codes[best], np.where(v[..., i_sea] > 0, 200, 0)).astype(np.uint8)
        return out

    full1 = np.full((NY1, NX1), 255, dtype=np.uint8)
    full1[r_top:r_bot] = majority(view)
    # level 0 sums the 4 x 4 level-1 counts of each cell (rows outside the product stay 255)
    full0 = np.full((NY0, NX0), 255, dtype=np.uint8)
    q0, q1 = -(-r_top // 4), r_bot // 4
    v0 = view[q0 * 4 - r_top:q1 * 4 - r_top].reshape(q1 - q0, 4, NX0, 4, K).sum(axis=(1, 3))
    full0[q0:q1] = majority(v0)
    sz0 = gz_write(os.path.join(gd, "lc0.bin.gz"), full0)
    sz1 = gz_write(os.path.join(gd, "lc1.bin.gz"), full1)
    tot = view.reshape(-1, K).sum(axis=0).astype(np.float64)
    land_tot = tot.copy()
    land_tot[i_sea] = 0
    land_tot[i_nd] = 0
    share = {str(c): round(float(land_tot[i] / land_tot.sum()), 5) for i, c in enumerate(LC_CODES) if land_tot[i] > 0}
    stats = {"product_rows_lat": [lat_top, lat_bot], "pixel_deg": [T.dx, T.dy], "origin": [T.x0, T.y0],
             "pixels_counted": int(tot.sum()), "pixels_unknown_code": unknown,
             "land_pixel_share_by_code": share,
             "level1_cells_by_code": {str(int(c)): int(n) for c, n in zip(*np.unique(full1, return_counts=True))}}
    write_index(gd, "landcover", {
        "source": "Copernicus Global Land Service Land Cover 100 m, collection 3, epoch 2019, V3.0.1, discrete classification (PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif)",
        "citation": "Buchhorn, M.; Smets, B.; Bertels, L.; De Roo, B.; Lesiv, M.; Tsendbazar, N.-E.; Herold, M.; Fritz, S. (2020). Copernicus Global Land Service: Land Cover 100m: collection 3: epoch 2019: Globe (Version V3.0.1) [Data set]. Zenodo. doi:10.5281/zenodo.3939050",
        "licence": "CC BY 4.0; Contains modified Copernicus Service information 2019",
        "levels": [
            {"file": "lc0.bin.gz", "nx": NX0, "ny": NY0, "step_deg": 1 / S0, "encoding": "uint8 LC100 code of the majority land class (200 = no land pixel, sea; 0 = no input data; 255 = outside the product)", "bytes_gz": sz0[0], "bytes_raw": sz0[1]},
            {"file": "lc1.bin.gz", "nx": NX1, "ny": NY1, "step_deg": 1 / S1, "encoding": "as level 0", "bytes_gz": sz1[0], "bytes_raw": sz1[1]},
        ],
        "stats": stats,
    })
    print(json.dumps(stats, indent=1))


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "pop":
        build_pop(sys.argv[2], sys.argv[3], sys.argv[4])
    elif len(sys.argv) == 4 and sys.argv[1] == "lc":
        build_lc(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(__doc__)
