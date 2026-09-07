#!/usr/bin/env bash
# Exercises release-version.sh against throwaway git repositories.
set -euo pipefail

script="$(cd "$(dirname "$0")" && pwd)/release-version.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0

assert_eq() { # actual expected description
  if [ "$1" == "$2" ]; then
    echo "ok   - $3"
  else
    echo "FAIL - $3: expected '$2', got '$1'"
    failures=$((failures + 1))
  fi
}

new_repo() { # name -> prints path
  local dir="$tmp/$1"
  mkdir -p "$dir"
  git -C "$dir" init -q -b main
  git -C "$dir" config user.email test@example.com
  git -C "$dir" config user.name test
  git -C "$dir" config commit.gpgsign false
  cat > "$dir/Cargo.toml" <<'TOML'
[package]
name = "simple-converter"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1.0.200", features = ["derive"] }
TOML
  cat > "$dir/Cargo.lock" <<'LOCK'
version = 4

[[package]]
name = "serde"
version = "1.0.200"

[[package]]
name = "simple-converter"
version = "0.1.0"
dependencies = [
 "serde",
]
LOCK
  git -C "$dir" add -A
  git -C "$dir" commit -qm "chore: initial commit"
  echo "$dir"
}

commit() { # dir message [body]
  echo "$RANDOM" >> "$1/file.txt"
  git -C "$1" add -A
  if [ $# -ge 3 ]; then
    git -C "$1" commit -qm "$2" -m "$3"
  else
    git -C "$1" commit -qm "$2"
  fi
}

run() { # dir args... -> stdout of the script
  (cd "$1" && bash "$script" "${@:2}" 2>/dev/null)
}

field() { grep "^$2=" <<<"$1" | cut -d= -f2-; }

repo=$(new_repo initial)
out=$(run "$repo" --dry-run)
assert_eq "$(field "$out" version)" "0.1.0" "first release uses the Cargo.toml version"
assert_eq "$(field "$out" tag)" "v0.1.0" "first release tag"
assert_eq "$(field "$out" changed)" "true" "first release is a change"

repo=$(new_repo patch)
git -C "$repo" tag v0.1.0
commit "$repo" "fix: something"
commit "$repo" "docs: readme"
assert_eq "$(field "$(run "$repo" --dry-run)" version)" "0.1.1" "fix bumps patch"

repo=$(new_repo minor)
git -C "$repo" tag v0.1.0
commit "$repo" "fix: a"
commit "$repo" "feat(ui): b"
assert_eq "$(field "$(run "$repo" --dry-run)" version)" "0.2.0" "feat bumps minor"

repo=$(new_repo major-bang)
git -C "$repo" tag v0.1.0
commit "$repo" "feat!: drop old flag"
assert_eq "$(field "$(run "$repo" --dry-run)" version)" "1.0.0" "type! bumps major"

repo=$(new_repo major-footer)
git -C "$repo" tag v0.1.0
commit "$repo" "refactor: rework config" "BREAKING CHANGE: config file moved"
assert_eq "$(field "$(run "$repo" --dry-run)" version)" "1.0.0" "BREAKING CHANGE footer bumps major"

repo=$(new_repo nothing)
git -C "$repo" tag v0.1.0
out=$(run "$repo" --dry-run)
assert_eq "$(field "$out" changed)" "false" "no commits since tag means no release"
assert_eq "$(field "$out" version)" "0.1.0" "version unchanged without commits"

repo=$(new_repo override)
git -C "$repo" tag v0.1.0
commit "$repo" "fix: tiny"
assert_eq "$(field "$(run "$repo" --dry-run --bump minor)" version)" "0.2.0" "--bump overrides auto"
assert_eq "$(field "$(run "$repo" --dry-run --bump=major)" version)" "1.0.0" "--bump=major override"

repo=$(new_repo writes)
git -C "$repo" tag v0.1.0
commit "$repo" "feat: write files"
out=$(run "$repo")
assert_eq "$(field "$out" version)" "0.2.0" "write mode reports version"
assert_eq "$(grep -m1 '^version = ' "$repo/Cargo.toml")" 'version = "0.2.0"' "Cargo.toml package version updated"
assert_eq "$(grep -c 'version = "1.0.200"' "$repo/Cargo.toml" "$repo/Cargo.lock" | tr '\n' ' ')" "$repo/Cargo.toml:1 $repo/Cargo.lock:1 " "dependency versions untouched"
assert_eq "$(awk '/^name = "simple-converter"/{f=1} f && /^version/{print; exit}' "$repo/Cargo.lock")" 'version = "0.2.0"' "Cargo.lock package entry updated"

if [ "$failures" -ne 0 ]; then
  echo "$failures assertion(s) failed"
  exit 1
fi
echo "all release-version tests passed"
