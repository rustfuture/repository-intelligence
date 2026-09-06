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
citations = re.findall(r"(?<![\w/.-])([A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*\.[A-Za-z0-9_-]+):(\d+)", output)
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
