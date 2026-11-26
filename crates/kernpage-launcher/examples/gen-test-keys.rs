//! Writes the TEST-ONLY signing keypair used by the unit tests and the local fake feed to
//! `tests/fixtures/`. The secret key is deliberately unencrypted and public: it must never be
//! used for anything real. Run from the crate directory: `cargo run -p kernpage-launcher
//! --example gen-test-keys`. Also prints a throwaway public key for the production placeholder.

use minisign::KeyPair;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::fs::create_dir_all(&dir)?;
    let kp = KeyPair::generate_encrypted_keypair(Some(String::new()))?;
    std::fs::write(dir.join("test-signing.pub"), kp.pk.to_box()?.to_string())?;
    std::fs::write(dir.join("test-signing.key"), kp.sk.to_box(None)?.to_string())?;
    let other = KeyPair::generate_encrypted_keypair(Some(String::new()))?;
    std::fs::write(dir.join("test-other.pub"), other.pk.to_box()?.to_string())?;
    std::fs::write(dir.join("test-other.key"), other.sk.to_box(None)?.to_string())?;
    let throwaway = KeyPair::generate_encrypted_keypair(Some(String::new()))?;
    println!("throwaway public key (secret discarded): {}", throwaway.pk.to_base64());
    println!("wrote test keys to {}", dir.display());
    Ok(())
}
