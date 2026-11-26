//! `kernpage-release keygen | sign-feed` (see docs/RELEASING.md). Never prints secret material.

use kernpage_release::*;
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!(
        "kernpage-release keygen --out-dir DIR --name NAME [--password-file FILE]
  Writes DIR/NAME.key (secret; unencrypted unless --password-file) and DIR/NAME.pub. Refuses to overwrite.
kernpage-release sign-feed --version X.Y.Z --msi FILE --setup FILE --notes FILE --out-dir DIR
    [--previous index.json (needs index.json.minisig beside it)] [--released YYYY-MM-DD]
    [--repo owner/name] [--base-url URL] [--expires-days N] [--keep N]
    [--launcher-source crates/kernpage-launcher/src/feed.rs]
  Secret key: env KERNPAGE_SIGNING_KEY (the key file's text) or --key-file FILE;
  password (only if the key is encrypted): env KERNPAGE_SIGNING_KEY_PASSWORD or --password-file FILE."
    );
    std::process::exit(2)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { usage() };
    let get = |name: &str| -> Option<String> { args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned() };
    let need = |name: &str| get(name).ok_or_else(|| format!("missing {name}"));
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("cannot read {p}: {e}"));
    match cmd.as_str() {
        "keygen" => {
            let dir = PathBuf::from(need("--out-dir")?);
            let name = need("--name")?;
            let password = get("--password-file").map(|p| read(&p).map(|s| s.trim().to_owned())).transpose()?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let (key_path, pub_path) = (dir.join(format!("{name}.key")), dir.join(format!("{name}.pub")));
            if key_path.exists() || pub_path.exists() {
                return Err(format!("{} or {} already exists; refusing to overwrite", key_path.display(), pub_path.display()));
            }
            let (sk, pk, b64, id) = generate(password.as_deref(), &name)?;
            // create_new: never follows or overwrites an existing file or link at these names
            // (no check-then-write race; security audit PW-SEC-011).
            let write_new = |p: &PathBuf, text: String| -> Result<()> {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(p).map_err(|e| format!("cannot create {}: {e}", p.display()))?;
                f.write_all(text.as_bytes()).map_err(|e| e.to_string())
            };
            write_new(&key_path, sk)?;
            write_new(&pub_path, pk)?;
            println!("{name}: key id {id}, encrypted: {}", password.is_some());
            println!("{name}: public key {b64}");
            Ok(())
        }
        "sign-feed" => {
            let key_text = match std::env::var("KERNPAGE_SIGNING_KEY") {
                Ok(t) if !t.trim().is_empty() => t,
                _ => read(&get("--key-file").ok_or("no secret key: set KERNPAGE_SIGNING_KEY or pass --key-file")?)?,
            };
            let password = match std::env::var("KERNPAGE_SIGNING_KEY_PASSWORD") {
                Ok(p) if !p.is_empty() => Some(p),
                _ => get("--password-file").map(|p| read(&p).map(|s| s.trim().to_owned())).transpose()?,
            };
            let sk = load_secret(&key_text, password.as_deref())?;
            let pk = public_of(&sk)?;
            let mut trusted = vec![pk.clone()];
            if let Some(src) = get("--launcher-source") {
                let listed = launcher_trusted_keys(&read(&src)?)?;
                if !listed.contains(&pk.to_base64()) {
                    return Err(format!("the signing key ({}) is not in the launcher's PRODUCTION_PUBLIC_KEYS; installed launchers would reject this feed", key_id(&pk)));
                }
                trusted = listed.iter().map(|k| minisign::PublicKey::from_base64(k).map_err(|e| e.to_string())).collect::<Result<_>>()?;
            }
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
            let version = need("--version")?;
            let repo = get("--repo").unwrap_or_else(|| DEFAULT_REPO.to_owned());
            let previous = match get("--previous") {
                Some(p) => {
                    let idx = std::fs::read(&p).map_err(|e| format!("cannot read {p}: {e}"))?;
                    Some(verify_previous(&idx, &read(&format!("{p}.minisig"))?, &trusted)?)
                }
                None => None,
            };
            let opts = BuildOpts {
                files: vec![FileSpec { kind: "msi".into(), path: need("--msi")?.into() }, FileSpec { kind: "setup".into(), path: need("--setup")?.into() }],
                base_url: get("--base-url").unwrap_or_else(|| format!("https://github.com/{repo}/releases/download/v{version}")),
                notes: read(&need("--notes")?)?,
                released: get("--released").unwrap_or_else(|| date_of(now)),
                expires_days: get("--expires-days").map(|s| s.parse().map_err(|_| "bad --expires-days".to_owned())).transpose()?.unwrap_or(DEFAULT_EXPIRES_DAYS),
                keep: get("--keep").map(|s| s.parse().map_err(|_| "bad --keep".to_owned())).transpose()?.unwrap_or(DEFAULT_KEEP),
                version,
                now_unix: now,
            };
            let signed = build_and_sign(&opts, &sk, previous.as_ref())?;
            let out = PathBuf::from(need("--out-dir")?);
            write_outputs(&signed, &out)?;
            println!("signed index serial {} with key {} -> {}", signed.serial, key_id(&pk), out.display());
            Ok(())
        }
        _ => usage(),
    }
}
