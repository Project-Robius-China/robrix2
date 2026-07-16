#!/usr/bin/env bash
# apply-app-icon.sh — inject Robrix's branded launcher icon into the generated
# OpenHarmony (HarmonyOS) DevEco project.
#
# Why this is needed:
#   `cargo makepad ohos ... deveco` scaffolds the DevEco project from cargo-makepad's
#   own template, which ships makepad's placeholder icon and has no hook to use the
#   project's real icon (unlike the Android path, which reads
#   [package.metadata.makepad.android].icons in Cargo.toml). It also regenerates the
#   project under target/ on every run, so the icon cannot simply be committed there.
#   This script copies Robrix's icons (packaging/ohos/media/) into the freshly
#   generated project.
#
# When to run: AFTER `cargo makepad ohos ... deveco`, BEFORE `... build`.
#
#   usage: packaging/ohos/apply-app-icon.sh [deveco_project_dir]
#          deveco_project_dir defaults to target/makepad-open-harmony/robrix
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
MEDIA="$HERE/media"
PROJ="${1:-target/makepad-open-harmony/robrix}"

[ -d "$PROJ" ] || {
  echo "error: DevEco project not found at '$PROJ'" >&2
  echo "       run 'cargo makepad ohos ... deveco -p robrix' first." >&2
  exit 1
}

APPSCOPE_MEDIA="$PROJ/AppScope/resources/base/media"
ENTRY_MEDIA="$PROJ/entry/src/main/resources/base/media"
[ -d "$APPSCOPE_MEDIA" ] && [ -d "$ENTRY_MEDIA" ] || {
  echo "error: '$PROJ' does not look like a makepad DevEco project (missing media dirs)" >&2
  exit 1
}

# app-level icon (AppScope/app.json5 -> $media:app_icon)
cp "$MEDIA/app_icon.png"   "$APPSCOPE_MEDIA/app_icon.png"
# launcher layered icon (media/layered_image.json -> $media:foreground over $media:background)
cp "$MEDIA/foreground.png" "$ENTRY_MEDIA/foreground.png"
cp "$MEDIA/background.png" "$ENTRY_MEDIA/background.png"
# start-window icon (module.json5 EntryAbility -> $media:startIcon)
cp "$MEDIA/startIcon.png"  "$ENTRY_MEDIA/startIcon.png"

echo "Robrix app icon applied to $PROJ"
