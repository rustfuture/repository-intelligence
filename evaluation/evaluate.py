#!/usr/bin/env python3
"""Dependency-free evaluator for the authored retrieval question set."""
import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    questions = json.loads((root / "evaluation/questions.json").read_text())
    corpus = root / "evaluation" / "corpus"
    answerable = [q for q in questions if q.get("answerable", True) and q.get("evidence")]
    unanswerable = [q for q in questions if not q.get("answerable", True)]

    ranks = []
    for question in answerable:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--locked", "--", str(corpus), *question["query"].split()],
            cwd=root, text=True, capture_output=True, check=True,
        )
        paths = [line.split("\t", 1)[0].rsplit(":", 1)[0] for line in result.stdout.splitlines()]
        rank = next((i + 1 for i, path in enumerate(paths[:5]) if path == question["evidence"]), None)
        if rank is not None:
            ranks.append(rank)

    recall = len(ranks) / len(answerable) if answerable else 0.0
    mrr = sum(1 / rank for rank in ranks) / len(answerable) if answerable else 0.0

    print(json.dumps({
        "questions": len(questions),
        "answerable": len(answerable),
        "unanswerable": len(unanswerable),
        "recall_at_5": round(recall, 4),
        "mrr": round(mrr, 4)
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
