//! Output: Save as PDF (File > Save & Send, Save As) and Print (File > Print).
//!
//! Both build the PDF from the open publication, including unsaved edits, with
//! `kernpage_core::export_pdf` (the same renderer as the `pub2pdf` tool).
//!
//! File > Print prints directly (see `printing`): the Print page offers the installed printers
//! (default preselected), Printer Properties, copies, which pages (all, the current page or a
//! custom list such as `1-3, 5`) and collation. Clicking Print builds the PDF, then a worker
//! thread draws each chosen page at the printer's resolution and sends it with GDI; the page
//! shows "Printing page N of M..." and a failure comes back as the error box. "Open as PDF
//! instead" keeps the older way: the PDF is created and opened in the system's PDF viewer.

use crate::app::App;
use crate::icons::{self, Icon};
use crate::nav::paint_thumb;
use crate::backstage::{green_link, page_title, section_title, selected_row};
use crate::printing::{self, PageScope, PrintJob, PrinterList, Shared};
use eframe::egui::{self, Color32, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::path::{Path, PathBuf};

/// What the Print page has chosen so far.
pub struct PrintState {
    pub printers: Option<PrinterList>,
    pub printer: Option<String>,
    pub copies: u32,
    pub scope: PageScope,
    pub range: String,
    pub collate: bool,
    /// The printer's own settings from Printer Properties, and the printer they belong to.
    pub devmode: Option<(String, Vec<u8>)>,
    pub running: Option<Shared>,
}

impl Default for PrintState {
    fn default() -> Self {
        PrintState {
            printers: None,
            printer: None,
            copies: 1,
            scope: PageScope::All,
            range: String::new(),
            collate: true,
            devmode: None,
            running: None,
        }
    }
}

pub fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

/// Open `path` with the system's default program for it.
pub(crate) fn open_with_system(path: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    {
        // Tests never start Explorer or a PDF viewer: the request is recorded instead.
        test_os::OPENED.with(|o| o.borrow_mut().push(path.to_path_buf()));
        Ok(())
    }
    #[cfg(not(test))]
    {
        open_with_system_now(path)
    }
}

#[cfg(not(test))]
fn open_with_system_now(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut cmd = {
        // explorer.exe opens a file with its default program and shows no console window.
        let mut c = std::process::Command::new(kernpage_core::sysexe::windows_exe("explorer.exe"));
        c.arg(path);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    cmd.spawn().map(|_| ())
}

fn file_stem(name: &str) -> String {
    let stem = name.trim_end_matches(".pub").trim_end_matches(".kern").trim();
    let clean: String = stem
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    if clean.is_empty() {
        "Publication".to_owned()
    } else {
        clean
    }
}

impl App {
    fn pdf_bytes(&mut self) -> Option<Vec<u8>> {
        self.commit_edit();
        let ed = self.editor.as_ref()?;
        let doc = ed.doc();
        let dirs = pub2pdf::default_font_dirs();
        // A broken picture or font must never take the editor down.
        let doc = doc.clone();
        match std::panic::catch_unwind(move || kernpage_core::export_pdf(&doc, dirs)) {
            Ok(b) => Some(b),
            Err(_) => {
                self.error = Some(
                    "KernPage cannot create the PDF.\n\nSomething in this publication could not be drawn."
                        .to_owned(),
                );
                None
            }
        }
    }

    /// Write the open publication to `target` as a PDF.
    pub fn export_pdf_to(&mut self, target: &Path) {
        let Some(bytes) = self.pdf_bytes() else {
            return;
        };
        let shown = target
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        self.error = Some(match std::fs::write(target, bytes) {
            Ok(()) => format!("The PDF \"{shown}\" was created.\n\n{}", target.display()),
            Err(e) => format!("KernPage cannot save \"{shown}\".\n\n{e}"),
        });
    }

    /// Choose a file name and create the PDF (Save & Send > Create a PDF).
    pub fn create_pdf(&mut self) {
        let Some(ed) = &self.editor else { return };
        let name = format!("{}.pdf", file_stem(&ed.doc().name));
        let picked = self.dialogs.pick_save(&crate::filedialog::SaveRequest {
            title: "Publish as PDF".to_owned(),
            default_dir: self.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf),
            default_name: name,
            filters: vec![crate::filedialog::Filter::new("PDF (*.pdf)", &["pdf"])],
        });
        if let Some(mut p) = picked {
            if p.extension().is_none() {
                p.set_extension("pdf");
            }
            self.export_pdf_to(&p);
        }
    }

    /// "Open as PDF instead": create the PDF in the temporary folder and open it in the PDF viewer.
    pub fn open_as_pdf(&mut self) {
        let Some(bytes) = self.pdf_bytes() else {
            return;
        };
        let name = self
            .editor
            .as_ref()
            .map_or_else(|| "Publication".to_owned(), |e| file_stem(&e.doc().name));
        #[cfg(test)]
        let dir: PathBuf = test_os::print_dir();
        #[cfg(not(test))]
        let dir: PathBuf = std::env::temp_dir().join("KernPage");
        let path = dir.join(format!("{name}.pdf"));
        let result = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(&path, bytes))
            .and_then(|()| open_with_system(&path));
        if let Err(e) = result {
            self.error = Some(format!(
                "KernPage cannot open the PDF.\n\n{e}\n\nUse Save & Send to create a PDF and print it from there."
            ));
        }
    }

    /// The pages the current settings mean (0-based), or the message to show.
    fn print_pages(&self) -> Result<Vec<usize>, String> {
        let total = self.editor.as_ref().map_or(0, |e| e.doc().pages.len());
        match self.print.scope {
            PageScope::All => Ok((0..total).collect()),
            PageScope::Current => Ok(vec![self.page.min(total.saturating_sub(1))]),
            PageScope::Custom => printing::parse_page_ranges(&self.print.range, total),
        }
    }

    /// Click on Print: build the PDF and hand the job to a worker thread.
    pub fn start_print(&mut self, ctx: &egui::Context) {
        if self.print.running.is_some() {
            return;
        }
        let Some(printer) = self.print.printer.clone() else {
            self.error = Some(
                "There is no printer to print to.\n\nInstall a printer in Windows, or use \"Open as PDF instead\"."
                    .to_owned(),
            );
            return;
        };
        let pages = match self.print_pages() {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let Some(bytes) = self.pdf_bytes() else {
            return;
        };
        let Some(ed) = self.editor.as_ref() else { return };
        let doc = ed.doc();
        let first = pages.first().and_then(|&p| doc.pages.get(p));
        let job = PrintJob {
            printer: printer.clone(),
            copies: self.print.copies.clamp(1, 999),
            collate: self.print.collate,
            landscape: first.is_some_and(|p| p.width_emu > p.height_emu),
            doc_name: file_stem(&doc.name),
            devmode: self.print.devmode.as_ref().filter(|(p, _)| *p == printer).map(|(_, d)| d.clone()),
            pages,
        };
        let ctx = ctx.clone();
        self.print.running = Some(printing::submit(job, bytes, move || ctx.request_repaint()));
    }

    /// Each frame: notice that a print job has finished, and report its error.
    pub fn poll_print(&mut self, ctx: &egui::Context) {
        let Some(run) = self.print.running.clone() else { return };
        let Ok(p) = run.lock() else {
            self.print.running = None;
            return;
        };
        if p.finished {
            if let Some(e) = &p.error {
                self.error = Some(format!("KernPage cannot print.\n\n{e}"));
            }
            drop(p);
            self.print.running = None;
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
    }

    /// File > Print: settings column (Print button, printer, settings) and the preview.
    pub fn print_page(&mut self, ui: &mut Ui) {
        page_title(ui, "Print");
        let Some(ed) = self.editor.as_ref() else {
            return;
        };
        let pages = ed.doc().pages.len();
        let size = ed.doc().pages.first().map(|p| (p.width_emu, p.height_emu));
        self.print_sheet = self.print_sheet.min(pages.saturating_sub(1));
        if self.print.printers.is_none() {
            let list = printing::list_printers();
            if self.print.printer.is_none() {
                self.print.printer = list.default.clone().or_else(|| list.names.first().cloned());
            }
            self.print.printers = Some(list);
        }
        let names = self.print.printers.as_ref().map(|l| l.names.clone()).unwrap_or_default();
        let range_problem = (self.print.scope == PageScope::Custom)
            .then(|| self.print_pages().err())
            .flatten();
        let mut print = false;
        let mut as_pdf = false;
        let mut props = false;
        let soon = "Not available yet";
        let busy = self.print.running.clone();
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(300.0, 640.0), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal_top(|ui| {
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(72.0, 74.0), Sense::click());
                        #[cfg(test)]
                        crate::backstage::seen("print:Print".to_owned(), rect, busy.is_none(), false);
                        let p = ui.painter();
                        p.rect_filled(
                            rect,
                            0.0,
                            if resp.hovered() { crate::theme::hover_bg() } else { crate::theme::card() },
                        );
                        p.rect_stroke(rect, 0.0, Stroke::new(1.0, crate::theme::frame()), StrokeKind::Inside);
                        icons::paint(
                            p,
                            egui::Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 26.0), Vec2::splat(30.0)),
                            Icon::Print,
                            crate::theme::text(),
                        );
                        p.text(
                            rect.center_bottom() - Vec2::new(0.0, 12.0),
                            egui::Align2::CENTER_CENTER,
                            "Print",
                            egui::FontId::proportional(13.0),
                            crate::theme::text(),
                        );
                        if resp.on_hover_text("Print (Ctrl+P)").clicked() {
                            print = true;
                        }
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Copies of print job:").color(crate::theme::text()));
                            ui.add(egui::DragValue::new(&mut self.print.copies).range(1..=999).speed(0.1));
                        });
                    });
                    if let Some(run) = &busy {
                        if let Ok(p) = run.lock() {
                            let total = p.sheets.max(1);
                            ui.label(
                                egui::RichText::new(format!("Printing page {} of {total}\u{2026}", p.sheet.clamp(1, total)))
                                    .color(crate::theme::accent_text()),
                            );
                        }
                    }
                    ui.add_space(10.0);
                    section_title(ui, "Printer");
                    if names.is_empty() {
                        ui.label(egui::RichText::new("No printer is installed").color(crate::theme::disabled()));
                    } else {
                        egui::ComboBox::from_id_salt("print_printer")
                            .width(290.0)
                            .selected_text(self.print.printer.clone().unwrap_or_default())
                            .show_ui(ui, |ui| {
                                for n in &names {
                                    ui.selectable_value(&mut self.print.printer, Some(n.clone()), n);
                                }
                            });
                    }
                    if self.print.printer.is_some() {
                        props = green_link(ui, "Printer Properties");
                    } else {
                        ui.add_enabled(false, egui::Label::new(egui::RichText::new("Printer Properties").underline()));
                    }
                    ui.add_space(8.0);
                    section_title(ui, "Settings");
                    egui::ComboBox::from_id_salt("print_pages")
                        .width(290.0)
                        .selected_text(self.print.scope.label())
                        .show_ui(ui, |ui| {
                            for s in [PageScope::All, PageScope::Current, PageScope::Custom] {
                                ui.selectable_value(&mut self.print.scope, s, s.label());
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.label("Pages:");
                        let custom = self.print.scope == PageScope::Custom;
                        ui.add_enabled(
                            custom,
                            egui::TextEdit::singleline(&mut self.print.range)
                                .hint_text(format!("1-{pages}"))
                                .desired_width(190.0),
                        )
                        .on_disabled_hover_text("Choose Custom Print to type the pages");
                    });
                    if let Some(e) = &range_problem {
                        let first = e.lines().next().unwrap_or_default();
                        ui.label(egui::RichText::new(first).color(Color32::from_rgb(190, 40, 40)).size(12.0));
                    }
                    egui::ComboBox::from_id_salt("print_collate")
                        .width(290.0)
                        .selected_text(if self.print.collate { "Collated" } else { "Uncollated" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.print.collate, true, "Collated");
                            ui.selectable_value(&mut self.print.collate, false, "Uncollated");
                        });
                    let landscape = printing::is_landscape(size);
                    ui.label(
                        egui::RichText::new(if landscape {
                            "Landscape Orientation (from the publication)"
                        } else {
                            "Portrait Orientation (from the publication)"
                        })
                        .color(crate::theme::disabled()),
                    );
                    ui.add_space(4.0);
                    let paper = size.map_or_else(String::new, |(w, h)| {
                        format!("{} x {}\"", inches(w), inches(h))
                    });
                    setting(ui, "Publication page size", &paper, Icon::Size, soon);
                    setting(ui, "Print One Sided", "Only print on one side of the sheet", Icon::Page, soon);
                    setting(ui, "Composite RGB", "Print as RGB color publication.", Icon::Palette, soon);
                    let mut keep = false;
                    ui.add_enabled(false, egui::Checkbox::new(&mut keep, "Save settings with publication"));
                    ui.add_space(8.0);
                    as_pdf = green_link(ui, "Open as PDF instead");
                });
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                let Some(ed) = self.editor.as_ref() else {
                    return;
                };
                let Some(page) = ed.doc().pages.get(self.print_sheet) else {
                    return;
                };
                let avail = ui.available_size();
                let max_h = (avail.y - 90.0).clamp(200.0, 620.0);
                let max_w = (avail.x - 60.0).clamp(200.0, 560.0);
                let aspect = page.height_emu as f32 / page.width_emu.max(1) as f32;
                let (mut w, mut h) = (max_w, max_w * aspect);
                if h > max_h {
                    h = max_h;
                    w = h / aspect;
                }
                let (outer, _) = ui.allocate_exact_size(Vec2::new(max_w, 22.0), Sense::hover());
                let p = ui.painter();
                let mid = outer.center();
                let line = Stroke::new(1.0, crate::theme::frame());
                p.text(mid, egui::Align2::CENTER_CENTER, format!("{}\"", inches(page.width_emu)), egui::FontId::proportional(13.0), crate::theme::disabled());
                p.line_segment([egui::Pos2::new(outer.min.x, mid.y), egui::Pos2::new(mid.x - 26.0, mid.y)], line);
                p.line_segment([egui::Pos2::new(mid.x + 26.0, mid.y), egui::Pos2::new(outer.max.x, mid.y)], line);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
                paint_thumb(ui, rect, ed.doc(), self.print_sheet, &self.fonts, &mut self.pics);
                ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, crate::theme::text()), StrokeKind::Outside);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if nav_arrow(ui, 2, self.print_sheet > 0) {
                        self.print_sheet -= 1;
                    }
                    ui.label(egui::RichText::new(format!("{}  of {pages}", self.print_sheet + 1)).color(crate::theme::text()));
                    if nav_arrow(ui, 1, self.print_sheet + 1 < pages) {
                        self.print_sheet += 1;
                    }
                });
            });
        });
        if props {
            if let Some(p) = self.print.printer.clone() {
                let landscape = printing::is_landscape(size);
                let cur = self.print.devmode.as_ref().filter(|(n, _)| *n == p).map(|(_, d)| d.as_slice());
                if let Some(d) = printing::printer_properties(&p, cur, landscape) {
                    self.print.devmode = Some((p, d));
                }
            }
        }
        if as_pdf {
            self.open_as_pdf();
        }
        if print {
            let ctx = ui.ctx().clone();
            self.start_print(&ctx);
        }
    }

    /// File > Export: the export types and the Create a PDF page.
    pub fn export_page(&mut self, ui: &mut Ui) {
        page_title(ui, "Export");
        let mut create = false;
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(390.0, 500.0), |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    export_row(ui, "Create a PDF", Icon::Print, true);
                    export_row(ui, "Publish HTML", Icon::Globe, false);
                    export_row(ui, "Change File Type", Icon::Rename, false);
                    ui.add_space(10.0);
                    section_title(ui, "Package for Printing");
                    export_row(ui, "Save for Photo Printing", Icon::Picture, false);
                    export_row(ui, "Save for a Commercial Printer", Icon::Palette, false);
                    export_row(ui, "Save for Another Computer", Icon::Save, false);
                });
            });
            let r = ui.min_rect();
            ui.painter().line_segment(
                [egui::Pos2::new(r.right() + 8.0, r.top()), egui::Pos2::new(r.right() + 8.0, r.top() + 520.0)],
                Stroke::new(1.0, crate::theme::line()),
            );
            ui.add_space(20.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Create a PDF").size(24.0).color(crate::theme::accent_text()));
                ui.add_space(4.0);
                for t in [
                    "Keeps your fonts, pictures and layout exactly as they are",
                    "Hard for others to change",
                    "Opens in any PDF reader",
                ] {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                        ui.painter().rect_filled(egui::Rect::from_center_size(r.center(), Vec2::splat(4.0)), 0.0, crate::theme::text());
                        ui.label(egui::RichText::new(t).color(crate::theme::text()).size(13.0));
                    });
                }
                ui.add_space(8.0);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(78.0, 80.0), Sense::click());
                #[cfg(test)]
                crate::widgets::probe::record(crate::widgets::probe::Seen {
                    label: "export:Create PDF".to_owned(),
                    tip: String::new(),
                    rect,
                    enabled: true,
                    selected: false,
                });
                let p = ui.painter();
                p.rect_filled(rect, 0.0, if resp.hovered() { crate::theme::hover_bg() } else { crate::theme::card() });
                p.rect_stroke(rect, 0.0, Stroke::new(1.0, crate::theme::frame()), StrokeKind::Inside);
                icons::paint(
                    p,
                    egui::Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 24.0), Vec2::splat(28.0)),
                    Icon::Print,
                    crate::theme::text(),
                );
                p.text(rect.center_bottom() - Vec2::new(0.0, 22.0), egui::Align2::CENTER_CENTER, "Create", egui::FontId::proportional(13.0), crate::theme::text());
                p.text(rect.center_bottom() - Vec2::new(0.0, 8.0), egui::Align2::CENTER_CENTER, "PDF", egui::FontId::proportional(13.0), crate::theme::text());
                if resp.clicked() {
                    create = true;
                }
            });
        });
        if create {
            self.create_pdf();
        }
    }
}

fn inches(emu: i64) -> String {
    let s = format!("{:.2}", emu as f64 / 914_400.0);
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A disabled combo-style row: icon, title, description (the 2021 Print settings).
fn setting(ui: &mut Ui, title: &str, desc: &str, icon: Icon, tip: &str) {
    let w = ui.available_width().min(290.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 38.0), Sense::hover());
    #[cfg(test)]
    crate::backstage::seen(format!("setting:{title}"), rect, false, false);
    let p = ui.painter();
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, crate::theme::frame()), StrokeKind::Inside);
    icons::paint(
        p,
        egui::Rect::from_center_size(rect.min + Vec2::new(20.0, 19.0), Vec2::splat(20.0)),
        icon,
        crate::theme::disabled(),
    );
    let (ty, dy) = if desc.is_empty() { (19.0, 0.0) } else { (11.0, 27.0) };
    p.text(rect.min + Vec2::new(40.0, ty), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(13.0), crate::theme::text());
    if dy > 0.0 {
        p.text(rect.min + Vec2::new(40.0, dy), egui::Align2::LEFT_CENTER, desc, egui::FontId::proportional(12.0), crate::theme::disabled());
    }
    tri(p, rect.max - Vec2::new(14.0, 19.0), 0, crate::theme::disabled());
    resp.on_hover_text(tip);
    ui.add_space(4.0);
}

fn export_row(ui: &mut Ui, label: &str, icon: Icon, selected: bool) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(384.0, 56.0), Sense::hover());
    #[cfg(test)]
    crate::backstage::seen(format!("exportrow:{label}"), rect, selected, selected);
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 0.0, selected_row());
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, crate::theme::accent()), StrokeKind::Inside);
    }
    let col = if selected { crate::theme::text() } else { crate::theme::disabled() };
    icons::paint(p, egui::Rect::from_center_size(rect.min + Vec2::new(28.0, 28.0), Vec2::splat(28.0)), icon, col);
    p.text(rect.min + Vec2::new(68.0, 28.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(15.0), if selected { crate::theme::text() } else { crate::theme::disabled() });
    if !selected {
        resp.on_hover_text(crate::widgets::COMING_SOON);
    }
}

/// A small filled triangle: dir 0 down, 1 right, 2 left.
fn tri(p: &egui::Painter, c: egui::Pos2, dir: u8, col: Color32) {
    let a = 4.0;
    let pts = match dir {
        0 => vec![c + Vec2::new(-a, -a * 0.5), c + Vec2::new(a, -a * 0.5), c + Vec2::new(0.0, a * 0.5)],
        1 => vec![c + Vec2::new(-a * 0.5, -a), c + Vec2::new(-a * 0.5, a), c + Vec2::new(a * 0.5, 0.0)],
        _ => vec![c + Vec2::new(a * 0.5, -a), c + Vec2::new(a * 0.5, a), c + Vec2::new(-a * 0.5, 0.0)],
    };
    p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
}

fn nav_arrow(ui: &mut Ui, dir: u8, enabled: bool) -> bool {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), sense);
    #[cfg(test)]
    crate::backstage::seen(format!("printnav:{}", if dir == 2 { "prev" } else { "next" }), rect, enabled, false);
    tri(ui.painter(), rect.center(), dir, if enabled { crate::theme::text() } else { crate::theme::disabled() });
    resp.clicked()
}

/// Test seam for the operating system: what would have been opened, and where Print writes.
#[cfg(test)]
pub mod test_os {
    use std::cell::RefCell;
    use std::path::PathBuf;
    thread_local! {
        pub(super) static OPENED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
        static PRINT_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }
    pub fn set_print_dir(dir: PathBuf) {
        PRINT_DIR.with(|d| *d.borrow_mut() = Some(dir));
    }
    pub fn print_dir() -> PathBuf {
        PRINT_DIR
            .with(|d| d.borrow().clone())
            .unwrap_or_else(|| std::env::temp_dir().join("KernPage test"))
    }
    /// Everything the app asked the system to open since the last call.
    pub fn take_opened() -> Vec<PathBuf> {
        OPENED.with(|o| std::mem::take(&mut *o.borrow_mut()))
    }
}
