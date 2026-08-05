#!/usr/bin/env bash
# Re-applies the `Html::is_interactive() -> false` fix to the local cargo
# checkout of makepad.
#
# WHY THIS EXISTS
# ---------------
# Message text selection needs it. A selectable `PortalList` refuses to begin a
# drag-selection over anything `find_interactive_widget_from_point` reports as
# interactive, and `Html` inherits `Widget::is_interactive`'s default `true` —
# unlike `Markdown` and `TextFlow`, which both return `false`. So every press on
# an HTML message body became a drag-to-scroll and no selection was created.
#
# The real fix is upstream: https://github.com/ZhangHanDong/makepad/pull/5
# Until that merges and `Cargo.toml`'s `[patch]` picks up the new rev, this
# script is what makes selection work locally.
#
# THIS IS TEMPORARY AND NOT DURABLE. Editing a cargo git checkout is invisible
# to everyone else: a fresh clone, another machine, and CI will all build
# without it, and selection will silently not work there. Delete this script
# once the PR has merged.
#
# Usage:  ./scripts/patch-makepad-html-selectable.sh
set -euo pipefail

HTML_RS=$(find "${CARGO_HOME:-$HOME/.cargo}/git/checkouts" \
    -path '*/makepad-*/*/widgets/src/html.rs' -print -quit)

if [[ -z "${HTML_RS}" ]]; then
    echo "error: makepad checkout not found — run 'cargo build' once first." >&2
    exit 1
fi

if grep -q 'fn is_interactive' "${HTML_RS}"; then
    echo "already patched: ${HTML_RS}"
    exit 0
fi

python3 - "${HTML_RS}" <<'PY'
import sys
path = sys.argv[1]
src = open(path).read()
anchor = "impl Widget for Html {\n"
if anchor not in src:
    sys.exit("error: could not find 'impl Widget for Html' — makepad rev changed?")
patch = anchor + """    /// `Html` is a text container, not a click target. See
    /// scripts/patch-makepad-html-selectable.sh.
    fn is_interactive(&self) -> bool {
        false
    }

"""
open(path, "w").write(src.replace(anchor, patch, 1))
print(f"patched {path}")
PY

# Cargo fingerprints git dependencies by rev, so an edited checkout is ignored
# until the crate is forced to rebuild.
cargo clean -p makepad-widgets
echo "done — run 'cargo build' to pick it up."
