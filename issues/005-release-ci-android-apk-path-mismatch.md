# Issue #005: Release CI — Android build succeeds but APK upload fails due to path mismatch

**Date:** 2026-04-09
**Severity:** High
**Status:** Open
**Affected component:** `.github/workflows/` (Release CI), `makepad-packaging-action`

## Summary
The Android release job compiles successfully and builds the APK, but fails at the upload step because `makepad-packaging-action` looks for the APK at a path that doesn't match where `cargo-makepad` actually outputs it.

## Symptoms
- Compilation succeeds (~13 min)
- "APK Build completed" message appears
- Upload fails with: `Missing artifacts on disk: .../target/makepad-android-apk/robrix/apk/robrix_v0.0.1-pre-alpha-4_aarch64.apk`
- The actual APK was built under `target/android/makepad-android-apk/...` (note the extra `android/` path segment)

## Root Cause
Path mismatch between `cargo-makepad` APK output directory and `makepad-packaging-action`'s expected artifact location:
- **Expected by action:** `target/makepad-android-apk/robrix/apk/robrix_v0.0.1-pre-alpha-4_aarch64.apk`
- **Actual output:** `target/android/makepad-android-apk/robrix/apk/...` (likely)

This suggests `cargo-makepad` changed its output directory structure, or `makepad-packaging-action@v1` has a hardcoded path that doesn't account for the `android/` subdirectory.

## Reproduction
1. Push a release tag to `Project-Robius-China/robrix2`
2. Observe the "Release Robrix for Android (aarch64)" job — compilation and APK build succeed, but upload fails

## Fix Applied
None yet.

## Remaining Issues
1. Verify the actual APK output path on the CI runner (add an `ls -R target/` debug step)
2. Update `makepad-packaging-action` to use the correct path, or pin a compatible version of `cargo-makepad`
3. Report upstream if this is a bug in `makepad-packaging-action@v1`

## Files Changed
None

## Test Verification
| Before | After |
|--------|-------|
| Android: APK built but upload fails — path mismatch | Pending fix |

## Reference
- CI run: https://github.com/Project-Robius-China/robrix2/actions/runs/24117713131
- Job ID: 70365022944
