# Release notes

## 0.1.0 — baseline candidate

- Deterministic line-cited lexical search remains the compatibility CLI mode.
- Declaration-aware evidence chunks, deterministic offline `HashEmbedding` semantic retrieval, and RRF (`k=60`) hybrid retrieval are available in the Rust library, CLI, and HTTP `/search` endpoint.
- `Index::save_to`/`load_from` provide the portable `RI_INDEX_V1` snapshot format; `--index` writes source, revision, and recent commit metadata, while `--load-index` queries a saved snapshot.
- Git revision metadata and incremental A/M/D/C/T synchronization are supported; renames are handled as delete plus update, and dirty worktrees use hash-based refresh.
- Local `/health`, `/search`, `/commits`, and `/reload` HTTP endpoints plus `--analytics` and `--commits` CLI modes are available.
- Optional answer path uses `extractive-selection-v1`: the model returns evidence IDs only and the application renders verbatim source text with file/line references; extra prose, invalid IDs, and mixed selections are refused atomically. An exact quotation is not evidence that the source is relevant or true.
- Optional extractive-contract and single-fixture prompt-injection smoke scripts remain separate from normal CI because they require a local model; they check quotation integrity and one injected sentinel, not general injection immunity.
- The stable GitHub Actions job runs the Rust suite and both deterministic retrieval evaluators; a separate job checks the Rust 1.85 MSRV.

Local sequential HTTP benchmark on 2026-09-14 (200 requests against the fixed authored corpus): p50 `0.448 ms`, p95 `0.566 ms`, max `0.675 ms`. These figures are localhost plumbing measurements, not production capacity claims.

The product's deterministic retrieval run on the fixed 20-question corpus reported Recall@5 `1.00` and MRR `1.0000` for lexical, Recall@5 `1.00` and MRR `0.9375` for `HashEmbedding` semantic retrieval, and Recall@5 `1.00` and MRR `1.0000` for hybrid RRF. The authored corpus is a plumbing/regression fixture, not evidence of general retrieval quality.

The separate optional `evaluation/evaluate_hybrid.py` experiment uses local Ollama `nomic-embed-text`. Its current fixture result is lexical MRR `1.0000`, embedding MRR `0.9167`, and hybrid MRR `0.8667` (all Recall@5 `1.00`); this external comparison is not a product-quality claim.

This is not a production service. Authentication, TLS, rate limiting, multi-tenant isolation, immutable commit-snapshot indexing, broad retrieval/answer evaluation, and AGY token/cost accounting remain outside this release candidate.
