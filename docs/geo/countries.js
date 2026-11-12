// Which country a point is in, from docs/geo/countries.bin (Natural Earth 1:50m admin-0, public domain,
// simplified; tools/geo/build_countries.py). Used ONLY to choose a statistical reference's envelope
// (CIRISHolon tables name a country); never drawn as a claim about borders. Natural Earth draws de facto
// boundaries and states its own caveat on disputed areas; so does this mask.
// countriesDecode(arrayBuffer) -> mask; countryAt(mask, lat, lon) -> ISO 3166-1 numeric, or 0 for an
// entity with no ISO code, or null at sea / outside every country.
"use strict";
function countriesDecode(buf){
  const dv = new DataView(buf), u8 = new Uint8Array(buf);
  if (u8[0] !== 67 || u8[1] !== 67 || u8[2] !== 49) throw new Error("not a CC1 file");
  const n = dv.getUint32(4, true), rings = []; let i = 8;
  for (let k = 0; k < n; k++){
    const iso = dv.getUint16(i, true), np = dv.getUint32(i + 2, true); let la = dv.getInt32(i + 6, true), lo = dv.getInt32(i + 10, true); i += 14;
    const P = new Int32Array(2 * np); P[0] = la; P[1] = lo; let a0 = la, a1 = la, o0 = lo, o1 = lo;
    for (let p = 1; p < np; p++){
      const b = dv.getInt8(i);
      if (b === -128){ la += dv.getInt16(i + 1, true); lo += dv.getInt16(i + 3, true); i += 5; } else { la += b; lo += dv.getInt8(i + 1); i += 2; }
      P[2 * p] = la; P[2 * p + 1] = lo; if (la < a0) a0 = la; if (la > a1) a1 = la; if (lo < o0) o0 = lo; if (lo > o1) o1 = lo;
    }
    rings.push({ iso, P, box: [a0, o0, a1, o1] });
  }
  return { rings };
}
function countryAt(mask, lat, lon){
  const y = Math.round(lat * 100), x = Math.round(((lon + 540) % 360 - 180) * 100), odd = new Map();
  for (const r of mask.rings){
    const b = r.box; if (y < b[0] || y > b[2] || x < b[1] || x > b[3]) continue;
    const P = r.P, n = P.length / 2; let inside = false;
    for (let a = 0, c = n - 1; a < n; c = a++){
      const ya = P[2 * a], xa = P[2 * a + 1], yc = P[2 * c], xc = P[2 * c + 1];
      if ((ya > y) !== (yc > y)){ const num = (xc - xa) * (y - ya), den = yc - ya, lhs = (x - xa) * den; if (den > 0 ? lhs < num : lhs > num) inside = !inside; } // exact integers, as countries.rs
    }
    if (inside) odd.set(r.iso, !odd.get(r.iso));
  }
  for (const [iso, v] of odd) if (v) return iso;
  return null;
}
// countryAt, else the country whose edge is nearest within maxDeg (default 0.15 degree, about 15 km): the mask is
// simplified at 0.05 degree, so a coastal city's centre can fall just offshore; open sea stays null
function countryNear(mask, lat, lon, maxDeg){
  const hit = countryAt(mask, lat, lon); if (hit !== null) return hit;
  const lim = Math.round((maxDeg || 0.15) * 100), y = Math.round(lat * 100), x = Math.round(((lon + 540) % 360 - 180) * 100), cl = Math.max(0.05, Math.cos(lat * Math.PI / 180));
  let best = null, bd = lim * lim;
  for (const r of mask.rings){
    const b = r.box; if (y < b[0] - lim || y > b[2] + lim || x < b[1] - lim / cl || x > b[3] + lim / cl) continue;
    const P = r.P, n = P.length / 2;
    for (let a = 0, c = n - 1; a < n; c = a++){
      const ay = P[2 * a], ax = P[2 * a + 1] * cl, cy = P[2 * c], cx = P[2 * c + 1] * cl, px = x * cl, dx = cx - ax, dy = cy - ay, L = dx * dx + dy * dy;
      const t = L > 0 ? Math.max(0, Math.min(1, ((px - ax) * dx + (y - ay) * dy) / L)) : 0, ex = ax + t * dx - px, ey = ay + t * dy - y, d = ex * ex + ey * ey;
      if (d < bd){ bd = d; best = r.iso; }
    }
  }
  return best;
}
if (typeof module !== "undefined") module.exports = { countriesDecode, countryAt, countryNear };
