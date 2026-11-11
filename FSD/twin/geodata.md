# Twin package `geodata`: real population and land cover at coarse grain

Approved by Eric on 2026-11-11 ("Approve GHSL and land cover, add them to the twin"). Two datasets,
both CC BY 4.0, both fetched once from the publisher's host by `tools/geodata/fetch_geodata.sh`,
reduced by `tools/geodata/build_geodata.py` to page assets under `docs/geo/gd/` and a drop-in
for globe.js's population grid (`docs/geo/popgrid_ghs.js`), read by `docs/geo/geodata.js`.
The raw files are never committed.

## Sources

| | Population | Land cover |
|---|---|---|
| Product | GHS-POP R2023A, epoch 2020, WGS84 30 arc-second | Copernicus Global Land Service Land Cover 100 m (CGLS-LC100), collection 3, epoch 2019, V3.0.1, discrete classification map |
| Publisher, host | European Commission, Joint Research Centre; `jeodpp.jrc.ec.europa.eu/ftp/jrc-opendata/GHSL/` | Copernicus Global Land Service (producers VITO, IIASA, WUR); Zenodo record 3939050 |
| File | `GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.zip` (482,351,880 B), holding `..._V1_0.tif` (382,936,052 B) | `PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif` (1,713,345,228 B, md5 as Zenodo lists) |
| sha256 | zip `8de311f4...89c45`, tif `630f8b25...35e43` (full values in `tools/geodata/geodata_manifest.tsv`) | see `tools/geodata/geodata_manifest.tsv` |
| Content | people per cell, float64, NoData -200; 43,202 x 21,384 cells of 1/120 degree, origin -180.0079 E, 89.0996 N; tiled 256 x 256, LZW | UN-LCCS class code per cell, uint8 (0 = no input data, 200 = open sea); cells of 1/1008 degree; LZW |
| Licence as read | `copyright.txt` beside the file: "Any copyright and/or sui generis right on the dataset is licensed under the Creative Commons Attribution 4.0 International (CC BY 4.0) licence [2]. Reuse is allowed provided appropriate credit is given and any changes are indicated." | Zenodo record licence: Creative Commons Attribution 4.0 International. Product User Manual (CGLOPS1_PUM_LC100-V3 I3.4, §4): "Where the user has adapted or modified the products, the statement should be: Contains modified Copernicus Service information [Year]". CLMS data policy: "full, open and free access" |
| Citation | Schiavina, Marcello; Freire, Sergio; Carioli, Alessandra; MacManus, Kytt (2023): GHS-POP R2023A - GHS population grid multitemporal (1975-2030). European Commission, Joint Research Centre (JRC). doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE. Concept and method: Pesaresi et al. (2024), Int. J. Digital Earth 17(1), doi:10.1080/17538947.2024.2390454 | Buchhorn, M.; Smets, B.; Bertels, L.; De Roo, B.; Lesiv, M.; Tsendbazar, N.-E.; Herold, M.; Fritz, S. (2020). Copernicus Global Land Service: Land Cover 100m: collection 3: epoch 2019: Globe (Version V3.0.1) [Data set]. Zenodo. doi:10.5281/zenodo.3939050 |

Why these two. GHS-POP: the task names it; epoch 2020 is the release's estimate nearest the census
inputs (2025 is carried forward by growth rates), and the 30 arc-second WGS84 file is the coarsest
global file on a lat/lon grid (the 1 km file is Mollweide; the 100 m and 3 arc-second sets were not
taken). Land cover, of the four candidates: ESA WorldCover is CC BY 4.0 but ships only as 10 m
tiles (about 117 GB); MODIS MCD12C1 (0.05 degree, no restrictions) is HDF4 behind an Earthdata
login; ESA CCI Land Cover 300 m comes through the Climate Data Store under the Copernicus licence
with an account. CGLS-LC100 is CC BY 4.0 on an open Zenodo record, one 1.7 GB file, and its legend
(23 UN-LCCS classes) separates urban, cropland, forest types, bare, snow and water. Total raw
download 2.20 GB (cap 3 GB).

## Processing

Both GeoTIFFs are LZW. tifffile on this box has no LZW codec, so `build_geodata.py` reads the
tile offsets with tifffile and hands each compressed 256 x 256 tile to Pillow's libtiff decoder,
wrapped in a one-strip 8-bit TIFF it builds itself (both files use predictor 1, so the decoded
bytes are the tile's raw samples). Nothing shipped with the data is executed.

Output grid (both): equal-angle, cell-centred, row 0 at the north; cell (r, c) covers latitudes
90-(r+1)s .. 90-r*s and longitudes -180+c*s .. -180+(c+1)*s. Level 0: s = 1/2 degree (720 x 360);
level 1: s = 1/8 degree (2880 x 1440, 13.9 km at the equator). Every source pixel goes whole to
the cell holding its centre: no pixel is split, so every person lands in exactly one cell.

Population encoding. `pop0.bin.gz`: float32 people per level-0 cell. `pop1.bin.gz`: uint8 code
per level-1 cell, 0 = under 0.1 person, k >= 1 stands for 10^(-1 + (k-0.5) x 8/255) people
(log-spaced 0.1 to 10^7; one code spans x1.075). The loader rescales each 4 x 4 block of level 1
to its level-0 total, so level 1 carries shape only and never moves a total.

Land-cover encoding. `lc0.bin.gz`, `lc1.bin.gz`: uint8, the LC100 code of the cell's majority LAND
class: the most frequent code among its 100 m pixels that are neither 0 (no input) nor 200 (open
sea); a tie goes to the lower code. A cell with no land pixel is 200 if it has any sea pixel,
else 0. 255 = outside the product. Level 0 votes over the summed counts of its 16 level-1 cells.
A coastal cell therefore carries its land class, not "sea": the globe's land mask (Natural Earth)
decides where land is; this raster only says what the land is.

Nodata rule. Population: GHS-POP NoData (-200) and any negative value count as 0 people (the 2020
30 arc-second file has none of either). Land cover: `geoLandcoverAt` returns null for 0 and 255,
and the page keeps its own ground rule there (Antarctica, and any cell PROBA-V never observed).

### Totals check (population)

| | people |
|---|---|
| JRC published world total, 2020 (GHSL Data Package 2023, Table 20, from the 1 km Mollweide grid) | 7,840,952,947 |
| Sum of the 30 arc-second source, float64 (this build) | 7,840,952,947.01 |
| Level 0 (float32 cells), summed | 7,840,952,952.7 (+5.7: float32 rounding) |
| Level 1 as the loader rebuilds it | 7,840,952,952.7 (equal to level 0 by construction) |

Level 1 against the exact 1/8-degree sums, cells of at least 1,000 people: median relative error
1.6 %, 99th percentile 4.6 %, worst 6.8 % (the code step plus the block rescale). 43.5 people
sit in level-0 cells whose 16 level-1 cells all coded 0; the loader spreads those evenly.

## The POP_GRID_B64 drop-in (`docs/geo/popgrid_ghs.js`)

`POP_GRID_GHS_B64` has the layout `initPopGrid` reads (u16 nCells, u16 nLocs, u8 region per
locality, pad, then 8 bytes per cell: i16 lat x 100, i16 lon x 100, u16 weight, u8 locality,
u8 region << 4 | radius). Cells are the 1.5-degree cells the old grid used (centres -179.25 + 1.5k
east, -89.25 + 1.5k north) holding at least 20,000 people (`POPGRID_MIN`, MODEL: keeps 99.87 % of
people in 4,559 cells; the old grid's smallest cell held 1.1e-5 of its weight, the new floor is
2.6e-6 of the world); weight = people / 1,000,
rounded; radius 12 (0.48 degree) as on every old cell. Each cell's locality is the locality of the
nearest old cell, so the 181 localities and 9 regions and everything keyed on them (seats per
locality, regional rooms) keep their meaning. That partition is MODEL, inherited, not data.

| | old grid | GHS-POP drop-in |
|---|---|---|
| cells | 2,961 | 4,559 |
| source recorded | no | GHS-POP R2023A 2020 |
| people represented | unknown (weights sum to 1,206,230, unit unknown) | 7,830,639,988 in kept cells, of 7,840,952,947 (99.87 %) |
| extent (cell centres) | 38.25 S to 63.75 N | 54.75 S to 69.75 N |
| cells in both / only one | 2,732 in both; 190 old-only (1.5 % of old weight); 1,827 new-only (13.3 % of new weight) | |
| agreement of per-cell shares on the union | Pearson 0.72, Spearman 0.52, total-variation distance 0.37 | |

Top cells, share of the world: old grid: 27.75 N 81.75 E (0.54 %), 30.75 N 120.75 E (0.52 %),
29.25 N 77.25 E (0.50 %), 26.25 N 86.25 E (0.47 %), 35.25 N 114.75 E (0.46 %). GHS-POP: 6.75 S
107.25 E (West Java with Jakarta, 0.72 %), 23.25 N 90.75 E (Dhaka, 0.68 %), 29.25 N 77.25 E (Delhi,
0.64 %), 30.75 N 120.75 E (Shanghai, 0.64 %), 30.75 N 30.75 E (Nile delta with Cairo, 0.63 %).
The old grid has no cell on Java, the Nile delta or the Pearl River delta at the top, and carries
3.8 % of its weight in SE Asia & Oceania against 9.0 % in GHS-POP; it puts 10.5 % in North America
against 6.4 %, and 5.3 % in Russia & C. Asia against 3.2 %. 549 of the new cells (1.8 % of people)
lie more than 500 km from any old cell (Pacific islands, Papua New Guinea, Hawaii); they take the
locality of the nearest old cell all the same.

## What each raster can say, and what it cannot (validity envelope)

- Grain. The finest real number is the 1/8-degree cell (about 14 km by 14 km at the equator,
  less east-west toward the poles). Inside a cell nothing is real: no street, no building, no
  household. The orbit view reads level 0 (about 55 km).
- Epoch. Population is the 2020 estimate (GPW v4.11 census counts carried to 2020 and rescaled to
  UN WPP 2022 country totals and UN WUP 2018 city totals); land cover is 2019. Neither is today.
- Population biases. Disaggregation inside a census unit follows built-up volume, so people sit
  where buildings are; census units are coarse in parts of Africa and Asia; the epoch is modelled
  between censuses. Residential population only: no daytime or seasonal population.
- Land-cover biases. 80.6 +/- 0.4 % overall accuracy at level 1 of the legend, 75.4 % at level 2
  (PUM §5); shrubs and herbaceous wetland are the least accurate classes. A majority vote at
  1/8 degree erases every minority class: a city smaller than half a cell disappears into the
  surrounding cropland or forest. No data south of about 60 S (Antarctica) or where PROBA-V had no
  input.
- Suggested ground colours in `GEO_LC_LEGEND` are display choices (MODEL), not data; the official
  LC100 colours are kept beside them for the legend.

**Real data stops at the coarse tier.** Below 1/8 degree every street, building, household and
device on the page is generated. Generated geometry may follow these rasters' cell values (how many
people, which ground class), never a real road or building, and nothing in this package reads or
ships a per-road or per-building source.

## Attribution (CC BY 4.0 for both; required wherever the layers are shown)

Page footer (`GEO_ATTRIBUTION` in `docs/geo/geodata.js`):

> Population: GHS-POP R2023A (epoch 2020), © European Union 1995-2026, JRC, CC BY 4.0 (Schiavina
> et al. 2023, doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE), aggregated to 1/2° and 1/8°
> cells. Land cover: Copernicus Global Land Service Land Cover 100 m, epoch 2019 (Buchhorn et al.
> 2020, doi:10.5281/zenodo.3939050), CC BY 4.0; contains modified Copernicus Service information
> 2020, reduced to the majority land class per 1/2° and 1/8° cell.

The README carries the same two credits under "Data credits". Neither publisher endorses this
twin; the GHSL "how to cite" page asks that the dataset and the concept paper be cited, not the
website. The CGLS-LC100 manual also asks users to "inform Copernicus about the outcome of the use"
and send publications to its help desk: an open item for Eric, not a licence condition of CC BY.

## Reproduce

    tools/geodata/fetch_geodata.sh <raw_root>          # 2.20 GB, two new empty directories
    mkdir <raw_root>/ghs_pop/x && unzip -j <raw_root>/ghs_pop/*.zip '*_30ss_V1_0.tif' -d <raw_root>/ghs_pop/x
    python3 -E -P tools/geodata/build_geodata.py pop <raw_root>/ghs_pop/x/GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif docs docs/globe.js
    python3 -E -P tools/geodata/build_geodata.py lc  <raw_root>/lc100/PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif docs
    node --test tools/geodata/geodata.test.js

The build is deterministic (gzip with mtime 0 and no name): a rerun gives the same bytes.
`docs/globe.js` is read for the old grid's locality partition; once globe.js carries the drop-in,
pass the commit before the swap (ad4e614) to rebuild the same partition.

## Ledger entries (for `crates/cewp-evidence/src/page_ledger.rs`, applied by the globe package)

`page_ledger.rs` was already changed on twin/globe when this package was written, so the entries
are given here to apply there, once globe.js carries the drop-in and the ground tint reads land
cover. Replace `pop-grid-source` (move it out of the FRONTIER block) and the `shows` of
`pop-grid-cells`; add `land-cover-source`.

```rust
    PageClaim {
        id: "pop-grid-source",
        class: Measured,
        shows: "A land-bounded world population grid: GHS-POP R2023A, epoch 2020, summed to 1.5-degree cells of at least 20,000 people.",
        source: "docs/geo/popgrid_ghs.js: POP_GRID_GHS_B64, built by tools/geodata/build_geodata.py from GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif (Schiavina, Freire, Carioli, MacManus 2023, European Commission JRC, doi:10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE, CC BY 4.0); FSD/twin/geodata.md",
        window: "Epoch 2020: GPW v4.11 census counts carried to 2020 and rescaled to UN WPP 2022 country totals and UN WUP 2018 city totals, disaggregated by built-up volume. World total 7,840,952,947 (the JRC's published figure, matched by the build); the kept cells hold 7,830,639,988 (99.87 %).",
        failure: &[
            "GHS-POP is itself a model of census counts between censuses: residential people only, placed where buildings are, coarse where census units are coarse.",
            "The 181 localities and 9 regions the cells are grouped into are inherited from the old grid (nearest old cell): a MODEL partition, not data.",
            "Real data stops at the 1.5-degree cell: every street, building and device inside it is generated.",
        ],
        reproduce: "python3 -E -P tools/geodata/build_geodata.py pop <GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif> docs docs/globe.js; node --test tools/geodata/geodata.test.js",
        ..BASE
    },
    PageClaim {
        id: "land-cover-source",
        class: Measured,
        shows: "The ground's colour follows real land cover: the majority land class of each 1/8-degree cell (Copernicus Global Land Service Land Cover 100 m, 2019).",
        source: "docs/geo/gd/lc0.bin.gz, lc1.bin.gz and docs/geo/geodata.js geoLandcoverAt, built by tools/geodata/build_geodata.py from PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif (Buchhorn et al. 2020, doi:10.5281/zenodo.3939050, CC BY 4.0; contains modified Copernicus Service information 2020)",
        window: "Epoch 2019 (PROBA-V, 100 m), about 80 N to 60 S. Overall accuracy 80.6 +/- 0.4 % at level 1 of the legend (Product User Manual I3.4).",
        failure: &[
            "A majority vote at 1/8 degree erases minority classes: a town smaller than half a cell disappears into the cropland or forest around it.",
            "Shrubs and herbaceous wetland are the classes the producers map least accurately.",
            "The ground colour per class is a display choice of this twin (MODEL), not part of the data.",
            "No data over Antarctica: the page's own ground rule holds there.",
        ],
        reproduce: "python3 -E -P tools/geodata/build_geodata.py lc <PROBAV_LC100_..._EPSG-4326.tif> docs; node --test tools/geodata/geodata.test.js",
        ..BASE
    },
```

`pop-grid-cells` becomes: `shows: "4,559 populated land cells in 181 localities and 9 regions."`, its
failure line: `"The counts describe the embedded grid (GHS-POP 2020, 1.5-degree cells of at least
20,000 people; pop-grid-source)."`. The other entries that cite `pop-grid-source, FRONTIER`
(the street patch and the night-lights layer) drop that clause. The page's "2,961 density cells"
text (globe.js) becomes 4,559.
