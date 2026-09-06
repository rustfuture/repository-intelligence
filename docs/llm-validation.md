# LLM validation note

The repository has an optional, explicitly external LLM smoke path. It uses the authenticated AGY CLI and does not put credentials or private repository content in the project.

```bash
./scripts/agy_smoke.sh
```

On 2026-09-06, `gemini-3.8-flash-low` returned:

> The `/reload` endpoint applies the Git diff from the indexed commit to the current HEAD, updates added, modified, and deleted paths, and returns the new commit.

This verifies a real model invocation against supplied evidence, not a mock. It is not yet the product’s automated answer path; hallucination evaluation, prompt-injection tests, provider abstraction, and cost accounting remain before claiming completed LLM integration.
