# Releasing KernPage

A release is a git tag `vX.Y.Z`. Pushing the tag runs `.github/workflows/release.yml`, which builds,
tests, packages, signs the update feed and publishes a GitHub release. No server is involved: the
launcher's update feed is two assets of the newest release.

## Cut a release

1. Make sure `main` is what you want to ship and CI is green locally. Run the full sample set too: set `KERNPAGE_EXTRA_SAMPLES` to the folder that holds the extra `.pub` samples (it mirrors `samples/`; see `samples/SOURCES.md`) and run `cargo test --workspace`. Without it, tests that need an extra sample print `skipped: ...` and pass, so a plain run does not cover them. Check that no `skipped:` line appears (`cargo test --workspace -- --nocapture`).
1a. Local installer builds need `cargo install cargo-about --locked --features cli` (package.ps1 generates `THIRD_PARTY_LICENSES.html` with it).
2. Bump `[workspace.package] version` in `Cargo.toml` (numeric `X.Y.Z` only; no pre-release tags) and commit.
3. Write `docs/release-notes/X.Y.Z.md` (create the folder for the first release; shown in the launcher's "What's new"; also the GitHub release text):

   ```
   ## Added
   - ...
   ## Fixed
   - ...
   ## Changed
   - ...
   ```

   Commit it. Push `main`.
4. Tag that commit and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
   The workflow refuses a tag that does not match the `Cargo.toml` version, is not on `main`, or has no notes file.
5. The `build` job runs the full test suite, `package.ps1` and `check-installer.ps1`. The `publish` job uses
   the `release` environment (the signing key lives there as the secret `KERNPAGE_SIGNING_KEY`). When the
   repository can use required reviewers (public repositories, or a paid plan), approve the run in the
   Actions tab. It then signs `index.json` / `index.json.minisig`, creates the release as a draft with
   `KernPage-X.Y.Z-x64.msi`, `KernPage-X.Y.Z-Setup.exe`, the index and its signature, and
   publishes it as "latest".
6. Check: `https://github.com/jmitchell238/kernpage/releases/latest/download/index.json` downloads, and
   a launcher offers the new version.

**Dry run (no publishing):** Actions > release > Run workflow. It builds and tests, then signs the feed with a
throwaway key generated on the runner (the real key is never used, and the `release` environment only allows
`v*` tags anyway), and uploads everything as the `release-build` artifact. Run one before the first real release
to prove the toolchain setup (GNU Rust + MSYS2 gcc + WiX 5.0.2) works on the runner.

## How the feed works

- `FEED_URL` in `crates/kernpage-launcher/src/feed.rs` is
  `https://github.com/jmitchell238/kernpage/releases/latest/download/index.json`; the signature is
  fetched from `.../index.json.minisig`. GitHub redirects both to its HTTPS asset hosts; the launcher follows
  redirects (HTTPS only, enforced for redirects too) and does not trust the host: the minisign signature, the
  SHA-256, the exact size and the per-file signature are what is trusted.
- Only a full release (not a draft, not a pre-release) counts as "latest". Use plain `X.Y.Z` tags.
- Each release's index lists this version and up to 10 older ones (taken from the previous index, which must
  have been signed by a key the launcher trusts), so "Install an older version" keeps working. Older versions'
  file links keep pointing at their own release's assets.
- **Serial** (anti-rollback): the Unix time of signing (and always higher than the previous serial).
- **Expiry**: each index is valid for 730 days (`--expires-days`). The launcher refuses an expired index, and
  a static asset cannot refresh itself, so a feed with a short expiry would stop offering updates (and
  "go back") after a quiet period. Two years keeps the feed working through long gaps while still bounding how
  long a stolen old index could be replayed to someone who has never seen a newer one (anyone who has seen a
  newer serial rejects it regardless). Trade-off: a longer window means weaker freshness protection, a shorter
  one means a release is needed (or a re-sign) before it elapses. If more than ~18 months pass without a
  release, cut a point release or re-sign: run the workflow steps by hand with `kernpage-release sign-feed`
  (below) using the same installers.
- Between a release being published and the two index assets being visible everywhere there can be a few
  seconds where a check fails verification ("could not verify"); the next check succeeds.

## The signing tool

`crates/kernpage-release` (not shipped in any installer):

```
cargo run --release -p kernpage-release -- keygen --out-dir <dir> --name <name> [--password-file <file>]
cargo run --release -p kernpage-release -- sign-feed --version X.Y.Z --msi <msi> --setup <Setup.exe> `
    --notes docs/release-notes/X.Y.Z.md --out-dir signed [--previous index.json] `
    --launcher-source crates/kernpage-launcher/src/feed.rs
```

The secret key comes from the environment variable `KERNPAGE_SIGNING_KEY` (the key file's text) or `--key-file`;
a password, if the key has one, from `KERNPAGE_SIGNING_KEY_PASSWORD` or `--password-file`. `--previous` needs
`index.json.minisig` beside it. `--launcher-source` makes the tool refuse a key the launcher does not list.
Secret material is never printed. The launcher crate has round-trip tests (`feed.rs`: "release tool output
verifies with the launcher's own code").

## Keys

- Two minisign keys. Public halves are in `PRODUCTION_PUBLIC_KEYS` (`feed.rs`): primary first, backup second;
  the launcher accepts a feed or file signed by either.
- **Primary** (`2AE2B2F3B1D1F1D0`): signs every release, in CI. It has no password: CI cannot type one, and a
  password stored beside the key in the same GitHub environment would add nothing. It is protected by being an
  environment secret of `release` only, usable only by `v*` tags, and (when available) by the reviewer gate.
- **Backup** (`43F3DD9F4C08A821`): password-protected, never in GitHub. Kept offline (USB + password manager).
- The secret key files are kept outside this repository, by the maintainer. Nothing secret belongs in this repository.

### Rotating the primary key (planned, nothing wrong)

1. `kernpage-release keygen --out-dir <private dir> --name primary2`.
2. Release a launcher that lists the backup and `primary2` (and drops the old primary), still signed with the OLD
   primary so installed launchers accept it.
3. Wait until most installs have updated, then replace the `KERNPAGE_SIGNING_KEY` environment secret
   (`gh secret set KERNPAGE_SIGNING_KEY --env release < primary2.key`) and sign from then on with `primary2`.
4. Generate a new offline backup the same way when the old backup is used up (list it in a launcher release).

### If the primary key leaks (or you suspect it)

A leaked key alone cannot push a malicious update: the launcher only reads the feed from your GitHub release
URL and downloads over HTTPS, so an attacker would also need write access to the repository (or to break TLS).
Still, act at once:

1. Delete the secret: `gh secret delete KERNPAGE_SIGNING_KEY --env release --repo jmitchell238/kernpage`, and
   check the repository's collaborators, tokens and recent releases.
2. On a trusted machine, sign a release with the BACKUP key (installed launchers trust it): set
   `KERNPAGE_SIGNING_KEY` to the backup key text and `KERNPAGE_SIGNING_KEY_PASSWORD` to its password and run
   `sign-feed` (use `--previous` of the last good index). This release's launcher lists the backup and a NEW key and
   no longer lists the leaked one. Upload the result by hand (`gh release create`).
3. Generate the new primary, set the new secret, and write a new backup plan. Installs that never update keep
   trusting the leaked key, which is why step 1 matters: protect the feed location, not just the key.
4. If a malicious feed was published, delete that release and publish a higher-serial feed signed by the backup key;
   the `blocklist` of the index can withdraw bad versions.

## GitHub settings in place

- Environment `release`: only tags matching `v*` may deploy; secret `KERNPAGE_SIGNING_KEY`.
- Required reviewers and tag rulesets need a public repository or a paid plan; configure them when the repository
  goes public: environment `release` > required reviewer = owner, and a ruleset restricting creation of `v*` tags
  to repository admins.
- Workflow: no `pull_request_target`; default permissions none; `contents: read` for the build job and
  `contents: write` only for the publish job; every action pinned by commit SHA. To update an action: look up the
  new tag's commit with `gh api repos/<owner>/<action>/git/ref/tags/<tag>` (dereference annotated tags) and edit the pin.
