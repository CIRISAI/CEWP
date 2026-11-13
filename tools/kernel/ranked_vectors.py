#!/usr/bin/env python3
"""Vectors for ranked choice from the constitution's own model (formal/ranked_choice/ranked_choice_model.py), read with
`git show` at one commit and run here; nothing is fetched. Writes docs/kernel/ranked_choice_vectors.json: every profile
of up to four ballots over three options (a, b, c, declared in that order) with the model's winner and rounds, plus
per-person cases for the agreement rule. Both kernels (crates/cewp-kernel ranked.rs, docs/kernel/kernel.js) must give
every vector.
Usage: python3 -I tools/kernel/ranked_vectors.py <CIRISConstitution checkout> <commit>
"""
import hashlib, itertools, json, os, subprocess, sys, types

holon, commit = sys.argv[1], sys.argv[2]
src = subprocess.run(["git", "-C", holon, "show", f"{commit}:formal/ranked_choice/ranked_choice_model.py"], check=True, capture_output=True).stdout
full = subprocess.run(["git", "-C", holon, "rev-parse", commit], check=True, capture_output=True, text=True).stdout.strip()
m = types.ModuleType("ranked_choice_model"); exec(compile(src, "ranked_choice_model.py", "exec"), m.__dict__)
options = ["a", "b", "c"]
R = m.rankings(options, 3)
def election(s, options, floor):
    """An ELECTION (CC 4.5.13 at this commit): the model's finish, except that a tie for weakest in any round seats
    nobody (the count runs again over the ballots then held). The model states it does not model this
    (formal/moderator_election/README.md), so the tie test is this script's, on the model's own rounds."""
    w, rounds = m.finish(s, options, floor)
    if w is None: return None
    standing = list(options)
    from collections import Counter
    while True:
        first = Counter()
        for ranking, n in s.items():
            top = next((o for o in ranking if o in standing), None)
            if top is not None: first[top] += n
        live = sum(first.values()); lead = max(standing, key=lambda o: first[o])
        if 2 * first[lead] > live or len(standing) == 1: return lead
        least = min(first[o] for o in standing)
        if sum(1 for o in standing if first[o] == least) > 1: return None
        standing.remove(min(standing, key=lambda o: first[o]))

profiles = []
for size in range(0, 5):
    for profile in itertools.combinations_with_replacement(R, size):
        w, rounds = m.finish(m.summary(profile), options)
        e = election(m.summary(profile), options, m.FLOOR)
        profiles.append(["".join(p) for p in profile] + [e or "", w or "", rounds])  # ballots, then the election's winner ("" none), the general count's winner, rounds
# the agreement rule: (person, ranking) rows; a person whose rows disagree has none
persons = []
for rows in [
    [("p1", ("a",)), ("p1", ("a",)), ("p2", ("b",))],
    [("p1", ("a",)), ("p1", ("b",)), ("p2", ("b",)), ("p3", ("a", "b"))],
    [("p1", ("a", "b")), ("p2", ("b", "a")), ("p2", ("b", "a")), ("p3", ("c",)), ("p3", ("c", "a"))],
    [("p1", ("c",)), ("p2", ("c",)), ("p3", ("a",)), ("p4", ("b",)), ("p4", ("a",))],
]:
    persons.append({"rows": [[p, list(r)] for p, r in rows], "winner": m.winner(rows, options)})
out = {"source": f"CIRISConstitution {full[:7]} formal/ranked_choice/ranked_choice_model.py (finish, winner), run by tools/kernel/ranked_vectors.py",
       "model_sha256": hashlib.sha256(src).hexdigest(), "options_declared": options, "floor": m.FLOOR, "profile_form": "[ballot, ..., election winner or empty, general-count winner or empty, rounds]; a ballot is its options best first", "profiles": profiles, "persons": persons}
path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "docs", "kernel", "ranked_choice_vectors.json")
with open(path, "w") as fh:
    json.dump(out, fh, separators=(",", ":")); fh.write("\n")
print(f"{len(profiles)} profiles, {len(persons)} person cases -> {os.path.normpath(path)}")
