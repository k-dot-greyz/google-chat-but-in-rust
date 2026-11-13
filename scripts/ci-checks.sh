#!/bin/sh
# Static checks for the reference index, the empty crate list, and the write gate.
# Prints the metrics a review should be able to read without opening the diff.
set -eu
cd "$(dirname "$0")/.."

fail() {
  echo "ci: $1" >&2
  exit 1
}

dep_lines=$(awk '
  /^\[dependencies\]/ { on = 1; next }
  /^\[/ { on = 0 }
  on && $0 !~ /^[[:space:]]*$/ && $0 !~ /^[[:space:]]*#/ { count++ }
  END { print count + 0 }
' Cargo.toml)
[ "$dep_lines" -eq 0 ] || fail "Cargo.toml dependencies must stay empty (found ${dep_lines})"
if grep -E -q 'serde|sqlx|rusqlite|nutype|schemars' Cargo.toml; then
  fail "a refused crate is named in Cargo.toml"
fi

profile_ids=$(grep -c '"id":' modules/profiles.json)
[ "$profile_ids" -eq 7 ] || fail "expected 7 profile ids, found ${profile_ids}"

admits=$(grep -c 'admit_request(' src/hull.rs)
[ "$admits" -ge 4 ] || fail "create, patch, delete, and push must call admit_request"

for test_name in \
  a_write_without_a_profile_is_refused \
  a_bearer_header_is_not_a_profile \
  a_profile_in_the_query_is_refused \
  radio_cannot_push_the_shell_pipe_or_edit_cards \
  two_profile_headers_are_not_one_profile \
  shell_still_pushes_its_own_pipes
do
  grep -q "fn ${test_name}" tests/gate.rs || fail "missing auth test ${test_name}"
done

for path in \
  docs/INDEX.md docs/LOG.md docs/BIBLE.md docs/CI.md \
  docs/SITREP.md docs/META.md \
  docs/WHITEPAPER.md docs/ARCHITECTURE.md docs/FLOWS.md \
  docs/PLUGINS.md docs/INTEGRATIONS.md \
  SPEC.md AGENTS.md CONTRIBUTING.md CODEOWNERS \
  requirements/TOOLCHAIN.md requirements/FR-GATE.md \
  requirements/FR-1.md requirements/FR-2.md requirements/FR-3.md \
  requirements/FR-SESSION.md \
  modules/profiles.json modules/policy.json \
  src/gate.rs src/airlock.rs src/schema.rs src/radio.rs \
  tests/gate.rs tests/flow.rs tests/radio_factory.rs tests/radio_fr.rs \
  .github/workflows/verify.yml .github/workflows/codeql.yml
do
  [ -f "$path" ] || fail "missing ${path}"
done

for path in \
  docs/INDEX.md docs/LOG.md docs/BIBLE.md docs/CI.md \
  docs/SITREP.md docs/META.md \
  docs/WHITEPAPER.md docs/ARCHITECTURE.md docs/FLOWS.md \
  docs/PLUGINS.md docs/INTEGRATIONS.md \
  SPEC.md AGENTS.md CONTRIBUTING.md \
  requirements/TOOLCHAIN.md requirements/FR-GATE.md \
  requirements/FR-1.md requirements/FR-2.md requirements/FR-3.md requirements/FR-SESSION.md
do
  head -n 16 "$path" | grep -q '^status:' || fail "${path} has no status"
  head -n 16 "$path" | grep -q '^index: docs/INDEX.md' || fail "${path} is not wired to the index"
done

head -n 16 docs/LOG.md | grep -q '^append_only: true' || fail "LOG.md must stay append-only"
head -n 16 docs/INDEX.md | grep -q '^status: reference' || fail "INDEX.md must stay reference"

python3 - <<'PY'
from pathlib import Path
import re
import sys
idx = Path("docs/INDEX.md")
missing = []
for link in re.findall(r"\]\(([^)]+)\)", idx.read_text()):
    path = link.split("#", 1)[0]
    if not path or path.startswith("http"):
        continue
    if not (idx.parent / path).resolve().exists():
        missing.append(path)
if missing:
    sys.exit("index links missing: " + ", ".join(missing))
PY

echo "metrics: deps=0 profiles=${profile_ids} admit_sites=${admits} auth_tests=6 index=linked"
