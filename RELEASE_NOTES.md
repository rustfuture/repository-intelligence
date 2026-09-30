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

Local sequential HTTP benchmark (200 requests against the fixed authored corpus), as committed in `evaluation/v3/benchmark-local-http.json`: p50 `0.857 ms`, p95 `0.920 ms`, max `2.110 ms`. These figures are localhost plumbing measurements, not production capacity claims. An earlier version of these notes gave p50 `0.448 ms` / p95 `0.566 ms` / max `0.675 ms` (2026-09-14, commit `2ecd148`) and, before that, `0.210 / 0.432 / 0.698 ms` (2026-09-06, commit `33d1a16`); no output file for either was committed, so they are not reproducible from the repository and are superseded by the committed artifact.

Retrieval comparison, from the committed `evaluation/results_modes.json` (recorded 2026-09-16T20:01:04Z; corpus `evaluation/corpus`, 30 answerable questions of `evaluation/questions.json`; the 20-question column is the subset labelled `heldout` in that file, and all questions were previously exposed). Recall@5 is `1.00` for every mode on both sets.

| Mode | MRR, all 30 | MRR, 20-question subset |
|---|---:|---:|
| lexical | 1.0000 | 1.0000 |
| `HashEmbedding` semantic | 0.8428 | 0.7892 |
| `HashEmbedding` hybrid (RRF) | 0.9667 | 1.0000 |
| `nomic-embed-text` semantic | 0.9178 | 0.8767 |
| `nomic-embed-text` hybrid (RRF) | 0.9667 | 0.9750 |

Lexical is the best or tied-best mode in both columns; neither hybrid mode beats it. The authored corpus is a plumbing/regression fixture, not evidence of general retrieval quality.

Historical figures that no longer match a committed output: an earlier version of these notes reported `HashEmbedding` semantic MRR `0.9375` on a 20-question corpus (also recorded in `docs/implementation-report-2026-09-14.md`), and, from the optional `evaluation/evaluate_hybrid.py`, `nomic-embed-text` embedding MRR `0.9167` and hybrid MRR `0.8667`. They were measured on the 20-question `evaluation/questions.json` of commit `2ecd148` and earlier, before it was replaced by the 42-question set on 2026-09-16 (commit `277b1e1`); no output file for them is committed, and the current results are in the table above. `evaluation/evaluate_hybrid.py` embeds corpus lines through a local Ollama server and has no committed output.

This is not a production service. Authentication, TLS, rate limiting, multi-tenant isolation, immutable commit-snapshot indexing, broad retrieval/answer evaluation, and AGY token/cost accounting remain outside this release candidate.
