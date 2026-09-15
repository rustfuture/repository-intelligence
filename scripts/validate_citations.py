#!/usr/bin/env python3
"""Validate path:line citations emitted by the optional answer CLI."""
import re
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
output = subprocess.run(
    ["cargo", "run", "--quiet", "--locked", "--", "--answer", str(root), "What does the reload endpoint do?"],
    cwd=root, text=True, capture_output=True, check=True,
).stdout
answer = output.lower()
required_concepts = ("git diff", "added", "modified", "deleted", "commit")
missing = [concept for concept in required_concepts if concept not in answer]
if missing:
    raise SystemExit(f"answer is missing required concepts: {', '.join(missing)}")
body = "\n".join(
    line for line in output.splitlines()
    if not any(line.startswith(prefix) for prefix in ("commit=", "model=", "duration_ms=", "cost_usd="))
)
citations = re.findall(r"(?<![\w/.-])([A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*\.[A-Za-z0-9_-]+):(\d+)", body)
if not citations:
    raise SystemExit("no path:line citations found")
for relative, raw_line in citations:
    path = (root / relative).resolve()
    if root not in path.parents or not path.is_file():
        raise SystemExit(f"citation escapes repository or is not a file: {relative}")
    line = int(raw_line)
    if not 1 <= line <= len(path.read_text(errors="replace").splitlines()):
        raise SystemExit(f"citation line out of range: {relative}:{line}")
print(f"validated {len(citations)} citation(s)")
