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

# Collect labels from env first, then fall back to GitHub event payload.
labels_csv="${PR_LABELS:-}"
if [[ -z "$labels_csv" && -n "${GITHUB_EVENT_PATH:-}" && -f "${GITHUB_EVENT_PATH}" ]]; then
  if command -v jq >/dev/null 2>&1; then
    labels_csv="$(jq -r '.pull_request.labels[].name' "$GITHUB_EVENT_PATH" 2>/dev/null | paste -sd ',' -)"
  fi
fi

labels_normalized=",$(echo "$labels_csv" | tr '[:upper:]' '[:lower:]'),"
echo "Detected PR labels: ${labels_csv:-<none>}"

if [[ "$labels_normalized" == *",breaking,"* || "$labels_normalized" == *",feature,"* ]]; then
  echo "Public API snapshots changed and required label is present."
  exit 0
fi

echo "::error::Public API snapshots changed but PR is missing required label: breaking or feature."
echo "Changed snapshot files:"
echo "$changed_snapshots"
exit 1
