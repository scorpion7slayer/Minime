#!/usr/bin/env bash
set -euo pipefail

dmg_path="${MINIME_DMG_PATH:?MINIME_DMG_PATH is required}"

if [[ ! -f "$dmg_path" ]]; then
  echo "The DMG does not exist: $dmg_path" >&2
  exit 1
fi

signature_details="$(codesign --display --verbose=4 "$dmg_path" 2>&1)"
if ! grep -q "Authority=Developer ID Application:" <<<"$signature_details"; then
  echo "The DMG is not signed with a Developer ID Application certificate" >&2
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

xcrun notarytool submit "$dmg_path" "${notary_arguments[@]}" --wait
xcrun stapler staple "$dmg_path"
xcrun stapler validate "$dmg_path"
codesign --verify --verbose=2 "$dmg_path"
hdiutil verify "$dmg_path" >/dev/null
spctl \
  --assess \
  --type open \
  --context context:primary-signature \
  --verbose=4 \
  "$dmg_path"

echo "Notarized and stapled $dmg_path"
