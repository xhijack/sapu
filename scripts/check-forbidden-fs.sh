#!/usr/bin/env sh
# Fails if any removal API or the clippy opt-out appears outside the one
# allowed file. Complements clippy.toml's disallowed-methods.
set -eu
cd "$(dirname "$0")/.."
ALLOWED="crates/sapu-core/src/safety/dispose.rs"
PATTERN='remove_file\|remove_dir\|remove_dir_all\|trash::delete\|allow(clippy::disallowed_methods)'
hits=$(grep -rn --include='*.rs' "$PATTERN" crates app 2>/dev/null | grep -v "^$ALLOWED:" || true)
if [ -n "$hits" ]; then
  echo "Forbidden file-removal usage outside $ALLOWED:" >&2
  echo "$hits" >&2
  exit 1
fi
echo "ok: removal APIs only in $ALLOWED"
