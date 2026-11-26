//! The native file dialogs behind one small seam: the app asks `App::dialogs`, whose default
//! (`Native`) shows the real rfd dialog. UI tests install a fake that returns scripted paths
//! and records what it was asked.

use std::path::PathBuf;

/// A file type filter: the label shown in the dialog and its extensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub label: String,
    pub extensions: Vec<String>,
}

impl Filter {
    pub fn new(label: &str, extensions: &[&str]) -> Filter {
        Filter {
            label: label.to_owned(),
            extensions: extensions.iter().map(|e| (*e).to_owned()).collect(),
        }
    }
}

/// An Open dialog (Open Publication, Insert Picture).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    pub title: String,
    pub filters: Vec<Filter>,
}

/// A Save dialog (Save As, Publish as PDF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRequest {
    pub title: String,
    pub default_dir: Option<PathBuf>,
    pub default_name: String,
    pub filters: Vec<Filter>,
}

pub trait FileDialogs {
    /// The file the user picked, or `None` when the dialog was cancelled.
    fn pick_open(&self, req: &OpenRequest) -> Option<PathBuf>;
    fn pick_save(&self, req: &SaveRequest) -> Option<PathBuf>;
}

/// The real dialogs.
pub struct Native;

impl FileDialogs for Native {
    fn pick_open(&self, req: &OpenRequest) -> Option<PathBuf> {
        let mut dlg = rfd::FileDialog::new().set_title(&req.title);
        for f in &req.filters {
            dlg = dlg.add_filter(&f.label, &f.extensions);
        }
        dlg.pick_file()
    }

    fn pick_save(&self, req: &SaveRequest) -> Option<PathBuf> {
        let mut dlg = rfd::FileDialog::new().set_title(&req.title);
        for f in &req.filters {
            dlg = dlg.add_filter(&f.label, &f.extensions);
        }
        dlg = dlg.set_file_name(&req.default_name);
        if let Some(dir) = &req.default_dir {
            dlg = dlg.set_directory(dir);
        }
        dlg.save_file()
    }
}
