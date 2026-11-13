#!/bin/sh
# The review methodology. Static reference and auth checks, format, clippy, then tests.
# Green suite must pass. The three FR tests must still fail on NotImplemented.
set -eu
cd "$(dirname "$0")/.."
sh scripts/verify-toolchain.sh
sh scripts/ci-checks.sh
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline --lib --tests -- --skip fr1_ --skip fr2_ --skip fr3_
set +e
fr=$(cargo test --offline --test radio_fr -- --test-threads=8 2>&1)
code=$?
set -e
if [ "$code" -eq 0 ]; then
  echo "FR suite went green. That requires a requirement status change." >&2
  exit 1
fi
printf '%s\n' "$fr" | grep -q 'NotImplemented' || {
  echo "FR suite failed for a reason other than NotImplemented" >&2
  printf '%s\n' "$fr" >&2
  exit 1
}
fr_failed=$(printf '%s\n' "$fr" | grep -c '^test fr' || true)
echo "metrics: fmt=clean clippy=denied-warnings fr_failed=${fr_failed} fr_reason=NotImplemented"
echo "workspace verified: green suite passed, FR-1 FR-2 FR-3 still red"
