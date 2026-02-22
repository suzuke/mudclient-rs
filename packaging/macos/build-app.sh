#!/bin/bash
set -e

# ── Configuration ──
APP="MudClient.app"
DMG="MudClient.dmg"
TARGETS=("aarch64-apple-darwin" "x86_64-apple-darwin")

echo "🔨 Building release for ${TARGETS[*]}..."

# ── Build both architectures ──
for target in "${TARGETS[@]}"; do
    echo "  → Installing target $target (if needed)..."
    rustup target add "$target" || true
    echo "  → Building $target..."
    cargo build -p mudgui --release --target "$target"
done

# ── Create .app bundle ──
rm -rf "$APP" "$DMG"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

# ── Create Universal Binary with lipo ──
echo "🔗 Creating Universal Binary..."
lipo -create \
    target/aarch64-apple-darwin/release/mudgui \
    target/x86_64-apple-darwin/release/mudgui \
    -output "$APP/Contents/MacOS/mudgui"
chmod +x "$APP/Contents/MacOS/mudgui"

# ── Copy Info.plist ──
cp packaging/macos/Info.plist "$APP/Contents/"

# ── Copy resources ──
cp -r scripts "$APP/Contents/Resources/scripts"
cp -r data "$APP/Contents/Resources/data"
cp -r docs "$APP/Contents/Resources/docs"

# ── Copy icon if exists ──
if [ -f packaging/macos/AppIcon.icns ]; then
    cp packaging/macos/AppIcon.icns "$APP/Contents/Resources/"
fi

echo "✅ Built: $APP"

# ── Create DMG ──
echo "📦 Creating DMG..."
DMG_TEMP="dmg_staging"
rm -rf "$DMG_TEMP"
mkdir -p "$DMG_TEMP"
cp -r "$APP" "$DMG_TEMP/"
ln -s /Applications "$DMG_TEMP/Applications"

hdiutil create -volname "MudClient" \
    -srcfolder "$DMG_TEMP" \
    -ov -format UDZO \
    "$DMG"

rm -rf "$DMG_TEMP"

echo "✅ Built: $DMG"
echo "   Double-click DMG to install, or run: open $APP"
