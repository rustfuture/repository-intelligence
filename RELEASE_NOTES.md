# Release notes

## 0.1.0 — baseline candidate

- Deterministic line-cited lexical index and CLI.
- Git revision metadata and incremental A/M/D reload behavior.
- Local `/health`, `/search`, and `/reload` HTTP endpoints.
- Fixed 20-question authored retrieval evaluation.
- Optional real AGY/Gemini answer path with latency metadata.
- Citation-shape, file/line-range, semantic-concept, and prompt-injection smoke gates.
- Stable and Rust 1.85 GitHub Actions checks.

Local sequential HTTP benchmark on the fixed authored corpus (200 requests): p50 `0.210 ms`, p95 `0.432 ms`, max `0.698 ms`. These figures are localhost plumbing measurements, not production capacity claims.

This is not a production service. Authentication, TLS, durable storage, embedding/hybrid retrieval, broad evaluation, and cost/token accounting remain outside this release candidate.
