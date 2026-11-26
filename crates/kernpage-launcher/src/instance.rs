//! One launcher at a time: a named mutex on Windows; a second launch brings the first
//! window to the front and exits. Other systems do not enforce a single instance yet.

pub const WINDOW_TITLE: &str = "KernPage Launcher";

#[cfg(windows)]
mod imp {
    use super::WINDOW_TITLE;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Keeps the mutex alive for the life of the process.
    pub struct Guard(#[allow(dead_code)] isize);

    /// `Some(guard)` when we are the first launcher; `None` when another one is already
    /// running (it has been brought to the front).
    pub fn acquire() -> Option<Guard> {
        let name = wide("Local\\KernPageLauncherSingleInstance");
        // SAFETY: plain Win32 calls with NUL-terminated UTF-16 strings that outlive the calls.
        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
            if !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
                show_main_window();
                return None;
            }
            Some(Guard(handle as isize))
        }
    }

    /// Show, restore and focus the launcher window (also when it is hidden in the tray).
    pub fn show_main_window() {
        let title = wide(WINDOW_TITLE);
        // SAFETY: as above.
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_SHOW);
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct Guard;
    pub fn acquire() -> Option<Guard> {
        Some(Guard)
    }
    pub fn show_main_window() {}
}

pub use imp::{acquire, show_main_window};
