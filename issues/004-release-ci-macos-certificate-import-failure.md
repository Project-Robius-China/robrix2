# Issue #004: Release CI — macOS builds fail on code signing certificate import

**Date:** 2026-04-09
**Severity:** High
**Status:** Open
**Affected component:** `.github/workflows/` (Release CI)

## Summary
Both macOS release jobs (aarch64 on macos-14, x86_64 on macos-15-intel) fail after successful compilation when `cargo-packager` attempts to import the `.p12` signing certificate into the macOS keychain.

## Symptoms
- Compilation succeeds (~15-30 min)
- Packaging step fails at certificate import
- Error: `Failed to import certificate: security: SecKeychainItemImport: One or more parameters passed to a function were not valid.`
- Affects both macOS architectures identically

## Root Cause
`cargo-packager` calls `security import cert.p12 -k cargo-packager.keychain -P "" ...` but the certificate data (from GitHub secrets) is either:
1. Missing or empty in the fork repo
2. Corrupted / incorrectly base64-encoded
3. Has a non-empty password that isn't being passed

The `-P ""` (empty password) suggests the secret for the certificate password may also be missing.

## Reproduction
1. Push a release tag to `Project-Robius-China/robrix2`
2. Observe both macOS jobs fail at the "Package (macos)" step after compilation completes

## Fix Applied
None yet.

## Remaining Issues
1. Verify macOS signing certificate secrets are properly configured (certificate .p12 + password)
2. If code signing is not needed for the fork, configure the workflow to skip signing or use ad-hoc signing
3. Consider making signing optional via a workflow input flag

## Files Changed
None

## Test Verification
| Before | After |
|--------|-------|
| macOS aarch64: fails at certificate import after 15min build | Pending fix |
| macOS x86_64: fails at certificate import after 30min build | Pending fix |

## Reference
- CI run: https://github.com/Project-Robius-China/robrix2/actions/runs/24117713131
- Jobs: 70365022919 (aarch64), 70365022955 (x86_64)
