# Minime

Minime is a desktop image compressor and format converter written in Rust with [GPUI](https://gpui.rs/). It reduces file size or creates a pixel-exact copy in another format without sending the image anywhere.

Everything runs locally. Minime does not upload images, metadata, or usage statistics.

## Features

- drag and drop or use the native file picker;
- process several images in one batch without blocking the interface;
- use English by default or switch to French;
- follow the system theme or force light or dark mode;
- compare the original image with the generated result;
- inspect the format, dimensions, file size, and saved space;
- use `Auto` to find the smallest exact image representation;
- convert to lossless WebP, PNG, QOI, TIFF, BMP, or Farbfeld;
- choose fast, balanced, or maximum PNG effort without changing quality;
- save beside the originals or in another folder;
- optionally reject a converted file when it is larger than the original;
- preserve the ICC profile when the output format supports it;
- verify every generated image pixel by pixel before writing it;
- avoid overwriting files by generating names such as `photo.minime.webp` and `photo.minime-2.webp`;
- reject animated GIF, WebP, and APNG files instead of flattening them silently;
- run on macOS, Windows, and Linux.

## Supported formats

Inputs: static `PNG` / `APNG`, `JPEG` / `JFIF`, static `WebP`, static `GIF`, `BMP`, `TIFF`, `TGA`, `DDS`, `QOI`, `ICO`, `Farbfeld`, `PNM`, `PPM`, `PGM`, `PAM`, and `PBM`.

| Output | Preserved depth | ICC profile | Intended use |
| --- | --- | --- | --- |
| Auto | 8 or 16-bit | Yes | chooses the smallest lossless PNG or WebP result |
| PNG | 8 or 16-bit | Yes | universal lossless output |
| WebP lossless | 8-bit | Yes | compact modern output |
| QOI | 8-bit | No | very fast encoding and decoding |
| TIFF | 8 or 16-bit | Yes | archives and production workflows |
| BMP | 8-bit | No | compatibility with older tools |
| Farbfeld (`.ff`) | 16-bit RGBA | No | simple open image format |

Animated images are not supported yet. Minime refuses an output that cannot preserve the source depth or color profile. AVIF and JPEG are not offered as outputs because their usual encoding process is lossy.

## What “lossless” means in Minime

Minime uses a strict definition: the dimensions and every decoded 16-bit RGBA value must be identical before and after encoding. If verification fails, Minime does not write the file.

During conversion, Minime applies the EXIF orientation to the pixels so that the visible result stays the same. It copies ICC profiles to PNG, WebP, and TIFF outputs. Other non-visual metadata is not guaranteed when changing formats.

The effort setting changes how thoroughly Minime searches for a smaller PNG representation. It never lowers visual quality.

## Keyboard shortcuts

| Action | macOS | Windows / Linux |
| --- | --- | --- |
| Add images | `⌘O` | `Ctrl+O` |
| Compress or convert | `⌘Return` | `Ctrl+Return` |
| Clear the queue | `⌘⇧K` | `Ctrl+Shift+K` |

## Development

Requirements:

- Rust 1.88 or newer;
- the platform dependencies required by GPUI.

Run Minime locally:

```bash
cargo run
```

Local development builds use the application identifier `dev.minime.app`.

On Ubuntu or Debian, install the usual GPUI development libraries:

```bash
sudo apt-get install libasound2-dev libfontconfig1-dev libwayland-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev
```

GPUI compiles its Metal shaders at runtime on macOS. This keeps the local Rust build from requiring the separate Metal Toolchain component.

## Build

Build a release binary for the current platform:

```bash
cargo build --release
```

The executable is written to `target/release/minime`, or `target/release/minime.exe` on Windows.

### Development macOS bundle

Create a local `.app` bundle:

```bash
./scripts/package-macos.sh
open dist/Minime.app
```

This bundle has these development properties:

- bundle identifier: `dev.minime.app`;
- version: the version in `Cargo.toml`;
- signature: ad hoc (`codesign -`);
- intended use: local development only.

An ad hoc signature is not suitable for a public download. Gatekeeper expects a Developer ID signature and Apple notarization for software distributed outside the Mac App Store.

## Official macOS identity

Official releases use one stable identifier everywhere:

```text
io.github.scorpion7slayer.minime
```

The release build passes this value to GPUI and writes it to `CFBundleIdentifier`. Do not change it after the first public release. If you want another official identifier, change it before registering the App ID and before publishing the first tag in all of these places:

- `.github/workflows/release.yml`;
- `scripts/package-macos.sh`;
- `scripts/notarize-macos.sh`;
- this README.

Release versions are not hard-coded in the packaged plist:

| Source | Final plist value |
| --- | --- |
| tag `v1.2.3` | `CFBundleShortVersionString = 1.2.3` |
| GitHub Actions run number | `CFBundleVersion` |
| release workflow constant | `CFBundleIdentifier = io.github.scorpion7slayer.minime` |

The tag must use exactly `vMAJOR.MINOR.PATCH`, and its version must match the package version in `Cargo.toml`.

## Sign and notarize Minime for macOS

Public macOS releases need both a Developer ID signature and notarization. Signing proves who built the application. Notarization lets Apple scan the signed archive and issue the ticket used by Gatekeeper.

### 1. Join the Apple Developer Program

Use an active Apple Developer Program membership. A free Apple account cannot create the `Developer ID Application` certificate required for distribution outside the Mac App Store.

### 2. Register the official App ID

In [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/identifiers/list):

1. open **Identifiers**;
2. select **App IDs** and then **App**;
3. register `io.github.scorpion7slayer.minime` as an explicit identifier;
4. do not enable capabilities that Minime does not use.

Minime currently needs no special entitlement. Do not add sandbox exceptions, JIT permissions, disabled library validation, or `get-task-allow`. Add an entitlement only if a future feature genuinely requires it.

### 3. Create a Developer ID Application certificate

In the Apple Developer portal:

1. open **Certificates** and press **+**;
2. select **Developer ID**;
3. choose **Developer ID Application**;
4. create and upload the requested certificate signing request;
5. download the `.cer` file and open it to install it in Keychain Access.

`Developer ID Installer` is for signed `.pkg` installers. Minime currently distributes a zipped `.app`, so it needs `Developer ID Application`.

Confirm the installed signing identity:

```bash
security find-identity -v -p codesigning
```

Use the 40-character SHA-1 fingerprint shown in the first column. It is more reliable than the display name in automated shells, especially when the certificate owner’s name contains accented characters. For example:

```text
0123456789ABCDEF0123456789ABCDEF01234567
```

### 4. Export the certificate for GitHub Actions

In Keychain Access:

1. open **My Certificates**;
2. expand the Developer ID Application certificate and make sure its private key is present;
3. export the certificate and private key as a password-protected `.p12` file;
4. keep the `.p12` file and its password private.

Apple certificates, `.p12` files, provisioning profiles, and App Store Connect API keys are ignored by this repository and must never be committed.

### 5. Create notarization credentials

Find your 10-character Team ID in the Apple Developer membership page. Then create an app-specific password at [account.apple.com](https://account.apple.com/) under **Sign-In and Security → App-Specific Passwords**. Two-factor authentication must be enabled on the Apple Account.

For local notarization, store the credentials in the macOS Keychain:

```bash
xcrun notarytool store-credentials "minime-notary" \
  --apple-id "YOUR_APPLE_ID" \
  --team-id "YOUR_TEAM_ID"
```

Enter the app-specific password when prompted. Do not enter the normal Apple Account password.

### 6. Build, sign, and notarize locally

Set the release version and the certificate fingerprint returned by `security find-identity`:

```bash
export MINIME_VERSION="0.1.0"
export MINIME_BUILD_NUMBER="1"
export MINIME_SIGN_IDENTITY="0123456789ABCDEF0123456789ABCDEF01234567"

MINIME_RELEASE=1 ./scripts/package-macos.sh
MINIME_NOTARY_PROFILE="minime-notary" ./scripts/notarize-macos.sh
```

The first command:

- builds Minime with the official application identifier;
- injects the version and build number into `Info.plist`;
- signs with a secure timestamp;
- enables Hardened Runtime with `codesign --options runtime`;
- verifies the code signature.

The second command:

- creates the temporary ZIP submitted to Apple;
- waits for the notarization result with `notarytool`;
- staples the ticket to `Minime.app`;
- validates the ticket, signature, and Gatekeeper assessment;
- creates `dist/Minime-0.1.0-macos.zip` from the stapled application.

You can repeat the final checks manually:

```bash
codesign --verify --deep --strict --verbose=2 dist/Minime.app
xcrun stapler validate dist/Minime.app
spctl --assess --type execute --verbose=4 dist/Minime.app
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' dist/Minime.app/Contents/Info.plist
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' dist/Minime.app/Contents/Info.plist
```

Test the final ZIP on another Mac account or machine after downloading it through a browser. That exercises Gatekeeper and quarantine behavior more realistically than opening the build directly from `dist/`.

## Configure signed GitHub releases

The release workflow imports the Developer ID certificate into a temporary keychain, signs Minime, notarizes it, staples the ticket, builds Windows and Linux archives, and publishes all three archives to [GitHub Releases](https://github.com/scorpion7slayer/Minime/releases).

Add these repository secrets under **Settings → Secrets and variables → Actions**:

| Secret | Value |
| --- | --- |
| `MACOS_CERTIFICATE` | Base64 representation of the exported `.p12` file |
| `MACOS_CERTIFICATE_PASSWORD` | Password chosen when exporting the `.p12` file |
| `MACOS_SIGNING_IDENTITY` | 40-character SHA-1 fingerprint from `security find-identity` |
| `APPLE_ID` | Apple Account email used for notarization |
| `APPLE_TEAM_ID` | 10-character Apple Developer Team ID |
| `APPLE_APP_PASSWORD` | Apple app-specific password, not the normal account password |

Encode and upload the certificate from macOS:

```bash
base64 -i DeveloperIDApplication.p12 | gh secret set MACOS_CERTIFICATE
gh secret set MACOS_CERTIFICATE_PASSWORD
gh secret set MACOS_SIGNING_IDENTITY
gh secret set APPLE_ID
gh secret set APPLE_TEAM_ID
gh secret set APPLE_APP_PASSWORD
```

The last five commands prompt for their values. GitHub encrypts repository secrets and exposes them only to workflow steps that reference them.

## Publish a version with a tag

Before creating a tag:

1. update `version` in `Cargo.toml`;
2. run the checks below;
3. commit and push the release changes to `main`;
4. create an annotated semantic version tag pointing to that commit;
5. push the tag.

Example for version `0.2.0`:

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings

git add Cargo.toml Cargo.lock
git commit -m "Prepare Minime 0.2.0"
git push origin main

git tag -a v0.2.0 -m "Minime 0.2.0"
git push origin v0.2.0
```

Pushing `v0.2.0` starts `.github/workflows/release.yml`. The workflow rejects malformed tags and tags whose version does not match `Cargo.toml`. When all three platform builds succeed, it creates or updates the matching release at [github.com/scorpion7slayer/Minime/releases](https://github.com/scorpion7slayer/Minime/releases).

Do not move or reuse a published version tag. If a release needs a correction, increment the patch version and publish a new tag.

## Project checks

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The tests cover exact QOI, TIFF, BMP, and Farbfeld outputs, PNG size reduction, non-destructive output names, byte formatting, conversion behavior, and preference serialization.

## Project layout

- `src/compression.rs`: decoding, orientation, encoders, PNG optimization, exact verification, and atomic writes;
- `src/main.rs`: GPUI interface, introduction, preview, settings, drag and drop, and asynchronous processing;
- `src/localization.rs` and `src/preferences.rs`: English/French copy and cross-platform preferences;
- `packaging/macos/Info.plist`: development plist template updated by the packaging script;
- `scripts/package-macos.sh`: development or Developer ID-signed macOS bundle;
- `scripts/notarize-macos.sh`: notarization, stapling, Gatekeeper validation, and final ZIP;
- `.github/workflows/ci.yml`: macOS, Windows, and Linux validation;
- `.github/workflows/release.yml`: tag-driven signed release publication.

## Official references

- [Apple: Create Developer ID certificates](https://developer.apple.com/help/account/certificates/create-developer-id-certificates)
- [Apple: Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Apple: Customizing the notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
- [Apple: CFBundleIdentifier](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleidentifier)
- [GitHub: Installing an Apple certificate on macOS runners](https://docs.github.com/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications)
- [GitHub: Secrets in GitHub Actions](https://docs.github.com/actions/concepts/security/secrets)
- [GitHub: Managing releases](https://docs.github.com/repositories/releasing-projects-on-github/managing-releases-in-a-repository)

## License

[MIT](LICENSE)
