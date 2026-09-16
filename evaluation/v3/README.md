# Evaluation v3 — extractive selection

This is the current evaluation for the answer path. It measures the
`extractive-selection-v1` contract:

- the model may return only evidence IDs (`[E1]`, at most three) or `NONE`;
- the application validates every ID and prints verbatim source text;
- the evaluator checks **quotation integrity** and **expected-span coverage** —
  never natural-language entailment.

## Files

| Path | Contents |
|---|---|
| `questions.json` | 52 authored questions (10 dev, 42 held-out: 30 answerable, 12 unanswerable). Pinned `expected_sources` and `expected_answer` keys; no per-question code. |
| `evaluate.py` | Runs all 42 held-out questions against the local Ollama model and writes `raw.jsonl`, `summary.json`, `report.md`, `manifest.json`. `--render-only <run-dir>` re-renders summary/report from `raw.jsonl` with **no model calls**. |
| `test_evaluate.py` | Offline scoring-rule tests including the renderer. |
| `run-01/` | Frozen run: per-question raw records, derived summary and report, manifest with commit/dirty state, source/corpus/question SHA-256, and model digests. |

## Scoring rule (generic, data-driven)

For each held-out question the evaluator derives a verdict from the raw record:

| Verdict | Meaning |
|---|---|
| `expected_source_covered` | accepted, quotation exact, all cited spans relevant, all `expected_sources` covered |
| `partial_source_coverage` | accepted and relevant, but not every expected span was selected |
| `irrelevant_selection` | accepted, but no cited span overlaps an expected source |
| `quotation_failure` | accepted, but quoted text does not match the cited range |
| `false_selection` | accepted on an **unanswerable** question |
| `false_refusal` | answerable question not accepted |
| `correct_refusal` | unanswerable question not accepted |
| `provider_error` | the provider failed; not counted as a refusal |

`summary.json` reports true numerators/denominators, `false_accepts`,
`false_rejects`, `model_called`, and `pre_model_refusals` (questions where no
lexical anchor existed and the model was never called — a fact about the index,
not about model or guard refusal behaviour).

## What is not claimed

Source overlap is not entailment. `expected_source_covered` means the model
selected the frozen expected span with an exact quotation; it does **not** prove
the excerpt answers the question in natural language. Semantic correctness stays
a manual inspection (`run-01/manual-review.md`). The corpus is authored and the
questions were previously exposed, so this is a regression record, not
unseen-generalization evidence. No thresholds were tuned on the reported run.

## Reproduce

```sh
python3 evaluation/v3/evaluate.py --output evaluation/v3/my-run
python3 evaluation/v3/evaluate.py --render-only evaluation/v3/my-run   # no model calls
```
