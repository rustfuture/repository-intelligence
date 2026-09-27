# Measured Results and Historical Notes

## v3 Regression Record

[Full v3 regression record](../evaluation/v3/run-03/report.md): 42 previously exposed questions — 30 answerable, 12 unanswerable; 41 model calls and 1 pre-model refusal. Local Qwen and Nomic digests, corpus, questions, and source hashes are recorded in the manifest.

### Outcome Breakdown

| Outcome | Count |
|---|---:|
| Expected source fully covered | 21 |
| Irrelevant selection | 5 |
| Partial source coverage | 2 |
| False refusal | 2 |
| Correct refusal on unanswerable questions | 7 |
| False selection on unanswerable questions | 5 |

Derived from the same raw records: **12 false accepts** (5 irrelevant + 2 partial + 5 selections on unanswerable questions) and **2 false rejects**. An earlier aggregate refusal figure did not separate these outcomes.

All accepted excerpts matched their source text in this run — that is quotation integrity, **not** answer correctness. Source-overlap scoring uses frozen expected spans and does not establish entailment. The corpus is small, authored, and previously exposed: this is regression evidence, not an unseen generalization benchmark.

## Historical Notes

Old v1/v2 reports are historical; their 12/12 refusal and universal injection-defense claims are superseded. Retrieval and generation metrics are never pooled. A pre-model refusal means the index had no lexical anchor and the model was never called — a fact about the index, not model or guard refusal accuracy.

## Local HTTP Benchmark

The committed HTTP benchmark (`evaluation/v3/benchmark-local-http.json`) measures 200 sequential localhost requests on the fixed authored corpus at p50 0.857 ms / p95 0.920 ms. It measures request overhead on that setup only, not generation latency.
