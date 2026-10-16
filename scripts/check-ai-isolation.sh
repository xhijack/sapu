#!/usr/bin/env sh
# The AI crate must never depend on sapu-core (CLAUDE.md rule 3): it may
# only produce suggestions, never reach safety/cleaner. Checked from the
# resolved dependency graph, not from text.
set -eu
cd "$(dirname "$0")/.."
if cargo tree -p sapu-ai -e normal | grep -q 'sapu-core'; then
  echo "sapu-ai depends on sapu-core; the AI layer must stay isolated" >&2
  exit 1
fi
echo "ok: sapu-ai has no dependency on sapu-core"
