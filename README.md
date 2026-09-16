# Repository Intelligence

A local Rust repository index with lexical, neural and hybrid retrieval, source
line references, incremental updates, and model-assisted **source selection**.

## Answer contract

`--answer` and `--answer-json` use extractive-selection-v1. The model returns
only evidence IDs. The application validates every ID and renders the original
source text with file/line references. Extra prose, invalid IDs and mixed
valid/invalid output are refused atomically.

This deliberately replaces free-form answers approved by word-overlap heuristics.
**An exact quotation is not proof that a source is true or answers the question.**
Selections may be irrelevant, malicious repository text may be quoted as data,
and the local model can make false selections or refuse useful sources. The UI
labels output as source excerpts, not verified factual answers. No shell/tool
execution is granted to the answering model.

The older heuristic citation functions remain experimental library APIs for
compatibility with the unfinished work; the CLI does not use them as an
entailment verifier. XML delimiters are not a security boundary.

## Run

Requires Rust 1.85+; local neural inference requires an already installed Ollama
daemon, `nomic-embed-text`, and `qwen2.5-coder:1.5b`.

```sh
cargo build --locked
cargo run --locked -- evaluation/corpus "reload"
cargo run --locked -- --embedding nomic --semantic evaluation/corpus "index header"
USE_OLLAMA=1 cargo run --locked -- --embedding nomic --answer-json evaluation/corpus "What static string does reload return in api.rs?"
```

Default retrieval uses deterministic HashEmbedding (not a learned model).
`RI_EMBEDDING_PROVIDER=nomic` selects local neural embeddings.
`USE_OLLAMA=1` selects Ollama; without it the existing AGY adapter is used.
No automatic hash fallback is claimed when a selected neural provider fails.

## Architecture

Files → filtered scanner → code chunks and line spans → lexical/vector index →
hybrid evidence → model selects IDs → application renders source quotations.

The local HTTP service exposes `/health`, `/search?q=...`, `/commits?q=...`
and `/reload`. It has no TLS/auth/multi-tenant isolation. Bind to loopback.
Git synchronization handles changed/deleted files and marks dirty worktrees;
a dirty label alone is not an immutable snapshot identifier.

## Current measured results

[Full v3 regression record](evaluation/v3/run-01/report.md):
42 previously exposed held-out questions, 30 answerable and 12 unanswerable;
41 model calls and 1 pre-model refusal. Local Qwen and Nomic digests, corpus,
questions and source hashes are recorded in the manifest.

| Outcome | Count |
|---|---:|
| Expected source fully covered | 18 |
| Irrelevant selection | 8 |
| Partial source coverage | 1 |
| False refusal | 3 |
| Correct refusal on unanswerable questions | 9 |
| False selection on unanswerable questions | 3 |

Derived from the same raw records: **12 false accepts** (accepted selections that
failed the expected-source rule: 8 irrelevant + 1 partial + 3 selections on
unanswerable questions) and **3 false rejects** (answerable questions that were
not accepted). Those are the numbers the old single "refusal accuracy" figure hid.

All accepted excerpts matched their source text in this run. That is quotation
integrity, **not** 100% answer correctness. Source-overlap scoring is generic and
uses frozen expected spans; it does not establish entailment. The tiny authored
corpus and previously exposed questions are regression evidence, not an unseen
generalization benchmark. Latency includes process startup, index rebuild and
generation. No held-out tuning was performed after this run.

Reproduce into a new directory (existing outputs are never overwritten):
```sh
python3 evaluation/v3/evaluate.py --output evaluation/v3/my-run
python3 evaluation/v3/evaluate.py --render-only evaluation/v3/my-run   # re-render from raw.jsonl, no model calls
```

`summary.json` reports true numerators/denominators and separates `false_accepts`,
`false_rejects`, `model_called` and `pre_model_refusals`. A pre-model refusal means
the index had no lexical anchor and the model was never called; it is a fact about
the index, not model or guard refusal accuracy. The trap classification in
`evaluation/results_modes.json` is reported the same way, which is what the old
single "12/12 refusal accuracy" number conflated.

Old v1/v2 reports are historical. In particular the previous 12/12 refusal and
universal injection-defense claims are superseded; v2's real generation sample
contained only four traps. Retrieval and generation metrics must not be pooled.

## Verification

```sh
cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 -m unittest discover -s evaluation/v3 -p 'test_*.py'
```

Offline tests cover valid selections as well as malformed IDs, mixed selections,
uncited prose, altered numbers, negation, reversed relations and added claims.
They enforce the extractive protocol, not general natural-language reasoning.

## Design references

- [ALCE (EMNLP 2023)](https://aclanthology.org/2023.emnlp-main.398/):
  citation presence and support are different evaluation dimensions.
- [Anthropic long-context experiments](https://www.anthropic.com/news/prompting-long-context):
  quote extraction can make source use more inspectable.

A further synthesis layer would need its own evaluation. Adding a larger model
or an NLI judge does not by itself guarantee correctness.

## License

MIT. The evaluation corpus is authored for this project.

