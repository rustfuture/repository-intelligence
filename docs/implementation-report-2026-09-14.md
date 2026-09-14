# Implementation report — 2026-09-14

## 1. Starting state

The repository started as a small Rust 2021 binary/library with no third-party dependencies. It indexed eligible text files in memory, stored an inverted token map, returned line-level lexical hits, and exposed a single-threaded local HTTP server. Git reload understood ordinary add/modify/delete paths. The optional `--answer` command sent five lexical lines to AGY or Ollama. Embedding and hybrid retrieval existed only in an external Python/Ollama experiment, and no durable index or code-aware chunk model existed.

## 2. Main problems found

- The product README described an offline RAG experiment as separate from the Rust product, so the advertised pipeline was incomplete.
- Re-indexing had no file hashes and could not refresh a dirty worktree reliably.
- Rename/copy handling and quoted Git paths were incomplete.
- Retrieval evidence was a single line, with no symbol or source span metadata.
- There was no reusable index snapshot, commit-subject search, or unanswerable guard.
- Citation filtering accepted incomplete ranges and could admit duplicate or sensitive records from a hand-crafted index.
- CI measured only the lexical evaluator.

## 3. Architecture changes

The Rust library now has a safe scanner, content-hash tracked files, lightweight declaration-aware chunks, a public `EmbeddingProvider` seam, deterministic `HashEmbedding` vectors, cosine semantic retrieval, and documented reciprocal-rank fusion (`RRF(k=60)`) for hybrid retrieval. `sync_worktree` handles changed/new/deleted files; `sync_git` handles add/modify/delete/copy/type-change/rename records and dirty worktrees. Up to 100 commit IDs/dates/subjects are retained for lightweight commit-aware search.

Evidence objects contain path, start/end line, source kind, optional symbol, text, retrieval source, and score. Grounded answer context requires a lexical anchor, and the CLI validates citations against the exact retrieved spans. Missing evidence produces an explicit refusal. `RI_INDEX_V1` snapshots store source, revision, and commit metadata; vectors are recomputed on load.

## 4. Files changed

- `src/lib.rs`: indexing, chunking, embedding, fusion, persistence, commit metadata, safety, and tests.
- `src/main.rs`: semantic/hybrid/commit/index CLI modes, grounded evidence handling, citation validation, and HTTP JSON responses.
- `src/llm.rs`: provider prompts and explicit untrusted-evidence/refusal rules.
- `evaluation/evaluate_modes.py`: deterministic lexical/semantic/hybrid comparison.
- `.github/workflows/ci.yml`, `scripts/validate.sh`: run both evaluators in normal validation.
- `README.md`, `docs/architecture.md`, `docs/llm-validation.md`, `RELEASE_NOTES.md`: public-facing capability and limitation documentation.
- `tests/ri_regression.rs`: quoted Git paths, snapshot policy, duplicate records, provider seam, and citation regression tests.

## 5. Verification

Passed on the local checkout:

```text
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked                 15 tests passed (10 unit + 5 integration)
python3 evaluation/evaluate.py      Recall@5 1.00, MRR 1.0000
python3 evaluation/evaluate_modes.py
```

The full `scripts/validate.sh` flow also passed, including HTTP health, reload, and hybrid search smoke checks. A 200-request localhost benchmark was recorded separately; its values are plumbing measurements, not capacity claims.

## 6. Evaluation

On the authored 20-question, file-level corpus:

| mode | Recall@5 | MRR |
| --- | ---: | ---: |
| lexical | 1.00 | 1.0000 |
| semantic (`hash-token-v1`) | 1.00 | 0.9375 |
| hybrid RRF (`k=60`) | 1.00 | 1.0000 |

This corpus is a deterministic regression fixture. `HashEmbedding` is a vector baseline, not a trained language embedding model; the numbers do not establish general retrieval quality.

## 7. Remaining technical debt

Immutable commit-tree indexing, language-specific AST parsers, a learned embedding provider, a larger independently held-out corpus, full historical blob/blame retrieval, background watching, answer-quality evaluation, and HTTP authentication/TLS/rate limiting remain open. AGY does not expose token or monetary usage in the current provider path.

## 8. Public-readiness decision

The repository is a credible public engineering candidate for local repository indexing and evidence-grounded retrieval, with its current limitations disclosed. It is not a production multi-tenant service and should not claim trained semantic quality or general LLM answer correctness. GitHub visibility was intentionally not changed by this implementation; publishing remains a separate account-level decision.
