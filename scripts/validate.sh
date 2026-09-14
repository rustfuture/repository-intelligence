#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 evaluation/evaluate.py
python3 evaluation/evaluate_modes.py

port="${RI_VALIDATION_PORT:-28184}"
cargo run --quiet --locked -- --serve "127.0.0.1:${port}" . >"${TMPDIR:-/tmp}/repository-intelligence-${port}.log" 2>&1 &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null || true' EXIT
for _ in $(seq 1 30); do
    if curl -fsS "http://127.0.0.1:${port}/health" >/dev/null 2>&1; then break; fi
    sleep 0.1
done
health="$(curl -fsS "http://127.0.0.1:${port}/health")"
reload="$(curl -fsS "http://127.0.0.1:${port}/reload")"
search="$(curl -fsS "http://127.0.0.1:${port}/search?q=line+cited")"
printf 'health=%s\nreload=%s\nsearch=%s\n' "$health" "$reload" "${search:0:240}"
