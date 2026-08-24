#!/bin/bash
# Builds the Rust observation core, links it into the Swift surface, and lays out
# the .app bundle. No Xcode required — swiftc and cargo do the whole job.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
APP="$ROOT/build/AgentPet.app"
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

echo "==> building observation core"
cargo build --release --manifest-path "$ROOT/core/Cargo.toml"

echo "==> laying out bundle"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>            <string>AgentPet</string>
  <key>CFBundleDisplayName</key>     <string>Agent Pet</string>
  <key>CFBundleIdentifier</key>      <string>gg.deza.agent-pet</string>
  <key>CFBundleExecutable</key>      <string>AgentPet</string>
  <key>CFBundlePackageType</key>     <string>APPL</string>
  <key>CFBundleShortVersionString</key> <string>0.1.0</string>
  <key>LSMinimumSystemVersion</key>  <string>13.0</string>
  <!-- Accessory app: no dock icon, nothing to switch to. -->
  <key>LSUIElement</key>             <true/>
</dict>
</plist>
PLIST

echo "==> compiling surface"
swiftc -O \
  -target arm64-apple-macos13.0 \
  -import-objc-header "$ROOT/macos/bridge/agentpet.h" \
  "$ROOT"/macos/Sources/AgentPet/*.swift \
  -L "$ROOT/core/target/release" -lagentpet_core \
  -o "$APP/Contents/MacOS/AgentPet"

echo "==> built $APP"
