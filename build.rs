fn main() {
    // Note: `#[cfg(windows)]` checks the *host* OS, not the *target*.
    // We must check the target env at runtime to avoid running this
    // when cross-compiling (e.g., building for Android on a Windows CI runner).
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        #[cfg(windows)]
        {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("resources/icon.ico");
            // Explicit VERSIONINFO fields. Without these, Windows shows
            // "Unknown publisher" in the UAC/SmartScreen install prompt
            // (CompanyName/LegalCopyright are empty by default), and the
            // ProductName/FileDescription fall back to the lowercase crate
            // name "robrix" instead of the product name.
            res.set("CompanyName", "GOSIM Foundation");
            res.set("ProductName", "Robrix");
            res.set("FileDescription", "Robrix - Matrix chat client");
            res.set("LegalCopyright", "Copyright - 2023-2026 Project Robius");
            res.compile().expect("Failed to compile Windows resources");
        }
    }

    link_system_cjk_font(&target_os);

    // Get version info about Robrix, the matrix SDK, and testflight.
    println!("cargo:rerun-if-changed=Cargo.lock");
    let (sdk_version, sdk_git_rev, sdk_url) = read_matrix_sdk_info();
    println!("cargo:rustc-env=MATRIX_SDK_VERSION={sdk_version}");
    println!("cargo:rustc-env=MATRIX_SDK_GIT_REV={sdk_git_rev}");
    println!("cargo:rustc-env=MATRIX_SDK_URL={sdk_url}");

    let (robrix_git_rev, robrix_url) = read_robrix_git_info();
    println!("cargo:rustc-env=ROBRIX_GIT_COMMIT_HASH={robrix_git_rev}");
    println!("cargo:rustc-env=ROBRIX_GIT_COMMIT_URL={robrix_url}");

    println!("cargo:rerun-if-env-changed=TESTFLIGHT_BUILD_NUMBER");
    let testflight_build = std::env::var("TESTFLIGHT_BUILD_NUMBER").unwrap_or_default();
    println!("cargo:rustc-env=TESTFLIGHT_BUILD_NUMBER={testflight_build}");
}

/// Returns Robrix's own current git commit info as a commit hash and a permalink.
fn read_robrix_git_info() -> (String, String) {
    // Tell cargo to re-run when the git-tracked HEAD changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Ok(head) = std::fs::read_to_string(".git/HEAD") {
        if let Some(branch_ref) = head.trim().strip_prefix("ref: ") {
            println!("cargo:rerun-if-changed=.git/{branch_ref}");
        }
    }

    let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
    else {
        return (String::new(), String::new());
    };
    if !output.status.success() {
        return (String::new(), String::new());
    }
    let full_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if full_sha.len() < 8 {
        return (String::new(), String::new());
    }
    let short_rev: String = full_sha.chars().take(8).collect();
    let url = format!("https://github.com/project-robius/robrix/tree/{full_sha}");
    (short_rev, url)
}

/// Parses Cargo.lock to find the resolved version of `matrix-sdk`.
///
/// Returns `(version, short_git_rev, url)`.
fn read_matrix_sdk_info() -> (String, String, String) {
    let Ok(lockfile_text) = std::fs::read_to_string("Cargo.lock") else {
        return (String::new(), String::new(), String::new());
    };
    let Ok(lockfile) = toml::from_str::<toml::Value>(&lockfile_text) else {
        return (String::new(), String::new(), String::new());
    };

    let Some(pkg) = lockfile
        .get("package")
        .and_then(|p| p.as_array())
        .and_then(|pkgs| {
            pkgs.iter().find(|p| {
                p.get("name").and_then(|n| n.as_str()) == Some("matrix-sdk")
            })
        })
    else {
        return (String::new(), String::new(), String::new());
    };

    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source = pkg.get("source").and_then(|s| s.as_str()).unwrap_or("");

    // Git sources look like `git+<repo-url>?<query>#<full-commit>`.
    // The repo URL is the prefix before `?` or `#`; the commit is after `#`.
    let (git_rev, url) = if let Some(rest) = source.strip_prefix("git+") {
        let (left, full_commit) = rest.rsplit_once('#').unwrap_or((rest, ""));
        let base = left.split_once('?').map_or(left, |(b, _)| b);
        let short_rev: String = full_commit.chars().take(8).collect();
        let url = if full_commit.is_empty() {
            base.to_string()
        } else {
            format!("{base}/tree/{full_commit}")
        };
        (short_rev, url)
    } else if !version.is_empty() {
        // Registry/path/other sources: fall back to the crates.io URL.
        (String::new(), format!("https://crates.io/crates/matrix-sdk/{version}"))
    } else {
        (String::new(), String::new())
    };

    (version, git_rev, url)
}

/// Set (non-empty, not "0") to forbid host system fonts and link only the
/// bundled OFL fallbacks. Required for every packaged/distributable build.
const BUNDLED_FONTS_ENV: &str = "ROBRIX_BUNDLED_FONTS";

/// Resolve the CJK face the UI should use into one stable path,
/// `resources/fonts/system_cjk.ttc`, which the DSL then references.
///
/// On macOS that is the system PingFang, which users expect Chinese text to
/// look like and which no other bundled face matches. It is **symlinked, never
/// copied**: the bytes stay Apple's, on the user's own disk, so nothing about
/// this redistributes the font. Everywhere else — and on any Mac where the
/// font is missing — the link points at the bundled LXGWWenKai instead, so the
/// DSL path always resolves and font loading cannot panic.
///
/// PingFang ships only inside a private framework and only as a 32-face
/// collection. On macOS 26+ Apple rebuilt it around the proprietary `hvgl`
/// (hierarchical variable glyphs) table with **no** `glyf`/`CFF` at all,
/// which no open-source parser can outline. Makepad's text stack handles
/// that on macOS via a CoreText outline fallback (see the fork's
/// `draw/src/text/coretext.rs`), so hvgl-only files are accepted when
/// targeting macOS; everywhere else `outline_format_supported` skips them —
/// they would silently render blank — and the next candidate (Hiragino Sans
/// GB, the pre-PingFang system Chinese sans, CFF) is used instead.
///
/// **Packaged builds must set `ROBRIX_BUNDLED_FONTS=1`.** Every packager in
/// use (`cargo makepad android|apple`, `cargo packager` via
/// robius-packaging-commands) copies `resources/` *by value*, dereferencing
/// symlinks — and on Windows the link is already a real copy. Without the
/// switch a macOS `.app` would carry PingFang and a Windows installer
/// Microsoft YaHei: redistribution of proprietary fonts. With it, host fonts
/// are never considered and both links resolve to the OFL-licensed bundled
/// faces, so a distributable ships exactly one copy of each and nothing
/// proprietary. `cargo run` from a checkout keeps using the system fonts.
///
/// The bundled CJK fallback lives in `fonts/bundled/`, *outside* `resources/`,
/// precisely so that packagers do not ship it a second time under its own
/// name next to the materialised `system_cjk.ttc`.
fn link_system_cjk_font(target_os: &str) {
    println!("cargo:rerun-if-changed=resources/fonts");
    println!("cargo:rerun-if-changed=fonts/bundled");
    println!("cargo:rerun-if-env-changed={BUNDLED_FONTS_ENV}");

    let bundled_only = std::env::var_os(BUNDLED_FONTS_ENV)
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false);
    // Surface the decision to the crate (tests assert on it) — build.rs is
    // the only place that sees the build-time environment.
    println!(
        "cargo:rustc-env={BUNDLED_FONTS_ENV}={}",
        if bundled_only { "1" } else { "" }
    );
    if bundled_only {
        println!("cargo:warning=font: {BUNDLED_FONTS_ENV} set — using bundled fonts only");
    }

    // Apple keeps PingFang inside a private framework rather than in
    // /System/Library/Fonts. The path is private API surface, so treat its
    // absence as normal and fall back.
    const MACOS_PINGFANG: &str =
        "/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/PingFangUI.ttc";

    // SFNS.ttf is San Francisco, the macOS UI face. Single-face (so makepad's
    // hardcoded face index 0 is right) and variable, so one file serves both
    // regular and bold via the `wght` axis.
    //
    // On Linux, font layout under /usr/share/fonts is distro-specific
    // (Debian: `truetype/<pkg>/`, `opentype/<pkg>/`; Arch: `<pkg>/` or `TTF/`;
    // Fedora: `<pkg>/`; NixOS: elsewhere entirely), so hardcoded paths alone
    // are only ever right on one family of distros. Ask fontconfig first —
    // it is the one thing every desktop Linux agrees on — and keep the
    // hardcoded paths as a second line for machines without `fc-match`.
    let mut cjk_candidates: Vec<String> = Vec::new();
    let mut latin_candidates: Vec<String> = Vec::new();
    match target_os {
        _ if bundled_only => {}
        "macos" => {
            cjk_candidates.extend(
                [
                    MACOS_PINGFANG,
                    "/System/Library/Fonts/Hiragino Sans GB.ttc",
                    "/System/Library/Fonts/STHeiti Light.ttc",
                ]
                .map(String::from),
            );
            latin_candidates.extend(
                ["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/HelveticaNeue.ttc"]
                    .map(String::from),
            );
        }
        "windows" => {
            cjk_candidates
                .extend(["C:/Windows/Fonts/msyh.ttc", "C:/Windows/Fonts/simhei.ttf"].map(String::from));
            latin_candidates
                .extend(["C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/arial.ttf"].map(String::from));
        }
        // Only the host's own desktop Linux may borrow host fonts. build.rs
        // runs on the host but `target_os` is the *target*: for android /
        // ios / ohos cross-builds (CI does Android on Ubuntu) a fontconfig
        // query here would write the host's font path into the symlink and
        // either ship a host font in the package or ship a dangling link.
        // Those targets get an empty candidate list and always take the
        // bundled fallback.
        "linux" => {
            cjk_candidates.extend(fc_match(&["sans-serif:lang=zh-cn", "sans-serif:lang=zh"], Some("zh-cn")));
            cjk_candidates.extend(
                [
                    // Debian / Ubuntu
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                    // Arch (noto-fonts-cjk, wqy-microhei)
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
                    // Fedora (google-noto-sans-cjk-fonts)
                    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/google-noto-sans-cjk-fonts/NotoSansCJK-Regular.ttc",
                ]
                .map(String::from),
            );
            latin_candidates.extend(fc_match(&["sans-serif:lang=en", "sans-serif"], Some("en")));
            latin_candidates.extend(
                [
                    // Debian / Ubuntu
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                    // Arch (ttf-dejavu, ttf-liberation, noto-fonts)
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                    "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
                    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                    // Fedora
                    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
                    "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
                    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
                ]
                .map(String::from),
            );
        }
        _ => {}
    }

    // Makepad's CoreText fallback makes hvgl-only fonts renderable, but only
    // on macOS builds.
    let allow_hvgl = target_os == "macos";
    link_font(
        "resources/fonts/system_cjk.ttc",
        &cjk_candidates,
        "fonts/bundled/LXGWWenKaiRegular.ttf",
        allow_hvgl,
    );
    link_font(
        "resources/fonts/system_latin.ttf",
        &latin_candidates,
        "resources/fonts/LiberationMono-Regular.ttf",
        allow_hvgl,
    );
}

/// Ask fontconfig which file it would use for each `pattern`, in order,
/// then — if `required_lang` is set — every installed font that actually
/// covers that language (`fc-list :lang=<x>`), best-looking first.
///
/// `fc-match` treats `:lang=` as a preference, not a constraint: a user
/// fontconfig that pins `sans-serif` to one family (Omarchy binds it to
/// Liberation Sans) makes it return that family for `sans-serif:lang=zh-cn`
/// too, even though it has no CJK glyphs. So each `fc-match` answer is
/// checked against its own `%{lang}` coverage and dropped if it lacks
/// `required_lang`; the `fc-list` sweep then finds a real match by coverage.
///
/// Returns whatever resolved (deduplicated, possibly empty) — fontconfig
/// missing or failing is normal on minimal systems and simply yields nothing,
/// leaving the hardcoded candidates to try. Only sfnt-looking files are kept
/// (bitmap / Type1 fonts are useless to makepad); `outline_format_supported`
/// filters the rest downstream.
fn fc_match(patterns: &[&str], required_lang: Option<&str>) -> Vec<String> {
    use std::process::Command;

    fn is_sfnt(file: &str) -> bool {
        let lower = file.to_ascii_lowercase();
        lower.ends_with(".ttf") || lower.ends_with(".ttc") || lower.ends_with(".otf")
    }
    fn covers(lang_list: &str, lang: Option<&str>) -> bool {
        match lang {
            None => true,
            Some(lang) => lang_list.split('|').any(|l| l.trim().eq_ignore_ascii_case(lang)),
        }
    }
    fn push(out: &mut Vec<String>, file: &str) {
        if !file.is_empty() && is_sfnt(file) && !out.iter().any(|f| f == file) {
            out.push(file.to_string());
        }
    }

    let mut out: Vec<String> = Vec::new();
    for pattern in patterns {
        let Ok(output) = Command::new("fc-match")
            .args(["-f", "%{file}\t%{lang}", pattern])
            .output()
        else {
            return out; // fc-match not installed; fontconfig is unavailable
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parts = text.trim().splitn(2, '\t');
        let file = parts.next().unwrap_or("").trim();
        let langs = parts.next().unwrap_or("");
        if covers(langs, required_lang) {
            push(&mut out, file);
        }
    }

    // Coverage sweep: every font that really has glyphs for the language.
    // Prefer well-known UI sans faces in a Regular weight; makepad only
    // reads face 0 of a collection and has no synthetic bold, so a face that
    // is Light/Bold-only would render every label in that weight.
    if let Some(lang) = required_lang {
        if let Ok(output) = Command::new("fc-list")
            .args(["-f", "%{file}\t%{family}\t%{style}\n", &format!(":lang={lang}")])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut scored: Vec<(u32, String)> = Vec::new();
                for line in text.lines() {
                    let mut cols = line.split('\t');
                    let file = cols.next().unwrap_or("").trim();
                    let family = cols.next().unwrap_or("").to_ascii_lowercase();
                    let style = cols.next().unwrap_or("").to_ascii_lowercase();
                    if !is_sfnt(file) {
                        continue;
                    }
                    let styles: Vec<&str> = style.split(',').map(str::trim).collect();
                    // "Regular" alone, not "Medium,Regular" (fontconfig's alias
                    // for a weight's un-styled name).
                    let regular = styles.contains(&"regular") && styles.len() == 1
                        || styles == ["regular", "regular"];
                    if !regular {
                        continue;
                    }
                    let mut score = 0u32;
                    if family.contains("serif") && !family.contains("sans") {
                        continue; // serif faces are not a UI font
                    }
                    if family.contains("mono") {
                        continue;
                    }
                    if family.contains("noto sans cjk") || family.contains("source han sans") {
                        score += 30;
                    }
                    if family.contains("noto sans") || family.contains("dejavu sans")
                        || family.contains("liberation sans") || family.contains("wenquanyi")
                        || family.contains("wqy")
                    {
                        score += 20;
                    }
                    if family.contains(" sc") || family.contains("simplified") {
                        score += 5;
                    }
                    if !scored.iter().any(|(_, f)| f == file) {
                        scored.push((score, file.to_string()));
                    }
                }
                scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
                for (_, file) in scored {
                    push(&mut out, &file);
                }
            }
        }
    }
    out
}

/// Point `link` at the first usable `candidates` entry, else at `fallback`.
///
/// "Usable" means the file exists AND face 0 carries outlines the text stack
/// can render: `glyf`, `CFF ` or `CFF2` everywhere, plus Apple's proprietary
/// `hvgl` when `allow_hvgl` (macOS targets, where makepad's CoreText
/// fallback decodes it — e.g. PingFangUI.ttc on macOS 26). Elsewhere
/// hvgl-only fonts parse fine but produce zero glyphs, so text silently
/// renders blank — skip them.
///
/// Symlinked, never copied: the bytes stay the platform's, on the user's own
/// disk, so nothing here redistributes a system font. The link always resolves
/// to *something*, because the DSL references it unconditionally and makepad
/// panics on a font face it cannot load.
///
/// The link target is always made absolute. A symlink's target is resolved
/// relative to the *link's own directory*, so writing the crate-relative
/// `resources/fonts/X.ttf` verbatim would produce a link that resolves to
/// `resources/fonts/resources/fonts/X.ttf` — a dangling link, and a blank UI.
fn link_font(link: &str, candidates: &[String], fallback: &str, allow_hvgl: bool) {
    use std::path::{Path, PathBuf};

    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let absolute = |p: &Path| -> PathBuf {
        let p = if p.is_absolute() { p.to_path_buf() } else { manifest_dir.join(p) };
        // Canonicalize when possible so `read_link == target` below stays
        // stable across builds; keep the joined path if the file is missing.
        std::fs::canonicalize(&p).unwrap_or(p)
    };

    // The link itself must NOT be canonicalized: that follows an existing
    // symlink and would make `link` the system font path itself.
    let link = manifest_dir.join(link);
    let target = candidates
        .iter()
        .map(|c| absolute(Path::new(c)))
        .find(|p| p.is_file() && outline_format_supported(p, allow_hvgl))
        .unwrap_or_else(|| absolute(Path::new(fallback)));

    if !target.is_file() {
        println!("cargo:warning={} not linked: no font found", link.display());
        return;
    }

    // Already correct? Two shapes must be recognised, and either may be left
    // over from a previous build in the *other* mode (or, on Windows, from a
    // checkout where the file is a plain copy):
    //   - a symlink whose target is exactly `target` (unix), or
    //   - a regular file whose bytes equal `target` (Windows copy mode).
    // Anything else — dangling link, link to another font, stale copy — is
    // removed and rebuilt. `remove_file` unlinks a symlink itself (never its
    // target) and deletes a regular file, so one call covers both shapes.
    let up_to_date = match std::fs::symlink_metadata(&link) {
        Err(_) => false,
        Ok(meta) if meta.file_type().is_symlink() => {
            std::fs::read_link(&link).ok().as_deref() == Some(target.as_path())
        }
        Ok(meta) => {
            !cfg!(unix) && meta.is_file() && same_contents(&link, &target)
        }
    };
    if up_to_date {
        return;
    }
    let _ = std::fs::remove_file(&link);

    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&target, &link);
    #[cfg(not(unix))]
    let linked = std::fs::copy(&target, &link).map(|_| ());

    match linked {
        Ok(()) => println!("cargo:warning=font: {} -> {}", link.display(), target.display()),
        Err(e) => println!("cargo:warning=could not link {}: {e}", link.display()),
    }
}

/// Byte-for-byte equality of two files (used only where the link is a real
/// copy, i.e. Windows). Sizes are compared first so a mismatch is cheap;
/// fonts are tens of MB at most, so a full read on match is acceptable for
/// a build script that only reaches this when the file already exists.
fn same_contents(a: &std::path::Path, b: &std::path::Path) -> bool {
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else { return false };
    if ma.len() != mb.len() {
        return false;
    }
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(da), Ok(db)) => da == db,
        _ => false,
    }
}

/// Does face 0 of this font file carry an outline table the text stack can
/// render — `glyf`, `CFF ` or `CFF2` (ttf_parser), or `hvgl` when
/// `allow_hvgl` (macOS CoreText fallback)? Handles both bare sfnt files and
/// `ttcf` collections. Any parse hiccup returns false — the caller then just
/// tries the next candidate, which is always the safe direction.
fn outline_format_supported(path: &std::path::Path, allow_hvgl: bool) -> bool {
    let Ok(data) = std::fs::read(path) else { return false };
    let be32 = |off: usize| -> Option<u32> {
        data.get(off..off + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let be16 = |off: usize| -> Option<u16> {
        data.get(off..off + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    };
    let face_off = if data.get(..4) == Some(b"ttcf") {
        match be32(12) {
            Some(off) => off as usize,
            None => return false,
        }
    } else {
        0
    };
    let Some(num_tables) = be16(face_off + 4) else { return false };
    (0..num_tables as usize).any(|i| {
        let rec = face_off + 12 + 16 * i;
        match data.get(rec..rec + 4) {
            Some(b"glyf") | Some(b"CFF ") | Some(b"CFF2") => true,
            Some(b"hvgl") => allow_hvgl,
            _ => false,
        }
    })
}
