#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_bundle="$project_dir/dist/Minime.app"
development_bundle_id="dev.minime.app"
official_bundle_id="io.github.scorpion7slayer.minime"
release_mode="${MINIME_RELEASE:-0}"

cargo_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_dir/Cargo.toml" | head -n 1)"
if [[ -z "$cargo_version" ]]; then
  echo "Unable to read the package version from Cargo.toml" >&2
  exit 1
fi

if [[ "$release_mode" == "1" ]]; then
  version="${MINIME_VERSION:?MINIME_VERSION is required for an official release}"
  bundle_id="${MINIME_BUNDLE_ID:-$official_bundle_id}"
  signing_identity="${MINIME_SIGN_IDENTITY:?MINIME_SIGN_IDENTITY is required for an official release}"
else
  version="${MINIME_VERSION:-$cargo_version}"
  bundle_id="${MINIME_BUNDLE_ID:-$development_bundle_id}"
  signing_identity="${MINIME_SIGN_IDENTITY:--}"
fi

build_number="${MINIME_BUILD_NUMBER:-1}"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "MINIME_VERSION must use MAJOR.MINOR.PATCH, for example 1.2.3" >&2
  exit 1
fi

if [[ ! "$build_number" =~ ^[0-9]+([.][0-9]+){0,2}$ ]]; then
  echo "MINIME_BUILD_NUMBER must contain one to three dot-separated integers" >&2
  exit 1
fi

if [[ ! "$bundle_id" =~ ^[A-Za-z0-9-]+([.][A-Za-z0-9-]+)+$ ]]; then
  echo "The bundle identifier is invalid: $bundle_id" >&2
  exit 1
fi

temporary_root="$(mktemp -d)"
iconset_dir="$temporary_root/Minime.iconset"
staged_bundle="$temporary_root/Minime.app"

cleanup() {
  rm -rf "$temporary_root"
}
trap cleanup EXIT

cd "$project_dir"
if [[ -n "${MINIME_BINARY_PATH:-}" ]]; then
  binary_path="$MINIME_BINARY_PATH"
  if [[ ! -x "$binary_path" ]]; then
    echo "MINIME_BINARY_PATH does not point to an executable: $binary_path" >&2
    exit 1
  fi
else
  MINIME_APP_ID="$bundle_id" cargo build --release
  binary_path="$project_dir/target/release/minime"
fi

mkdir -p "$staged_bundle/Contents/MacOS" "$staged_bundle/Contents/Resources"
cp "$binary_path" "$staged_bundle/Contents/MacOS/minime"
cp "$project_dir/packaging/macos/Info.plist" "$staged_bundle/Contents/Info.plist"
cp "$project_dir/assets/minime.svg" "$staged_bundle/Contents/Resources/minime.svg"
sips \
  -s format png \
  -z 840 1320 \
  -s dpiWidth 144 \
  -s dpiHeight 144 \
  "$project_dir/packaging/macos/dmg-background.svg" \
  --out "$staged_bundle/Contents/Resources/MinimeDmgBackground.png" >/dev/null
chmod +x "$staged_bundle/Contents/MacOS/minime"

/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $bundle_id" "$staged_bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$staged_bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build_number" "$staged_bundle/Contents/Info.plist"

mkdir -p "$iconset_dir"
for icon_size in 16 32 128 256 512; do
  double_size=$((icon_size * 2))
  sips -s format png -z "$icon_size" "$icon_size" \
    "$project_dir/assets/minime.svg" \
    --out "$iconset_dir/icon_${icon_size}x${icon_size}.png" >/dev/null
  sips -s format png -z "$double_size" "$double_size" \
    "$project_dir/assets/minime.svg" \
    --out "$iconset_dir/icon_${icon_size}x${icon_size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset_dir" -o "$staged_bundle/Contents/Resources/MinimeAppIcon.icns"

if [[ "$signing_identity" == "-" ]]; then
  codesign --force --sign - "$staged_bundle"
else
  codesign \
    --force \
    --options runtime \
    --timestamp \
    --sign "$signing_identity" \
    "$staged_bundle"
fi

codesign --verify --deep --strict --verbose=2 "$staged_bundle"

mkdir -p "$project_dir/dist"
rm -rf "$app_bundle"
mv "$staged_bundle" "$app_bundle"
touch "$app_bundle"

echo "Created $app_bundle"
echo "Bundle ID: $bundle_id"
echo "Version: $version ($build_number)"
if [[ "$signing_identity" == "-" ]]; then
  echo "Signature: ad hoc development signature"
else
  echo "Signature: $signing_identity"
fi
