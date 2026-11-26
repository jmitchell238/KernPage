# KernPage: Tables (Insert > Table, Table Design and Layout tabs), as Publisher 2021 shows them

Source: Publisher 2021 (version 16.0, Windows 11, 1920 x 1080 screen) driven through the public COM object model (`Shapes.AddTable`, `Table.Rows/Columns.Add`, `.Delete`, `Shape.Delete`, `Cell`) and read through the UI Automation tree (ribbon tabs, groups, buttons, their enabled state, galleries and dialogs) on 2026-11-15. This file describes behaviour and **labels in words only**; no screenshot, icon or artwork is stored in the repository. Command names, labels and values are quoted exactly because users expect them. The file-format side is in `docs/spec/PUB-FORMAT-SPEC.md` section 4E.1; ground-truth samples are `samples/gen/table-*.pub`.

Method: visible UI, UI Automation names and states, the public COM object model and our own samples only. Only the public automation interface, the visible program and our own files were used. Everything was done on throw-away copies of `samples/blank.pub`; the ribbon was driven by UI Automation patterns (select tab, expand, invoke), not by key presses or mouse clicks, so a **caret inside a cell was never placed** (see section 8 for what that leaves unobserved). Nothing was saved to the Publisher installation.

## 1. Insert tab, Tables group

- Ribbon tab **Insert**. Groups left to right: **Pages**, **Tables**, **Illustrations**, **Building Blocks**, **Text**, **Links**, **Header & Footer**.
- The **Tables** group has one large drop-down button, **Table** (icon above the label, small arrow below). The button has no click action of its own; it only opens the gallery.
- The gallery (a pop-up of about 240 x 280 px) has, top to bottom:
  1. a section header **Insert Table**;
  2. a grid of squares **10 columns wide and 8 rows high**. Each square's name (tooltip and automation name) is `<columns>x<rows> Table`, for example **3x2 Table** = 3 columns by 2 rows, **10x8 Table** is the bottom-right square. Hovering highlights the block from the top-left square to the hovered one (usual Office behaviour; hover was not simulated);
  3. one command row **Insert Table...** under a separator.
- Choosing a grid square inserts the table at once (no dialog, no undo prompt). Choosing **Insert Table...** opens the **Create Table** dialog (section 2).

### 1.1 What a table inserted from the grid looks like

Inserted with an empty page and the whole page shown (3x2 square):

- 3 columns of **72 pt (1 inch)** each and 2 rows of **27.105 pt** (about 0.376 in), so **216 x 54.21 pt (3.00 x 0.75 in)**; one more column adds 72 pt, one more row 27.105 pt (a 6-column, 4-row table is 432 x 108.42 pt).
- Placed **horizontally centred on the page** (3x2: left 196.875 pt; 6 columns: left 88.875 pt, on a 612 x 792 pt page; about 1 pt off exact centre) with its **top edge at 369.79 pt** in both cases (the top does not depend on the table's height, so a taller table is not re-centred vertically; the page was shown whole in the test).
- The new shape is selected, named **"Control 1"** (the shape counter shared with text boxes, so after a text box the first table is "Control 2"), `Type` = 18 (`pbTable`).
- Cells: **white fill**, **no borders** (gridlines are only the View Gridlines helper), **Calibri 10 pt**, black, left aligned, top aligned, horizontal text, cell margins **0.04 in = 2.88 pt on all four sides**, every cell an empty paragraph. `Table.GrowToFitText` is true (rows grow when text wraps) and `TableDirection` is left to right. The shape itself has no outline.
- `Shapes.AddTable(Rows, Columns, Left, Top, Width, Height)` creates the same kind of table at the given place; width and height are divided evenly (300 x 100 pt, 2 x 2 gives 150 pt columns, 50 pt rows). Publisher clamps very small cells (minimum column width 16.94 pt; minimum row height 17.97 pt for few rows, 26.75 pt when 5 or 50 rows were requested in a small box). `AddTable` with 0 rows raises an error; 50 x 50 works.

## 2. Create Table dialog (Insert Table...)

- Title **Create Table**, title-bar buttons for context help and close. Two spin boxes and two buttons:
  - **Number of rows:** (observed value on first opening: **2**)
  - **Number of columns:** (observed value on first opening: **6**) (Publisher may remember the last values; not tested)
  - **OK** (default) and **Cancel**.
- OK inserts the table exactly like a grid square with that many rows and columns (4 rows x 6 columns: 432 x 108.42 pt, centred horizontally). Cancel leaves the page unchanged (zero shapes).
- The spin boxes accept typed values (set to 4 and 6 in the test).

## 3. Contextual tabs when a table is selected

Selecting a table shape (the whole table) adds a contextual tab group labelled **Table Tools** at the right end of the ribbon tab row, with two tabs: **Table Design** and **Layout**. The Home tab stays the active tab (the contextual tabs are not selected automatically). Tab row with a table selected, left to right: **File**, **Home**, **Insert**, **Page Design**, **Review**, **View**, **Help**, then **Table Tools**: **Table Design**, **Layout**. (No "Table Layout" tab exists; the second tab is simply named **Layout**.)

### 3.1 Table Design tab

Groups, left to right:

1. **Table Formats**: the **Table Formats** gallery button (a drop-down gallery of table styles named **Table Style 1** ... **Table Style 65**, five per row, 13 rows, each a 90 x 70 px thumbnail), the **Fill** split button (main button and a drop-down arrow, "More Options"), and the **Format Table...** dialog launcher (small arrow at the group's bottom-right).
2. **Borders**: a border-line preview (a small image, with an **Open** drop-down arrow beside it for the line style/width chooser) and the **Borders** split button (its main button reads **Bottom Border** until another border is chosen; drop-down arrow beside it).
3. **WordArt Styles**: **Quick Styles** gallery, **Text Fill** (split), **Text Outline** (split), **Text Effects** drop-down, and the **Fill Effects...** dialog launcher.
4. **Typography**: **Drop Cap**, **Number Style**, **Ligatures**, **Stylistic Sets**, **Swash**, **Stylistic Alternates**, and the **Font...** dialog launcher.

**Format Table...** opens a dialog titled **Format Table** with four tabs: **Colors and Lines**, **Size**, **Layout**, **Cell Properties** (on **Colors and Lines**: Fill group with **Color**, **Fill Effects...**, **Transparency**; Line group with **Color**, **Transparency**, **Width**, **BorderArt...**; a **Preview** with push buttons for each border edge and a **Presets** area; a short instruction to pick the borders in the preview and then their colour and line style; **Shape Effects...**, **OK**, **Cancel**). Other tabs not opened.

### 3.2 Layout tab

Groups, left to right:

1. **Table**: **Select** (drop-down with **Select Cell**, **Select Column**, **Select Row**, **Select Table**) and **View Gridlines** (toggle button).
2. **Rows & Columns**: **Delete** (drop-down with **Delete Columns**, **Delete Rows**, **Delete Table**), **Insert Above** (large button), **Insert Below**, **Insert Left**, **Insert Right** (three small buttons stacked beside it).
3. **Merge**: **Merge Cells**, **Split Cells...** (opens a dialog), **Diagonals** (drop-down).
4. **Alignment**: one drop-down group button (cell text alignment).
5. **Arrange**: one drop-down group button (the usual position/order/group commands).
6. **Size**: one group button (table size fields).

Enabled state with the **whole table selected** (as when it was just inserted or clicked on its frame): **Delete** enabled, of its menu only **Delete Table** enabled (**Delete Columns** and **Delete Rows** are disabled); **Insert Above / Below / Left / Right** and **Merge Cells** disabled (they need a cell or cells selected, i.e. a caret or selection inside the table); **Select** menu items listed above exist.

## 4. Insert Above / Below / Left / Right (add rows and columns)

Observed through the object model (the buttons themselves were disabled in the test, see section 3.2): `Table.Rows.Add(BeforeRow)` is **Insert Above** on that row; `Table.Columns.Add(BeforeColumn)` is **Insert Left** of that column. Passing -1 (or no argument) appends at the end, which is what **Insert Below** on the last row and **Insert Right** on the last column produce. Indexes are 1-based; 0 and Count + 1 are rejected with "Invalid index".

- The table **gets bigger by the new row's height (or column's width)**: a 3x3 table of 360 x 120 pt (40 pt rows, 120 pt columns) becomes 360 x 160 pt or 480 x 120 pt. The table's left and top do not change, no other row/column moves or resizes.
- The new row/column takes the size of the row/column it is inserted before (a 70 pt row 1 plus Insert Above gives a new 70 pt row; a 50 pt column 1 plus Insert Left gives a 50 pt column). Appended rows/columns had the common size.
- **Existing cell text stays in its cells** (the text shifts down/right with its cell); the new cells are **empty**, Calibri 10 like a new table, default margins.
- Every row insert in a 3-column table adds 3 cells; every column insert in a 3-row table adds 3 cells.

## 5. Delete Rows / Columns / Table

- **Delete Rows** with a cell in row *n* removes row *n* with its text (3x3 -> 2x3, the cell text A2/B2/C2 is gone, A3/B3/C3 move up); the table **gets smaller by that row's height** (120 -> 80 pt). **Delete Columns** works the same on a column (360 -> 240 pt wide, B-column text gone). Observed through `Rows(n).Delete()` / `Columns(n).Delete()`. A table's **last remaining row cannot be deleted** (the call is refused with an access-denied error; the UI presumably disables it or offers Delete Table).
- **Delete Table** (**Layout** > **Delete** > **Delete Table**, worked through the UI with the whole table selected) removes the table shape completely; the page has no shapes left. `Shape.Delete()` does the same. In the saved file the page is the blank page again (spec 4E.1.5).
- With the whole table selected **Delete Rows** and **Delete Columns** are disabled; invoking them did nothing (table unchanged).

## 6. Resizing and text growth (object model)

- `Row.Height` and `Column.Width` set a row/column; the table frame resizes to the new total (column 1 from 150 to 50 pt made a 300 pt table 200 pt wide; row 1 from 50 to 70 pt made a 100 pt table 120 pt high).
- With `GrowToFitText` on (always, in these tests) a row grows when its text wraps: a 129-character sentence in a 150 pt-wide column made row 1 grow from 50 to 61.55 pt and the table from 100 to 111.55 pt; the other row stayed 50 pt.
- `Shape.Left` / `Shape.Top` move the whole table.

## 7. Behaviour summary for the KernPage implementation

- Insert > Table: gallery of 10 x 8 squares named `CxR Table`, plus **Insert Table...** (Create Table dialog, **Number of rows:** / **Number of columns:**, 2 x 6 initially). Default result: columns 72 pt, rows 27.105 pt (0.376 in), centred horizontally near the page middle, white cells, no borders, Calibri 10, margins 0.04 in, rows grow with text.
- Selecting a table shows **Table Tools** with **Table Design** (Table Formats, Borders, WordArt Styles, Typography) and **Layout** (Table, Rows & Columns, Merge, Alignment, Arrange, Size).
- Insert Above/Below/Left/Right need a cell selection; they add a row/column of the neighbouring size, grow the table, keep all text and add empty cells. Delete Rows/Columns need a cell selection and shrink the table; Delete Table removes the whole object. The last row/column cannot be deleted by row/column delete.
- A table is one object on the page (with its own selection frame) whose text is one story; cells are not individual shapes for the user.

## 8. Not observed

- The Insert Above / Below / Left / Right and Delete Rows / Columns buttons applied through the ribbon (no caret could be placed without sending mouse or keyboard input to the screen, which was avoided on a machine in use); only the object model results above. Whether a multi-cell selection inserts several rows, and where the caret lands afterwards, are unknown.
- Key tips (Alt sequences) of the Table Tools tabs and groups.
- The **Table Formats** gallery contents (what the 65 styles look like), **Fill** and **Borders** drop-downs, **Merge Cells**, **Split Cells...**, **Diagonals**, **Alignment** and **Size** groups, the **Size**, **Layout** and **Cell Properties** tabs of Format Table.
- Dragging column/row borders with the mouse, Tab key in the last cell (a new row in Word), pasting a table, Table menu on right-click, undo behaviour.
- Tables on master pages, tables on page 2, tables spanning a page edge, text wrap around a table.
