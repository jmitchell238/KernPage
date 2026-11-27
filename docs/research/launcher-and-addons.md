# KernPage Launcher and add-ons: research and plan

*Research date: 2026-11-12. Research only; nothing here is built yet. Where a fact could not be confirmed from a public source it is marked **(verify)**.*

## 1. What we want

KernPage ships with a small, separate program, the **KernPage Launcher**. It is the front door for everything around the editor:

- shows the installed KernPage version, the latest version, and an update badge;
- Open KernPage, Open file, Recent files;
- an "Available to install" section for add-ons (template packs, spelling dictionaries, offline translation models, fonts, later plugins);
- settings, Windows toast notifications, and a clear "X is available: What's new / Later / Update and restart" prompt;
- a details panel for KernPage itself and for each add-on (section 3).

Design rules that make it trustworthy: it never installs anything the user did not approve (unless they turned on automatic updates), every download is checked against a signature that the app already trusts, nothing is sent except a plain request for a public file, and a failed update never leaves the user without a working KernPage.

## 2. The reference: ArtCraft Launcher

Source: the public repository [storytold/craft-launcher](https://github.com/storytold/craft-launcher) (README read on 2026-11-12). We describe behaviour only; no code is copied.

- **Licence:** dual MIT or Apache-2.0 (KernPage itself is GPL-3.0-only from after 0.2.0). The name, logo and "A" mark are trademarks excluded from the licence. Lesson: we can freely study and even reuse ideas, but must never reuse its branding.
- **Technology:** a Rust workspace; the desktop app is native egui (no Electron or web view), plus a WebAssembly build for the browser. Tray icon on Linux via StatusNotifierItem. This is the same stack as KernPage, which makes it the best reference we have.
- **Update source:** reads each app's GitHub Releases, picks the build for the user's OS and CPU, and verifies it against the release's SHA-256 checksums before installing. A local mirror can replace GitHub for testing (set by an environment variable and only honoured for loopback addresses; `cargo xtask local serve` serves "the builds and a release feed"). Its feed schema is not documented in the README **(verify in the source)**.
- **Windows install:** uses the release MSI through Windows Installer. Per-machine installs go to Program Files and ask for administrator permission each time; per-user installs go to `%LOCALAPPDATA%\Programs\...` and keep that scope on update. The launcher's own MSI is per-user and needs no admin rights.
- **Self-update:** "Update and restart" downloads, verifies, replaces and reopens; a running app is never replaced; the replacement uses the same package format the launcher was installed with. The README does not name a helper executable, but the owner's description (a helper exe) matches the standard pattern in section 6.
- **UX:** grid and table views, light and dark, tray icon, 13 languages, notifications, logs that keep the previous two runs.
- **What we do better:** require a *signature* (not only a checksum from the same place as the file), keep the previous version for rollback, and describe add-ons with a richer, signed index.

## 3. The details panel (owner request)

Clicking a card (KernPage itself or any add-on) opens an **App details** panel, modelled on ArtCraft's:

- **Header:** big icon, name, one-line tagline, "Version X . date", a primary button (Open / Install / Update) with a "..." menu (Reinstall, Roll back, Uninstall, Copy link, Show in folder, Release notes), buttons for Website, GitHub and Releases, and previous / next / close arrows so the user can page through cards.
- **Left column, "01 / Features":** numbered feature cards (title + two lines of text, optional small image).
- **Right column, "02 / Latest release":** tiles for version, released date, download size and platforms; then "What's changed" taken from the release notes, showing the first few items with "Show N more", and a "Full changelog" link.

Every one of these fields has to exist in the feed, so the index (section 7) carries them for the app and for add-ons alike. Notes are shipped as short structured Markdown in the index so the panel never needs to load a web page; the full changelog is only a link.

Accessibility for the panel: arrow keys and Tab reach every button, Esc closes, prev/next have text names, and the panel is read correctly by Windows Narrator through AccessKit (egui already uses it).

## 4. How best-in-class products do it

| Product | What to learn | Source |
|---|---|---|
| **JetBrains Toolbox App** | Installs, updates, rolls back and uninstalls each IDE; keeps the previous version of each tool for instant rollback and by default deletes those backups after three days (a setting). Early-access builds are installable from the same app. | [JetBrains Toolbox](https://www.jetbrains.com/toolbox-app/), [FAQ](https://jetbrains.com/toolbox-app/faq) |
| **Adobe Creative Cloud desktop app** | "Other versions" menu on every app tile; only the current and previous major releases are offered; old versions are removed by default. Users complain when updating silently deletes older versions: a warning about keeping what the user had. | [Adobe help](https://helpx.adobe.com/download-install/using/acc/install-previous-version.html), [community thread](https://community.adobe.com/t5/download-install-discussions/warning-updating-creative-cloud-desktop-changed-my-preferences-deleted-old-versions-of-cc-apps/td-p/12552845) |
| **Affinity** | Could not find reliable public information on its updater **(verify)**. Worth noting only that the 2025 relaunch is a free single app for Windows and Mac ([DigitalProduction](https://digitalproduction.com/2025/10/30/affinity-free-all-in-one-design-suite/)). |
| **VS Code extensions** | Per-item page with README, feature list, changelog tab, version history and pre-release toggle; extensions update on their own and can be pinned. Marketplace terms limit use to Microsoft's products, so open forks use the Open VSX registry. **(verify details)** |
| **Blender Extensions platform** | The model closest to what we want. A repository is a static `index.json` (a version field, a blocklist and a `data` list); each entry includes the archive URL, size and a hash written as `sha256:...`; extensions carry a `blender_manifest.toml`. Anyone can host a repository, even from a local folder. Includes a **blocklist** to pull bad items. | [Static repository](https://docs.blender.org/manual/en/dev/advanced/extensions/creating_repository/static_repository.html), [listing API v1](https://developer.blender.org/docs/features/extensions/api_listing/v1/) |
| **Inkscape, Scribus, LibreOffice** | Add-ons and dictionaries are found on web pages and installed by hand (an `.oxt` file in LibreOffice, extension folders in Inkscape). Works, but discovery, versions and updates are manual. This is the gap to beat. **(verify)** |
| **Velopack** | Successor to Squirrel. Rust core plus a Rust crate, Windows/macOS/Linux, automatic **delta** packages, channels (one `releases.{channel}.json` per channel), a built-in GitHub Releases source. Rollback is not a self-hosted feature; it is done by editing the feed. | [Velopack docs](https://docs.velopack.io/), [deltas](https://docs.velopack.io/packaging/deltas), [channels](https://docs.velopack.io/packaging/channels), [crate](https://docs.rs/crate/velopack/1.1.1/source/README.md) |
| **Squirrel.Windows** | Windows-only, described by its own fork maintainer as in disrepair; Clowd.Squirrel is archived in favour of Velopack. Do not choose it. | [Clowd.Squirrel](https://www.GitHub.com/clowd/Clowd.Squirrel) |
| **WinSparkle** | Classic, GUI-first updater for Win32 apps, appcast feed; no delta updates per an older comparison **(verify)**. C library, would need FFI. | [comparison](https://omaha-consulting.com/best-update-framework-for-windows) |
| **tauri updater / cargo-packager-updater** | JSON endpoint plus **minisign** signature, verified with the `minisign-verify` crate. Proves the "static JSON + minisign" design at scale. | [minisign-verify](https://lib.rs/crates/minisign-verify), [tauri-plugin-updater deps](https://deps.rs/crate/tauri-plugin-updater/2.10.1) |
| **cargo-dist, `self_update` crate** | cargo-dist builds installers and release pages and can ship an updater for the *installer it made*; `self_update` replaces a single binary from GitHub Releases. Both suit command-line tools, not an MSI-installed GUI suite with add-ons. **(verify, not covered by sources found)** |
| **MSIX / App Installer** | Native Windows updates, rollback and clean uninstall, but needs a signing certificate Windows trusts (or a sideload setup) and changes how files and fonts behave. A good later option, not a starting point. **(verify)** |

## 5. Recommendation in one paragraph

Keep the **WiX MSI** as the one installer format. Build the Launcher as a second small egui program that talks to a **signed static JSON index** hosted on GitHub Pages and downloads files from **GitHub Releases**. Verify every download with **SHA-256 from the index plus a minisign (ed25519) signature**, then run the **MSI silently with a verbose log**. Do not adopt Velopack as the core: it wants to own the installer format and file layout. Borrow its ideas (channels, deltas, a helper that applies the update after the app exits).

## 6. Architecture

### 6.1 Programs and installs

- `kernpage.exe` (the editor), `kernpage-launcher.exe` (dashboard, tray, updater) and `kernpage-updater-helper.exe` (tiny, no UI, sits next to the launcher).
- **Per-user vs per-machine.** Recommended default: **per-user** install under `%LOCALAPPDATA%\Programs\KernPage`, so updates never need UAC (the model ArtCraft uses for its launcher). Offer **per-machine** for schools and offices through a separate MSI choice, and keep the scope on upgrade. A per-machine update needs an elevation prompt each time: the helper requests it once, only after the user clicks Update. A dual-purpose WiX package can do both; this is a packaging decision for step 3 (see section 9).
- **Add-ons** live in a per-user data folder (for example `%LOCALAPPDATA%\KernPage\addons\<id>\<version>`), not Program Files, so installing a dictionary never needs admin rights. System-wide fonts are the exception; install fonts per-user (Windows 10 1809 and later supports it) **(verify)**.
- The launcher and editor share a small config file so settings (channel, appearance) agree.

### 6.2 The update flow

1. Launcher fetches `index.json` and `index.json.minisig` (a plain HTTPS GET, ETag/If-None-Match to save bandwidth).
2. Verifies the signature with the public key **compiled into the launcher**. If it fails: show "Could not verify the update list", change nothing.
3. Compares versions (SemVer, `semver` crate), applies channel rules (stable, optionally pre-release), shows the badge.
4. On "Update and restart": download to a temp folder with resume support; check size, SHA-256 and the file's own minisign signature; check the **Authenticode** signature too (`WinVerifyTrust`) once we sign.
5. Close the editor politely (ask to save; never kill it). Start the helper and exit the launcher.
6. Helper runs `msiexec /i <file>.msi /qn /norestart /l*v <log>` (major upgrade in the MSI replaces the old version), records success or the exit code, relaunches the app.
7. Before step 6 the old MSI (or its version number and download URL) is kept so **rollback** can reinstall it. Keep the previous version for a configurable time (JetBrains: three days; we suggest "keep the last one until the next successful update").
8. Failure: log is one click away ("Show log"), the old version is reinstalled automatically when the MSI exit code is not 0 or 3010.

### 6.3 Recommended crates

| Need | Crate | Notes |
|---|---|---|
| Window UI | `eframe`/`egui` (already used), `egui_extras` | Same toolkit as the editor and ArtCraft. |
| HTTP | `reqwest` with `rustls-tls` (or lighter `ureq` for the blocking helper) | Proxy: honour `HTTPS_PROXY`/system proxy; `reqwest` does by default **(verify for system proxy on Windows)**. |
| JSON | `serde`, `serde_json` | |
| Signature | `minisign-verify` (zero dependencies, streaming verifier) | [lib.rs](https://lib.rs/crates/minisign-verify). Sign with the `minisign` tool or `rsign2`. Verify-only code in the app keeps the attack surface tiny. |
| Hash | `sha2` | |
| Versions | `semver` | |
| Authenticode check | `windows` crate (`WinVerifyTrust`) | |
| Toast notifications | `tauri-winrt-notification` or `winrt-notification` (both depend on the `windows` crate) | [deps.rs](https://deps.rs/crate/tauri-winrt-notification/0.7.2). Note: action buttons ("Update and restart") need an app identity (an AppUserModelID registered by the MSI shortcut). |
| Tray icon | `tray-icon` | Used by Tauri. |
| Start at login | per-user `Run` registry key via `windows-registry`/`winreg` | |
| Single instance | named mutex via `windows` crate | |
| Log | `tracing` + `tracing-appender` | Keep two previous logs like ArtCraft. |
| Paths | `directories` | |
| Archives | `zip` | For add-on packs. |
| Delta (later) | `bsdiff`/`qbsdiff` or `zstd` with a dictionary | Only if full MSI size becomes a problem. |
| Embedded self-test | local fake feed server: `tiny_http` | For step 2 testing. |

`self_update` and Velopack's crate are viable, but each assumes its own packaging; the table above keeps the MSI.

### 6.4 Integrity and signing

- **Required, not optional:** every installable thing (app MSI and every add-on) is covered by SHA-256 *and* a minisign signature from a project key. The signing key is stored offline (hardware key or an encrypted file held by the owner), never in plain CI secrets for app releases. Publish the public key in the repository and in the launcher.
- **Key rotation:** the index lists the next public key; the launcher accepts a key change only when the new key is signed by the old one. Keep a documented recovery plan.
- **Why not TUF or Sigstore first:** TUF gives rollback and freeze protection, but is large to run for a small project. Cheap equivalents: put an `expires` date and a monotonically increasing `serial` in the index; the launcher rejects an older serial or an expired index (stops downgrade and "freeze" attacks), and a `blocklist` as in Blender. GitHub artifact attestations and Sigstore cosign give a second, *provenance* proof (this file was built by this workflow) and are free for public repositories; add them when the repo goes public. **(verify attestation availability for private repos)** Sources: [GitHub blog on provenance](https://github.blog/security/supply-chain-security/where-does-your-software-really-come-from/), [cosign README](https://cdn.jsdelivr.net/gh/sigstore/cosign@main/README.md).
- **Authenticode code signing** (separate from the above; it is what Windows SmartScreen and "Verified publisher" look at):
  - **SignPath Foundation**: free certificates for open-source projects; the certificate is issued to SignPath Foundation as the publisher; needs an OSI-approved licence, a public repository, multi-factor authentication for all team members, a published code signing policy page, and a stated attribution. Examples that use it: [Pandoc](https://pandoc.org/code-signing-policy.html), [Ink/Stitch](https://inkstitch.org/code-signing-policy), [OpenRCT2](https://openrct2.io/code-signing-policy). Sources: [SignPath open source](https://signpath.io/product/open-source), [terms](https://signpath.org/terms). **Needs the repo to be public, so it must wait until the owner decides to release.**
  - **Azure Artifact Signing (formerly Trusted Signing)**: low monthly cost (check current pricing); public-trust certificates were limited to organisations in the USA, Canada, EU and UK, with individual availability unclear and reported problems for individuals outside those regions. Check Microsoft's current page before relying on it. Sources: [Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/5915697/can-an-irish-sole-trader-(non-incorporated)-qualif), [DevClass](https://www.devclass.com/security/2026/01/14/code-signing-windows-apps-may-be-easier-and-more-secure-with-new-azure-artifact-service/4079554).
  - **Paid OV/EV certificate** (DigiCert, Sectigo, GlobalSign) as the fallback.
  - Until signed, local MSI builds are fine; no public release before the owner agrees. The existing `packaging/windows/sign.ps1` is the place to wire this up.
- **Add-on trust:** official add-ons are signed by the project key. Community add-ons (later) get a second tier: a separate "community" index, clearly labelled, signed by the maintainers after review. Plugins that run code are a later, separate security design (sandbox, permission list); do not ship them before that exists.

### 6.5 Privacy

- No telemetry, no accounts, no identifiers. The update check is a plain HTTPS GET of a public file; GitHub and the CDN will see an IP address and the `User-Agent` (use a fixed string such as `KernPageLauncher/1.2` and say so in the docs; do not include machine or user ids).
- A single switch, **"Check for updates automatically"**, is off-able; "Manual" means the launcher never connects until the user clicks "Check now". Document exactly what is requested in Help.
- Offline: the launcher starts and works without a network, shows "Last checked: date", caches the last good index, and add-ons already installed keep working.
- Proxy: use system proxy settings, allow a manual proxy URL in settings, and show a clear error with the log path.

### 6.6 Other topics

- **Delta updates:** not needed at first (MSI is small; Velopack and Squirrel need them for big Electron apps). Add only if the MSI grows beyond tens of MB; add-ons can use per-file hashes to download only changed files.
- **Rollback:** list of past releases from the index; "Install this version" runs the same flow with a confirmation; refuse to go below a `minimum_supported` version listed in the index.
- **Appearance:** follow the system theme, with light/dark override.
- **Accessibility:** full keyboard use, AccessKit, high-contrast, scalable text, no information by colour alone (the update badge also has text).
- **Cross-platform future:** keep the index format platform-neutral (`platforms` array with `os`, `arch`, `kind` = `msi`, `dmg`, `appimage`, `zip`, `deb`). Windows uses the MSI flow; Mac would use signed and notarized `.dmg`/`.pkg`; Linux could use AppImage or distro packages, where the launcher only *notifies* and does not install over a package manager. ArtCraft supports the same set of formats ([README](https://github.com/storytold/craft-launcher)).

## 7. Feed format

Two layers, both static files on GitHub Pages (free, version-controlled, CDN-backed), signed:

- `https://<pages-host>/kernpage/index.json` + `index.json.minisig`
- binaries on GitHub Releases (the index gives the exact asset URLs; the launcher never scrapes the Releases API, which avoids rate limits and keeps behaviour identical if hosting changes). A second mirror URL per file is allowed.

Design points: modelled on the Blender listing (a version field, a blocklist, a data array), Velopack (channels), and what the details panel needs.

```json
{
  "schema": 1,
  "serial": 42,
  "generated": "2026-11-12T00:00:00Z",
  "expires": "2027-02-12T00:00:00Z",
  "keys": { "current": "RWQ...", "next": null },
  "blocklist": [ { "id": "dict-xx", "versions": "<1.2.0", "reason": "..." } ],
  "items": [
    {
      "id": "kernpage",
      "kind": "app",
      "name": "KernPage",
      "tagline": "Open, edit and print Publisher files.",
      "icon": "https://.../icons/kernpage.png",
      "website": "https://...", "repo": "https://github.com/OWNER/kernpage",
      "releases_url": "https://github.com/OWNER/kernpage/releases",
      "license": "MIT OR Apache-2.0",
      "features": [
        { "title": "Opens .pub files", "text": "Pages, text, pictures and tables keep their layout.", "image": null }
      ],
      "channels": {
        "stable": [
          {
            "version": "0.9.0",
            "released": "2026-12-01",
            "prerelease": false,
            "min_launcher": "0.2.0",
            "notes": "## Added\n- ...\n## Fixed\n- ...",
            "changelog_url": "https://.../CHANGELOG.md#090",
            "files": [
              {
                "os": "windows", "arch": "x86_64", "kind": "msi", "scope": ["user", "machine"],
                "url": "https://github.com/.../kernpage-0.9.0-x64.msi",
                "size": 31457280, "sha256": "ab12...", "sig": "untrusted comment: ...\nRUQ...",
                "authenticode_subject": "CN=..."
              }
            ]
          }
        ]
      }
    },
    {
      "id": "dict-en-us", "kind": "dictionary", "name": "English (US) spelling",
      "tagline": "Hunspell dictionary", "icon": "...",
      "description": "Spelling and hyphenation for American English.",
      "license": "see addon", "requires": { "kernpage": ">=0.9.0" },
      "languages": ["en-US"],
      "channels": { "stable": [ { "version": "2026.10", "released": "2026-10-20", "notes": "...", "files": [ { "os": "any", "kind": "zip", "url": "...", "size": 1200000, "installed_size": 3500000, "sha256": "...", "sig": "..." } ] } ] }
    }
  ]
}
```

Every field in the details panel maps here: icon, name, tagline, `Version X . date` (version + released), Website/GitHub/Releases buttons (`website`, `repo`, `releases_url`), numbered features (`features`), the release tiles (version, released, `files[].size`, platforms from `files[].os/arch`), "What's changed" with "Show N more" (`notes`, shown first five items) and "Full changelog" (`changelog_url`). Add-ons use the same item shape, plus `kind`, `requires`, `installed_size`, `depends`, and `license` (essential for fonts and models). Old versions stay in the list so rollback and "What's new since your version" work. Items without a signature are rejected.

The index can be produced by a small `xtask` (the ArtCraft repo does the same with `cargo xtask`) run by the release workflow, with the final minisign step done on the owner's machine.

## 8. The add-ons catalogue

Cards show icon, name, tagline, version, size, description, last updated, status (Installed, Update available, Not installed) and a "..." menu: Install, Update, Reinstall, Remove, Details, Release notes, Website. Filters by kind and language; a search box.

Kinds and sources:

- **Template packs**: ZIP of `.pub`/KernPage documents with a manifest and per-template licence; created by us (no Microsoft templates may be used, see the research rules in CONTRIBUTING).
- **Spelling dictionaries (Hunspell)**: `.dic` and `.aff` files; licences vary per language (many are LGPL, MPL or GPL); show the licence on the card. Start from the LibreOffice dictionaries repository and verify each licence **(verify, no source retrieved)**.
- **Offline translation models**: Mozilla's Bergamot / Firefox Translations models and engine are under MPL-2.0 and run fully on the computer once downloaded ([Project Bergamot](https://en.wikipedia.org/wiki/Project_Bergamot), [mozilla/firefox-translations-models](https://github.com/mozilla/firefox-translations-models) **(verify model licence per language pair)**). Models are tens of MB each, so show sizes clearly and download only when chosen. This matches the ROADMAP's "Translate" item.
- **Fonts (OFL)**: SIL Open Font Licence families; the add-on includes the licence file. Per-user font install.
- **Plugins (later)**: only after a permission and sandbox design (for example WebAssembly with a capability list).

## 9. Phased plan with acceptance criteria

**Step 1: Launcher window, open and recent, settings (no network).**
- New crate `kernpage-launcher` (egui). Dashboard shows installed version (read from the install), Open KernPage, Open file (file picker, `.pub` and KernPage files), Recent files (shared list written by the editor; click to open; remove entry; clear list), and a Settings page with every control listed in section 1 stored in a config file (controls that need later steps are visible and disabled with a short note).
- Accept when: works offline; keyboard-only use works; light, dark and system themes switch live; closing with "keep in tray" on hides to the tray and reopens; "start at login" adds and removes the Run key; second launch focuses the first; unit tests for config load/save and recent list; Narrator reads the main controls.

**Step 2: Update check against a local fake feed.**
- Index parser, signature verification with a test key, serial/expiry rules, channel and pre-release filtering, update badge, "What's new" panel, details panel (section 3), toast notification, "Later" snooze, manual vs automatic mode, proxy field, offline cached index. Local fake feed served by a test server (`--feed-url http://127.0.0.1:PORT`, accepted only for loopback in release builds, like ArtCraft's mirror).
- Accept when: tests prove a tampered index, wrong key, expired index, older serial, and wrong hash are all refused with a clear message; manual mode makes zero network requests (test with a counting server); toast appears with What's new / Later / Update buttons; details panel shows every field from a sample index; the app starts with the network off.

**Step 3: Download, verify, install, relaunch, and the MSI changes.**
- Resumable download with a progress bar and cancel; SHA-256 + minisign + Authenticode check; helper exe runs the silent MSI with `/l*v` log, relaunches; automatic rollback on failure; manual rollback to a chosen older version; auto-update setting; WiX changes: stable `UpgradeCode`, major-upgrade element, per-user default and per-machine option, shortcut with AppUserModelID (for toast actions), launcher included in the same MSI, a `--version` handshake, no reboot required.
- Accept when: from version N a fake release N+1 installs and KernPage opens as N+1 with user data intact; a corrupted MSI is rejected before running; killing the helper midway leaves a working install; failed MSI triggers rollback; per-user update shows no UAC; the log file exists and "Show log" opens it; tests on a clean Windows VM for install, upgrade, repair and uninstall; `check-installer.ps1` extended accordingly.

**Step 4: Add-ons.**
- Catalogue view, install/update/remove into the per-user add-on folder, dependency and `requires` checks, signature verification per package, blocklist enforcement, disk-space check, and editor hooks (editor reads installed dictionaries, templates, fonts).
- Accept when: install, update and remove a dictionary, a template pack and a font from a local fake feed with no admin prompt; editor sees them without restart (or prompts for one); an incompatible add-on is greyed out with a reason; a blocklisted add-on is disabled on next check; removing an add-on leaves nothing behind.

**Before any public release (not part of the steps above):** owner's decision on going public; apply to SignPath Foundation or choose another certificate; publish the code-signing policy and the privacy statement; set up the offline key ceremony.

## 10. Open questions for the owner

**Decided 2026-11-12:** per-user install by default; index hosted on the project's GitHub Pages; launcher bundled in the KernPage MSI (no separate installer). Signing route is deferred until release.


1. Per-user default install (recommended) or per-machine?
2. Which signing route when the time comes (SignPath Foundation needs a public repo)?
3. Hosting of the index: GitHub Pages under the project (recommended) or a custom domain (lets us move hosting later without breaking old launchers).
4. Should the launcher be offered as its own MSI too, or only bundled?
