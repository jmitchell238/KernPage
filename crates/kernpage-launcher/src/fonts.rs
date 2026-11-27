//! The launcher's fonts: loaded from the Windows fonts folder at start-up, never bundled
//! (see `kernpage_core::sysfonts`).

use eframe::egui::{Context, FontData, FontDefinitions, FontFamily};
use kernpage_core::sysfonts::{self, FontPlan};
use std::sync::Arc;

/// Turns a font plan into egui font definitions (the page-text family is not used here).
pub fn definitions_from(plan: &FontPlan) -> FontDefinitions {
    let mut defs = FontDefinitions::empty();
    for (k, bytes) in &plan.data {
        let data = match bytes {
            std::borrow::Cow::Borrowed(b) => FontData::from_static(b),
            std::borrow::Cow::Owned(v) => FontData::from_owned(v.clone()),
        };
        defs.font_data.insert(k.clone(), Arc::new(data));
    }
    defs.families.insert(FontFamily::Proportional, plan.proportional.clone());
    defs.families.insert(FontFamily::Monospace, plan.monospace.clone());
    defs
}

/// Installs Segoe UI, Consolas and the symbol fonts into `ctx`.
pub fn install(ctx: &Context) {
    ctx.set_fonts(definitions_from(&sysfonts::plan()));
}
