//! Fetching the signed index: blocking HTTP (ureq + rustls), ETag revalidation, a verified
//! on-disk cache for offline use, and the Windows system proxy.
//!
//! What is requested, exactly: `GET <feed url>` (with `If-None-Match` when a cached copy
//! exists) and `GET <feed url>.minisig`. Nothing else is sent: no identifiers, just a fixed
//! `User-Agent` of `KernPageLauncher/<version>`.

use crate::feed::{self, Feed, FeedError};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_BODY: u64 = 4 * 1024 * 1024;

pub struct Fetch {
    pub url: String,
    pub cache_dir: PathBuf,
    pub timeout: Duration,
    /// Highest serial seen before.
    pub last_serial: u64,
}

#[derive(Debug)]
pub struct Fetched {
    pub feed: Feed,
    /// Unix time of the (re)validation.
    pub checked_at: i64,
}

fn user_agent() -> String {
    format!("KernPageLauncher/{}", crate::app::app_version())
}

fn agent(url: &str, timeout: Duration) -> ureq::Agent {
    build_agent(url, Some(timeout))
}

/// The agent for large downloads: no overall time limit (a 40 MB file on a slow line is
/// fine), but a stalled connection still times out, and no `Accept-Encoding` is sent so a
/// `Range` resume always means raw file bytes.
pub fn download_agent(url: &str) -> ureq::Agent {
    build_agent(url, None)
}

fn build_agent(url: &str, timeout: Option<Duration>) -> ureq::Agent {
    let mut b = ureq::Agent::config_builder().http_status_as_error(false).user_agent(user_agent());
    b = match timeout {
        Some(t) => b.timeout_global(Some(t)),
        None => b
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_per_read(Some(Duration::from_secs(30)))
            .accept_encoding(""),
    };
    // ureq reads HTTP(S)_PROXY / ALL_PROXY by default; add the Windows system proxy when no
    // environment proxy is set. Never proxy loopback (the local test feed).
    if feed::is_loopback_url(url) {
        b = b.proxy(None);
    } else {
        // Real feeds and downloads are HTTPS only, redirects included (GitHub redirects release
        // assets to its HTTPS asset hosts). Loopback test feeds stay plain http.
        b = b.https_only(true);
        if std::env::var_os("HTTPS_PROXY").is_none()
            && std::env::var_os("https_proxy").is_none()
            && std::env::var_os("ALL_PROXY").is_none()
            && std::env::var_os("all_proxy").is_none()
        {
            if let Some(p) = system_proxy_url(url).and_then(|u| ureq::Proxy::new(&u).ok()) {
                b = b.proxy(Some(p));
            }
        }
    }
    b.build().into()
}

struct Reply {
    status: u16,
    etag: Option<String>,
    body: Vec<u8>,
}

fn get(agent: &ureq::Agent, url: &str, if_none_match: Option<&str>) -> Result<Reply, FeedError> {
    let mut req = agent.get(url);
    if let Some(t) = if_none_match {
        req = req.header("If-None-Match", t);
    }
    let resp = req.call().map_err(|e| FeedError::Unreachable(short_error(&e)))?;
    let status = resp.status().as_u16();
    let etag = resp.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_owned);
    let body = if status == 200 {
        resp.into_body()
            .with_config()
            .limit(MAX_BODY)
            .read_to_vec()
            .map_err(|e| FeedError::Unreachable(short_error(&e)))?
    } else {
        Vec::new()
    };
    Ok(Reply { status, etag, body })
}

pub fn short_error(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Timeout(_) => "the request timed out".to_owned(),
        ureq::Error::HostNotFound => "the server name was not found".to_owned(),
        ureq::Error::ConnectionFailed => "connection failed".to_owned(),
        other => other.to_string(),
    }
}

/// One update check: fetch, verify, cache. Honours the cached ETag.
pub fn check(f: &Fetch, now: i64) -> Result<Fetched, FeedError> {
    let key = feed::trusted_keys_for_url(&f.url)?;
    let agent = agent(&f.url, f.timeout);
    let cached = Cache::new(&f.cache_dir).load_raw();
    let etag = cached.as_ref().and_then(|c| c.etag.clone());

    let r = get(&agent, &f.url, etag.as_deref().filter(|_| cached.is_some()))?;
    match r.status {
        304 => {
            if let Some(c) = cached {
                let feed = feed::parse_verified(&c.index, &c.sig, &key, now, f.last_serial)?;
                Cache::new(&f.cache_dir).write_meta(c.etag.as_deref(), now);
                return Ok(Fetched { feed, checked_at: now });
            }
            // 304 without a cache cannot happen unless the server is confused.
            Err(FeedError::Http(304))
        }
        200 => {
            let sig_url = format!("{}.minisig", f.url);
            let s = get(&agent, &sig_url, None)?;
            if s.status != 200 {
                return Err(FeedError::Http(s.status));
            }
            let sig = String::from_utf8_lossy(&s.body).into_owned();
            let feed = feed::parse_verified(&r.body, &sig, &key, now, f.last_serial)?;
            Cache::new(&f.cache_dir).store(&r.body, &sig, r.etag.as_deref(), now);
            Ok(Fetched { feed, checked_at: now })
        }
        other => Err(FeedError::Http(other)),
    }
}

// ---- verified cache --------------------------------------------------------------------------------

pub struct Cache {
    dir: PathBuf,
}

pub struct CachedRaw {
    pub index: Vec<u8>,
    pub sig: String,
    pub etag: Option<String>,
    pub checked_at: i64,
}

impl Cache {
    pub fn new(dir: &Path) -> Self {
        Cache { dir: dir.to_path_buf() }
    }

    pub fn load_raw(&self) -> Option<CachedRaw> {
        let index = std::fs::read(self.dir.join("index.json")).ok()?;
        let sig = std::fs::read_to_string(self.dir.join("index.json.minisig")).ok()?;
        let meta = std::fs::read_to_string(self.dir.join("meta.txt")).unwrap_or_default();
        let mut lines = meta.lines();
        let etag = lines.next().filter(|l| !l.is_empty()).map(str::to_owned);
        let checked_at = lines.next().and_then(|l| l.parse().ok()).unwrap_or(0);
        Some(CachedRaw { index, sig, etag, checked_at })
    }

    /// The cached index if it still verifies, is unexpired and not older than `last_serial`.
    /// No network access.
    pub fn load_verified(&self, url: &str, now: i64, last_serial: u64) -> Option<Fetched> {
        let raw = self.load_raw()?;
        let key = feed::trusted_keys_for_url(url).ok()?;
        let feed = feed::parse_verified(&raw.index, &raw.sig, &key, now, last_serial).ok()?;
        Some(Fetched { feed, checked_at: raw.checked_at })
    }

    fn store(&self, index: &[u8], sig: &str, etag: Option<&str>, now: i64) {
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        let _ = std::fs::write(self.dir.join("index.json"), index);
        let _ = std::fs::write(self.dir.join("index.json.minisig"), sig);
        self.write_meta(etag, now);
    }

    fn write_meta(&self, etag: Option<&str>, now: i64) {
        let _ = std::fs::write(self.dir.join("meta.txt"), format!("{}\n{}\n", etag.unwrap_or(""), now));
    }
}

// ---- Windows system proxy ----------------------------------------------------------------------------

/// Pick the proxy for `url` out of a Windows `ProxyServer` value: either `host:port` or
/// `http=host:port;https=host:port;...`. Returns a URL ureq understands.
pub fn parse_proxy_server(value: &str, url: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let scheme = if url.starts_with("https://") { "https" } else { "http" };
    let pick = if value.contains('=') {
        value
            .split(';')
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| k.trim() == scheme)
            .map(|(_, v)| v.trim())?
    } else {
        value
    };
    if pick.contains("://") {
        Some(pick.to_owned())
    } else {
        Some(format!("http://{pick}"))
    }
}

#[cfg(windows)]
fn system_proxy_url(url: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let query = |name: &str| -> Option<String> {
        let out = std::process::Command::new(kernpage_core::sysexe::system32("reg.exe"))
            .args(["query", KEY, "/v", name])
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let line = text.lines().find(|l| l.trim_start().starts_with(name))?;
        let mut parts = line.split_whitespace();
        parts.next()?;
        parts.next()?;
        Some(parts.collect::<Vec<_>>().join(" "))
    };
    let enabled = query("ProxyEnable")?;
    if !(enabled == "0x1" || enabled == "1") {
        return None;
    }
    parse_proxy_server(&query("ProxyServer")?, url)
}

#[cfg(not(windows))]
fn system_proxy_url(_url: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devfeed::*;
    use crate::devserver::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pw-net-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn serve(opts: &SampleOpts) -> (Server, Files) {
        let files: Files = Arc::new(Mutex::new(HashMap::new()));
        put(&files, opts);
        let server = Server::start(0, static_handler(files.clone())).unwrap();
        (server, files)
    }

    fn put(files: &Files, opts: &SampleOpts) {
        let b = sample_bytes(opts);
        let s = sign_test(&b);
        let mut f = files.lock().unwrap();
        f.insert("/index.json".into(), b);
        f.insert("/index.json.minisig".into(), s.into_bytes());
    }

    fn fetch(server: &Server, dir: &Path, last_serial: u64) -> Fetch {
        Fetch { url: server.url("/index.json"), cache_dir: dir.to_path_buf(), timeout: Duration::from_secs(5), last_serial }
    }

    #[test]
    fn fetches_verifies_and_caches_then_revalidates_with_etag() {
        let dir = tmp("etag");
        let (server, files) = serve(&SampleOpts::new(3, "http://127.0.0.1:1"));
        let now = feed::now_unix();
        let got = check(&fetch(&server, &dir, 0), now).unwrap();
        assert_eq!(got.feed.serial, 3);
        assert_eq!(server.hits(), 2); // index + signature
        // Second check: 304 for the index, signature not fetched again.
        let again = check(&fetch(&server, &dir, 3), now + 10).unwrap();
        assert_eq!(again.checked_at, now + 10);
        assert_eq!(server.hits(), 3);
        // The cache works with the server gone.
        let cached = Cache::new(&dir).load_verified(&server.url("/index.json"), now, 3).unwrap();
        assert_eq!(cached.feed.serial, 3);
        // A new index replaces it.
        put(&files, &SampleOpts::new(4, "http://127.0.0.1:1"));
        assert_eq!(check(&fetch(&server, &dir, 3), now).unwrap().feed.serial, 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_bad_feeds_and_does_not_cache_them() {
        let dir = tmp("bad");
        let now = feed::now_unix();
        let (server, files) = serve(&SampleOpts::new(3, "http://127.0.0.1:1"));
        // tampered
        files.lock().unwrap().get_mut("/index.json").unwrap().push(b' ');
        assert_eq!(check(&fetch(&server, &dir, 0), now).unwrap_err(), FeedError::BadSignature);
        // wrong key
        let b = sample_bytes(&SampleOpts::new(3, "x"));
        let sig = sign_with(OTHER_SECRET, &b);
        {
            let mut f = files.lock().unwrap();
            f.insert("/index.json".into(), b);
            f.insert("/index.json.minisig".into(), sig.into_bytes());
        }
        assert_eq!(check(&fetch(&server, &dir, 0), now).unwrap_err(), FeedError::WrongKey);
        // expired
        let mut o = SampleOpts::new(3, "x");
        o.expires = "2020-01-01T00:00:00Z".into();
        put(&files, &o);
        assert!(matches!(check(&fetch(&server, &dir, 0), now).unwrap_err(), FeedError::Expired { .. }));
        // older serial
        put(&files, &SampleOpts::new(3, "x"));
        assert!(matches!(check(&fetch(&server, &dir, 7), now).unwrap_err(), FeedError::Rollback { .. }));
        assert!(Cache::new(&dir).load_raw().is_none());
        // missing file / unreachable
        files.lock().unwrap().remove("/index.json");
        assert_eq!(check(&fetch(&server, &dir, 0), now).unwrap_err(), FeedError::Http(404));
        let dead = {
            let s = Server::start(0, static_handler(files.clone())).unwrap();
            s.url("/index.json")
        };
        let f = Fetch { url: dead, cache_dir: dir.clone(), timeout: Duration::from_secs(2), last_serial: 0 };
        assert!(matches!(check(&f, now).unwrap_err(), FeedError::Unreachable(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn proxy_server_values() {
        assert_eq!(parse_proxy_server("proxy.corp:8080", "https://x/y").unwrap(), "http://proxy.corp:8080");
        assert_eq!(
            parse_proxy_server("http=a:1;https=b:2;ftp=c:3", "https://x/y").unwrap(),
            "http://b:2"
        );
        assert_eq!(parse_proxy_server("http=a:1;https=b:2", "http://x/y").unwrap(), "http://a:1");
        assert!(parse_proxy_server("ftp=c:3", "https://x/y").is_none());
        assert!(parse_proxy_server("", "https://x/y").is_none());
    }
}
