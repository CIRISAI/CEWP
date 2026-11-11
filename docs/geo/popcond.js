// Population conditioning of the generated street tier (MODEL rules over GHS-POP R2023A 2020, an
// external published estimate; FSD/twin/globe.md "Where streets are built").
//
// The tiling's holon tree (holon.js, tiling.js) apportions households from a 1.5-degree root cell
// down to street tiles. Its root and every child's weight are the GHS-POP people inside the holon's
// rectangle: each 1/8-degree cell's people (geodata.js level 1, conserved to the 1/2-degree totals)
// times the share of that cell the rectangle overlaps (by degrees, uniform within a cell). A cell
// under POPCOND.FLOOR_KM2 people per km² counts zero, so empty land (forest, bare ground, snow and
// ice, wetland, water: whatever GHS-POP leaves without people) gets no households, hence no streets
// and no devices. Households = people / POPCOND.PPH, apportioned by the kernel's largest-remainder
// split, so a street tile's households are its rectangle's people / PPH within the split's rounding
// (popcond.test.js checks it: the conditional-consistency check of CIRISHolon#3).
"use strict";
const POPCOND = {
  FLOOR_KM2: 1,     // MODEL: below one person per km² of cell area a 1/8-degree cell holds no generated household
  PPH: 2.5,         // MODEL: persons per household (micro.rs P::PersonsPerHousehold default)
  WEIGHT_SCALE: 16, // ext weights are u32 people x 16, so a child with a few people keeps a non-zero weight
  STEP: 0.125, NX: 2880, NY: 1440,
};
// km² of a 1/8-degree cell in row r (sphere of radius 6371.0088 km, as geodata.js geoCellKm2)
function pcCellKm2(r){ const s = POPCOND.STEP, R = 6371.0088, d = Math.PI / 180, n = 90 - r * s, so = 90 - (r + 1) * s; return R * R * s * d * (Math.sin(n * d) - Math.sin(so * d)); }
// people inside [la0, la1] x [lo0, lo1] (degrees, lo0 < lo1, at most 360 wide) from level-1 people P
// (Float32Array 2880 x 1440, row 0 north); null when P is not loaded
function pcPeople(P, la0, lo0, la1, lo1){
  if (!P) return null;
  const s = POPCOND.STEP, NX = POPCOND.NX, NY = POPCOND.NY;
  const r0 = Math.max(0, Math.floor((90 - la1) / s)), r1 = Math.min(NY - 1, Math.ceil((90 - la0) / s) - 1);
  const c0 = Math.floor((lo0 + 180) / s), c1 = Math.ceil((lo1 + 180) / s) - 1;
  let sum = 0;
  for (let r = r0; r <= r1; r++){
    const top = 90 - r * s, bot = top - s, fy = (Math.min(top, la1) - Math.max(bot, la0)) / s; if (fy <= 0) continue;
    const km2 = pcCellKm2(r);
    for (let c = c0; c <= c1; c++){
      const cc = ((c % NX) + NX) % NX, p = P[r * NX + cc]; if (!(p > 0) || p / km2 < POPCOND.FLOOR_KM2) continue;
      const left = -180 + c * s, fx = (Math.min(left + s, lo1) - Math.max(left, lo0)) / s; if (fx <= 0) continue;
      sum += p * fx * fy;
    }
  }
  return sum;
}
const pcHouseholds = (people) => Math.round(people / POPCOND.PPH);
const pcWeight = (people) => Math.min(4e9, Math.round(people * POPCOND.WEIGHT_SCALE));
// the 1/8-degree cell under a point: { people, perKm2 } (cell area, not land area), or null
function pcCellAt(P, lat, lon){
  if (!P) return null; const s = POPCOND.STEP;
  const r = Math.max(0, Math.min(POPCOND.NY - 1, Math.floor((90 - lat) / s))), c = Math.min(POPCOND.NX - 1, Math.floor((((lon + 180) % 360) + 360) % 360 / s));
  const people = P[r * POPCOND.NX + c]; return { people, perKm2: people / pcCellKm2(r) };
}
if (typeof module !== "undefined") module.exports = { POPCOND, pcCellKm2, pcPeople, pcHouseholds, pcWeight, pcCellAt };
