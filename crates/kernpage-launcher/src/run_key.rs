//! "Start at login": a value in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//! The registry is behind the [`RunStore`] trait so the logic can be tested without touching it.

use std::io;
use std::path::Path;

pub const VALUE_NAME: &str = "KernPage Launcher";

pub trait RunStore {
    fn get(&self, name: &str) -> Option<String>;
    fn set(&mut self, name: &str, command: &str) -> io::Result<()>;
    fn remove(&mut self, name: &str) -> io::Result<()>;
}

/// The command line stored in the Run key: the quoted path of the launcher.
pub fn command_for(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// True when the Run value exists.
pub fn is_enabled(store: &dyn RunStore) -> bool {
    store.get(VALUE_NAME).is_some()
}

/// Add (or refresh, so a moved install still works) or remove the Run value.
pub fn set_enabled(store: &mut dyn RunStore, enabled: bool, exe: &Path) -> io::Result<()> {
    if enabled {
        store.set(VALUE_NAME, &command_for(exe))
    } else if store.get(VALUE_NAME).is_some() {
        store.remove(VALUE_NAME)
    } else {
        Ok(())
    }
}

/// The real current-user Run key (through `reg.exe`); does nothing on other systems.
pub struct RegistryRunStore;

#[cfg(all(windows, not(test)))]
mod real {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

    fn reg(args: &[&str]) -> io::Result<std::process::Output> {
        Command::new(kernpage_core::sysexe::system32("reg.exe"))
            .args(args)
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .output()
    }

    impl RunStore for RegistryRunStore {
        fn get(&self, name: &str) -> Option<String> {
            let out = reg(&["query", KEY, "/v", name]).ok()?;
            if !out.status.success() {
                return None;
            }
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .find(|l| l.trim_start().starts_with(name))
                .and_then(|l| l.split("REG_SZ").nth(1))
                .map(|v| v.trim().to_owned())
        }
        fn set(&mut self, name: &str, command: &str) -> io::Result<()> {
            let out = reg(&["add", KEY, "/v", name, "/t", "REG_SZ", "/d", command, "/f"])?;
            out.status.success().then_some(()).ok_or_else(|| io::Error::other("reg add failed"))
        }
        fn remove(&mut self, name: &str) -> io::Result<()> {
            let out = reg(&["delete", KEY, "/v", name, "/f"])?;
            out.status.success().then_some(()).ok_or_else(|| io::Error::other("reg delete failed"))
        }
    }
}

#[cfg(test)]
impl RunStore for RegistryRunStore {
    fn get(&self, name: &str) -> Option<String> {
        crate::testhooks::with(|f| f.run_key.get(name).cloned())
    }
    fn set(&mut self, name: &str, command: &str) -> io::Result<()> {
        crate::testhooks::with(|f| {
            if f.run_key_fails {
                return Err(io::Error::other("reg add failed"));
            }
            f.run_key.insert(name.to_owned(), command.to_owned());
            Ok(())
        })
    }
    fn remove(&mut self, name: &str) -> io::Result<()> {
        crate::testhooks::with(|f| {
            if f.run_key_fails {
                return Err(io::Error::other("reg delete failed"));
            }
            f.run_key.remove(name);
            Ok(())
        })
    }
}

#[cfg(all(not(windows), not(test)))]
impl RunStore for RegistryRunStore {
    fn get(&self, _: &str) -> Option<String> {
        None
    }
    fn set(&mut self, _: &str, _: &str) -> io::Result<()> {
        Ok(())
    }
    fn remove(&mut self, _: &str) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Fake(HashMap<String, String>);
    impl RunStore for Fake {
        fn get(&self, n: &str) -> Option<String> {
            self.0.get(n).cloned()
        }
        fn set(&mut self, n: &str, c: &str) -> io::Result<()> {
            self.0.insert(n.into(), c.into());
            Ok(())
        }
        fn remove(&mut self, n: &str) -> io::Result<()> {
            self.0.remove(n);
            Ok(())
        }
    }

    #[test]
    fn add_and_remove_the_run_value() {
        let mut s = Fake::default();
        let exe = Path::new("C:/Apps/KernPage/kernpage-launcher.exe");
        assert!(!is_enabled(&s));
        set_enabled(&mut s, true, exe).unwrap();
        assert!(is_enabled(&s));
        assert_eq!(s.get(VALUE_NAME).unwrap(), "\"C:/Apps/KernPage/kernpage-launcher.exe\"");
        set_enabled(&mut s, false, exe).unwrap();
        assert!(!is_enabled(&s));
        // Removing when absent is not an error.
        set_enabled(&mut s, false, exe).unwrap();
    }

    #[test]
    fn enabling_again_refreshes_the_path() {
        let mut s = Fake::default();
        set_enabled(&mut s, true, Path::new("/old/l.exe")).unwrap();
        set_enabled(&mut s, true, Path::new("/new/l.exe")).unwrap();
        assert_eq!(s.get(VALUE_NAME).unwrap(), "\"/new/l.exe\"");
        assert_eq!(s.0.len(), 1);
    }
}
