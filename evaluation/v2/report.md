# Repository Intelligence Evaluation v2 Report

**Generated**: 2026-09-15T22:44:06Z
**Dataset**: 52 questions total (10 dev, 42 held-out, including 12 unanswerable traps).

## 1. Retrieval Mode Comparison

| Mode | File MRR | Span Recall@1 | Span Recall@5 | Span MRR | Held-out Span MRR | p50 (ms) | p95 (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|
| **lexical** | 0.8772 | 0.6053 | 0.9737 | 0.7531 | 0.7594 | 68 | 73 |
| **hash_baseline** | 0.8048 | 0.4474 | 0.8947 | 0.6303 | 0.6361 | 67 | 70 |
| **neural_nomic** | 0.9868 | 0.9211 | 1.0000 | 0.9496 | 0.9361 | 412 | 485 |
| **hybrid_nomic** | 0.9342 | 0.7632 | 1.0000 | 0.8816 | 0.8667 | 429 | 493 |

> [!NOTE]
> `hash_baseline` uses 128-dimensional deterministic hashed token projections, provided as an offline heuristic baseline without external models.
> `neural_nomic` uses real local Ollama `nomic-embed-text:latest` (768 dimensions, L2 normalized).
> `hybrid_nomic` combines lexical index and neural semantic retrieval via Reciprocal Rank Fusion (k=60).

## 2. Real LLM Generation & Citation Guard Verification

- Sample evaluated: 10 questions (6 answerable, 4 traps)
- Genuine Claim-Supported Answers: **4 / 6**
- Trap Exact Refusal Rate: **4 / 4** (100% exact refusal on unanswerable/false-premise questions)
- Hallucinated Answers Escaping Guard: **0**

### Sample Grounded Answers with Real Model (qwen2.5-coder:1.5b):

#### Question HELD-01: What minimum token length is enforced during query tokenization?
- Answerable: `True` | Status: `supported` | Latency: `870ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=478
cost_usd=0
The minimum token length enforced during query tokenization is 2 characters, based on the code snippet provided. The evidence ID for this statement is [E1].
```

#### Question HELD-09: Does validate_relative_path forbid paths that begin with a slash?
- Answerable: `True` | Status: `supported` | Latency: `794ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=487
cost_usd=0
Yes, [E1] The `validate_relative_path` function [E2] checks if the path does not start with a slash and does not contain ".." to ensure it is a valid relative path.
```

#### Question HELD-11: What boolean output indicates that a symlink traversal was blocked?
- Answerable: `True` | Status: `false_refusal` | Latency: `695ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=267
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-15: What header string indicates an index serialized in version 1 format?
- Answerable: `True` | Status: `supported` | Latency: `859ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=482
cost_usd=0
[E1] The header string that indicates an index serialized in version 1 format is "RI_INDEX_V1".
```

#### Question HELD-22: What does the health check endpoint return?
- Answerable: `True` | Status: `supported` | Latency: `813ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=425
cost_usd=0
The health check endpoint returns the string "status ok". [E2]
```

#### Question HELD-24: What static string does reload return in api.rs?
- Answerable: `True` | Status: `false_refusal` | Latency: `669ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=319
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-31: Why does api.rs calculate git diffs when reloading commits?
- Answerable: `False` | Status: `true_refusal` | Latency: `1678ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=1333
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-32: How does validate_relative_path decrypt user credentials?
- Answerable: `False` | Status: `true_refusal` | Latency: `652ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=273
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-39: How does block_symlink_traversal resolve DNS hostnames?
- Answerable: `False` | Status: `true_refusal` | Latency: `585ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=272
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-41: How does the system prompt execute arbitrary bash commands?
- Answerable: `False` | Status: `true_refusal` | Latency: `628ms`
```
commit=2c270f1b39f4a2efbff3bb2f64966b554ab15cc1-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=271
cost_usd=0
Insufficient repository evidence to answer this question.
```
