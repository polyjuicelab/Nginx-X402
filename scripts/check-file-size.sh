#!/bin/bash
# Check that Rust source files don't exceed maximum line count
# This script enforces a maximum file size to maintain code readability
#
# Usage:
#   ./scripts/check-file-size.sh [max_lines] [directories...]
#   Example: ./scripts/check-file-size.sh 500 src tests

set -e

MAX_LINES="${1:-500}"
shift || true
DIRECTORIES="${@:-src tests}"

if [ -z "$DIRECTORIES" ]; then
    echo "Usage: $0 [max_lines] [directories...]"
    echo "Example: $0 500 src tests"
    exit 1
fi

echo "Checking Rust source files for maximum ${MAX_LINES} lines..."
echo "Directories: $DIRECTORIES"
echo ""

ERRORS=0
FILES_CHECKED=0

# Find all Rust source files
while IFS= read -r -d '' file; do
    FILES_CHECKED=$((FILES_CHECKED + 1))
    lines=$(wc -l < "$file" | tr -d ' ')
    
    if [ "$lines" -gt "$MAX_LINES" ]; then
        echo "❌ ERROR: $file exceeds ${MAX_LINES} lines (has ${lines} lines)"
        ERRORS=$((ERRORS + 1))
    fi
done < <(find $DIRECTORIES -name "*.rs" -type f -print0 2>/dev/null || true)

echo ""
echo "Checked ${FILES_CHECKED} files"

if [ $ERRORS -gt 0 ]; then
    echo ""
    echo "❌ Found ${ERRORS} file(s) exceeding ${MAX_LINES} lines"
    echo "Please split large files into smaller modules to maintain code readability"
    exit 1
else
    echo "✅ All files are within ${MAX_LINES} lines"
    exit 0
fi

