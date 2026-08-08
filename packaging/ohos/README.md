# HarmonyOS (OpenHarmony) app icon

Robrix's branded launcher icon for the HarmonyOS build lives here, plus a script
that applies it to the generated DevEco project.

## Why this exists

`cargo makepad ohos ... deveco` scaffolds the DevEco project from cargo-makepad's
built-in template. That template ships **makepad's placeholder icon**, and the ohos
generator has **no hook to use the project's real icon** (the Android path does —
it reads `[package.metadata.makepad.android].icons` in `Cargo.toml`). The project is
also regenerated under `target/` on every `deveco` run, so the icon cannot simply be
committed into the generated tree.

So without this step, Robrix shows the generic makepad icon on the HarmonyOS launcher
instead of the Robrix logo.

## Files

| File | Used by | Notes |
|------|---------|-------|
| `media/foreground.png` | `$media:foreground` (layered launcher icon) | 288×288, transparent cube in the Ø192 safe zone |
| `media/background.png` | `$media:background` (layered launcher icon) | 288×288, solid white |
| `media/app_icon.png`   | `AppScope/app.json5` → `$media:app_icon` | 216×216, cube on white |
| `media/startIcon.png`  | `EntryAbility` → `$media:startIcon` | 288×288, cube on white |

The launcher icon follows HarmonyOS's **layered icon** convention: a transparent
foreground (the Robrix cube, kept inside the Ø192 safe area) over a solid background,
with the system supplying the mask and elevation — no baked-in card/shadow. The cube
artwork is derived from `resources/robrix_logo_alpha.png`.

## Build flow

Run the icon step between `deveco` and `build`:

```bash
# 1. generate the DevEco project + cross-compile librobrix.so
cargo makepad ohos --deveco-home="/Applications/DevEco-Studio.app/Contents" deveco -p robrix --release

# 2. inject Robrix's icon (this script) — must come after deveco, before build
packaging/ohos/apply-app-icon.sh

# 3. package the .hap
cargo makepad ohos --deveco-home="/Applications/DevEco-Studio.app/Contents" build -p robrix --release
```

## Follow-up

The proper long-term fix is upstream: teach cargo-makepad's ohos generator to read the
project's icon (as the Android path already does) and generate the layered icon, which
would make this script unnecessary.
