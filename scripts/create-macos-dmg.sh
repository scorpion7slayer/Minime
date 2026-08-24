#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_bundle="${MINIME_APP_BUNDLE:-$project_dir/dist/Minime.app}"
version="${MINIME_VERSION:-$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_bundle/Contents/Info.plist")}"
release_arch="${MINIME_RELEASE_ARCH:-apple-silicon}"
volume_name="Minime $version"
output_path="${MINIME_DMG_OUTPUT:-$project_dir/dist/Minime-$version-macos-$release_arch.dmg}"
signing_identity="${MINIME_SIGN_IDENTITY:--}"
temporary_root="$(mktemp -d)"
read_write_dmg="$temporary_root/Minime-rw.dmg"
mount_dir="/Volumes/$volume_name"
mounted=0

cleanup() {
  if [[ "$mounted" == "1" ]]; then
    hdiutil detach "$mount_dir" -quiet || true
  fi
  rm -rf "$temporary_root"
}
trap cleanup EXIT

if [[ ! -d "$app_bundle" ]]; then
  echo "Build Minime.app before creating the DMG: $app_bundle" >&2
  exit 1
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "MINIME_VERSION must use MAJOR.MINOR.PATCH, for example 1.2.3" >&2
  exit 1
fi

case "$release_arch" in
  apple-silicon)
    expected_binary_arch="arm64"
    ;;
  intel)
    expected_binary_arch="x86_64"
    ;;
  *)
    echo "MINIME_RELEASE_ARCH must be apple-silicon or intel" >&2
    exit 1
    ;;
esac

binary_arches="$(lipo -archs "$app_bundle/Contents/MacOS/minime")"
if [[ "$binary_arches" != "$expected_binary_arch" ]]; then
  echo "Expected a $expected_binary_arch app, found: $binary_arches" >&2
  exit 1
fi

codesign --verify --deep --strict --verbose=2 "$app_bundle"

if [[ -e "$mount_dir" ]]; then
  echo "A volume named $volume_name is already mounted. Eject it and try again." >&2
  exit 1
fi

mkdir -p "$(dirname "$output_path")"
hdiutil create \
  -size 100m \
  -fs HFS+ \
  -volname "$volume_name" \
  -type UDIF \
  -ov \
  "$read_write_dmg" >/dev/null

hdiutil attach \
  "$read_write_dmg" \
  -mountpoint "$mount_dir" \
  -readwrite \
  -noverify \
  -noautoopen >/dev/null
mounted=1

ditto "$app_bundle" "$mount_dir/Minime.app"
ln -s /Applications "$mount_dir/Applications"
cp "$app_bundle/Contents/Resources/MinimeAppIcon.icns" "$mount_dir/.VolumeIcon.icns"
SetFile -a V "$mount_dir/.VolumeIcon.icns"
SetFile -a C "$mount_dir"

osascript <<APPLESCRIPT
tell application "Finder"
  tell disk "$volume_name"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set pathbar visible of container window to false
    set bounds of container window to {120, 120, 780, 572}
    set theViewOptions to the icon view options of container window
    set arrangement of theViewOptions to not arranged
    set icon size of theViewOptions to 128
    set text size of theViewOptions to 14
    set background picture of theViewOptions to file "Minime.app:Contents:Resources:MinimeDmgBackground.png"
    set position of item "Minime.app" of container window to {170, 210}
    set position of item "Applications" of container window to {490, 210}
    update without registering applications
    delay 2
    close
  end tell
end tell
APPLESCRIPT

rm -rf \
  "$mount_dir/.fseventsd" \
  "$mount_dir/.Spotlight-V100" \
  "$mount_dir/.Trashes"
sync
hdiutil detach "$mount_dir" -quiet
mounted=0

rm -f "$output_path"
hdiutil convert \
  "$read_write_dmg" \
  -format UDZO \
  -imagekey zlib-level=9 \
  -ov \
  -o "$output_path" >/dev/null

if [[ "$signing_identity" != "-" ]]; then
  codesign --force --timestamp --sign "$signing_identity" "$output_path"
  codesign --verify --verbose=2 "$output_path"
fi

hdiutil verify "$output_path" >/dev/null

echo "Created $output_path"
echo "Architecture: $expected_binary_arch"
if [[ "$signing_identity" == "-" ]]; then
  echo "DMG signature: unsigned development image"
else
  echo "DMG signature: $signing_identity"
fi
