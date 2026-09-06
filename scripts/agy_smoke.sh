#!/usr/bin/env bash
set -euo pipefail

command -v agy >/dev/null || { echo 'agy CLI is required for this optional smoke test' >&2; exit 2; }
agy --model "${AGY_MODEL:-gemini-3.8-flash-low}" --print-timeout 60s --print \
  'Answer using only this evidence: /reload applies the Git diff from the indexed commit to current HEAD, updates added/modified/deleted paths, and returns the new commit. Respond with one sentence and do not invent details.'
