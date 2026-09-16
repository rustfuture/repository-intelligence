# Extractive regression evaluation

Mode: `extractive-selection-v1`. The model returns evidence IDs only; the application
renders verbatim source text. This report is rendered from `raw.jsonl`; regenerate with
`python3 evaluation/v3/evaluate.py --render-only <run-dir>` (no model calls).

## Aggregate (numerator / denominator)

- Questions: 42 held-out (30 answerable, 12 unanswerable)
- Model calls: 41 / 42; pre-model refusals (no lexical anchor, model never called): 1
- Provider errors: 0
- False accepts (accepted selection that failed the rule): 8
- False rejects (answerable question not accepted): 3

| Verdict | Count |
|---|---:|
| `correct_refusal` | 8 |
| `expected_source_covered` | 23 |
| `false_refusal` | 3 |
| `false_selection` | 4 |
| `irrelevant_selection` | 3 |
| `partial_source_coverage` | 1 |

## Per-question verdicts

| Question | Split | Answerable | Decision | Model called | Verdict | Quotation exact | Expected span coverage |
|---|---|---|---|---|---|---|---|
| HELD-01 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-02 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-03 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-04 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-05 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-06 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-07 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-08 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-09 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-10 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-11 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-12 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-13 | heldout | True | `refused` | True | `false_refusal` | None | False |
| HELD-14 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-15 | heldout | True | `accepted` | True | `irrelevant_selection` | True | False |
| HELD-16 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-17 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-18 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-19 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-20 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-21 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-22 | heldout | True | `accepted` | True | `irrelevant_selection` | True | False |
| HELD-23 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-24 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-25 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-26 | heldout | True | `accepted` | True | `partial_source_coverage` | True | False |
| HELD-27 | heldout | True | `refused` | True | `false_refusal` | None | False |
| HELD-28 | heldout | True | `refused` | True | `false_refusal` | None | False |
| HELD-29 | heldout | True | `accepted` | True | `irrelevant_selection` | True | False |
| HELD-30 | heldout | True | `accepted` | True | `expected_source_covered` | True | True |
| HELD-31 | heldout | False | `accepted` | True | `false_selection` | True | False |
| HELD-32 | heldout | False | `accepted` | True | `false_selection` | True | False |
| HELD-33 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-34 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-35 | heldout | False | `pre_model_refusal` | False | `correct_refusal` | None | False |
| HELD-36 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-37 | heldout | False | `accepted` | True | `false_selection` | True | False |
| HELD-38 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-39 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-40 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-41 | heldout | False | `refused` | True | `correct_refusal` | None | False |
| HELD-42 | heldout | False | `accepted` | True | `false_selection` | True | False |

## Limits

- Source overlap is **not** entailment. `expected_source_covered` means the model selected
  the frozen expected span with an exact quotation; it does not prove the excerpt answers
  the question in natural language. Semantic correctness is not automatically judged.
- The corpus and questions are authored and were previously exposed; this is a regression
  record, not unseen-generalization evidence. No thresholds were tuned on this run.
- Accepted text is always verbatim source text, so quotation integrity is checked; the
  remaining risk is relevance and interpretation, which stay manual.
