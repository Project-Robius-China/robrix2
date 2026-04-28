# Robrix iOS Icon Packaging

This directory contains iOS app icon assets and a helper script for patching a built `.app` bundle.

## Files

- `icons/Assets.xcassets/AppIcon.appiconset/` — iOS AppIcon asset catalog files
- `../Info-iOS.plist` — iOS-specific Info.plist keys
- `apply_ios_app_icons.sh` — compiles assets and patches Info.plist

## Usage

1. Build Robrix for iOS first:

```bash
cargo makepad apple ios \
  --org=rs.robius \
  --app=robrix \
  run-device -p robrix --release
```

2. Patch the built app bundle to add AppIcon metadata and iOS plist keys:

```bash
./packaging/ios/apply_ios_app_icons.sh \
  ./target/makepad-apple-app/aarch64-apple-ios/release/robrix.app \
  1
```

After this, the bundle contains compiled icon assets (`Assets.car`) and required icon plist entries (`CFBundleIcons`, `CFBundleIconName`, etc.).

If the app was already code-signed, re-sign after this patch step.
