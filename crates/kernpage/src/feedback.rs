//! Help > Report a Problem / Suggest an Idea: open a new GitHub issue in the web browser, with the
//! KernPage and Windows version filled in. There is no live support and nothing is sent from the
//! editor; the person reads the text and submits it on GitHub (or closes the tab).

/// The project's GitHub repository (the same address the Launcher's feed uses).
pub const REPO_URL: &str = "https://github.com/jmitchell238/kernpage";

/// Percent-encode `s` for a URL query value (RFC 3986 unreserved characters stay as they are).
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The new-issue page for a bug, with the versions and headings filled in.
pub fn report_url(version: &str, windows: &str) -> String {
    let body = format!(
        "KernPage version: {version}\nWindows: {windows}\n\nWhat happened:\n\nWhat did you expect:\n\nSteps to reproduce:\n"
    );
    format!("{REPO_URL}/issues/new?labels=bug&body={}", url_encode(&body))
}

/// The new-issue page for a suggestion.
pub fn idea_url(version: &str) -> String {
    let body = format!("KernPage version: {version}\n\nWhat would you like KernPage to do?\n\nWhy would it help?\n");
    format!("{REPO_URL}/issues/new?labels=idea&body={}", url_encode(&body))
}

/// The Windows version, e.g. "Windows 10.0 (build 22631)"; "not Windows" elsewhere.
pub fn windows_version() -> String {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct OsVersionInfo {
            size: u32,
            major: u32,
            minor: u32,
            build: u32,
            platform: u32,
            csd: [u16; 128],
        }
        #[link(name = "ntdll")]
        extern "system" {
            fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
        }
        let mut v = OsVersionInfo { size: std::mem::size_of::<OsVersionInfo>() as u32, major: 0, minor: 0, build: 0, platform: 0, csd: [0; 128] };
        // SAFETY: `v` is a valid, correctly sized OSVERSIONINFOW; RtlGetVersion only fills it in.
        if unsafe { RtlGetVersion(&mut v) } == 0 {
            return format!("Windows {}.{} (build {})", v.major, v.minor, v.build);
        }
        "Windows (version unknown)".to_owned()
    }
    #[cfg(not(windows))]
    {
        "not Windows".to_owned()
    }
}

/// Open `url` in the default web browser.
pub(crate) fn open_url(url: &str) {
    #[cfg(test)]
    {
        // Tests never start a browser: the address is recorded instead.
        test_os::OPENED.with(|o| o.borrow_mut().push(url.to_owned()));
    }
    #[cfg(not(test))]
    {
        #[cfg(windows)]
        let mut cmd = {
            let mut c = std::process::Command::new(kernpage_core::sysexe::system32("rundll32.exe"));
            c.args(["url.dll,FileProtocolHandler", url]);
            c
        };
        #[cfg(target_os = "macos")]
        let mut cmd = {
            let mut c = std::process::Command::new("open");
            c.arg(url);
            c
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        let mut cmd = {
            let mut c = std::process::Command::new("xdg-open");
            c.arg(url);
            c
        };
        let _ = cmd.spawn();
    }
}

pub fn report_a_problem() {
    open_url(&report_url(crate::app_version(), &windows_version()));
}

pub fn suggest_an_idea() {
    open_url(&idea_url(crate::app_version()));
}

/// Test seam: the addresses the app asked the system to open.
#[cfg(test)]
pub mod test_os {
    use std::cell::RefCell;
    thread_local! {
        pub(super) static OPENED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    pub fn take_opened() -> Vec<String> {
        OPENED.with(|o| std::mem::take(&mut *o.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encode_keeps_unreserved_and_escapes_the_rest() {
        assert_eq!(url_encode("AZaz09-._~"), "AZaz09-._~");
        assert_eq!(url_encode("a b\nc&d=e/f?g#h%"), "a%20b%0Ac%26d%3De%2Ff%3Fg%23h%25");
        assert_eq!(url_encode("\u{e9}"), "%C3%A9");
    }

    #[test]
    fn the_urls_carry_the_version_and_headings() {
        let r = report_url("1.2.3", "Windows 10.0 (build 1)");
        assert!(r.starts_with("https://github.com/jmitchell238/kernpage/issues/new?labels=bug&body="));
        assert!(r.contains("KernPage%20version%3A%201.2.3%0AWindows%3A%20Windows%2010.0%20%28build%201%29"), "{r}");
        assert!(r.contains("Steps%20to%20reproduce"));
        let i = idea_url("1.2.3");
        assert!(i.starts_with("https://github.com/jmitchell238/kernpage/issues/new?labels=idea&body="));
        assert!(i.contains("1.2.3") && i.contains("Why%20would%20it%20help"));
        assert!(!r.contains(' ') && !i.contains(' ') && !r.contains('\n'));
    }

    #[test]
    fn the_windows_version_is_not_empty() {
        assert!(!windows_version().is_empty());
    }
}
