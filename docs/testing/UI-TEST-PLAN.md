# UI test plan (egui_kittest)

Goal: every UI feature and expected behaviour of the editor (`crates/kernpage`) and the
launcher (`crates/kernpage-launcher`) is covered by a headless test that sends real
mouse / keyboard events through the whole UI (`cargo test`, no GPU, no window).

Why: the "shapes snap back after a drag" bug lived from editor v1 because unit tests never
sent real input (egui clears `press_origin` on the release frame). These tests press, move and
release one frame at a time, so that class of bug is caught.

## How the tests work

- Harness: `egui_kittest 0.37.0` (feature `eframe`; no `wgpu`/`snapshot`, so no GPU and no
  image snapshots; we assert on the document model, app state, painted rectangles and
  AccessKit labels). Dev-dependency of `crates/kernpage` only.
- Tests live inside the binary crate: `crates/kernpage/src/ui_tests/` (`#[cfg(test)] mod ui_tests`
  in `main.rs`; no `lib.rs` split, the binary is unchanged).
  - `support.rs` (also batch 4 helpers: `open_gallery`, `click_gallery_row`, `page_size`, `margins`, `undo_steps`): `TestUi` (builds the real `App` in a `Harness`), page-inches to screen
    conversion, `click_at`, `click_mods`, `double_click_at`, `right_click_at`, `drag`,
    `drag_begin` + `release_primary` (inspect the release frame), `key(CTRL, Key::Z)`,
    `type_text`, model accessors, `painted(id)`.
  - One module per area (`canvas_mouse`, `shortcuts`, `text_edit`, `ribbon_home`, `page_setup`, `view_chrome`, `documents`, `autorecover`, `backstage`, `ribbon_contextual`, `ribbon_other` exist: the whole editor is covered; the launcher is covered too, in its own crate: `dashboard`, `cards`, `details`, `download`, `settings`, `fit`).
  - Batch 5 helpers in `support.rs`: `TestEnv` (a private AutoRecover folder, settings file, recent list and scratch `files` folder per test, installed per thread because tests run in parallel; `TestUi::start_in(env)` starts the app in a prepared environment, e.g. with a snapshot already in the AutoRecover folder), `TestUi.dialogs` (see S-dlg below), `TestUi.env`.
  - Batch 6 helpers: `TestUi::ribbon_body()` (the controls of the tab body, without launchers and quick-access / status bar buttons), `greyed()`, `body(name)` / `click_body(name)` (a body control by exact label: the status bar has buttons with the same tooltips as "Whole Page"), `click_all_greyed_and_expect_nothing()` (a real click on every greyed control: no edit, same tab / tool / selection, no dialog, no edit mode).
  - Batch 6 seams (all `#[cfg(test)]`, no behaviour change): `backstage::seen(label, rect, enabled, selected)` records a widget in the ribbon probe, with label prefixes `nav:Back`, `tile:<caption>`, `recent:<file name>`, `tabrow:<tab>`, `link:<text>`, `flat:<button>`, `place:<place>`, `folder:<name>` (File view), `setting:<row>`, `exportrow:<row>`, `print:Print`, `printnav:prev|next` (Print / Export), `shape_gal:<row>,<col>` (Insert Shapes gallery), `size:Height|Width[:up|down]` (the size boxes and their spin arrows), `swatch:<n>` (Shape Fill / Outline palette), `gallery` and `scheme_gallery` (inert galleries). **S-os is done for the calls the editor makes**: `printing::test_os` supplies the printer list ("Test Laser", "Test Inkjet", the latter the default), records every `PrintJob` the Print button submits (`take_printed()`) and every Printer Properties request (`take_properties()`) instead of touching a printer; the page-range parser has its own unit tests; the Windows PDF renderer is checked by hand with the ignored `render_page_to_png` test (`KP_RENDER_PDF`, `KP_RENDER_OUT`, `KP_RENDER_DPI`). Also: `output::test_os` records what `open_with_system` was asked to open (Explorer, the PDF viewer; `take_opened()`) instead of starting it, and redirects Print's temp folder to the test's `files\print` (`set_print_dir`).
- Side effects are kept out: `init_env()` points `KERNPAGE_AUTORECOVER_DIR`,
  `KERNPAGE_SETTINGS_FILE` and `KERNPAGE_RECENT_FILE` at a per-process temp folder.
- **S-dlg is done** (`crates/kernpage/src/filedialog.rs`): `trait FileDialogs { pick_open(&OpenRequest), pick_save(&SaveRequest) }` with `OpenRequest { title, filters }`, `SaveRequest { title, default_dir, default_name, filters }` and `Filter { label, extensions }`. `App.dialogs: Box<dyn FileDialogs>`; the default `Native` calls rfd exactly as before (Open Publication, Save As, Publish as PDF, Insert / Change Picture: every rfd call in the editor goes through it). Tests install `support::Dialogs`: `script_open(Some(path) | None)`, `script_save(..)` (answers are consumed in order; **an unscripted dialog panics**), `opens()` / `saves()` (what each dialog was asked), `all_used()`. `TestUi::start` installs it, so no test can show a real dialog.
- Other seams added in batch 5 (all `#[cfg(test)]`, no behaviour change): `autorecover::test_env` (per-thread AutoRecover folder and settings file, because the env vars are process-wide), `AutoRecover::make_due()` (the timer fires on the next frame: injectable clock) and `flush()` (wait for the background writer), `App::window_title()` (the title last sent to the window), `App::exit_allowed()` (an exit was confirmed), `backstage::spin_probe` (where the minutes box's spin arrows are drawn), and `widgets::probe` entries `nav:<Entry>` (the File view's left column: `click_ribbon("nav:Options")`), `tab:File` and `export:Create a PDF`.
- Test seams in the app (all `#[cfg(test)]`, none change behaviour): `App.probe`
  (`canvas::Probe`: page origin on screen, zoom, the screen rectangle each shape was
  painted at in the last frame, and the clip rectangle shapes are painted through, which is the
  page when Scratch Area is off). Cursor icon: `harness.output().platform_output.cursor_icon`.
  Clipboard: the editor's own in-app clipboard (`app.clipboard`), no OS clipboard involved.
- Finding widgets: `harness.get_by_label("Delete Object")`, `.query_by_label(..)`, then
  `.click()` / `.click_secondary()` / `.type_text()`. Ribbon buttons carry their label as
  AccessKit label; icon-only buttons need their tooltip text ("Bold (Ctrl+B)") checked.
  Where a widget has no AccessKit label, a seam (below) or a `probe` field is added.
- Icon-only ribbon buttons (`widgets::Btn`) expose their tooltip as AccessKit label, so tests find them with `get_by_label("Bold (Ctrl+B)")`. In sections 2 and 3 the test names in the tables are indicative; the real names are in `ui_tests/shortcuts.rs` and `ui_tests/text_edit.rs`.
- Known harness note: kittest makes a missing glyph a panic; the ribbon's Character Spacing
  icon uses U+2194 which egui's bundled fonts lack, so the harness uses `allow_missing_glyphs()`.
  (Real app: probably tofu unless a system font fills in; worth a look separately.)

Legend. Status: ✅ tested in step 1, ⬜ planned, ➖ greyed "coming soon" (test only that it is
disabled and does nothing). Seam: **S-dlg** native file dialog (rfd) needs a "pick file"
function-pointer seam; **S-probe** needs a probe field (widget has no AccessKit label);
**S-clock** needs a controllable clock / timer; **S-net** needs a fake feed (launcher
already has `--feed-url`/devfeed); **S-os** needs a fake shell-open / explorer / clipboard
hook; blank = none needed.

---

## 1. Canvas mouse (module `canvas_mouse`) ~ 47 tests, 47 done (the picture submenu is tested in `documents`)

| Feature | Expected behaviour | Test | Status |
|---|---|---|---|
| Click a shape | selects it, handles appear, no edit | `click_selects_a_shape_and_empty_space_deselects` | ✅ |
| Click empty page | deselects | same | ✅ |
| Shift+click | adds / removes from selection | `shift_click_adds_and_removes_from_the_selection` | ✅ |
| Ctrl+click | same toggle as Shift | `ctrl_click_toggles_selection` | ✅ |
| Escape | clears selection, back to Select tool | `escape_clears_the_selection` | ✅ |
| Drag a shape | stays at the drop point, one undo step, Ctrl+Z restores, Ctrl+Y redoes | `dragging_a_shape_moves_it_to_the_drop_point_as_one_undo_step` | ✅ |
| No-flash rule | release frame paints the shape at the drop point | `the_release_frame_paints_the_shape_at_the_drop_position` | ✅ |
| Click without moving | no edit, no undo step | `a_click_without_movement_does_not_move_or_edit` | ✅ |
| Drag a multi-selection | all selected move together, one undo step, selection kept | `dragging_a_multi_selection_moves_every_selected_shape` | ✅ |
| Drag a group | members move with it | `dragging_a_group_moves_its_members` | ✅ |
| Resize by corner handle | opposite corner fixed | `dragging_the_bottom_right_handle_...`, `dragging_the_top_left_handle_...` | ✅ |
| Resize by edge handles (4) | only that edge moves | `edge_handles_resize_one_dimension` | ✅ |
| Resize minimum size | cannot go below the minimum | `resize_stops_at_minimum_size` | ✅ |
| Resize release frame | painted at the new size on the release frame | `the_release_frame_paints_the_resized_shape` | ✅ |
| Resize a line | end handles move the end, no min size | `line_end_handle_moves_the_end_and_has_no_minimum` | ✅ |
| Resize a group | members scale | `resizing_a_group_scales_members` | ✅ |
| Marquee | selects shapes fully inside; no edit | `marquee_around_both_shapes_selects_both`, `marquee_around_one_shape_...` | ✅ |
| Shift+marquee | adds to selection | `shift_marquee_adds_to_the_existing_selection` | ✅ |
| Marquee on empty | deselects | `marquee_on_empty_space_deselects_...` | ✅ |
| Rectangle / Oval / Line / Text Box tools: drag | creates shape of that kind at the drawn box, selected, tool resets, one undo step | `drawing_with_the_*_tool_...` (4) | ✅ |
| Tool click (no drag) | default-size shape at the click | `clicking_with_a_drawing_tool_creates_a_default_sized_shape` | ✅ |
| Tiny drag with a tool | falls back to the default size | `tiny_drag_with_a_tool_uses_default_size` | ✅ |
| Line tool + Shift | angle snaps to 0/45/90 | `shift_constrains_line_angle` | ✅ |
| Right-click a shape | selects it, menu: Cut, Copy, Paste, order, Group, Ungroup, Delete Object | `right_clicking_a_shape_selects_it_and_its_menu_deletes_it` | ✅ |
| Right-click inside multi-selection | keeps selection; "Delete Objects" | `right_click_in_multi_selection_keeps_it` | ✅ |
| Right-click empty page | Paste + Zoom submenu only | `right_clicking_empty_page_offers_paste_and_zoom_only` | ✅ |
| Right-click menu items (each) | Cut/Copy/Paste/Bring Forward/to Front/Send Backward/to Back/Group/Ungroup/Delete do their command | `context_menu_cut`, `_copy_then_paste`, `_bring_forward`, `_bring_to_front`, `_send_backward`, `_send_to_back`, `_group_and_ungroup`, `_delete_objects_...`, `_paste_is_disabled_...` | ✅ |
| Right-click picture | extra "Change Picture" submenu (Change Picture... / Remove Picture) | `documents::right_clicking_a_picture_offers_the_change_picture_submenu`, `right_click_remove_picture_deletes_it`, `a_rectangle_has_no_change_picture_submenu` | ✅ |
| Double-click a text box | enters edit mode | `double_clicking_a_text_box_edits_it_...` | ✅ |
| Double-click a table cell | edits that cell | `double_click_table_cell_edits_it` | ✅ |
| Click a cell of selected table | edits that cell | `click_selected_table_cell_edits_it` | ✅ |
| Click while editing | commits the text as one undo step | covered by the double-click test | ✅ |
| Cursor icons | Move over shape, resize icons over handles, crosshair with a tool | `cursor_icon_follows_hover` | ✅ |
| Scroll / wheel | pans at 100% zoom; Ctrl+wheel zooms | `wheel_scrolls_the_page`, `ctrl_wheel_zooms` | ✅ |
| Drag outside the page (scratch area) | shape can sit on scratch; hidden when Scratch Area off | `scratch_area_shapes_hidden_when_off` | ✅ |

## 2. Keyboard shortcuts (module `shortcuts`) 29 tests, all done (the Ctrl+O / Ctrl+S dialog paths are in `documents`)

| Shortcut | Behaviour | Test | Status |
|---|---|---|---|
| Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z | undo / redo | `ctrl_z_y_shift_z` | ✅ `ctrl_z_undoes_and_ctrl_y_and_ctrl_shift_z_redo` |
| Ctrl+A | selects all top-level shapes | `ctrl_a_selects_...` | ✅ |
| Delete / Backspace | deletes selection (not while editing text) | `delete_removes_selection` | ✅ |
| Arrow keys / Shift+arrows | nudge 0.02 in / 0.1 in; one undo each | `arrows_nudge` | ✅ |
| Ctrl+C / X / V | copy / cut / paste shapes (paste offset) | `copy_cut_paste_shortcuts` | ✅ |
| Ctrl+Shift+G / Ctrl+Shift+H | group / ungroup | `group_ungroup_shortcuts` | ✅ |
| Ctrl+B / I / U | toggle bold/italic/underline on selection or box | `format_shortcuts_toggle` | ✅ |
| Ctrl+L / E / R / J | alignment | `align_shortcuts` | ✅ |
| Ctrl+N / Ctrl+O | File > New / Open | `ctrl_n_opens_the_new_publication_page_of_the_file_view`, `ctrl_o_with_unsaved_changes_asks_first` | ✅ (Ctrl+O dialog: `documents::ctrl_o_asks_for_a_publication_file_and_opens_the_picked_one`, `cancelling_the_open_dialog_does_nothing`) |
| Ctrl+S / Ctrl+Shift+S | Save / Save As | `ctrl_s_saves_a_publication_that_has_a_file_without_a_dialog` | ✅ Ctrl+S with a path; Save As / first save in `documents` (`save_of_a_never_saved_publication_is_save_as_with_pub_first_then_pdf`, `save_as_offers_the_current_file_name_and_folder`) |
| Ctrl+P | opens File > Print | `ctrl_p_opens_print` | ✅ |
| Ctrl+Shift+N | insert page | `insert_page_shortcut` | ✅ |
| F9 | toggle 100% / whole page | `f9_toggles_zoom` | ✅ |
| PageUp / PageDown | previous / next page | `page_keys_change_page` | ✅ |
| Shortcuts ignored behind a modal prompt | nothing runs while an error/prompt is open | `shortcuts_blocked_by_prompt` | ✅ |

## 3. Text editing (module `text_edit`) 42 tests: all pass

| Feature | Behaviour | Test | Status |
|---|---|---|---|
| Typing in a text box | text changes; commit on click-away is one undo step | `typing_is_committed_as_one_undo_step_...` | ✅ |
| Backspace/Delete/Enter in editor | edits text, Enter adds paragraph | `enter_adds_paragraph` | ✅ |
| Arrow/Home/End/Shift+select | caret and selection | `caret_and_selection_keys` | ✅ |
| Select word (double-click in editor) | selects word | `double_click_selects_word` | ✅ |
| Escape in editor | commits and leaves edit mode | `escape_leaves_edit` | ✅ |
| Text clipboard in editor | Cut/Copy/Paste via context menu and Ctrl+X/C/V | `ctrl_c_in_the_editor_...`, `paste_in_the_editor_...`, `context_menu_in_the_editor_...` | ✅ via Event::Copy/Cut/Paste and `platform_output` (no OS clipboard) |
| Formatting selection | Bold/Italic/Underline/Font/Size/Color apply to selected runs only | `format_applies_to_selection` | ✅ |
| Formatting a whole box | with no edit, applies to all text | `format_whole_text_box` | ✅ |
| Caret only | char controls have nothing to apply | `caret_only_char_controls_noop` | ✅ |
| Alignment buttons | left/center/right/justify | `alignment_buttons` | ✅ |
| Line Spacing menu + Options dialog | see 4 and 8 | `ribbon_home` | ✅ |
| Table cell editing | Tab / Shift+Tab move cells, never insert a tab | `tab_moves_between_cells` | ✅ |
| Text overflow / linked boxes | text flows to next linked box | `a_linked_story_is_shown_split_across_its_frames` ✅; `text_typed_into_a_linked_frame_flows_on_into_the_next_frame` ✅ (re-flow on commit); `one_undo_after_a_flowing_edit_restores_both_frames` ✅ |
| Linked frames: overflow box, link arrows, Linking group, delete / copy / pages (module `linked_frames`, 19 tests) | docs/ui/LINKED-TEXT.md | overflow box only while the overflowing last frame or a standalone box is selected (shape or caret), red handles (`the_overflow_box_shows_only_while_the_overflowing_last_frame_is_selected`, `no_overflow_box_on_a_frame_that_is_not_last_and_none_when_the_text_fits`, `a_standalone_box_whose_text_does_not_fit_shows_the_box_when_selected`, `typing_into_a_standalone_box_turns_the_overflow_box_on_and_deleting_turns_it_off`); the box does nothing when clicked, none for several selected (`the_overflow_box_shows_in_text_edit_mode_and_clicking_it_does_nothing`, `no_boxes_are_drawn_with_several_frames_selected`); arrows on the edges, clicking = Next / Previous (`the_arrows_sit_on_the_edges_and_clicking_them_goes_next_and_previous`); Linking group states for standalone / first / middle / last / caret / several (`the_linking_group_follows_the_publisher_table`); Next / Previous incl. page switch and ends (`next_and_previous_enter_the_neighbour_frame_changing_page`); Create Link / Break inert, "Coming soon" (`create_link_and_break_show_their_state_but_do_nothing_yet`); delete middle / first / last / all with one Ctrl+Z (`deleting_a_middle_frame_relinks_the_chain_and_one_undo_restores_it`, `deleting_the_first_or_the_last_frame`, `deleting_every_frame_removes_the_story`); save after a delete asks and loses nothing (`saving_after_deleting_a_linked_frame_asks_and_loses_nothing`); paste gives an empty unlinked box at +9 pt, Ctrl+D inert (`pasting_a_linked_frame_makes_a_new_empty_unlinked_box_nine_points_off`, `ctrl_d_does_nothing`); duplicate / delete / move page (`duplicating_a_page_with_linked_frames_makes_empty_frames_in_a_new_chain`, `deleting_a_page_removes_its_frames_from_the_chain`, `moving_a_page_does_not_change_the_chain`) ✅. Core tests in `kernpage-core/tests/linked_story.rs` (phase 4 / 5 block). Not testable: Duplicate and Ctrl+drag copy (the editor has neither), the delete-page "contains objects" prompt (the editor has none), Text Fit (a greyed placeholder only). |
| Edit then undo | restores text | `undo_restores_text` | ✅ |
| Ribbon clicks keep text focus | clicking Bold keeps editor focused | `ribbon_keeps_edit_focus` | ✅ |

## 4. Home tab (module `ribbon_home`) 49 tests, all pass (Pictures button: enabled state here, the insert itself in `documents`)

Buttons are found through a test seam, `widgets::probe` (every ribbon button drawn in the last frame, with label, tooltip, rect, enabled and pressed state; `TestUi::ribbon("Cut")`, `click_ribbon`, `click_ribbon_arrow`, `click_tab`), because labelled buttons have no AccessKit name. Menus and dialogs are then found by AccessKit label.

Enabled: Clipboard (Paste, Cut, Copy), Font (name, size box, Grow/Shrink Font, Bold, Italic,
Underline, Font Color + palette, Clear Formatting), Paragraph (Align L/C/R/Justify, Line
Spacing menu + "Line Spacing Options..."), Objects (Rectangle/Oval/Line/Text Box tools, Shapes
menu, Pictures, Group, Ungroup), Arrange (Align menu incl. Relative to Margin Guides, Distribute
H/V, Text Wrapping menu: Square/Tight/Through/Top and Bottom/None, Columns One/Two/Three,
Rotate/Flip, Bring Forward/to Front, Send Backward/to Back), Pages group.
Greyed ➖: Copy Formatting, Styles, Find, Replace, Select, Subscript, Superscript, Edit Wrap Points,
More Layout Options..., More Columns..., More Rotation Options..., Insert Blank/Duplicate Page live in Page group.

| Feature | Behaviour | Test | Status |
|---|---|---|---|
| Tab selection | clicking a tab shows its groups; contextual tabs appear / disappear with the selection; mixed selection keeps the common tab; falls back to Home | `clicking_each_tab_shows_that_tabs_groups`, `contextual_tabs_appear_and_disappear_with_the_selection`, `a_contextual_tab_can_be_opened_and_falls_back_to_home_when_the_selection_goes`, `a_mixed_selection_keeps_only_the_tabs_both_kinds_have` | ✅ |
| Paste/Cut/Copy buttons | enabled by selection / clipboard; run command | `cut_and_copy_need_a_selection_and_paste_needs_something_copied`, `the_copy_button_copies_and_the_paste_button_pastes_a_duplicate`, `the_cut_button_removes_the_selection_and_puts_it_on_the_clipboard` | ✅ |
| Font name / size box | typing a size applies on Enter; invalid ignored; lists pick | `the_size_box_shows_the_current_size`, `typing_a_size_and_pressing_enter_applies_it_as_one_undo_step`, `an_invalid_size_is_ignored_and_the_box_goes_back_to_the_real_size`, `the_size_list_picks_a_size`, `the_size_list_scrolls_to_the_largest_sizes`, `the_font_name_list_applies_the_chosen_family`, `font_controls_are_greyed_when_no_text_is_selected` | ✅ (fixed: Enter in the size box was undone one frame later) |
| Grow/Shrink Font | step through size list; stop at 72 / 8 | `grow_and_shrink_font_step_through_the_size_list`, `grow_font_stops_at_72_and_shrink_font_at_8` | ✅ |
| Bold/Italic/Underline buttons | toggle, show pressed state | `bold_italic_and_underline_toggle_and_show_the_pressed_state`, `mixed_bold_shows_unpressed_and_a_click_makes_everything_bold` | ✅ |
| Font Color split | arrow opens palette; main applies last colour | `the_font_color_main_button_applies_the_last_colour`, `the_font_color_arrow_opens_the_palette_and_a_swatch_applies_and_becomes_the_last_colour` | ✅ |
| Clear Formatting | resets runs | `clear_formatting_resets_the_runs_in_one_undo_step` | ✅ |
| Align buttons | like shortcuts; pressed state | `the_align_buttons_align_every_paragraph_of_the_selected_box_and_show_pressed`, `the_paragraph_buttons_are_greyed_for_a_shape_without_text` | ✅ |
| Line Spacing menu | choices apply spacing, check mark | `the_line_spacing_menu_applies_the_chosen_multiple`, `the_line_spacing_menu_marks_the_current_spacing` | ✅ |
| Line Spacing Options dialog | prefilled, OK one undo step, Cancel / Esc discard, Exactly mode | `line_spacing_options_dialog_opens_prefilled_and_ok_applies_in_one_undo_step`, `line_spacing_options_cancel_and_escape_discard_the_changes`, `line_spacing_options_exactly_mode_applies_points` | ✅ (fixed: Esc was swallowed by the canvas shortcuts) |
| Object tool buttons | arm the tool; next canvas drag creates | `draw_text_box_arms_the_tool_shows_pressed_and_the_next_drag_creates_a_text_box`, `clicking_draw_text_box_twice_disarms_it` | ✅ |
| Shapes menu | Rectangle/Oval/Line pick | `the_shapes_menu_arms_rectangle_oval_and_line` | ✅ |
| Pictures button | opens file picker then inserts | `the_pictures_button_is_available_with_a_document` (enabled state); insert: `documents::the_home_pictures_button_inserts_the_picked_picture_at_its_natural_size`, `cancelling_insert_picture_adds_nothing`, `a_file_that_is_not_a_picture_shows_an_error`, `an_unreadable_picture_file_shows_an_error`, `a_jpeg_can_be_inserted_too` | ✅ |
| Group / Ungroup buttons | enabled rules (>=2, one group) | `group_and_ungroup_follow_the_enabled_rules` | ✅ |
| Align menu (6 aligns + distribute) | move shapes; margin guides option | `align_is_greyed_without_a_selection_and_its_items_need_two_objects`, `align_items_line_up_the_selected_objects`, `align_left_moves_to_the_leftmost_edge_of_the_selection`, `distribute_needs_three_objects_and_spaces_the_gaps_evenly`, `relative_to_margin_guides_aligns_a_single_object_to_the_margins`, `distribute_relative_to_margins_needs_two_objects` | ✅ |
| Text Wrapping + Columns menus | set wrap type / column count | `wrap_text_is_greyed_without_a_selection`, `the_wrap_text_menu_sets_each_wrap_type_and_marks_the_current_one`, `the_columns_menu_sets_one_two_or_three_columns_on_a_text_box`, `the_columns_button_is_greyed_for_a_shape_that_is_not_a_text_box` | ✅ |
| Rotate/Flip menu | rotate 90, flip H/V (lines only) | `rotate_right_and_left_turn_the_selection_by_90_degrees`, `flip_is_available_for_lines_only_and_more_rotation_options_is_greyed` | ✅ |
| Order buttons | z-order commands | `bring_forward_and_send_backward_move_one_step`, `the_split_arrows_offer_bring_to_front_and_send_to_back` | ✅ |
| Greyed buttons | disabled, no effect | `every_coming_soon_control_on_home_is_disabled_and_does_nothing`, `the_coming_soon_controls_stay_disabled_with_no_selection_too` | ✅ ➖ |

Ribbon look (pw-r3i0 step 1, pw-23e6): `the_ui_font_is_segoe_ui_when_installed` (system Segoe UI is first in the UI font family, page fallback unchanged), `the_size_box_is_wide_enough_for_12_and_10_5`, `the_size_box_in_the_ribbon_is_at_least_that_wide`. Colours and sizes are checked by eye with `--screenshot ... --scale 1.25`.

## 5. Insert tab (module `ribbon_other`) 9 tests, all pass

Enabled: Pages (Insert Blank Page, Insert Duplicate Page), Illustrations (Pictures, Shapes menu), Tables (Table, see 7a), Text (Draw Text Box). Greyed ➖ (17, exactly these and nothing else): Online Pictures, Picture Placeholder, Page Elements, Calendars, Borders & Accents, Advertisements, Contact Details, Styled Text, Insert File, Symbol, Date & Time, Object, Link, Bookmark, Header, Footer, Page Number.

| Feature | Test | Status |
|---|---|---|
| Live and greyed controls | `the_insert_tab_has_working_and_greyed_controls_as_planned` | ✅ |
| Every greyed control inert | `every_greyed_insert_control_does_nothing` | ✅ ➖ |
| Insert Blank Page (after the current page, shown, one undo step) | `insert_blank_page_adds_a_page_after_this_one_and_shows_it`, `insert_blank_page_goes_after_the_current_page_not_at_the_end` | ✅ |
| Insert Duplicate Page | `insert_duplicate_page_copies_the_page_and_its_shapes`, `the_insert_page_menu_lists_blank_and_duplicate_and_opening_it_changes_nothing` | ✅ |
| Draw Text Box (arm, pressed, draw, disarm) | `the_insert_tabs_draw_text_box_arms_the_tool_and_clicking_again_disarms_it` | ✅ |
| Shapes menu | `the_insert_tabs_shapes_menu_arms_each_tool` | ✅ |
| Pictures | `documents::the_insert_tab_pictures_button_does_the_same` | ✅ |
| Tabs greyed without a publication | `the_tabs_are_greyed_without_a_publication` | ✅ |

## 6. Page Design, Data, Review, Help, View tabs, ribbon minimize and fit (module `ribbon_other`) 30 tests: 29 pass, 1 ignored (tab strip BUG)

| Tab | Enabled | Greyed ➖ | Tests | Status |
|---|---|---|---|---|
| Page Design | Page Setup group (module `page_setup`), Delete (more than one page) | Change Template, Options, Guides, Move, Rename, Delete (one page), Align To check boxes, Page Background group (Fonts, Apply Image, Background, Master Pages; the group is collapsed into a pop-up at 1280 pt), Palettes gallery | `page_design_greyed_controls_are_disabled_and_do_nothing`, `the_page_background_group_holds_greyed_fonts_images_backgrounds_and_master_pages`, `page_design_delete_removes_the_current_page_and_is_undoable`, `the_palettes_gallery_is_shown_as_a_large_inert_gallery` | ✅ (the colour names / swatches are painted text: only the gallery's size and inertness are asserted) |
| Data (our own mock-up) | none | all 11 named controls and the 4 row-navigation icons; the row box is disabled | `the_data_tab_sits_between_page_design_and_review`, `the_data_tab_is_greyed_without_a_publication`, `data_has_every_control_greyed_and_none_does_anything`, `search_finds_the_data_tab_and_its_numbering_command` | ✅ ➖ |
| Review | none | all 5 | `review_has_every_control_greyed_and_none_does_anything` | ✅ ➖ |
| Help | Help (About box) | Report a Problem, Suggest an Idea (enabled; each opens a pre-filled GitHub issue in the browser, recorded in tests by `feedback::test_os`; no live support on purpose) | `the_help_button_shows_the_about_box_and_ok_closes_it`, `escape_and_enter_close_the_about_box`, `help_report_a_problem_and_suggest_an_idea_are_enabled_with_tooltips`, `clicking_report_a_problem_opens_a_prefilled_bug_issue_once`, `clicking_suggest_an_idea_opens_a_prefilled_idea_issue_once`, `search_finds_report_a_problem_and_suggest_an_idea`, `the_question_mark_beside_the_ribbon_shows_the_same_about_box` | ✅ |
| View | Normal, Single Page (selected), Show check boxes (Boundaries, Guides, Rulers, Page Navigation, Scratch Area), Zoom (100%, Whole Page, Page Width, zoom list) | Master Page, Two-Page Spread, Selected Objects, Arrange All, Cascade, Switch Windows, Fields, Graphics Manager, Baselines | `the_view_tab_has_working_and_greyed_controls_as_planned`, `clicking_normal_and_single_page_keeps_the_current_views`, `the_show_check_boxes_toggle_boundaries_guides_rulers_navigation_and_scratch_area`, `the_unavailable_show_check_boxes_are_greyed_and_inert`, `the_view_tab_zoom_buttons_set_the_zoom_mode`, `the_view_tab_zoom_list_picks_a_zoom` (Rulers / Page Navigation effects: `view_chrome`) | ✅ (Boundaries / Guides: the app flag is asserted, not the drawing; Boundaries starts off like Publisher 2021) |
| Ribbon minimize (the ^ button) | hide / show, tab strip stays, page area moves up, a tab picked while minimized shows on expand, canvas still works, greyed without a publication | | `the_minimize_button_hides_the_ribbon_and_the_expand_button_brings_it_back`, `a_tab_picked_while_the_ribbon_is_minimized_is_shown_when_it_is_expanded`, `the_canvas_still_works_with_the_ribbon_minimized`, `the_minimize_button_is_greyed_without_a_publication` | ✅ |
| Ribbon fit | no control sticks out of the window or overlaps another, the tab strip does not run under Help / Minimize, at 1280 / 1000 / 800 (the window minimum) and, for the fixed tabs, 700 / 600; groups collapse into pop-ups and come back | | `every_fixed_tab_fits_the_window_at_the_minimum_width_and_wider`, `the_fixed_tabs_still_fit_below_the_minimum_window_width`, `the_contextual_tabs_fit_the_window_at_the_minimum_width_and_wider`, `narrowing_the_window_shrinks_groups_and_widening_it_restores_them`, `a_collapsed_group_opens_a_pop_up_with_its_controls_and_closes_on_an_outside_click`, `the_ribbon_fit_survives_switching_tabs_while_narrow` | ✅ |
| Tab strip at narrow width | with a text box selected, below ~800 pt the Text Box tab runs under Help / Minimize | `the_tab_strip_does_not_run_under_the_help_and_minimize_buttons_at_700` | ignored: BUG (needs the strip to scroll or the tabs to shrink; larger than a small fix) |

## 7. Contextual tabs (module `ribbon_contextual`) 41 tests, all pass

Tab appears only for a matching selection (`context_tabs`): Shape Format (text box or shape), Text Box (text box), Picture Format (picture), Table Design + Layout (table); mixed selections keep only common tabs. The tests drive the real controls: Height / Width boxes (type + Enter, spin arrows), the Insert Shapes gallery cells, Shape Fill / Outline (main button, arrow palette, No Fill / No Outline, Weight submenu), the Arrange group, Text Box font group and Columns, Picture Format Pictures..., plus every greyed control of every tab (real click, nothing changes).

| Area | Tests | Status |
|---|---|---|
| Tabs per object kind, tab stays / falls back to Home | `each_kind_of_object_gets_its_own_contextual_tabs`, `a_contextual_tab_stays_while_the_selection_still_has_it`, `picking_the_contextual_tab_does_not_change_the_selection_or_the_document` (mixed selections: `ribbon_home`) | ✅ |
| Shape Format live / greyed | `shape_format_has_working_and_greyed_controls_as_planned`, `shape_format_greyed_controls_do_nothing` (Edit Shape, Change Shape, Shape Effects, Measurement, Group / Ungroup for one object, the style gallery and 17 of the 21 gallery cells) | ✅ ➖ |
| Edit Text, shape gallery | `edit_text_starts_editing_a_selected_text_box_only`, `the_shape_gallery_cells_arm_the_draw_tools`, `a_gallery_tool_draws_the_shape_on_the_next_drag_as_one_undo_step`, `escape_disarms_a_gallery_tool_and_the_cell_arms_it_again` | ✅ |
| Height / Width | `the_height_and_width_boxes_show_the_selected_objects_size`, `typing_a_width_and_pressing_enter_resizes_in_one_undo_step`, `typing_a_height_with_a_unit_is_accepted`, `an_invalid_size_is_ignored_and_the_box_shows_the_real_size_again`, `the_spinner_arrows_step_the_size_by_a_tenth_of_an_inch`, `the_spinner_never_goes_below_the_smallest_size`, `the_size_boxes_are_greyed_for_a_multiple_selection_and_do_nothing`, `the_size_boxes_resize_pictures_and_tables_too` | ✅ |
| Fill / Outline | `the_shape_fill_button_applies_the_last_colour_as_one_undo_step`, `the_shape_fill_palette_applies_a_swatch_and_it_becomes_the_last_colour`, `no_fill_removes_the_fill`, `shape_fill_applies_to_a_text_box_too`, `the_shape_outline_button_adds_an_outline_in_the_last_colour`, `the_outline_palette_picks_a_colour_and_no_outline_removes_it`, `the_outline_weight_menu_sets_the_line_width_and_keeps_the_colour`, `a_weight_turns_a_missing_outline_on_in_black`, `fill_and_outline_apply_to_every_selected_shape_in_one_undo_step` | ✅ |
| Arrange group (order, wrap, rotate, align, group / ungroup) on the tabs | `the_shape_format_arrange_group_orders_wraps_rotates_and_aligns`, `the_shape_format_group_and_ungroup_buttons_work_on_two_shapes`, `the_align_menu_on_the_shape_format_tab_needs_two_objects`, `the_picture_format_arrange_group_works_on_a_picture`, `the_table_layout_arrange_group_works_on_a_table` | ✅ |
| Text Box tab | `the_text_box_tab_font_group_formats_the_whole_box`, `the_text_box_columns_menu_sets_the_column_count`, `text_box_greyed_controls_do_nothing` (Text Fit, Text Direction, Hyphenation, Margins, Create Link, Break, Next, Previous, WordArt, Drop Cap, 13 icon-only), `text_box_live_controls_are_enabled` | ✅ ➖ |
| Picture Format | `the_picture_format_pictures_button_inserts_a_picture`, `cancelling_pictures_on_the_picture_format_tab_adds_nothing`, `picture_format_greyed_controls_do_nothing` (Corrections, Recolor, Compress, Reset, Swap, Border, Effects, Caption, Arrange Thumbnails, Crop, Fit, Fill, Clear Crop, gallery); Change / Remove Picture in `documents` | ✅ ➖ |
| Table Design / Layout | `table_design_is_entirely_greyed_and_inert`, `table_layout_has_only_rows_columns_arrange_and_size_controls_live` (Rows & Columns: see 7a; merge, split, gridlines, select stay greyed), `a_table_has_no_fill_or_outline_command_and_its_tabs_stay_while_editing_a_cell`; cell editing and Tab: `canvas_mouse`, `text_edit` | ✅ ➖ |

## 7a. Tables (module `tables`) 27 tests, all pass (docs/ui/TABLES.md)
Real pointer clicks and key presses; the grid is read through `table_ui::probe` (it is painted, not widgets).

| Feature | Test | Status |
|---|---|---|
| Insert > Table drop-down: 10 x 8 grid, "Insert Table" header, "Insert Table..." row; hover highlights the top-left block and shows "CxR Table" (3x2, 10x8, 1x1, none off the grid); hovering inserts nothing | `the_table_button_opens_a_ten_by_eight_grid_named_columns_x_rows` | ✅ |
| Click 3x2: 2 rows x 3 columns at `default_table_rect`, selected, Select tool, Table Design / Layout tabs, one undo step, Ctrl+Z removes | `clicking_three_by_two_inserts_the_default_table_and_selects_it`, `the_biggest_square_inserts_ten_columns_and_eight_rows`, `the_table_is_inserted_on_the_current_page`, `the_table_button_is_live_on_home_too_and_does_the_same` | ✅ |
| Create Table dialog: first values 2 rows / 6 columns; typed values + Enter; OK button; Esc and Cancel insert nothing; 0 / text / empty / negative keep it open with a message until fixed; a million cells shows the editor's error | `insert_table_dialog_starts_with_two_rows_and_six_columns`, `typed_values_and_enter_insert_the_table`, `the_ok_button_inserts_too_and_the_untouched_defaults_make_two_by_six`, `escape_and_cancel_close_the_dialog_without_a_table`, `invalid_counts_keep_the_dialog_open_with_a_message_until_fixed`, `a_huge_table_reports_the_command_error` | ✅ |
| Enabled states: whole table selected = only Delete (menu: Delete Table); a cell (caret) = Insert x4 and all three menu items; Publisher's menu order Columns, Rows, Table; Select, View Gridlines, Merge, Split, Diagonals stay "Coming soon" | `with_the_whole_table_selected_only_delete_table_is_enabled`, `with_a_cell_selected_everything_in_rows_and_columns_is_enabled`, `the_delete_menu_lists_columns_rows_table_in_publishers_order`, `the_rest_of_the_layout_tab_stays_coming_soon` | ✅ |
| Insert Above / Below / Left / Right: counts, table grows, caret moves into the new track, text of other cells kept, typing continues in the caret cell, one undo step each (Ctrl+Z with the cell untouched undoes the document) | `insert_above_adds_a_row_above_the_caret_cell_and_moves_the_caret_into_it`, `insert_below_left_and_right_place_the_caret_in_the_new_track`, `inserting_keeps_the_text_of_the_other_cells_and_adds_empty_ones`, `the_caret_keeps_working_after_an_insert` | ✅ |
| Delete Rows / Columns (caret to the track that took its place, or the one before when it was last), Delete Table (from the frame and from a cell), undo restores | `delete_rows_removes_the_caret_row_and_keeps_a_caret_cell`, `delete_columns_removes_the_caret_column_and_keeps_a_caret_cell`, `delete_table_removes_the_whole_object`, `delete_table_works_from_a_cell_too` | ✅ |
| Last row / column: error dialog "...the last row of a table cannot be deleted...", nothing changed, no undo step, caret kept | `deleting_the_last_row_or_column_shows_the_editors_error_and_changes_nothing` | ✅ |
| Save after UI edits and reopen: same rows, columns and cell count (grid insert + Insert Below / Right + Delete Columns; dialog insert) | `a_table_edited_in_the_ribbon_saves_and_reopens_with_the_same_grid`, `a_table_inserted_from_the_dialog_saves_and_reopens` | ✅ |

Not Publisher-verified (TABLES.md section 8): where the caret lands after Insert / Delete, multi-cell selections (one row / column per click here), and whether Publisher remembers the dialog's last values (we always start at 2 / 6).

## 7b. Bullets and numbering (module `lists`) 31 tests, all pass (docs/ui/BULLETS.md, pw-cibs step 3)
Real pointer clicks and key presses. The gallery tiles, the dialog's bullet buttons and the character grids are painted shapes, so their rectangles come from `lists_ui::probe` (area, name, rect, orange / chosen state); text boxes, combo boxes and buttons are found through AccessKit. No Tab / Enter / Backspace list keys (deferred by the owner).

| Feature | Test | Status |
|---|---|---|
| Home > Paragraph: Bullets and Numbering split buttons (arrow opens the gallery), Decrease / Increase Indent; greyed with no text to format | `the_buttons_are_greyed_with_no_text_to_format`; `ribbon_home::every_coming_soon_control_on_home_is_disabled_and_does_nothing` (six icon-only controls stay greyed) | ✅ |
| Bullets gallery: header "Bullet Styles", 7 tiles of 90 x 90 in rows of three (None, Small, Large, Filled Diamond, Arrow, Open Diamond, Asterisk), footer "Bullets and Numbering...", None highlighted with no list | `the_bullets_gallery_has_publishers_header_tiles_and_footer` | ✅ |
| Each bullet tile: Symbol code / size (183 10, 183 12, 168, 222, 224, 42), hanging 18 pt indent on every paragraph, the glyph the canvas draws, one undo step, gallery closes | `each_bullet_tile_puts_its_bullet_on_every_paragraph_of_the_box` | ✅ |
| Current tile gets the orange border (the 12 pt dot is not the 10 pt one); None clears the list and the indents; a numbered selection highlights no bullet tile | `the_tile_of_the_current_bullet_gets_the_orange_border_and_none_clears`, `a_numbered_selection_highlights_no_bullet_tile` | ✅ |
| Bullets / Numbering main buttons toggle the default bullet / `1.` and show pressed | `the_bullets_button_itself_toggles_the_default_bullet`, `numbering_none_clears_and_the_numbering_button_toggles` | ✅ |
| Numbering gallery: None + 8 tiles in Publisher's order (`1.`, `1)`, `I.`, `A.`, `a.`, `a)`, `i.`, `i)`), type / separator / labels drawn, current tile highlighted, one undo step | `the_numbering_gallery_has_none_and_eight_tiles_named_by_their_sample`, `each_numbering_tile_numbers_the_paragraphs_and_is_highlighted_afterwards` | ✅ |
| Choosing a tile keeps the paragraphs' list indent | `choosing_a_tile_keeps_the_list_indent_of_the_paragraphs` | ✅ |
| Footer of either gallery opens the dialog (Bullets tab / Numbering tab); Cancel and Esc change nothing | `the_footer_of_each_gallery_opens_the_dialog_on_its_tab`, `cancel_leaves_the_document_alone` | ✅ |
| Bullets tab: 7 buttons (No Bullet + types 1-6), No Bullet chosen with no list; Size, Character... and Indent greyed until a bullet is chosen; choosing sets its own size (12pt for the large dot) and 0.25" | `the_bullets_tab_has_seven_buttons_and_no_bullet_greys_size_character_and_indent` | ✅ |
| Nothing applies until OK; OK is one command (one undo step, Ctrl+Z undoes it all) | `the_dialog_applies_nothing_until_ok_and_ok_is_one_undo_step` | ✅ |
| Pre-fill from the selection (bullet: type, size, indent, tab; numbered: format, separator, start, indent, tab) | `the_dialog_opens_pre_filled_from_a_bulleted_selection`, `the_numbering_tab_lists_the_formats_and_separators_and_opens_pre_filled` | ✅ |
| Numbering tab: Format list, Separator list (None . ) ] : () [] -- and the ninth), Start at, Indent list by; OK applies format + separator + start + indent; spinner arrows; unusable box greys OK and Enter | `changing_format_separator_start_and_indent_applies_on_ok`, `the_separator_list_has_publishers_nine_entries`, `the_spinner_arrows_step_the_boxes`, `a_box_that_holds_nothing_usable_keeps_ok_from_closing_the_dialog` | ✅ |
| Bullets tab with No Bullet and OK clears a numbered list | `the_numbering_tab_with_the_no_bullet_button_clears_the_list` | ✅ |
| Bullet Character dialog: 32 x 7 grid, recently used row, "Symbol: n", Character code box, "from: Symbol (decimal)"; pick a cell, OK / Cancel back to the Bullets tab; typing a code; font combo; session memory of recent picks | `the_bullet_character_dialog_has_a_32_column_grid_recent_symbols_and_the_code_boxes`, `picking_a_cell_and_ok_sets_the_bullet_character_back_in_the_bullets_tab`, `typing_a_character_code_selects_that_character`, `the_font_combo_changes_the_font_and_the_recent_row_remembers_picks_for_the_session` | ✅ |
| Increase / Decrease Indent: 0.25 in steps, plain paragraphs move the left indent, list paragraphs the list indent (first line hangs), Decrease greyed and inert at 0, one undo step per click | `increase_and_decrease_indent_step_a_quarter_inch_and_stop_at_zero`, `the_indent_buttons_move_the_list_indent_of_list_paragraphs` | ✅ |
| Selection: only the paragraphs touched by a selection or the caret; the dialog uses the selection of the moment it opened; text untouched | `a_selection_formats_only_the_paragraphs_it_touches_and_a_caret_only_its_own`, `the_dialog_applies_to_the_paragraphs_selected_when_it_opened` | ✅ |
| Undo through the Ctrl+Z shortcut | `ctrl_z_while_editing_text_undoes_one_list_step_after_the_edit_is_committed` (selected box), others above | ✅ |
| Inside a table cell (caret in the cell: that cell's paragraph only; whole table selected: greyed) | `lists_work_inside_a_table_cell` | ✅ |

Guessed, not Publisher-verified (BULLETS.md section 7 still open): the Separator entries show the bare marks the UI Automation read gave (`.` `)` ...; the ninth entry shows U+3001), the last three Format entries (`One, Two, Three ...`, `First, Second, Third ...`, `01, 02, 03, ...`), the spinner steps (Size 1 pt, Indent 0.1 in), Decrease / Increase Indent step (0.25 in) and what they do to a plain paragraph (left indent), the main Bullets / Numbering buttons toggling the default bullet / `1.`, the default recently-used symbols' font (Arial), the Bullet Character font list, and the footer having no list icon.

## 7c. Subscript and Superscript (module `script`) 7 tests, all pass (spec 4S, pw-df5)

| Check | Test | Status |
|---|---|---|
| Both Home buttons are live, named "Subscript (Ctrl+=)" / "Superscript (Ctrl+Shift+=)", start unpressed | `buttons_are_live_named_and_start_unpressed` | ✅ |
| Greyed when a rectangle is selected | `buttons_are_greyed_for_a_rectangle` | ✅ |
| Click on, click again off; pressed state follows the selection | `superscript_toggles_on_and_off` | ✅ |
| Turning one on turns the other off | `turning_one_on_turns_the_other_off` | ✅ |
| One undo step per click | `each_click_is_one_undo_step` | ✅ |
| Partial selection while editing splits the runs | `a_partial_selection_in_edit_mode_splits_the_runs` | ✅ |
| Shortcuts Ctrl+Shift+= (also reported as Plus) and Ctrl+= | `keyboard_shortcuts_toggle_script` | ✅ (shortcuts are the Word convention: to confirm against Publisher 2021) |

Same buttons on the Text Box Tools tab (shared Font group): counted in `ribbon_contextual`. Not done: Font dialog checkboxes (the app has no Font dialog yet).

## 8. Dialogs and prompts (module `dialogs`) ~ 18 tests

| Dialog | Behaviour | Tests | Status |
|---|---|---|---|
| Line Spacing Options | Before/After spin, Exactly vs Multiple, OK applies one undo step, Cancel/Esc discards | `ribbon_home::line_spacing_options_*` (see 4) | ✅ |
| Page Setup (dialog) | Width/Height/margins fields, layout type radio (One page per sheet enabled; Booklet, Email, Envelope, Folded card, Multiple pages, Web page), preview, validation messages (negative, leave room, not a length), OK applies one undo step | `the_launcher_arrow_opens_the_page_setup_dialog_prefilled`, `size_gallery_page_setup_row_opens_the_same_dialog`, `typed_units_set_a_custom_size_and_ok_is_one_undo_step`, `page_setup_ok_moves_shapes_by_half_the_size_change`, `invalid_input_shows_a_message_and_keeps_the_dialog_open`, `cancel_discards_the_dialog`, `escape_discards_the_page_setup_dialog`, `enter_is_ok`, `tab_moves_through_width_height_top_left_bottom_right`, `the_dialog_is_modal_...`, `the_layout_type_box_offers_one_page_per_sheet_and_greys_the_rest` (module `page_setup`) | ✅ (fixed: Esc was swallowed by the canvas shortcuts) |
| Custom Margins / Layout Guides | Top/Left/Bottom/Right, Margin/Grid/Baseline/Master Pages tabs ("Not available yet" for grid) | `layout_guides_dialog_shows_the_margin_guides_tab_and_the_unavailable_ones`, `layout_guides_box_order_is_left_right_top_bottom_and_ok_sets_only_the_margins`, `layout_guides_cancel_and_escape_discard`, `layout_guides_enter_is_ok_and_validation_keeps_it_open`, `layout_guides_margins_must_leave_room_on_the_page` | ✅ |
| Create New Page Size | Custom size creates size | `page_setup_is_enabled_and_the_other_two_size_commands_are_greyed` (the dialog itself is greyed, nothing to test) | ➖ |
| Error dialog | modal text + OK; blocks shortcuts | `documents::opening_a_missing_file_shows_an_error_and_keeps_the_document` (OK closes it), `opening_a_damaged_file_shows_an_error_and_keeps_the_document` | ✅ (blocking of shortcuts: `shortcuts_blocked_by_prompt`) |
| Unsaved-changes prompt | Save / Don't Save / Cancel before Close/Open/New/Exit | `documents::{new,open,close,exit}_{cancel,dont_save,save,save_then_cancel_the_dialog}` (16), `escape_on_the_unsaved_question_cancels_it`, `nothing_is_asked_when_there_are_no_unsaved_changes` | ✅ (New is triggered through `App::request(Pending::New)`; the New page tiles belong to the backstage batch) |
| Unsupported-features prompt | lists edits that cannot be written; Save supported changes / Cancel (Esc cancels) | `documents::saving_changes_that_cannot_be_written_asks_first`, `cancel_on_the_unsupported_prompt_writes_nothing`, `escape_on_the_unsupported_prompt_cancels_it`, `save_supported_changes_writes_the_file`, `the_unsupported_prompt_blocks_the_keyboard_shortcuts` | ✅ |
| Window close with unsaved | close request shows the prompt (Pending::Exit), Cancel keeps the window | `documents::exit_*` (the close request is injected as a `ViewportEvent::Close`) | ✅ |
| Gallery drop-downs (Margins, Orientation, Size, Wrap, Columns) | open, pick item, close on outside click / Esc | `a_gallery_closes_on_escape_and_on_an_outside_click_without_changing_anything` (Margins, Orientation, Size; Wrap/Columns menus are in section 4) | ✅ (fixed: Esc was swallowed by the canvas shortcuts) |

## 9. File screen (module `backstage`) 60 tests, all pass

Left nav: Home, New, Open, Info, Save, Save As, Share, Export, Print, Close, Account, Feedback, Options, Back arrow. Entries, tiles, recent rows, links and buttons are found through the `backstage::seen` probe labels; page text through AccessKit; the Open / Save dialogs are the scripted fakes; Explorer and the PDF viewer are recorded (`output::test_os`).

| Page | Behaviour | Tests | Status |
|---|---|---|---|
| Nav | which entries are enabled with / without a publication, disabled entries inert, each entry selects its page (exactly one drawn selected), File tab opens / closes the view (Info with a publication, Home without), another ribbon tab leaves it, Back arrow returns untouched, Esc returns (fixed: Esc did nothing), Ctrl+N / Ctrl+P | `without_a_publication_only_the_always_available_entries_are_enabled`, `with_a_publication_every_entry_and_the_back_arrow_are_enabled`, `clicking_a_disabled_entry_does_nothing`, `each_entry_opens_its_page_and_is_drawn_selected`, `the_back_arrow_returns_to_the_publication_untouched`, `escape_returns_to_the_publication_from_every_page`, `escape_without_a_publication_stays_in_the_file_view`, `the_file_tab_opens_and_closes_the_file_view`, `clicking_another_ribbon_tab_leaves_the_file_view`, `the_ribbon_and_the_canvas_are_not_drawn_in_the_file_view`, `ctrl_n_and_ctrl_p_open_the_new_and_print_pages_from_anywhere`, `opening_a_publication_from_any_page_closes_the_file_view` | ✅ |
| Save / Save As / Close entries | Save of a new publication asks for a file and returns; cancel stays; a changed saved file is written without a dialog; an unchanged one just returns (fixed: stayed in the File view); Save As page (This PC, Browse, Recent Folders rows run the dialog); Close (clean closes, dirty asks, Cancel keeps the view) | `the_save_entry_of_a_new_publication_asks_for_a_file_and_returns_to_the_editor`, `cancelling_the_save_entrys_dialog_stays_in_the_file_view`, `the_save_entry_writes_a_changed_saved_publication_without_asking`, `the_save_entry_of_an_unchanged_saved_publication_just_returns`, `the_save_as_page_offers_this_pc_and_browse`, `save_as_browse_asks_for_a_file_and_saves_there`, `save_as_lists_the_recent_folders_and_a_click_runs_the_save_dialog`, `the_close_entry_closes_a_clean_publication_and_shows_home`, `the_close_entry_asks_first_when_there_are_unsaved_changes` | ✅ |
| Home | Welcome, tiles (Blank 8.5 x 11, Blank 11 x 8.5, Church Bulletin enabled; More Blank Page Sizes greyed), tile starts the new publication (unsaved, no undo step, asks first when dirty), More templates / More publications links, Recent tab selected / Pinned greyed, 8 newest recent rows, click opens, missing file shows the error | `the_home_page_shows_the_new_tiles_and_the_open_list`, `the_blank_portrait_tile_starts_a_new_letter_publication`, `the_blank_landscape_tile_starts_a_new_landscape_publication`, `the_church_bulletin_tile_starts_the_bulletin_template`, `more_blank_page_sizes_is_greyed_and_inert`, `a_new_tile_with_unsaved_changes_asks_first`, `a_clean_publication_is_replaced_by_a_new_tile_without_asking`, `more_templates_opens_the_new_page_and_more_publications_the_open_page`, `the_pinned_tab_is_greyed_and_inert`, `the_home_page_lists_the_eight_newest_recent_publications_and_a_click_opens_one`, `a_recent_row_whose_file_is_gone_shows_an_error`, `a_recent_row_with_unsaved_changes_asks_first` | ✅ |
| New | Blank tiles, Online tab (offline message, disabled search), Built-in tab (Church Bulletin live; Order of Service, Prayer List and the 20 categories greyed and inert), tab remembered | `the_new_page_shows_blank_tiles_and_the_office_tab_is_offline`, `the_new_pages_blank_tiles_create_publications`, `the_built_in_tab_offers_the_church_bulletin_and_greys_the_rest`, `the_built_in_church_bulletin_tile_creates_the_bulletin`, `the_new_page_remembers_its_tab_and_can_go_back_to_office` | ✅ |
| Open | places, Recent / Publications selected, Folders greyed, Browse runs the Open dialog, all recent rows (up to 20), click opens and moves to the top | `the_open_page_shows_places_tabs_and_an_empty_recent_list`, `the_open_pages_browse_button_runs_the_open_dialog`, `the_open_page_lists_all_recent_publications_and_a_click_opens_one`, `a_recent_file_that_no_longer_exists_is_not_listed_after_a_restart` | ✅ |
| Info | name, folder breadcrumb or "Not saved yet", Pages, Page Size, Size, Last Saved, Copy path (clipboard), Open file location (recorded), unsupported rows greyed | `info_of_a_new_publication_shows_its_facts_and_greys_the_file_buttons`, `info_counts_the_pages`, `info_of_a_saved_publication_shows_the_folder_size_and_date`, `info_copy_path_puts_the_file_path_on_the_clipboard`, `info_open_file_location_asks_the_system_to_open_the_folder`, `info_unsupported_rows_are_described_and_their_buttons_are_greyed` | ✅ |
| Print | sheet counter, prev / next arrows (greyed at the ends, inert), settings rows greyed and inert, Print writes the PDF and opens it (recorded), Ctrl+P | `the_print_page_shows_the_first_sheet_and_the_page_counter`, `the_print_preview_arrows_step_through_the_sheets_and_stop_at_both_ends`, `a_single_page_publication_has_both_preview_arrows_greyed`, `the_print_settings_are_greyed_and_inert`, `the_print_button_writes_a_pdf_and_opens_it_in_the_viewer`, `ctrl_p_then_the_print_button_prints_unsaved_text_edits_too` | ✅ (the last one only checks that a PDF is opened after Ctrl+P) |
| Export | Create a PDF is the selected row, the 5 others greyed and inert, the big Create button is the only way (writes in `documents`) | `the_export_page_offers_create_pdf_and_greys_the_other_rows`, `clicking_the_greyed_export_rows_does_nothing`, `clicking_the_selected_create_pdf_row_itself_does_nothing_the_big_button_creates`; `documents::export_*` | ✅ |
| Share / Account | notice pages, also without a publication | `share_account_and_feedback_are_notice_pages` | ✅ |
| Feedback | two links (Report a Problem, Suggest an Idea) open pre-filled GitHub issues | `the_feedback_page_offers_the_two_github_issue_links` | ✅ |
| Options | Save category, the four greyed options and the personal templates box, OK / Cancel return (Home without a publication); AutoRecover box: `autorecover` | `the_options_page_has_the_save_category_and_greyed_extra_options`, `ok_and_cancel_on_the_options_page_return_to_the_publication` | ✅ |

## 10. Pages pane, rulers, zoom, status bar (module `view_chrome`) 24 tests, all pass

Seams: `nav::probe` (thumbnails with current flag, close button), `canvas::Probe.ruler_h / ruler_v` (ruler rectangles and the cursor marker position). The window title row is done in batch 5.

| Feature | Behaviour | Tests | Status |
|---|---|---|---|
| Pages pane | thumbnails, current page highlighted, click selects page, Hide Page Navigation, "Pages" header | `the_pages_pane_has_a_header_and_one_thumbnail_per_page_in_order`, `the_current_page_thumbnail_is_highlighted`, `clicking_a_thumbnail_selects_that_page_and_drops_the_selection`, `the_close_button_hides_the_pane_and_the_view_tab_shows_it_again` | ✅ |
| Pages pane right-click | Insert Blank Page, Insert Duplicate Page, Delete Page (disabled with one page), Move Page Up/Down | `the_pages_menu_lists_its_commands`, `delete_and_move_are_greyed_when_they_cannot_apply`, `insert_blank_page_adds_a_page_after_the_clicked_one_...`, `insert_duplicate_page_copies_the_page_and_its_shapes`, `delete_page_removes_it_keeps_the_view_sensible_and_undo_restores_it`, `move_page_up_and_down_reorder_the_pages_and_follow_the_moved_page` | ✅ |
| Rulers | horizontal and vertical, hover marker, hidden with View > Rulers | `rulers_are_drawn_along_the_top_and_left_of_the_page_area`, `the_rulers_marker_follows_the_cursor_and_disappears_off_the_canvas`, `view_rulers_hides_and_shows_both_rulers` | ✅ |
| Status bar | cursor position in inches, page n of N (selected object position/size is not shown by the app) | `the_status_bar_shows_the_page_number_and_count`, `the_status_bar_shows_the_cursor_position_in_inches_over_the_page` | ✅ |
| Zoom controls | Whole Page, Zoom In/Out buttons, slider, percent, clamp to MIN/MAX | `the_status_bar_starts_at_whole_page_and_shows_the_zoom_percentage`, `zoom_in_and_zoom_out_step_through_the_zoom_levels`, `zoom_in_from_a_fit_mode_goes_to_the_next_level_above_the_fitted_zoom`, `zoom_buttons_stop_at_the_largest_and_smallest_zoom`, `the_zoom_slider_sets_a_custom_zoom_and_clamps_at_both_ends`, `zoom_widgets_do_not_change_the_document_or_the_selection` | ✅ |
| View buttons | Single Page, Two-Page Spread (➖ "coming soon" tooltip) | `the_view_buttons_single_page_is_selected_and_two_page_spread_is_greyed` | ✅ ➖ |
| Whole page / page width fit | zoom recalculates on window resize | `the_whole_page_button_fits_the_page_in_the_window`, `whole_page_and_page_width_follow_the_window_size` | ✅ |
| Window title | "Name*" while dirty, "(Recovered)" | `documents::the_window_title_shows_a_star_while_there_are_unsaved_changes`, `the_window_title_is_the_product_name_without_a_publication`, `autorecover::a_crashed_sessions_snapshot_is_recovered_on_start` (reads `App::window_title()`) | ✅ |

## 11. Document operations and AutoRecover (modules `documents`, `autorecover`) 99 tests, all pass

`documents` 62 tests, `autorecover` 37. Dialog paths use the S-dlg fake (`TestUi.dialogs`); every test has its own `TestEnv`.

| Feature | Behaviour | Tests | Status |
|---|---|---|---|
| Open (Ctrl+O, File > Open > Browse) | Open dialog titled "Open Publication", filters .pub then All files; picked file opens; Cancel does nothing; recent list updated (newest first, no duplicates, persisted) | `ctrl_o_asks_for_a_publication_file_and_opens_the_picked_one`, `cancelling_the_open_dialog_does_nothing`, `cancelling_the_open_dialog_on_the_start_screen_stays_there`, `file_open_browse_opens_the_picked_file`, `opening_a_file_adds_it_to_the_recent_list_at_the_top` | ✅ |
| Open damaged / missing file | error dialog text, document kept, not added to recent | `opening_a_missing_file_shows_an_error_and_keeps_the_document`, `opening_a_damaged_file_shows_an_error_and_keeps_the_document` | ✅ |
| Save of a never-saved doc | Save As dialog, "Test.pub", filters .pub first then PDF, no folder; Cancel writes nothing and stays unsaved; a name without extension gets .pub | `save_of_a_never_saved_publication_is_save_as_with_pub_first_then_pdf`, `cancelling_the_first_save_writes_nothing_and_stays_unsaved`, `a_name_typed_without_an_extension_gets_dot_pub` | ✅ |
| Save As | default name and folder of the current file, publication moves to the new file, Cancel changes nothing, PDF choice exports without saving | `save_as_offers_the_current_file_name_and_folder`, `file_save_as_browse_runs_the_save_as_dialog`, `cancelling_save_as_changes_nothing`, `saving_as_pdf_exports_a_pdf_and_leaves_the_publication_unsaved` | ✅ |
| Save then reopen | same objects, kinds, rectangles, text, page size; Ctrl+S on an unchanged file writes nothing; saving adds to recent | `a_saved_publication_reopens_with_the_same_content`, `ctrl_s_on_an_unchanged_file_writes_nothing`, `saving_adds_the_file_to_the_recent_list` | ✅ |
| Unsupported-changes prompt | see 8 | 5 tests | ✅ |
| Unsaved-changes prompt (New / Open / Close / Exit x Save, Don't Save, Cancel, Save then cancel the dialog) | see 8 | 18 tests | ✅ |
| Dirty tracking | dirty after edit, clean after save and after undo-to-saved | `a_saved_publication_is_dirty_again_after_an_edit_but_clean_after_undo_to_the_saved_state` | ✅ |
| Window title | product name; "Name*"; "(Recovered)" | 3 tests | ✅ |
| Pictures | insert (Home and Insert tab buttons) at natural size, centred, selected, one undo step; cancel; non-picture and unreadable files show errors; JPEG; change keeps height and centre, width from the new proportions, undoable; remove (arrow menu and right-click) | 19 tests ("Insert Picture" and "Change Picture" blocks of `documents`) | ✅ (scale-down of oversize pictures is covered by the core unit tests) |
| Export PDF (File > Export > Create a PDF) | "Publish as PDF" dialog, default "<name>.pdf" and the file's folder, writes a PDF, success message, Cancel writes nothing | 4 tests | ✅ |
| Open sample .pub | shapes load, name shown | `with_sample` helper used everywhere | ✅ |
| Tables: edit cell, Tab nav, table tabs | see 1, 3 and 7 | `canvas_mouse::double_click_table_cell_edits_it`, `text_edit::tab_moves_between_cells`, `ribbon_contextual::table_*`, `tables::*` | ✅ |
| Groups, arrange/align, z-order | see 1, 4 | in `canvas_mouse` / `ribbon_home` | ✅ |
| Page setup via galleries | module `page_setup` (38 tests) | | ✅ |
| Options > Save page | checkbox + minutes box: defaults and saved settings shown; digits only; > 120 refused per keystroke; 0 and empty allowed while typing but not kept; Up/Down keys and spin buttons step and clamp 1-120 (0 counts as the old value); unchecked greys the box and the arrows; OK applies, saves to the settings file and leaves the page; Cancel discards | 15 tests in `autorecover` | ✅ |
| AutoRecover tick | snapshot after the interval when dirty (`AutoRecover save - <name>(xxxx).kprec`, meta, current content, typed-but-uncommitted text); none when unchanged / before the interval / checkbox off; no rewrite of unchanged content, same file reused | 8 tests | ✅ |
| AutoRecover removal | deleted on save, undo-to-saved (next tick), Don't Save on close, normal exit, opening another publication, turning the option off; kept on Cancel | 7 tests | ✅ |
| Recover on start | dead-pid snapshot opens as "<name> (Recovered)", dirty, snapshot file taken; Save = Save As at the original folder / name; Cancel keeps it recovered; Close asks; live-pid snapshot ignored; unreadable one moved to `unreadable\`; newest recovered, others wait | 7 tests | ✅ |

## 12. Launcher (`crates/kernpage-launcher`, `src/ui_tests/`, separate harness) 127 tests, all done

Same `Harness::new_eframe` approach as the editor. Support layer (`ui_tests/support.rs`): `TestUi` / `Setup` (size,
config text, installed version, editor present / running, install scope, recent list, install report, feed), a private
`TestEnv` per test (config, recent list, updates folder, editor stub, scratch files), `FeedServer` (the in-process loopback
feed signed with the test key: options for a real tiny installer, bad SHA, blocklist, tampered index, wrong key, expired,
an older release; a gate that holds file downloads so progress / cancel can be seen; files can be removed or changed mid-test)
and screen helpers (`has`, `click`, `click_last` for the top-most control when a modal covers the page, `hover` + tooltip,
`badges()`, `card_rect`, `rows()`, `controls()`, `combos()`, `click_peek` for the viewport commands of a click frame).

Seams (module `testhooks`, test builds only, per thread, behaviour-preserving): editor launch (`launch_editor`),
"Open file location", opening URLs, Notepad (`install::show_log`), Windows toasts (`notify`), tray (`tray::create`), the Run
key (`RegistryRunStore`), the helper start (`install::start_helper`), the editor process check (`install::process_running`),
the install scope, the config / recent / updates folders, the Open file dialog (panics when unscripted), "Quit launcher"
(recorded, the process is not ended) and what the frame painted without accessibility labels (status badges, card and recent
row rectangles). The feed URL is always a loopback server or a dead loopback port; the real feed is never contacted.

| Area | Features | Tests (module) | Status |
|---|---|---|---|
| Top bar / toolbar | title, DASHBOARD, Settings, Refresh (tooltip, checks the feed), Open file... (scripted dialog, Cancel), UPDATE ALL | `topbar_and_toolbar_controls_are_present`, `manual_mode_sends_nothing_until_refresh`, `open_file_launches_the_editor_with_the_picked_file`, `open_file_cancel_launches_nothing`, `update_all_starts_every_download` (`dashboard`) | ✅ |
| Dashboard states | no feed, checked / update available, up to date, newer installed, editor missing (NOT FOUND), blocked, blocked-but-update note, bad checksum entry, errors: unreachable, tampered, wrong key, expired, older serial; serial remembered | 15 tests in `dashboard` (`update_available_state`, `up_to_date_state`, `unreachable_feed_shows_notice_and_footer`, `tampered_feed_is_refused`, ...) | ✅ |
| Footer / modes | "Checking for updates...", automatic interval text, automatic mode checks at start | `footer_says_checking_during_a_check`, `footer_shows_the_automatic_interval`, `automatic_mode_checks_by_itself_at_start` (`dashboard`) | ✅ |
| Notice / report bars | notice Dismiss; install report success / 3010 / failure / no log; Show log -> recorded Notepad call; Dismiss; shown once | `notice_bar_shows_a_launch_error_and_dismisses`, `report_*` (4) (`dashboard`) | ✅ |
| Toasts | once per version, saved, none when notifications off | `an_update_toasts_once_and_is_saved`, `no_toast_when_notifications_are_off` | ✅ |
| Announce modal | content, Later (tooltip, holds for the session), What's new, Esc, Update and restart -> download -> helper call; skipped version not announced | 5 tests + `skipped_version_is_not_announced` (`dashboard`) | ✅ |
| Skip version | Skip this version / Stop skipping, persisted | `skip_this_version_hides_the_update_and_can_be_undone` | ✅ |
| Item cards | OPEN / OPEN x.y.z, UPDATE TO, card / name / tagline click opens details, add-on facts | `open_button_launches_the_editor`, `open_with_an_update_pending_still_opens_the_installed_one`, `update_button_downloads_the_new_version`, `clicking_the_card_body_opens_the_details`, `clicking_the_card_name_or_tagline_opens_the_details`, `add_on_card_shows_its_facts` (`cards`) | ✅ (fixed: name / tagline swallowed the click) |
| "..." menu | items, greyed Reinstall / Uninstall / Roll back with hover reasons, Release page, Copy link (clipboard), Open file location, Skip only with an update, add-on menu (Website, Release page, no app-only items) | `more_menu_*` (8), `greyed_menu_items_do_nothing_when_clicked`, `add_on_more_menu_*` (2) (`cards`) | ✅ |
| Add-on cards / stub | INSTALL (tooltip), stub "NOT BUILT YET", OK / Esc, nothing downloaded | `add_on_install_opens_the_stub_dialog`, `add_on_stub_closes_with_escape` | ✅ |
| Recent files | rows, NOT FOUND row, click opens, NOT FOUND click does nothing, remove from list (file kept), Clear list + confirm / Cancel / Esc (files kept), empty state, Refresh reloads | 9 tests (`cards`) | ✅ |
| Details modal | greyed links before a check, sections, features, info boxes, What's changed 6 / more / fewer, Full changelog, Website / GitHub / Releases, close (tooltip, Esc), previous / next, add-on details, buttons in the modal, blocked note | 10 tests (`details`) | ✅ |
| Rollback | Roll back enabled only with older versions, list, Close / Esc, confirm text + Cancel, Go back and restart (progress, helper call with the older version), failure + Retry, dev build | 7 tests (`details`) | ✅ |
| Download / install | progress + Cancel (tooltip) + resume, queue ("Waiting..."), error + Retry, wrong checksum, too short, too large, Ready, Install and restart -> helper call + Close command, helper failure notice, editor running (greyed + tooltip, becomes enabled), Update and restart waits for the editor, dev build (NotInstalled) disabled + tooltip | 13 tests (`download`) | ✅ |
| Auto-install policy | setting on + window visible (downloads only), window hidden (installs), hidden + editor running (waits), off, skipped version; close-to-tray hides the window | 7 tests (`download`) | ✅ |
| Settings | every control: language box, check mode radios, interval, three checkboxes (persisted), Version box greyed, Check now, appearance radios, tray (also failure), start at login (Run key, failure, registry is the truth), Report a problem greyed, Quit, Done / Esc, persistence across a restart, pre-releases change the offer | 19 tests (`settings`) | ✅ (fixed: auto-install checkbox was greyed) |
| Fit / theme | dashboard, bars, details, settings, announce / stub / confirm, rollback at 1093x798, 900x650, 760x520: nothing cut off, modal buttons on screen, scrolling reaches everything; dark / light palette | 8 tests (`fit`) | ✅ (fixed: long notice / report text overflowed) |
| Instance / tray icon / toast / Run key themselves | single instance mutex, real tray icon, real toast, real registry | the real OS pieces cannot be tested headless (their calls are recorded through the seams); covered by unit tests (`run_key`, `instance`) | n/a |

App bugs found while writing these tests, fixed (3): clicking a card's name or tagline did not open the details (selectable labels took the
click; one line in `item_card`); "Download and install updates automatically" was greyed out although the whole auto-install path exists
(Settings now saves it); the install-report and notice bars ran past the window edge with a long message (`Label::wrap`), which also
widened the whole page. Not changed: the Settings combo boxes (language, interval, version) have no accessible name (screen readers), so the
harness first-frame accessibility check is switched off only when the dialog is open at start; the stale Settings note "Installing updates and
choosing an older version arrive in the next step." and the greyed "Version" box are leftovers of the earlier step (rollback exists under "...").

---

## Counts

| Area | Planned tests | Done in step 1 |
|---|---|---|
| 1 Canvas mouse | ~48 | 47 |
| 2 Keyboard shortcuts | 29 | 29 |
| 3 Text editing | 41 | 41 (1 ignored) |
| 4 Home tab | 49 | 49 |
| 5 Insert tab | 9 | 9 (`ribbon_other`) |
| 6 Other tabs, minimize, ribbon fit | 26 | 26 (`ribbon_other`; 1 ignored) |
| 7 Contextual tabs | 41 | 41 |
| 8 Dialogs and prompts | ~18 | prompts done in `documents` (counted in 11) |
| 9 File screen | 60 | 60 |
| 10 Chrome (pages pane, rulers, zoom, status) | 24 | 24 |
| 11 Documents / AutoRecover | 99 | 99 (`documents` 62, `autorecover` 37) |
| 12 Launcher | ~40 | 127 (`dashboard` 35, `cards` 28, `details` 17, `download` 20, `settings` 19, `fit` 8) |
| 8 (Page Setup, Layout Guides, galleries only) | | 38 |
| Launcher total (`cargo test -p kernpage-launcher`) | | 191 (64 unit + 127 UI), about 15 s |
| Editor total (`cargo test -p kernpage`) | | 525 (1 ignored: tab strip at narrow width), about 71 s |

## Proposed batches (8-12 tests each, one area per batch, priority order)

Priority = where unseen input bugs hurt most (real mouse/keyboard on the editing surface) first.

1. Batch 2 done (canvas mouse remainder); only the picture submenu (S-dlg) is left in section 1.
2. Batch 3: Keyboard shortcuts (all of section 2). Needs no seams except S-dlg for Ctrl+O/S (stub).
3. Batch 4: Text editing (typing, selection keys, escape, formatting through shortcuts, table cell Tab).
4. Batch 5: Home tab clipboard / font / paragraph / alignment buttons, plus Bold/size box (find buttons by label).
5. Batch 6: Home tab objects / arrange / wrap / columns / rotate / order menus; greyed Home buttons.
6. Batch 7: Page setup galleries and dialogs (Page Setup, Layout Guides, Create New Page Size, validation).
7. Batch 8: Pages pane + Insert page/duplicate/delete/move + page keys; rulers, zoom and status bar.
8. Batch 9 (done as batch 5): native-dialog seam, Save / Save As / Open / unsaved prompts / error dialog / close handling / pictures / Export PDF (module `documents`).
9. Batch 10 (done as batch 5): AutoRecover + Options page (module `autorecover`).
10. Batch 11 (done as batch 6): File screen pages (nav, New, Open, Info, Print, Export, Account, Feedback; module `backstage`).
11. Batch 12 (done as batch 6): Contextual tabs (shape/text box/picture/table), pictures, tables, groups (module `ribbon_contextual`).
12. Batch 13 (done as batch 6): View / Insert / Page Design / Review / Help tabs, ribbon collapse and fit, all greyed buttons (➖) (module `ribbon_other`).
13. Batch 7 (launcher, done): harness + seams, dashboard, cards, details / rollback, download / install, settings, fit.

## What cannot be tested with kittest (and why)

- Real native dialogs (rfd), the system PDF viewer, Explorer, browser, clipboard of the OS, tray icon and toast notifications, run-at-login registry key: OS integration. Covered by seams (stub the call and assert it was requested) or existing unit tests.
- Pixel-exact appearance (fonts, icon art, colours): no image snapshots (needs wgpu/GPU). Layout is checked through rectangles and AccessKit instead; `--screenshot` remains the visual check.
- True OS-level window events (min/max, DPI change, drag of the title bar) and cursor image: harness covers only egui input. Cursor icon is readable from the egui output.
- Fonts installed on the machine: system font scan runs on a thread; tests use egui's bundled fonts, so text metrics differ from the real app (assert on model, not on glyph positions).
- Canvas underline (pw-enij): unit tests in `render::tests` lay a galley out with egui's bundled fonts and assert the segment sits 0.12 em below the baseline (not at the row bottom, also with extra line spacing), scales with size, follows a raised script run and skips non-underlined text; the on-screen look is not pixel-checked. Text inset (pw-dris): the canvas uses one inset (`INSET_PT`) on all four sides; the model has no per-box margins.
- Editor rows that stay untested headless (all else in sections 1 to 11 is covered): the drawing of View > Boundaries and Guides and of the colour scheme names and gallery glyphs (painted shapes: only the flag / the gallery size and inertness are asserted); the hover tooltip texts of greyed controls (only their disabled state and inertness are asserted); what Explorer and the PDF viewer do with the file (the request is recorded); how the ribbon looks in the real font (tests use egui's bundled fonts, so the fit tests have the same widths as the app only approximately).
