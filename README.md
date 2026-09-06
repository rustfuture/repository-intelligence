# Repository Intelligence

This repository begins the Repository Intelligence project with a deterministic lexical-search baseline. It indexes text files, preserves line references, ignores common generated/binary files, and supports explicit file update/removal operations.

## Quick start

```bash
cargo test --locked
cargo run --locked -- . "bounded worker"
cargo run --locked -- --serve 127.0.0.1:8080 .
./scripts/validate.sh
cargo run --locked -- --answer . "What does the reload endpoint do?"
```

The output is a path, line number, and source line. This is a retrieval baseline, not an LLM answer and not a security boundary. Repository content is treated as untrusted data; no instructions found in source files are executed.

The server exposes `GET /health`, `GET /reload`, and `GET /search?q=term+term`. Reload applies the Git diff from the indexed commit to the current HEAD and returns the new commit. Search returns the indexed Git commit plus path, line, score, and source text. It is a local development API; authentication, TLS, rate limiting, and multi-tenant isolation are not implemented.

## Implemented capabilities

- Line-cited lexical retrieval with source snippet extraction
- Automatic exclusion of build/temporary artifacts (`target/`, `.git/`, binaries)
- Changed-file refresh and removed-file disappearance
- Git-aware incremental diff synchronization (`Index::sync_git`, `GET /reload`)
- Grounded LLM answer generation (`--answer`) passing local cited evidence to Gemini via AGY

## Deliberate boundaries & non-claims

- **Embedding & Hybrid Retrieval**: The comparison script `evaluation/evaluate_hybrid.py` is an offline Python evaluation using local Ollama `nomic-embed-text` and Reciprocal Rank Fusion. Vector embeddings and hybrid search are **not** integrated into the Rust product binary, library, or HTTP API.
- **Reranking & Neural Search**: Neural rerankers and cross-encoders are not implemented.
- **Filesystem Watcher**: Background filesystem watching is not implemented; synchronization is triggered explicitly via CLI or HTTP `/reload`.
- **Production Controls**: Authentication, TLS, rate limiting, and multi-tenant isolation remain out of scope for this local development prototype.


An optional real Gemini smoke invocation is documented in `docs/llm-validation.md`; it is deliberately separate from the deterministic baseline.

`--answer` is the optional product path: it retrieves local cited evidence, sends only that evidence to AGY/Gemini, and prints the indexed commit with the model answer. The AGY CLI is an external prerequisite; no credentials are stored in this repository.

Run `./scripts/llm_answer_smoke.sh` to verify a real answer contains a repository `path:line` citation and no unvalidated `file://` URI.

## License

MIT. The evaluation corpus in this repository is authored specifically for this project.

Architecture and limitations are documented in `docs/architecture.md`; the current baseline is summarized in `RELEASE_NOTES.md`. Run `python3 scripts/benchmark.py` for the local fixed-corpus latency measurement.
