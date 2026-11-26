//! UI tests: drive the real editor UI headless with egui_kittest (mouse and keyboard events,
//! one frame per event) and assert on the document model and on what the canvas painted.
//!
//! Layout: `support` is the harness and input helpers; one module per area of the UI
//! (see docs/testing/UI-TEST-PLAN.md).

mod backstage;
mod demo;
mod canvas_mouse;
mod ribbon_contextual;
mod ribbon_other;
mod autorecover;
mod documents;
mod linked_frames;
mod linked_text;
mod logo;
mod masters;
mod idle;
mod lists;
mod script;
mod page_setup;
mod view_chrome;
mod ribbon_home;
mod shortcuts;
mod support;
mod tables;
mod update_notice;
mod text_edit;
mod theme;
