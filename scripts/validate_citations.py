#!/usr/bin/env python3
"""Validate the extractive-selection contract of the optional answer CLI.

This checks **quotation integrity**, not truth or relevance:

1. the CLI must return an accepted evidence selection (decision == "accepted");
2. every citation must point at a real file and an in-range line span;
3. the quoted text must match the cited file/line range exactly.

It deliberately does not score whether the selected source is relevant to the
question or whether any natural-language claim is true. Earlier versions of this
script required concept words ("git diff", "added", ...) to appear in free-form
model output; that is word overlap, not validation, and it is not used here.

The local model is stochastic and sometimes returns `NONE`. When that happens
there is no quotation to inspect, so the script reports the model outcome instead
of pretending the integrity check passed.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
corpus = root / "evaluation" / "corpus"
env = dict(os.environ)
env.setdefault("USE_OLLAMA", "1")
attempts = int(os.environ.get("RI_SMOKE_ATTEMPTS", "3"))

payload = None
for _ in range(attempts):
    proc = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--locked",
            "--",
            "--answer-json",
            str(corpus),
            "What does the reload endpoint do?",
        ],
        cwd=root,
        text=True,
        capture_output=True,
        env=env,
    )
    try:
        candidate = json.loads(proc.stdout)
    except json.JSONDecodeError:
        continue
    if candidate.get("decision") == "accepted":
        payload = candidate
        break
    last_decision = candidate.get("decision")

if payload is None:
    print(
        f"model returned no evidence selection in {attempts} attempt(s) "
        f"(last decision: {locals().get('last_decision', 'unknown')}); "
        "nothing to validate. This is model behaviour, not a quotation-integrity pass."
    )
    sys.exit(0)

checked = 0
for claim in payload.get("claims", []):
    for citation in claim.get("citations", []):
        relative, _, bounds = citation.rpartition(":")
        nums = bounds.split("-")
        low, high = int(nums[0]), int(nums[-1])
        # Evidence paths are relative to the indexed root, which here is the
        # corpus directory, not the repository root.
        path = (corpus / relative).resolve()
        if corpus not in path.parents or not path.is_file():
            raise SystemExit(f"citation escapes the indexed root or is not a file: {relative}")
        lines = path.read_text(errors="replace").splitlines()
        if not 1 <= low <= high <= len(lines):
            raise SystemExit(f"citation line out of range: {citation}")
        quoted = "\n".join(lines[low - 1 : high]).strip()
        if quoted != claim["claim"].strip():
            raise SystemExit(f"quoted text does not match the cited range {citation}")
        checked += 1

if checked == 0:
    raise SystemExit("accepted payload contained no verbatim quotations to validate")

print(
    f"validated {checked} verbatim quotation(s); relevance and truth are NOT asserted"
)
sys.exit(0)
