# Architecture

```text
working tree
      |
      v
safe scanner + content hashes
      |
      v
source lines + lexical term map
      |
      v
declaration-aware chunks (or 40-line text chunks)
      |                         |
      v                         v
HashEmbedding vectors      lexical ranker
      |                         |
      +----------+--------------+
                 v
        semantic ranker + RRF (k=60)
                 |
                 v
       citable evidence (path:line span)
                 |
                 +--> optional AGY/Ollama answer provider

Git diff / dirty worktree -->> incremental synchronization
```

`Index` is built from the repository working tree, not an immutable Git tree. It owns source lines, an inverted term map, code-aware chunks, content hashes, vectors, the indexed revision, and up to 100 recent commit records (SHA, date, and subject). The checked-in `HashEmbedding` provider is deterministic and offline; `EmbeddingProvider` is the replacement seam for another provider.

The scanner skips symlinks, hidden path components, common build/dependency directories, selected binary extensions, and sensitive file names. `sync_worktree` hashes the current eligible files, re-indexes changed or new files, and removes files that disappeared. `sync_git` uses `git diff --name-status --find-renames`; add/modify/copy/type-change paths are updated, deletes are removed, and renames are handled as delete plus update. Dirty or unavailable Git history falls back to the worktree refresh.

Chunks start at lightweight function, class, struct, trait, or impl declarations when recognized. A declaration chunk is capped at 80 source lines; regions without declarations use 40-line generic chunks. Every evidence item preserves its repository-relative path, start/end lines, source text, kind, and retrieval score.

The default CLI mode remains line-level lexical search for compatibility. `--semantic` uses vector cosine similarity, `--hybrid` and the HTTP `/search` endpoint use reciprocal-rank fusion with `k=60`, and `--commits` searches the retained commit subjects/dates. `--analytics` exposes index statistics. `save_to` and `load_from` implement the portable `RI_INDEX_V1` snapshot format (source, revision, and commit metadata); vectors are recomputed on load, and the CLI exposes snapshot creation through `--index` and hybrid querying through `--load-index`.

The local HTTP process keeps its in-memory index behind an `RwLock`; `/reload` takes a write lock while `/health`, `/search`, and `/commits` take a read lock. The accept loop is single-threaded. `/search` returns hybrid evidence and the indexed revision; `/reload` reports the new revision after synchronization.

The optional `--answer` path requires lexical evidence before constructing up to five hybrid evidence chunks. It sends that evidence in a provider prompt to AGY (default model configurable with `AGY_MODEL`) or to Ollama when `USE_OLLAMA` is set. Repository text is data, not instructions. The CLI performs heuristic citation screening against the supplied evidence; the optional smoke scripts additionally check expected concepts and that cited paths/lines exist. These checks are not proof of answer correctness.

Current limitations are the working-tree snapshot boundary, simple ASCII tokenization, the untrained hash-vector baseline, conservative declaration parsing, the 100-record commit metadata limit, no full historical blob or blame retrieval, no background watcher, and no authentication, TLS, rate limiting, or multi-tenant isolation. External LLM calls receive retrieved repository text, and AGY does not provide token or monetary usage data in this path.
