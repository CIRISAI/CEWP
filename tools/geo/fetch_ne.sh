#!/usr/bin/env bash
# Fetch the Natural Earth vector layers (public domain) for the CEWP crisp zoom, step (c).
# Source: the Natural Earth GitHub mirror (nvkelso/natural-earth-vector), GeoJSON, v5.1.2 tag.
# Raw files land in a NEW empty directory outside every repo; a manifest records URL, bytes, sha256.
set -euo pipefail
RAW=${1:?usage: fetch_ne.sh <new empty dir outside the repo>}
[ -e "$RAW" ] && { echo "refusing: $RAW exists (must start empty)"; exit 1; }
mkdir -p "$RAW"; cd "$RAW"
BASE=https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson
# NE_FILES overrides the list (one more layer of the same dataset, e.g. NE_FILES=ne_50m_admin_0_countries)
FILES=${NE_FILES:-"
ne_110m_land ne_110m_admin_0_boundary_lines_land ne_110m_lakes ne_110m_rivers_lake_centerlines
ne_50m_land ne_50m_admin_0_boundary_lines_land ne_50m_lakes ne_50m_rivers_lake_centerlines
ne_10m_land ne_10m_admin_0_boundary_lines_land ne_10m_lakes ne_10m_rivers_lake_centerlines ne_10m_populated_places_simple
"}
printf 'url\tbytes\tsha256\n' > manifest.tsv
for f in $FILES; do
  u="$BASE/$f.geojson"
  curl --fail --location --silent --show-error --proto '=https' --max-filesize 200000000 -o "$f.geojson" "$u"
  printf '%s\t%s\t%s\n' "$u" "$(stat -c %s "$f.geojson")" "$(sha256sum "$f.geojson" | cut -d' ' -f1)" >> manifest.tsv
done
column -t manifest.tsv; du -sh "$RAW"; df -h /tmp | tail -1
