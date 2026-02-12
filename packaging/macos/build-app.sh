#!/bin/bash
set -e

# Build release
echo "🔨 Building release..."
cargo build -p mudgui --release

# Create .app bundle
APP="MudClient.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

# Copy executable
cp target/release/mudgui "$APP/Contents/MacOS/"
chmod +x "$APP/Contents/MacOS/mudgui"

# Copy Info.plist
cp packaging/macos/Info.plist "$APP/Contents/"

# Copy resources
cp -r scripts "$APP/Contents/Resources/scripts"
cp -r docs "$APP/Contents/Resources/docs"

# Copy icon if exists
if [ -f packaging/macos/AppIcon.icns ]; then
    cp packaging/macos/AppIcon.icns "$APP/Contents/Resources/"
fi

echo "✅ Built: $APP"
echo "   Double-click to launch, or run: open $APP"
