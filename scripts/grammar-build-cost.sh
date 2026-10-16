#!/usr/bin/env bash
# Clean build time of studio-ui with and without the syntax grammar crates.
#
# Each variant builds into its own empty target directory, so neither reuses the other's
# dependencies. The grammars are the `syntax-grammars` feature of studio-ui. Exits 1 when the
# build with grammars takes more than MAX_INCREASE_PERCENT (default 20) longer than the build
# without. The "without" build runs first, so a warm file cache favours the "with" build; the
# limit leaves room for that and for runner noise.
#
# Usage: scripts/grammar-build-cost.sh [work-dir]
set -euo pipefail

max_increase="${MAX_INCREASE_PERCENT:-20}"
work="${1:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}}"
manifest="$(cd "$(dirname "$0")/.." && pwd)/desktop/Cargo.toml"

# A dependency download is not build time.
cargo fetch --locked --manifest-path "$manifest"

timed_build() { # <name> [cargo flags...]; prints the elapsed seconds
  local name="$1"
  shift
  local dir="$work/grammar-cost-$name"
  rm -rf "$dir"
  local start end
  start="$(date +%s)"
  # set -e does not reach into the command substitution that runs this function (bash 3.2 on
  # macOS has no inherit_errexit), so a failed build returns explicitly and the caller stops.
  CARGO_TARGET_DIR="$dir" cargo build --locked --manifest-path "$manifest" -p studio-ui --lib "$@" >&2 ||
    { echo "::error::studio-ui build ($name) failed" >&2; return 1; }
  end="$(date +%s)"
  rm -rf "$dir"
  echo $((end - start))
}

without="$(timed_build without --no-default-features)"
with="$(timed_build with)"

# Integer percent, rounded to the nearest whole number.
increase=$(((with - without) * 100 / (without > 0 ? without : 1)))
summary="studio-ui clean build on ${RUNNER_OS:-$(uname -s)}: ${without}s without grammars, ${with}s with, ${increase}% more (limit ${max_increase}%)"
echo "$summary"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "$summary" >> "$GITHUB_STEP_SUMMARY"
fi

if [ "$increase" -gt "$max_increase" ]; then
  echo "::error::The syntax grammars add ${increase}% to the clean studio-ui build, over the ${max_increase}% limit"
  exit 1
fi
