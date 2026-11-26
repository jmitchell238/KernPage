//! Local test feed for the KernPage Launcher. Builds a sample index (KernPage 0.2.0 plus one
//! add-on), signs it with the TEST-ONLY key in tests/fixtures, and serves it on 127.0.0.1.
//!
//!   cargo run -p kernpage-launcher --example fake-feed -- [--port 8750] [--seconds N]
//!       [--expired] [--serial N] [--bad-sha] [--block "<0.2.0"] [--tamper] [--wrong-key]
//!       [--app-file <msi> --app-version <ver> [--app-setup <Setup.exe>]]
//!       [--app-older <msi> --app-older-version <ver> [--app-older-setup <Setup.exe>]]
//!
//! `--app-file` serves a REAL installer as the KernPage release `--app-version` (true
//! size and SHA-256, signed with the test key; Range requests are supported). `--app-older`
//! adds one older release the same way, for trying "Install an older version".
//!
//! Then: kernpage-launcher --feed-url http://127.0.0.1:8750/index.json --check-now
//! `--seconds N` makes the server exit by itself after N seconds.

#[path = "../src/devfeed.rs"]
mod devfeed;
#[path = "../src/devserver.rs"]
mod devserver;

use devfeed::*;
use devserver::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn main() {
    let mut port = 8750u16;
    let mut seconds: Option<u64> = None;
    let mut serial = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let (mut expired, mut bad_sha, mut tamper, mut wrong_key) = (false, false, false, false);
    let mut block = None;
    let (mut app_file, mut app_version, mut older_file, mut older_version) = (None::<String>, None::<String>, None::<String>, None::<String>);
    let (mut app_setup, mut older_setup) = (None::<String>, None::<String>);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => port = args.next().and_then(|v| v.parse().ok()).expect("--port N"),
            "--seconds" => seconds = args.next().and_then(|v| v.parse().ok()),
            "--serial" => serial = args.next().and_then(|v| v.parse().ok()).expect("--serial N"),
            "--block" => block = args.next(),
            "--app-file" => app_file = args.next(),
            "--app-version" => app_version = args.next(),
            "--app-setup" => app_setup = args.next(),
            "--app-older-setup" => older_setup = args.next(),
            "--app-older" => older_file = args.next(),
            "--app-older-version" => older_version = args.next(),
            "--expired" => expired = true,
            "--bad-sha" => bad_sha = true,
            "--tamper" => tamper = true,
            "--wrong-key" => wrong_key = true,
            other => panic!("unknown option {other}"),
        }
    }
    let base = format!("http://127.0.0.1:{port}");
    let mut o = SampleOpts::new(serial, &base);
    let real = |what: &str, file: Option<String>, version: Option<String>| -> Option<RealFile> {
        match (file, version) {
            (None, None) => None,
            (Some(f), Some(v)) => Some(RealFile::from_path(std::path::Path::new(&f), &v).unwrap_or_else(|e| panic!("cannot read {f}: {e}"))),
            _ => panic!("{what} needs both its file and its version"),
        }
    };
    let with_setup = |r: Option<RealFile>, setup: Option<String>| {
        r.map(|r| match setup {
            Some(p) => r.with_setup_from_path(std::path::Path::new(&p)).unwrap_or_else(|e| panic!("cannot read {p}: {e}")),
            None => r,
        })
    };
    o.app_real = with_setup(real("--app-file", app_file, app_version), app_setup);
    o.app_older = with_setup(real("--app-older", older_file, older_version), older_setup);
    o.bad_sha = bad_sha;
    o.block_kernpage = block;
    if expired {
        o.expires = "2020-01-01T00:00:00Z".into();
    }
    let mut index = sample_bytes(&o);
    let sig = if wrong_key { sign_with(OTHER_SECRET, &index) } else { sign_test(&index) };
    if tamper {
        index.push(b'\n');
    }
    let files: Files = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut f = files.lock().unwrap();
        f.insert("/index.json".into(), index);
        f.insert("/index.json.minisig".into(), sig.into_bytes());
        for (name, bytes) in served_files(&o) {
            f.insert(format!("/files/{name}"), bytes);
        }
    }
    let server = Server::start(port, static_handler(files)).expect("cannot bind the port");
    println!("fake feed serving {}", server.url("/index.json"));
    println!("feed URL: {}", server.url("/index.json"));
    println!("run: kernpage-launcher --feed-url {} --check-now", server.url("/index.json"));
    if let Some(r) = &o.app_real {
        println!("serving a real installer as KernPage {}", r.version);
    }
    if let Some(r) = &o.app_older {
        println!("serving a real installer as the older release {}", r.version);
    }
    match seconds {
        Some(s) => std::thread::sleep(std::time::Duration::from_secs(s)),
        None => loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        },
    }
}
