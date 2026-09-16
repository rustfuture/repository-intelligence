#!/usr/bin/env python3
"""Compare lexical, local embedding, and reciprocal-rank-fusion retrieval."""
import json
import math
import subprocess
import urllib.request
from pathlib import Path

root = Path(__file__).resolve().parents[1]
corpus = root / "evaluation/corpus"
questions = json.loads((root / "evaluation/questions.json").read_text())
documents = []
for path in sorted(corpus.iterdir()):
    if path.is_file():
        for number, line in enumerate(path.read_text().splitlines(), 1):
            if line.strip():
                documents.append((path.name, number, line.strip()))


def embed(inputs):
    request = urllib.request.Request(
        "http://127.0.0.1:11434/api/embed",
        data=json.dumps({"model": "nomic-embed-text", "input": inputs}).encode(),
        headers={"Content-Type": "application/json"},
    )
    return json.loads(urllib.request.urlopen(request, timeout=120).read())["embeddings"]


def cosine(a, b):
    denominator = math.sqrt(sum(x*x for x in a)) * math.sqrt(sum(x*x for x in b))
    return sum(x*y for x, y in zip(a, b)) / denominator if denominator else 0.0


def metrics(rankings, answerable):
    ranks = []
    for question, ranking in zip(answerable, rankings):
        rank = next((i + 1 for i, path in enumerate(ranking[:5]) if path == question["evidence"]), None)
        if rank is not None:
            ranks.append(rank)
    n = len(answerable)
    return {"recall_at_5": round(len(ranks) / n, 4) if n else 0.0, "mrr": round(sum(1/rank for rank in ranks) / n, 4) if n else 0.0}


answerable = [q for q in questions if q.get("answerable", True) and q.get("evidence")]
doc_vectors = embed([text for _, _, text in documents])
query_vectors = embed([question["query"] for question in answerable])
lexical_rankings, embedding_rankings, hybrid_rankings = [], [], []
for question, query_vector in zip(answerable, query_vectors):
    output = subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "--", str(corpus), *question["query"].split()],
        cwd=root, text=True, capture_output=True, check=True,
    ).stdout
    lexical = [line.split("\t", 1)[0].rsplit(":", 1)[0] for line in output.splitlines()]
    embedding_docs = sorted(range(len(documents)), key=lambda i: cosine(query_vector, doc_vectors[i]), reverse=True)
    embedding = [documents[i][0] for i in embedding_docs]
    scores = {}
    for ranking in (lexical, embedding):
        for rank, path in enumerate(ranking, 1):
            scores[path] = scores.get(path, 0.0) + 1 / (60 + rank)
    hybrid = [path for path, _ in sorted(scores.items(), key=lambda item: (-item[1], item[0]))]
    lexical_rankings.append(lexical)
    embedding_rankings.append(embedding)
    hybrid_rankings.append(hybrid)

print(json.dumps({
    "questions": len(questions),
    "answerable": len(answerable),
    "model": "nomic-embed-text",
    "lexical": metrics(lexical_rankings, answerable),
    "embedding": metrics(embedding_rankings, answerable),
    "hybrid_rrf": metrics(hybrid_rankings, answerable),
    "scope": "fixed authored corpus; file-level relevance; local Ollama embeddings",
}, indent=2))
