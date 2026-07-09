#!/usr/bin/env bash
#
# Build / sign / run Robrix on OpenHarmony / HarmonyOS via cargo-makepad.
#
# Usage:
#   harmony/ohos.sh deveco     # cross-compile librobrix.so + scaffold the DevEco project
#   harmony/ohos.sh build      # assemble the (unsigned) HAP with hvigor
#   harmony/ohos.sh sign       # sign the HAP with the bundled OpenHarmony debug materials
#   harmony/ohos.sh deploy     # install the signed HAP on the connected emulator/device and launch
#   harmony/ohos.sh run        # build + sign + deploy (the usual one-shot)
#   harmony/ohos.sh logs       # stream hilog for the app
#   harmony/ohos.sh shot       # grab a screenshot to harmony/robrix_screen.jpeg
#
# Prereqs: DevEco Studio installed; a HarmonyOS emulator running (or a device via USB).
# Override the DevEco path with:  DEVECO_HOME=/path/to/DevEco-Studio.app/Contents
#
# NOTE: this build depends on local patches to the makepad `dev` checkout that fix
# its OpenHarmony support (see harmony/README.md). Those are NOT yet upstreamed.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

DEVECO_HOME="${DEVECO_HOME:-/Applications/DevEco-Studio.app/Contents}"
ARCH="${OHOS_ARCH:-aarch64}"
CRATE="robrix"
BUNDLE="dev.makepad.${CRATE}"

OHOS="$DEVECO_HOME/sdk/default/openharmony"
SYSROOT="$OHOS/native/sysroot"
LIB="$OHOS/toolchains/lib"
HDC="$OHOS/toolchains/hdc"
JAVA="$DEVECO_HOME/jbr/Contents/Home/bin/java"
JAR="$LIB/hap-sign-tool.jar"

PRJ="$REPO/target/makepad-open-harmony/$CRATE"
OUT="$PRJ/entry/build/default/outputs/default"
HAP_UNSIGNED="$OUT/makepad-default-unsigned.hap"
HAP_SIGNED="$OUT/makepad-default-signed.hap"
SIGN_DIR="$REPO/harmony"

[[ -d "$SYSROOT" ]] || { echo "error: OHOS sysroot missing at $SYSROOT (set DEVECO_HOME)"; exit 1; }

# aws-lc-sys (pulled by matrix-sdk's rustls/aws-lc-rs) runs bindgen with a bare
# libclang that doesn't know the OHOS sysroot, so <stdlib.h> isn't found. Point
# bindgen at the OHOS sysroot + target. Target-suffixed so desktop builds are unaffected.
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos="--target=aarch64-linux-ohos --sysroot=$SYSROOT"
export BINDGEN_EXTRA_CLANG_ARGS="--target=aarch64-linux-ohos --sysroot=$SYSROOT"
export DEVECO_HOME

# The emulator/simulator's virtualized GLES can't do makepad's partial texture
# uploads (they render the font atlas + icons/avatars black). MAKEPAD=ohos_sim
# switches makepad to full texture uploads + emulator EGL. For a REAL device,
# override with an empty value:  MAKEPAD= harmony/ohos.sh run
export MAKEPAD="${MAKEPAD-ohos_sim}"

makepad() { cargo makepad ohos --deveco-home="$DEVECO_HOME" --arch="$ARCH" "$@" -p "$CRATE"; }

cmd_deveco() { makepad deveco; }
cmd_build()  { makepad build; }

cmd_sign() {
  [[ -f "$HAP_UNSIGNED" ]] || { echo "error: no unsigned HAP; run 'build' first"; exit 1; }
  local udid nb na
  udid="$("$HDC" shell bm get --udid 2>/dev/null | tr -d '\r' | tail -1 | tr -d ' ')"
  [[ -n "$udid" ]] || { echo "error: could not read device UDID (is the emulator running?)"; exit 1; }
  nb="$(( $(date +%s) - 86400 ))"; na="$(( $(date +%s) + 63072000 ))"
  echo "signing for udid=$udid"

  # App cert chain: the CA-issued leaf (embedded in the debug profile template) + sub-CA + root.
  local KT="$DEVECO_HOME/jbr/Contents/Home/bin/keytool"
  python3 - "$LIB" "$SIGN_DIR" <<'PY'
import json,sys
lib,out=sys.argv[1],sys.argv[2]
t=json.load(open(f"{lib}/UnsgnedDebugProfileTemplate.json"))
open(f"{out}/_app_leaf.pem","w").write(t["bundle-info"]["development-certificate"])
PY
  "$KT" -exportcert -rfc -alias "openharmony application ca"      -keystore "$LIB/OpenHarmony.p12" -storepass 123456 -file "$SIGN_DIR/_app_subca.pem"  2>/dev/null
  "$KT" -exportcert -rfc -alias "openharmony application root ca" -keystore "$LIB/OpenHarmony.p12" -storepass 123456 -file "$SIGN_DIR/_app_rootca.pem" 2>/dev/null
  cat "$SIGN_DIR/_app_leaf.pem" "$SIGN_DIR/_app_subca.pem" "$SIGN_DIR/_app_rootca.pem" > "$SIGN_DIR/app-signing-cert.pem"

  # Debug provisioning profile: this bundle + this device UDID + fresh validity.
  python3 - "$nb" "$na" "$udid" "$BUNDLE" "$SIGN_DIR" <<'PY'
import json,sys
nb,na,udid,bundle,out=int(sys.argv[1]),int(sys.argv[2]),sys.argv[3],sys.argv[4],sys.argv[5]
leaf=open(f"{out}/_app_leaf.pem").read().strip()
prof={"version-name":"2.0.0","version-code":2,"uuid":"fe686e1b-3770-4824-a938-961b140a7c98",
 "validity":{"not-before":nb,"not-after":na},"type":"debug",
 "bundle-info":{"developer-id":"OpenHarmony","development-certificate":leaf+"\n","bundle-name":bundle,"apl":"normal","app-feature":"hos_normal_app"},
 "acls":{"allowed-acls":[""]},"permissions":{"restricted-permissions":[""]},
 "debug-info":{"device-ids":[udid],"device-id-type":"udid"},"issuer":"pki_internal"}
json.dump(prof,open(f"{out}/robrix-debug-profile.json","w"),indent=2)
PY

  "$JAVA" -jar "$JAR" sign-profile -mode localSign \
    -keyAlias "openharmony application profile debug" -keyPwd 123456 \
    -profileCertFile "$LIB/OpenHarmonyProfileDebug.pem" \
    -inFile "$SIGN_DIR/robrix-debug-profile.json" -signAlg SHA256withECDSA \
    -keystoreFile "$LIB/OpenHarmony.p12" -keystorePwd 123456 -outFile "$SIGN_DIR/robrix.p7b" >/dev/null

  "$JAVA" -jar "$JAR" sign-app -mode localSign \
    -keyAlias "openharmony application release" -keyPwd 123456 \
    -appCertFile "$SIGN_DIR/app-signing-cert.pem" -profileFile "$SIGN_DIR/robrix.p7b" \
    -inFile "$HAP_UNSIGNED" -signAlg SHA256withECDSA \
    -keystoreFile "$LIB/OpenHarmony.p12" -keystorePwd 123456 \
    -outFile "$HAP_SIGNED" -compatibleVersion 12 -signCode 1 >/dev/null
  echo "signed -> $HAP_SIGNED"
}

cmd_deploy() {
  [[ -f "$HAP_SIGNED" ]] || { echo "error: no signed HAP; run 'sign' first"; exit 1; }
  "$HDC" shell aa force-stop "$BUNDLE" >/dev/null 2>&1 || true
  "$HDC" shell rm -rf "data/local/tmp/$CRATE" >/dev/null 2>&1 || true
  "$HDC" shell mkdir -p "data/local/tmp/$CRATE" >/dev/null
  "$HDC" file send "$HAP_SIGNED" "data/local/tmp/$CRATE/$CRATE.hap" >/dev/null
  "$HDC" shell bm install -p "data/local/tmp/$CRATE/$CRATE.hap"
  "$HDC" shell rm -rf "data/local/tmp/$CRATE" >/dev/null 2>&1 || true
  "$HDC" shell aa start -a EntryAbility -b "$BUNDLE"
}

cmd_run()  { cmd_build; cmd_sign; cmd_deploy; }
cmd_logs() { "$HDC" shell hilog | grep -iE "robrix|makepad|EntryAbility|$BUNDLE"; }
cmd_shot() {
  "$HDC" shell snapshot_display -f "/data/local/tmp/${CRATE}_screen.jpeg" >/dev/null
  "$HDC" file recv "/data/local/tmp/${CRATE}_screen.jpeg" "$SIGN_DIR/robrix_screen.jpeg" >/dev/null
  echo "screenshot -> $SIGN_DIR/robrix_screen.jpeg"
}

case "${1:-run}" in
  deveco) cmd_deveco ;;
  build)  cmd_build ;;
  sign)   cmd_sign ;;
  deploy) cmd_deploy ;;
  run)    cmd_run ;;
  logs)   cmd_logs ;;
  shot)   cmd_shot ;;
  *) echo "usage: $0 {deveco|build|sign|deploy|run|logs|shot}"; exit 2 ;;
esac
