#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/robrix-distribution-test.XXXXXX")

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

"$SCRIPT_DIR/generate-distribution-assets.sh" \
    --repository Project-Robius-China/robrix2 \
    --tag v1.1.0 \
    --assets-json "$REPO_ROOT/packaging/distribution/testdata/v1.1.0-assets.json" \
    --output "$TEMP_DIR/output"

OUTPUT="$TEMP_DIR/output"

bash -n "$OUTPUT/robrix-installer.sh"
jq -e '
    .schema_version == 1
    and .version == "1.1.0"
    and .platforms.macos.aarch64.sha256 == "783375cd75c8fc3ad38ccc3ad70e8368faf83606ad4788b99118cf6af6404e7a"
    and .platforms.windows.x86_64.sha256 == "82171be7936cc97965f02baf20cef2c820b5bd2890e80c39fe1e034480d047e2"
' "$OUTPUT/robrix-dist-manifest.json" >/dev/null

assert_selection() {
    local expected_asset=$1
    shift
    local selection
    selection=$(env ROBRIX_INSTALLER_DRY_RUN=1 "$@" sh "$OUTPUT/robrix-installer.sh")
    grep -F "asset=${expected_asset}" <<<"$selection" >/dev/null
}

assert_selection \
    "robrix-1.1.0-macos-aarch64-release.dmg" \
    ROBRIX_INSTALLER_OS=darwin ROBRIX_INSTALLER_ARCH=arm64
assert_selection \
    "robrix-1.1.0-macos-x86_64-release.dmg" \
    ROBRIX_INSTALLER_OS=darwin ROBRIX_INSTALLER_ARCH=x86_64
assert_selection \
    "robrix-1.1.0-linux-aarch64-release.deb" \
    ROBRIX_INSTALLER_OS=linux ROBRIX_INSTALLER_ARCH=aarch64 ROBRIX_LINUX_RELEASE=22.04
assert_selection \
    "robrix-1.1.0-linux-x86_64-release.deb" \
    ROBRIX_INSTALLER_OS=linux ROBRIX_INSTALLER_ARCH=x86_64 ROBRIX_LINUX_RELEASE=24.04

DISTRO_OUTPUT="$TEMP_DIR/distro-output"
"$SCRIPT_DIR/generate-distribution-assets.sh" \
    --repository Project-Robius-China/robrix2 \
    --tag v1.2.0 \
    --assets-json "$REPO_ROOT/packaging/distribution/testdata/v1.2.0-assets.json" \
    --output "$DISTRO_OUTPUT"

distro_22_selection=$(
    ROBRIX_INSTALLER_DRY_RUN=1 \
    ROBRIX_INSTALLER_OS=linux \
    ROBRIX_INSTALLER_ARCH=x86_64 \
    ROBRIX_LINUX_RELEASE=22.04 \
        sh "$DISTRO_OUTPUT/robrix-installer.sh"
)
grep -F 'asset=robrix-1.2.0-ubuntu-22.04-x86_64-release.deb' \
    <<<"$distro_22_selection" >/dev/null

distro_24_selection=$(
    ROBRIX_INSTALLER_DRY_RUN=1 \
    ROBRIX_INSTALLER_OS=linux \
    ROBRIX_INSTALLER_ARCH=x86_64 \
    ROBRIX_LINUX_RELEASE=24.04 \
        sh "$DISTRO_OUTPUT/robrix-installer.sh"
)
grep -F 'asset=robrix-1.2.0-ubuntu-24.04-x86_64-release.deb' \
    <<<"$distro_24_selection" >/dev/null

ROBRIX_NPM_SKIP_INSTALL=1 node "$OUTPUT/npm-package/install.js" >/dev/null
ruby -c "$OUTPUT/Casks/robrix.rb" >/dev/null
ruby -c "$REPO_ROOT/Casks/robrix.rb" >/dev/null

for manifest in "$OUTPUT"/winget/ProjectRobiusChina.Robrix/1.1.0/*.yaml; do
    ruby -e 'require "yaml"; YAML.safe_load(File.read(ARGV.fetch(0)), aliases: false)' "$manifest"
done

if command -v pwsh >/dev/null 2>&1; then
    ROBRIX_INSTALLER_DRY_RUN=1 ROBRIX_INSTALLER_ARCH=AMD64 \
        pwsh -NoProfile -File "$OUTPUT/robrix-installer.ps1" |
        grep -F 'asset=robrix-1.1.0-windows-x86_64-release.exe' >/dev/null
fi

if rg -n '@[A-Z0-9_]+@' \
    "$OUTPUT/robrix-installer.sh" \
    "$OUTPUT/robrix-installer.ps1" \
    "$OUTPUT/Casks/robrix.rb" \
    "$OUTPUT/npm-package" \
    "$OUTPUT/winget"; then
    echo "unresolved distribution template placeholder" >&2
    exit 1
fi

tar -tzf "$OUTPUT/robrix-1.1.0-npm-package.tgz" |
    grep -F 'package/robrix-installer.sh' >/dev/null
unzip -Z1 "$OUTPUT/robrix-1.1.0-winget-manifests.zip" |
    grep -F 'ProjectRobiusChina.Robrix.installer.yaml' >/dev/null
grep -F '  robrix-installer.sh' "$OUTPUT/SHA256SUMS" >/dev/null
grep -F '  robrix-1.1.0-npm-package.tgz' "$OUTPUT/SHA256SUMS" >/dev/null

echo "Robrix distribution tests passed."
