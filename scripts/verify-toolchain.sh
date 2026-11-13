#!/bin/sh
# Confirm the compiler matches the recorded floor. Does not bump it.
set -eu
cd "$(dirname "$0")/.."
floor=$(sed -n 's/^toolchain_floor: "\(.*\)"/\1/p' requirements/TOOLCHAIN.md | head -n 1)
if [ -z "$floor" ]; then
  echo "toolchain floor missing from requirements/TOOLCHAIN.md" >&2
  exit 1
fi
got=$(rustc -V)
case "$got" in
  "rustc ${floor} "*) ;;
  *)
    echo "toolchain is not gucci: floor ${floor}, got ${got}" >&2
    echo "read https://blog.rust-lang.org/releases/latest before changing the floor" >&2
    exit 1
    ;;
esac
echo "gucci: ${got}"
echo "next: read https://blog.rust-lang.org/releases/latest and requirements/TOOLCHAIN.md"
