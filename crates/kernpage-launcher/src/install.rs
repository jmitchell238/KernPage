//! Installing a verified update (an MSI, or Setup.exe on a Setup.exe install).
//!
//! The running launcher cannot replace its own files, so it copies itself to
//! `<updates>\helper\kernpage-update-helper.exe` and starts that copy in "helper mode"
//! (`--apply-update`). The helper waits for the launcher to exit, verifies the downloaded file
//! AGAIN (SHA-256 and signature, from a handle that nobody can write to or swap while the
//! installer runs), runs `msiexec` (with a UAC prompt for a per-machine install) or, when the
//! app was installed by Setup.exe, the new version's `Setup.exe /quiet`, writes
//! `last-install.json` and starts the installed launcher again. The next start of the launcher
//! reads that file and tells the user how it went. Windows Installer rolls a failed install
//! back by itself, so a failure leaves the old version in place.
//!
//! Why Setup.exe for a Setup.exe install: the bundle (Burn) is what Settings > Apps shows and
//! uninstalls. Running the plain MSI instead would replace the bundle's embedded MSI (different
//! ProductCode, MajorUpgrade) and leave a bundle row that no longer removes anything. So the
//! bundle stays the owner: the feed carries a `setup` file next to the `msi` file, and the
//! launcher never offers the MSI on a Setup.exe install (`Release::file_for`).
//!
//! Going back to an older version has one more wrinkle: Burn refuses to run an older Setup.exe
//! over a newer bundle (exit code 1638, nothing changed). Then the helper keeps a copy of the
//! installed bundle (from Package Cache), uninstalls it quietly, runs the older Setup.exe, and if
//! that fails puts the saved copy back (`run_setup`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const LAUNCHER_EXE: &str = "kernpage-launcher.exe";
pub const RESULT_FILE: &str = "last-install.json";
const HELPER_EXE: &str = "kernpage-update-helper.exe";

/// Where this copy of the launcher is installed, which decides how the MSI is run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallScope {
    PerUser,
    PerMachine,
    /// A dev build or a copy run from anywhere else: never installs.
    NotInstalled,
}

/// Arguments of helper mode: `--apply-update <msi> --wait-pid <pid> [--target-version <ver>]`
/// plus `--install-dir <folder>` (the folder the launcher was installed in; the helper runs
/// from a copy, so it cannot find that out itself).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyUpdate {
    pub msi: PathBuf,
    pub wait_pid: u32,
    pub target_version: Option<String>,
    pub install_dir: Option<PathBuf>,
    /// A loopback test feed to keep using after the restart (`--relaunch-feed-url`).
    pub relaunch_feed_url: Option<String>,
}

/// The arguments the helper starts the launcher with afterwards: none in production; a
/// loopback test feed is carried over (and checked right away) so a dev test keeps its feed.
pub fn relaunch_args(feed_url: Option<&str>) -> Vec<String> {
    match feed_url.filter(|u| crate::feed::is_loopback_url(u)) {
        Some(u) => vec!["--feed-url".to_owned(), u.to_owned(), "--check-now".to_owned()],
        None => Vec::new(),
    }
}

/// What the helper tells the next launcher start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallResult {
    pub target_version: String,
    pub ok: bool,
    pub exit_code: Option<u32>,
    pub message: String,
    pub log_path: String,
    /// Seconds since 1970 (UTC).
    pub finished_at: i64,
}

fn folder_is_under(dir: &Path, roots: &[PathBuf]) -> bool {
    let is_app_dir = dir
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("KernPage"));
    is_app_dir
        && dir.parent().is_some_and(|p| {
            let p = p.to_string_lossy().to_lowercase();
            roots.iter().any(|r| r.to_string_lossy().to_lowercase() == p)
        })
}

/// Classify an install folder: `KernPage` directly under one of the roots.
pub fn scope_of_dir(dir: &Path, per_user_roots: &[PathBuf], machine_roots: &[PathBuf]) -> InstallScope {
    if folder_is_under(dir, per_user_roots) {
        InstallScope::PerUser
    } else if folder_is_under(dir, machine_roots) {
        InstallScope::PerMachine
    } else {
        InstallScope::NotInstalled
    }
}

fn env_roots(vars: &[&str], sub: Option<&str>) -> Vec<PathBuf> {
    vars.iter()
        .filter_map(std::env::var_os)
        .map(|v| {
            let p = PathBuf::from(v);
            match sub {
                Some(s) => p.join(s),
                None => p,
            }
        })
        .collect()
}

/// The scope of an install folder, using the real `%LOCALAPPDATA%\Programs` / Program Files.
pub fn scope_of_install_dir(dir: &Path) -> InstallScope {
    scope_of_dir(dir, &env_roots(&["LOCALAPPDATA"], Some("Programs")), &env_roots(&["ProgramFiles", "ProgramW6432"], None))
}

/// How the running launcher was installed.
pub fn install_scope() -> InstallScope {
    #[cfg(test)]
    if let Some(s) = crate::testhooks::peek(|f| f.scope) {
        return s;
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(scope_of_install_dir))
        .unwrap_or(InstallScope::NotInstalled)
}

/// The `msiexec` arguments for a silent install, or `None` when nothing may be installed.
pub fn msiexec_args(msi: &Path, log: &Path, scope: InstallScope) -> Option<Vec<String>> {
    let property = match scope {
        InstallScope::PerUser => "MSIINSTALLPERUSER=1",
        InstallScope::PerMachine => "ALLUSERS=1",
        InstallScope::NotInstalled => return None,
    };
    Some(vec![
        "/i".to_owned(),
        msi.display().to_string(),
        "/qn".to_owned(),
        "/norestart".to_owned(),
        "/l*v".to_owned(),
        log.display().to_string(),
        property.to_owned(),
    ])
}

/// The arguments of the quiet Setup.exe (Burn bundle) run. Desktop-shortcut choices are not
/// passed: the MSI inside remembers them (HKCU `Software\KernPage`).
pub fn setup_args(log: &Path) -> Vec<String> {
    vec!["/quiet".to_owned(), "/norestart".to_owned(), "/log".to_owned(), log.display().to_string()]
}

/// A file the helper may run is an MSI or a Setup.exe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerKind {
    Msi,
    Setup,
}

impl InstallerKind {
    pub fn of(path: &Path) -> Option<InstallerKind> {
        match path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref() {
            Some("msi") => Some(InstallerKind::Msi),
            Some("exe") => Some(InstallerKind::Setup),
            _ => None,
        }
    }
}

/// No quote, control character or `%` in a path that goes on a command line.
pub fn path_is_plain(path: &Path) -> bool {
    let s = path.to_string_lossy();
    !s.is_empty() && !s.chars().any(|c| c == '"' || c == '%' || c.is_control())
}

/// UpgradeCode of the Setup.exe bundle (packaging/windows/kernpage-bundle.wxs); never changes.
pub const BUNDLE_UPGRADE_CODE: &str = "5D0E7B3A-2C41-4F6B-9A18-C3E7F0A4B6D2";

/// Where the bundle's uninstall entry is looked for (per-user bundle, so HKCU).
#[cfg_attr(not(windows), allow(dead_code))]
const UNINSTALL_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// From `reg query <Uninstall> /s /v BundleUpgradeCode` output: the registry key (`HKEY_...\<id>`)
/// whose `BundleUpgradeCode` value contains the bundle's UpgradeCode.
pub fn bundle_key_from_reg_output(text: &str) -> Option<String> {
    let mut key: Option<&str> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.to_uppercase().starts_with("HKEY_") {
            key = Some(t);
        } else if t.starts_with("BundleUpgradeCode") && t.to_uppercase().contains(BUNDLE_UPGRADE_CODE) {
            return key.map(str::to_owned);
        }
    }
    None
}

/// The seam for "was this installed by Setup.exe": the bundle's uninstall key, if it exists.
pub trait BundleProbe {
    fn bundle_key(&self) -> Option<String>;
}

/// The real current-user registry (through `reg.exe`); finds nothing on other systems.
pub struct RegistryBundleProbe;

impl BundleProbe for RegistryBundleProbe {
    fn bundle_key(&self) -> Option<String> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let out = std::process::Command::new(kernpage_core::sysexe::system32("reg.exe"))
                .args(["query", UNINSTALL_KEY, "/s", "/v", "BundleUpgradeCode"])
                .creation_flags(0x0800_0000)
                .output()
                .ok()?;
            bundle_key_from_reg_output(&String::from_utf8_lossy(&out.stdout))
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

/// From `reg query <bundle key> /v UninstallString` output: the bundle's own exe, the copy Burn
/// keeps in `%LOCALAPPDATA%\Package Cache\{id}\`, which is also what Settings > Apps runs.
pub fn bundle_exe_from_reg_output(text: &str) -> Option<PathBuf> {
    let line = text.lines().find(|l| l.trim_start().starts_with("UninstallString"))?;
    let rest = &line[line.find('"')? + 1..];
    let path = PathBuf::from(&rest[..rest.find('"')?]);
    let low = path.to_string_lossy().to_lowercase();
    (low.ends_with(".exe") && low.contains(r"\package cache\")).then_some(path)
}

/// The Burn exit code for "an older bundle was run over a newer one; nothing was changed"
/// (ERROR_PRODUCT_VERSION, also seen as the HRESULT 0x80070666).
pub fn is_downgrade_blocked(code: u32) -> bool {
    code == 1638 || code == 0x8007_0666
}

/// What `run_setup` needs from the machine (a seam, so the sequence can be tested).
pub trait BundleOps {
    /// Run a program hidden and wait; its exit code.
    fn run(&mut self, program: &Path, args: &[String]) -> Result<u32, String>;
    /// The installed bundle's own exe in Package Cache.
    fn installed_bundle_exe(&mut self) -> Option<PathBuf>;
    /// Copy a file somewhere safe (under the updates folder); the copy's path.
    fn save_copy(&mut self, from: &Path) -> Result<PathBuf, String>;
}

/// Run the new version's Setup.exe quietly over a Setup.exe install; the bundle stays the owner.
///
/// An upgrade (or the same version) just works. Going back to an OLDER version is refused by Burn
/// (exit 1638, nothing changed), so then: save a copy of the installed bundle, uninstall it
/// quietly, run the older Setup.exe, and if that fails run the saved copy to put the version that
/// was installed back. Publications and settings are never touched by any of these steps.
pub fn run_setup(ops: &mut dyn BundleOps, setup: &Path, log: &Path) -> Result<u32, String> {
    let code = ops.run(setup, &setup_args(log))?;
    if !is_downgrade_blocked(code) {
        return Ok(code);
    }
    let Some(current) = ops.installed_bundle_exe() else {
        return Ok(code); // cannot go back safely: report the refusal, nothing was changed
    };
    let saved = ops.save_copy(&current).map_err(|e| format!("Could not go back to the older version safely ({e}). Nothing was changed."))?;
    let uninstall_log = log.with_extension("uninstall.log");
    let removed = ops.run(&current, &["/uninstall".to_owned(), "/quiet".to_owned(), "/norestart".to_owned(), "/log".to_owned(), uninstall_log.display().to_string()])?;
    if removed != 0 && removed != 3010 {
        return Ok(removed); // the current version could not be removed first: it is still installed
    }
    let code = ops.run(setup, &setup_args(log)).unwrap_or(1603);
    if code == 0 || code == 3010 {
        return Ok(code);
    }
    // The older version did not install: put the previous one back.
    let restore_log = log.with_extension("restore.log");
    let restored = ops.run(&saved, &setup_args(&restore_log)).unwrap_or(1603);
    Err(if restored == 0 || restored == 3010 {
        format!("The older version could not be installed (error {code}), so your previous version was put back. Your publications are not changed.")
    } else {
        format!("The older version could not be installed (error {code}) and the previous version could not be put back (error {restored}). Run the KernPage Setup.exe again to reinstall. Your publications are not changed.")
    })
}

/// True when this launcher belongs to an install made by Setup.exe (per-user, with the bundle's
/// Settings > Apps entry present). Decided once per run. Such an install must be updated by
/// running Setup.exe, never the plain MSI.
pub fn installed_via_bundle() -> bool {
    #[cfg(test)]
    if let Some(b) = crate::testhooks::peek(|f| f.bundle) {
        return b;
    }
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| install_scope() == InstallScope::PerUser && RegistryBundleProbe.bundle_key().is_some())
}

/// One command-line string (for `ShellExecuteEx`), quoting what has spaces.
#[cfg_attr(not(windows), allow(dead_code))]
fn join_command_line(args: &[String]) -> String {
    args.iter()
        .map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ")
}

const ERROR_CANCELLED: u32 = 1223;

/// Plain-English meaning of an `msiexec` exit code: `(ok, message)`.
pub fn map_exit_code(code: u32) -> (bool, String) {
    match code {
        0 => (true, "KernPage was updated.".to_owned()),
        // Setup.exe (Burn) reports a version older than the one installed this way.
        1638 | 0x8007_0666 => (false, "A newer version is already installed, so nothing was changed.".to_owned()),
        3010 => (true, "KernPage was updated. Restart Windows to finish the update.".to_owned()),
        1602 | ERROR_CANCELLED => (false, "The update was cancelled. Your current version is still installed.".to_owned()),
        1618 => (
            false,
            "Another installation is already in progress. Finish it, then try again. Your current version is still installed.".to_owned(),
        ),
        n => (
            false,
            format!("The update could not be installed (error {n}). Windows put your current version back, so KernPage still works as before."),
        ),
    }
}

// ---- the result file --------------------------------------------------------------------------------

pub fn write_result(dir: &Path, r: &InstallResult) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(r).map_err(std::io::Error::other)?;
    std::fs::write(dir.join(RESULT_FILE), text)
}

/// Read and delete the result file; a missing or unreadable file gives `None`.
pub fn take_result(dir: &Path) -> Option<InstallResult> {
    let path = dir.join(RESULT_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    serde_json::from_str(&text).ok()
}

// ---- paths -----------------------------------------------------------------------------------------

/// Where the helper copy of the launcher lives.
pub fn helper_copy_path(updates_dir: &Path) -> PathBuf {
    updates_dir.join("helper").join(HELPER_EXE)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn logs_dir(updates_dir: &Path) -> PathBuf {
    updates_dir.parent().map(|p| p.join("logs")).unwrap_or_else(|| updates_dir.join("logs"))
}

#[cfg_attr(not(windows), allow(dead_code))]
fn log_file_name(version: &str, stamp: &str) -> String {
    let v: String = version.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+') { c } else { '_' }).collect();
    format!("install-{v}-{stamp}.log")
}

// ---- starting the helper (from the launcher) -------------------------------------------------------------

/// Copy the running launcher aside and start it in helper mode. The caller must then close
/// the launcher so the helper can proceed.
#[cfg(test)]
pub fn start_helper(msi: &Path, version: &str, updates_dir: &Path, feed_url: Option<&str>) -> Result<(), String> {
    crate::testhooks::with(|f| {
        f.helpers.push(crate::testhooks::HelperCall {
            msi: msi.to_path_buf(),
            version: version.to_owned(),
            updates_dir: updates_dir.to_path_buf(),
            feed_url: feed_url.map(str::to_owned),
        });
        f.helper_result.clone()
    })
}

#[cfg(all(windows, not(test)))]
pub fn start_helper(msi: &Path, version: &str, updates_dir: &Path, feed_url: Option<&str>) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe().map_err(|e| format!("Could not find the launcher: {e}"))?;
    let helper = helper_copy_path(updates_dir);
    if let Some(dir) = helper.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Could not prepare the update: {e}"))?;
    }
    std::fs::copy(&exe, &helper).map_err(|e| format!("Could not prepare the update: {e}"))?;
    let mut cmd = std::process::Command::new(&helper);
    cmd.arg("--apply-update").arg(msi).arg("--wait-pid").arg(std::process::id().to_string()).arg("--target-version").arg(version);
    if let Some(dir) = exe.parent() {
        cmd.arg("--install-dir").arg(dir);
    }
    if let Some(u) = feed_url.filter(|u| crate::feed::is_loopback_url(u)) {
        cmd.arg("--relaunch-feed-url").arg(u);
    }
    cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start the update: {e}"))
}

#[cfg(all(not(windows), not(test)))]
pub fn start_helper(_msi: &Path, _version: &str, _updates_dir: &Path, _feed_url: Option<&str>) -> Result<(), String> {
    Err("Installing updates works only on Windows.".to_owned())
}

/// True while a process with this file name (for example `kernpage.exe`) is running.
#[cfg(windows)]
pub fn process_running(exe_name: &str) -> bool {
    #[cfg(test)]
    if let Some(r) = crate::testhooks::peek(|f| f.editor_running) {
        return r;
    }
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut more = Process32FirstW(snap, &mut entry) != 0;
        while more {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case(exe_name) {
                found = true;
                break;
            }
            more = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(not(windows))]
pub fn process_running(_exe_name: &str) -> bool {
    false
}

// ---- helper mode ---------------------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use super::*;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Wait up to `max_ms` for the process to end. A process that is already gone counts as ended.
    pub fn wait_for_pid(pid: u32, max_ms: u32) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if h.is_null() {
                return true;
            }
            let r = WaitForSingleObject(h, max_ms);
            CloseHandle(h);
            r == 0
        }
    }

    pub fn local_stamp() -> String {
        let t = unsafe {
            let mut t = std::mem::zeroed();
            GetLocalTime(&mut t);
            t
        };
        format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
    }

    /// Run `program` (msiexec or a Setup.exe) hidden and wait for its exit code.
    pub fn run_plain(program: &Path, args: &[String]) -> Result<u32, String> {
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new(program)
            .args(args)
            .creation_flags(0x0800_0000)
            .status()
            .map_err(|e| format!("Could not start the installer: {e}"))?;
        status.code().map(|c| c as u32).ok_or_else(|| "The Windows Installer ended unexpectedly.".to_owned())
    }

    /// Run msiexec with the "runas" verb (UAC prompt) and wait for it.
    pub fn run_elevated(args: &[String]) -> Result<u32, String> {
        let verb = wide("runas");
        let file = wide(&kernpage_core::sysexe::system32("msiexec.exe").to_string_lossy());
        let params = wide(&join_command_line(args));
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOCLOSEPROCESS;
            info.lpVerb = verb.as_ptr();
            info.lpFile = file.as_ptr();
            info.lpParameters = params.as_ptr();
            info.nShow = SW_HIDE;
            if ShellExecuteExW(&mut info) == 0 {
                return match GetLastError() {
                    ERROR_CANCELLED => Ok(ERROR_CANCELLED),
                    e => Err(format!("Could not start the Windows Installer (error {e}).")),
                };
            }
            let h: HANDLE = info.hProcess;
            if h.is_null() {
                return Err("The Windows Installer ended unexpectedly.".to_owned());
            }
            WaitForSingleObject(h, INFINITE);
            let mut code = 0u32;
            let ok = GetExitCodeProcess(h, &mut code) != 0;
            CloseHandle(h);
            if ok {
                Ok(code)
            } else {
                Err("Could not read the result of the Windows Installer.".to_owned())
            }
        }
    }
}

/// Run helper mode to the end and return the process exit code.
#[cfg(windows)]
pub fn run_helper(a: &ApplyUpdate) -> i32 {
    let updates = crate::download::updates_dir();
    let version = a.target_version.clone().unwrap_or_else(|| "unknown".to_owned());
    let scope = a.install_dir.as_deref().map_or(InstallScope::NotInstalled, scope_of_install_dir);
    let log = logs_dir(&updates).join(log_file_name(&version, &win::local_stamp()));
    let (exit_code, ok, message) = match apply(a, scope, &log) {
        Ok(code) => {
            let (ok, msg) = map_exit_code(code);
            (Some(code), ok, msg)
        }
        Err(msg) => (None, false, msg),
    };
    let result = InstallResult {
        target_version: version,
        ok,
        exit_code,
        message,
        log_path: if log.is_file() { log.display().to_string() } else { String::new() },
        finished_at: crate::feed::now_unix(),
    };
    let _ = write_result(&updates, &result);
    if let Some(exe) = a.install_dir.as_ref().map(|d| d.join(LAUNCHER_EXE)).filter(|e| e.is_file()) {
        let _ = std::process::Command::new(exe).args(relaunch_args(a.relaunch_feed_url.as_deref())).spawn();
    }
    i32::from(!ok)
}

#[cfg(windows)]
struct RealBundleOps<'a> {
    updates: &'a Path,
}

#[cfg(windows)]
impl BundleOps for RealBundleOps<'_> {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<u32, String> {
        win::run_plain(program, args)
    }

    fn installed_bundle_exe(&mut self) -> Option<PathBuf> {
        use std::os::windows::process::CommandExt;
        let key = RegistryBundleProbe.bundle_key()?;
        let out = std::process::Command::new(kernpage_core::sysexe::system32("reg.exe"))
            .args(["query", &key, "/v", "UninstallString"])
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        bundle_exe_from_reg_output(&String::from_utf8_lossy(&out.stdout)).filter(|p| p.is_file())
    }

    fn save_copy(&mut self, from: &Path) -> Result<PathBuf, String> {
        let dir = self.updates.join("helper").join("restore");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let to = dir.join(from.file_name().ok_or("no file name")?);
        std::fs::copy(from, &to).map_err(|e| e.to_string())?;
        Ok(to)
    }
}

#[cfg(windows)]
fn apply(a: &ApplyUpdate, scope: InstallScope, log: &Path) -> Result<u32, String> {
    if scope == InstallScope::NotInstalled {
        return Err("Installing works only in an installed copy of KernPage. Nothing was changed.".to_owned());
    }
    let Some(kind) = InstallerKind::of(&a.msi).filter(|_| path_is_plain(&a.msi)) else {
        return Err("The update file is not an installer this launcher will run. Nothing was changed.".to_owned());
    };
    if !a.msi.is_file() {
        return Err("The downloaded update file is missing. Nothing was changed.".to_owned());
    }
    // A Setup.exe install is updated only through Setup.exe (see the module notes), and Setup.exe
    // is per-user only.
    let bundle = RegistryBundleProbe.bundle_key().is_some();
    match (kind, bundle, scope) {
        (InstallerKind::Msi, true, _) => {
            return Err("This copy was installed with Setup.exe, so it is updated with Setup.exe, not the plain MSI. Nothing was changed.".to_owned())
        }
        (InstallerKind::Setup, _, InstallScope::PerMachine) => {
            return Err("Setup.exe installs only for the current user, so it cannot update an all-users install. Nothing was changed.".to_owned())
        }
        _ => {}
    }
    if !win::wait_for_pid(a.wait_pid, 60_000) {
        return Err("KernPage Launcher did not close in time, so the update was not started. Nothing was changed.".to_owned());
    }
    // Verify again, now, from a handle nobody can write to, rename or delete until this function
    // returns (security review M2): the check and the use are on the same locked file.
    let keys = crate::feed::trusted_keys_for_url(a.relaunch_feed_url.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
    let _locked = crate::download::open_verified_for_install(&a.msi, &crate::download::updates_dir(), &keys)
        .map_err(|e| format!("{} Nothing was changed.", crate::download::refusal_text(&e)))?;
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Could not create the log folder: {e}"))?;
    }
    match kind {
        InstallerKind::Setup => run_setup(&mut RealBundleOps { updates: &crate::download::updates_dir() }, &a.msi, log),
        InstallerKind::Msi => {
            let args = msiexec_args(&a.msi, log, scope).ok_or_else(|| "Nothing may be installed here.".to_owned())?;
            match scope {
                InstallScope::PerMachine => win::run_elevated(&args),
                _ => win::run_plain(&kernpage_core::sysexe::system32("msiexec.exe"), &args),
            }
        }
    }
}

#[cfg(not(windows))]
pub fn run_helper(_a: &ApplyUpdate) -> i32 {
    eprintln!("Installing updates works only on Windows.");
    1
}

/// Open a log in Notepad.
pub fn show_log(path: &str) {
    #[cfg(test)]
    {
        crate::testhooks::with(|f| f.logs.push(path.to_owned()));
    }
    #[cfg(not(test))]
    {
        let _ = std::process::Command::new(kernpage_core::sysexe::windows_exe("notepad.exe")).arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn relaunch_args_only_for_loopback() {
        assert!(relaunch_args(None).is_empty());
        assert!(relaunch_args(Some("https://example.com/feed.json")).is_empty());
        assert_eq!(relaunch_args(Some("http://127.0.0.1:8123/feed.json")), ["--feed-url", "http://127.0.0.1:8123/feed.json", "--check-now"]);
    }

    #[test]
    fn scope_follows_the_install_folder() {
        let user = [p(r"C:\Users\x\AppData\Local\Programs")];
        let machine = [p(r"C:\Program Files")];
        let s = |d| scope_of_dir(Path::new(d), &user, &machine);
        assert_eq!(s(r"c:\users\x\appdata\local\programs\KernPage"), InstallScope::PerUser);
        assert_eq!(s(r"C:\Program Files\KernPage"), InstallScope::PerMachine);
        assert_eq!(s(r"C:\src\kernpage\target-win\release"), InstallScope::NotInstalled);
        assert_eq!(s(r"C:\Program Files\Other"), InstallScope::NotInstalled);
    }

    #[test]
    fn msiexec_args_per_user_and_per_machine() {
        let (msi, log) = (Path::new("a b.msi"), Path::new("l.log"));
        let user = msiexec_args(msi, log, InstallScope::PerUser).unwrap();
        assert_eq!(user, ["/i", "a b.msi", "/qn", "/norestart", "/l*v", "l.log", "MSIINSTALLPERUSER=1"]);
        let machine = msiexec_args(msi, log, InstallScope::PerMachine).unwrap();
        assert_eq!(machine.last().unwrap(), "ALLUSERS=1");
        assert!(!machine.iter().any(|a| a.contains("MSIINSTALLPERUSER")));
        assert!(msiexec_args(msi, log, InstallScope::NotInstalled).is_none());
        assert_eq!(join_command_line(&user), "/i \"a b.msi\" /qn /norestart /l*v l.log MSIINSTALLPERUSER=1");
    }

    #[test]
    fn setup_exe_runs_quietly_with_a_log() {
        assert_eq!(setup_args(Path::new(r"C:\l\x.log")), ["/quiet", "/norestart", "/log", r"C:\l\x.log"]);
    }

    #[test]
    fn installer_kind_follows_the_extension_and_paths_must_be_plain() {
        assert_eq!(InstallerKind::of(Path::new("a/B.MSI")), Some(InstallerKind::Msi));
        assert_eq!(InstallerKind::of(Path::new("a/KernPage-1.0.0-Setup.exe")), Some(InstallerKind::Setup));
        assert_eq!(InstallerKind::of(Path::new("a/evil.bat")), None);
        assert_eq!(InstallerKind::of(Path::new("a/noext")), None);
        assert!(path_is_plain(Path::new(r"C:\Users\a b\x.msi")));
        for bad in ["a\"b.msi", "a%TEMP%.msi", "a\nb.msi", ""] {
            assert!(!path_is_plain(Path::new(bad)), "{bad:?}");
        }
    }

    #[derive(Default)]
    struct Fake {
        /// Exit codes handed out in order by `run`; the calls made are recorded.
        codes: Vec<u32>,
        calls: Vec<(String, Vec<String>)>,
        bundle: Option<PathBuf>,
        save_fails: bool,
    }

    impl BundleOps for Fake {
        fn run(&mut self, program: &Path, args: &[String]) -> Result<u32, String> {
            self.calls.push((program.display().to_string(), args.to_vec()));
            Ok(self.codes.remove(0))
        }
        fn installed_bundle_exe(&mut self) -> Option<PathBuf> {
            self.bundle.clone()
        }
        fn save_copy(&mut self, from: &Path) -> Result<PathBuf, String> {
            if self.save_fails {
                return Err("disk full".into());
            }
            Ok(PathBuf::from("saved").join(from.file_name().unwrap()))
        }
    }

    fn fake(codes: &[u32]) -> Fake {
        Fake { codes: codes.to_vec(), bundle: Some(p(r"C:\Package Cache\{id}\Setup-2.exe")), ..Default::default() }
    }

    const SETUP: &str = r"C:\u\Setup-1.exe";

    #[test]
    fn an_upgrade_is_one_quiet_setup_run() {
        let mut f = fake(&[0]);
        assert_eq!(run_setup(&mut f, Path::new(SETUP), Path::new("l.log")), Ok(0));
        assert_eq!(f.calls, vec![(SETUP.to_owned(), vec!["/quiet".to_owned(), "/norestart".to_owned(), "/log".to_owned(), "l.log".to_owned()])]);
        // A real failure of an upgrade is reported as is; no uninstall is attempted.
        let mut f = fake(&[1603]);
        assert_eq!(run_setup(&mut f, Path::new(SETUP), Path::new("l.log")), Ok(1603));
        assert_eq!(f.calls.len(), 1);
    }

    #[test]
    fn going_back_uninstalls_the_bundle_then_runs_the_older_setup() {
        let mut f = fake(&[1638, 0, 0]);
        assert_eq!(run_setup(&mut f, Path::new(SETUP), Path::new("l.log")), Ok(0));
        let progs: Vec<&str> = f.calls.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(progs, [SETUP, r"C:\Package Cache\{id}\Setup-2.exe", SETUP]);
        assert_eq!(f.calls[1].1[..3], ["/uninstall", "/quiet", "/norestart"]);
    }

    #[test]
    fn going_back_that_fails_puts_the_previous_version_back() {
        let mut f = fake(&[0x8007_0666, 0, 1603, 0]);
        let e = run_setup(&mut f, Path::new(SETUP), Path::new("l.log")).unwrap_err();
        assert!(e.contains("previous version was put back") && e.contains("1603"), "{e}");
        assert_eq!(f.calls[3].0, r"saved\Setup-2.exe", "the saved copy of the old bundle is run");
        let mut f = fake(&[1638, 0, 1603, 1603]);
        let e = run_setup(&mut f, Path::new(SETUP), Path::new("l.log")).unwrap_err();
        assert!(e.contains("could not be put back") && e.contains("Setup.exe again"), "{e}");
    }

    #[test]
    fn going_back_is_not_started_without_a_safe_way_back() {
        // No bundle exe to save: the refusal is reported and nothing was uninstalled.
        let mut f = Fake { codes: vec![1638], ..Default::default() };
        assert_eq!(run_setup(&mut f, Path::new(SETUP), Path::new("l.log")), Ok(1638));
        assert_eq!(f.calls.len(), 1);
        // The copy cannot be saved: nothing is uninstalled.
        let mut f = fake(&[1638]);
        f.save_fails = true;
        assert!(run_setup(&mut f, Path::new(SETUP), Path::new("l.log")).unwrap_err().contains("Nothing was changed"));
        assert_eq!(f.calls.len(), 1);
        // The uninstall itself fails: the current version stays, no older Setup run.
        let mut f = fake(&[1638, 1603]);
        assert_eq!(run_setup(&mut f, Path::new(SETUP), Path::new("l.log")), Ok(1603));
        assert_eq!(f.calls.len(), 2);
    }

    #[test]
    fn bundle_exe_is_read_from_the_uninstall_string() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\x\\{id}\r\n    UninstallString    REG_EXPAND_SZ    \"C:\\Users\\a\\AppData\\Local\\Package Cache\\{id}\\KernPage-1.0.0-Setup.exe\"  /uninstall\r\n";
        assert_eq!(bundle_exe_from_reg_output(out), Some(p(r"C:\Users\a\AppData\Local\Package Cache\{id}\KernPage-1.0.0-Setup.exe")));
        assert_eq!(bundle_exe_from_reg_output(&out.replace("Package Cache", "Elsewhere")), None, "only Package Cache is trusted");
        assert_eq!(bundle_exe_from_reg_output(""), None);
        assert!(is_downgrade_blocked(1638) && is_downgrade_blocked(0x8007_0666) && !is_downgrade_blocked(1603));
    }

    #[test]
    fn bundle_key_is_found_in_reg_output() {
        let out = "\r\nHKEY_CURRENT_USER/Software/Uninstall/{aaa}\r\n    BundleUpgradeCode    REG_SZ    {11111111-0000-0000-0000-000000000000}\r\n\r\nHKEY_CURRENT_USER/Software/Uninstall/{bbb}\r\n    BundleUpgradeCode    REG_MULTI_SZ    {5d0e7b3a-2c41-4f6b-9a18-c3e7f0a4b6d2}\r\n";
        assert!(bundle_key_from_reg_output(out).unwrap().ends_with("{bbb}"));
        assert!(bundle_key_from_reg_output("").is_none());
        assert!(bundle_key_from_reg_output(&out.replace("5d0e7b3a", "00000000")).is_none());
    }

    #[test]
    fn exit_codes_map_to_plain_messages() {
        assert!(map_exit_code(0).0);
        let (ok, m) = map_exit_code(3010);
        assert!(ok && m.contains("Restart Windows"));
        for c in [1602, 1223] {
            let (ok, m) = map_exit_code(c);
            assert!(!ok && m.contains("cancelled") && m.contains("still installed"));
        }
        let (ok, m) = map_exit_code(1618);
        assert!(!ok && m.contains("already in progress"));
        let (ok, m) = map_exit_code(1603);
        assert!(!ok && m.contains("1603") && m.contains("put your current version back"));
    }

    #[test]
    fn result_file_round_trip() {
        let dir = std::env::temp_dir().join(format!("pw-install-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(take_result(&dir).is_none());
        let r = InstallResult {
            target_version: "1.2.3".into(),
            ok: false,
            exit_code: Some(1603),
            message: "failed".into(),
            log_path: r"C:\logs\x.log".into(),
            finished_at: 1_700_000_000,
        };
        write_result(&dir, &r).unwrap();
        assert_eq!(take_result(&dir), Some(r));
        assert!(!dir.join(RESULT_FILE).exists(), "the file is deleted once read");
        std::fs::write(dir.join(RESULT_FILE), "not json").unwrap();
        assert!(take_result(&dir).is_none());
        assert!(!dir.join(RESULT_FILE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn helper_and_log_paths() {
        let updates = p("data").join("KernPage").join("updates");
        assert_eq!(helper_copy_path(&updates), updates.join("helper").join("kernpage-update-helper.exe"));
        assert_eq!(logs_dir(&updates), p("data").join("KernPage").join("logs"));
        assert_eq!(log_file_name("1.2.3-beta.1", "20260102-030405"), "install-1.2.3-beta.1-20260102-030405.log");
        assert_eq!(log_file_name("a/b", "s"), "install-a_b-s.log");
    }
}
