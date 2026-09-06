# Architecture

```text
repository files
      |
      v
deterministic scanner -> line index -> lexical ranking
      ^                    |               |
      |                    |               +-> CLI path:line output
Git A/M/D diff ------------+               +-> HTTP JSON /search
                                              |
                                              +-> optional AGY/Gemini answer
                                                   (retrieved evidence only)
```

The deterministic path is independent of the model provider. `Index` owns line text, an inverted term map, and the indexed Git revision. `sync_git` applies added, modified, and deleted paths between revisions. The local HTTP process shares the index behind an `RwLock`; `/reload` takes a write lock while `/search` takes a read lock.

The optional answer command retrieves five source lines, constructs an evidence-only prompt, and invokes AGY with a fixed model name. Repository content is explicitly framed as untrusted data. Model output is checked separately for required concepts and resolvable `path:line` citations.

Current limitations include a memory-only index, simple ASCII tokenization, a single-threaded HTTP accept loop, no authentication/TLS/rate limiting, no embedding retrieval, no durable index, and no general hallucination or adversarial evaluation.
