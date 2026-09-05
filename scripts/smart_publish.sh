#!/usr/bin/env bash
# smart_publish.sh — bash port of scripts/smart_publish.ps1
# Usage: cargo login once, then ./scripts/smart_publish.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CRATES=(
  rustapi-macros
  rustapi-validate
  rustapi-openapi
  rustapi-core
  rustapi-testing
  rustapi-extras
  rustapi-toon
  rustapi-ws
  rustapi-view
  rustapi-grpc
  rustapi-mcp
  rustapi-rs
  cargo-rustapi
)

workspace_version() {
  awk '
    /^\[workspace\.package\]/ { in_wp=1; next }
    in_wp && /^\[/ { in_wp=0 }
    in_wp && /^version[[:space:]]*=/ {
      if (match($0, /"[^"]+"/)) { print substr($0, RSTART+1, RLENGTH-2); exit }
    }
  ' Cargo.toml
}

local_version() {
  local path="$1"
  if grep -qE 'version\.workspace[[:space:]]*=[[:space:]]*true' "$path/Cargo.toml" 2>/dev/null; then
    workspace_version
  else
    awk '
      /^version[[:space:]]*=/ {
        if (match($0, /"[^"]+"/)) { print substr($0, RSTART+1, RLENGTH-2); exit }
      }
    ' "$path/Cargo.toml"
  fi
}

remote_version() {
  local name="$1"
  # Prefer exact version presence over max_version when checking skip
  local max
  max=$(curl -sL -A 'rustapi-smart-publish/1.0' "https://crates.io/api/v1/crates/${name}" \
    | python3 -c "import sys,json
try:
 d=json.load(sys.stdin)
 print(d.get('crate',{}).get('max_version','') or '')
except Exception:
 print('')" 2>/dev/null || true)
  echo "$max"
}

remote_has_version() {
  local name="$1" ver="$2"
  curl -sL -A 'rustapi-smart-publish/1.0' -o /dev/null -w '%{http_code}' \
    "https://crates.io/api/v1/crates/${name}/${ver}" | grep -q '^200$'
}

LOCAL_WS=$(workspace_version)
echo "Using workspace SemVer from Cargo.toml: $LOCAL_WS"
if [[ ! -f "$HOME/.cargo/credentials.toml" && ! -f "$HOME/.cargo/credentials" && -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "ERROR: no crates.io token. Run cargo login first." >&2
  exit 1
fi
echo "Starting Smart Publish Process..."

strip_dev_dep_line() {
  # $1 path, $2 crate name prefix to comment out (e.g. rustapi-core)
  local path="$1" dep="$2"
  sed -i -E "s/^(${dep}[[:space:]]*=[[:space:]]*\{.*)$/# STRIPPED FOR PUBLISH: \1/" "$path/Cargo.toml"
}

for name in "${CRATES[@]}"; do
  path="crates/${name}"
  local_ver=$(local_version "$path")
  remote_ver=$(remote_version "$name")

  if remote_has_version "$name" "$local_ver"; then
    echo "Checking $name... Local: $local_ver | Remote: has $local_ver [SKIP]"
    continue
  fi

  printf 'Checking %s... Local: %s | Remote: %s' "$name" "$local_ver" "${remote_ver:-none}"
  if [[ -z "${remote_ver}" ]]; then
    echo " [NEW]"
  else
    echo " [UPDATE] $remote_ver -> $local_ver"
  fi

  backup=$(mktemp)
  cp "$path/Cargo.toml" "$backup"

  # Strip reverse/circular / later-crate deps so cargo can package without index hits.
  case "$name" in
    rustapi-validate)
      strip_dev_dep_line "$path" rustapi-core
      echo "   [HACK] stripped rustapi-core dev-dep"
      ;;
    rustapi-core)
      strip_dev_dep_line "$path" rustapi-testing
      echo "   [HACK] stripped rustapi-testing dev-dep"
      ;;
    rustapi-mcp)
      strip_dev_dep_line "$path" rustapi-rs
      echo "   [HACK] stripped rustapi-rs dev-dep"
      ;;
  esac

  echo "   Publishing $local_ver..."
  set +e
  cargo publish -p "$name" --allow-dirty --no-verify
  rc=$?
  set -e
  cp "$backup" "$path/Cargo.toml"
  rm -f "$backup"
  echo "   [HACK] restored manifest (if modified)"

  if [[ $rc -ne 0 ]]; then
    echo "ERROR: cargo publish failed for $name (exit $rc)" >&2
    exit $rc
  fi
  echo "   Waiting 10s for index propagation..."
  sleep 10
done

echo "Smart Publish Completed!"
echo "Optional: git tag -a v${LOCAL_WS} -m \"v${LOCAL_WS}\" && git push origin v${LOCAL_WS}"
