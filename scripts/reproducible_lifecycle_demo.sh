#!/usr/bin/env bash
# Reproducible Lifecycle Demo for repository-intelligence
#
# Demonstrates:
# 1. Dedicated isolated temporary Git repo creation
# 2. Initial indexing with real Ollama embedding (nomic-embed-text)
# 3. Hybrid search retrieval
# 4. Evidence-grounded answer generation with real LLM (qwen2.5-coder:1.5b)
# 5. Exact refusal on unanswerable question
# 6. File modification -> index update -> new content appears, old content purged
# 7. File deletion -> index update -> deleted content purged from search & evidence
# 8. Snapshot/commit/dirty state verification
# 9. Clean self-contained teardown

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
BIN="$REPO_ROOT/target/debug/repository-intelligence"

if [[ ! -x "$BIN" ]]; then
    echo "Building repository-intelligence..."
    (cd "$REPO_ROOT" && cargo build --locked)
fi

DEMO_DIR="$(mktemp -d /tmp/ri-lifecycle-demo-XXXXXX)"
cleanup() {
    rm -rf "$DEMO_DIR"
}
trap cleanup EXIT INT TERM

echo "============================================================"
echo " repository-intelligence Reproducible Lifecycle Demo"
echo " Workspace: $DEMO_DIR"
echo "============================================================"

# Step 1: Initialize Git Repo & Authored Source Files
echo ""
echo "[Step 1/8] Initializing temporary git repository..."
git -C "$DEMO_DIR" init -q
git -C "$DEMO_DIR" config user.name "Demo User"
git -C "$DEMO_DIR" config user.email "demo@example.invalid"

mkdir -p "$DEMO_DIR/src"
cat <<'EOF' > "$DEMO_DIR/src/auth.rs"
pub fn verify_token_lifetime(issued_at: u64, now: u64) -> bool {
    now.saturating_sub(issued_at) < 3600
}
EOF

cat <<'EOF' > "$DEMO_DIR/src/config.rs"
pub fn database_host_cluster() -> &'static str {
    "postgres.internal.cluster"
}
EOF

git -C "$DEMO_DIR" add .
git -C "$DEMO_DIR" commit -qm "feat: initial auth and config services"
INITIAL_COMMIT="$(git -C "$DEMO_DIR" rev-parse HEAD)"
echo "Initial commit: $INITIAL_COMMIT"

# Step 2: Index with Real Neural Embeddings
echo ""
echo "[Step 2/8] Indexing repository with nomic-embed-text..."
"$BIN" --embedding nomic-embed-text --index "$DEMO_DIR" "$DEMO_DIR/index.ri"

if [[ ! -f "$DEMO_DIR/index.ri" ]]; then
    echo "ERROR: Index file not generated." >&2
    exit 1
fi
echo "Index saved successfully."

# Step 3: Neural/Hybrid Retrieval
echo ""
echo "[Step 3/8] Running hybrid search for 'verify_token_lifetime'..."
SEARCH_OUT="$("$BIN" --embedding nomic-embed-text --hybrid "$DEMO_DIR" "verify_token_lifetime")"
echo "$SEARCH_OUT"
if ! echo "$SEARCH_OUT" | grep -q "src/auth.rs:1-3"; then
    echo "ERROR: Expected 'src/auth.rs:1-3' in hybrid search results." >&2
    exit 1
fi
echo "✓ Hybrid retrieval found expected span."

# Step 4: Grounded Answer with Real LLM (qwen2.5-coder:1.5b)
echo ""
echo "[Step 4/8] Asking grounded question with real model (qwen2.5-coder:1.5b)..."
export USE_OLLAMA=1
export OLLAMA_MODEL="qwen2.5-coder:1.5b"
ANSWER_OUT="$("$BIN" --embedding nomic-embed-text --answer "$DEMO_DIR" "What is the token lifetime in seconds?")"
echo "$ANSWER_OUT"
if ! echo "$ANSWER_OUT" | grep -qE "(\[E1\]|\[src/auth.rs:1-3\])"; then
    echo "ERROR: Answer did not include verified citation." >&2
    exit 1
fi
echo "✓ Grounded answer verified with valid citation."

# Step 5: Exact Refusal on Unanswerable Query
echo ""
echo "[Step 5/8] Asking unanswerable question..."
REFUSAL_OUT="$("$BIN" --embedding nomic-embed-text --answer "$DEMO_DIR" "Where is the Stripe payment gateway key stored?")"
echo "$REFUSAL_OUT"
if ! echo "$REFUSAL_OUT" | grep -q "Insufficient repository evidence to answer this question."; then
    echo "ERROR: Expected exact refusal on unanswerable question." >&2
    exit 1
fi
echo "✓ Exact refusal verified."

# Step 6: Modify File Content & Verify Old Content Purged
echo ""
echo "[Step 6/8] Modifying src/auth.rs (renaming function & changing lifetime)..."
cat <<'EOF' > "$DEMO_DIR/src/auth.rs"
pub fn verify_extended_token_lifetime(issued_at: u64, now: u64) -> bool {
    now.saturating_sub(issued_at) < 7200
}
EOF

# Update index
"$BIN" --embedding nomic-embed-text --index "$DEMO_DIR" "$DEMO_DIR/index.ri"

# Verify new term is found
NEW_SEARCH="$("$BIN" --embedding nomic-embed-text --hybrid "$DEMO_DIR" "verify_extended_token_lifetime")"
echo "New search hits:"
echo "$NEW_SEARCH"
if ! echo "$NEW_SEARCH" | grep -q "verify_extended_token_lifetime"; then
    echo "ERROR: New function not found after modification." >&2
    exit 1
fi

# Verify old term is PURGED
OLD_SEARCH="$("$BIN" --embedding nomic-embed-text "$DEMO_DIR" "verify_token_lifetime")"
if echo "$OLD_SEARCH" | grep -q "verify_token_lifetime"; then
    echo "ERROR: Old term was NOT purged after modification." >&2
    exit 1
fi
echo "✓ Content modification verified: new term present, old term completely purged."

# Step 7: Delete File & Verify Purged from Search
echo ""
echo "[Step 7/8] Deleting src/config.rs..."
rm -f "$DEMO_DIR/src/config.rs"

# Update index
"$BIN" --embedding nomic-embed-text --index "$DEMO_DIR" "$DEMO_DIR/index.ri"

CONFIG_SEARCH="$("$BIN" --embedding nomic-embed-text "$DEMO_DIR" "database_host_cluster")"
if [[ -n "$CONFIG_SEARCH" ]]; then
    echo "ERROR: Deleted file content still returned in search: $CONFIG_SEARCH" >&2
    exit 1
fi
echo "✓ File deletion verified: deleted file completely purged from index."

# Step 8: Check Git Snapshot / Dirty State
echo ""
echo "[Step 8/8] Checking git snapshot state..."
ANALYTICS="$("$BIN" --analytics "$DEMO_DIR")"
echo "Analytics: $ANALYTICS"
if ! echo "$ANALYTICS" | grep -q '"files": 1'; then
    echo "ERROR: Expected 1 remaining file in index." >&2
    exit 1
fi
echo "✓ Snapshot analytics match remaining worktree."

echo ""
echo "============================================================"
echo " SUCCESS: All 8 lifecycle steps passed cleanly!"
echo "============================================================"
