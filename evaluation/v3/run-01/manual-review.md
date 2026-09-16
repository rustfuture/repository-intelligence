# Source inspection (not an automatic entailment score)

The coordinator read these outputs against the checked-in corpus after the run.
No answers or thresholds were tuned from these results.

| Question | Inspection |
|---|---|
| HELD-01 | retrieval.rs contains `.filter(|t| t.len() >= 2)`, answering minimum token length. |
| HELD-17 | storage.rs contains `Err("invalid magic header")`, answering the error-message question. |
| HELD-24 | api.rs returns `"reloaded commit"`; the excerpt answers the static-string question. |
| HELD-31 | untrusted.md asserts a Git-diff behavior absent from the stub. False selection; exact quotation does not make the document trustworthy. |
| HELD-32 | indexing.rs update stub does not explain credential decryption. False selection. |
| HELD-42 | README does not contain gzip implementation. False selection. |

These six checks do not establish semantic correctness for the entire dataset.
The full run has 18 expected-source coverage results, not 18 automatically proven
natural-language answers. All accepted source text matched the indexed file ranges.
