# Minime

Minime is a small desktop image compressor and format converter written in Rust with [GPUI](https://gpui.rs/). It can reduce an image's file size or convert it to another lossless format, then verifies the result pixel by pixel before saving it.

Everything happens on the device. Minime does not upload images, file names, metadata, or usage statistics. It only connects to GitHub Releases when the user checks for an update or opts into automatic update checks.

## Downloads

Each release is packaged separately for its native processor architecture.

| Platform | Architecture | Package |
| --- | --- | --- |
| macOS | Apple Silicon (`arm64`) | `Minime-VERSION-macos-apple-silicon.dmg` |
| macOS | Intel (`x86_64`) | `Minime-VERSION-macos-intel.dmg` |
| Windows | x64 | `Minime-VERSION-windows-x64-setup.exe` |
| Windows | ARM64 | `Minime-VERSION-windows-arm64-setup.exe` |
| Linux | x64 | `.flatpak` bundle or `.tar.gz` archive |
| Linux | ARM64 | `.flatpak` bundle or `.tar.gz` archive |

On macOS, open the correct DMG and drag **Minime** to **Applications**. On Windows, use the Setup executable; the installed release binary is a graphical application and does not open a Command Prompt window. On Linux, Flatpak is the easiest desktop installation, while the archive is useful for a portable native installation.

Releases and in-app update checks require the GitHub repository to be public. A private repository deliberately returns no unauthenticated release feed, and Minime never embeds a private GitHub token in desktop builds.

## Features

- drag and drop or use the native file picker;
- process several images in one batch without blocking the interface;
- use English by default or switch to French;
- follow the system theme or force light or dark mode;
- compare the original image with the generated result;
- inspect format, dimensions, file size, and saved space;
- use `Auto` to find the smallest exact image representation;
- convert to lossless WebP, PNG, QOI, TIFF, BMP, or Farbfeld;
- choose fast, balanced, or maximum PNG effort without changing quality;
- save beside the originals or in another folder;
- optionally reject a converted file when it is larger than the original;
- preserve the ICC profile when the output format supports it;
- verify every generated image pixel by pixel before writing it;
- avoid overwriting files by generating names such as `photo.minime.webp` and `photo.minime-2.webp`;
- reject animated GIF, WebP, and APNG files instead of flattening them silently;
- check for updates manually or, with consent, once a day.

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

## What “lossless” means

Minime uses a strict definition: the dimensions and every decoded 16-bit RGBA value must be identical before and after encoding. If verification fails, Minime does not write the file.

During conversion, Minime applies the EXIF orientation to the pixels so the visible result stays the same. It copies ICC profiles to PNG, WebP, and TIFF outputs. Other non-visual metadata is not guaranteed when changing formats.

The effort setting changes how thoroughly Minime searches for a smaller PNG representation. It never lowers visual quality.

## Keyboard shortcuts

| Action | macOS | Windows / Linux |
| --- | --- | --- |
| Add images | `⌘O` | `Ctrl+O` |
| Compress or convert | `⌘Return` | `Ctrl+Return` |
| Clear the queue | `⌘⇧K` | `Ctrl+Shift+K` |

## Updates

After the introduction, Minime asks whether it may check for updates automatically. Declining leaves update checks fully manual. When enabled, Minime checks at startup at most once every 24 hours. Installing an update always requires a separate click.

| Installation | Update behavior |
| --- | --- |
| macOS DMG | Downloads the correct Apple Silicon or Intel DMG, verifies its SHA-256 digest, Developer ID signature, Team ID, bundle ID, architecture, and Gatekeeper result, then replaces the complete app bundle. |
| Windows Setup | Downloads the matching x64 or ARM64 Setup executable, verifies its SHA-256 digest, and runs it silently in the existing per-user install folder. |
| Linux archive | Downloads the matching x64 or ARM64 archive, verifies its SHA-256 digest, and replaces the native binary. |
| Flatpak | Checks can report a new version, but Flatpak owns the read-only app files, so installation must happen through the Flatpak source. |

The GitHub release `.flatpak` files are standalone bundles. They do not, by themselves, create a permanent update remote. Until Minime is published on Flathub or another Flatpak repository, a user of those bundles must install the newer bundle manually. The native Linux archive can use Minime's in-app updater immediately.

The release contains two installer assets with fixed names required by the update client: `minime-installer.sh` for macOS/Linux and `minime-installer.ps1` for Windows. They select the correct architecture at runtime. `SHA256SUMS` covers every public package and both update installers.

## Development

Requirements:

- Rust 1.88 or newer;
- the platform dependencies required by GPUI.

Run Minime locally:

```bash
cargo run
```

Local development builds use the application identifier `dev.minime.app`. Official release builds use `io.github.scorpion7slayer.minime`.

On Ubuntu or Debian, install the usual GPUI development libraries:

```bash
sudo apt-get install libasound2-dev libfontconfig1-dev libwayland-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev
```

GPUI compiles its Metal shaders at runtime on macOS. This keeps the local Rust build from requiring the separate Metal Toolchain component.

Build a release binary for the current platform:

```bash
cargo build --release
```

The executable is written to `target/release/minime`, or `target/release/minime.exe` on Windows.

### Development macOS bundle and DMG

Create an ad hoc-signed app and a local DMG for the current Mac architecture:

```bash
./scripts/package-macos.sh

# Use apple-silicon on an arm64 Mac, or intel on an x86_64 Mac.
MINIME_RELEASE_ARCH=apple-silicon \
MINIME_SIGN_IDENTITY=- \
./scripts/create-macos-dmg.sh
```

The development app uses `dev.minime.app` and an ad hoc signature. It is only for local testing and is not suitable for a public download.

## Official application identity

Official releases use one stable identifier everywhere:

```text
io.github.scorpion7slayer.minime
```

The release build passes it to GPUI and writes it to `CFBundleIdentifier`. Do not change it after public distribution has begun.

| Build | Application identifier | Version source |
| --- | --- | --- |
| Local development | `dev.minime.app` | `Cargo.toml` |
| Official release | `io.github.scorpion7slayer.minime` | tag `vMAJOR.MINOR.PATCH` |

For an official release, the tag version, `Cargo.toml`, `Cargo.lock`, and the newest Flatpak metainfo release must match. `CFBundleVersion` uses the GitHub Actions run number so every macOS build has an increasing build identifier.

## Sign and notarize macOS releases

Public DMGs require a Developer ID Application certificate and Apple notarization. The workflow signs and notarizes both `Minime.app` and the final DMG for Apple Silicon and Intel.

### 1. Join the Apple Developer Program

Use an active paid Apple Developer Program membership. A free Apple account cannot create the Developer ID Application certificate used to distribute software outside the Mac App Store.

### 2. Register the official identifier

In [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/identifiers/list):

1. open **Identifiers**;
2. select **App IDs**, then **App**;
3. register `io.github.scorpion7slayer.minime` as an explicit identifier;
4. leave unused capabilities disabled.

Minime currently needs no special entitlement. Do not add sandbox exceptions, JIT permissions, disabled library validation, or `get-task-allow` unless a future feature genuinely requires one.

### 3. Create a Developer ID Application certificate

In the Apple Developer portal:

1. open **Certificates** and press **+**;
2. select **Developer ID**;
3. choose **Developer ID Application**;
4. create and upload the requested certificate signing request;
5. download the `.cer` and open it to install it in Keychain Access.

`Developer ID Installer` is for `.pkg` installers. A DMG containing a signed `.app` uses the Developer ID Application identity.

Confirm the identity:

```bash
security find-identity -v -p codesigning
```

Use the 40-character SHA-1 fingerprint from the first column as `MACOS_SIGNING_IDENTITY`. It is more reliable in automation than a display name containing spaces or accented characters.

### 4. Export the certificate

In Keychain Access:

1. open **My Certificates**;
2. expand the Developer ID Application certificate and confirm its private key is present;
3. export the certificate and private key as a password-protected `.p12`;
4. keep the `.p12` and its password private.

Certificates, private keys, provisioning profiles, and notarization credentials must never be committed.

### 5. Create notarization credentials

Find the 10-character Team ID on the Apple Developer membership page. Then create an app-specific password at [account.apple.com](https://account.apple.com/) under **Sign-In and Security → App-Specific Passwords**. Do not use the normal Apple Account password.

For local builds, store the credentials in the macOS Keychain:

```bash
xcrun notarytool store-credentials "minime-notary" \
  --apple-id "YOUR_APPLE_ID" \
  --team-id "YOUR_TEAM_ID"
```

Enter the app-specific password when prompted.

### 6. Build one signed DMG locally

Run these commands on the architecture being packaged. An Apple Silicon Mac produces the Apple Silicon DMG; an Intel Mac produces the Intel DMG. The GitHub workflow uses two native runners so it can create both.

```bash
export MINIME_VERSION="0.2.0"
export MINIME_BUILD_NUMBER="1"
export MINIME_SIGN_IDENTITY="YOUR_40_CHARACTER_CERTIFICATE_FINGERPRINT"
export MINIME_RELEASE_ARCH="apple-silicon" # use intel on an Intel Mac

MINIME_RELEASE=1 ./scripts/package-macos.sh
MINIME_NOTARY_PROFILE="minime-notary" ./scripts/notarize-macos.sh
./scripts/create-macos-dmg.sh

MINIME_DMG_PATH="dist/Minime-${MINIME_VERSION}-macos-${MINIME_RELEASE_ARCH}.dmg" \
MINIME_NOTARY_PROFILE="minime-notary" \
./scripts/notarize-macos-dmg.sh
```

The sequence is intentional:

1. build and sign `Minime.app` with Hardened Runtime and a secure timestamp;
2. notarize and staple the app;
3. create and sign the styled DMG;
4. notarize and staple the DMG;
5. validate signatures, notarization tickets, Gatekeeper, architecture, and disk image integrity.

Repeat the final checks manually when needed:

```bash
codesign --verify --deep --strict --verbose=2 dist/Minime.app
xcrun stapler validate dist/Minime.app
spctl --assess --type execute --verbose=4 dist/Minime.app

DMG="dist/Minime-0.2.0-macos-apple-silicon.dmg"
codesign --verify --verbose=2 "$DMG"
xcrun stapler validate "$DMG"
hdiutil verify "$DMG"
spctl --assess --type open --context context:primary-signature --verbose=4 "$DMG"
```

Download the final DMG through a browser and test it from another Mac account or machine. That exercises quarantine, Gatekeeper, the drag-to-Applications layout, first launch, and the dock icon more realistically than opening the local build output.

### 7. Configure GitHub Actions secrets

Add these secrets under **Settings → Secrets and variables → Actions**:

| Secret | Value |
| --- | --- |
| `MACOS_CERTIFICATE` | Base64 representation of the exported `.p12` |
| `MACOS_CERTIFICATE_PASSWORD` | Password chosen while exporting the `.p12` |
| `MACOS_SIGNING_IDENTITY` | 40-character SHA-1 fingerprint from `security find-identity` |
| `APPLE_ID` | Apple Account email used for notarization |
| `APPLE_TEAM_ID` | 10-character Apple Developer Team ID |
| `APPLE_APP_PASSWORD` | Apple app-specific password |

From macOS with the GitHub CLI authenticated:

```bash
base64 -i DeveloperIDApplication.p12 | gh secret set MACOS_CERTIFICATE
gh secret set MACOS_CERTIFICATE_PASSWORD
gh secret set MACOS_SIGNING_IDENTITY
gh secret set APPLE_ID
gh secret set APPLE_TEAM_ID
gh secret set APPLE_APP_PASSWORD
```

The last five commands prompt securely for their values.

## Release workflow

`.github/workflows/release.yml` uses native GitHub-hosted runners for:

- macOS Apple Silicon and Intel;
- Windows x64 and ARM64;
- Linux x64 and ARM64;
- Flatpak x86_64 and aarch64.

The ARM GitHub-hosted runners used by this workflow require the repository to be public. Make the repository public before pushing the release tag.

`workflow_dispatch` can build every package without publishing a release. A pushed semantic version tag runs the same builds and publishes the result to [GitHub Releases](https://github.com/scorpion7slayer/Minime/releases).

Before creating a tag:

1. update the version in `Cargo.toml` and Flatpak metainfo;
2. refresh `Cargo.lock`;
3. run all project checks;
4. commit and push the release changes to `main`;
5. create an annotated tag pointing to that exact commit;
6. push the tag.

Example for version `0.2.0`:

```bash
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings

git add Cargo.toml Cargo.lock packaging scripts .github README.md src
git commit -m "Prepare Minime 0.2.0"
git push origin main

git tag -a v0.2.0 -m "Minime 0.2.0"
git push origin v0.2.0
```

Do not move or reuse a published version tag. If a release needs a correction, increment the patch version and publish a new tag.

## Project checks

```bash
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
```

The tests cover exact QOI, TIFF, BMP, and Farbfeld outputs, PNG size reduction, non-destructive output names, byte formatting, conversion behavior, preference serialization, and update timing.

## Project layout

- `src/compression.rs`: decoding, orientation, encoders, PNG optimization, exact verification, and atomic writes;
- `src/main.rs`: GPUI interface, introduction, update consent, preview, settings, drag and drop, and asynchronous processing;
- `src/updater.rs`: release checks, platform-aware installation, restart, and rollback cleanup;
- `src/localization.rs` and `src/preferences.rs`: English/French copy and cross-platform preferences;
- `packaging/macos`: app plist and styled DMG background;
- `packaging/windows`: Inno Setup installer definition;
- `packaging/linux`: Flatpak manifest, desktop entry, and AppStream metadata;
- `packaging/update`: checksum-verifying updater templates;
- `scripts/package-macos.sh`: development or Developer ID-signed app bundle;
- `scripts/notarize-macos.sh`: app notarization and stapling;
- `scripts/create-macos-dmg.sh`: styled, architecture-specific DMG creation;
- `scripts/notarize-macos-dmg.sh`: final DMG notarization and validation;
- `.github/workflows/ci.yml`: regular macOS, Windows, and Linux validation;
- `.github/workflows/release.yml`: multi-architecture package and tag-driven release publication.

## Official references

- [Apple: Create Developer ID certificates](https://developer.apple.com/help/account/certificates/create-developer-id-certificates)
- [Apple: Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [Apple: Customizing the notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
- [Apple: CFBundleIdentifier](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleidentifier)
- [GitHub: GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub: Installing an Apple certificate on macOS runners](https://docs.github.com/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications)
- [GitHub: Secrets in GitHub Actions](https://docs.github.com/actions/concepts/security/secrets)
- [GitHub: Managing releases](https://docs.github.com/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
- [Flatpak: Building your first application](https://docs.flatpak.org/en/latest/first-build.html)
- [Flatpak: GitHub Actions](https://github.com/flatpak/flatpak-github-actions)
- [Inno Setup: Architecture identifiers](https://jrsoftware.org/ishelp/topic_archidentifiers.htm)

## License

[MIT](LICENSE)
