<!--
[INPUT]: Current UI/native behavior and dated official Anthropic terms, policies, and support pages.
[OUTPUT]: User-facing operation guide, account-risk boundaries, official recovery routes, and source dates.
[POS]: Repository usage guide; English counterpart of usage.zh.md and the in-app bilingual help page.
[PROTOCOL]: Update this header when changing the guide, then check CLAUDE.md.
-->
# CCBaga usage and safety guide

English · [简体中文](usage.zh.md)

Last reviewed: 2026-10-12, against repository code including the 0.2.34 functionality. Open **Usage guide** in the sidebar or from the notice on **Easy setup**. Interface language changes only the interface; it does not change the machine or browser language.

Check policy effective dates: Anthropic's main Usage Policy page already displays the version effective **2026-11-12**, announced on October 8. The preceding version is effective **2025-09-15**. The sources below distinguish these versions; a published future version is not treated as already in force. Policies and product behavior can change.

## What CCBaga does

CCBaga-A is a local Tauri application. It manages selected Claude Code and Claude Desktop files on the current computer, starts and observes the separate CCBaga-B browser, reads selected system settings, and provides fixed links and an install prompt for the optional Network Skill. CCBaga-B reports measurements back to CCBaga-A through a tokenized loopback session.

The application does not operate Anthropic's servers, validate an account with Anthropic, decide whether a location is supported, or guarantee that a browser or network configuration will be accepted. A green browser result means that the listed measurements matched the selected target at that moment. It is not an anonymity, privacy, or account-acceptance guarantee.

## Page guide

- **Easy setup** prepares a region for the action you select; merely selecting a region does not apply it. Cache cleanup closes B, clears the listed caches, and reopens it only if it was running. Fingerprint rotation restarts B with a new seed, the selected language/timezone, and an empty explicit browser proxy to follow TUN; verify the exit afterwards. System reset applies the region's timezone/language/format. The Claude action opens the identity confirmation; it does not delete data immediately.
- **Claude identities** archives and restores approved local Claude directories. Login and identity data travel with an archive. Skills, plugins, MCP configuration, personal settings, project records, conversation history, and caches are separate retention choices. The empty identity creates a clean local workspace; it does not hide the device or erase server-side records.
- **Browser** separates editable settings from measured observations. Clearing site data removes the companion browser's cookies, site storage, and recovery sessions without a backup. The page does not independently verify DNS or WebRTC leaks.
- **Computer** reads and applies timezone, preferred language, region, and location settings. Some changes need system authorization, a restart, or sign-out. These controls do not change the proxy node or public IP.
- **Network** only links to fixed resources and provides a bilingual Skill installation prompt. The app does not collect SSH keys or proxy credentials and does not perform remote deployment. Review the prompt, paths, keys, and commands in the selected AI tool before approving any remote change.

### Identity and deletion choices

Before a switch, save Claude work and expand **Keep in workspace**. Checked categories stay shared in the active workspace; unchecked categories move into the archive. Login and identity data always move. Confirm the outgoing name, target, and retained categories; recognized Claude processes are stopped after confirmation. Each switch saves the outgoing state before restoring the target. If a transaction was interrupted, use the recovery option before another switch. Expired or revoked logins can still require authentication again.

**Direct delete** removes the current login/identity data and unchecked categories without creating a new archive; existing archives remain. Deleting a saved card deletes only that archive, not the current workspace. These actions require confirmation and cannot be undone by the app. Local archives are encrypted, but are not an external backup, a server-account backup, or a guarantee against access by the same user or an administrator. Only the default supported directories are managed; custom Claude profiles may remain outside this scope.

### Browser, system, CLI, and updates

Stop B before saving fingerprint/proxy settings or clearing site data. Cache cleanup keeps sign-ins, bookmarks, and passwords; **Clear site data** also deletes the listed cookies, site storage, and recovery sessions without a backup. Setting B as the default browser requests the operating system's confirmation. **Align US environment** checks a US exit and changes local timezone, language, region, and location settings; it does not change a proxy node. Measurements are snapshots, and different domain rules can still route Claude differently from an exit-check service.

**Claude Code CLI** maintenance is inside Claude identities. It detects supported installation methods and confirms uninstall/reinstall before stopping recognized Claude processes. It preserves personal data and the identity vault. Reinstall requires network access. Installer download/parse checks can fail before the old native program is removed, but a later installation failure is not a full rollback guarantee and may leave a partial installation. Read the log and reported state before retrying.

**Check for updates** checks availability; installing requires your choice. Windows updates install both A and B after closing B. macOS in-app updates replace A only; use the complete DMG for B. Linux uses packages/package managers or source updates. Save work before installing and restarting.

## Claude account and ban risk

Even while using CCBaga, a Claude account may be warned, rate-limited, suspended, or terminated. The project cannot predict or prevent that outcome and is not an official Anthropic product. Local changes to language, timezone, browser fingerprint, proxy, VPS, or network exit do not make an unsupported location or use case compliant, restore an account, or remove server-side records.

The official safeguards article lists repeated Usage Policy violations, account creation from an unsupported location, and Terms of Service violations as ban categories. They do not diagnose a specific account. Relevant rules come from several documents:

- **Consumer Terms (effective 2025-10-08), §§2–3:** account/credential sharing and resale are prohibited. Automated or non-human service access requires an Anthropic API key or other explicit permission. This exception matters: permitted Claude Code or API use is not automatically prohibited automation. The terms also prohibit bypassing systems/protective measures; §12 describes suspension and termination.
- **Usage Policy (effective 2025-09-15), “Do Not Abuse our Platform”:** prohibits malicious coordination across accounts to evade detection/guardrails, automated registration or spam, ban circumvention through another account, and access that violates supported-region rules. Other policy sections address prohibited content and harmful uses.
- **Usage Policy update (effective 2026-11-12):** explicitly expands examples about unauthorized proxy/resale access and misrepresenting the product/client. These examples are identified as a future-effective update on this review date, not as the exact wording of the preceding policy.

The Supported Regions Policy says that use by people physically located in an unlisted region, and use by entities incorporated, headquartered, or controlled from an unlisted region, is unsupported. A proxy or fingerprint setting cannot change that policy determination.

This guide does not give a ban probability or claim that an IP, keyword, Chinese input, fingerprint, or CCBaga caused a ban. A social-media report cannot by itself establish cause for another account. Direct X-post verification was unavailable during this review; no X post is cited as factual evidence here. Conclusions are based on the official documents linked below, not search snippets or third-party “ban avoidance” advice.

## If access is restricted

1. Preserve the visible message, time, sign-in method, and support correspondence. Do not send passwords, cookies, session tokens, API keys, or full payment details to a third party.
2. Distinguish a login error, usage limit, or [service incident](https://status.claude.com/) from an explicit suspension/termination notice. If an organization is on hold while your account is in good standing, use **Request a review** for that organization.
3. For an account suspension/termination, sign in with the affected account at [claude.ai/restricted](https://claude.ai/restricted) and submit the official appeal. The form requires login. Banned Free, Pro, and Max users can access export/deletion options there; the export scope can be restricted by the violation involved.
4. Do not create or borrow an account to evade a ban or use unauthorized access relays. Appeal outcome, timing, data availability, and any refund depend on Anthropic's applicable rules and purchase channel; CCBaga cannot promise them.

## Official sources

- [Usage Policy, preceding version](https://www.anthropic.com/legal/archive/22742366-2ef0-4c7a-a833-6523f10d3944) — effective 2025-09-15; platform-abuse and prohibited-use rules.
- [Usage Policy, latest published version](https://www.anthropic.com/legal/aup) — effective 2026-11-12 at review time.
- [2026 Usage Policy update announcement](https://www.anthropic.com/news/2026-usage-policy-update) — published 2026-10-08; confirms the November 12 effective date.
- [Supported countries and regions](https://www.anthropic.com/supported-countries) — current supported-location and supported-entity rules.
- [Safeguards warnings and appeals](https://support.claude.com/en/articles/8241253-safeguards-warnings-and-appeals) — ban categories, account/organization appeals, and data controls.
- [Anthropic Consumer Terms](https://www.anthropic.com/legal/consumer-terms) — effective 2025-10-08 at review time; account responsibility, permitted access, suspension, and termination.

This guide is explanatory documentation, not legal advice and not a promise of account access.
