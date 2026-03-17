#!/usr/bin/env bash
set -euo pipefail

changes="$(git status --short --untracked-files=all -- src generated)"

if [[ -n "$changes" ]]; then
    echo "Generated content is out of date."
    echo "Run 'make sync-generated' after changing facts/."
    echo "If you only changed templates/, 'make build-pages' is enough."
    echo "Then commit src/ and generated/."
    echo
    printf '%s\n' "$changes"
    exit 1
fi
