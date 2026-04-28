# Issue #009: Makepad Apple HTTP backend triggers UB on empty-body appservice probe

**Date:** 2026-04-20
**Severity:** High (can abort the app from a normal settings action)
**Status:** Mitigated locally, upstream bug remains
**Affected component:** Makepad Apple network backend (`platform/network/src/backend/apple/http.rs`) and Robrix `src/settings/bot_settings.rs`

## Summary

On Apple platforms, Makepad's HTTP backend can abort the process when an HTTP response has `content-length: 0` and the backend still calls `slice::from_raw_parts(bytes, length)` with a null `bytes` pointer.

Robrix hit this through the new `Lab > App Service > Check Now` matrix-gateway probe:

- probe target: `GET /_matrix/app/v1/transactions/test`
- gateway response: `405 Method Not Allowed`
- response body: empty (`content-length: 0`)

The result was a process abort during a normal user action in Settings.

## Symptoms

- Clicking `Lab > App Service > Check Now` can abort the app on macOS
- Console shows a panic from Makepad's Apple HTTP backend:

```text
thread '<unnamed>' panicked at .../platform/network/src/backend/apple/http.rs:318:49:
unsafe precondition(s) violated: slice::from_raw_parts requires the pointer to be aligned and non-null
```

- The abort happens after the matrix-gateway probe returns an otherwise-valid HTTP response

## Reproduction

1. Run local OctOS Matrix appservice gateway on `127.0.0.1:8009`
2. In Robrix, open `Lab > App Service`
3. Click `Check Now`
4. Robrix sends:
   - a normal OctOS service health probe
   - a matrix gateway probe to `GET http://127.0.0.1:8009/_matrix/app/v1/transactions/test`
5. Gateway responds `405 Method Not Allowed` with `content-length: 0`
6. Makepad Apple backend aborts the process

## Root Cause

In Makepad's Apple HTTP backend:

- [http.rs](/Users/zhangalex/.cargo/git/checkouts/makepad-69d78fae78fc8901/5e6d7b3/platform/network/src/backend/apple/http.rs:318)

the callback unconditionally does:

```rust
let bytes: *const u8 = msg_send![data, bytes];
let length: usize = msg_send![data, length];
let data_bytes: &[u8] = std::slice::from_raw_parts(bytes, length);
```

For a zero-length response body, `length == 0` but `bytes` can still be null on the Apple side. That violates the precondition of `from_raw_parts`, and UB checks turn it into a panic/abort.

## Why Robrix Triggered It

The new appservice health UI added a second probe for `Matrix Gateway` in:

- [src/settings/bot_settings.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/settings/bot_settings.rs:1)

The first implementation used a `GET` request to the Matrix appservice transaction endpoint because:

- `405` proves the endpoint is alive
- the endpoint only allows `PUT`

That part was logically correct, but it accidentally selected an empty-body response shape that Makepad's Apple backend cannot safely handle.

## Local Mitigation Applied

Robrix now avoids the empty-body response shape instead of relying on Makepad to handle it safely.

The matrix-gateway probe was changed to:

- `PUT /_matrix/app/v1/transactions/test`
- `Content-Type: application/json`
- body = `{}`

This produces a `401 Unauthorized` response with a non-empty body, which is sufficient to prove the Matrix appservice gateway is alive without triggering the Apple backend bug.

Robrix now treats these as reachable for the gateway probe:

- `200`
- `401`
- `405`

## Impact

- The bug is not specific to OctOS or Matrix
- Any Makepad app using the Apple HTTP backend can trigger it if it receives a valid zero-length HTTP response body and the underlying `NSData` pointer is null
- This is an upstream correctness/safety bug in Makepad's Apple networking backend

## Recommended Upstream Fix

Makepad should special-case empty response bodies before calling `from_raw_parts`, for example:

```rust
let data_bytes: &[u8] = if length == 0 {
    &[]
} else {
    std::slice::from_raw_parts(bytes, length)
};
```

or otherwise guarantee a non-null pointer for zero-length bodies.

## Local Status

- Robrix workaround: applied
- User-facing check flow: preserved
- Upstream Makepad bug: still open

## Related Files

- [src/settings/bot_settings.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/settings/bot_settings.rs:1)
- [resources/i18n/en.json](/Users/zhangalex/Work/Projects/FW/robius/robrix2/resources/i18n/en.json:279)
- [resources/i18n/zh-CN.json](/Users/zhangalex/Work/Projects/FW/robius/robrix2/resources/i18n/zh-CN.json:279)
- [http.rs](/Users/zhangalex/.cargo/git/checkouts/makepad-69d78fae78fc8901/5e6d7b3/platform/network/src/backend/apple/http.rs:318)
