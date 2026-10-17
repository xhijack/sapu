#!/usr/bin/env sh
# Tauri runs commands that are not `async` on the main thread: any disk,
# keychain, network or large-data work there freezes the window (spinning
# cursor). Only commands that copy small in-memory state, or hand work to a
# thread and return at once, may stay synchronous. Everything else must be
# `async` and do its work in `spawn_blocking`.
set -eu
cd "$(dirname "$0")/.."
ALLOWED=" scan_pause scan_resume scan_cancel scan_status scan_targets scan_start \
clean_check_start clean_check_status clean_last clean_run_start clean_run_status clean_run_last \
care_last dedupe_cancel dedupe_start organize_analyze_start organize_apply_start \
organize_status organize_plan organize_targets protected_paths open_url \
open_disk_access_settings reveal_path reveal_app_binary relaunch_as_admin "
bad=""
for f in app/src-tauri/src/*.rs; do
  names=$(awk '/#\[tauri::command/ {getline; if ($0 !~ /async fn/) { sub(/.*fn /, ""); sub(/[(<].*/, ""); print } }' "$f")
  for n in $names; do
    case "$ALLOWED" in
      *" $n "*) ;;
      *) bad="$bad\n  $f: $n" ;;
    esac
  done
done
if [ -n "$bad" ]; then
  printf "Synchronous Tauri commands that may block the main thread:%b\n" "$bad" >&2
  printf "Make them async and move the work into spawn_blocking.\n" >&2
  exit 1
fi
echo "ok: no blocking work in synchronous Tauri commands"
