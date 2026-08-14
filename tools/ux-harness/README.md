# ux-harness

A UX audit harness for Robrix2. It drives the real app through **makepad's
internal event channel**, captures the frames the app actually renders, and
turns both into a scored report with per-finding evidence.

No screen recording, no accessibility permissions, no window server. The app
runs under makepad's headless CPU renderer and speaks newline-delimited JSON
over stdin/stdout, so the whole thing works over SSH and in CI.

## What it measures

| Dimension | Weight | How |
|---|---:|---|
| Layout integrity | 1.5 | post-layout widget rects: unreachable, overflowing, collapsed, or stacked controls |
| Target size | 1.0 | every enabled control against `RBX_TAP_MIN` (mobile) / 28dp (desktop) |
| Legibility & contrast | 1.5 | WCAG contrast of the design tokens **and** of real text sampled from rendered pixels |
| Feedback & responsiveness | 1.5 | click → frame diff + widget-tree diff; how long the UI keeps redrawing |
| Keyboard & focus | 1.5 | Tab presses that change no pixels = no visible focus indicator |
| State coverage | 1.5 | blank-screen detection; presence of an actionable control |
| Visual consistency | 1.0 | raw hex literals outside the token layer |
| Internationalization | 1.5 | untranslated screens, missing keys, byte-identical translations |

Each dimension starts at 10 and loses 3.0 / 1.5 / 0.6 / 0.2 per
critical / high / medium / low finding. The overall score is the weighted mean
of the dimensions that actually ran — a dimension whose rules did not run is
reported as **not run**, never as a pass.

## Usage

### Static only (runs anywhere, no build needed)

```bash
cd tools/ux-harness
cargo run -- static --repo ../.. --out ../../target/ux-audit
```

Scores token contrast, token discipline, and translation coverage from the
source tree.

### Full audit (drives the app)

Build the app with makepad's headless CPU renderer, then point the harness at
the binary:

```bash
MAKEPAD=headless CARGO_TARGET_DIR=target-headless cargo build --profile fast

cd tools/ux-harness
cargo run -- run \
  --repo ../.. \
  --app ../../target-headless/fast/robrix \
  --out ../../target/ux-audit
```

Outputs `ux-report.md`, `ux-report.json`, and every captured frame under
`frames/` (including `scene_*.png`, one per measurement point).

`rustc` must be on `PATH`: the headless renderer JIT-compiles each shader into
a dylib on first use. The JIT cache lands in `--out/jit`, so a second run is
much faster than the first.

### Contrast helper

```bash
cargo run -- contrast --fg '#687283' --bg '#FFFFFF'
# #687283 on #FFFFFF = 4.86:1 (target 4.5:1)
# passes
```

When a pair fails it prints the nearest compliant color along the same hue, so
a palette fix keeps its hue relationships.

## How it drives the app

Makepad's headless backend (`MAKEPAD=headless`, run with `--stdin-loop`) reads
`StudioToApp` messages on stdin and writes `AppToStudio` on stdout — the same
channel Makepad Studio uses to remote-control a running app.

- **Input**: `MouseDown/Up/Move`, `KeyDown/Up`, `TextInput`, `Scroll` are
  delivered as genuine `Event`s, through the same dispatch a real click takes.
- **Introspection**: `WidgetSnapshot` returns every widget's post-layout rect,
  visibility, enabled state, and text — an accessibility-tree equivalent.
- **Capture**: every draw cycle writes a PNG to `MAKEPAD_HEADLESS_OUT_DIR`.

Two details are load-bearing:

1. **The backend is pull-driven.** It only advances timers, next-frames and
   repaints when it receives a `Tick`, and answers with `RequestAnimationFrame`
   while it still has work. `Driver::settle` ticks until the app goes quiet,
   which is what makes a run reproducible rather than a race. A UI with a
   focused text field never fully idles — the caret blink asks for frames
   forever — so settle is also bounded by a wall clock.
2. **Makepad's JSON has trailing commas.** `makepad_micro_serde` emits
   `{"a":1,}` when a struct's last field is a `None` option, which its own
   parser accepts and `serde_json` rejects. `proto::strip_trailing_commas`
   handles it; without it every `WidgetSnapshot` response silently fails to
   parse.

## Adding a scene

Scenes live in `src/scenes.rs`. A scene is a short deterministic path ending in
a measurement:

```rust
driver.set_viewport(1280.0, 800.0, 1.0)?;
driver.settle(60)?;
let widgets = driver.widget_snapshot()?;
let frame = driver.capture("my_scene")?;
findings.extend(check_geometry(&ctx, &widgets));
```

Keep them small and independent: a failure should name one interaction, not
"the app".

## Rule discipline

A rule that cannot measure something confidently stays quiet, and says so as a
`harness.*` finding rather than blaming the app. Concretely:

- Widgets reported at `(0,0,0,0)` were never drawn (every closed modal in the
  tree looks like this) — no geometry rule may speak about them.
- Content below or right of the viewport is normal scrolling, not a layout bug;
  only negative coordinates are unreachable.
- Contrast is only sampled for widgets fully inside the captured frame, and
  only for rects shaped like a line of text.
- Makepad emits a key-focus rect after pointer input but never after a key
  event, so the *absence* of one proves nothing. The focus rule is pixel-based
  instead.
