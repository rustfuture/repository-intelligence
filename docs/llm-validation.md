# LLM validation note

The repository has an optional, explicitly external LLM smoke path. It uses the authenticated AGY CLI and does not put credentials or private repository content in the project.

```bash
./scripts/agy_smoke.sh
```

On 2026-09-06, `gemini-3.8-flash-low` returned:

> The `/reload` endpoint applies the Git diff from the indexed commit to the current HEAD, updates added, modified, and deleted paths, and returns the new commit.

This verifies a real model invocation against supplied evidence, not a mock. It is not yet the product’s automated answer path; hallucination evaluation, prompt-injection tests, provider abstraction, and cost accounting remain before claiming completed LLM integration.

The optional Rust CLI path (`cargo run --locked -- --answer . "What does the reload endpoint do?"`) was also run with the same model. It retrieved local evidence, passed only the top cited lines to AGY, and returned an answer with the indexed commit. `scripts/llm_answer_smoke.sh` requires a repository `path:line` citation, rejects `file:///` links, verifies every cited file and line number exists in the checkout, and checks the expected `/reload` concepts (Git diff, added/modified/deleted paths, and commit). This is a narrow semantic regression test, not a general hallucination benchmark.

The CLI records model name and wall-clock duration for each call. AGY does not expose token usage or monetary cost in this path, so `cost_usd=unknown` is reported instead of an estimate.
