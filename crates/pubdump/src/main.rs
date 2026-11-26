use pub_format::{Document, Shape};
use std::path::PathBuf;
use std::process::ExitCode;

const EMU_PER_INCH: f64 = 914_400.0;

fn inches(emu: i64) -> f64 {
    emu as f64 / EMU_PER_INCH
}

fn print_shape(s: &Shape, page_w_emu: u32, page_h_emu: u32) {
    // Anchors are measured from the page centre; convert to the top-left corner.
    let (cx, cy) = (
        inches(page_w_emu.into()) / 2.0,
        inches(page_h_emu.into()) / 2.0,
    );
    let (x0, y0) = (
        cx + inches(s.left_emu.into()),
        cy + inches(s.top_emu.into()),
    );
    let (x1, y1) = (
        cx + inches(s.right_emu.into()),
        cy + inches(s.bottom_emu.into()),
    );
    println!(
        "  shape #{} (kind {}): x {x0:.2}-{x1:.2} in, y {y0:.2}-{y1:.2} in",
        s.object_number, s.kind
    );
    match (s.width_emu, s.height_emu) {
        (Some(w), Some(h)) => println!(
            "    size {:.2} x {:.2} in",
            inches(w.into()),
            inches(h.into())
        ),
        _ => println!("    size unknown"),
    }
    match s.story {
        Some(n) => println!("    story {n}"),
        None => println!("    story none"),
    }
    if let Some(g) = s.geometry {
        println!("    geometry {g:?}, fill {:?}, line {:?}", s.fill, s.line);
    }
    if let Some(t) = &s.table {
        println!("    table {}x{}: {:?}", t.rows, t.columns, t.cells);
    }
    if let Some(pic) = &s.picture {
        println!("    picture: {:?}, {} bytes", pic.format, pic.bytes.len());
    }
    for (i, para) in s.paragraphs.iter().enumerate() {
        println!("    paragraph {} ({:?}):", i + 1, para.alignment);
        for r in &para.runs {
            let mut attrs = vec![format!("{} {:.1}pt", r.font, r.size_pt)];
            if r.bold {
                attrs.push("bold".into());
            }
            if r.italic {
                attrs.push("italic".into());
            }
            if r.underline {
                attrs.push("underline".into());
            }
            if let Some(c) = r.color {
                attrs.push(format!("color {c:?}"));
            }
            println!("      [{}] {:?}", attrs.join(", "), r.text);
        }
    }
}

fn print_document(doc: &Document) {
    println!(
        "page size: {:.2} x {:.2} in",
        inches(doc.page_width_emu.into()),
        inches(doc.page_height_emu.into())
    );
    println!("pages: {}", doc.pages.len());
    for (i, page) in doc.pages.iter().enumerate() {
        println!(
            "page {} (object {}): {} shape(s)",
            i + 1,
            page.object_number,
            page.shapes.len()
        );
        for s in &page.shapes {
            print_shape(s, doc.page_width_emu, doc.page_height_emu);
        }
    }
    println!("stories: {}", doc.stories.len());
    println!("text:");
    println!("{}", doc.text.replace("\r\n", "¶\n").replace('\r', "¶\n"));
    println!("fonts:");
    for f in &doc.fonts {
        println!("  {f}");
    }
}

fn main() -> ExitCode {
    let mut json = false;
    let mut path: Option<PathBuf> = None;
    for arg in std::env::args().skip(1) {
        if arg == "--json" {
            json = true;
        } else {
            path = Some(arg.into());
        }
    }
    let Some(path) = path else {
        eprintln!("usage: pubdump [--json] FILE.pub");
        return ExitCode::from(2);
    };
    match pub_format::read(&path) {
        Ok(doc) if json => {
            println!("{}", serde_json::to_string_pretty(&doc).expect("serialize"));
            ExitCode::SUCCESS
        }
        Ok(doc) => {
            print_document(&doc);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pubdump: {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}
