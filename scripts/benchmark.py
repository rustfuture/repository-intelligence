#!/usr/bin/env python3
"""Measure local HTTP query latency on the fixed authored corpus."""
import json
import os
import statistics
import subprocess
import time
import urllib.request
from pathlib import Path

root = Path(__file__).resolve().parents[1]
port = int(os.environ.get("RI_BENCHMARK_PORT", "28185"))
server = subprocess.Popen(
    ["cargo", "run", "--quiet", "--locked", "--", "--serve", f"127.0.0.1:{port}", "evaluation/corpus"],
    cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
try:
    url = f"http://127.0.0.1:{port}/search?q=reload+commit"
    for _ in range(50):
        try:
            urllib.request.urlopen(url, timeout=1).read()
            break
        except OSError:
            time.sleep(0.1)
    samples = []
    for _ in range(200):
        started = time.perf_counter_ns()
        urllib.request.urlopen(url, timeout=2).read()
        samples.append((time.perf_counter_ns() - started) / 1_000_000)
    ordered = sorted(samples)
    print(json.dumps({
        "requests": len(samples),
        "p50_ms": statistics.median(ordered),
        "p95_ms": ordered[int(len(ordered) * 0.95) - 1],
        "max_ms": max(ordered),
        "scope": "localhost, fixed authored corpus, sequential requests",
    }, indent=2))
finally:
    server.terminate()
    server.wait(timeout=5)
