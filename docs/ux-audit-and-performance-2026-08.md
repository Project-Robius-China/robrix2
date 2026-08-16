# UX audit, system fonts, and performance — working notes

Session of 2026-08-12/13. Written as a handoff: what changed, what was
measured (and under what conditions), what broke and why, and what is still
open. Numbers here are only quoted where I actually ran them; anything
unverified is labelled as such.

---

## ⚠️ Read this first: the working copy is not under version control

`~/home/robrix` was extracted from a **source tarball**, not cloned — `git
clone` failed at the time because the Xcode Command Line Tools were missing.
So there is **no `.git`, no history, and no way to diff or revert** any of the
changes below.

Everything in "What changed" is uncommitted and unprotected. Putting this
directory under git is the highest-priority follow-up, ahead of any of the
remaining technical work.

```bash
cd ~/home/robrix && git init && git add -A && git commit -m "baseline: tarball + session changes"
```

---

## Environment

| thing | state |
|---|---|
| Xcode Command Line Tools | installed mid-session (26.6). Nothing compiled before this. |
| Rust toolchain | `~/.cargo/bin`, **not on the default PATH** — prefix commands with `PATH="$HOME/.cargo/bin:$PATH"` |
| Docker Desktop | installed to `/Applications` this session; 29.7.2, Compose v5.3.1 |
| Docker VM | raised to 31 GB / 12 CPUs (default 8.3 GB OOM'd the Palpo build). Original settings backed up in the session scratchpad. |
| Accessibility permission | **pending** — needed for synthetic input; see "Open threads" |

`rustc` must be on the PATH of any *child* process too: makepad's headless
renderer shells out to `rustc` to JIT-compile shaders, and silently renders
nothing if it can't find it.

---

## Palpo homeserver (running in Docker)

```
palpo-and-octos-deploy-palpo-1            0.0.0.0:8128->8008
palpo-and-octos-deploy-palpo_postgres-1   5432

user:     @robrix:127.0.0.1:8128
password: ktCzumTHucZdTCpGNqIH
server:   http://127.0.0.1:8128
```

Start/stop: `cd palpo-and-octos-deploy && docker compose up -d palpo` /
`docker compose down`. Octos is deliberately **not** running (needs
`MOONSHOT_API_KEY` and a second repo).

Test rooms created via the Matrix API: `Perf test 性能测试` (250 messages),
`中文字体测试 PingFang`, `23234`.

Robrix takes credentials positionally, which is how to launch it logged in
without touching the GUI:

```bash
PATH="$HOME/.cargo/bin:$PATH" ./target/release/robrix \
  @robrix:127.0.0.1:8128 ktCzumTHucZdTCpGNqIH http://127.0.0.1:8128
```

Add `--login-screen` to force the login screen while keeping the session.
**Do not** pass `--stdin-loop` alongside the credentials: robrix parses its own
argv with clap, an undeclared flag makes `Cli::try_parse()` fail, and the
auto-login silently doesn't happen. Use `MAKEPAD_STDIN_LOOP=1` instead.

---

## 1. UX harness and audit

New tool: **`tools/ux-harness/`** — a standalone binary (detached workspace, so
the root `cargo build` is unaffected). It drives the real app through makepad's
studio stdin protocol under the headless CPU renderer, captures the frames it
renders, and scores eight UX dimensions with per-finding evidence. No display,
no screen-recording permission; works over SSH and in CI. See its README for
the rules and their limits.

```bash
MAKEPAD=headless CARGO_TARGET_DIR=target-headless cargo build --profile fast
cd tools/ux-harness
cargo run -- run --repo ../.. --app ../../target-headless/fast/robrix --out ../../target/ux-audit
cargo run -- static --repo ../..     # source-only rules, runs anywhere
```

**Score: 7.7 → 9.4 / 10.** Both runs used the same harness build and the same
rules — I rebuilt a pristine copy of the original tree and re-scored it after
the rules were final, so the movement is the app's, not the ruler's.

| dimension | before | after |
|---|---:|---:|
| Legibility & contrast | 0.0 | 7.9 |
| Keyboard & focus | 7.0 | 10.0 |
| Target size | 8.6 | 10.0 |
| Visual consistency | 8.5 | 10.0 |
| Layout / State / Feedback / i18n | 10.0 / 10.0 / 8.5 / 9.2 | unchanged |

Report artifact: <https://claude.ai/code/artifact/9ff3168d-2776-4bed-a5ad-dd74c253681f>

### What was fixed

- **11 token pairs failed WCAG AA**, three below 3:1 — the primary CTA was
  white on `#119FB3` at 3.17:1, timestamps `#8A98AE` at 2.92:1. Each failing
  colour was walked toward black along its own hue until it cleared 4.5:1.
  Guarded by `cargo test --lib shared::design_tokens`, which fails the build if
  a pair regresses.
- **Tab did nothing at all.** Not a missing focus ring — that was already
  implemented correctly. Nothing ever *held* focus, and makepad's `NavControl`
  advances Tab from an existing focus or not at all, so a keyboard-only user
  could not reach a single control. The login screen now claims the User ID
  field on its first laid-out frame (`place_initial_focus`).
- **65 raw hex literals** across 13 screens → 0, at identical values, plus
  three new token groups (light syntax palette, composer ramp, accent washes).
- **7 undersized hit targets** → 0, sized to `RBX_TAP_MIN` / `RBX_CONTROL_H_SM`.
- Four zh-CN strings that were still English.

### Still open from the audit

- `feedback.dead-click`: at 420 dp, clicking *Sign in securely* with empty
  fields changes nothing. Desktop correctly shows the missing-user-ID modal.
- **The login screen never activates its mobile layout at any window size.**
  `sync_login_responsive_layout` runs once at startup, measures a width of 0,
  returns early, and nothing calls it again on desktop. Diagnosed but
  deliberately **not** shipped: forcing the branch on reveals that the mobile
  layout stacks its status footer over the form, and that three `set_visible`
  calls query `Label`s through `.view()` and silently do nothing. Enabling it
  would trade one bug for a worse one.
- 39 of 106 DSL files ship user-facing copy that never reaches the translation
  layer.
- Two `legibility.contrast` findings on welcome-screen buttons are almost
  certainly harness artifacts — makepad reports per-widget `visible`, not
  effective visibility, so a screen that is built but never presented still
  reports stale rects inside the viewport. Left scored against the app rather
  than silently suppressed.

---

## 2. SSO / GitHub login

**The bug:** the login screen never asked the homeserver what it supports. Six
provider buttons came from a hardcoded array and the provider id was guessed as
`format!("oidc-{}", brand)`. Meanwhile `HsCapabilities.sso_providers` — already
harvested from `/_matrix/client/v3/login` by the capability probe — had **zero
readers**. Consequences: dead buttons on servers with no SSO (Palpo advertises
only `m.login.password` + `m.login.application_service`), and on servers that do
have SSO the guessed id falls through to *generic* SSO, so the provider choice
is discarded. Clicking "GitHub" against matrix.org signed in via matrix.org's
default SSO, not GitHub.

**The fix:** added `sso_supported: bool` to `HsCapabilities` (distinct from the
providers list being empty — matrix.org advertises `m.login.sso` and names no
providers, so "empty" must not mean "no SSO"); extracted `sso_visibility()` as a
pure, testable rule; `apply_sso_availability()` applies it; the SSO row is
hidden when the server advertises no `m.login.sso`; `SpawnSSOServer` now sends
the id the server advertised. Six tests in `sso_visibility_tests`.

Verified working against Palpo by hand.

**Not fixed:** a server advertising a provider this build has no button for
(Keycloak, Okta, SAML) still shows nothing. Doing that properly needs buttons
built from the advertised list — either a fixed pool of generic slots (low
risk) or a horizontal PortalList (more correct, more work).

---

## 3. Fonts — now the system's, not bundled

Goal: ship no text fonts, because LXGWWenKai (19 MB) was being packaged.

**`build.rs`** resolves two symlinks per platform — symlinked, never copied, so
nothing redistributes a system font:

```
resources/fonts/system_latin.ttf -> /System/Library/Fonts/SFNS.ttf
resources/fonts/system_cjk.ttc   -> /System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/PingFangUI.ttc
```

with Windows (`segoeui.ttf`, `msyh.ttc`) and Linux (fontconfig, then distro
paths) candidates and bundled fallbacks, so the DSL path always resolves —
makepad panics on a font face it cannot load.

**Packaged builds** (`cargo packager`, `cargo makepad android|apple`) copy
`resources/` by value, dereferencing these links, so they must be built with
`ROBRIX_BUNDLED_FONTS=1`: host fonts are then never considered and both links
resolve to the bundled OFL faces (LiberationMono, and LXGWWenKai from
`bundled_fonts/` — kept outside `resources/` so it is not shipped twice).
CI and `packaging/build-macos-dmg.sh` set it. See issue #303.

**`styles.rs`** declares `APP_FONT_REGULAR` / `APP_FONT_BOLD`;
**`design_tokens.rs`** declares its own `RBX_FONT_REGULAR` / `RBX_FONT_BOLD`;
all 69 `theme.font_*` references across 25 files now point at those or at the
derived styles. Verified visually: Chinese renders in PingFang, Latin in San
Francisco.

### The rule that cost four failed builds — font resources and module scope

Font resources do **not** survive certain derivations across `script_mod!`
boundaries, and the failure is **silent**: text renders as nothing, no panic,
no log line.

| form | result |
|---|---|
| assign into `theme.font_regular` (whole style or one nested member) | **all text blank** |
| derive from a base declared in **another** module | that text blank |
| derive from a base in the **same** module | works |
| use an already-derived style cross-module (`REGULAR_TEXT {…}`) | works |

This is why `design_tokens.rs` repeats the family declaration instead of
reusing `styles.rs`'s. It looks like duplication and is not.

Two related facts worth keeping:

- PingFang exists only as a **32-face collection inside a private framework**.
  `ttf_parser` reads collections natively, and face 0 is `.蘋方 UI Text-簡`
  (Simplified UI Text) — which is the face we want, and just as well, because
  `FontMember` exposes **no face index**.
- `weight: 700.0` on a `FontMember` makes text vanish, so it is removed.
  **There is currently no real bold** — bold renders at regular weight. Neither
  SF's `wght` axis nor PingFang's bold faces are reachable without a makepad
  change.

Emoji is still the bundled `NotoColorEmoji.ttf` (10.6 MB) — out of scope for
"Chinese and English". Dropping it means Apple Color Emoji, a 192 MB CBDT
collection untested against makepad's renderer.

Unrelated pre-existing log noise: `Failed to load resource:
…/widgets/resources/IBMPlexSans-Text.ttf` comes from makepad's *draw-crate*
default family, whose `self:../../widgets/resources/…` path resolves one level
above the makepad checkout when makepad is a git dependency. It logs once per
run and affects only that unused default.

---

## 4. Performance

### Measured, with conditions

| comparison | result | conditions |
|---|---|---|
| build profile, native startup + sync | **3.8× CPU** (5.99 s vs 1.58 s) | same binary, same workload |
| build profile, headless per frame | **14.4×** (23 233 ms vs 1 615 ms median) | same binary, same driven input |
| **robrix2 vs upstream**, native startup + sync, cold | **2.8× CPU** (3.77 s vs 1.34 s), **+36 % RSS** (394 MB vs 290 MB) | both `--release`, both cold isolated `HOME`, both verified logged in and synced the same 3 rooms |

**The dominant finding was the build profile.** `[profile.fast]` is
`opt-level = 0` — Cargo.toml says so: *"For very fast compilation, not for
creating an actual high-quality executable."* Every binary handed over before
this point was unoptimized. **Use `--release` for anything performance-related.**

On robrix2 vs upstream: not like-for-like. robrix2 is 115 881 lines across 161
files, upstream 53 034 across 111, and the extra is startup-active — agent
registry, spaces, bot/app-service discovery, TSP state. But **robrix2 logged in
twice** in that run where upstream logged in once, which means a duplicated
client build and duplicated initial sync. That is the one clearly actionable
item in the gap.

### Code-level findings

- **Widget-tree rebuild per scroll frame.** PortalList item recycling marks the
  tree structurally dirty every frame, so `WidgetTree::sync_dirty()` runs
  `rebuild_dense()` over the whole tree — 1130 widgets desktop, 2217 mobile, as
  measured by the harness. `app.rs:858` documents it and works around it by
  gating six root-level `ids!()` lookups behind cheap action-type pre-scans;
  `sync_app_language` has its own guard. **Upstream has neither workaround** —
  plausibly because its tree is half the size.
- **`new_batch` on the Timeline view** — applied, from upstream's commit
  `7e7f93a` ("fix perf issues surrounding unnecessary redraws"). Gives the
  message list its own GPU draw list so recycling doesn't re-batch the parent.
  **Unmeasured here**: the headless rasteriser is blind to GPU batching. Watch
  for layering glitches (bot cards over the composer) — the fork's own comments
  document that hazard.
- **Mobile only:** `room_screen/mod.rs:1489` rebuilds the room header every
  `draw_walk` — `to_string()`, `get_client()`, `get_room()`, `format!("{n}
  members")`, `set_room()` — with no change guard, while the layout block
  immediately below uses exactly that guard pattern (`applied_layout_state`).
- **Not a problem, so don't chase it:** message content is not re-parsed per
  frame (`ItemDrawnStatus { profile_drawn, content_drawn }` caching is real, and
  is byte-identical between the two projects — robrix2's is upstream's,
  relocated). The two "this might be really slow" notes in `rooms_list.rs` are
  TODOs that pass a hardcoded `0`.

### What is NOT measured

**There is no scroll profile for either build.** Three sampling windows (8 s,
12 s, 60 s) all caught the app idle — >99.8 % of main-thread samples parked in
`mach_msg2_trap`. The headless path could not reach the message list either:
the backend only draws on a screenshot request, so hit-testing runs against
stale geometry and synthetic clicks never opened a room (three attempts). The
native macOS stdin loop can't substitute — it only draws into a swapchain that
Makepad Studio supplies.

---

## Open threads

1. **Put this directory under git.** Nothing above is committed.
2. **Accessibility permission** → System Settings → Privacy & Security →
   Accessibility → enable `Terminal`. This unblocks synthetic clicks and
   scrolls, which lets the scroll profile be captured without a human, and also
   lets the UX harness verify keyboard focus natively. Both symbol builds are
   ready (`CARGO_PROFILE_RELEASE_STRIP=none cargo build --release`, 6658
   symbols) — `scratchpad/robrix-upstream/target/release/robrix` for upstream.
3. **The scroll profile itself**: open a room, drive a fixed scroll, `sample`
   for 30 s, repeat identically on upstream. The question is whether
   `rebuild_dense` dominates robrix2's scroll and whether upstream pays it too.
4. **The duplicate login** in robrix2's startup.
5. `LoginAction::LoginFailure` is posted immediately *before* "Logged in
   successfully" on the CLI auto-login path — a flash of "login failed" on a
   login that works.
6. Startup log errors, pre-existing and unrelated to any change here:
   `room_filter_search_results.rs:38` — "cannot push to frozen vec".

## Harness limitations worth knowing

- Makepad emits a key-focus rect after pointer input but **never** after a key
  event, so the absence of one proves nothing. The focus rule is pixel-based.
- Widgets at `(0,0,0,0)` were never drawn (every closed modal looks like this);
  no geometry rule may speak about them.
- `makepad_micro_serde` emits **trailing commas** when a struct's last field is
  a `None` option. Its own parser accepts them, `serde_json` rejects them —
  without `proto::strip_trailing_commas` every `WidgetSnapshot` response
  silently fails to parse.
- The headless backend is **pull-driven**: it only advances timers and repaints
  on a `Tick`. A screen with a focused text field never fully idles, because
  the caret blink asks for frames forever.
