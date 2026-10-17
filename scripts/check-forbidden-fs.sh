#!/usr/bin/env sh
# Fails if any removal API, the rename API, or the clippy opt-out appears
# outside the one file allowed for each. Complements clippy.toml.
set -eu
cd "$(dirname "$0")/.."
REMOVE_ALLOWED="crates/sapu-core/src/safety/dispose.rs"
RENAME_ALLOWED="crates/sapu-core/src/organize/apply.rs"
REMOVE_PATTERN='remove_file\|remove_dir\|remove_dir_all\|trash::delete\|TrashContext\|allow(clippy::disallowed_methods)'
hits=$(grep -rn --include='*.rs' "$REMOVE_PATTERN" crates app 2>/dev/null | grep -v "^$REMOVE_ALLOWED:" || true)
if [ -n "$hits" ]; then
  echo "Forbidden file-removal usage outside $REMOVE_ALLOWED:" >&2
  echo "$hits" >&2
  exit 1
fi
# Tests may rename to simulate a person moving files; they are the only
# code allowed to use `.expect(`, which is how they are told apart here.
hits=$(grep -rn --include='*.rs' 'fs::rename' crates app 2>/dev/null | grep -v "^$RENAME_ALLOWED:" | grep -v '\.expect(' || true)
if [ -n "$hits" ]; then
  echo "Forbidden rename usage outside $RENAME_ALLOWED:" >&2
  echo "$hits" >&2
  exit 1
fi
echo "ok: removal APIs only in $REMOVE_ALLOWED; rename only in $RENAME_ALLOWED"
