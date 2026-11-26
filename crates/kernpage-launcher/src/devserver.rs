//! A tiny single-purpose HTTP/1.1 server on 127.0.0.1 for the tests and the local fake feed
//! (`examples/fake-feed.rs`). Not part of the shipped launcher. Self-contained on purpose: the
//! example includes this file with `#[path]`.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn ok(body: Vec<u8>) -> Self {
        Response { status: 200, headers: Vec::new(), body }
    }
    pub fn status(status: u16) -> Self {
        Response { status, headers: Vec::new(), body: Vec::new() }
    }
}

pub type Handler = Arc<dyn Fn(&Request) -> Response + Send + Sync>;

pub struct Server {
    pub port: u16,
    hits: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// Start on 127.0.0.1; `port` 0 picks a free port.
    pub fn start(port: u16, handler: Handler) -> std::io::Result<Server> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let port = listener.local_addr()?.port();
        let hits = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (h, s) = (hits.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if s.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(conn) = conn else { continue };
                if let Some(req) = read_request(&conn) {
                    h.fetch_add(1, Ordering::SeqCst);
                    let resp = handler(&req);
                    let _ = write_response(conn, &resp);
                }
            }
        });
        Ok(Server { port, hits, stop, thread: Some(thread) })
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }

    /// Requests seen so far.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port)); // wake the accept loop
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read_request(conn: &TcpStream) -> Option<Request> {
    conn.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok()?;
    let mut reader = BufReader::new(conn);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    let mut headers = Vec::new();
    loop {
        let mut l = String::new();
        if reader.read_line(&mut l).ok()? == 0 || l.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            headers.push((k.trim().to_owned(), v.trim().to_owned()));
        }
    }
    Some(Request { method, path, headers })
}

fn write_response(mut conn: TcpStream, r: &Response) -> std::io::Result<()> {
    let reason = match r.status {
        200 => "OK",
        206 => "Partial Content",
        304 => "Not Modified",
        404 => "Not Found",
        416 => "Range Not Satisfiable",
        _ => "Status",
    };
    let mut head = format!("HTTP/1.1 {} {}\r\nConnection: close\r\nContent-Length: {}\r\n", r.status, reason, r.body.len());
    for (k, v) in &r.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    conn.write_all(head.as_bytes())?;
    if r.status != 304 {
        conn.write_all(&r.body)?;
    }
    conn.flush()
}

/// A weak content tag for the ETag header.
pub fn etag_for(body: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    body.hash(&mut h);
    format!("\"{:016x}\"", h.finish())
}

/// Files served from memory by path, with ETag / If-None-Match handling.
pub type Files = Arc<Mutex<HashMap<String, Vec<u8>>>>;

pub fn static_handler(files: Files) -> Handler {
    Arc::new(move |req: &Request| {
        let files = files.lock().unwrap();
        let Some(body) = files.get(req.path.as_str()) else {
            return Response::status(404);
        };
        // "Range: bytes=N-" (what the launcher's resumable download sends).
        if let Some(start) = req.header("Range").and_then(|r| r.strip_prefix("bytes=")).and_then(|r| r.strip_suffix('-')).and_then(|n| n.trim().parse::<usize>().ok()) {
            if start >= body.len() {
                let mut r = Response::status(416);
                r.headers.push(("Content-Range".into(), format!("bytes */{}", body.len())));
                return r;
            }
            let mut r = Response::status(206);
            r.body = body[start..].to_vec();
            r.headers.push(("Content-Range".into(), format!("bytes {start}-{}/{}", body.len() - 1, body.len())));
            return r;
        }
        let tag = etag_for(body);
        let mut r = if req.header("If-None-Match") == Some(tag.as_str()) {
            Response::status(304)
        } else {
            Response::ok(body.clone())
        };
        r.headers.push(("ETag".to_owned(), tag));
        r
    })
}
