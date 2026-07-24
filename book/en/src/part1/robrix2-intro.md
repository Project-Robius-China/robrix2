# Introducing Robrix2

> **Scope**: This chapter covers what Robrix2 is technically and how to get it running. No prerequisites; readers who only want to deploy HAgency can skip ahead to Chapter 4 and come back as needed.

[Robrix2](https://github.com/Project-Robius-China/robrix2) is a native Matrix client written in **Rust**, with a UI layer built on [Makepad 2.0](https://github.com/makepad/makepad) — a GPU-rendered Rust UI framework. It is one of the flagship projects of the Robius cross-platform application ecosystem.

## Technical Profile

- **Rust all the way down**: client logic is built on [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk) (the official SDK, shared with Element X), and the UI is rendered by Makepad shaders. No Electron, no WebView — lower memory usage and genuinely native startup speed.
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

Robrix2 is a standard Matrix client — it depends on no proprietary server-side extensions. You can log into the same account with Element to verify that messages interoperate; conversely, most of HAgency's collaboration (rooms, threads, @mentions) is visible in any Matrix client. What Robrix2 adds is a **native agent experience**: approval cards, agent badges, workflow command completion — all of which degrade to plain text in a generic client.
