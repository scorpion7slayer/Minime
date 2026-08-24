#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_bundle="$project_dir/dist/Minime.app"
official_bundle_id="io.github.scorpion7slayer.minime"
expected_bundle_id="${MINIME_BUNDLE_ID:-$official_bundle_id}"
temporary_root="$(mktemp -d)"

cleanup() {
  rm -rf "$temporary_root"
}
trap cleanup EXIT

if [[ ! -d "$app_bundle" ]]; then
  echo "Build the signed release app before notarizing it: $app_bundle" >&2
  exit 1
fi

version="${MINIME_VERSION:-$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_bundle/Contents/Info.plist")}"
actual_bundle_id="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app_bundle/Contents/Info.plist")"

if [[ "$actual_bundle_id" != "$expected_bundle_id" ]]; then
  echo "Expected bundle ID $expected_bundle_id, found $actual_bundle_id" >&2
  exit 1
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "MINIME_VERSION must use MAJOR.MINOR.PATCH, for example 1.2.3" >&2
  exit 1
fi

signature_details="$(codesign --display --verbose=4 "$app_bundle" 2>&1)"
if ! grep -q "Authority=Developer ID Application:" <<<"$signature_details"; then
  echo "Minime.app is not signed with a Developer ID Application certificate" >&2
  exit 1
fi

notary_arguments=()
if [[ -n "${MINIME_NOTARY_PROFILE:-}" ]]; then
  notary_arguments+=(--keychain-profile "$MINIME_NOTARY_PROFILE")
elif [[ -n "${MINIME_NOTARY_APPLE_ID:-}" && -n "${MINIME_NOTARY_TEAM_ID:-}" && -n "${MINIME_NOTARY_PASSWORD:-}" ]]; then
  notary_arguments+=(
    --apple-id "$MINIME_NOTARY_APPLE_ID"
    --team-id "$MINIME_NOTARY_TEAM_ID"
    --password "$MINIME_NOTARY_PASSWORD"
  )
else
  echo "Set MINIME_NOTARY_PROFILE or the three MINIME_NOTARY_* credentials" >&2
  exit 1
fi

submission_archive="$temporary_root/Minime-$version-notarization.zip"
release_archive="$project_dir/dist/Minime-$version-macos.zip"

ditto -c -k --sequesterRsrc --keepParent "$app_bundle" "$submission_archive"
xcrun notarytool submit "$submission_archive" "${notary_arguments[@]}" --wait
xcrun stapler staple "$app_bundle"
xcrun stapler validate "$app_bundle"
codesign --verify --deep --strict --verbose=2 "$app_bundle"
spctl --assess --type execute --verbose=4 "$app_bundle"

rm -f "$release_archive"
ditto -c -k --sequesterRsrc --keepParent "$app_bundle" "$release_archive"

echo "Created notarized release archive: $release_archive"
