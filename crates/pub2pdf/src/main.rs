use pub2pdf::{convert, Options, FONT_DIRS_ENV};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: pub2pdf [options] IN.pub OUT.pdf

Renders every page of a Publisher .pub file to PDF.
  --outlines / --no-outlines  draw text box outlines in light gray (default: on)
  --font-dir DIR              look for fonts in DIR (repeatable). Without it the
                              KERNPAGE_FONT_DIRS environment variable (path-separated)
                              is used, else C:\\Windows\\Fonts or /mnt/c/Windows/Fonts
Text boxes, tables, drawn shapes (rectangle, ellipse, line), PNG/JPEG pictures and WMF/EMF clip art are
drawn. Fonts found in the font directories are embedded as subsets (unless their
embedding permission forbids it); other fonts fall back to the standard Helvetica,
Times or Courier. Tables use equal-size cells. Text overflow is not clipped.";

fn main() -> ExitCode {
    let mut opts = Options::default();
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--outlines" => opts.outlines = true,
            "--no-outlines" => opts.outlines = false,
            "--font-dir" => match args.next() {
                Some(d) => dirs.push(d.into()),
                None => {
                    eprintln!("pub2pdf: --font-dir needs a directory\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            a if a.starts_with("--font-dir=") => dirs.push(a["--font-dir=".len()..].into()),
            a if a.starts_with("--") => {
                eprintln!("pub2pdf: unknown option {a}\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => files.push(arg.into()),
        }
    }
    if !dirs.is_empty() {
        opts.font_dirs = dirs;
    } else if std::env::var_os(FONT_DIRS_ENV).is_none() && opts.font_dirs.is_empty() {
        eprintln!("pub2pdf: no font directory found; using standard fonts");
    }
    let [input, output] = files.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match convert(input, output, &opts) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pub2pdf: {e}");
            ExitCode::FAILURE
        }
    }
}
