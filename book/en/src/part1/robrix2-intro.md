# Introducing Robrix2

> **Scope**: This chapter covers what Robrix2 is technically and how to get it running. No prerequisites; readers who only want to deploy HAgency can skip ahead to Chapter 4 and come back as needed.

[Robrix2](https://github.com/Project-Robius-China/robrix2) is a native Matrix client written in **Rust**, with a UI layer built on [Makepad 2.0](https://github.com/makepad/makepad) — a GPU-rendered Rust UI framework. It is one of the flagship projects of the Robius cross-platform application ecosystem.

“Native” describes the implementation and rendering path, not an unbenchmarked performance claim. The practical focus is bringing Matrix rooms, threads, device encryption, and a multi-tab workspace together while layering Agent-specific views over standard events.

## Technical Profile

- **Rust all the way down**: client logic uses [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk), and Makepad shaders render the UI. There is no Electron or WebView; memory and startup performance should be measured on each target platform.
- **Cross-platform**: a single codebase runs on macOS, Windows, and Linux, and can be packaged for mobile.
- **Sliding Sync**: uses Matrix's next-generation sync protocol; the room list and timelines load on demand, so even an account with hundreds of rooms cold-starts quickly.
- **Multi-tab workspace**: rooms, threads, and DMs open side by side as tabs (Tab/Dock). This is its biggest interaction departure from traditional single-pane IMs, designed for keeping an eye on multiple collaboration sites at once — Part II will show what this design is really for.

## Quick Start

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2
cargo run            # normal run
cargo run -- --hot   # development mode with hot reload
```

After first launch, sign in with any Matrix account: one on your self-hosted homeserver (see Chapter 4), or one on a public server such as matrix.org.

## Relationship to Other Matrix Clients

Robrix2 is a standard Matrix client. Rooms, threads, and mentions remain visible in compatible clients. Robrix2 adds native approval cards, Agent badges, and workflow text completion. A generic client may show only fallback/raw custom events and **cannot approve by sending ordinary text**; owner approval requires Robrix2 or another UI that emits the structured verdict.

This defines the interoperability boundary: public collaboration should use standard messages and relations where possible; security-sensitive extensions must be verified server-side rather than trusted because one client drew a button.
