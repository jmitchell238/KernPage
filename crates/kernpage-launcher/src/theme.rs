//! KernPage Launcher colours and egui styling (light and dark).

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub bar: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub ok: Color32,
    pub ok_bg: Color32,
    /// "Update available" amber.
    pub attn: Color32,
    pub attn_bg: Color32,
    pub err: Color32,
    pub err_bg: Color32,
    pub dark: bool,
}

impl Palette {
    pub fn light() -> Self {
        Palette {
            bg: Color32::from_rgb(0xF3, 0xF1, 0xEC),
            bar: Color32::from_rgb(0xFB, 0xFA, 0xF7),
            card: Color32::WHITE,
            card_hover: Color32::from_rgb(0xF8, 0xFA, 0xFE),
            border: Color32::from_rgb(0xD9, 0xD5, 0xCC),
            text: Color32::from_rgb(0x1F, 0x24, 0x30),
            muted: Color32::from_rgb(0x69, 0x6F, 0x7D),
            accent: Color32::from_rgb(0x2B, 0x5B, 0xA8),
            on_accent: Color32::WHITE,
            ok: Color32::from_rgb(0x1B, 0x72, 0x48),
            ok_bg: Color32::from_rgb(0xDF, 0xF1, 0xE7),
            attn: Color32::from_rgb(0x9A, 0x5B, 0x00),
            attn_bg: Color32::from_rgb(0xFF, 0xEE, 0xCF),
            err: Color32::from_rgb(0xC2, 0x4B, 0x45),
            err_bg: Color32::from_rgb(0xFB, 0xE9, 0xE7),
            dark: false,
        }
    }

    pub fn dark() -> Self {
        Palette {
            bg: Color32::from_rgb(0x12, 0x15, 0x1A),
            bar: Color32::from_rgb(0x17, 0x1B, 0x21),
            card: Color32::from_rgb(0x1C, 0x21, 0x29),
            card_hover: Color32::from_rgb(0x23, 0x29, 0x33),
            border: Color32::from_rgb(0x2D, 0x34, 0x3F),
            text: Color32::from_rgb(0xE7, 0xE9, 0xED),
            muted: Color32::from_rgb(0x8F, 0x98, 0xA6),
            accent: Color32::from_rgb(0x6F, 0xA1, 0xF0),
            on_accent: Color32::from_rgb(0x0B, 0x12, 0x20),
            ok: Color32::from_rgb(0x72, 0xD0, 0x9C),
            ok_bg: Color32::from_rgb(0x16, 0x32, 0x23),
            attn: Color32::from_rgb(0xF2, 0xB3, 0x5A),
            attn_bg: Color32::from_rgb(0x3A, 0x2B, 0x12),
            err: Color32::from_rgb(0xE0, 0x6C, 0x66),
            err_bg: Color32::from_rgb(0x3A, 0x1F, 0x22),
            dark: true,
        }
    }

    pub fn for_theme(t: egui::Theme) -> Self {
        match t {
            egui::Theme::Dark => Self::dark(),
            egui::Theme::Light => Self::light(),
        }
    }
}

/// Apply the palette and text sizes to the egui context.
pub fn apply(ctx: &egui::Context, p: &Palette) {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.extreme_bg_color = p.bg;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    let r = CornerRadius::same(6);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.bg_fill = p.card;
    v.widgets.inactive.weak_bg_fill = p.card;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.bg_fill = p.card_hover;
    v.widgets.hovered.weak_bg_fill = p.card_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.interaction.tooltip_delay = 0.2;
    });
}
