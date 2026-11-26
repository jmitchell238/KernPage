//! Absolute paths of the Windows programs KernPage starts (`reg.exe`, `msiexec.exe`,
//! `explorer.exe`, `notepad.exe`, `rundll32.exe`).
//!
//! A bare name such as `"msiexec.exe"` is looked up by Windows in several folders, among them the
//! application folder and, for `ShellExecute`, the current directory, so a planted look-alike
//! could run instead (and, for the elevated installer start, with administrator rights). These
//! helpers give the real file in the Windows folder (security audit PW-SEC-005).

use std::path::{Path, PathBuf};

/// The Windows folder: `%SystemRoot%` when it is an existing absolute folder, else `C:\Windows`.
pub fn windows_dir() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.is_dir())
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

/// `%SystemRoot%\System32\<name>` on Windows; the bare name elsewhere (tests on other systems).
pub fn system32(name: &str) -> PathBuf {
    if cfg!(windows) {
        windows_dir().join("System32").join(name)
    } else {
        Path::new(name).to_path_buf()
    }
}

/// `%SystemRoot%\<name>` on Windows (for `explorer.exe`, `notepad.exe`); the bare name elsewhere.
pub fn windows_exe(name: &str) -> PathBuf {
    if cfg!(windows) {
        windows_dir().join(name)
    } else {
        Path::new(name).to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn the_programs_are_absolute_and_exist() {
        for p in [system32("reg.exe"), system32("msiexec.exe"), system32("rundll32.exe"), windows_exe("explorer.exe"), windows_exe("notepad.exe")] {
            assert!(p.is_absolute(), "{p:?}");
            assert!(p.is_file(), "{p:?} is missing");
        }
    }

    #[test]
    fn the_folder_is_never_relative() {
        assert!(windows_dir().is_absolute() || !cfg!(windows));
    }
}
