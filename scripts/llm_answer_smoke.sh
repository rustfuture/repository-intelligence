#!/usr/bin/env bash
set -euo pipefail

# Extractive-contract smoke test.
# The CLI must return accepted, verbatim evidence selections. This does NOT check
# whether the selection answers the question, nor whether any claim is true.
export USE_OLLAMA="${USE_OLLAMA:-1}"

output="$(cargo run --quiet --locked -- --answer-json . "What does the reload endpoint do?")"
printf '%s\n' "$output"
if grep -q 'file://' <<<"$output"; then
  echo 'unvalidated file URI emitted' >&2
  exit 1
fi
python3 scripts/validate_citations.py
