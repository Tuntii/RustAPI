#!/usr/bin/env bash
set -euo pipefail

base_sha="${BASE_SHA:?BASE_SHA is required}"
head_sha="${HEAD_SHA:?HEAD_SHA is required}"

changed_snapshots="$(
  git diff --name-only "$base_sha" "$head_sha" -- \
    api/public/rustapi-rs.default.txt \
    api/public/rustapi-rs.all-features.txt
)"

if [[ -z "$changed_snapshots" ]]; then
  echo "No public API snapshot changes detected."
  exit 0
fi

labels=",${PR_LABELS:-},"
if [[ "$labels" == *",breaking,"* || "$labels" == *",feature,"* ]]; then
  echo "Public API snapshots changed and required label is present."
  exit 0
fi

echo "::error::Public API snapshots changed but PR is missing required label: breaking or feature."
echo "Changed snapshot files:"
echo "$changed_snapshots"
exit 1
