# Launcher test fixtures

These files are used only by the tests of `kernpage-launcher`.

- `test-signing.key` and `test-signing.pub`: a minisign key pair for signing fake update feeds in tests.
- `test-other.key` and `test-other.pub`: a second pair, used to check that a signature from an unknown key is refused.

The two `.key` files are secret keys, but they are test-only. They were created without a password and exist only so the tests can sign sample data. They protect nothing.

Release builds never trust them. A release build trusts only the public keys listed in `PRODUCTION_PUBLIC_KEYS` in `src/feed.rs`. The test public key is compiled in only for debug builds, test builds and builds made with the `dev-feed` cargo feature, and even then only for loopback feed addresses (see `trusted_keys_for_url` in `src/feed.rs`). The real release signing keys are kept outside this repository.
