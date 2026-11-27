# Installing KernPage on Windows

## For users

Run `KernPage-<version>-Setup.exe` (shows the KernPage icon; installs just for you, no administrator prompt). It wraps the MSI below with a license page and one Settings > Apps entry. Or run `KernPage-<version>-x64.msi` directly. The installer shows a license page (MIT or Apache-2.0, your option), then installs. The finish page has **Launch** and **Close** buttons; silent installs never launch the app. A picture-by-picture guide for users is in [INSTALL.md](../../INSTALL.md).

- **Just for you (default):** no administrator rights needed. Files go to
  `%LOCALAPPDATA%\Programs\KernPage`.
- **All users:** choose this when Windows asks, or run `msiexec /i <msi> ALLUSERS=1` from an
  elevated prompt. Files go to `C:\Program Files\KernPage`.

You get two Start menu shortcuts. "KernPage" opens the editor (`kernpage.exe`) directly. If you turn on automatic update checks in the Launcher, the editor can also check quietly in the background and, when an update is available, show an "Update available" bar that starts the launcher; by default nothing is sent until you press Check in the Launcher (see [PRIVACY.md](../../PRIVACY.md)). "KernPage Launcher" opens `kernpage-launcher.exe` (installed beside `kernpage.exe` in the same MSI) for updates, going back to an older version and add-ons. Double-clicking a `.pub` (via Open with) starts `kernpage.exe` directly. You also get an entry in Settings > Apps. Installing a newer MSI replaces the
old version in place; installing an older one also replaces it (this is how the launcher's "Install an older version" goes back to a previous release; your publications are not touched). Uninstall from Settings > Apps.

`.pub` files: KernPage appears under **Open with**. The installer never makes it the default
program, so Microsoft Publisher (or whatever you use) keeps double-click.

`.kern` files (KernPage's own format): KernPage is the default program, so double-click opens them.

KernPage is an independent project. It is not affiliated with, endorsed by or sponsored by Microsoft. Microsoft, Publisher, Office and Windows are trademarks of the Microsoft group of companies.

Silent install: `msiexec /i KernPage-<version>-x64.msi /qn` (per-user), add `ALLUSERS=1` for
all users. The package is dual-purpose (`ALLUSERS=2`, `MSIINSTALLPERUSER=1`), which is why a plain
install is per-user; `ALLUSERS=1` (elevated) forces per-machine.

### Desktop shortcuts

Both installers can add desktop shortcuts: "KernPage" (on by default) and "KernPage Launcher"
(off by default). `Setup.exe` shows two checkboxes on its license page (custom theme `bundle-theme.xml` /
`bundle-theme.wxl`, based on WiX's stock RtfTheme). The MSI takes the public properties
`DESKTOPSHORTCUT_APP` and `DESKTOPSHORTCUT_LAUNCHER` (`1` = create, `0` = none), e.g.
`msiexec /i <msi> /qn DESKTOPSHORTCUT_LAUNCHER=1 DESKTOPSHORTCUT_APP=0`; `Setup.exe /quiet` accepts the
same names. The choice is remembered under `Software\KernPage` (HKCU, or HKLM for all-users), so an
upgrade without those properties (the launcher's update) keeps it; a value given on the command line wins.
Uninstalling removes the shortcuts. Re-running `Setup.exe` over an existing install starts with the defaults again.

### Setup.exe vs the MSI

`Setup.exe` is a WiX Burn bundle (`kernpage-bundle.wxs`, WixStandardBootstrapperApplication with the
license page and logo). It is per-user only (`PerMachine="no"`, `asInvoker` manifest, so no UAC). It
embeds a per-user-only build of the MSI (`PkgScope=perUser`, same MSI `UpgradeCode`) because Burn needs
a fixed scope, and passes `ARPSYSTEMCOMPONENT=1` so only the bundle's row shows in Settings > Apps.
The bundle's own `UpgradeCode` is `5D0E7B3A-2C41-4F6B-9A18-C3E7F0A4B6D2`; never change it.

The standalone `KernPage-<version>-x64.msi` (dual-scope) is still produced; the launcher's update feed lists it
for installs made from the MSI. **An install made by Setup.exe is updated with Setup.exe, never the plain MSI.**
Running the plain MSI would replace the bundle's embedded MSI (different ProductCode, `MajorUpgrade`) and
leave a Settings > Apps row whose uninstall no longer removes anything. So the feed lists, for every
release, both files: `kind: "msi"` and `kind: "setup"` (the version's `KernPage-<version>-Setup.exe`).
The launcher decides which one it may use (`Release::file_for` in `crates/kernpage-launcher/src/feed.rs`):

| Installed by | Update / "Install an older version" downloads | Runs |
| --- | --- | --- |
| `Setup.exe` (the bundle's HKCU uninstall entry, `BundleUpgradeCode` = the UpgradeCode above, exists) | the release's `setup` file | `Setup.exe /quiet /norestart /log <file>` (the MSI inside remembers the desktop-shortcut choices) |
| the MSI (per-user or all-users) | the release's `msi` file | `msiexec /i <msi> /qn /norestart ...` (UAC for all-users) |

A release that has no `setup` file is not installable on a Setup.exe install (the launcher says "The update list
has no download for this computer"); one with only a `setup` file is not installable on an MSI install. Files
are listed MSI first, so a launcher from before this change still takes the MSI. Setup.exe is per-user only, so
an all-users install always uses the MSI. The helper itself refuses to run a plain MSI when the bundle entry
exists, and refuses a `setup` file on an all-users install.

### Update keys, feeds and verification

- **Feed**: `index.json` plus `index.json.minisig` at `FEED_URL` (`crates/kernpage-launcher/src/feed.rs`): the assets of
  the newest GitHub release (`https://github.com/jmitchell238/kernpage/releases/latest/download/index.json`);
  set the `KERNPAGE_FEED_URL` environment variable at build time to use another. Releases and the signing
  tool are described in `docs/RELEASING.md`. Schema 1; each release lists `files`, each with
  `os`, `arch`, `kind` (`msi`, `setup`, `zip`...), `url` (HTTPS), `size`, `sha256` and `sig` (a minisign signature
  of the file's bytes). The index and every file are checked before use: signature, SHA-256, exact size
  (download capped at the listed size), HTTPS, expiry and a serial that never goes backwards.
- **Keys**: `PRODUCTION_PUBLIC_KEYS` in `feed.rs` is a list of bare minisign public keys. A feed or file verifies
  when ANY listed key signed it. The list holds the primary key (signs releases in CI) and an offline backup key; to rotate, release a launcher that
  lists the new key beside the old one, wait for installs to move, then sign with the new key (see
  `docs/RELEASING.md`). Secret keys live outside the repository.
- **Test feeds**: the TEST-ONLY key in `crates/kernpage-launcher/tests/fixtures`, `--feed-url`, `--update-now` and
  plain-http loopback links exist only in debug builds and builds made with `--features dev-feed`
  (`feed::DEV_FEEDS`). A normal `cargo build --release` has none of them: it trusts only
  `PRODUCTION_PUBLIC_KEYS`, and `check-installer.ps1` checks that. The local test feed is
  `cargo run -p kernpage-launcher --example fake-feed -- --app-file <msi> --app-version <v> --app-setup <Setup.exe>`.
- **Verified twice**: the launcher verifies a download and writes `<file>.verify.json` (size, SHA-256, signature)
  beside it. The update helper (`--apply-update <file>`) accepts only a file inside
  `%LOCALAPPDATA%\KernPage\updates`, opens it so nobody can write to, rename or delete it
  (`FILE_SHARE_READ`), verifies size, SHA-256 and signature on that open handle, and keeps the handle until the
  installer exits. Anything that does not verify is refused ("Nothing was changed").

## For builders

One-time tooling (per-user, no admin):

```powershell
dotnet tool install --global wix --version 5.0.2
wix extension add -g WixToolset.UI.wixext/5.0.2
wix extension add -g WixToolset.Util.wixext/5.0.2
wix extension add -g WixToolset.BootstrapperApplications.wixext/5.0.2
```

Build (from anywhere):

```powershell
.\packaging\windows\package.ps1            # cargo build --release, then the MSI and Setup.exe
.\packaging\windows\package.ps1 -SkipBuild # reuse target-win\release\kernpage.exe
.\packaging\windows\package.ps1 -CustomArt # navy banner/dialog bitmaps from assets\ (default is WiX's stock art)
.\packaging\windows\package.ps1 -SkipBuild -OverrideVersion 9.0.1 # update tests only: another version, Cargo.toml untouched
```

Output: `target-win\installer\KernPage-<version>-x64.msi` and `KernPage-<version>-Setup.exe`. The version is read from the
workspace `Cargo.toml` (numeric X.Y.Z). With `-OverrideVersion`, the build (not `-SkipBuild`) also gets that version in the
`KERNPAGE_VERSION` environment variable, so the installed About box, `--version` and the Launcher's update check all
report the installer's version; the variable is removed again after `cargo build`. A plain `cargo build` without it
reports the `Cargo.toml` version.

Verify the result without installing anything:

```powershell
.\packaging\windows\check-installer.ps1    # read-only queries of the MSI tables and the Setup.exe
```

Optional signing (nothing is signed by default):

```powershell
.\packaging\windows\sign.ps1 -File <msi> -CertThumbprint <sha1>
```

### Files

| File | Purpose |
| --- | --- |
| `kernpage.wxs` | Product definition: scope, upgrade, shortcut, `.pub` Open-with, ARP entries |
| `kernpage-bundle.wxs` | Setup.exe Burn bundle: icon, per-user, hides the MSI's ARP row, desktop-shortcut variables |
| `bundle-theme.xml`, `bundle-theme.wxl` | Setup.exe window layout and strings (stock RtfTheme plus the two shortcut checkboxes) |
| `kernpage.en-us.wxl` | User-visible installer strings |
| `package.ps1` | Builds the exes (editor and launcher) and the MSI; makes the license page from `LICENSE-MIT` and `LICENSE-APACHE` |
| `check-installer.ps1` | Read-only MSI, Setup.exe and release-launcher checks |
| `sign.ps1` | Signing helper that needs an explicit certificate |
| `assets/` | `kernpage.ico`, `banner.bmp` (493x58), `dialog.bmp` (493x312); regenerate with `make_art.py` |

The `UpgradeCode` in `kernpage.wxs` must never change, or upgrades will stop replacing old
installs.

The GUIDs never change: the MSI `UpgradeCode` (`7A952D88-8013-4CAC-85AF-3A3A1722E1DF`), the bundle
`UpgradeCode` (`5D0E7B3A-2C41-4F6B-9A18-C3E7F0A4B6D2`) and every component GUID. With them unchanged, a newer
installer upgrades an installed copy in place (`MajorUpgrade` replaces the old files, shortcuts and folder).
