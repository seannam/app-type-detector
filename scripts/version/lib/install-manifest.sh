#!/usr/bin/env bash
# Read/write helpers for scripts/version/.install-manifest.json.
#
# The manifest records a SHA256 baseline for every file the version skill
# vendored into a repo, so /version:update can distinguish:
#   - "file is pristine since install" (safe to overwrite)
#   - "user modified this file on purpose" (do not silently overwrite)
#
# Shape is governed by ~/.claude/skills/version/presets/_install-manifest.schema.json.
#
# Public functions:
#   manifest_write <path> <preset-string> <installed-at> [<file-tsv>...]
#   manifest_read <path>
#   manifest_has_file <path> <local-path>
#   manifest_hash_for <path> <local-path>
#   manifest_source_for <path> <local-path>
#   manifest_list_paths <path>
#   manifest_skill_version <path>
#
# File-tsv format: "<local-path>\t<canonical-source>\t<sha256>". One per arg.
# installed_at: ISO 8601 string for the original install time; empty preserves
# existing value when the manifest already exists.
#
# Source-only; do not execute.

# shellcheck disable=SC2155

set -euo pipefail

_install_manifest_require_jq() {
  command -v jq >/dev/null 2>&1 || {
    printf '[version] ERROR: jq is required for install-manifest.sh\n' >&2
    exit 11
  }
}

# Strip $HOME from a path so the manifest is portable across machines.
_install_manifest_portable_source() {
  local p="$1"
  case "$p" in
    "$HOME"/*) printf '~/%s' "${p#"$HOME"/}" ;;
    *)         printf '%s' "$p" ;;
  esac
}

_install_manifest_expand() {
  local p="$1"
  case "$p" in
    "~/"*) printf '%s/%s' "$HOME" "${p#"~/"}" ;;
    *)     printf '%s' "$p" ;;
  esac
}

# Write the manifest JSON atomically.
#   $1 manifest path
#   $2 preset string (e.g. "node:auto::.")
#   $3 installed_at (ISO 8601; empty to preserve existing / use now)
#   $4..$N file-tsv lines
manifest_write() {
  _install_manifest_require_jq
  local out="$1"; shift
  local preset="$1"; shift
  local installed_at="$1"; shift
  local now
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  # Preserve installed_at across re-writes when caller passes empty.
  if [ -z "$installed_at" ]; then
    if [ -f "$out" ]; then
      installed_at="$(jq -r '.installed_at // empty' "$out" 2>/dev/null || true)"
    fi
    [ -z "$installed_at" ] && installed_at="$now"
  fi

  # Build files[] from TSV args via jq to sidestep quoting pitfalls.
  local files_json="[]"
  local spec local_path canonical sha installed per_installed
  for spec in "$@"; do
    # Parse TSV
    IFS=$'\t' read -r local_path canonical sha <<<"$spec"
    [ -z "$local_path" ] && continue
    [ -z "$sha" ] && continue
    canonical="$(_install_manifest_portable_source "$canonical")"
    # Preserve per-file installed_at if already present.
    per_installed=""
    if [ -f "$out" ]; then
      per_installed="$(jq -r --arg p "$local_path" '.files[]? | select(.path == $p) | .installed_at' "$out" 2>/dev/null || true)"
    fi
    [ -z "$per_installed" ] && per_installed="$installed_at"

    files_json="$(jq \
      --arg path "$local_path" \
      --arg source "$canonical" \
      --arg sha "$sha" \
      --arg installed "$per_installed" \
      --arg updated "$now" \
      '. + [{path: $path, source: $source, sha256: $sha, installed_at: $installed, updated_at: $updated}]' \
      <<<"$files_json")"
  done

  local skill_ver
  if [ -f "$VERSION_SKILL_ROOT/VERSION" ]; then
    skill_ver="$(tr -d '[:space:]' < "$VERSION_SKILL_ROOT/VERSION")"
  else
    skill_ver="0.0.0"
  fi

  local tmp
  tmp="$(mktemp)"
  jq -n \
    --argjson files "$files_json" \
    --arg skill_version "$skill_ver" \
    --arg installed_at "$installed_at" \
    --arg last_updated_at "$now" \
    --arg preset "$preset" \
    '{schema_version: 1, skill_version: $skill_version, installed_at: $installed_at, last_updated_at: $last_updated_at, preset: $preset, files: $files}' \
    > "$tmp"
  mkdir -p "$(dirname "$out")"
  mv "$tmp" "$out"
}

manifest_read() {
  local path="$1"
  [ -f "$path" ] || return 0
  cat "$path"
}

manifest_has_file() {
  _install_manifest_require_jq
  local path="$1" local_path="$2"
  [ -f "$path" ] || return 1
  jq -e --arg p "$local_path" '.files[]? | select(.path == $p)' "$path" >/dev/null 2>&1
}

manifest_hash_for() {
  _install_manifest_require_jq
  local path="$1" local_path="$2"
  [ -f "$path" ] || return 1
  jq -r --arg p "$local_path" '.files[]? | select(.path == $p) | .sha256' "$path"
}

manifest_source_for() {
  _install_manifest_require_jq
  local path="$1" local_path="$2"
  [ -f "$path" ] || return 1
  local src
  src="$(jq -r --arg p "$local_path" '.files[]? | select(.path == $p) | .source' "$path")"
  _install_manifest_expand "$src"
}

manifest_list_paths() {
  _install_manifest_require_jq
  local path="$1"
  [ -f "$path" ] || return 0
  jq -r '.files[]?.path' "$path"
}

manifest_skill_version() {
  _install_manifest_require_jq
  local path="$1"
  [ -f "$path" ] || { printf '0.0.0'; return 0; }
  jq -r '.skill_version // "0.0.0"' "$path"
}
