#!/usr/bin/env bash
# Fetch the two public-domain rasters for the globe's terrain and imagery, step (d):
#   NOAA ETOPO 2022, 60 arc-second global surface elevation (netCDF), from NOAA NCEI;
#   Natural Earth shaded relief, 1:10m (SR_HR, 44,367,365 bytes), from naciscdn.org: the host the
#   download links on naturalearthdata.com point to (its own /download/ path answered HTTP 500 to a
#   HEAD request on 2026-11-10).
# Usage: tools/geo/fetch_dem.sh <new empty directory outside every repo>
# The directory must not exist yet. Nothing in it is ever run: the zip is listed and unpacked
# with unzip, and the files are read only by tools/geo/build_dem.py. A manifest records each
# file's URL, bytes and sha256; the raw files are never committed.
set -euo pipefail
RAW=${1:?usage: fetch_dem.sh <new empty directory>}
[ -e "$RAW" ] && { echo "refusing: $RAW exists (must start empty)"; exit 1; }
df -h /tmp | tail -1
mkdir -p "$RAW"; cd "$RAW"
FILES="
https://www.ngdc.noaa.gov/thredds/fileServer/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc
https://naciscdn.org/naturalearth/10m/raster/SR_HR.zip
"
printf 'url\tfinal_url\tbytes\tsha256\n' > manifest.tsv
for u in $FILES; do
  f=$(basename "$u")
  final=$(curl --fail --location --silent --show-error --proto '=https' --proto-redir '=https' \
    --max-filesize 900000000 -o "$f" -w '%{url_effective}' "$u")
  printf '%s\t%s\t%s\t%s\n' "$u" "$final" "$(stat -c %s "$f")" "$(sha256sum "$f" | cut -d' ' -f1)" >> manifest.tsv
done
# list before unpacking; refuse absolute paths or parent references in the archive
unzip -Z1 SR_HR.zip > zip_list.txt
if grep -qE '^/|(^|/)\.\.(/|$)' zip_list.txt; then echo "refusing: unsafe path in SR_HR.zip"; exit 1; fi
mkdir sr && unzip -q -d sr SR_HR.zip
column -t manifest.tsv; cat zip_list.txt; du -sh "$RAW"; df -h /tmp | tail -1
