#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! KernPage Launcher: the front door for KernPage. Step 1: window, open, recent
//! files and settings. No networking.

mod app;
mod config;
mod content;
#[cfg(test)]
mod devfeed;
#[cfg(test)]
mod devserver;
#[cfg(any(debug_assertions, feature = "dev-feed"))]
mod devcli;
mod download;
mod feed;
mod install;
mod instance;
mod mark;
mod net;
mod notify;
mod policy;
mod quiet;
mod run_key;
#[cfg(test)]
mod testhooks;
mod theme;
mod tray;
mod updates;
mod views;
#[cfg(test)]
mod ui_tests;

use app::{App, Options};
use config::Appearance;
use std::path::PathBuf;

const USAGE: &str = "Usage: kernpage-launcher [--screenshot out.png] [--window-size WxH] [--details] [--settings] [--theme light|dark]\n  [--feed-url <url>] [--check-now] [--update-dialog] [--check-update]\n--feed-url (and --update-now [--update-to <version>], a headless update for tests) exist only in debug and `dev-feed` builds; a normal release build refuses them.";

fn parse_args() -> Result<Options, String> {
    parse_args_from(std::env::args().skip(1))
}

fn parse_args_from(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut opts = Options::default();
    let mut apply = install::ApplyUpdate::default();
    let (mut have_msi, mut have_pid) = (false, false);
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match a.as_str() {
            "--screenshot" => opts.screenshot = Some(PathBuf::from(value("--screenshot")?)),
            "--details" => opts.details = true,
            "--check-now" => opts.check_now = true,
            "--update-dialog" => opts.update_dialog = true,
            "--check-update" => opts.check_update = true,
            "--feed-url" => {
                let u = value("--feed-url")?;
                if !(u.starts_with("http://") || u.starts_with("https://")) {
                    return Err(format!("--feed-url needs an http:// or https:// address, got {u:?}"));
                }
                if !feed::DEV_FEEDS {
                    return Err("--feed-url is not available in this build".to_owned());
                }
                opts.feed_url = Some(u);
            }
            "--update-now" | "--update-to" => {
                if !feed::DEV_FEEDS {
                    return Err(format!("{a} is not available in this build"));
                }
                if a == "--update-to" {
                    opts.update_to = Some(value("--update-to")?);
                }
                opts.update_now = true;
            }
            "--apply-update" => {
                apply.msi = PathBuf::from(value("--apply-update")?);
                have_msi = true;
            }
            "--wait-pid" => {
                let v = value("--wait-pid")?;
                apply.wait_pid = v.parse().map_err(|_| format!("--wait-pid needs a process number, got {v:?}"))?;
                have_pid = true;
            }
            "--relaunch-feed-url" => {
                let u = value("--relaunch-feed-url")?;
                if !feed::is_loopback_url(&u) {
                    return Err("--relaunch-feed-url is only accepted for loopback hosts (127.0.0.1 or localhost)".to_owned());
                }
                apply.relaunch_feed_url = Some(u);
            }
            "--target-version" => apply.target_version = Some(value("--target-version")?),
            "--install-dir" => apply.install_dir = Some(PathBuf::from(value("--install-dir")?)),
            "--settings" => opts.settings = true,
            "--theme" => {
                let v = value("--theme")?;
                opts.theme = Some(match Appearance::parse(&v) {
                    Some(t) => t,
                    None => return Err(format!("--theme needs light, dark or system, got {v:?}")),
                });
            }
            "--window-size" => {
                let v = value("--window-size")?;
                let bad = || format!("--window-size needs <width>x<height>, got {v:?}");
                let (w, h) = v.split_once(['x', 'X']).ok_or_else(bad)?;
                opts.window_size = Some((
                    w.trim().parse().map_err(|_| bad())?,
                    h.trim().parse().map_err(|_| bad())?,
                ));
            }
            "-h" | "--help" => return Err(USAGE.to_owned()),
            s => return Err(format!("unknown option {s}\n{USAGE}")),
        }
    }
    if have_msi {
        if !have_pid {
            return Err(format!("--apply-update needs --wait-pid
{USAGE}"));
        }
        opts.apply_update = Some(apply);
    } else if have_pid || apply.target_version.is_some() || apply.install_dir.is_some() || apply.relaunch_feed_url.is_some() {
        return Err(format!("--wait-pid, --target-version, --install-dir and --relaunch-feed-url go with --apply-update
{USAGE}"));
    }
    Ok(opts)
}

/// The `--version` handshake line: `<name> <version>`, nothing else.
fn version_line() -> String {
    format!("kernpage-launcher {}", app::app_version())
}

/// Release builds are GUI programs (no console window). For `--version` and argument errors
/// run from a terminal, attach to the parent console so the text shows; output that is
/// already redirected (a pipe or file) is left alone.
fn attach_parent_console() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{
            AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
        };
        if GetStdHandle(STD_OUTPUT_HANDLE).is_null() {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

/// DLLs that are loaded later (by name, at run time) come only from the Windows system folder
/// and the program's own folder, never from the current directory or `PATH` (so a file opened
/// from a folder that contains a planted DLL cannot load it; security audit PW-SEC-004).
fn harden_dll_search() {
    #[cfg(windows)]
    // SAFETY: a plain Win32 call with constant flags; it only narrows the DLL search order.
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{
            SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_APPLICATION_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
        };
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR);
    }
}

fn main() -> eframe::Result {
    harden_dll_search();
    // Handled first: no window, instance lock or network activity.
    if std::env::args().nth(1).as_deref() == Some("--version") {
        attach_parent_console();
        println!("{}", version_line());
        return Ok(());
    }
    let opts = match parse_args() {
        Ok(o) => o,
        Err(msg) => {
            attach_parent_console();
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    // Helper mode (see install.rs): no window and no instance lock.
    if let Some(apply) = &opts.apply_update {
        std::process::exit(install::run_helper(apply));
    }
    // Headless download-and-install for the end-to-end test of a dev build (see devcli.rs).
    #[cfg(any(debug_assertions, feature = "dev-feed"))]
    if opts.update_now {
        std::process::exit(devcli::run(opts.feed_url.clone(), opts.update_to.clone()));
    }
    // Quiet check for the editor (see quiet.rs): no window and no instance lock.
    if opts.check_update {
        std::process::exit(quiet::run(opts.feed_url.clone()));
    }
    // Screenshot runs must not collide with (or focus) a real launcher.
    let _single = if opts.screenshot.is_none() {
        match instance::acquire() {
            Some(g) => Some(g),
            None => return Ok(()),
        }
    } else {
        None
    };
    let (w, h) = opts.window_size.unwrap_or((1100.0, 760.0));
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([w, h])
        .with_min_inner_size([w.min(760.0), h.min(520.0)])
        .with_title(instance::WINDOW_TITLE)
        .with_icon(window_icon());
    let native = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        instance::WINDOW_TITLE,
        native,
        Box::new(move |cc| Ok(Box::new(App::new(cc, opts)))),
    )
}

/// Title-bar / taskbar icon: the real app icon (packaging/windows/icon/png-master/256.png).
fn window_icon() -> eframe::egui::IconData {
    const PNG: &[u8] = include_bytes!("../../../packaging/windows/icon/png-master/256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(PNG)).read_info().expect("icon png");
    let mut buf = vec![0u8; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut buf).expect("icon frame");
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        other => panic!("unsupported icon png colour type {other:?}"),
    };
    eframe::egui::IconData { rgba, width: info.width, height: info.height }
}

#[cfg(test)]
mod icon_tests {
    #[test]
    fn window_icon_decodes_to_rgba() {
        let i = super::window_icon();
        assert!(i.width > 0 && i.height > 0);
        assert_eq!(i.rgba.len(), (i.width * i.height * 4) as usize);
        assert!(i.rgba.chunks_exact(4).any(|p| p[3] != 0));
    }
}

#[cfg(test)]
mod version_tests {
    #[test]
    fn version_line_is_name_and_cargo_version() {
        assert_eq!(super::version_line(), format!("kernpage-launcher {}", crate::app::app_version()));
    }
}

#[cfg(test)]
mod arg_tests {
    use super::parse_args_from;
    use std::path::PathBuf;

    fn parse(a: &[&str]) -> Result<crate::app::Options, String> {
        parse_args_from(a.iter().map(|s| s.to_string()))
    }

    #[test]
    fn helper_mode_arguments() {
        let o = parse(&["--apply-update", r"C:\u\x.msi", "--wait-pid", "42", "--target-version", "1.2.3", "--install-dir", r"C:\i"]).unwrap();
        let a = o.apply_update.unwrap();
        assert_eq!(a.msi, PathBuf::from(r"C:\u\x.msi"));
        assert_eq!(a.wait_pid, 42);
        assert_eq!(a.target_version.as_deref(), Some("1.2.3"));
        assert_eq!(a.install_dir, Some(PathBuf::from(r"C:\i")));
        let min = parse(&["--apply-update", "x.msi", "--wait-pid", "7"]).unwrap().apply_update.unwrap();
        assert_eq!((min.wait_pid, min.target_version, min.install_dir), (7, None, None));
    }

    #[test]
    fn helper_mode_errors() {
        assert!(parse(&["--apply-update", "x.msi"]).is_err());
        assert!(parse(&["--apply-update", "x.msi", "--wait-pid", "abc"]).is_err());
        assert!(parse(&["--wait-pid", "1"]).is_err());
        assert!(parse(&["--apply-update"]).is_err());
        assert!(parse(&[]).unwrap().apply_update.is_none());
    }

    #[test]
    fn dev_only_flags_follow_the_build() {
        let feed = parse(&["--feed-url", "http://127.0.0.1:9/index.json"]);
        let now = parse(&["--update-now"]);
        if crate::feed::DEV_FEEDS {
            assert!(feed.is_ok() && now.unwrap().update_now);
            assert!(parse(&["--update-to", "1.2.3"]).unwrap().update_to.as_deref() == Some("1.2.3"));
        } else {
            assert!(feed.is_err() && now.is_err() && parse(&["--update-to", "1.2.3"]).is_err());
        }
    }

    #[test]
    fn relaunch_feed_url_rules() {
        let ok = parse(&["--apply-update", "x.msi", "--wait-pid", "7", "--relaunch-feed-url", "http://127.0.0.1:9/f.json"]).unwrap();
        assert_eq!(ok.apply_update.unwrap().relaunch_feed_url.as_deref(), Some("http://127.0.0.1:9/f.json"));
        assert!(parse(&["--apply-update", "x.msi", "--wait-pid", "7", "--relaunch-feed-url", "https://example.com/f.json"]).is_err());
        assert!(parse(&["--relaunch-feed-url", "http://127.0.0.1:9/f.json"]).is_err());
        assert!(parse(&[]).unwrap().apply_update.is_none());
    }
}
