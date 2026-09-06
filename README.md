# Repository Intelligence

This repository begins the Repository Intelligence project with a deterministic lexical-search baseline. It indexes text files, preserves line references, skips configured build and dependency directories, and supports explicit file update/removal operations.

## Quick start

```bash
cargo test --locked
cargo run --locked -- . "bounded worker"
cargo run --locked -- --serve 127.0.0.1:8080 .
./scripts/validate.sh
cargo run --locked -- --answer . "What does the reload endpoint do?"
```

### CLI Output Modes

1. **Deterministic Lexical Search (Default)**:
   ```bash
   cargo run --locked -- <repo_path> "<query>"
   ```
   Outputs matching lines in the format: `<path>:<line_number> <source_line_text>`. This is a retrieval baseline, not an LLM answer and not a security boundary. Repository content is treated as untrusted data; no instructions found in source files are executed.

2. **Grounded LLM Answer Generation (`--answer`)**:
   ```bash
   cargo run --locked -- --answer <repo_path> "<question>"
   ```
   Retrieves top lexical hits, includes the matching source lines as context in a prompt, and calls the external `agy` CLI to generate a natural language explanation. The AGY CLI is an external prerequisite; no credentials or keys are stored in this repository.

The local HTTP server exposes `GET /health`, `GET /reload`, and `GET /search?q=term+term`. Reload applies the Git diff from the indexed commit to the current HEAD and returns the new commit. Search returns the indexed Git commit plus path, line, score, and source text. It is a local development API; authentication, TLS, rate limiting, and multi-tenant isolation are not implemented.

## Implemented capabilities

- Line-cited lexical retrieval with source snippet extraction (`path:line text`)
- Configured directory and extension filtering: skips known build directories (`.git/`, `target/`, `node_modules/`) and non-text file extensions
- Incremental file update and removal tracking
- Git-aware commit diff synchronization (`Index::sync_git`, `GET /reload`)
- Grounded model query generation (`--answer`) via external AGY CLI

## Retrieval Evaluation & Offline Research

The authored evaluation benchmark is located in `evaluation/questions.json`, with its fixed corpus in `evaluation/corpus/`.

- **Deterministic Lexical Baseline**:
  ```bash
  python3 evaluation/evaluate.py
  ```
  Reproduces the lexical retrieval baseline (`Recall@5 = 1.00`, `MRR = 1.0000`, 20 questions). This small authored corpus serves as a determinism, citation, and plumbing test, not proof of general retrieval quality or LLM answer accuracy.

- **Offline Embedding & Hybrid Retrieval Experiment**:
  ```bash
  python3 evaluation/evaluate_hybrid.py
  ```
  Requires a local Ollama instance serving `nomic-embed-text`. Historically measured on this corpus:
  - Lexical MRR: `1.0000` (Recall@5 = 1.00)
  - Embedding MRR: `0.9167` (Recall@5 = 1.00)
  - Hybrid RRF MRR: `0.8667` (Recall@5 = 1.00)
  
  Hybrid retrieval did not improve ranking on this small corpus and resulted in degraded MRR. It is retained strictly as an offline Python evaluation experiment and is **not** promoted as a product gain.

## Deliberate boundaries & non-claims

- **No In-Binary Hybrid or Vector Search**: Vector embeddings and hybrid search algorithms exist only in the offline Python script `evaluation/evaluate_hybrid.py`. They are **not** integrated into the Rust product binary, library, or HTTP API.
- **No Runtime Citation Verification**: In `--answer` mode, the prompt asks the external model to cite lines. The Rust product binary does **not** parse, validate, or cryptographically prove citations at runtime; citation presence is checked only as an offline test assertion via `./scripts/llm_answer_smoke.sh`.
- **No Deep Binary Detection**: File filtering relies strictly on path names (`.git`, `target`, `node_modules`) and file extension matching. General byte-level content sniffing, null-byte scanning, or MIME detection are not implemented.
- **No Background Watcher**: Background filesystem events are not monitored; synchronization is triggered explicitly via CLI or HTTP `/reload`.
- **No Production Hardening**: Authentication, TLS, rate limiting, and multi-tenant isolation remain out of scope for this local development prototype.

## License

MIT. The evaluation corpus in this repository is authored specifically for this project.

Architecture and limitations are documented in `docs/architecture.md`; the current baseline is summarized in `RELEASE_NOTES.md`. Run `python3 scripts/benchmark.py` for the local fixed-corpus latency measurement.

