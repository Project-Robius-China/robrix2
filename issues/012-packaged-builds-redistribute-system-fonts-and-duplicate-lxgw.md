# Packaged builds materialise host system fonts (licence risk); mobile packages also ship LXGWWenKai twice

**Status:** Fixed on `fix/bundled-fonts-packaging`; final packaged-artifact
inspection remains for CI or a machine with the platform toolchains installed.

## Summary

Since #298, `build.rs` resolves the UI fonts into two stable symlinks that the
DSL references unconditionally:

```
resources/fonts/system_latin.ttf -> <system Latin font>  | fallback: resources/fonts/LiberationMono-Regular.ttf
resources/fonts/system_cjk.ttc   -> <system CJK font>    | fallback: resources/fonts/LXGWWenKaiRegular.ttf
```

The design claim was *"symlinked, never copied — the bytes stay the
platform's, on the user's own disk, so nothing redistributes a system font"*,
and *"ship no text fonts, because LXGWWenKai (19 MB) was being packaged"*
(docs/ux-audit-and-performance-2026-08.md §3). Both hold for `cargo run`.
Neither holds for **packaged builds**, which copy the `resources/` tree:

### A. (High) Desktop / mobile packages materialise host system fonts

Every packager in use dereferences the symlink and writes a regular file:

- `cargo makepad apple` / `cargo makepad android`: `cp_all` walks with
  `Path::is_file()` and copies with `fs::read` → `File::create`
  (`tools/cargo_makepad/src/shell.rs:238-300`). Both follow symlinks.
- Windows: there is no symlink at all — `build.rs` uses `fs::copy` on
  non-unix, so `resources/fonts/system_cjk.ttc` *is* a copy of
  `C:/Windows/Fonts/msyh.ttc` before packaging even starts.
- Desktop `cargo packager` (DMG/MSI/deb) takes files from
  `dist/resources/robrix`, produced by `robius-packaging-commands` copying
  `resources/`; same outcome (to be confirmed by inspecting a built `.app`,
  but Windows is already certain).

So a macOS `.app`/DMG built on a Mac contains a real copy of
`PingFangUI.ttc` (or Hiragino Sans GB) plus `SFNS.ttf`; a Windows MSI
contains Microsoft YaHei and Segoe UI. That is redistribution of Apple /
Microsoft proprietary fonts — the exact thing the scheme was meant to avoid —
and PingFangUI.ttc alone is far larger than the 19 MB LXGW it replaced.

### B. (Low) Mobile packages ship LXGWWenKai twice

On android/ios targets `build.rs` always takes the bundled fallback (host
fonts must not leak into cross-builds — #302). Because the fallback file
*and* the symlink both live inside the packaged `resources/` tree, the
APK/IPA gets `LXGWWenKaiRegular.ttf` under its own name **and** again as the
materialised `system_cjk.ttc`: ~38 MB of CJK font instead of the intended 0,
i.e. +19 MB versus before #298. (LiberationMono is duplicated too, but it is
108 KB and directly referenced by the DSL as the monospace face, so it must
stay under `resources/`.)

Nothing is functionally broken; A is a licensing/size problem, B is size only.

## Root cause

The DSL can only name a compile-time `crate_resource` path, so the "which
font" decision is baked into the `resources/` tree at build time, and every
packager copies that tree by value. Symlinks are an adequate mechanism for
"run from the source tree", not for "produce a distributable".

## Implemented resolution (no makepad change)

1. **The bundled CJK fallback was moved out of `resources/`** to
   `bundled_fonts/LXGWWenKaiRegular.ttf`, and `build.rs` now points its fallback
   there. Only the resolved `system_cjk.ttc` is packaged → fixes B
   (38 MB → 19 MB). The new directory is covered by `cargo:rerun-if-changed`.
   `LiberationMono-Regular.ttf` stays.

2. **A packaging switch now forbids host fonts:**
   `ROBRIX_BUNDLED_FONTS=1`:
   - `build.rs`: `cargo:rerun-if-env-changed=ROBRIX_BUNDLED_FONTS`; when
     set, skip all system-font candidates on every target and link/copy only
     the OFL-licensed bundled fallbacks.
   - It is set in every formal packaging path: `builds.yml` (iOS/Android
     jobs), `release.yml` (desktop and release jobs), and
     `packaging/build-macos-dmg.sh`; manual `cargo packager` usage is documented
     in the README for POSIX shells and PowerShell.
   Result: `cargo run` keeps using system fonts (PingFang/SF on macOS,
   YaHei/Segoe on Windows, fontconfig on Linux); every distributable ships
   exactly one copy of LXGWWenKai + LiberationMono and no proprietary font.
   → fixes A.

3. (Optional, additive) Subset LXGWWenKai (OFL permits it) to
   GB2312/通用规范汉字表 via `pyftsubset`, ~3–5 MB, to shrink what step 2
   ships. Trade-off: rare characters render as tofu where no system CJK font
   is available.

Not achievable without makepad work: a desktop/mobile *package* that ships
no CJK font at all and uses the device's own (Android
`/system/fonts/NotoSansCJK-Regular.ttc`, iOS PingFang) at runtime. That
needs a font defined from an absolute runtime path / bytes rather than a
compile-time `crate_resource` (`draw`'s `Loader::define_font` takes
`SharedBytes` but nothing exposes it at the widget/DSL layer). Track
separately if wanted.

`cargo makepad android build --small-fonts` is *not* a solution: it swaps
`LXGWWenKaiRegular.ttf` for IBMPlexSans (no CJK glyphs) and leaves
`system_cjk.ttc` at full size since it only rewrites known filenames.

## Verification

- Normal and bundled builds both pass all four `cjk_font_tests`; switching
  normal → bundled → normal reruns `build.rs` and replaces both links.
- On Linux, normal mode resolves to the installed Noto CJK/Liberation Sans;
  bundled mode resolves only to LXGWWenKai/LiberationMono.
- A by-value enumeration of `resources/fonts/`, matching the packager's
  symlink-dereferencing copy semantics, contains one CJK font in bundled mode
  and no host-font hashes.
- A real APK, macOS `.app`, and Windows package still need final artifact
  inspection in CI or on hosts with cargo-makepad/cargo-packager and their
  platform toolchains installed.

## Related

- #298 — introduced the symlink scheme
- #302 — Linux fix; restricted host-font linking to `target_os == "linux"`
