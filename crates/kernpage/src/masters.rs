//! Master pages in the editor: the Master Page view (View > Master Page, Ctrl+M) with its own
//! ribbon tab, the Add / Duplicate / Rename and Apply dialogs, the Master Pages drop-down of
//! Page Design, and Insert > Page Number and Header / Footer, which put text boxes on the
//! master. The model and its commands are in `kernpage_core` (`MasterPage`, `Command::AddMaster`
//! and friends); the view edits a master through the pseudo page index `master_surface`, so
//! every shape command works on it unchanged.

use crate::app::{App, Tab};
use crate::fontmgr::FontMgr;
use crate::icons::Icon;
use crate::render::PictureCache;
use crate::widgets::{group, Btn, ACCENT_COLOR};
use eframe::egui::{self, Key, Popup, Stroke, Ui};
use kernpage_core::{
    master_surface, master_surface_parts, Align, Command, Document, Field, MasterPage, Paragraph,
    Rect, Run, Shape,
};

/// The Master Page view is open.
#[derive(Clone, Copy, Debug)]
pub struct MasterView {
    /// The page and tab to go back to on Close Master Page.
    pub return_page: usize,
    pub return_tab: Tab,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MasterDialogKind {
    Add,
    Duplicate,
    Rename,
}

/// The Add Master Page / Duplicate Master Page / Rename Master Page dialog: a one-character
/// Page ID, a description of at most 40 characters and (for Add and Duplicate) a two-page
/// check box.
#[derive(Clone, Debug)]
pub struct MasterDialog {
    pub kind: MasterDialogKind,
    /// The master being duplicated or renamed.
    pub master: usize,
    pub id: String,
    pub name: String,
    pub description: String,
    pub two_page: bool,
    pub fresh: bool,
}

impl MasterDialog {
    /// The command OK runs, or what is wrong with the input.
    pub fn command(&self, masters: &[MasterPage]) -> Result<Command, String> {
        let mut chars = self.id.chars();
        let id = match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_whitespace() && !c.is_control() => c,
            _ => return Err("The Page ID must be one letter or digit.".to_owned()),
        };
        let used = masters
            .iter()
            .enumerate()
            .any(|(i, m)| m.id == id && !(self.kind == MasterDialogKind::Rename && i == self.master));
        if used {
            return Err(format!("Another master page already has the ID {id}."));
        }
        let description: String = self.description.chars().take(MasterPage::MAX_DESCRIPTION).collect();
        Ok(match self.kind {
            MasterDialogKind::Add => Command::AddMaster {
                copy_of: None,
                id: Some(id),
                description,
                two_page: self.two_page,
            },
            MasterDialogKind::Duplicate => Command::AddMaster {
                copy_of: Some(self.master),
                id: Some(id),
                description,
                two_page: self.two_page,
            },
            MasterDialogKind::Rename => Command::SetMasterInfo {
                master: self.master,
                id,
                name: self.name.clone(),
                description,
            },
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ApplyScope {
    Current,
    All,
    Range,
}

/// The Apply Master Page dialog: which master, and to which pages.
#[derive(Clone, Debug)]
pub struct ApplyDialog {
    pub master: usize,
    pub scope: ApplyScope,
    /// 1-based page numbers of the range, as typed.
    pub from: String,
    pub to: String,
    /// The page "current page" means.
    pub current: usize,
}

impl ApplyDialog {
    /// The 0-based pages to apply to, or what is wrong with the range.
    pub fn pages(&self, count: usize) -> Result<Vec<usize>, String> {
        match self.scope {
            ApplyScope::Current => Ok(vec![self.current.min(count.saturating_sub(1))]),
            ApplyScope::All => Ok((0..count).collect()),
            ApplyScope::Range => {
                let num = |s: &str| s.trim().parse::<usize>().ok().filter(|n| (1..=count).contains(n));
                match (num(&self.from), num(&self.to)) {
                    (Some(a), Some(b)) if a <= b => Ok((a - 1..b).collect()),
                    (Some(_), Some(_)) => Err("The first page must not come after the last.".to_owned()),
                    _ => Err(format!("Enter page numbers from 1 to {count}.")),
                }
            }
        }
    }
}

/// What a click in the Pages pane's master list asks for.
#[derive(Clone, Copy, Debug)]
pub enum MasterAct {
    Go(usize, bool),
    Add,
    Duplicate(usize),
    Rename(usize),
    Delete(usize),
}

/// Where Insert > Page Number puts the number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NumberSpot {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl NumberSpot {
    pub const ALL: [NumberSpot; 6] = [
        NumberSpot::TopLeft,
        NumberSpot::TopCenter,
        NumberSpot::TopRight,
        NumberSpot::BottomLeft,
        NumberSpot::BottomCenter,
        NumberSpot::BottomRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NumberSpot::TopLeft => "Top Left",
            NumberSpot::TopCenter => "Top Center",
            NumberSpot::TopRight => "Top Right",
            NumberSpot::BottomLeft => "Bottom Left",
            NumberSpot::BottomCenter => "Bottom Center",
            NumberSpot::BottomRight => "Bottom Right",
        }
    }

    fn top(self) -> bool {
        matches!(self, NumberSpot::TopLeft | NumberSpot::TopCenter | NumberSpot::TopRight)
    }

    fn align(self) -> Align {
        match self {
            NumberSpot::TopLeft | NumberSpot::BottomLeft => Align::Left,
            NumberSpot::TopCenter | NumberSpot::BottomCenter => Align::Center,
            NumberSpot::TopRight | NumberSpot::BottomRight => Align::Right,
        }
    }
}

/// Height of a header, footer or page-number box: 0.3 in.
const BAND_H: i64 = 274_320;
/// Width of a page-number box: 1 in.
const NUMBER_W: i64 = 914_400;

/// A text box holding a page-number field.
fn number_box(spot: NumberSpot, page: (i64, i64), m: kernpage_core::Margins) -> Shape {
    let (pw, ph) = page;
    let x = match spot.align() {
        Align::Left => m.left,
        Align::Right => pw - m.right - NUMBER_W,
        _ => (pw - NUMBER_W) / 2,
    };
    // Inside the margin, beside the page edge it belongs to.
    let y = if spot.top() { (m.top - BAND_H).max(0) } else { (ph - m.bottom).min(ph - BAND_H) };
    let run = Run {
        field: Some(Field::PageNumber),
        ..Run::plain("#")
    };
    let mut p = Paragraph::plain("#");
    p.runs = vec![run];
    p.align = spot.align();
    Shape::text_box(Rect::new(x, y, NUMBER_W, BAND_H), vec![p])
}

/// An empty header or footer text box across the margins.
fn band_box(header: bool, page: (i64, i64), m: kernpage_core::Margins) -> Shape {
    let (pw, ph) = page;
    let y = if header { (m.top - BAND_H).max(0) } else { (ph - m.bottom).min(ph - BAND_H) };
    Shape::text_box(
        Rect::new(m.left, y, (pw - m.left - m.right).max(NUMBER_W), BAND_H),
        vec![Paragraph::plain("")],
    )
}

/// The ids of the existing header and footer boxes of a master half: a text box that sits in
/// the top or bottom band.
fn header_footer(shapes: &[Shape], page: (i64, i64), m: kernpage_core::Margins) -> (Option<u64>, Option<u64>) {
    let header = shapes
        .iter()
        .find(|s| s.is_text_box() && s.rect.y + s.rect.h <= m.top + BAND_H)
        .map(|s| s.id);
    let footer = shapes
        .iter()
        .find(|s| s.is_text_box() && s.rect.y >= page.1 - m.bottom - BAND_H)
        .map(|s| s.id);
    (header, footer)
}

/// What a master-pages menu choice does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAct {
    /// Apply master `n` to the page.
    Use(usize),
    /// Turn the page's master off.
    None,
    Edit,
    ApplyDialog,
}

impl App {
    // ---- state -------------------------------------------------------------------------

    pub fn in_master_view(&self) -> bool {
        self.master_view.is_some()
    }

    fn masters(&self) -> &[MasterPage] {
        self.editor.as_ref().map_or(&[], |e| e.doc().masters.as_slice())
    }

    /// `(master, left half)` being edited in the Master Page view.
    pub fn editing_master(&self) -> Option<(usize, bool)> {
        self.master_view.as_ref()?;
        master_surface_parts(self.page)
    }

    /// The real page the Master Page view was opened from (or the current page).
    pub fn base_page(&self) -> usize {
        self.master_view.as_ref().map_or(self.page, |v| v.return_page)
    }

    /// The master page `page` uses, the first when it has none; `None` without any master.
    fn master_of_page(&self, page: usize) -> Option<usize> {
        let doc = self.editor.as_ref()?.doc();
        if doc.masters.is_empty() {
            return None;
        }
        Some(doc.pages.get(page).and_then(|p| p.master).filter(|&m| m < doc.masters.len()).unwrap_or(0))
    }

    /// Every master half in the order the Pages pane lists them.
    fn master_surfaces(&self) -> Vec<(usize, bool)> {
        self.masters()
            .iter()
            .enumerate()
            .flat_map(|(i, m)| {
                let halves: &[bool] = if m.two_page { &[true, false] } else { &[false] };
                halves.iter().map(move |&left| (i, left))
            })
            .collect()
    }

    /// The text the status bar shows in the Master Page view.
    pub fn master_status(&self) -> Option<String> {
        let (m, left) = self.editing_master()?;
        let mp = self.masters().get(m)?;
        let half = match (mp.two_page, left) {
            (true, true) => " (left page)",
            (true, false) => " (right page)",
            _ => "",
        };
        Some(format!("Editing master page: {}{half}", mp.name))
    }

    // ---- entering and leaving -----------------------------------------------------------

    /// Open the Master Page view on `master` (default: the master the current page uses). A
    /// document without a master gets master A first.
    pub fn enter_master_view(&mut self, master: Option<usize>) {
        if self.editor.is_none() {
            return;
        }
        if self.master_view.is_some() {
            if let Some(m) = master {
                self.go_master(m, false);
            }
            return;
        }
        self.commit_edit();
        let page = self.page;
        if self.masters().is_empty() {
            self.exec(Command::EnsureMaster);
        }
        let count = self.masters().len();
        let m = master.or_else(|| self.master_of_page(page)).filter(|&m| m < count).unwrap_or(0);
        self.master_view = Some(MasterView {
            return_page: page,
            return_tab: self.tab,
        });
        self.set_master_surface(master_surface(m, false));
        self.tab = Tab::MasterPage;
        self.backstage = false;
    }

    /// Close Master Page: back to the page the view was opened from.
    pub fn leave_master_view(&mut self) {
        let Some(v) = self.master_view.take() else { return };
        self.commit_edit();
        self.page = v.return_page.min(self.page_count().saturating_sub(1));
        self.selection = None;
        self.multi.clear();
        self.edit = None;
        self.tool = crate::app::Tool::Select;
        self.reset_scroll = true;
        self.tab = if v.return_tab.is_contextual() { Tab::Home } else { v.return_tab };
    }

    pub fn toggle_master_view(&mut self) {
        if self.master_view.is_some() {
            self.leave_master_view();
        } else {
            self.enter_master_view(None);
        }
    }

    fn set_master_surface(&mut self, surface: usize) {
        self.commit_edit();
        self.page = surface;
        self.selection = None;
        self.multi.clear();
        self.edit = None;
        self.reset_scroll = true;
    }

    /// Show master `master` (its left half when `left` and it is a two-page master).
    pub fn go_master(&mut self, master: usize, left: bool) {
        if self.master_view.is_none() {
            return;
        }
        let Some(m) = self.masters().get(master) else { return };
        let left = left && m.two_page;
        self.set_master_surface(master_surface(master, left));
    }

    /// Page Up / Page Down in the Master Page view.
    pub fn step_master(&mut self, delta: isize) {
        let all = self.master_surfaces();
        let Some(at) = self.editing_master().and_then(|c| all.iter().position(|&x| x == c)) else {
            return;
        };
        let to = (at as isize + delta).clamp(0, all.len() as isize - 1) as usize;
        let (m, left) = all[to];
        self.go_master(m, left);
    }

    /// After undo, redo or a delete: the master being edited may be gone.
    pub(crate) fn keep_master_view_valid(&mut self) {
        if self.master_view.is_none() {
            return;
        }
        let Some(ed) = &self.editor else { return };
        if ed.doc().surface(self.page).is_some() {
            return;
        }
        let want = master_surface_parts(self.page).map_or(0, |p| p.0);
        let count = ed.doc().masters.len();
        if count == 0 {
            self.leave_master_view();
        } else {
            self.page = master_surface(want.min(count - 1), false);
            self.selection = None;
            self.multi.clear();
            self.edit = None;
        }
    }

    // ---- master actions -----------------------------------------------------------------

    pub fn open_master_dialog(&mut self, kind: MasterDialogKind, master: usize) {
        let Some(ed) = &self.editor else { return };
        let doc = ed.doc();
        let src = doc.masters.get(master);
        // A document with no master yet will get A first, so the new one is B.
        let id = if doc.masters.is_empty() {
            'B'
        } else {
            match kind {
                MasterDialogKind::Rename => src.map_or('A', |m| m.id),
                _ => doc.next_master_id().unwrap_or('Z'),
            }
        };
        self.master_dialog = Some(MasterDialog {
            kind,
            master,
            id: id.to_string(),
            name: src.map(|m| m.name.clone()).unwrap_or_default(),
            description: if kind == MasterDialogKind::Rename {
                src.map(|m| m.description.clone()).unwrap_or_default()
            } else {
                String::new()
            },
            two_page: false,
            fresh: true,
        });
    }

    /// The master the Master Page tab acts on.
    fn current_master_index(&self) -> usize {
        self.editing_master().map_or(0, |m| m.0)
    }

    pub fn delete_master(&mut self) {
        let Some((m, _)) = self.editing_master() else { return };
        let before = self.masters().len();
        self.exec(Command::DeleteMaster { master: m });
        let after = self.masters().len();
        if after < before {
            self.set_master_surface(master_surface(m.min(after.saturating_sub(1)), false));
        }
    }

    pub fn toggle_two_page_master(&mut self) {
        let Some((m, left)) = self.editing_master() else { return };
        let Some(two) = self.masters().get(m).map(|x| x.two_page) else { return };
        self.exec(Command::SetTwoPageMaster { master: m, two_page: !two });
        if two && left {
            self.go_master(m, false);
        }
    }

    /// Apply `master` (or, with `None`, no master) to `pages`.
    pub fn apply_master(&mut self, pages: Vec<usize>, master: Option<usize>) {
        if self.masters().is_empty() {
            self.exec(Command::EnsureMaster);
        }
        self.exec(Command::ApplyMaster { pages, master });
    }

    pub fn open_apply_dialog(&mut self) {
        if self.editor.is_none() {
            return;
        }
        let current = self.base_page();
        let master = self
            .editing_master()
            .map(|m| m.0)
            .or_else(|| self.master_of_page(current))
            .unwrap_or(0);
        let count = self.page_count();
        self.apply_dialog = Some(ApplyDialog {
            master,
            scope: ApplyScope::Current,
            from: "1".to_owned(),
            to: count.to_string(),
            current,
        });
    }

    // ---- Insert: page numbers, header and footer ----------------------------------------

    /// The pseudo page index of the master half the current page uses (the view's own surface
    /// in the Master Page view); the document gets master A when it has none.
    fn master_target(&mut self) -> Option<usize> {
        if self.master_view.is_some() {
            return Some(self.page);
        }
        if self.editor.is_none() {
            return None;
        }
        if self.masters().is_empty() {
            self.exec(Command::EnsureMaster);
        }
        let m = self.master_of_page(self.page)?;
        let left = self.masters().get(m).is_some_and(|x| x.two_page) && self.page % 2 == 1;
        Some(master_surface(m, left))
    }

    fn surface_geometry(&self, surface: usize) -> Option<((i64, i64), kernpage_core::Margins)> {
        let ed = self.editor.as_ref()?;
        let p = ed.doc().surface(surface)?;
        Some(((p.width_emu, p.height_emu), p.margins_emu))
    }

    /// Insert > Page Number > a position: a text box with a page-number field on the master of
    /// the current page, so every page that uses the master shows its number there.
    pub fn insert_page_number(&mut self, spot: NumberSpot) {
        let Some(surface) = self.master_target() else { return };
        let Some((size, margins)) = self.surface_geometry(surface) else { return };
        self.commit_edit();
        let ids = self.exec(Command::AddShape {
            page: surface,
            shape: number_box(spot, size, margins),
        });
        if self.master_view.is_some() {
            self.selection = ids.first().copied();
            self.tool = crate::app::Tool::Select;
        }
    }

    /// Insert > Page Number > Insert in Current Text Box: a page-number field at the caret.
    pub fn insert_page_number_field(&mut self, ctx: &egui::Context) {
        let Some(edit) = self.edit.as_ref() else { return };
        let caret = crate::app::caret_chars(ctx, edit);
        let (page, id, cell) = (edit.page, edit.id, edit.cell);
        let Some(paras) = self.formatted_paragraphs() else { return };
        let new = kernpage_core::text::insert_field(&paras, caret, Field::PageNumber);
        let cmd = match cell {
            None => Command::SetText { page, id, paragraphs: new.clone() },
            Some(cell) => Command::SetCellText { page, id, cell, paragraphs: new.clone() },
        };
        self.exec(cmd);
        if let Some(e) = &mut self.edit {
            e.text = kernpage_core::text::paragraphs_to_text(&new);
        }
        self.pending_selection = Some(caret + 1..caret + 1);
    }

    /// Insert > Header / Footer: the Master Page view of the current page's master with a header
    /// box at the top margin and a footer box at the bottom margin (made when missing); the
    /// header (or footer) is selected for typing.
    pub fn header_footer(&mut self, footer: bool) {
        if self.editor.is_none() {
            return;
        }
        if self.master_view.is_none() {
            self.enter_master_view(None);
        }
        let surface = self.page;
        let Some((size, margins)) = self.surface_geometry(surface) else { return };
        let shapes = self
            .editor
            .as_ref()
            .and_then(|e| e.doc().surface(surface))
            .map(|p| p.shapes.clone())
            .unwrap_or_default();
        let (mut head, mut foot) = header_footer(&shapes, size, margins);
        if head.is_none() {
            head = self
                .exec(Command::AddShape { page: surface, shape: band_box(true, size, margins) })
                .first()
                .copied();
        }
        if foot.is_none() {
            foot = self
                .exec(Command::AddShape { page: surface, shape: band_box(false, size, margins) })
                .first()
                .copied();
        }
        if let Some(id) = if footer { foot } else { head } {
            self.tool = crate::app::Tool::Select;
            self.selection = Some(id);
            self.multi.clear();
            self.begin_edit(id);
        }
    }

    // ---- ribbon ---------------------------------------------------------------------------

    /// The "Master Page" tab, shown only in the Master Page view.
    pub(crate) fn master_page_tab(&mut self, ui: &mut Ui) {
        let masters = self.masters().to_vec();
        let (cur, left) = self.editing_master().unwrap_or((0, false));
        let two = masters.get(cur).is_some_and(|m| m.two_page);
        let has = !masters.is_empty();
        group(ui, "Master Page", false, |ui| {
            if Btn::large(Icon::New, "Add Master\nPage")
                .tip("Add Master Page")
                .enabled(has)
                .show(ui)
                .clicked()
            {
                self.open_master_dialog(MasterDialogKind::Add, cur);
            }
            if Btn::large(Icon::Copy, "Duplicate")
                .tip("Duplicate this master page")
                .enabled(has)
                .show(ui)
                .clicked()
            {
                self.open_master_dialog(MasterDialogKind::Duplicate, cur);
            }
            crate::widgets::col(ui, |ui| {
                if Btn::small(Icon::Rename, "Rename")
                    .tip("Rename this master page")
                    .enabled(has)
                    .show(ui)
                    .clicked()
                {
                    self.open_master_dialog(MasterDialogKind::Rename, cur);
                }
                if Btn::small(Icon::Trash, "Delete")
                    .tip("Delete this master page")
                    .enabled(has)
                    .show(ui)
                    .clicked()
                {
                    self.delete_master();
                }
            });
            if Btn::large(Icon::TwoPage, "Two-Page\nMaster")
                .tip("Two-Page Master")
                .selected(two)
                .enabled(has)
                .show(ui)
                .clicked()
            {
                self.toggle_two_page_master();
            }
        });
        group(ui, "Apply", false, |ui| {
            let r = Btn::large(Icon::Pages, "Apply To")
                .tip("Apply this master page to pages")
                .dropdown()
                .enabled(has)
                .show(ui);
            self.open_for_debug(ui, &r, "apply-to");
            let mut run: Option<ApplyScope> = None;
            let mut dialog = false;
            Popup::menu(&r).show(|ui| {
                ui.set_min_width(150.0);
                if ui.button("Current Page").clicked() {
                    run = Some(ApplyScope::Current);
                    ui.close();
                }
                if ui.button("All Pages").clicked() {
                    run = Some(ApplyScope::All);
                    ui.close();
                }
                if ui.button("Pages...").clicked() {
                    dialog = true;
                    ui.close();
                }
            });
            match run {
                Some(ApplyScope::Current) => self.apply_master(vec![self.base_page()], Some(cur)),
                Some(_) => self.apply_master((0..self.page_count()).collect(), Some(cur)),
                None => {}
            }
            if dialog {
                self.open_apply_dialog();
            }
        });
        group(ui, "Close", false, |ui| {
            if Btn::large(Icon::Close, "Close Master\nPage")
                .tip("Close Master Page")
                .show(ui)
                .clicked()
            {
                self.leave_master_view();
            }
        });
        let _ = left;
    }

    /// The list of masters with the page's current one marked, then None, Edit Master Pages
    /// and Apply Master Page. Shared by the Page Design drop-down and the Pages pane's menu.
    fn master_menu_items(&self, page: usize) -> (Vec<(String, bool)>, bool) {
        match self.editor.as_ref() {
            Some(e) => menu_items_of(e.doc(), page),
            None => (Vec::new(), false),
        }
    }

    fn run_menu_act(&mut self, page: usize, act: MenuAct) {
        match act {
            MenuAct::Use(m) => self.apply_master(vec![page], Some(m)),
            MenuAct::None => self.apply_master(vec![page], None),
            MenuAct::Edit => {
                let m = self.master_of_page(page);
                self.enter_master_view(m);
            }
            MenuAct::ApplyDialog => self.open_apply_dialog(),
        }
    }

    /// Page Design > Master Pages.
    pub(crate) fn master_pages_dropdown(&mut self, ui: &mut Ui) {
        let has = self.editor.is_some();
        let page = self.base_page();
        let (items, ignore) = self.master_menu_items(page);
        let r = Btn::large(Icon::Pages, "Master\nPages")
            .tip("Master Pages")
            .dropdown()
            .enabled(has)
            .show(ui);
        self.open_for_debug(ui, &r, "master-pages");
        let mut act = None;
        Popup::menu(&r).show(|ui| {
            ui.set_min_width(200.0);
            for (i, (label, current)) in items.iter().enumerate() {
                if ui.add(egui::Button::new(label).selected(*current)).clicked() {
                    act = Some(MenuAct::Use(i));
                    ui.close();
                }
            }
            if ui.add(egui::Button::new("None").selected(ignore)).clicked() {
                act = Some(MenuAct::None);
                ui.close();
            }
            ui.separator();
            if ui.button("Edit Master Pages").clicked() {
                act = Some(MenuAct::Edit);
                ui.close();
            }
            if ui.button("Apply Master Page...").clicked() {
                act = Some(MenuAct::ApplyDialog);
                ui.close();
            }
        });
        if let Some(a) = act {
            self.run_menu_act(page, a);
        }
    }

    /// Insert > Header & Footer group.
    pub(crate) fn header_footer_group(&mut self, ui: &mut Ui) {
        let has = self.editor.is_some();
        group(ui, "Header & Footer", false, |ui| {
            if Btn::large(Icon::Header, "Header")
                .tip("Header: edit the header of the master page")
                .enabled(has)
                .show(ui)
                .clicked()
            {
                self.header_footer(false);
            }
            if Btn::large(Icon::Footer, "Footer")
                .tip("Footer: edit the footer of the master page")
                .enabled(has)
                .show(ui)
                .clicked()
            {
                self.header_footer(true);
            }
            let r = Btn::large(Icon::Hash, "Page\nNumber")
                .tip("Page Number")
                .dropdown()
                .enabled(has)
                .show(ui);
            self.open_for_debug(ui, &r, "page-number");
            let editing = self.edit.is_some();
            let mut spot = None;
            let mut field = false;
            Popup::menu(&r).show(|ui| {
                ui.set_min_width(190.0);
                for s in NumberSpot::ALL {
                    if ui.button(s.label()).clicked() {
                        spot = Some(s);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.add_enabled(editing, egui::Button::new("Insert in Current Text Box")).clicked() {
                    field = true;
                    ui.close();
                }
                // Page numbers cannot be formatted yet (sections have no dialog).
                ui.add_enabled(false, egui::Button::new("Format Page Numbers..."))
                    .on_disabled_hover_text("Not available yet");
            });
            if let Some(s) = spot {
                self.insert_page_number(s);
            }
            if field {
                let ctx = ui.ctx().clone();
                self.insert_page_number_field(&ctx);
            }
        });
    }

    /// View > Views: Normal and Master Page.
    pub(crate) fn views_group(&mut self, ui: &mut Ui) {
        let has = self.editor.is_some();
        let master = self.master_view.is_some();
        group(ui, "Views", false, |ui| {
            if Btn::large(Icon::Page, "Normal")
                .selected(!master)
                .enabled(has)
                .tip("Normal view")
                .show(ui)
                .clicked()
                && master
            {
                self.leave_master_view();
            }
            if Btn::large(Icon::Pages, "Master\nPage")
                .selected(master)
                .enabled(has)
                .tip("Master Page view (Ctrl+M)")
                .show(ui)
                .clicked()
                && !master
            {
                self.enter_master_view(None);
            }
        });
    }

    /// The Pages pane's right-click menu entries "Master Pages" for page `page`.
    pub(crate) fn master_submenu_items(&self, page: usize) -> (Vec<(String, bool)>, bool) {
        self.master_menu_items(page)
    }

    /// Run a choice from the Pages pane's Master Pages menu: `Some(n)` apply master n, `None`
    /// no master; `edit` opens the Master Page view; `dialog` the Apply dialog.
    pub(crate) fn pane_master_choice(&mut self, page: usize, choice: PaneMasterChoice) {
        match choice {
            PaneMasterChoice::Use(m) => self.run_menu_act(page, MenuAct::Use(m)),
            PaneMasterChoice::None => self.run_menu_act(page, MenuAct::None),
            PaneMasterChoice::Edit => self.run_menu_act(page, MenuAct::Edit),
            PaneMasterChoice::Dialog => self.run_menu_act(page, MenuAct::ApplyDialog),
        }
    }

    pub(crate) fn run_master_act(&mut self, a: MasterAct) {
        match a {
            MasterAct::Go(m, left) => self.go_master(m, left),
            MasterAct::Add => {
                let cur = self.current_master_index();
                self.open_master_dialog(MasterDialogKind::Add, cur);
            }
            MasterAct::Duplicate(m) => self.open_master_dialog(MasterDialogKind::Duplicate, m),
            MasterAct::Rename(m) => self.open_master_dialog(MasterDialogKind::Rename, m),
            MasterAct::Delete(m) => {
                self.go_master(m, false);
                self.delete_master();
            }
        }
    }

    // ---- dialogs ----------------------------------------------------------------------------

    pub(crate) fn master_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.master_dialog else { return };
        let (mut ok, mut cancel) = (false, false);
        let masters: Vec<MasterPage> = self.editor.as_ref().map(|e| e.doc().masters.clone()).unwrap_or_default();
        let error = d.command(&masters).err();
        let title = match d.kind {
            MasterDialogKind::Add => "Add Master Page",
            MasterDialogKind::Duplicate => "Duplicate Master Page",
            MasterDialogKind::Rename => "Rename Master Page",
        };
        let fresh = std::mem::take(&mut d.fresh);
        egui::Modal::new(egui::Id::new("master_dialog")).show(ctx, |ui| {
            ui.set_width(330.0);
            ui.heading(title);
            ui.separator();
            egui::Grid::new("master_dialog_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label("Page ID:");
                let r = ui.add(egui::TextEdit::singleline(&mut d.id).desired_width(48.0));
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Page ID"));
                if fresh {
                    r.request_focus();
                    if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                        st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(d.id.chars().count()),
                        )));
                        st.store(ui.ctx(), r.id);
                    }
                }
                ui.end_row();
                if d.kind == MasterDialogKind::Rename {
                    ui.label("Name:");
                    let r = ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(220.0));
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Name"));
                    ui.end_row();
                }
                ui.label("Description:");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut d.description)
                        .char_limit(MasterPage::MAX_DESCRIPTION)
                        .desired_width(220.0),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Description"));
                ui.end_row();
            });
            if d.kind != MasterDialogKind::Rename {
                ui.add_space(6.0);
                ui.checkbox(&mut d.two_page, "Two-page master");
            }
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(error.clone().unwrap_or_default())
                    .color(egui::Color32::from_rgb(0xC0, 0x30, 0x30))
                    .size(12.0),
            );
            ui.add_space(4.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel = ui.button("  Cancel  ").clicked();
                ok = ui
                    .add(egui::Button::new("     OK     ").stroke(Stroke::new(1.5, ACCENT_COLOR)))
                    .clicked();
            });
        });
        ctx.input(|i| {
            if i.key_pressed(Key::Enter) {
                ok = true;
            }
            if i.key_pressed(Key::Escape) {
                cancel = true;
            }
        });
        if ok {
            let Some(d) = &self.master_dialog else { return };
            if let Ok(cmd) = d.command(&masters) {
                let kind = d.kind;
                self.master_dialog = None;
                let before = self.masters().len();
                self.exec(cmd);
                // The new master is shown.
                let after = self.masters().len();
                if kind != MasterDialogKind::Rename && after > before && self.master_view.is_some() {
                    self.go_master(after - 1, false);
                }
            }
        } else if cancel {
            self.master_dialog = None;
        }
    }

    pub(crate) fn apply_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.apply_dialog else { return };
        let (mut ok, mut cancel) = (false, false);
        let masters: Vec<String> = self
            .editor
            .as_ref()
            .map(|e| e.doc().masters.iter().map(master_label).collect())
            .unwrap_or_default();
        let masters = if masters.is_empty() {
            vec![format!("A - {}", MasterPage::default_name('A'))]
        } else {
            masters
        };
        let count = self.editor.as_ref().map_or(0, |e| e.doc().pages.len());
        let error = if d.scope == ApplyScope::Range { d.pages(count).err() } else { None };
        egui::Modal::new(egui::Id::new("apply_dialog")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.heading("Apply Master Page");
            ui.separator();
            ui.label("Select a master page:");
            egui::ComboBox::from_id_salt("apply_master")
                .selected_text(masters.get(d.master).cloned().unwrap_or_default())
                .width(300.0)
                .show_ui(ui, |ui| {
                    for (i, label) in masters.iter().enumerate() {
                        ui.selectable_value(&mut d.master, i, label);
                    }
                });
            ui.add_space(8.0);
            ui.label("Apply to:");
            ui.radio_value(&mut d.scope, ApplyScope::Current, "Current page");
            ui.radio_value(&mut d.scope, ApplyScope::All, "All pages");
            ui.horizontal(|ui| {
                ui.radio_value(&mut d.scope, ApplyScope::Range, "Pages");
                let r = ui.add(egui::TextEdit::singleline(&mut d.from).desired_width(40.0));
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "From page"));
                ui.label("to");
                let r = ui.add(egui::TextEdit::singleline(&mut d.to).desired_width(40.0));
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "To page"));
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(error.clone().unwrap_or_default())
                    .color(egui::Color32::from_rgb(0xC0, 0x30, 0x30))
                    .size(12.0),
            );
            ui.add_space(4.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel = ui.button("  Cancel  ").clicked();
                ok = ui
                    .add(egui::Button::new("     OK     ").stroke(Stroke::new(1.5, ACCENT_COLOR)))
                    .clicked();
            });
        });
        ctx.input(|i| {
            if i.key_pressed(Key::Enter) {
                ok = true;
            }
            if i.key_pressed(Key::Escape) {
                cancel = true;
            }
        });
        if ok {
            let Some(d) = &self.apply_dialog else { return };
            if let Ok(pages) = d.pages(count) {
                let master = d.master;
                self.apply_dialog = None;
                self.apply_master(pages, Some(master));
            }
        } else if cancel {
            self.apply_dialog = None;
        }
    }
}

/// The masters as menu rows ("A - Master Page A", whether page `page` uses it) and whether
/// the page ignores its master (the "None" row).
pub fn menu_items_of(doc: &Document, page: usize) -> (Vec<(String, bool)>, bool) {
    let p = doc.pages.get(page);
    let ignore = p.is_some_and(|p| p.ignore_master);
    let cur = p.and_then(|p| p.master).unwrap_or(0);
    let items = if doc.masters.is_empty() {
        // No master yet: master A is there once it is used.
        vec![(format!("A - {}", MasterPage::default_name('A')), !ignore)]
    } else {
        doc.masters
            .iter()
            .enumerate()
            .map(|(i, m)| (master_label(m), i == cur && !ignore))
            .collect()
    };
    (items, ignore)
}

/// A choice from the Pages pane's Master Pages menu.
#[derive(Clone, Copy, Debug)]
pub enum PaneMasterChoice {
    Use(usize),
    None,
    Edit,
    Dialog,
}

/// "A - Master Page A" (the description follows when there is one).
pub fn master_label(m: &MasterPage) -> String {
    if m.description.is_empty() {
        format!("{} - {}", m.id, m.name)
    } else {
        format!("{} - {} ({})", m.id, m.name, m.description)
    }
}

/// The Pages pane in the Master Page view: a thumbnail of every master (both halves of a
/// two-page master), the current one marked, and an "Add master page" card. Returns what was
/// asked for.
pub fn master_list(
    ui: &mut Ui,
    doc: &Document,
    cur: usize,
    fonts: &FontMgr,
    pics: &mut PictureCache,
) -> Option<MasterAct> {
    use crate::nav::{add_card, paint_thumb, thumb_label, THUMB_W};
    let mut act = None;
    ui.add_space(6.0);
    let n = doc.masters.len();
    for (i, m) in doc.masters.iter().enumerate() {
        let halves: &[bool] = if m.two_page { &[true, false] } else { &[false] };
        for &left in halves {
            let surface = master_surface(i, left);
            let Some(page) = doc.surface(surface) else { continue };
            let aspect = page.height_emu as f32 / page.width_emu.max(1) as f32;
            let size = egui::vec2(THUMB_W, THUMB_W * aspect);
            let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
            let current = surface == cur;
            thumb_label(ui, rect, &resp, current);
            paint_thumb(ui, rect, doc, surface, fonts, pics);
            crate::nav::thumb_border(ui, rect, current);
            #[cfg(test)]
            crate::nav::probe::with(|p| p.thumbs.push((surface, rect, current)));
            let half = match (m.two_page, left) {
                (true, true) => " (left)",
                (true, false) => " (right)",
                _ => "",
            };
            let text = egui::RichText::new(format!("{}{half}", master_label(m)))
                .color(crate::theme::text())
                .size(12.0);
            ui.label(if current { text.strong() } else { text });
            ui.add_space(8.0);
            if resp.clicked() {
                act = Some(MasterAct::Go(i, left));
            }
            resp.context_menu(|ui| {
                if ui.button("Duplicate").clicked() {
                    act = Some(MasterAct::Duplicate(i));
                    ui.close();
                }
                if ui.button("Rename...").clicked() {
                    act = Some(MasterAct::Rename(i));
                    ui.close();
                }
                if ui.add_enabled(n > 1, egui::Button::new("Delete")).clicked() {
                    act = Some(MasterAct::Delete(i));
                    ui.close();
                }
            });
        }
    }
    if add_card(ui, "Add master page", "Add a new master page") {
        act = Some(MasterAct::Add);
    }
    act
}
