# Evaluation v2 — historical, superseded

This directory is kept as **historical evidence** for the v2 evaluation round. It
is not the current evaluation and its framing must not be reused.

What was wrong with the v2 framing:

- The real-model generation sample covered only **10 questions (6 answerable,
  4 traps)**, but the repository-level text reported a 12/12 trap-refusal rate
  taken from a different, non-model check. Numerator and denominator did not
  match.
- "Claim support" was decided by question-ID-specific keyword checks
  (`if q_id == "HELD-01" and "2" in output_text: ...`) inside the evaluator.
  That is not an independent correctness judgement, and passing those checks
  required almost nothing from the model.
- The free-form answer path it measured was later replaced by
  `extractive-selection-v1`, where the model returns evidence IDs and the
  application prints verbatim source text.

Current evaluation: [`evaluation/v3`](../v3/README.md) and
[`evaluation/v3/run-01/report.md`](../v3/run-01/report.md). Do not pool v2 and v3
numbers.

`report.md` and `summary.json` here are frozen v2 outputs and are unedited except
for the banner at the top of `report.md`. The v3 evaluator publishes the
pre-model-refusal count separately, which is what v2 conflated.
