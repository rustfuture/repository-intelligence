#!/usr/bin/env bash
set -euo pipefail

output="$(cargo run --quiet --locked -- --answer . "What does the reload endpoint do?")"
printf '%s\n' "$output"
grep -Eq '(^|[^[:alnum:]_])README\.md:[0-9]+' <<<"$output"
if grep -q 'file://' <<<"$output"; then
  echo 'unvalidated file URI emitted by model' >&2
  exit 1
fi
