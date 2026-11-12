#!/bin/sh
# Writes the commit whose code the page runs (git rev-parse HEAD) into docs/source.js (the page's
# AGPL source link reads CEWP_SOURCE_COMMIT) and into docs/sw.js (SW_STAMP names the service
# worker's cache, so each deploy gets a new cache and the old one is deleted). Run it as the last
# step before committing a page candidate; the commit that carries the stamp differs from the
# stamped commit only by these two files. docs/geo/sw.test.js checks the two stamps agree.
set -e
cd "$(dirname "$0")/../.."
sha=$(git rev-parse HEAD)
printf '// Written by tools/page/stamp_source.sh: the commit whose code this page runs (AGPL source link).\nconst CEWP_SOURCE_COMMIT = "%s";\n' "$sha" > docs/source.js
sed -i "s/^const SW_STAMP = \"[0-9a-f]*\";/const SW_STAMP = \"$sha\";/" docs/sw.js
grep -q "^const SW_STAMP = \"$sha\";" docs/sw.js
echo "docs/source.js and docs/sw.js -> $sha"
