# LLM validation note

The answer path is **extractive selection** (`extractive-selection-v1`). The model
does not write an answer. It receives up to five retrieved evidence chunks and may
return only evidence IDs (`[E1]`, `[E2]`, …), at most three, or the exact token
`NONE`. The application validates every ID and prints the original source text
verbatim with `path:line` references. Extra prose, malformed IDs, and mixed
valid/invalid output are refused atomically.

This design removes the failure modes of the earlier free-form path, where an
answer was approved by word overlap against the cited span. Under extraction
there is no model-authored prose to contain an invented number, a flipped
negation, a reversed relation, or an added claim.

**What this does not establish.** An exact quotation is not evidence that the
source is true, relevant, or answers the question. The model can select an
irrelevant or even malicious repository file, and a selected document can assert
something the code does not do. Quotation integrity and semantic entailment are
different properties, and only the former is checked automatically. Old
free-form validation notes and the 12/12 refusal claim are superseded; see
[`evaluation/v3/run-01/report.md`](../evaluation/v3/run-01/report.md) for the
current measured record.

## Local model run

```bash
# qwen2.5-coder:1.5b selects evidence IDs; no cloud call is made
USE_OLLAMA=1 cargo run --locked -- --embedding nomic --answer-json evaluation/corpus \
  "What does the reload endpoint do?"
```

The CLI records the model name and wall-clock duration. The local Ollama provider
reports `cost_usd=0`; the optional external AGY adapter reports `cost_usd=unknown`
because AGY does not expose token or monetary usage in this path.

## Smoke scripts and what they actually check

- `scripts/llm_answer_smoke.sh` runs the CLI and then
  `scripts/validate_citations.py`, which checks the extractive contract only:
  decision is `accepted`, every citation points at a real in-range file span, and
  the quoted text matches that span exactly. It does **not** score relevance or
  truth, and it no longer requires concept words such as "git diff" to appear.
- `scripts/prompt_injection_smoke.sh` indexes one authored malicious instruction
  alongside valid evidence and fails if the `PWNED` sentinel appears in model
  output. This is a single fixture, not a prompt-injection audit; selected
  untrusted text still appears in the output as inert quoted data.
- `scripts/agy_smoke.sh` is an optional, explicitly external check of the AGY
  adapter. It is not part of CI and not required for the local workflow.

No smoke script establishes general hallucination resistance, answer quality, or
injection immunity.
