#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/release.sh <patch|minor|major|release|rc|beta|alpha|VERSION> [options] [-- <cargo-release args...>]

Options:
  --execute         Actually perform the release (default is dry-run)
  --skip-checks     Skip fmt/clippy/test/dist preflight checks
  -y, --yes         Skip confirmation prompt
  -e, --editor      Open $EDITOR to compose release notes; the result is
                    written under '## [Unreleased]' in CHANGELOG.md
  -m, --message MSG Use MSG as the release notes (written under
                    '## [Unreleased]' in CHANGELOG.md)
  -h, --help        Show this help

Notes source:
  By default, the script uses whatever is already under '## [Unreleased]'
  in CHANGELOG.md. --editor and --message override that for one-off notes
  written at release time.

Examples:
  scripts/release.sh patch
  scripts/release.sh minor --execute
  scripts/release.sh patch --editor --execute
  scripts/release.sh patch --message "fix flake build" --execute
  scripts/release.sh 0.2.0 --execute -- --no-verify
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

# Extract the body of the "## [Unreleased]" section from CHANGELOG.md
# (everything between that heading and the next "## [...]" heading).
extract_unreleased_notes() {
  awk '
    /^## \[Unreleased\]/ { found = 1; next }
    found && /^## \[/   { exit }
    found               { print }
  ' CHANGELOG.md
}

# Replace the body of "## [Unreleased]" in CHANGELOG.md with the given notes.
# The heading itself is preserved; everything between it and the next
# "## [...]" heading is replaced with: blank line, notes, blank line.
write_unreleased_notes() {
  local notes="$1"
  local heading_line next_line tmp
  heading_line="$(grep -n '^## \[Unreleased\]' CHANGELOG.md | head -1 | cut -d: -f1)"
  [[ -n "$heading_line" ]] || die "CHANGELOG.md is missing the '## [Unreleased]' heading"

  next_line="$(awk -v start="$heading_line" 'NR > start && /^## \[/ { print NR; exit }' CHANGELOG.md)"

  tmp="$(mktemp)"
  head -n "$heading_line" CHANGELOG.md > "$tmp"
  printf '\n%s\n\n' "$notes" >> "$tmp"
  if [[ -n "$next_line" ]]; then
    tail -n "+$next_line" CHANGELOG.md >> "$tmp"
  fi
  mv "$tmp" CHANGELOG.md
}

# Open $EDITOR with a template; return the user-edited content (without
# the leading instruction comment lines).
compose_notes_in_editor() {
  local editor="${EDITOR:-${VISUAL:-vi}}"
  local tmp
  tmp="$(mktemp -t sqs-release-notes.XXXXXX.md)"
  cat > "$tmp" <<'EOF'
# Write release notes below. Lines starting with '#' (alone) are stripped.
# A blank file aborts the release.
#
# Suggested format:
#
# ### Changed
# - ...
#
# ### Removed
# - ...

EOF
  "$editor" "$tmp" </dev/tty >/dev/tty 2>&1 || die "editor exited non-zero"
  # Strip lines that start with '# ' or are exactly '#' (instructions).
  local content
  content="$(grep -vE '^#( |$)' "$tmp" || true)"
  rm -f "$tmp"
  printf '%s' "$content"
}

if [[ $# -eq 0 ]]; then
  usage
  exit 1
fi

level_or_version=""
execute=false
skip_checks=false
assume_yes=false
notes_mode="changelog"   # changelog | editor | message
notes_message=""
extra_args=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --execute)
      execute=true
      shift
      ;;
    --skip-checks)
      skip_checks=true
      shift
      ;;
    -y|--yes)
      assume_yes=true
      shift
      ;;
    -e|--editor)
      [[ "$notes_mode" == "changelog" ]] || die "--editor and --message are mutually exclusive"
      notes_mode="editor"
      shift
      ;;
    -m|--message)
      [[ "$notes_mode" == "changelog" ]] || die "--editor and --message are mutually exclusive"
      [[ $# -ge 2 ]] || die "--message requires a value"
      notes_mode="message"
      notes_message="$2"
      shift 2
      ;;
    --)
      shift
      extra_args=("$@")
      break
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      if [[ -n "$level_or_version" ]]; then
        die "multiple release levels/versions provided: '$level_or_version' and '$1'"
      fi
      level_or_version="$1"
      shift
      ;;
  esac
done

[[ -n "$level_or_version" ]] || die "missing release level/version"

need_cmd git
need_cmd cargo
need_cmd dist
need_cmd cargo-release

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || die "not inside a git repository"
cd "$repo_root"

branch="$(git branch --show-current)"
[[ "$branch" == "main" ]] || die "releases must be cut from 'main' (current: '${branch:-detached}')"

if [[ -n "$(git status --porcelain)" ]]; then
  die "working tree is not clean; commit or stash changes before releasing"
fi

if [[ ! -f "release.toml" ]]; then
  die "release.toml not found at repo root"
fi

[[ -f "CHANGELOG.md" ]] || die "CHANGELOG.md not found at repo root"

case "$notes_mode" in
  editor)
    notes_message="$(compose_notes_in_editor)"
    [[ -n "$(printf '%s\n' "$notes_message" | sed -e '/^[[:space:]]*$/d')" ]] \
      || die "no release notes provided; aborting"
    write_unreleased_notes "$notes_message"
    echo "Wrote release notes from \$EDITOR into CHANGELOG.md [Unreleased]."
    ;;
  message)
    [[ -n "$(printf '%s' "$notes_message" | sed -e '/^[[:space:]]*$/d')" ]] \
      || die "--message must contain non-whitespace release notes; refusing to release"
    write_unreleased_notes "$notes_message"
    echo "Wrote --message into CHANGELOG.md [Unreleased]."
    ;;
  changelog)
    : # use existing content
    ;;
esac

unreleased_notes="$(extract_unreleased_notes)"
unreleased_trimmed="$(printf '%s\n' "$unreleased_notes" | sed -e '/^[[:space:]]*$/d')"
if [[ -z "$unreleased_trimmed" ]]; then
  cat >&2 <<EOF
error: CHANGELOG.md has no content under '## [Unreleased]'.

Refusing to release without notes. Provide them in one of three ways:
  1. Write notes under '## [Unreleased]' in CHANGELOG.md, then re-run.
  2. Re-run with --editor to compose notes in \$EDITOR.
  3. Re-run with --message "..." to pass notes inline.
EOF
  exit 1
fi

if [[ "$skip_checks" == false ]]; then
  echo "Running preflight checks..."
  cargo fmt --check
  cargo clippy -- -D warnings
  cargo test
  dist plan
fi

echo
echo "Release target: $level_or_version"
echo "Mode: $([[ "$execute" == true ]] && echo "execute" || echo "dry-run")"
echo "Notes source: $notes_mode"
echo "Config: release.toml"
echo
echo "Release notes (from CHANGELOG.md [Unreleased]):"
echo "----------------------------------------------"
printf '%s\n' "$unreleased_notes"
echo "----------------------------------------------"
echo

if [[ "$notes_mode" != "changelog" && "$execute" != true ]]; then
  echo "Note: --editor / --message wrote to CHANGELOG.md, but --execute was"
  echo "      not passed. The script will run cargo-release in dry-run mode."
  echo "      If you abort or the dry-run fails, run 'git checkout CHANGELOG.md'"
  echo "      to discard the staged notes."
  echo
fi

if [[ "$assume_yes" == false ]]; then
  read -r -p "Continue with cargo-release? [y/N] " reply
  case "$reply" in
    y|Y|yes|YES) ;;
    *) echo "Aborted."; exit 1 ;;
  esac
fi

cmd=(cargo release "$level_or_version" -c release.toml --no-confirm)
if [[ "$execute" == true ]]; then
  cmd+=(--execute)
fi
if [[ ${#extra_args[@]} -gt 0 ]]; then
  cmd+=("${extra_args[@]}")
fi

printf 'Running:'
printf ' %q' "${cmd[@]}"
printf '\n'

"${cmd[@]}"
