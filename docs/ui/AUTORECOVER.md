# KernPage: AutoRecover and document recovery, as Publisher 2021 shows it

Source: Publisher 2021 (version 16.0, Windows 11, 1920 x 1080 screen) driven by COM, simulated keyboard/mouse input and screenshots, observed on 2026-11-14. This file is **words only**: it records visible UI wording, registry settings, the files Publisher writes (names, sizes, timing) and behaviour. Nothing about the file's internals was examined beyond its container family. No screenshot is stored in the repository. Paths are written with `%APPDATA%` and `%LOCALAPPDATA%`.

Everything was done on throw-away copies of `samples/blank.pub`. The user's AutoRecover interval was temporarily changed to 1 minute and restored to its original value of 10 (registry evidence in section 7).

**Headline finding, different from Word/Excel:** Publisher 2021 has **no "Document Recovery" task pane and no keep/discard prompt**. After a crash, the next time Publisher starts it silently re-opens the crashed publication, straight from the AutoRecover file, with the text **"(Recovered)"** added to the title bar. See section 4.

## 1. Settings UI: File > Options > Save

File tab (Backstage) > **Options** (last item in the left list, below Account and Feedback) opens the **Publisher Options** dialog (title "Publisher Options"; left list: General, Proofing, **Save**, Language, Advanced, Customize Ribbon, Quick Access Toolbar, Add-ins, Trust Center; buttons **OK** and **Cancel** bottom right, "?" and close at top right). The **Save** page has the heading line "Customize how documents are saved." (floppy-disk icon) and one group, **Save documents**:

| Control | Exact label | Default / observed value |
|---|---|---|
| Check box + number box | "Save AutoRecover information every [ N ] minutes" (the number box is a spin box with up/down arrows, followed by the word "minutes") | checked, **10** |
| Check box | "Allow background saves" | checked |
| Check box | "Don't show the Backstage when opening or saving files with keyboard shortcuts" | unchecked |
| Check box | "Show additional places for saving, even if sign-in may be required." | checked |
| Check box | "Save to Computer by default" | unchecked |
| Text box | "Default personal templates location:" | empty |

Things the Save page does **not** have (so KernPage should not invent them): no "AutoRecover file location" box, no "Keep the last autosaved version if I close without saving" check box, no "Default file location" box, no "Disable AutoRecover for this document" option, no "Save AutoRecover information" explanation text.

The Advanced page (Editing options, Display, General...) has nothing about saving or recovery. Only the Save page matters.

### Limits and validation of the minutes box

- Valid range is **1 to 120**. Keystrokes that would make the number invalid are **silently rejected**: typing `121` leaves `12` (the final 1 is ignored), typing `999` leaves `99`, typing `0` is accepted into the box while editing but is not kept once OK is pressed.
- **No error message or dialog appears** for 0 or 121 (none was seen after pressing OK or Tab).
- Typing `0` and pressing OK closed the dialog and the stored value stayed **10** (the old value was kept; 0 is not saved).
- The spin arrows/Up/Down keys clamp at the ends: Up at 120 stays 120, Down at 1 stays 1. Up from 99 reaches 100, 101.
- Valid typed values take effect when OK is pressed, immediately (registry written at OK, no restart).

### Registry

Key: `HKCU\Software\Microsoft\Office\16.0\Publisher\Preferences`

- `AutoRecoverSave` (DWORD) = **1** (the check box)
- `AutoRecoverSaveInterval` (DWORD) = **10** (minutes)
- `BackgroundSave` (DWORD) = **1** ("Allow background saves")

The File MRU list lives under `HKCU\Software\Microsoft\Office\16.0\Publisher\File MRU` (see section 8). The AutoRecover **folder is not a registry setting** here and is not changeable in the UI.

## 2. AutoRecover files: where, when, what

- **Folder:** `%APPDATA%\Microsoft\Publisher\` (the same folder holds `PubAPSet.xml`, a 280-byte settings file that is not part of recovery). Not `%LOCALAPPDATA%\Microsoft\Office\UnsavedFiles`, not `%TEMP%`.
- **Name pattern:** `AutoRecover save - <document name without extension>(<4 hex chars>).pb26`. Examples seen: `AutoRecover save - t1(42D4).pb26`, `AutoRecover save - t2( 7BB).pb26` (the hex field is 4 characters wide, **space-padded on the left** when short), `AutoRecover save - t2(3D7C).pb26`. The 4-character code **differs every session/document-open**; it is not the process id.
- **Extension:** `.pb26` (not `.pub`). It is a Microsoft compound-document container (starts with the standard `D0 CF 11 E0 A1 B1 1A E1` signature), the same container family as a `.pub`, so it is not human-readable text.
- **Untitled documents** not observed (the test used named files). Naming and behaviour for untitled (never-saved) publications is **not observed**.
- **Size:** about the size of the publication: a blank 86,016-byte `.pub` with one text box added produced 89,088 to 91,648 bytes.
- **One file per open, modified document.** The file is created the first time the timer fires **after the document has become dirty** (unsaved edit), then the same file is overwritten in place.
- **Timing with the interval set to 1 minute:** the first file appeared 35 to 55 seconds after an edit (the timer is a free-running one-minute tick, not "one minute after the edit"; in one case it fired 5 seconds after the edit). Later writes: edit at 19:49:42 produced a rewrite at 19:50:39.
- **Only rewritten when there are new changes.** With the document still unsaved but unchanged, the file was **not** rewritten across several minutes (its last-write time stayed fixed through 3 intervals), and it did **not** appear for a recovered-but-unedited document.
- **Removed when the document is saved normally:** `Save` on the throw-away copy removed the AutoRecover file immediately (within the save).
- **Removed when closed without saving:** File > Close > **Don't Save** removed the file immediately.
- **Not removed when the process dies:** a killed process leaves the file in place (this is the basis of recovery, section 3).
- **Consumed on recovery:** when Publisher restarts and re-opens the recovered publication, the `.pb26` file is **deleted at once** (it was gone by the time the window was up, within 1 second of the process starting).
- The original `.pub` on disk is **not modified** by AutoRecover or by recovery (its last-write time and size were unchanged throughout).

## 3. Triggering recovery (what the test did)

1. Opened a copy of the blank, made an unsaved edit (text box with text), waited until `AutoRecover save - t2(xxxx).pb26` appeared.
2. Killed only the test's own `MSPUB.EXE` process.
3. Confirmed the `.pb26` file was still in `%APPDATA%\Microsoft\Publisher\` and the original `.pub` was untouched.
4. Started `MSPUB.EXE` again with no arguments.

## 4. The recovery experience, step by step

Repeated twice with identical results.

1. About 0.7 s after launch: the main window is up and titled just **"Publisher"**, with an empty, greyed ribbon and a "Page: 0 of 0" status bar (no start screen, no Backstage "Good evening" home page, no splash dialog).
2. About 1.7 s after launch: the title bar changes to **"t2.pub (Recovered) - Publisher"** and the publication is on screen with the unsaved edit present (page navigation pane showing page 1, the text box with "AutoRecover test edit" on the page). The window stays like that.
3. **There is no Document Recovery pane** (no pane at all beside the Pages pane), **no dialog, no message bar, no balloon**, no list of "Original" / "AutoRecovered" versions, no per-entry drop-down (no Open / Save As / Delete / Show Repairs / Close), no buttons and therefore no "keep or discard recovered files" prompt on closing a pane. All of the user's requested items for the pane are **not observed because they do not exist** in Publisher 2021. (The bead FILE-RECOVERY-PROMPT should therefore be a window-title-and-silent-reopen behaviour, not a pane.)
4. The recovered publication keeps its **original file name and folder**: the COM `Name` is `t2.pub`, `FullName` the original path, `Saved` is **False** (so it counts as having unsaved changes).
5. The only marker of recovery is the **"(Recovered)"** suffix in the title bar (window caption "t2.pub (Recovered) - Publisher"). File > Info shows the normal Info page (file name "t2", its folder path, Business Information, Design Checker, Embedded Font Information, Publication Properties) with **no recovery notice**. The Info page's "First Saved" and "Last Saved" both showed the time of recovery ("Today, 7:57 PM"), not the earlier original times, and "Size" showed 87.0KB.
6. **Undo state:** the Undo/Redo buttons in the Quick Access Toolbar were greyed (undo history is not recovered).
7. **File > Save on the recovered publication does not save quietly**: it opens the **Save As** dialog (standard Windows dialog, title "Save As", folder = the original folder, **File name: t2.pub** (name selected), **Save as type: Publisher Files (*.pub)**, plus Authors / Tags, Tools, Save, Cancel). The dialog was cancelled in the test (nothing saved); what happens after pressing Save (overwrite prompt, title losing "(Recovered)") is **not observed**.
8. Closing the recovered publication (File > Close) shows the ordinary save prompt (section 5). Choosing **Don't Save** closes it and leaves **no** AutoRecover file; the original `.pub` is unchanged.
9. If the recovered publication is left open and unedited, **no new AutoRecover file** is written. After a new edit the next tick writes `AutoRecover save - t2(xxxx).pb26` again (5 s after the edit in the test), under the same naming pattern.

Not observed: recovery when **several** publications were open at the crash; recovery of an **untitled** publication; recovery when the original file has since been deleted or changed; a crash with the document opened from a network or OneDrive location; whether the original file's name/path are shown if it can no longer be written.

## 5. The close prompt (normal close with unsaved changes)

Trigger: File > Close (Backstage) on a modified publication (also the same for a recovered one).

- A small modal dialog titled **"Microsoft Publisher"**, warning icon (yellow triangle with "!") on the left.
- Message: a question asking whether to save the changes made to the publication (wrapped over two lines; the publication's name is not quoted).
- Buttons, left to right: **Save**, **Don't Save**, **Cancel** (underlined access keys: S in Save, "n" in Don't Save). **Save** is the default (highlighted).
- Save: saves into the same file. Don't Save: closes and discards changes. Cancel: returns to the publication.
- After the last publication closes, Publisher shows the Backstage Home view (greeting "Good evening", a "New" gallery starting with Blank 8.5 x 11" and Blank 11 x 8.5", and a Recent / Pinned list). A new blank publication made from there is named "Publication2" in this session.

## 6. Behaviour summary for the KernPage implementation

1. **Settings:** On the Save page of Options provide exactly: check box "Save AutoRecover information every [N] minutes" (default checked, 10; range 1 to 120; invalid keystrokes silently refused; 0 and empty not accepted; no error text), plus the other Save controls as listed in section 1 if and when relevant. No AutoRecover-location control, no "keep last autosaved version" option.
2. **Folder and name:** Publisher writes recovery files to `%APPDATA%\Microsoft\Publisher\`; KernPage should use its own per-user folder (e.g. `%APPDATA%\KernPage\AutoRecover\`) but keep the name shape `AutoRecover save - <name>(<4 hex>).<ext>` with a KernPage extension (not `.pub`), one file per open modified publication, overwritten in place.
3. **Timer:** free-running tick every N minutes; on each tick write a recovery file only for publications that changed since their last recovery write (or since open). No write for clean or unchanged documents.
4. **Cleanup:** delete the publication's recovery file on a normal Save and on close with Save or Don't Save; leave it on a crash; delete it when it has been used for recovery.
5. **After a crash, at next start:** no pane and no prompt. Immediately (within about 1 to 2 seconds, before any start screen) re-open every publication that has a recovery file as an **unsaved** document that keeps its **original file name and folder**; title bar reads `<file name> (Recovered) - <app name>`; mark the document dirty; do not change the original file; undo history empty.
6. **Save on a recovered document** opens Save As with the original name and folder pre-filled and the type "Publisher Files (*.pub)" (the equivalent KernPage type); it does not overwrite silently. (After-save result not observed.)
7. **Close prompt:** asks whether to save the changes made to the publication, with buttons Save / Don't Save / Cancel, dialog title "Microsoft Publisher" (KernPage should use its own product name), warning icon.
8. No mention of recovery in File > Info.

## 7. Registry evidence of the temporary change

| Moment | `AutoRecoverSave` | `AutoRecoverSaveInterval` |
|---|---|---|
| Before any change | 1 | **10** |
| After typing 0 + OK (rejected) | 1 | 10 |
| After setting 1 (test value) | 1 | 1 |
| After restoring through Options > Save (typed 10, OK) | 1 | **10** |

## 8. Housekeeping notes

- The test publications were opened with Publisher from a scratch folder. The **File MRU** registry key (`HKCU\Software\Microsoft\Office\16.0\Publisher\File MRU`) was compared before and after: **unchanged** (no test publication was added to it). The per-user MRU service cache file under `%LOCALAPPDATA%\Microsoft\Office\16.0\MruServiceCache\...\Publisher\` was touched during the session (its two files were 2 bytes each, so effectively empty).
- At the end no `AutoRecover save - ...pb26` file remained in `%APPDATA%\Microsoft\Publisher\` (only the original `PubAPSet.xml`).
