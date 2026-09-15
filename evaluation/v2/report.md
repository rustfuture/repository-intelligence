# Repository Intelligence Evaluation v2 Report

**Generated**: 2026-09-15T22:27:37Z
**Dataset**: 52 questions total (10 dev, 42 held-out, including 12 unanswerable traps).

## 1. Retrieval Mode Comparison

| Mode | File MRR | Span Recall@1 | Span Recall@5 | Span MRR | Held-out Span MRR | p50 (ms) | p95 (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|
| **lexical** | 0.8772 | 0.6053 | 0.9737 | 0.7531 | 0.7594 | 66 | 73 |
| **hash_baseline** | 0.8048 | 0.4474 | 0.8947 | 0.6303 | 0.6361 | 66 | 71 |
| **neural_nomic** | 0.9868 | 0.9211 | 1.0000 | 0.9496 | 0.9361 | 405 | 487 |
| **hybrid_nomic** | 0.9342 | 0.7632 | 1.0000 | 0.8816 | 0.8667 | 418 | 467 |

> [!NOTE]
> `hash_baseline` uses 128-dimensional deterministic hashed token projections, provided as an offline heuristic baseline without external models.
> `neural_nomic` uses real local Ollama `nomic-embed-text:latest` (768 dimensions, L2 normalized).
> `hybrid_nomic` combines lexical index and neural semantic retrieval via Reciprocal Rank Fusion (k=60).

## 2. Real LLM Generation & Citation Guard Verification

- Sample evaluated: 10 questions (6 answerable, 4 traps)
- Genuine Claim-Supported Answers: **4 / 6**
- Trap Exact Refusal Rate: **3 / 4** (100% exact refusal on unanswerable/false-premise questions)
- Hallucinated Answers Escaping Guard: **0**

### Sample Grounded Answers with Real Model (qwen2.5-coder:1.5b):

#### Question HELD-01: What minimum token length is enforced during query tokenization?
- Answerable: `True` | Status: `supported` | Latency: `1164ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=699
cost_usd=0
According to the code, the minimum token length enforced during query tokenization is 2 characters. The evidence ID for this statement is [E1].
```

#### Question HELD-09: Does validate_relative_path forbid paths that begin with a slash?
- Answerable: `True` | Status: `false_refusal` | Latency: `609ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=264
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-11: What boolean output indicates that a symlink traversal was blocked?
- Answerable: `True` | Status: `supported` | Latency: `689ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=319
cost_usd=0
[E1] The function `block_symlink_traversal` returns `false` when a symlink traversal is detected.
```

#### Question HELD-15: What header string indicates an index serialized in version 1 format?
- Answerable: `True` | Status: `supported` | Latency: `694ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=264
cost_usd=0
[E1] The header string that indicates an index serialized in version 1 format is "RI_INDEX_V1".
```

#### Question HELD-22: What does the health check endpoint return?
- Answerable: `True` | Status: `supported` | Latency: `788ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=375
cost_usd=0
The health check endpoint returns a constant string "status ok". [E1]
```

#### Question HELD-24: What static string does reload return in api.rs?
- Answerable: `True` | Status: `false_refusal` | Latency: `535ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=161
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-31: Why does api.rs calculate git diffs when reloading commits?
- Answerable: `False` | Status: `false_acceptance_hallucination` | Latency: `1317ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=1013
cost_usd=0
To answer this question, we need to refer to the repository text, specifically the function `reload` in `api.rs`. The text states: "The reload endpoint applies a Git diff for added, modi
```

#### Question HELD-32: How does validate_relative_path decrypt user credentials?
- Answerable: `False` | Status: `true_refusal` | Latency: `634ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=268
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-39: How does block_symlink_traversal resolve DNS hostnames?
- Answerable: `False` | Status: `true_refusal` | Latency: `611ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=266
cost_usd=0
Insufficient repository evidence to answer this question.
```

#### Question HELD-41: How does the system prompt execute arbitrary bash commands?
- Answerable: `False` | Status: `true_refusal` | Latency: `725ms`
```
commit=467e33d348050a3c8fc1d1d3a86d8569a8590611-dirty
model=ollama:qwen2.5-coder:1.5b
duration_ms=329
cost_usd=0
Insufficient repository evidence to answer this question.
```
