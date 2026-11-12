// countries.bin: the point-in-country mask that chooses a reference's envelope. Border cases, an
// enclave, islands, a point at sea.
"use strict";
const test = require("node:test"), assert = require("node:assert"), fs = require("node:fs"), path = require("node:path");
const { countriesDecode, countryAt, countryNear } = require("./countries.js");
const b = fs.readFileSync(path.join(__dirname, "countries.bin")), M = countriesDecode(b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength));
test("known points land in the right country", () => {
  const cases = [
    ["Basel", 47.5596, 7.5886, 756], ["Loerrach", 47.6156, 7.6617, 276], ["El Paso (airport; at 1:50m the Rio Grande line puts downtown's centre point in Mexico)", 31.8072, -106.3776, 840], ["Ciudad Juarez", 31.6904, -106.4245, 484],
    ["Kaliningrad", 54.7104, 20.4522, 643], ["Reykjavik (an island)", 64.1466, -21.9426, 352], ["Palma de Mallorca (an island)", 39.5696, 2.6502, 724],
    ["Maseru (an enclave)", -29.3151, 27.4869, 426], ["Johannesburg", -26.2041, 28.0473, 710], ["Berlin", 52.52, 13.405, 276], ["Zurich", 47.3769, 8.5417, 756],
    ["New York", 40.7128, -74.006, 840], ["Honolulu", 21.3069, -157.8583, 840], ["Paris", 48.8566, 2.3522, 250], ["Oslo", 59.9139, 10.7522, 578],
    ["Suva, Fiji (an island)", -18.1416, 178.4419, 242], ["Chukotka, west of 180", 66.0, 179.0, 643]];
  for (const [name, la, lo, iso] of cases) assert.strictEqual(countryNear(M, la, lo), iso, name);
  for (const [name, la, lo] of [["mid-Atlantic", 0, -30], ["North Pacific", 30, -150], ["Southern Ocean", -60, 90]]){ assert.strictEqual(countryAt(M, la, lo), null, name); assert.strictEqual(countryNear(M, la, lo), null, name); }
  // the inland border cases hold for the exact test too, not only for the nearest-edge fallback
  for (const [name, la, lo, iso] of cases.slice(0, 5)) assert.strictEqual(countryAt(M, la, lo), iso, name + " (exact)");
  assert.ok(b.length < 100000, `countries.bin is ${b.length} bytes`);
});
