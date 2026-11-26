//! Static description of KernPage for the App details panel. Step 2 replaces this
//! with data from the signed update index.

pub struct Feature {
    pub title: &'static str,
    pub text: &'static str,
}

pub struct AppInfo {
    pub name: &'static str,
    pub tagline: &'static str,
    pub features: &'static [Feature],
}

pub const KERNPAGE: AppInfo = AppInfo {
    name: "KernPage",
    tagline: "Page layout and publishing. Opens and saves Publisher (.pub) files.",
    features: &[
        Feature {
            title: "Opens Publisher files",
            text: "Reads .pub files and shows text, shapes, pictures, tables, groups and linked text boxes.",
        },
        Feature {
            title: "Edits and saves .pub",
            text: "Change text and formatting, move and resize objects, and save back into the same file.",
        },
        Feature {
            title: "A familiar layout",
            text: "A ribbon and File screen with tabs and groups arranged in a way many Publisher users will recognise, with our own icons and colours. Some commands are still greyed out.",
        },
        Feature {
            title: "New content and pages",
            text: "Add text boxes, rectangles, ovals, lines, tables and pictures; use bullets and numbering; add, move, duplicate and delete pages; set page size and margins.",
        },
        Feature {
            title: "Export to PDF",
            text: "Save a publication as a PDF. Printing goes through your PDF viewer for now.",
        },
        Feature {
            title: "Open source, works offline",
            text: "Licensed MIT or Apache-2.0. Not affiliated with Microsoft. Still in development: no master pages, spell check, mail merge or templates yet.",
        },
    ],
};
