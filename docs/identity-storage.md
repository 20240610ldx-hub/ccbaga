# Local identity storage

CCBaga manages a local snapshot, not a server-side identity. Sign-in validity is determined by Anthropic. Only manage accounts you own and use them in accordance with the official terms.

## What an identity contains

Claude Code separates authentication from application data. Its [authentication reference](https://code.claude.com/docs/en/authentication) documents macOS Keychain credentials, the Linux/Windows credential file and separate configuration directories. The [directory reference](https://code.claude.com/docs/en/claude-directory#application-data) distinguishes transcripts, tool outputs, file snapshots, attachments, logs and configuration backups.

CCBaga archives the approved sign-in fields and selected Desktop login stores. The user's retention choices determine whether skills, plugins, MCP, settings, project state, history and caches stay in the workspace or follow the snapshot. Actual project source directories are excluded. Custom configuration directories and Console profiles outside the default directory are not automatically switched.

A large archive can contain conversation transcripts, images, tool results, file history and Desktop databases. These are not all authentication requirements. They remain recoverable because the user selected them for archiving. Keeping a category in the workspace avoids copying it into subsequent snapshots but also leaves that category visible there. Compression cannot guarantee a fixed size, particularly for images and compressed database content.

## One slot per recognized account

The native layer matches the account UUID and organization UUID. When the account UUID is unavailable, it uses a normalized email and the organization identifier; an organization name is a conservative fallback when its UUID is missing. The matching key is an HMAC-SHA256 digest under the local vault key, not a token or raw account identifier in SQLite. Display names and refreshed access tokens are not matching keys.

Saving a recognized account updates its existing slot and preserves its custom name. The current account has one current card, without another recent-use card. A manual sign-in to another account is matched against the actual local account metadata rather than the last restored slot. Unknown identities, including Desktop-only identities without a recognized Code account, are kept separate instead of being guessed or merged. This does not verify that local account metadata is current on the server.

## Safe replacement and recovery

Each update writes a new immutable archive generation. The archive is synchronized and fully authenticated before SQLite commits its pointer together with the recovery marker. Old generations are collected only after a committed replacement; a rejected database update leaves the previous generation intact. The recovery checkpoint is independent and cannot overwrite the snapshot needed for rollback.

SQLite v1/v2 catalogs migrate transactionally to v3. Legacy snapshots are indexed after archive verification. Duplicate recognized slots converge after a completed operation. Damaged and unidentified snapshots are not silently merged.

New archives use [Zstandard](https://github.com/facebook/zstd) level 3 with a checksum inside the existing AES-256-GCM authenticated container. Old gzip archives remain readable. Decoders validate the encrypted terminator, reject appended or truncated data and bound the Zstd window. Scratch extraction files do not need per-file synchronization because the durable authenticated archive remains the recovery source; restored workspace files are synchronized before completion.

The identity stop command shares the precise process matcher with CLI maintenance without rescanning all historical files. File traversal retains ownership and link checks, checks process state periodically and forces checks at transaction boundaries. It does not remove the recovery or validation steps to improve speed.

## Reproducible local checks

Run native regressions with `cargo test --manifest-path src-tauri/Cargo.toml`. They use temporary accounts and files, including repeated A/B switching, token refresh, organization isolation, rejected updates, interrupted recovery and v1 gzip migration.

Run the optional synthetic measurement with:

```sh
cargo test --manifest-path src-tauri/Cargo.toml synthetic_switch_and_compression_benchmark -- --ignored --nocapture
```

A local macOS debug run with 500 synthetic 256 KiB session files measured approximately 0.72 seconds to stash and 3.19 seconds to restore. A separate 13.5 MB JSONL sample compressed to 1.56 MB with Zstd versus 1.69 MB with gzip. These are synthetic measurements, not a latency or size guarantee for real accounts or other platforms. The normal test suite does not enforce machine-dependent timing thresholds.

The vault is protected local storage, not an isolation boundary against other programs running as the same user or an administrator. Encryption does not remove server records or guarantee that installed software cannot discover the vault.

## Linked browser data

**Link browser** is on by default for users without a saved preference; an explicit choice to turn it off is preserved. Initial browser data stays in place and is associated on the first identity save. Later identities get separate Chromium user data directories and configurations; existing data is retained. When re-enabling linking, the confirmation allows choosing whether to associate existing data with an unbound identity. Each directory uses Chromium's internal `Default` profile, with separate cookies and site storage. Switching closes the linked browser normally and reopens the selected directory. Save browser work first.

SQLite commits the browser selection with the identity switch and restores the previous selection during rollback. An interrupted identity operation blocks browser startup until recovery. An empty identity receives a fresh browser directory. Reopening a saved identity reuses its directory; browser data is not compressed, copied or merged during switching. A failure to reopen the browser is reported separately after a committed identity change, with a retry action.

The initial nonzero 32-bit fingerprint seed is derived from CCBaga's own random identity ID using SHA-256, with collisions registered and resolved locally. An unbound fresh browser directory reserves its random ID for the first saved identity, avoiding a seed change at that first save. It is not derived from an Anthropic account UUID or token. Renaming or refreshing credentials does not change it. Explicit fingerprint changes remain local to that identity and persist across switches.

Browser data remains outside the encrypted identity archive. Chromium and the OS manage its storage protection; this feature does not make the browser directory a CCBaga-encrypted vault. Unlinking returns to the original shared browser directory and retains identity directories. Deleting a saved identity retains browser data by default; deleting the associated browser data requires a separate selection and refuses a directory that is in use. No claim of secure disk erasure is made.

First use still requires website sign-in. Restoring Code/Desktop credentials does not create website cookies or verify that all clients use the same account. Separate profiles do not change the public IP, prevent server-side correlation, or guarantee that browsers, extensions and websites make no external connections. Custom profiles created manually inside Chromium remain outside the identity-card model. See the [Chromium user data directory reference](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/user_data_dir.md).

The browser check page places a compact identity card above the environment checks. It shows bounded display fields for the local browser binding, not a verified website login. If the browser belongs to A but local Claude metadata changes to B, the card retains A and marks it out of sync. Account tokens, account UUIDs and matching digests are never included in the page context. Disconnected and unreadable states are explicit.
