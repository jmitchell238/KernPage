#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! KernPage: the desktop editor.

mod app;
mod autorecover;
mod backstage;
mod canvas;
mod caption;
mod ctx_ribbon;
mod demo;
mod feedback;
mod filedialog;
mod fontmgr;
mod icons;
#[cfg(test)]
mod icons_color;
mod linkmarks;
mod logo;
mod lists_ui;
mod metapic;
mod masters;
mod nav;
mod output;
mod page_dialog;
mod page_setup;
mod printing;
mod recent;
mod search;
mod render;
mod ribbon;
mod ribbon_fit;
mod status;
mod table_ui;
mod theme;
mod thumbs;
mod update_notice;
mod widgets;

#[cfg(test)]
mod ui_tests;

use app::{App, Options};
use std::path::PathBuf;

const USAGE: &str =
    "Usage: kernpage [file.pub] [--screenshot out.png] [--open file.pub] [--page N] [--tab name] [--file-screen home|new|open|info|saveas|print|share|export|about|feedback|options] [--edit-cell shape-index,cell] [--edit-text shape-index[,start-end]] [--spacing-dialog] [--dialog page-setup|custom-margins] [--menu wrap|columns|margins|orientation|size] [--select i,j,...] [--window-size WxH] [--scale F] [--open-group name] [--print-layout one|copies|pages|booklet] [--print-sheet N] [--theme light|dark|system] [--accent teal|blue|purple|graphite|orange] [--demo] [--master]";

fn parse_args() -> Result<Options, String> {
    let mut opts = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match a.as_str() {
            "--screenshot" => opts.screenshot = Some(PathBuf::from(value("--screenshot")?)),
            "--open" => opts.open = Some(PathBuf::from(value("--open")?)),
            "--file-screen" | "--backstage" => opts.backstage = Some(value("--file-screen")?),
            "--edit-cell" => {
                let v = value("--edit-cell")?;
                let bad = || format!("--edit-cell needs <shape-index>,<cell>, got {v:?}");
                let (a, b) = v.split_once(',').ok_or_else(bad)?;
                opts.edit_cell = Some((
                    a.trim().parse().map_err(|_| bad())?,
                    b.trim().parse().map_err(|_| bad())?,
                ));
            }
            "--edit-text" => {
                let v = value("--edit-text")?;
                let bad = || format!("--edit-text needs <shape-index>[,<start>-<end>], got {v:?}");
                let (ix, sel) = match v.split_once(',') {
                    Some((a, b)) => (a, Some(b)),
                    None => (v.as_str(), None),
                };
                let range = match sel {
                    Some(r) => {
                        let (a, b) = r.split_once('-').ok_or_else(bad)?;
                        a.trim().parse().map_err(|_| bad())?..b.trim().parse().map_err(|_| bad())?
                    }
                    None => 0..0,
                };
                opts.edit_text = Some((ix.trim().parse().map_err(|_| bad())?, range));
            }
            "--spacing-dialog" => opts.spacing_dialog = true,
            "--dialog" => {
                let v = value("--dialog")?;
                if v != "page-setup" && v != "custom-margins" {
                    return Err(format!("--dialog needs page-setup or custom-margins, got {v:?}"));
                }
                opts.dialog = Some(v);
            }
            "--menu" => opts.menu = Some(value("--menu")?),
            "--select" => {
                let v = value("--select")?;
                for part in v.split(',') {
                    opts.select.push(
                        part.trim()
                            .parse()
                            .map_err(|_| format!("--select needs shape indices, got {v:?}"))?,
                    );
                }
            }
            "--scale" => {
                let v = value("--scale")?;
                opts.scale = Some(v.trim().parse().map_err(|_| format!("--scale needs a number, got {v:?}"))?);
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
            "--open-group" => opts.open_group = Some(value("--open-group")?),
            "--print-layout" => opts.print_layout = Some(value("--print-layout")?),
            "--print-sheet" => {
                let v = value("--print-sheet")?;
                opts.print_sheet = Some(v.parse::<usize>().map_err(|_| format!("bad sheet number {v:?}"))?);
            }
            "--theme" => opts.theme = Some(value("--theme")?),
            "--accent" => opts.accent = Some(value("--accent")?),
            "--demo" => opts.demo = true,
            "--master" => opts.master = true,
            "--tab" => opts.tab = Some(value("--tab")?),
            "--page" => {
                let v = value("--page")?;
                opts.page = v
                    .parse::<usize>()
                    .map_err(|_| format!("bad page number {v:?}"))?;
            }
            "-h" | "--help" => return Err(USAGE.to_owned()),
            s if s.starts_with("--") => return Err(format!("unknown option {s}\n{USAGE}")),
            s => opts.open = Some(PathBuf::from(s)),
        }
    }
    Ok(opts)
}

/// The version shown to the user and compared by update checks: the `KERNPAGE_VERSION` the installer
/// build was given (package.ps1 -OverrideVersion), else the crate's own version.
pub(crate) const fn app_version() -> &'static str {
    match option_env!("KERNPAGE_VERSION") {
        Some(v) => v,
        None => env!("CARGO_PKG_VERSION"),
    }
}

/// The `--version` handshake line: `<name> <version>`, nothing else.
fn version_line() -> String {
    format!("kernpage {}", app_version())
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
    if let Some(g) = &opts.open_group {
        ribbon_fit::set_debug_open(g);
    }
    if opts.screenshot.is_none() && !opts.demo {
        kernpage_core::autorecover::install_panic_hook();
    }
    let (w, h) = opts.window_size.unwrap_or((1280.0, 800.0));
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([w, h])
        .with_min_inner_size([w.min(800.0), h.min(500.0)])
        .with_title("KernPage")
        .with_icon(window_icon());
    let native = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "KernPage",
        native,
        Box::new(move |cc| Ok(Box::new(App::new(cc, opts)))),
    );
    if result.is_ok() {
        end_without_dll_teardown();
    }
    result
}

/// End the process at once after a normal close, skipping the unloading of system DLLs.
///
/// After printing, Windows' PDF renderer (`Windows.Data.Pdf`, used by `printing.rs`) releases
/// its Direct3D device while DLLs unload at exit, and some graphics drivers never return from
/// that: on 2026-11-25 a closed KernPage kept one core busy for hours inside the Intel driver
/// (`igd10um64xe.dll`, called from `d3d11.dll` and `Windows.Data.Pdf.dll` during exit), with no
/// window left. Everything KernPage owns is finished by now (the window is closed, files are
/// written and AutoRecover is cleaned up), so ending here loses nothing.
fn end_without_dll_teardown() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    #[cfg(windows)]
    // SAFETY: TerminateProcess on our own process handle; nothing runs after it.
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        TerminateProcess(GetCurrentProcess(), 0);
    }
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
        assert_eq!(super::version_line(), format!("kernpage {}", super::app_version()));
    }
}
