#!/usr/bin/env bash
# Decides the next release version from conventional commits since the last v* tag.
#
#   bash scripts/release-version.sh [--bump auto|patch|minor|major] [--dry-run]
#
# Prints `version=`, `tag=` and `changed=` lines (ready for $GITHUB_OUTPUT). Unless --dry-run
# is given and a bump is needed, the version is written to Cargo.toml and Cargo.lock.
#
# Rules (auto): a `type!:` subject or a `BREAKING CHANGE:` footer bumps major, a `feat:` bumps
# minor, anything else bumps patch. With no tag yet, the version in Cargo.toml is released as-is.
set -euo pipefail

bump=auto
dry_run=false
while [ $# -gt 0 ]; do
  case "$1" in
    --bump) bump="$2"; shift 2 ;;
    --bump=*) bump="${1#*=}"; shift ;;
    --dry-run) dry_run=true; shift ;;
    -h|--help) sed -n '2,11p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$bump" in auto|patch|minor|major) ;; *) echo "invalid --bump: $bump" >&2; exit 2 ;; esac

cd "$(git rev-parse --show-toplevel)"

manifest_version=$(grep -m1 -E '^version = "[^"]+"' Cargo.toml | cut -d'"' -f2)
last_tag=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || true)

if [ -z "$last_tag" ]; then
  # First release: publish whatever Cargo.toml says.
  next="$manifest_version"
  kind=initial
  count=$(git rev-list --count HEAD)
else
  current="${last_tag#v}"
  range="$last_tag..HEAD"
  count=$(git rev-list --count "$range")
  if [ "$count" -eq 0 ]; then
    echo "No commits since $last_tag; nothing to release." >&2
    printf 'version=%s\ntag=%s\nchanged=false\n' "$current" "$last_tag"
    exit 0
  fi

  kind="$bump"
  if [ "$kind" = auto ]; then
    subjects=$(git log --format='%s' "$range")
    bodies=$(git log --format='%b' "$range")
    if grep -Eq '^[a-zA-Z]+(\([^)]*\))?!:' <<<"$subjects" || grep -Eq '^BREAKING[ -]CHANGE:' <<<"$bodies"; then
      kind=major
    elif grep -Eq '^feat(\([^)]*\))?:' <<<"$subjects"; then
      kind=minor
    else
      kind=patch
    fi
  fi

  IFS=. read -r major minor patch <<<"$current"
  case "$kind" in
    major) next="$((major + 1)).0.0" ;;
    minor) next="$major.$((minor + 1)).0" ;;
    patch) next="$major.$minor.$((patch + 1))" ;;
  esac
fi

echo "Release $next ($kind bump, $count commit(s) since ${last_tag:-the beginning})" >&2

if [ "$dry_run" = false ] && [ "$next" != "$manifest_version" ]; then
  sed -i -E "0,/^version = \"[^\"]+\"/s//version = \"$next\"/" Cargo.toml
  awk -v newver="$next" '
    /^name = "simple-converter"$/ { in_pkg = 1 }
    in_pkg && /^version = / { sub(/"[^"]*"/, "\"" newver "\""); in_pkg = 0 }
    { print }
  ' Cargo.lock > Cargo.lock.tmp && mv Cargo.lock.tmp Cargo.lock
fi

printf 'version=%s\ntag=v%s\nchanged=true\n' "$next" "$next"
