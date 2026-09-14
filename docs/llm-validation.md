# LLM validation note

The repository has an optional, explicitly external LLM smoke path. It uses the authenticated AGY CLI; credentials are not stored in the repository, while retrieved repository evidence is sent to AGY for the requested answer.

```bash
./scripts/agy_smoke.sh
```

On 2026-09-06, `gemini-3.8-flash-low` returned:

> The `/reload` endpoint applies the Git diff from the indexed commit to the current HEAD, updates added, modified, and deleted paths, and returns the new commit.

This verifies a real model invocation against supplied evidence, not a mock. The Rust `--answer` command is the product's optional answer path, and the library now exposes a provider abstraction with AGY and Ollama implementations. The smoke result is still narrow: it does not establish general hallucination resistance or answer quality.

The optional Rust CLI path (`cargo run --locked -- --answer . "What does the reload endpoint do?"`) was also run with the same model. It retrieved up to five hybrid evidence chunks, passed only that evidence to AGY, and returned an answer with the indexed commit. `scripts/llm_answer_smoke.sh` requires a repository `path:line` citation, rejects `file:///` links, verifies every cited file and line number exists in the checkout, and checks the expected `/reload` concepts (Git diff, added/modified/deleted paths, and commit). This is a narrow semantic regression test, not a general hallucination benchmark.

The CLI records model name and wall-clock duration for each call. AGY does not expose token usage or monetary cost in this path, so `cost_usd=unknown` is reported instead of an estimate; the local Ollama provider reports `cost_usd=0` for local execution.

`scripts/prompt_injection_smoke.sh` indexes an authored malicious instruction alongside valid evidence. The gate fails if the model emits the `PWNED` sentinel or omits the expected reload concepts. This is a single attack fixture, not a comprehensive prompt-injection audit.
