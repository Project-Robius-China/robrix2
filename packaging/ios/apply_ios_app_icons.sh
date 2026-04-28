#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "Usage: $0 <path-to-robrix.app> [build-number]"
    echo "Example: $0 ./target/makepad-apple-app/aarch64-apple-ios/release/robrix.app 42"
    exit 1
fi

APP_BUNDLE_PATH="$1"
BUILD_NUMBER="${2:-1}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ASSET_CATALOG_PATH="$REPO_ROOT/packaging/ios/icons/Assets.xcassets"
IOS_INFO_PLIST_PATCH="$REPO_ROOT/packaging/Info-iOS.plist"
TARGET_INFO_PLIST="$APP_BUNDLE_PATH/Info.plist"
ASSET_INFO_PLIST="/tmp/robrix-AssetInfo.plist"

if [[ ! -d "$APP_BUNDLE_PATH" ]]; then
    echo "Error: app bundle not found: $APP_BUNDLE_PATH"
    exit 1
fi

if [[ ! -f "$TARGET_INFO_PLIST" ]]; then
    echo "Error: Info.plist not found: $TARGET_INFO_PLIST"
    exit 1
fi

if [[ ! -d "$ASSET_CATALOG_PATH" ]]; then
    echo "Error: asset catalog not found: $ASSET_CATALOG_PATH"
    exit 1
fi

if [[ ! -f "$IOS_INFO_PLIST_PATCH" ]]; then
    echo "Error: iOS plist patch file not found: $IOS_INFO_PLIST_PATCH"
    exit 1
fi

echo "Compiling iOS asset catalog..."
xcrun actool "$ASSET_CATALOG_PATH" \
    --compile "$APP_BUNDLE_PATH" \
    --platform iphoneos \
    --minimum-deployment-target 14.0 \
    --app-icon AppIcon \
    --output-partial-info-plist "$ASSET_INFO_PLIST"

echo "Merging asset metadata into Info.plist..."
/usr/libexec/PlistBuddy -c "Merge $ASSET_INFO_PLIST" "$TARGET_INFO_PLIST"

echo "Applying iOS-specific plist keys..."
/usr/libexec/PlistBuddy -c "Merge $IOS_INFO_PLIST_PATCH" "$TARGET_INFO_PLIST"

APP_VERSION="$(awk '
    /^\[package\]$/ { in_package=1; next }
    /^\[/ { in_package=0 }
    in_package && $1 == "version" {
        gsub(/"/, "", $3);
        print $3;
        exit;
    }
' "$REPO_ROOT/Cargo.toml" | sed 's/-.*$//')"

if [[ -z "$APP_VERSION" ]]; then
    APP_VERSION="0.0.1"
fi

echo "Setting version keys: CFBundleShortVersionString=$APP_VERSION, CFBundleVersion=$BUILD_NUMBER"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $APP_VERSION" "$TARGET_INFO_PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $BUILD_NUMBER" "$TARGET_INFO_PLIST"

echo "Done. App icon assets were compiled into $APP_BUNDLE_PATH"
echo "Note: if the app was already signed, re-sign it after this step."
