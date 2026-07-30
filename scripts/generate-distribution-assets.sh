#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPOSITORY="Project-Robius-China/robrix2"
TAG=""
ASSETS_JSON=""
OUTPUT_DIR=""

usage() {
    cat <<'EOF'
generate-distribution-assets.sh

Generate checksum-pinned Robrix installers and package-manager metadata.

USAGE:
    scripts/generate-distribution-assets.sh \
        --tag v1.2.3 \
        --assets-json release-assets.json \
        --output dist/distribution \
        [--repository owner/repository]

The assets JSON can be either a GitHub release object or its .assets array.
Each required desktop asset must include name, digest, and browser_download_url.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repository)
            REPOSITORY=${2:?missing repository}
            shift 2
            ;;
        --tag)
            TAG=${2:?missing tag}
            shift 2
            ;;
        --assets-json)
            ASSETS_JSON=${2:?missing assets JSON path}
            shift 2
            ;;
        --output)
            OUTPUT_DIR=${2:?missing output directory}
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

[[ -n "$TAG" ]] || { echo "--tag is required" >&2; exit 2; }
[[ "$TAG" == v* ]] || { echo "--tag must start with v" >&2; exit 2; }
[[ -f "$ASSETS_JSON" ]] || { echo "--assets-json must name a readable file" >&2; exit 2; }
[[ -n "$OUTPUT_DIR" ]] || { echo "--output is required" >&2; exit 2; }

for command in jq npm sed zip; do
    command -v "$command" >/dev/null 2>&1 ||
        { echo "$command is required" >&2; exit 1; }
done

VERSION=${TAG#v}
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] ||
    { echo "unsupported release version: $VERSION" >&2; exit 2; }
[[ "$REPOSITORY" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] ||
    { echo "invalid GitHub repository: $REPOSITORY" >&2; exit 2; }

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
NORMALIZED_ASSETS="$OUTPUT_DIR/.release-assets.json"

jq -e '
    (if type == "array" then . else .assets end)
    | if type == "array" then . else error("assets JSON must contain an array") end
' "$ASSETS_JSON" > "$NORMALIZED_ASSETS"

asset_record() {
    local candidate
    local record
    for candidate in "$@"; do
        if record=$(jq -ce --arg name "$candidate" '.[] | select(.name == $name)' "$NORMALIZED_ASSETS"); then
            printf '%s\n' "$record"
            return 0
        fi
    done
    echo "missing release asset; expected one of: $*" >&2
    return 1
}

record_name() {
    jq -r '.name' <<<"$1"
}

record_url() {
    local url
    local expected_prefix="https://github.com/${REPOSITORY}/releases/download/${TAG}/"
    url=$(jq -r '.browser_download_url // ""' <<<"$1")
    [[ "$url" == "${expected_prefix}"* ]] ||
        { echo "unexpected release asset URL: $url" >&2; return 1; }
    printf '%s\n' "$url"
}

record_sha256() {
    local digest
    digest=$(jq -r '.digest // ""' <<<"$1")
    [[ "$digest" =~ ^sha256:[0-9a-fA-F]{64}$ ]] ||
        { echo "asset has no GitHub SHA-256 digest: $(record_name "$1")" >&2; return 1; }
    printf '%s\n' "${digest#sha256:}" | tr '[:upper:]' '[:lower:]'
}

MACOS_AARCH64_RECORD=$(asset_record "robrix-${VERSION}-macos-aarch64-release.dmg")
MACOS_X86_64_RECORD=$(asset_record "robrix-${VERSION}-macos-x86_64-release.dmg")
WINDOWS_X86_64_RECORD=$(asset_record "robrix-${VERSION}-windows-x86_64-release.exe")

LINUX_22_AARCH64_RECORD=$(asset_record \
    "robrix-${VERSION}-ubuntu-22.04-aarch64-release.deb" \
    "robrix-${VERSION}-linux-aarch64-release.deb")
LINUX_22_X86_64_RECORD=$(asset_record \
    "robrix-${VERSION}-ubuntu-22.04-x86_64-release.deb" \
    "robrix-${VERSION}-linux-x86_64-release.deb")
LINUX_24_AARCH64_RECORD=$(asset_record \
    "robrix-${VERSION}-ubuntu-24.04-aarch64-release.deb" \
    "robrix-${VERSION}-linux-aarch64-release.deb")
LINUX_24_X86_64_RECORD=$(asset_record \
    "robrix-${VERSION}-ubuntu-24.04-x86_64-release.deb" \
    "robrix-${VERSION}-linux-x86_64-release.deb")

MACOS_AARCH64_ASSET=$(record_name "$MACOS_AARCH64_RECORD")
MACOS_AARCH64_URL=$(record_url "$MACOS_AARCH64_RECORD")
MACOS_AARCH64_SHA256=$(record_sha256 "$MACOS_AARCH64_RECORD")
MACOS_X86_64_ASSET=$(record_name "$MACOS_X86_64_RECORD")
MACOS_X86_64_URL=$(record_url "$MACOS_X86_64_RECORD")
MACOS_X86_64_SHA256=$(record_sha256 "$MACOS_X86_64_RECORD")
WINDOWS_X86_64_ASSET=$(record_name "$WINDOWS_X86_64_RECORD")
WINDOWS_X86_64_URL=$(record_url "$WINDOWS_X86_64_RECORD")
WINDOWS_X86_64_SHA256=$(record_sha256 "$WINDOWS_X86_64_RECORD")

LINUX_22_AARCH64_ASSET=$(record_name "$LINUX_22_AARCH64_RECORD")
LINUX_22_AARCH64_URL=$(record_url "$LINUX_22_AARCH64_RECORD")
LINUX_22_AARCH64_SHA256=$(record_sha256 "$LINUX_22_AARCH64_RECORD")
LINUX_22_X86_64_ASSET=$(record_name "$LINUX_22_X86_64_RECORD")
LINUX_22_X86_64_URL=$(record_url "$LINUX_22_X86_64_RECORD")
LINUX_22_X86_64_SHA256=$(record_sha256 "$LINUX_22_X86_64_RECORD")
LINUX_24_AARCH64_ASSET=$(record_name "$LINUX_24_AARCH64_RECORD")
LINUX_24_AARCH64_URL=$(record_url "$LINUX_24_AARCH64_RECORD")
LINUX_24_AARCH64_SHA256=$(record_sha256 "$LINUX_24_AARCH64_RECORD")
LINUX_24_X86_64_ASSET=$(record_name "$LINUX_24_X86_64_RECORD")
LINUX_24_X86_64_URL=$(record_url "$LINUX_24_X86_64_RECORD")
LINUX_24_X86_64_SHA256=$(record_sha256 "$LINUX_24_X86_64_RECORD")

replace_placeholder() {
    local file=$1
    local placeholder=$2
    local value=$3
    local escaped
    local temporary="${file}.tmp"

    escaped=$(printf '%s' "$value" | sed 's/[&|\\]/\\&/g')
    sed "s|${placeholder}|${escaped}|g" "$file" > "$temporary"
    mv "$temporary" "$file"
}

render_template() {
    local template=$1
    local destination=$2

    cp "$template" "$destination"
    replace_placeholder "$destination" "@VERSION@" "$VERSION"
    replace_placeholder "$destination" "@TAG@" "$TAG"
    replace_placeholder "$destination" "@REPOSITORY@" "$REPOSITORY"
    replace_placeholder "$destination" "@MACOS_AARCH64_ASSET@" "$MACOS_AARCH64_ASSET"
    replace_placeholder "$destination" "@MACOS_AARCH64_SHA256@" "$MACOS_AARCH64_SHA256"
    replace_placeholder "$destination" "@MACOS_X86_64_ASSET@" "$MACOS_X86_64_ASSET"
    replace_placeholder "$destination" "@MACOS_X86_64_SHA256@" "$MACOS_X86_64_SHA256"
    replace_placeholder "$destination" "@WINDOWS_X86_64_ASSET@" "$WINDOWS_X86_64_ASSET"
    replace_placeholder "$destination" "@WINDOWS_X86_64_SHA256@" "$WINDOWS_X86_64_SHA256"
    replace_placeholder "$destination" "@WINDOWS_X86_64_SHA256_UPPER@" \
        "$(printf '%s' "$WINDOWS_X86_64_SHA256" | tr '[:lower:]' '[:upper:]')"
    replace_placeholder "$destination" "@LINUX_22_AARCH64_ASSET@" "$LINUX_22_AARCH64_ASSET"
    replace_placeholder "$destination" "@LINUX_22_AARCH64_SHA256@" "$LINUX_22_AARCH64_SHA256"
    replace_placeholder "$destination" "@LINUX_22_X86_64_ASSET@" "$LINUX_22_X86_64_ASSET"
    replace_placeholder "$destination" "@LINUX_22_X86_64_SHA256@" "$LINUX_22_X86_64_SHA256"
    replace_placeholder "$destination" "@LINUX_24_AARCH64_ASSET@" "$LINUX_24_AARCH64_ASSET"
    replace_placeholder "$destination" "@LINUX_24_AARCH64_SHA256@" "$LINUX_24_AARCH64_SHA256"
    replace_placeholder "$destination" "@LINUX_24_X86_64_ASSET@" "$LINUX_24_X86_64_ASSET"
    replace_placeholder "$destination" "@LINUX_24_X86_64_SHA256@" "$LINUX_24_X86_64_SHA256"
}

SHELL_INSTALLER="$OUTPUT_DIR/robrix-installer.sh"
POWERSHELL_INSTALLER="$OUTPUT_DIR/robrix-installer.ps1"
render_template \
    "$REPO_ROOT/packaging/distribution/robrix-installer.sh.in" \
    "$SHELL_INSTALLER"
render_template \
    "$REPO_ROOT/packaging/distribution/robrix-installer.ps1.in" \
    "$POWERSHELL_INSTALLER"
chmod +x "$SHELL_INSTALLER"

jq -r '
    .[]
    | select((.digest // "") | test("^sha256:[0-9a-fA-F]{64}$"))
    | select(.name != "SHA256SUMS")
    | select(.name != "robrix-installer.sh")
    | select(.name != "robrix-installer.ps1")
    | select(.name != "robrix-dist-manifest.json")
    | select(.name | endswith("-npm-package.tgz") | not)
    | select(.name | endswith("-winget-manifests.zip") | not)
    | "\(.digest | sub("^sha256:"; "") | ascii_downcase)  \(.name)"
' "$NORMALIZED_ASSETS" > "$OUTPUT_DIR/SHA256SUMS"

jq -n \
    --arg version "$VERSION" \
    --arg tag "$TAG" \
    --arg repository "$REPOSITORY" \
    --arg macos_arm_asset "$MACOS_AARCH64_ASSET" \
    --arg macos_arm_url "$MACOS_AARCH64_URL" \
    --arg macos_arm_sha "$MACOS_AARCH64_SHA256" \
    --arg macos_x64_asset "$MACOS_X86_64_ASSET" \
    --arg macos_x64_url "$MACOS_X86_64_URL" \
    --arg macos_x64_sha "$MACOS_X86_64_SHA256" \
    --arg linux22_arm_asset "$LINUX_22_AARCH64_ASSET" \
    --arg linux22_arm_url "$LINUX_22_AARCH64_URL" \
    --arg linux22_arm_sha "$LINUX_22_AARCH64_SHA256" \
    --arg linux22_x64_asset "$LINUX_22_X86_64_ASSET" \
    --arg linux22_x64_url "$LINUX_22_X86_64_URL" \
    --arg linux22_x64_sha "$LINUX_22_X86_64_SHA256" \
    --arg linux24_arm_asset "$LINUX_24_AARCH64_ASSET" \
    --arg linux24_arm_url "$LINUX_24_AARCH64_URL" \
    --arg linux24_arm_sha "$LINUX_24_AARCH64_SHA256" \
    --arg linux24_x64_asset "$LINUX_24_X86_64_ASSET" \
    --arg linux24_x64_url "$LINUX_24_X86_64_URL" \
    --arg linux24_x64_sha "$LINUX_24_X86_64_SHA256" \
    --arg windows_x64_asset "$WINDOWS_X86_64_ASSET" \
    --arg windows_x64_url "$WINDOWS_X86_64_URL" \
    --arg windows_x64_sha "$WINDOWS_X86_64_SHA256" \
    '{
        schema_version: 1,
        app: "robrix",
        version: $version,
        tag: $tag,
        repository: $repository,
        release_url: ("https://github.com/" + $repository + "/releases/tag/" + $tag),
        platforms: {
            macos: {
                aarch64: { asset: $macos_arm_asset, url: $macos_arm_url, sha256: $macos_arm_sha },
                x86_64: { asset: $macos_x64_asset, url: $macos_x64_url, sha256: $macos_x64_sha }
            },
            linux: {
                ubuntu_22_04: {
                    aarch64: { asset: $linux22_arm_asset, url: $linux22_arm_url, sha256: $linux22_arm_sha },
                    x86_64: { asset: $linux22_x64_asset, url: $linux22_x64_url, sha256: $linux22_x64_sha }
                },
                ubuntu_24_04: {
                    aarch64: { asset: $linux24_arm_asset, url: $linux24_arm_url, sha256: $linux24_arm_sha },
                    x86_64: { asset: $linux24_x64_asset, url: $linux24_x64_url, sha256: $linux24_x64_sha }
                }
            },
            windows: {
                x86_64: { asset: $windows_x64_asset, url: $windows_x64_url, sha256: $windows_x64_sha }
            }
        }
    }' > "$OUTPUT_DIR/robrix-dist-manifest.json"

mkdir -p "$OUTPUT_DIR/Casks"
render_template \
    "$REPO_ROOT/packaging/homebrew/robrix.rb.in" \
    "$OUTPUT_DIR/Casks/robrix.rb"

NPM_PACKAGE_DIR="$OUTPUT_DIR/npm-package"
mkdir -p "$NPM_PACKAGE_DIR"
render_template "$REPO_ROOT/packaging/npm/package.json.in" "$NPM_PACKAGE_DIR/package.json"
render_template "$REPO_ROOT/packaging/npm/README.md.in" "$NPM_PACKAGE_DIR/README.md"
cp "$REPO_ROOT/packaging/npm/install.js" "$NPM_PACKAGE_DIR/install.js"
cp "$REPO_ROOT/LICENSE-MIT" "$NPM_PACKAGE_DIR/LICENSE-MIT"
cp "$SHELL_INSTALLER" "$NPM_PACKAGE_DIR/robrix-installer.sh"
cp "$POWERSHELL_INSTALLER" "$NPM_PACKAGE_DIR/robrix-installer.ps1"
chmod +x "$NPM_PACKAGE_DIR/install.js" "$NPM_PACKAGE_DIR/robrix-installer.sh"

PACKED_NAME=$(
    cd "$NPM_PACKAGE_DIR"
    npm_config_cache="$OUTPUT_DIR/.npm-cache" \
        npm pack --silent --pack-destination "$OUTPUT_DIR"
)
mv "$OUTPUT_DIR/$PACKED_NAME" "$OUTPUT_DIR/robrix-${VERSION}-npm-package.tgz"

WINGET_DIR="$OUTPUT_DIR/winget/ProjectRobiusChina.Robrix/$VERSION"
mkdir -p "$WINGET_DIR"
render_template \
    "$REPO_ROOT/packaging/winget/ProjectRobiusChina.Robrix.yaml.in" \
    "$WINGET_DIR/ProjectRobiusChina.Robrix.yaml"
render_template \
    "$REPO_ROOT/packaging/winget/ProjectRobiusChina.Robrix.installer.yaml.in" \
    "$WINGET_DIR/ProjectRobiusChina.Robrix.installer.yaml"
render_template \
    "$REPO_ROOT/packaging/winget/ProjectRobiusChina.Robrix.locale.en-US.yaml.in" \
    "$WINGET_DIR/ProjectRobiusChina.Robrix.locale.en-US.yaml"

(
    cd "$OUTPUT_DIR/winget"
    zip -qr "$OUTPUT_DIR/robrix-${VERSION}-winget-manifests.zip" .
)

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

for generated_asset in \
    "$SHELL_INSTALLER" \
    "$POWERSHELL_INSTALLER" \
    "$OUTPUT_DIR/robrix-dist-manifest.json" \
    "$OUTPUT_DIR/robrix-${VERSION}-npm-package.tgz" \
    "$OUTPUT_DIR/robrix-${VERSION}-winget-manifests.zip"; do
    printf '%s  %s\n' \
        "$(sha256_file "$generated_asset")" \
        "$(basename "$generated_asset")" >> "$OUTPUT_DIR/SHA256SUMS"
done
LC_ALL=C sort -o "$OUTPUT_DIR/SHA256SUMS" "$OUTPUT_DIR/SHA256SUMS"

rm -rf "$OUTPUT_DIR/.npm-cache"
rm -f "$NORMALIZED_ASSETS"
printf 'Generated Robrix %s distribution assets in %s\n' "$VERSION" "$OUTPUT_DIR"
