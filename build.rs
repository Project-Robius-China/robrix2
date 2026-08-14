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
fn link_system_cjk_font(target_os: &str) {
    println!("cargo:rerun-if-changed=resources/fonts");

    // Apple keeps PingFang inside a private framework rather than in
    // /System/Library/Fonts. The path is private API surface, so treat its
    // absence as normal and fall back.
    const MACOS_PINGFANG: &str =
        "/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/PingFangUI.ttc";

    // SFNS.ttf is San Francisco, the macOS UI face. Single-face (so makepad's
    // hardcoded face index 0 is right) and variable, so one file serves both
    // regular and bold via the `wght` axis.
    let cjk_candidates: &[&str] = match target_os {
        "macos" => &[
            MACOS_PINGFANG,
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ],
        "windows" => &["C:/Windows/Fonts/msyh.ttc", "C:/Windows/Fonts/simhei.ttf"],
        _ => &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ],
    };
    let latin_candidates: &[&str] = match target_os {
        "macos" => &["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/HelveticaNeue.ttc"],
        "windows" => &["C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/arial.ttf"],
        _ => &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ],
    };

    // Makepad's CoreText fallback makes hvgl-only fonts renderable, but only
    // on macOS builds.
    let allow_hvgl = target_os == "macos";
    link_font(
        "resources/fonts/system_cjk.ttc",
        cjk_candidates,
        "resources/fonts/LXGWWenKaiRegular.ttf",
        allow_hvgl,
    );
    link_font(
        "resources/fonts/system_latin.ttf",
        latin_candidates,
        "resources/fonts/LiberationMono-Regular.ttf",
        allow_hvgl,
    );
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
fn link_font(link: &str, candidates: &[&str], fallback: &str, allow_hvgl: bool) {
    use std::path::Path;

    let link = Path::new(link);
    let target = candidates
        .iter()
        .map(Path::new)
        .find(|p| p.exists() && outline_format_supported(p, allow_hvgl))
        .unwrap_or_else(|| Path::new(fallback));

    if !target.exists() {
        println!("cargo:warning={} not linked: no font found", link.display());
        return;
    }
    if std::fs::read_link(link).ok().as_deref() == Some(target) {
        return;
    }
    let _ = std::fs::remove_file(link);

    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(target, link);
    #[cfg(not(unix))]
    let linked = std::fs::copy(target, link).map(|_| ());

    match linked {
        Ok(()) => println!("cargo:warning=font: {} -> {}", link.display(), target.display()),
        Err(e) => println!("cargo:warning=could not link {}: {e}", link.display()),
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
