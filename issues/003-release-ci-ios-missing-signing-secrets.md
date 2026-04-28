# Issue #003: Release CI — iOS build fails due to missing signing secrets

**Date:** 2026-04-09
**Severity:** High
**Status:** Open
**Affected component:** `.github/workflows/` (Release CI)

## Summary
The iOS release job fails immediately because `ios_profile` and `ios_cert` secrets are not configured in the `Project-Robius-China/robrix2` fork repository.

## Symptoms
- iOS job fails in ~57s without compiling any code
- Error: `ios_profile and ios_cert are required for iOS device builds.`

## Root Cause
The `makepad-packaging-action` requires iOS provisioning profile and signing certificate secrets for device builds. These secrets exist in the upstream `project-robius/robrix` repo but are not available in the fork `Project-Robius-China/robrix2`.

## Reproduction
1. Push a release tag (e.g., `v0.0.1-pre-alpha-4`) to `Project-Robius-China/robrix2`
2. Observe the "Release Robrix for iOS" job failure

## Fix Applied
None yet.

## Remaining Issues
1. Add `ios_profile` and `ios_cert` secrets to the fork repo Settings > Secrets
2. Alternatively, skip iOS builds in the fork's release workflow if signing certs are unavailable

## Files Changed
None

## Test Verification
| Before | After |
|--------|-------|
| iOS job fails: "ios_profile and ios_cert are required" | Pending fix |

## Reference
- CI run: https://github.com/Project-Robius-China/robrix2/actions/runs/24117713131
- Job ID: 70365022917
