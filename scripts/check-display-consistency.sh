#!/usr/bin/env bash
#
# check-display-consistency.sh
#
# Detects standalone `*_to_string` functions that duplicate `fmt::Display`
# implementations. This enforces the convention documented in
# `src/main.rs`.
#
# Exit codes:
#   0 — No violations found (clean)
#   1 — Violations detected (see output for details)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SRC_DIR="$PROJECT_ROOT/src"

echo "Checking for standalone *_to_string functions that duplicate fmt::Display..."

# Find all standalone *_to_string function definitions
VIOLATIONS=$(grep -rn 'fn \(plan_status\|task_status\|task_status_value\|status\)_to_string' "$SRC_DIR/" 2>/dev/null || true)

if [ -z "$VIOLATIONS" ]; then
    echo "✅ PASS: No standalone *_to_string functions found."
    echo ""
    echo "Convention: Use fmt::Display for string conversion."
    echo "See src/main.rs for details."
    exit 0
fi

# Filter out false positives and track actual violations
VIOLATION_COUNT=0
echo "❌ FAIL: Found standalone *_to_string functions:"
echo ""
while IFS= read -r line; do
    # Skip if it looks like a method (contains 'self' or 'Self')
    if echo "$line" | grep -qE '(self|Self)'; then
        continue
    fi
    echo "  ⚠ $line"
    VIOLATION_COUNT=$((VIOLATION_COUNT + 1))
done <<< "$VIOLATIONS"

if [ "$VIOLATION_COUNT" -eq 0 ]; then
    echo ""
    echo "✅ PASS: All matches were filtered as false positives (methods or test helpers)."
    echo ""
    echo "Convention: Use fmt::Display for string conversion."
    echo "See src/main.rs for details."
    exit 0
fi

echo ""
echo "Convention violation: Standalone *_to_string functions duplicate fmt::Display."
echo "Fix: Implement fmt::Display on the type and use value.to_string() instead."
echo "See src/main.rs for the convention documentation."
echo ""
echo "To suppress this check, ensure the function is a method (takes self/Self)."

exit 1
