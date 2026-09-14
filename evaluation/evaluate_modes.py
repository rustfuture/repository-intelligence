#!/usr/bin/env python3
"""Compare the product's lexical, semantic and RRF hybrid retrieval modes.

The corpus and labels are intentionally small and authored. Results are a
regression signal for this repository, not a claim about general code search.
"""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
QUESTIONS = json.loads((ROOT / "evaluation/questions.json").read_text())


def path_from_result(line: str) -> str:
    first = line.split("\t", 1)[0]
    return re.sub(r":\d+(?:-\d+)?$", "", first)


def run(mode: str, query: str) -> list[str]:
    if mode == "lexical":
        command = ["cargo", "run", "--quiet", "--locked", "--", str(ROOT / "evaluation/corpus"), *query.split()]
    else:
        flag = "--semantic" if mode == "semantic" else "--hybrid"
        command = ["cargo", "run", "--quiet", "--locked", "--", flag, str(ROOT / "evaluation/corpus"), query]
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True)
    return [path_from_result(line) for line in result.stdout.splitlines() if line.strip()]


def metrics(rankings: list[list[str]]) -> dict[str, float]:
    ranks = []
    for question, ranking in zip(QUESTIONS, rankings):
        rank = next((index + 1 for index, path in enumerate(ranking[:5]) if path == question["evidence"]), None)
        if rank is not None:
            ranks.append(rank)
    return {
        "hit_rate": len(ranks) / len(QUESTIONS),
        "recall_at_5": len(ranks) / len(QUESTIONS),
        "mrr": sum(1 / rank for rank in ranks) / len(QUESTIONS),
    }


def main() -> None:
    results = {}
    for mode in ("lexical", "semantic", "hybrid"):
        results[mode] = metrics([run(mode, question["query"]) for question in QUESTIONS])
    print(json.dumps({
        "questions": len(QUESTIONS),
        "corpus": "evaluation/corpus",
        "embedding": "hash-token-v1 (deterministic offline baseline)",
        "fusion": "RRF(k=60)",
        "results": results,
        "interpretation": "authored file-level relevance; not a general quality claim",
    }, indent=2))


if __name__ == "__main__":
    main()
