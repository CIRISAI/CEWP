#!/usr/bin/env bash
# Fetch the two raw rasters behind docs/geo/pop/ and docs/geo/landcover/ (approved by Eric 2026-11-11:
# "Approve GHSL and land cover, add them to the twin"). Nothing else is fetched.
#
#   ghs_pop  JRC GHSL GHS-POP R2023A, epoch 2020, WGS84 30 arc-second (EPSG:4326), CC BY 4.0,
#            from the JRC open-data host (jeodpp.jrc.ec.europa.eu), one zip of 482,351,880 bytes.
#   lc100    Copernicus Global Land Service Land Cover 100 m, collection 3, epoch 2019 (V3.0.1),
#            discrete classification map, EPSG:4326, CC BY 4.0, from its Zenodo record 3939050
#            (published by the producers VITO/IIASA/WUR), one GeoTIFF of 1,713,345,228 bytes;
#            the md5 Zenodo lists is checked.
#
# Each dataset lands in its OWN new empty directory outside every repo, with a manifest.tsv
# (url, bytes, sha256). The raw files are untrusted data and are never committed; build_geodata.py
# reads them as arrays only. Usage:  tools/geodata/fetch_geodata.sh <raw_root>
set -euo pipefail
ROOT=${1:?usage: fetch_geodata.sh <raw_root>}

# get: one transfer that resumes (-C -) after a stall; a transfer under 10 kB/s for 120 s is a stall
# (the Zenodo transfer stalled once, for an hour, at 1.30 GB of 1.71 GB, on 2026-11-11).
get() { # dir url file maxbytes
  local d=$1 u=$2 f=$3 max=$4 n
  for n in 1 2 3 4 5 6; do
    curl --fail --location --silent --show-error --proto '=https' --max-filesize "$max" \
      --speed-limit 10000 --speed-time 120 -C - -o "$d/$f" "$u" && return 0
    echo "transfer $n of $f stopped at $(stat -c %s "$d/$f" 2>/dev/null) bytes; resuming"
  done
  return 1
}
manifest() { # dir url file
  local d=$1 u=$2 f=$3
  printf 'url\tbytes\tsha256\n%s\t%s\t%s\n' "$u" "$(stat -c %s "$d/$f")" "$(sha256sum "$d/$f" | cut -d' ' -f1)" > "$d/manifest.tsv"
  cat "$d/manifest.tsv"
}
fetch() { # dir url file maxbytes
  local d=$1 u=$2 f=$3 max=$4
  [ -e "$d" ] && { echo "refusing: $d exists (must start empty)"; exit 1; }
  mkdir -p "$d"
  get "$d" "$u" "$f" "$max"
  manifest "$d" "$u" "$f"
}

GHS=https://jeodpp.jrc.ec.europa.eu/ftp/jrc-opendata/GHSL/GHS_POP_GLOBE_R2023A/GHS_POP_E2020_GLOBE_R2023A_4326_30ss/V1-0/GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.zip
fetch "$ROOT/ghs_pop" "$GHS" GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.zip 600000000
[ "$(stat -c %s "$ROOT/ghs_pop/GHS_POP_E2020_GLOBE_R2023A_4326_30ss_V1_0.zip")" = 482351880 ] || { echo "ghs_pop: size differs from the listing"; exit 1; }

LCF=PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif
fetch "$ROOT/lc100" "https://zenodo.org/records/3939050/files/$LCF?download=1" "$LCF" 1800000000
[ "$(md5sum "$ROOT/lc100/$LCF" | cut -d' ' -f1)" = 6898fbe3fb46a1110cd65e3a81ed7624 ] || { echo "lc100: md5 differs from Zenodo's"; exit 1; }
echo "md5 6898fbe3fb46a1110cd65e3a81ed7624 matches Zenodo" >> "$ROOT/lc100/manifest.tsv.note"
du -sh "$ROOT"/*; df -h /tmp | tail -1
