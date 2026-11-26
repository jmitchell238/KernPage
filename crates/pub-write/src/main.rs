//! `pubpatch rewrite IN OUT` / `pubpatch move IN OUT --page N --shape K --left-pt ..`
//! Page and shape indexes are zero-based, as in `pub_format::Document::pages[..].shapes[..]`.
//! `--left-pt`/`--top-pt` are measured from the page's top-left corner.

use std::process::ExitCode;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pubpatch: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage: pubpatch rewrite IN OUT\n       pubpatch text IN OUT --story K --text \"para1\\npara2\"\n       pubpatch table IN OUT --story K --cells \"c1|c2|..\" (\\n = new paragraph)\n       pubpatch move IN OUT --page N --shape K --left-pt L --top-pt T --width-pt W --height-pt H";

fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("rewrite") if args.len() == 3 => {
            let out = pub_write::rewrite(&read(&args[1])?).map_err(|e| e.to_string())?;
            std::fs::write(&args[2], out).map_err(|e| e.to_string())
        }
        Some("text") if args.len() >= 3 => {
            let value = |name: &str| -> Result<&String, String> {
                let i = args
                    .iter()
                    .position(|a| a == name)
                    .ok_or_else(|| format!("missing {name}\n{USAGE}"))?;
                args.get(i + 1)
                    .ok_or_else(|| format!("{name} needs a value"))
            };
            let story = value("--story")?
                .parse::<usize>()
                .map_err(|_| "--story needs a number".to_string())?;
            let out = pub_write::set_story_text(
                &read(&args[1])?,
                story,
                &value("--text")?.replace("\\n", "\n"),
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(&args[2], out).map_err(|e| e.to_string())
        }
        Some("table") if args.len() >= 3 => {
            let value = |name: &str| -> Result<&String, String> {
                let i = args
                    .iter()
                    .position(|a| a == name)
                    .ok_or_else(|| format!("missing {name}\n{USAGE}"))?;
                args.get(i + 1)
                    .ok_or_else(|| format!("{name} needs a value"))
            };
            let story = value("--story")?
                .parse::<usize>()
                .map_err(|_| "--story needs a number".to_string())?;
            let cells: Vec<String> = value("--cells")?
                .split('|')
                .map(|c| c.replace("\\n", "\n"))
                .collect();
            let out = pub_write::set_table_text(&read(&args[1])?, story, &cells)
                .map_err(|e| e.to_string())?;
            std::fs::write(&args[2], out).map_err(|e| e.to_string())
        }
        Some("move") if args.len() >= 3 => {
            let flag = |name: &str| -> Result<f64, String> {
                let i = args
                    .iter()
                    .position(|a| a == name)
                    .ok_or_else(|| format!("missing {name}\n{USAGE}"))?;
                args.get(i + 1)
                    .and_then(|v| v.parse::<f64>().ok())
                    .ok_or_else(|| format!("{name} needs a number"))
            };
            let input = read(&args[1])?;
            let doc = pub_format::read_bytes(&input).map_err(|e| e.to_string())?;
            // Flags are page-top-left points; the file stores page-centre-relative EMU.
            let left =
                flag("--left-pt")? - f64::from(doc.page_width_emu) / 2.0 / pub_write::EMU_PER_POINT;
            let top =
                flag("--top-pt")? - f64::from(doc.page_height_emu) / 2.0 / pub_write::EMU_PER_POINT;
            let out = pub_write::set_shape_frame_pt(
                &input,
                flag("--page")? as usize,
                flag("--shape")? as usize,
                left,
                top,
                flag("--width-pt")?,
                flag("--height-pt")?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(&args[2], out).map_err(|e| e.to_string())
        }
        _ => Err(USAGE.into()),
    }
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}
