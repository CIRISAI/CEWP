#!/bin/sh
# Vendors the constitution's manifests the rule kernel reads into docs/kernel/tables/, byte for byte from one commit
# (git show, nothing fetched), and writes docs/kernel/tables/VERSION with that commit and each file's sha256: the kernel
# (crates/cewp-kernel/src/tables.rs load_all) refuses a file whose sha256 differs from VERSION.
# Usage: sh tools/kernel/vendor.sh <CIRISConstitution checkout> <commit> <label, e.g. rc19>
set -e
src="$1"; commit="$2"; label="$3"
[ -n "$src" ] && [ -n "$commit" ] && [ -n "$label" ] || { echo "usage: vendor.sh <checkout> <commit> <label>" >&2; exit 2; }
cd "$(dirname "$0")/../.."
out=docs/kernel/tables
full=$(git -C "$src" rev-parse "$commit")
date=$(git -C "$src" show -s --format=%cs "$commit")
files="transitions.json governance_policy.json governance_policy_vectors.json mesh_story_trace.json namespace_registry.json namespace_match_vectors.json reactions.json family_model.json"
for f in $files; do git -C "$src" show "$commit:manifests/$f" > "$out/$f"; done
{
  printf 'CIRISConstitution %s %s (%s): %s\n' "$label" "$(echo "$full" | cut -c1-7)" "$date" "$(for f in $files; do printf 'manifests/%s, ' "$f"; done | sed 's/, $//')"
  printf 'commit %s\n\nSHA-256 Hashes:\n' "$full"
  for f in $files; do printf '  %s: %s\n' "$f" "$(sha256sum "$out/$f" | cut -d' ' -f1)"; done
} > "$out/VERSION"
cat "$out/VERSION"
