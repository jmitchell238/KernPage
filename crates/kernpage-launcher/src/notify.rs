//! Windows toast notification for "an update is available".
//!
//! A toast needs an AppUserModelID (AUMID). Windows only shows our own name and icon for an
//! AUMID that a Start-menu shortcut registers, and only the installer creates that shortcut
//! (it carries `System.AppUserModel.ID` = `AUMID`). So the identity is used only when the
//! launcher runs from an installed folder (`install::install_scope()`); from a dev build there is no
//! shortcut, so the toast is sent under the stock PowerShell identity so it still appears (but
//! Windows labels it "Windows PowerShell"). Toast action buttons (What's new / Later / Update)
//! also need the registered identity to receive clicks; the in-app dialog carries those three
//! actions.

pub const AUMID: &str = "KernPage.Launcher";

#[cfg(test)]
pub fn update_available(name: &str, version: &str) -> bool {
    let _ = AUMID;
    crate::testhooks::with(|f| f.toasts.push((name.to_owned(), version.to_owned())));
    true
}

#[cfg(all(windows, not(test)))]
pub fn update_available(name: &str, version: &str) -> bool {
    use tauri_winrt_notification::Toast;
    let app_id = if crate::install::install_scope() != crate::install::InstallScope::NotInstalled { AUMID } else { Toast::POWERSHELL_APP_ID };
    Toast::new(app_id)
        .title(&format!("{name} {version} is available"))
        .text1("Open the KernPage Launcher to see what's new and update.")
        .show()
        .is_ok()
}

#[cfg(all(not(windows), not(test)))]
pub fn update_available(_name: &str, _version: &str) -> bool {
    let _ = AUMID;
    false
}
