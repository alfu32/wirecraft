#!/usr/bin/env bash
set -euo pipefail
if [[ $# != 2 ]]; then echo 'usage: package.sh VERSION TARGET' >&2; exit 2; fi
release_version=$1
release_target=$2
release_root=$(cd "$(dirname "$0")/../.." && pwd)
release_assets="$release_root/target/release-assets"
release_name="wirecraft-$release_version-$release_target"
release_stage="$release_root/target/macos-package-$release_target"
# All stage paths are fixed beneath target; never remove a user-supplied path.
rm -rf "$release_stage"
mkdir -p "$release_stage/input" "$release_stage/disk"
ditto -x -k "$release_assets/$release_name.zip" "$release_stage/input"
release_app="$release_stage/disk/Wirecraft.app"
mkdir -p "$release_app/Contents/MacOS" "$release_app/Contents/Resources"
cp -R "$release_stage/input/$release_name/." "$release_app/Contents/Resources/"
# Both executables sit together: `wirecraft desktop` starts its sibling application.
for release_command in wirecraft-desktop wirecraft; do
  mv "$release_app/Contents/Resources/$release_command" "$release_app/Contents/MacOS/$release_command"
  chmod +x "$release_app/Contents/MacOS/$release_command"
done
cat > "$release_app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>io.github.alfu32.wirecraft</string>
<key>CFBundleName</key><string>Wirecraft</string>
<key>CFBundleDisplayName</key><string>Wirecraft</string>
<key>CFBundleExecutable</key><string>wirecraft-desktop</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$release_version</string>
<key>CFBundleVersion</key><string>$release_version</string>
<key>CFBundleIconFile</key><string>wirecraft.icns</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>Wirecraft project</string>
<key>CFBundleTypeRole</key><string>Editor</string>
<key>CFBundleTypeExtensions</key><array><string>orch</string></array>
</dict></array>
</dict></plist>
PLIST
mkdir -p "$release_stage/wirecraft.iconset"
for release_size in 16 32 128 256 512; do
  sips -z "$release_size" "$release_size" "$release_root/packaging/wirecraft.png" --out "$release_stage/wirecraft.iconset/icon_${release_size}x${release_size}.png" >/dev/null
  release_double=$((release_size * 2))
  sips -z "$release_double" "$release_double" "$release_root/packaging/wirecraft.png" --out "$release_stage/wirecraft.iconset/icon_${release_size}x${release_size}@2x.png" >/dev/null
done
iconutil --convert icns "$release_stage/wirecraft.iconset" --output "$release_app/Contents/Resources/wirecraft.icns"
# Ad-hoc signatures make the ARM executables structurally valid; no identity/notarization secrets.
codesign --force --deep --sign - "$release_app"
codesign --verify --deep --strict "$release_app"
test "$("$release_app/Contents/MacOS/wirecraft-desktop" --version)" = "Wirecraft $release_version"
test "$("$release_app/Contents/MacOS/wirecraft" --version)" = "Wirecraft $release_version"
ln -s /Applications "$release_stage/disk/Applications"
release_dmg="$release_assets/wirecraft-$release_version-macos-${release_target%%-*}.dmg"
hdiutil create -volname "Wirecraft $release_version" -srcfolder "$release_stage/disk" -format UDZO -ov "$release_dmg"
hdiutil verify "$release_dmg"
(cd "$release_assets" && shasum -a 256 "$(basename "$release_dmg")" > "$(basename "$release_dmg").sha256")
