//! KernPage's editable document model, commands with undo/redo, and geometry
//! helpers. Contains no GUI code, so everything here is unit-testable anywhere.

pub mod autorecover;
pub mod command;
pub mod convert;
pub mod export;
pub mod geometry;
pub mod model;
pub mod native;
pub mod pagesetup;
pub mod picture;
pub mod recent;
pub mod save;
pub mod story;
pub mod sysfonts;
pub mod sysexe;
pub mod table;
pub mod template;
pub mod text;
pub mod wrap;

pub use command::{
    AlignMode, CharChange, Command, CommandError, Editor, StyleChange, TextFormat, ZOrder,
};
pub use convert::{from_pub, from_pub_flowed};
pub use export::{export_pdf, to_pub};
pub use geometry::{Handle, Rect};
pub use model::*;
pub use save::{
    write_atomic, FormatChange, FrameChange, LookChange, NewPicture, NewShape, NewTextBox,
    OrderChange, PageOp, PageSetupChange, NewTableObject, PictureChange, PictureDeletion, SaveError, SavePlan, SaveSource, StoryChange,
    TableChange, TableDeletion, TableRebuild,
};
