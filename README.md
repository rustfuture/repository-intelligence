# Repository Intelligence

This repository begins the Repository Intelligence project with a deterministic lexical-search baseline. It indexes text files, preserves line references, ignores common generated/binary files, and supports explicit file update/removal operations.

## Quick start

```bash
cargo test --locked
cargo run --locked -- . "bounded worker"
cargo run --locked -- --serve 127.0.0.1:8080 .
./scripts/validate.sh
```

The output is a path, line number, and source line. This is a retrieval baseline, not an LLM answer and not a security boundary. Repository content is treated as untrusted data; no instructions found in source files are executed.

The server exposes `GET /health`, `GET /reload`, and `GET /search?q=term+term`. Reload applies the Git diff from the indexed commit to the current HEAD and returns the new commit. Search returns the indexed Git commit plus path, line, score, and source text. It is a local development API; authentication, TLS, rate limiting, and multi-tenant isolation are not implemented.

## Current acceptance evidence

- line-cited lexical retrieval
- generated `target/` exclusion
- changed-file refresh
- removed-file disappearance

Embedding retrieval, reranking, commit-aware indexing, and generated answers are deliberately not claimed yet. See the portfolio scope report for the evaluation contract.

The library’s `Index::sync_git` applies added, modified, and deleted paths between two Git revisions and falls back to a full rebuild when history is unavailable. The HTTP `/reload` endpoint exposes this behavior without restarting the process.

The first authored evaluation set is in `evaluation/questions.json`, with its fixed corpus in `evaluation/corpus/`. Run `python3 evaluation/evaluate.py` to reproduce the current lexical baseline (`Recall@5 = 1.00`, `MRR = 1.0000`, 20 questions). This tiny authored corpus is a plumbing/citation check, not evidence of general retrieval quality or LLM answer quality.
