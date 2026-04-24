#!/usr/bin/env bash
# Refresh the files this skill vendored into the current repo, bringing them
# up to date with the canonical source under ~/.claude/scripts/version/ and
# ~/.claude/skills/version/presets/. Driven by scripts/version/.install-manifest.json.
#
# Classification per file:
#   up_to_date            recorded == local == canonical  (nothing to do)
#   upstream_changed      recorded == local != canonical  (safe overwrite)
#   user_modified         recorded != local == canonical  (local manually advanced; treat as up-to-date)
#   user_modified_and_drift  recorded != local != canonical  (conflict; needs --force)
#   missing_local         file deleted locally  (re-vendor)
#   missing_upstream      canonical file removed  (warn, leave local alone)
#   new_upstream          canonical script/lib not yet vendored  (vendor it)
#
# Usage:
#   update.sh [--dry-run] [--force] [--yes] [--include=A,B] [--exclude=C,D] [--no-commit]
#
# Never runs sync.sh, never creates tags, never edits VERSION / CHANGELOG /
# manifests. It only refreshes skill-owned vendored artifacts.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=./lib/common.sh
. "$SCRIPT_DIR/lib/common.sh"
# shellcheck source=./lib/install-manifest.sh
. "$SCRIPT_DIR/lib/install-manifest.sh"

dry_run="false"
force="false"
yes="false"
do_commit="true"
include_csv=""
exclude_csv=""

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run="true"; shift ;;
    --force)   force="true";   shift ;;
    --yes|-y)  yes="true";     shift ;;
    --no-commit) do_commit="false"; shift ;;
    --include=*) include_csv="${1#--include=}"; shift ;;
    --exclude=*) exclude_csv="${1#--exclude=}"; shift ;;
    -h|--help)
      sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) die "unknown arg: $1" ;;
  esac
done

is_repo_root || die "not inside a git repository" 30
require_cmd jq

MANIFEST="scripts/version/.install-manifest.json"
[ -f .version-preset ] || die "no .version-preset; run /version:install first" 10

if [ ! -f "$MANIFEST" ]; then
  cat >&2 <<MSG
[version] ERROR: no install manifest found at $MANIFEST.
         This repo was installed before /version:update existed. To create
         a baseline, re-run /version:install (or apply-preset.sh --force)
         on top of the existing setup and try again.
MSG
  exit 2
fi

preset_from_manifest="$(jq -r '.preset // ""' "$MANIFEST")"
recorded_skill_version="$(manifest_skill_version "$MANIFEST")"
current_skill_version="$(skill_version)"

# --- helpers ---------------------------------------------------------------

rewrite_manifest() {
  local state_preset
  state_preset="$(tr -d '[:space:]' < .version-preset)"
  local specs=()
  local entry path src sha abs_src

  while IFS= read -r entry; do
    path="$(jq -r '.path' <<<"$entry")"
    src="$(jq -r '.source' <<<"$entry")"
    if grep -qE "^missing_upstream	$(printf '%s' "$path" | sed 's/[.[\*^$()+?{|]/\\&/g')	" "$decisions_tsv" 2>/dev/null; then
      continue
    fi
    case "$src" in
      "~/"*) abs_src="$HOME/${src#"~/"}" ;;
      *)     abs_src="$src" ;;
    esac
    [ -f "$path" ] || continue
    sha="$(sha_or_empty "$path")"
    [ -z "$sha" ] && continue
    specs+=("$path	$abs_src	$sha")
  done < <(jq -c '.files[]?' "$MANIFEST")

  while IFS=$'\t' read -r cls local_path canonical _rec _loc _can; do
    case "$cls" in
      new_upstream|new_upstream_tracked_only)
        if ! manifest_has_file "$MANIFEST" "$local_path"; then
          [ -f "$local_path" ] || continue
          sha="$(sha_or_empty "$local_path")"
          [ -z "$sha" ] && continue
          specs+=("$local_path	$canonical	$sha")
        fi
        ;;
    esac
  done < "$decisions_tsv"

  local existing_installed_at
  existing_installed_at="$(jq -r '.installed_at // empty' "$MANIFEST")"
  if [ "${#specs[@]}" -gt 0 ]; then
    manifest_write "$MANIFEST" "$state_preset" "$existing_installed_at" "${specs[@]}"
  else
    manifest_write "$MANIFEST" "$state_preset" "$existing_installed_at"
  fi
  log "rewrote $MANIFEST (${#specs[@]} files tracked, skill_version $(skill_version))"
}

in_csv() {
  # in_csv <needle> <csv>  -> exit 0 if needle is one of csv entries.
  local needle="$1" csv="$2" tok
  [ -z "$csv" ] && return 1
  IFS=',' read -r -a _toks <<<"$csv"
  for tok in "${_toks[@]}"; do
    [ "$tok" = "$needle" ] && return 0
  done
  return 1
}

selected() {
  # Honor --include / --exclude. Exclude wins on overlap.
  local path="$1"
  if [ -n "$exclude_csv" ] && in_csv "$path" "$exclude_csv"; then
    return 1
  fi
  if [ -n "$include_csv" ]; then
    in_csv "$path" "$include_csv"
    return $?
  fi
  return 0
}

# Expand manifest-stored source ("~/..." portable form) back to absolute path.
resolve_source() {
  local s="$1"
  case "$s" in
    "~/"*) printf '%s/%s' "$HOME" "${s#"~/"}" ;;
    *)     printf '%s' "$s" ;;
  esac
}

sha_or_empty() {
  local p="$1"
  [ -f "$p" ] || { printf ''; return 0; }
  sha256_file "$p"
}

# Classify one file. Prints: "<classification>\t<local-path>\t<canonical-source>\t<recorded>\t<local>\t<canonical>".
classify_file() {
  local local_path="$1" canonical="$2" recorded_hash="$3"
  local local_hash canonical_hash cls
  local_hash="$(sha_or_empty "$local_path")"
  canonical_hash="$(sha_or_empty "$canonical")"

  if [ -z "$local_hash" ] && [ -z "$canonical_hash" ]; then
    cls="missing_both"
  elif [ -z "$local_hash" ]; then
    cls="missing_local"
  elif [ -z "$canonical_hash" ]; then
    cls="missing_upstream"
  elif [ "$local_hash" = "$canonical_hash" ]; then
    # Local matches canonical. Either truly pristine (recorded also matches)
    # or user hand-applied the upstream fix; either way, safe to treat as
    # up-to-date. Record-mismatch is resolved on the next manifest rewrite.
    cls="up_to_date"
  elif [ "$recorded_hash" = "$local_hash" ]; then
    # Local pristine, upstream moved; safe to overwrite.
    cls="upstream_changed"
  elif [ "$recorded_hash" = "$canonical_hash" ]; then
    # Upstream unchanged from baseline, user edited locally. Preserve without
    # requiring --force: there is no real conflict.
    cls="user_modified"
  else
    # Both sides moved from the baseline in different directions: true conflict.
    cls="user_modified_and_drift"
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$cls" "$local_path" "$canonical" "$recorded_hash" "$local_hash" "$canonical_hash"
}

# --- build the classification table ---------------------------------------

log "checking skill version: recorded $recorded_skill_version, canonical $current_skill_version"

decisions_tsv="$(mktemp)"
trap 'rm -f "$decisions_tsv"' EXIT

manifest_paths="$(manifest_list_paths "$MANIFEST")"

# Existing tracked files: classify each one.
while IFS= read -r local_path; do
  [ -z "$local_path" ] && continue
  if ! selected "$local_path"; then
    printf 'skipped\t%s\t\t\t\t\n' "$local_path" >> "$decisions_tsv"
    continue
  fi
  canonical="$(resolve_source "$(jq -r --arg p "$local_path" '.files[]? | select(.path == $p) | .source' "$MANIFEST")")"
  recorded="$(manifest_hash_for "$MANIFEST" "$local_path")"
  classify_file "$local_path" "$canonical" "$recorded" >> "$decisions_tsv"
done <<<"$manifest_paths"

# Detect new_upstream candidates: canonical scripts/libs that the installer
# would vendor today but are not yet tracked. Keep the candidate set tight
# (only scripts + lib + preset JSON + workflow) so we do not surface arbitrary
# files under ~/.claude.
preset_name="$(awk -F: '{print $1}' < .version-preset)"
mode="$(awk -F: '{print $2}' < .version-preset)"

declare -a candidate_locals=()
declare -a candidate_sources=()

add_candidate() {
  local local_path="$1" source_path="$2"
  candidate_locals+=("$local_path")
  candidate_sources+=("$source_path")
}

# Script candidates (mirror apply-preset.sh's top-level vendor list).
for f in current.sh sync.sh bump.sh release.sh changelog.sh apple-bump.sh adopt.sh update.sh; do
  add_candidate "scripts/version/$f" "$VERSION_SCRIPTS_ROOT/$f"
done
for f in common.sh manifest.sh install-manifest.sh commit-bump.sh should-release.sh existing-ci-detect.sh promote-specs.sh adopt-preflight.sh; do
  add_candidate "scripts/version/lib/$f" "$VERSION_SCRIPTS_ROOT/lib/$f"
done
if [ -n "$preset_name" ]; then
  add_candidate "scripts/version/presets/${preset_name}.json" "$PRESET_DIR/${preset_name}.json"
fi
if [ "$mode" = "auto" ]; then
  add_candidate ".github/workflows/auto-release-on-push.yml" "$VERSION_SKILL_ROOT/presets/workflows/auto-release-on-push.yml"
fi

for i in "${!candidate_locals[@]}"; do
  cand="${candidate_locals[$i]}"
  src="${candidate_sources[$i]}"
  if manifest_has_file "$MANIFEST" "$cand"; then
    continue
  fi
  # Only surface when the canonical exists (no point in suggesting ghosts).
  [ -f "$src" ] || continue
  if ! selected "$cand"; then
    continue
  fi
  canonical_hash="$(sha_or_empty "$src")"
  local_hash="$(sha_or_empty "$cand")"
  if [ -z "$local_hash" ]; then
    printf 'new_upstream\t%s\t%s\t\t\t%s\n' "$cand" "$src" "$canonical_hash" >> "$decisions_tsv"
  elif [ "$local_hash" = "$canonical_hash" ]; then
    # Already in sync but untracked; next manifest rewrite will add it.
    printf 'new_upstream_tracked_only\t%s\t%s\t\t%s\t%s\n' "$cand" "$src" "$local_hash" "$canonical_hash" >> "$decisions_tsv"
  else
    # Exists locally but differs and is untracked; treat as user_modified so
    # we do not silently overwrite a hand-installed variant.
    printf 'user_modified_and_drift\t%s\t%s\t\t%s\t%s\n' "$cand" "$src" "$local_hash" "$canonical_hash" >> "$decisions_tsv"
  fi
done

# --- summarize -------------------------------------------------------------

log "classification:"
printf '  %-26s %s\n' "STATE" "FILE" >&2
while IFS=$'\t' read -r cls path _src _rec _loc _can; do
  [ -z "$cls" ] && continue
  printf '  %-26s %s\n' "$cls" "$path" >&2
done < "$decisions_tsv"

total="$(wc -l < "$decisions_tsv" | tr -d ' ')"
upstream_changed="$(awk -F'\t' '$1=="upstream_changed"' "$decisions_tsv" | wc -l | tr -d ' ')"
user_mod_drift="$(awk -F'\t' '$1=="user_modified_and_drift"' "$decisions_tsv" | wc -l | tr -d ' ')"
user_mod="$(awk -F'\t' '$1=="user_modified"' "$decisions_tsv" | wc -l | tr -d ' ')"
missing_up="$(awk -F'\t' '$1=="missing_upstream"' "$decisions_tsv" | wc -l | tr -d ' ')"
missing_loc="$(awk -F'\t' '$1=="missing_local"' "$decisions_tsv" | wc -l | tr -d ' ')"
new_up="$(awk -F'\t' '$1=="new_upstream"' "$decisions_tsv" | wc -l | tr -d ' ')"
up_to_date="$(awk -F'\t' '$1=="up_to_date"' "$decisions_tsv" | wc -l | tr -d ' ')"

log "summary: $total entries ($up_to_date up_to_date, $upstream_changed upstream_changed, $user_mod user_modified, $user_mod_drift drift, $missing_up missing_upstream, $missing_loc missing_local, $new_up new_upstream)"

# --- apply -----------------------------------------------------------------

will_change=$((upstream_changed + missing_loc + new_up))
if [ "$user_mod_drift" -gt 0 ] && [ "$force" = "true" ]; then
  will_change=$((will_change + user_mod_drift))
fi

if [ "$will_change" -eq 0 ]; then
  if [ "$user_mod_drift" -gt 0 ]; then
    warn "$user_mod_drift file(s) have local and upstream changes; re-run with --force to overwrite"
  fi
  if [ "$missing_up" -gt 0 ]; then
    warn "$missing_up file(s) no longer exist upstream; leaving local copies alone"
  fi
  log "nothing to change"
  # Still rewrite manifest on non-dry-run to refresh skill_version + timestamps.
  if [ "$dry_run" = "false" ]; then
    rewrite_manifest
  fi
  exit 0
fi

if [ "$dry_run" = "true" ]; then
  log "dry-run: would change $will_change file(s); no writes, no commit"
  exit 0
fi

if [ "$yes" != "true" ] && [ -t 0 ]; then
  # Interactive terminal without --yes: proceed with a single visible line so
  # callers can still pipe in "y" if they need to. The slash command prompts
  # with AskUserQuestion before invoking; this is a last-resort safeguard.
  log "applying $will_change change(s) (--yes not passed, proceeding non-interactively; use --dry-run to inspect first)"
fi

touched=()
while IFS=$'\t' read -r cls local_path canonical _rec _loc _can; do
  [ -z "$cls" ] && continue
  case "$cls" in
    upstream_changed|missing_local|new_upstream)
      mkdir -p "$(dirname "$local_path")"
      cp "$canonical" "$local_path"
      case "$local_path" in
        *.sh) chmod +x "$local_path" 2>/dev/null || true ;;
      esac
      log "  updated: $local_path"
      touched+=("$local_path")
      ;;
    user_modified_and_drift)
      if [ "$force" = "true" ]; then
        mkdir -p "$(dirname "$local_path")"
        cp "$canonical" "$local_path"
        case "$local_path" in
          *.sh) chmod +x "$local_path" 2>/dev/null || true ;;
        esac
        log "  force-updated: $local_path"
        touched+=("$local_path")
      else
        warn "  skipped (user_modified_and_drift, no --force): $local_path"
      fi
      ;;
    user_modified)
      dbg "  leaving user_modified: $local_path"
      ;;
    up_to_date|new_upstream_tracked_only|skipped)
      :
      ;;
    missing_upstream)
      warn "  missing upstream: $local_path (canonical gone; leaving local copy)"
      ;;
  esac
done < "$decisions_tsv"

# --- rewrite manifest ------------------------------------------------------

rewrite_manifest

# --- commit ----------------------------------------------------------------

if [ "$do_commit" = "true" ] && [ "${#touched[@]}" -gt 0 ]; then
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    warn "not a git worktree; skipping commit"
    exit 0
  fi
  for p in "${touched[@]}"; do
    git add -- "$p" 2>/dev/null || true
  done
  git add "$MANIFEST" 2>/dev/null || true
  if git diff --cached --quiet; then
    warn "nothing staged to commit"
  else
    git commit -m "chore(version): update vendored skill artifacts (${#touched[@]} files refreshed)" >/dev/null
    log "committed update"
  fi
fi

log "done"
