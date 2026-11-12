#!/bin/sh
# Writes docs/source.js with the commit whose code the page runs (git rev-parse HEAD). Run it as the
# last step before committing a page candidate; the commit that carries the stamp differs from the
# stamped commit only by this one file. The page's AGPL source link reads CEWP_SOURCE_COMMIT.
set -e
cd "$(dirname "$0")/../.."
sha=$(git rev-parse HEAD)
printf '// Written by tools/page/stamp_source.sh: the commit whose code this page runs (AGPL source link).\nconst CEWP_SOURCE_COMMIT = "%s";\n' "$sha" > docs/source.js
echo "docs/source.js -> $sha"
