<!-- [INPUT]: Verified dual-app build pipeline and external Apple signing credentials. -->
<!-- [OUTPUT]: Maintainer instructions for a notarized Apple Silicon distribution. -->
<!-- [POS]: Public release contract referenced by the README; excludes private keys and local account paths. -->
<!-- [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md -->
# Releasing CCBaga for macOS

Public downloads contain **CCBaga-A.app** and **CCBaga-B.app** in one DMG. The current target is Apple Silicon, macOS 12 or later. Intel builds are not provided.

## Prerequisites

Follow the [development setup](../README.md#development), including the upstream Fingerprint Chromium engine. Install a valid **Developer ID Application** certificate in your keychain and provide an Apple API key with notarization access. Keep the `.p8` and encrypted updater signing key outside the repository. Preserve the updater key across repository renames; existing clients trust its public key.

Set these environment variables in your shell or CI secret store:

| Variable | Value |
| --- | --- |
| `CC_SIGN_IDENTITY` | Developer ID Application certificate name or SHA-1 fingerprint |
| `APPLE_API_KEY` | Apple API key ID |
| `APPLE_API_ISSUER` | Apple API issuer UUID |
| `APPLE_API_KEY_PATH` | Absolute path to the external `.p8` file |
| `TAURI_SIGNING_PRIVATE_KEY_PATH` | Absolute path to the encrypted Tauri updater key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for that updater key |

The development configuration uses ad hoc signing (`-`). It is not a public distribution signature.

## GitHub Actions

The [Release workflow](../.github/workflows/release.yml) runs on every push to **`release`**, including documentation changes. It can also be dispatched manually against that branch after the workflow is present on the default branch. There is no pull-request signing path.

Configure these **repository Secrets** once:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` containing the selected Developer ID Application certificate and its private key |
| `APPLE_CERTIFICATE_PASSWORD` | Password protecting that `.p12` |
| `APPLE_API_PRIVATE_KEY` | Complete contents of the Apple API `.p8` key |
| `APPLE_API_KEY` | Apple API key ID |
| `APPLE_API_ISSUER` | Apple API issuer UUID |
| `CC_SIGN_IDENTITY` | Matching Developer ID certificate name or SHA-1 fingerprint |
| `TAURI_SIGNING_PRIVATE_KEY` | Encrypted Tauri updater signing key contents |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Updater key password |

Import only the intended signing identity, not every identity in a personal keychain. Never commit key files or paste secret values into issues, logs or workflow YAML. The workflow writes credentials under `RUNNER_TEMP`, removes them in an `always()` cleanup step, and uploads only the DMG, signed update archive, public signature/update manifest, checksums and notarization results.

The base major/minor/patch comes from `package.json`. The workflow adds `github.run_number` to its base patch, sets the app version, and creates the corresponding `v` tag only after all checks succeed. For a base `0.2.0`, runs 1 and 2 release `v0.2.1` and `v0.2.2`. Failed runs can leave gaps. The publish job explicitly creates or checks the tag against the tested commit before using `gh release create --verify-tag`; a later branch push cannot silently retarget that release. Existing releases are not overwritten; use a new push or manual dispatch to produce a new version.

The build job has read-only repository access; only the separate publish job can create tags/releases. Actions are pinned to full commit SHAs, dependency locks are enforced, and Dependabot proposes reviewed updates. Each run keeps an immutable artifact for 14 days. Public repositories include GitHub provenance attestations; private repositories retain the notarization and checksum evidence without requiring GitHub Enterprise attestation support.

## Build and notarize

Install the hash-locked, release-only DMG tooling in a virtual environment:

```sh
python3 -m venv .venv-release
source .venv-release/bin/activate
python3 -m pip install --require-hashes --only-binary=:all: -r scripts/release-requirements.txt
```

```sh
python3 scripts/test-sign-browser.py
bun test
python3 scripts/release-macos.py
```

The release script performs this sequence:

1. Generate both app icons from the compressed brand assets.
2. Set the browser bundle identifier to `ai.local.requirement-french.browser` explicitly, even on a fresh upstream engine. Sign browser Mach-O files and nested bundles from the inside out with Hardened Runtime and secure timestamps. Only Renderer/GPU helpers receive `com.apple.security.cs.allow-jit`.
3. Submit the browser archive to Apple, require `Accepted`, staple and validate its ticket.
4. Embed the stapled browser archive, build and sign the cleaner with the embedded frontend (`custom-protocol`).
5. Notarize and staple the cleaner, then require Gatekeeper acceptance for both apps.
6. Stage the engine's `Browser.app` as the distribution name `CCBaga-B.app`. Use `dmgbuild` to package both apps and the Applications shortcut in an 800 × 500 pixel-art Finder window, with fixed icon positions and a drag arrow. Notices remain included as hidden files. No Finder UI automation is needed. Read-only mount the result and verify both app names, bundle identifiers and entry points, then embedded app signatures, Gatekeeper acceptance, shortcut and background before signing, notarizing, stapling and assessing the DMG.
7. Archive the final stapled A bundle as `.app.tar.gz`, verify the unpacked app’s signature, ticket and Gatekeeper acceptance, then sign those exact bytes with `tauri signer sign --app-version`. Automatic pre-staple updater artifacts stay disabled so the published signature covers the final bundle. The signing secrets are excluded from Cargo/frontend build environments and DMG tool subprocesses.
8. Write standard `latest.json` for `darwin-aarch64`, pointing to this version’s public GitHub Release. Run the official updater against the actual archive on a test-only loopback server; reject tampered bytes and forged versions without installing it. Write checksums for all downloadable installer/update files and the public notarization records.

CCBaga-A uses Tauri’s official updater with HTTPS, an embedded public key and `requireSignedVersion`. The sidebar never installs automatically. A updates in place and restarts; B is updated using the complete dual-app DMG. No authentication token is needed for this public repository. GitHub’s pinned Rust cache stores dependency builds, not signing keychains or release artifacts.

Apple may take several minutes to process each submission. A submission ID alone is not acceptance. The script stops on any failed build, signature, notarization, ticket or Gatekeeper check. Rebuilding or modifying a signed bundle requires a new signature and notarization.

## Before publishing

- Resolve the copied Cromma UI/Pika icon license provenance described in [NOTICE.md](../NOTICE.md).
- Inspect the actual staged source and release contents for credentials and user data.
- Run the native checks listed in the README and user acceptance on supported macOS hardware.
- Push reviewed code to `release`; inspect the successful Actions run and its uploaded DMG and `SHA256SUMS`.

Do not remove quarantine attributes, disable Gatekeeper or ask users to bypass a security warning. Signing and notarization establish publisher identity and Apple acceptance; they do not replace functional testing or guarantee every future macOS version.
