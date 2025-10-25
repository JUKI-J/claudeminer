#!/bin/bash
# Build ClaudeMiner and create DMG with drag-and-drop UI

set -e

echo "🔨 Building ClaudeMiner..."
cd "$(dirname "$0")/.."
npm run tauri build 2>&1 | grep -v "error running bundle_dmg.sh" || true

BUNDLE_DIR="src-tauri/target/release/bundle"
APP_PATH="$BUNDLE_DIR/macos/ClaudeMiner.app"
DMG_OUTPUT="$BUNDLE_DIR/ClaudeMiner_1.0.0_aarch64.dmg"

if [ ! -d "$APP_PATH" ]; then
    echo "❌ App bundle not found: $APP_PATH"
    exit 1
fi

echo ""
echo "📦 Creating DMG with drag-and-drop UI..."
cd "$BUNDLE_DIR"
rm -f ClaudeMiner_1.0.0_aarch64.dmg

create-dmg \
  --volname "ClaudeMiner" \
  --volicon "dmg/icon.icns" \
  --window-pos 200 120 \
  --window-size 660 400 \
  --icon-size 100 \
  --icon "ClaudeMiner.app" 180 170 \
  --hide-extension "ClaudeMiner.app" \
  --app-drop-link 480 170 \
  "ClaudeMiner_1.0.0_aarch64.dmg" \
  "macos/ClaudeMiner.app" 2>&1 | grep -E "(created:|Done|Disk image done)" || true

if [ -f "$DMG_OUTPUT" ]; then
    DMG_SIZE=$(ls -lh "$DMG_OUTPUT" | awk '{print $5}')
    echo ""
    echo "✅ DMG created successfully!"
    echo "📍 Location: $DMG_OUTPUT"
    echo "📦 Size: $DMG_SIZE"
    echo ""
    echo "🚀 To open DMG:"
    echo "   open $DMG_OUTPUT"
else
    echo "❌ Failed to create DMG"
    exit 1
fi
