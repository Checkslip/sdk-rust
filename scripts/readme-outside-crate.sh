#!/usr/bin/env bash
#
# Compile every Rust block of both READMEs the way a reader's project would.
#
# `src/lib.rs` pulls the READMEs into the crate docs with `include_str!`, so
# `cargo test` already compiles these blocks. That check is weaker than it
# looks: a doctest is compiled *inside* this crate, where `time`, `uuid` and
# `reqwest` are dependencies, and someone who ran `cargo add checkslip` has
# none of them.
#
# It cost us a published release to learn. Until 0.6.1 the README showed
#
#     use checkslip::time::macros::date;
#     date!(2026 - 11 - 01)
#
# and the test suite was green on every run, while that example failed in a
# reader's project with
#
#     error[E0433]: cannot find `time` in the list of imported crates
#
# because the macro expands to code naming `::time`, and re-exporting a crate
# does not re-export the path its own macros expand to.
#
# So the blocks are compiled a second time here, in a throwaway crate whose
# dependencies are exactly the ones the README tells a reader to install and
# nothing else. rustdoc is left to do the hidden `#` lines, the wrapping and
# the `?` handling, so none of those rules are re-implemented.
#
# Usage: scripts/readme-outside-crate.sh
# Exit 0 when every block compiles, 1 when one does not or when no block ran.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/src"
for name in README.md README.th.md; do
  if [ -f "$repo/$name" ]; then
    cp "$repo/$name" "$work/src/$name"
    printf '#![doc = include_str!("%s")]\n' "$name" >> "$work/src/lib.rs"
  fi
done

if [ ! -f "$work/src/lib.rs" ]; then
  echo "readme-outside-crate: neither README.md nor README.th.md is in $repo" >&2
  exit 1
fi

# The dependency list is the point of this check, so it is spelled out rather
# than inherited:
#   checkslip        with the two features the README demonstrates behind a cfg
#   tokio            README.md tells the reader to add it, with these features
#   futures-util     the README names it for TryStreamExt
# Nothing else belongs here. `time` and `uuid` are dependencies of this crate,
# not of its reader, and a block that needs one of them directly is the bug
# above coming back.
cat > "$work/Cargo.toml" <<EOF
[package]
name = "readme-outside-crate"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
checkslip = { path = "$repo", features = ["webhooks", "blocking"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"
EOF

echo "compiling the README blocks in $work"
if ! out="$(cd "$work" && cargo test --doc 2>&1)"; then
  echo "$out"
  echo
  echo "🔴 a README block needs something the README never asks the reader to install."
  echo "   It may still pass 'cargo test' in this crate, which compiles the same"
  echo "   block with this crate's own dependencies. Fix the README, not this check."
  exit 1
fi

ran="$(printf '%s\n' "$out" | sed -n 's/^test result: ok\. \([0-9]*\) passed.*/\1/p' \
  | awk '{ total += $1 } END { print total + 0 }')"
if [ "$ran" -eq 0 ]; then
  # A run that compiled nothing is not a pass: the doc attributes, the copied
  # files or the fences stopped matching, and a check that checks nothing must
  # not report success.
  echo "$out"
  echo
  echo "🔴 cargo ran no doctest at all, so this check proved nothing."
  exit 1
fi

echo "✅ $ran README block(s) compile with only what the README asks the reader to install"
