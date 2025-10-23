#!/bin/bash

# ClaudeMiner Notarization Script
# This script notarizes the DMG file and staples the notarization ticket

set -e

echo "🔐 Starting Notarization Process..."

# Check if environment variables are set
if [ -z "$APPLE_ID" ] || [ -z "$APPLE_PASSWORD" ] || [ -z "$APPLE_TEAM_ID" ]; then
    echo "❌ Error: Environment variables not set"
    echo ""
    echo "Please set the following environment variables:"
    echo "  export APPLE_ID=\"your-apple-id@email.com\""
    echo "  export APPLE_PASSWORD=\"your-app-specific-password\""
    echo "  export APPLE_TEAM_ID=\"JJX75F53MA\""
    echo ""
    echo "To generate an app-specific password:"
    echo "  1. Go to https://appleid.apple.com"
    echo "  2. Sign in > Security > App-Specific Passwords"
    echo "  3. Generate a password for 'ClaudeMiner'"
    exit 1
fi

DMG_PATH="src-tauri/target/release/bundle/dmg/ClaudeMiner_1.0.0_aarch64.dmg"

if [ ! -f "$DMG_PATH" ]; then
    echo "❌ Error: DMG file not found at $DMG_PATH"
    echo "Please run 'npm run tauri build' first"
    exit 1
fi

echo "📦 DMG File: $DMG_PATH"
echo "📧 Apple ID: $APPLE_ID"
echo "🏢 Team ID: $APPLE_TEAM_ID"
echo ""

# Submit for notarization
echo "📤 Submitting DMG for notarization..."
NOTARY_OUTPUT=$(xcrun notarytool submit "$DMG_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_PASSWORD" \
    --wait 2>&1)

echo "$NOTARY_OUTPUT"

# Check if notarization was successful
if echo "$NOTARY_OUTPUT" | grep -q "status: Accepted"; then
    echo "✅ Notarization successful!"

    # Staple the notarization ticket
    echo "📎 Stapling notarization ticket to DMG..."
    xcrun stapler staple "$DMG_PATH"

    echo ""
    echo "🎉 SUCCESS! Your DMG is now notarized and ready for distribution!"
    echo "📦 File: $DMG_PATH"
    echo ""
    echo "Next steps:"
    echo "  1. Test the DMG on a clean Mac to verify no warnings"
    echo "  2. Upload to GitHub Releases"
    echo "  3. Share the download link!"
else
    echo "❌ Notarization failed. Please check the output above."
    exit 1
fi
