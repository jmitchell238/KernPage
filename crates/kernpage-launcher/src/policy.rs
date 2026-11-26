//! When the launcher may fetch and install an update on its own ("Download and install
//! updates automatically"). A pure decision, so it can be tested without a window.

use crate::install::InstallScope;

pub struct AutoInstallInput {
    /// The Settings checkbox.
    pub setting: bool,
    /// A newer version of the KernPage app was found (not skipped, not blocked).
    pub is_newer: bool,
    /// The version is older than the installed one (going back is never automatic).
    pub is_downgrade: bool,
    pub scope: InstallScope,
    pub editor_running: bool,
    /// The launcher window is on screen (not hidden in the tray, not minimized).
    pub window_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoPlan {
    /// Start downloading in the background.
    pub download: bool,
    /// Install it now: nobody is using the editor or looking at the launcher.
    pub install: bool,
}

pub fn auto_install_plan(i: &AutoInstallInput) -> AutoPlan {
    let download = i.setting && i.is_newer && !i.is_downgrade;
    let install = download && i.scope != InstallScope::NotInstalled && !i.editor_running && !i.window_visible;
    AutoPlan { download, install }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> AutoInstallInput {
        AutoInstallInput {
            setting: true,
            is_newer: true,
            is_downgrade: false,
            scope: InstallScope::PerUser,
            editor_running: false,
            window_visible: false,
        }
    }

    #[test]
    fn installs_when_nobody_is_using_anything() {
        assert_eq!(auto_install_plan(&base()), AutoPlan { download: true, install: true });
    }

    #[test]
    fn setting_off_does_nothing() {
        let i = AutoInstallInput { setting: false, ..base() };
        assert_eq!(auto_install_plan(&i), AutoPlan { download: false, install: false });
    }

    #[test]
    fn not_newer_does_nothing() {
        let i = AutoInstallInput { is_newer: false, ..base() };
        assert_eq!(auto_install_plan(&i), AutoPlan { download: false, install: false });
    }

    #[test]
    fn a_downgrade_is_never_automatic() {
        let i = AutoInstallInput { is_downgrade: true, ..base() };
        assert_eq!(auto_install_plan(&i), AutoPlan { download: false, install: false });
    }

    #[test]
    fn a_dev_build_downloads_but_never_installs() {
        let i = AutoInstallInput { scope: InstallScope::NotInstalled, ..base() };
        assert_eq!(auto_install_plan(&i), AutoPlan { download: true, install: false });
    }

    #[test]
    fn a_running_editor_or_a_visible_window_holds_the_install_back() {
        for i in [AutoInstallInput { editor_running: true, ..base() }, AutoInstallInput { window_visible: true, ..base() }] {
            assert_eq!(auto_install_plan(&i), AutoPlan { download: true, install: false });
        }
    }

    #[test]
    fn both_install_scopes_may_install() {
        for scope in [InstallScope::PerUser, InstallScope::PerMachine] {
            assert!(auto_install_plan(&AutoInstallInput { scope, ..base() }).install);
        }
    }
}
