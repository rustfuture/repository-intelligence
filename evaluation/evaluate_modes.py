#!/usr/bin/env python3
"""Compare lexical, semantic, and RRF hybrid retrieval modes across splits."""
import json
import os
import re
import subprocess
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
QUESTIONS = json.loads((ROOT / "evaluation/questions.json").read_text())
CORPUS = ROOT / "evaluation/corpus"
RESULTS_FILE = ROOT / "evaluation/results_modes.json"


def path_from_result(line: str) -> str:
    first = line.split("\t", 1)[0]
    return re.sub(r":\d+(?:-\d+)?$", "", first)


def ollama_available() -> bool:
    try:
        url = os.environ.get("RI_EMBEDDING_URL", "http://127.0.0.1:11434")
        req = urllib.request.Request(f"{url.rstrip('/')}/api/tags")
        with urllib.request.urlopen(req, timeout=2) as resp:
            return resp.status == 200
    except Exception:
        return False


def run_query(bin_path: Path, mode: str, query: str, env_vars: dict) -> tuple[list[str], float]:
    env = os.environ.copy()
    env.update(env_vars)
    t0 = time.perf_counter()
    if mode == "lexical":
        cmd = [str(bin_path), str(CORPUS), *query.split()]
    elif mode == "semantic":
        cmd = [str(bin_path), "--semantic", str(CORPUS), query]
    else:
        cmd = [str(bin_path), "--hybrid", str(CORPUS), query]
    res = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, env=env, check=True)
    t1 = time.perf_counter()
    duration_ms = (t1 - t0) * 1000.0
    paths = [path_from_result(line) for line in res.stdout.splitlines() if line.strip()]
    return paths, duration_ms


def compute_metrics(questions: list[dict], rankings: list[list[str]], latencies: list[float]) -> dict:
    ranks_at_1 = 0
    ranks_at_3 = 0
    ranks_at_5 = []
    for q, ranking in zip(questions, rankings):
        evidence = q.get("evidence")
        if not evidence:
            continue
        try:
            rank = ranking[:5].index(evidence) + 1
            ranks_at_5.append(rank)
            if rank == 1:
                ranks_at_1 += 1
            if rank <= 3:
                ranks_at_3 += 1
        except ValueError:
            pass

    n = len(questions)
    latencies_sorted = sorted(latencies)
    p50 = latencies_sorted[int(len(latencies_sorted) * 0.50)] if latencies_sorted else 0.0
    p95 = latencies_sorted[int(len(latencies_sorted) * 0.95)] if latencies_sorted else 0.0

    return {
        "count": n,
        "recall_at_1": round(ranks_at_1 / n, 4) if n else 0.0,
        "recall_at_3": round(ranks_at_3 / n, 4) if n else 0.0,
        "recall_at_5": round(len(ranks_at_5) / n, 4) if n else 0.0,
        "mrr": round(sum(1.0 / r for r in ranks_at_5) / n, 4) if n else 0.0,
        "latency_p50_ms": round(p50, 2),
        "latency_p95_ms": round(p95, 2),
    }


def evaluate_refusal(bin_path: Path, unanswerable: list[dict]) -> dict:
    refusals = 0
    env = os.environ.copy()
    if ollama_available():
        env["USE_OLLAMA"] = "1"
    for q in unanswerable:
        cmd = [str(bin_path), "--answer", str(CORPUS), q["query"]]
        res = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, env=env)
        if "Insufficient repository evidence to answer this question." in res.stdout:
            refusals += 1

    total = len(unanswerable)
    return {
        "trap_count": total,
        "refusal_count": refusals,
        "refusal_accuracy": round(refusals / total, 4) if total else 1.0,
    }


def main() -> None:
    subprocess.run(["cargo", "build", "--quiet", "--locked"], cwd=ROOT, check=True)
    bin_path = ROOT / "target/debug/repository-intelligence"

    answerable = [q for q in QUESTIONS if q.get("answerable", True) and q.get("evidence")]
    unanswerable = [q for q in QUESTIONS if not q.get("answerable", True)]
    dev_answerable = [q for q in answerable if q.get("split") == "development"]
    heldout_answerable = [q for q in answerable if q.get("split") == "heldout"]

    has_ollama = ollama_available()

    modes = [
        ("lexical", "lexical", {"RI_EMBEDDING_PROVIDER": "hash"}),
        ("semantic_hash", "semantic", {"RI_EMBEDDING_PROVIDER": "hash"}),
        ("hybrid_hash", "hybrid", {"RI_EMBEDDING_PROVIDER": "hash"}),
    ]
    if has_ollama:
        modes.extend([
            ("semantic_nomic", "semantic", {"RI_EMBEDDING_PROVIDER": "nomic-embed-text"}),
            ("hybrid_nomic", "hybrid", {"RI_EMBEDDING_PROVIDER": "nomic-embed-text"}),
        ])

    mode_results = {}
    for label, mode_kind, env_vars in modes:
        rankings = []
        latencies = []
        for q in answerable:
            ranked, dur = run_query(bin_path, mode_kind, q["query"], env_vars)
            rankings.append(ranked)
            latencies.append(dur)

        all_metrics = compute_metrics(answerable, rankings, latencies)

        # compute per-split metrics
        dev_indices = [i for i, q in enumerate(answerable) if q.get("split") == "development"]
        heldout_indices = [i for i, q in enumerate(answerable) if q.get("split") == "heldout"]

        dev_metrics = compute_metrics(
            [answerable[i] for i in dev_indices],
            [rankings[i] for i in dev_indices],
            [latencies[i] for i in dev_indices],
        )
        heldout_metrics = compute_metrics(
            [answerable[i] for i in heldout_indices],
            [rankings[i] for i in heldout_indices],
            [latencies[i] for i in heldout_indices],
        )

        mode_results[label] = {
            "all": all_metrics,
            "development": dev_metrics,
            "heldout": heldout_metrics,
        }

    refusal_metrics = evaluate_refusal(bin_path, unanswerable)

    payload = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "corpus": "evaluation/corpus",
        "total_questions": len(QUESTIONS),
        "answerable_count": len(answerable),
        "unanswerable_count": len(unanswerable),
        "ollama_available": has_ollama,
        "refusal_metrics": refusal_metrics,
        "modes": mode_results,
    }

    RESULTS_FILE.write_text(json.dumps(payload, indent=2) + "\n")
    print(json.dumps(payload, indent=2))


if __name__ == "__main__":
    main()
