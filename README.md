# Repository Intelligence

This repository begins the Repository Intelligence project with a deterministic lexical-search baseline. It indexes text files, preserves line references, ignores common generated/binary files, and supports explicit file update/removal operations.

## Quick start

```bash
cargo test --locked
cargo run --locked -- . "bounded worker"
```

The output is a path, line number, and source line. This is a retrieval baseline, not an LLM answer and not a security boundary. Repository content is treated as untrusted data; no instructions found in source files are executed.

## Current acceptance evidence

- line-cited lexical retrieval
- generated `target/` exclusion
- changed-file refresh
- removed-file disappearance

Embedding retrieval, reranking, commit-aware indexing, and generated answers are deliberately not claimed yet. See the portfolio scope report for the evaluation contract.
