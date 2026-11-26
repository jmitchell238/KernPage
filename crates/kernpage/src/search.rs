//! The title bar's "Search commands" box: a table of the ribbon's tabs, groups and buttons.
//! Choosing a match switches to its tab and briefly outlines the button (see `flash`).
//!
//! The ribbon is drawn in immediate mode and has no command registry, so the table below
//! lists the labels by hand (name, tab, group). A test keeps it honest for the tab names.

use crate::app::Tab;

/// One searchable ribbon entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cmd {
    pub name: &'static str,
    pub tab: Tab,
    /// The ribbon group it sits in ("" for a tab itself).
    pub group: &'static str,
}

const fn c(name: &'static str, tab: Tab, group: &'static str) -> Cmd {
    Cmd { name, tab, group }
}

use Tab::{Data, Help, Home, Insert, PageDesign, Review, View};

pub const COMMANDS: &[Cmd] = &[
    c("Home", Home, ""),
    c("Insert", Insert, ""),
    c("Page Design", PageDesign, ""),
    c("Data", Data, ""),
    c("Review", Review, ""),
    c("View", View, ""),
    c("Help", Help, ""),
    // Home
    c("Paste", Home, "Clipboard"),
    c("Cut", Home, "Clipboard"),
    c("Copy", Home, "Clipboard"),
    c("Font", Home, "Font"),
    c("Font Size", Home, "Font"),
    c("Grow Font", Home, "Font"),
    c("Shrink Font", Home, "Font"),
    c("Clear Formatting", Home, "Font"),
    c("Bold", Home, "Font"),
    c("Italic", Home, "Font"),
    c("Underline", Home, "Font"),
    c("Subscript", Home, "Font"),
    c("Superscript", Home, "Font"),
    c("Font Color", Home, "Font"),
    c("Align Left", Home, "Paragraph"),
    c("Center", Home, "Paragraph"),
    c("Align Right", Home, "Paragraph"),
    c("Justify", Home, "Paragraph"),
    c("Line Spacing", Home, "Paragraph"),
    c("Draw Text Box", Home, "Objects"),
    c("Pictures", Home, "Objects"),
    c("Shapes", Home, "Objects"),
    c("Group", Home, "Arrange"),
    c("Ungroup", Home, "Arrange"),
    c("Align", Home, "Arrange"),
    c("Wrap Text", Home, "Arrange"),
    c("Columns", Home, "Arrange"),
    c("Rotate", Home, "Arrange"),
    c("Find", Home, "Editing"),
    c("Replace", Home, "Editing"),
    c("Select", Home, "Editing"),
    // Insert
    c("Page", Insert, "Pages"),
    c("Tables", Insert, "Tables"),
    c("Pictures", Insert, "Illustrations"),
    c("Draw Text Box", Insert, "Text"),
    c("Header", Insert, "Header & Footer"),
    c("Footer", Insert, "Header & Footer"),
    c("Link", Insert, "Links"),
    c("Bookmark", Insert, "Links"),
    // Page Design
    c("Margins", PageDesign, "Page Setup"),
    c("Orientation", PageDesign, "Page Setup"),
    c("Size", PageDesign, "Page Setup"),
    c("Page Setup", PageDesign, "Page Setup"),
    c("Guides", PageDesign, "Layout"),
    c("Delete Page", PageDesign, "Pages"),
    c("Palettes", PageDesign, "Palettes"),
    c("Background", PageDesign, "Page Background"),
    // Data
    c("Data Source", Data, "List"),
    c("Edit List", Data, "List"),
    c("Insert Field", Data, "Fields"),
    c("Picture Field", Data, "Fields"),
    c("Numbering", Data, "Fields"),
    c("Make Copies", Data, "Layout"),
    c("Repeating Area", Data, "Layout"),
    c("Per Sheet", Data, "Layout"),
    c("Preview", Data, "Check"),
    c("Find Row", Data, "Check"),
    c("Create", Data, "Create"),
    // Review
    c("Spelling", Review, "Proofing"),
    c("Thesaurus", Review, "Proofing"),
    // Help
    c("Report a Problem", Help, "Help"),
    c("Suggest an Idea", Help, "Help"),
    // View
    c("Normal", View, "Views"),
    c("Single Page", View, "Layout"),
    c("Two-Page Spread", View, "Layout"),
    c("Boundaries", View, "Show"),
    c("Rulers", View, "Show"),
    c("Page Navigation", View, "Show"),
    c("Scratch Area", View, "Show"),
    c("100%", View, "Zoom"),
    c("Whole Page", View, "Zoom"),
    c("Page Width", View, "Zoom"),
];

/// The entries matching `query` (case-insensitive): names starting with it first, then names
/// containing it, then entries whose group does; at most `max`.
pub fn find(query: &str, max: usize) -> Vec<Cmd> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(u8, usize, Cmd)> = COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(i, cmd)| {
            let n = cmd.name.to_lowercase();
            let score = if n.starts_with(&q) {
                0
            } else if n.contains(&q) {
                1
            } else if cmd.group.to_lowercase().contains(&q) {
                2
            } else {
                return None;
            };
            Some((score, i, *cmd))
        })
        .collect();
    scored.sort_by_key(|(s, i, _)| (*s, *i));
    scored.into_iter().take(max).map(|(_, _, c)| c).collect()
}

/// The button the last search chose, outlined for a moment so the eye finds it.
pub mod flash {
    use std::cell::RefCell;

    thread_local! {
        static FLASH: RefCell<Option<(String, f64)>> = const { RefCell::new(None) };
    }

    /// Outline buttons named `name` until `until` (egui time, seconds).
    pub fn set(name: &str, until: f64) {
        FLASH.with(|f| *f.borrow_mut() = Some((name.to_owned(), until)));
    }

    /// Whether a button with this label / tooltip is the one to outline at time `now`.
    pub fn matches(label: &str, tip: &str, now: f64) -> bool {
        FLASH.with(|f| {
            let f = f.borrow();
            let Some((name, until)) = f.as_ref() else { return false };
            if now > *until {
                return false;
            }
            let label = label.replace('\n', " ");
            let name = name.to_lowercase();
            label.to_lowercase() == name
                || tip.to_lowercase().split(" (").next().is_some_and(|t| t == name)
        })
    }
}

/// State of the title-bar search box.
#[derive(Default)]
pub struct SearchState {
    pub text: String,
    /// The highlighted row of the result list.
    pub sel: usize,
    /// Where the result list was drawn last frame (it stays open while the pointer is on it).
    popup: Option<egui::Rect>,
}

use crate::app::App;
use crate::icons::Icon;
use eframe::egui::{self, Align2, FontId, Key, Sense, Stroke, Ui, Vec2};

pub const SEARCH_ID: &str = "search_commands";

impl App {
    /// Run a search result: show its tab and outline its button for a moment.
    pub fn run_search(&mut self, ctx: &egui::Context, cmd: Cmd) {
        self.commit_edit();
        self.tab = cmd.tab;
        self.backstage = false;
        self.ribbon_collapsed = false;
        if !cmd.group.is_empty() {
            flash::set(cmd.name, ctx.input(|i| i.time) + 2.5);
            ctx.request_repaint();
        }
        self.search.text.clear();
        self.search.sel = 0;
        self.search.popup = None;
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::unique(SEARCH_ID)));
    }

    /// The box itself, centred in the title row, with the result list under it. Alt+Q
    /// focuses it; Up/Down/Enter/Esc work while it has the keyboard.
    pub(crate) fn search_box(&mut self, ui: &mut Ui, rect: egui::Rect) {
        let has_doc = self.editor.is_some();
        let ctx = ui.ctx().clone();
        let id = egui::Id::unique(SEARCH_ID);
        if has_doc && ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, Key::Q)) {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        let focused = ctx.memory(|m| m.has_focus(id));
        let results = if has_doc { find(&self.search.text, 8) } else { Vec::new() };
        if focused && !results.is_empty() {
            let (down, up) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                )
            });
            if down {
                self.search.sel = (self.search.sel + 1).min(results.len() - 1);
            }
            if up {
                self.search.sel = self.search.sel.saturating_sub(1);
            }
        }
        self.search.sel = self.search.sel.min(results.len().saturating_sub(1));
        let p = ui.painter();
        p.rect(
            rect,
            11.0,
            crate::theme::card(),
            Stroke::new(1.0, if focused { crate::theme::accent() } else { crate::theme::line() }),
            egui::StrokeKind::Inside,
        );
        crate::icons::paint(
            p,
            egui::Rect::from_center_size(rect.left_center() + Vec2::new(13.0, 0.0), Vec2::splat(13.0)),
            Icon::Find,
            if has_doc { crate::theme::group_label() } else { crate::theme::disabled() },
        );
        let before = self.search.text.clone();
        // A hand-made single-line field: an egui `TextEdit` would add one more text input to
        // the window's accessibility tree (and to every test that looks for the ribbon's boxes).
        let field = egui::Rect::from_min_max(rect.min + Vec2::new(26.0, 1.0), rect.max - Vec2::new(8.0, 1.0));
        let resp = ui.interact(rect, id, if has_doc { Sense::click() } else { Sense::hover() });
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, has_doc, "Search commands"));
        if resp.clicked() {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        if focused {
            ctx.input_mut(|i| {
                i.events.retain(|e| match e {
                    egui::Event::Text(t) | egui::Event::Paste(t) => {
                        self.search.text.extend(t.chars().filter(|c| !c.is_control()));
                        false
                    }
                    egui::Event::Key { key: Key::Backspace, pressed: true, .. } => {
                        self.search.text.pop();
                        false
                    }
                    _ => true,
                });
            });
        }
        let painter = ui.painter().with_clip_rect(field);
        let font = FontId::proportional(12.5);
        if self.search.text.is_empty() {
            painter.text(
                field.left_center(),
                Align2::LEFT_CENTER,
                "Search commands (Alt+Q)",
                font,
                if has_doc { crate::theme::group_label() } else { crate::theme::disabled() },
            );
            if focused {
                let x = field.left() + 0.5;
                painter.line_segment(
                    [egui::pos2(x, field.center().y - 7.0), egui::pos2(x, field.center().y + 7.0)],
                    Stroke::new(1.0, crate::theme::text()),
                );
            }
        } else {
            let galley = painter.layout_no_wrap(self.search.text.clone(), font, crate::theme::text());
            let shift = (field.width() - galley.size().x - 2.0).min(0.0);
            let pos = egui::pos2(field.left() + shift, field.center().y - galley.size().y / 2.0);
            let end = pos.x + galley.size().x;
            painter.galley(pos, galley, crate::theme::text());
            if focused {
                painter.line_segment(
                    [egui::pos2(end + 1.0, field.center().y - 7.0), egui::pos2(end + 1.0, field.center().y + 7.0)],
                    Stroke::new(1.0, crate::theme::text()),
                );
            }
        }
        #[cfg(test)]
        crate::widgets::probe::record(crate::widgets::probe::Seen {
            label: "search".to_owned(),
            tip: String::new(),
            rect,
            enabled: has_doc,
            selected: false,
        });
        if self.search.text != before {
            self.search.sel = 0;
        }
        let enter = focused && ctx.input(|i| i.key_pressed(Key::Enter));
        if enter {
            if let Some(cmd) = results.get(self.search.sel).copied() {
                self.run_search(&ctx, cmd);
                return;
            }
        }
        // egui drops the focus itself on Esc, so `focused` can already be false here.
        if (focused || !self.search.text.is_empty()) && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.search.text.clear();
            ctx.memory_mut(|m| m.surrender_focus(id));
            self.search.popup = None;
            return;
        }
        let pointer_in_list = self
            .search
            .popup
            .zip(ctx.pointer_latest_pos())
            .is_some_and(|(r, pos)| r.contains(pos));
        if !(focused || pointer_in_list) || results.is_empty() {
            self.search.popup = None;
            return;
        }
        let mut chosen = None;
        let sel = self.search.sel;
        let area = egui::Area::new(egui::Id::unique("search_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.left_bottom() + Vec2::new(0.0, 4.0))
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(8.0)
                    .fill(crate::theme::card())
                    .show(ui, |ui| {
                        ui.set_width(rect.width() - 16.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (i, cmd) in results.iter().enumerate() {
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
                            let p = ui.painter();
                            if i == sel || resp.hovered() {
                                p.rect_filled(r, 5.0, crate::theme::accent_tint());
                            }
                            p.text(
                                r.left_center() + Vec2::new(8.0, 0.0),
                                Align2::LEFT_CENTER,
                                cmd.name,
                                FontId::proportional(13.0),
                                crate::theme::text(),
                            );
                            let place = if cmd.group.is_empty() {
                                "Tab".to_owned()
                            } else {
                                format!("{} > {}", cmd.tab.name(), cmd.group)
                            };
                            p.text(
                                r.right_center() - Vec2::new(8.0, 0.0),
                                Align2::RIGHT_CENTER,
                                place,
                                FontId::proportional(11.5),
                                crate::theme::group_label(),
                            );
                            #[cfg(test)]
                            crate::widgets::probe::record(crate::widgets::probe::Seen {
                                label: format!("search-result:{}", cmd.name),
                                tip: String::new(),
                                rect: r,
                                enabled: true,
                                selected: i == sel,
                            });
                            if resp.clicked() {
                                chosen = Some(*cmd);
                            }
                        }
                    });
            });
        self.search.popup = Some(area.response.rect);
        if let Some(cmd) = chosen {
            self.run_search(&ctx, cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bold_on_home() {
        let r = find("bold", 8);
        assert_eq!(r.first().map(|c| (c.name, c.tab)), Some(("Bold", Tab::Home)));
    }

    #[test]
    fn prefix_matches_come_first() {
        let r = find("page", 20);
        assert!(r[0].name.to_lowercase().starts_with("page"));
        assert!(r.iter().any(|c| c.name == "Page Width"));
    }

    #[test]
    fn empty_query_finds_nothing() {
        assert!(find("  ", 8).is_empty());
        assert!(find("zzzz", 8).is_empty());
    }

    #[test]
    fn every_contextless_tab_is_findable() {
        for t in Tab::ALL {
            assert!(COMMANDS.iter().any(|c| c.name == t.name() && c.tab == t), "{}", t.name());
        }
    }

    /// Microsoft feature names must not appear in anything the user sees (owner decision 2026-11-25).
    #[test]
    fn no_microsoft_feature_names_in_labels_or_text() {
        let banned = [
            "Building Blocks", "Page Parts", "Graphics Manager", "Pack and Go", "Format Painter", "WordArt", "Backstage", "Design Checker", "Business Information",
        ];
        for c in COMMANDS {
            for b in banned {
                assert!(!c.name.contains(b) && !c.group.contains(b), "command {:?} uses {b}", c.name);
            }
        }
        let sources = [
            ("ribbon.rs", include_str!("ribbon.rs")),
            ("ctx_ribbon.rs", include_str!("ctx_ribbon.rs")),
            ("output.rs", include_str!("output.rs")),
            ("backstage.rs", include_str!("backstage.rs")),
            ("search.rs", include_str!("search.rs").split("#[cfg(test)]").next().unwrap_or("")),
            ("main.rs", include_str!("main.rs")),
            ("widgets.rs", include_str!("widgets.rs")),
        ];
        for (file, text) in sources {
            // Two-line ribbon labels are written with "\n"; read them as one line.
            let text = text.replace("\\n", " ");
            for b in banned {
                assert!(!text.contains(b), "{file} still contains {b:?}");
            }
        }
    }
}
