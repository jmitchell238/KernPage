# KernPage Launcher

The front door for KernPage: shows the installed version, update status, the
add-ons that can be listed, recent files, and settings. It checks the signed update feed and
installs KernPage updates (and older versions) after verifying them twice. Installing add-ons
is not built yet; the button opens a dialog that says so. Design and plan:
`docs/research/launcher-and-addons.md`.

## What the launcher requests, exactly

Only when a check runs: `GET <feed>/index.json` (with `If-None-Match` after the first time)
and `GET <feed>/index.json.minisig`. A fixed `User-Agent` of `KernPageLauncher/<version>`.
No identifiers, no telemetry. **Manual** mode (the default) makes no request until you press
Refresh or Check now; **Automatic** checks at start and every N hours. The last verified index
is cached next to `launcher.toml` so the launcher works offline.

The index is verified with a minisign signature before it is parsed. It is refused when the
signature is bad or made with another key, when it has expired, when its serial is lower than
one already seen, or when it is malformed. A release whose file has a bad SHA-256, no valid
signature or a non-HTTPS link is flagged and never offered.

Keys: `PRODUCTION_PUBLIC_KEYS` in `src/feed.rs` lists the public keys (primary + offline backup); a feed or
download verifies when any one of them signed it, which is how keys are rotated. A TEST-ONLY key in
`tests/fixtures/` is trusted only for loopback feeds, and only in debug / `--features dev-feed` builds: a normal
release build has no test key, no `--feed-url` and no `--update-now`. The production feed URL is `FEED_URL` in
`src/feed.rs`: the `index.json` / `index.json.minisig` assets of the newest GitHub release (override with
`KERNPAGE_FEED_URL` at build time). Releases are signed by `crates/kernpage-release` in CI (`docs/RELEASING.md`).
Feed file kinds, the Setup.exe rule and the
verify-twice install are described in `packaging/windows/INSTALLATION.md`.

## Try it with the local fake feed

```
cargo run -p kernpage-launcher --example fake-feed -- --port 8750
cargo run -p kernpage-launcher -- --feed-url http://127.0.0.1:8750/index.json --check-now
```

The fake feed offers KernPage 0.2.0 (6 features, release notes), a 0.3.0-beta.1 pre-release
and the add-on "Spelling dictionary: English (US)". Options: `--seconds N` (exit by itself),
`--expired`, `--tamper`, `--wrong-key`, `--bad-sha`, `--block "<0.2.0"`, `--serial N`, to see
each refusal message. Regenerate the test keys with `--example gen-test-keys`.
`--feed-url` exists only in debug and `dev-feed` builds. Add `--app-setup <Setup.exe>` (and `--app-older-setup`) to serve the Setup.exe a Setup.exe install updates with.

Launcher flags: `--feed-url`, `--check-now`, `--check-update` (no window: prints `update-available <version>` when a newer KernPage is offered; the editor runs it in the background), `--update-dialog`, `--details`, `--settings`,
`--theme light|dark`, `--screenshot out.png`, `--window-size WxH`.

## Toast notifications

Toasts need an AppUserModelID that only the installer's Start-menu shortcut registers
(`KernPage.Launcher`, `src/notify.rs`). When KernPage is not installed (for example a
development build), the toast is sent under the stock PowerShell identity so it still appears
(labelled "Windows PowerShell"), without action buttons; the What's new / Later / Update choices are in the in-app dialog.

## Tests

`cargo test -p kernpage-launcher`
