# Publisher 2021 user interface inventory

Source: Publisher 2021 (16.0.14334.20974, 64-bit, Click-to-Run), observed on Windows 10/11 at 1920x1080,
window maximised, Colorful theme. Method: UI Automation (names, control types, KeyTips, shortcuts,
bounding boxes) plus screenshots. Raw dumps and screenshots are in the local evidence files (not in the repository).
Labels are Publisher's own factual names. Tab and group names, order, KeyTips and shortcuts below are
what the ribbon showed. "Size" is large (about 90 px tall, icon over label) or small (about 26 px row).
Type "drop-down" = a button that only opens a menu/gallery (UIA MenuItem); "split button" = main
action plus a separate arrow.

## 1. Window chrome and theme

| Item | Publisher 2021 |
|---|---|
| Theme | Colorful (the default): white title bar and ribbon, dark-green accent. File tab is a solid dark-green (Publisher brand green) block with a white label; the status bar is solid dark green with white text; the backstage left rail is solid green with white labels. Canvas/work area is light grey; page has a thin drop shadow and dotted margin guide. Page Navigation thumbnails have a light grey-blue highlight for the selected page (page number below in bold). |
| Title bar | Single row (~43 px). Left: Quick Access Toolbar. Centre: "name.pub - Publisher" (document name, two spaces around the dash). Right: signed-in user widget (name and avatar; only when signed in), Help "?" button, Minimize, Restore Down/Maximize, Close. No app icon at the far left (the QAT replaces it). |
| Quick Access Toolbar (default, left of title, above the ribbon) | Save (Alt,1; Ctrl+S) | Undo (split, Alt,2) | Redo (split, Alt,3) | Touch/Mouse Mode (Alt,4) | Customize Quick Access Toolbar (arrow). Undo/Redo read "Can't Undo"/"Can't Redo" when empty. |
| Tab row | File button (green, KeyTip Alt,F), then Home, Insert, Page Design, Mailings, Review, View, Help. Contextual tabs appear to the right of Help in the accent colour, preceded by a coloured context label ("Drawing Tools", "Text Box Tools", "Picture Tools", "Table Tools"). The "Collapse the Ribbon" arrow sits at the bottom-right corner of the ribbon. |
| Tab labels | Plain text, no boxes; the selected tab is shown in bold/green text with a thin underline; the ribbon body is white. Group names are centred at the bottom of each group with vertical separator lines between groups; dialog launchers are small square-arrow glyphs at the group's bottom-right corner. |
| Ribbon height | About 120 px including the tab row (tab row ~38 px + body ~118 px at 100% scaling). |
| Left pane | "Pages" (Page Navigation) pane, 175 px wide, heading "Pages" in large green text, a small collapse/expand triangle under it, thumbnails with the page number beneath each (selected page: bold number, highlighted frame). Pane can be hidden via View > Show > Page Navigation. A section header "Section" collapses a section's pages. |
| Rulers | Horizontal ruler across the top of the canvas and a vertical ruler on the left, on by default, both ticked in inches. |
| Scrollbars | Standard vertical (right) and horizontal (bottom) scroll bars; at the bottom-right of the vertical bar are Previous Page and Next Page buttons. |
| Status bar (left to right) | Page indicator button "Page: 1 of 2" (opens Go To Page) | object position (x, y in inches, only when an object/cursor is present) | object size (w x h) | spacer | right side: Single Page and Two-Page Spread view buttons (tabs, Single Page selected by default) | Zoom Out (-) | zoom slider with thumb | Zoom In (+) | "Zoom NN%" percentage button (opens Zoom dialog) | Whole Page (fit) button at far right. |
| Default zoom | Document opens fitted to the window ("whole page"): about 31% for an 8.5x11 page on a 1080p maximised window; a freshly opened document is not at 100%. |
| Default view | Single Page, Normal view (not Master Page), rulers on, boundaries on, guides on. |

## 2. File (Backstage) view

The File button opens a full-window backstage (the ribbon is replaced; the title bar stays). The left rail
is solid green with white labels and a circled back-arrow button at the top. Left rail items, in order:

| # | Item | KeyTip | Note |
|---|---|---|---|
| 1 | Back (circled arrow) | - | returns to the document |
| 2 | Home | Alt, F, H | |
| 3 | New | Alt, F, N | |
| 4 | Open | Alt, F, O | |
| - | (separator line) | | |
| 5 | Info | Alt, F, I | |
| 6 | Save | Alt, F, S | acts immediately |
| 7 | Save As | Alt, F, A | page |
| 8 | Print | Alt, F, P | page |
| 9 | Share | Alt, F, Z | cloud sharing; not opened in this study |
| 10 | Export | Alt, F, E | page |
| 11 | Close | Alt, F, C | acts immediately (closes the publication) |
| - | (large gap, then separator line) | | |
| 12 | Account | Alt, F, D | signed-in user and Office account; not opened (personal data) |
| 13 | Feedback | Alt, F, K | not opened (sends data online) |
| 14 | Options | Alt, F, T | Publisher Options dialog; not opened |

There is no Exit, no Save & Send and no Recent/Help left-rail entry (those are Publisher 2010).

### Home page
Greeting heading that depends on the time of day ("Good morning/afternoon/evening"). Section **New**: a
collapsible header (chevron) and one row of template tiles: Blank 8.5 x 11", Blank 11 x 8.5", More Blank
Page Sizes, followed by four online-suggested featured templates (they vary), and a "More templates" link.
Section **Open**: two tabs, Recent and Pinned; the Recent list has columns Name (file name plus a breadcrumb
path under it) and Date modified, each row with a pin button; "More publications" link at the bottom.

### New page
Heading "New". Row of the three blank tiles. Two tabs, **Office** and **Built-In**. A search box "Search for
online templates" with a search button; a "Suggested searches" row: Personal, Design Sets, Labels, Event,
Paper, Business, Cards. Below, a large tile grid (5 per row) of online templates, each with a pin button.
(Built-In holds the installed template categories.)

### Open page
Heading "Open". Left column of places: Recent, OneDrive (account name), This PC, Add a Place, and a
**Browse** button. Right column: heading "Recent Publications" with tabs Publications / Folders, a "Pinned"
list (hint text: pin files to find them later), then groups by date (Today, ...).

### Info page
Heading "Info", then the publication name (large) and its full path as a button, with **Copy path** and
**Open file location** buttons. Left column of three sections, each a large icon button plus title and description:
**Business Information** (Edit Business Information; lists tagline, individual name, job title,
organisation name, address, phone/fax/e-mail), **Design Checker** (Run Design Checker),
**Embedded Font Information** (Manage Embedded Fonts). Right column: **Publication Properties** drop-down with
rows Template, Color Scheme, Color Mode, Publication Mode, Pages, Size, First Saved, Last Saved. (2010 also
had a thumbnail preview and a Commercial Print Information section; 2021 has neither.)

### Save As page
Heading "Save As". Places column: Recent, OneDrive, This PC, Add a Place, **Browse**. Right: a
Pinned folders list then Today/earlier recent folders ("Relevant Folders": Documents, Desktop, ...).

### Print page
Heading "Print". Two panes. Left (437 px column): a large **Print** button with "Copies of print job"
spinner; **Printer** (printer combo "Which Printer", link "Printer Properties"); **Settings** (combos: Print
What, Layout Gallery, Paper Size, Two-Sided Printing, Color Output; "Pages" text box; check box "Save
settings with publication"). Right: **Print Preview** with a page-number show/hide slider (Hide/Show Page
Numbers), a Ruler button, sheet navigator (Previous Sheet, number box, "of N", Next Sheet), a preview zoom
slider with percent, and Fit to Sheet, plus a View Multiple Sheets button.

### Export page
Heading "Export". Left list of export types, in two groups: (no heading) **Create PDF/XPS Document**,
**Publish HTML**, **Change File Type**; **Pack and Go**: **Save for Photo Printing**, **Save for a
Commercial Printer**, **Save for Another Computer**. Right pane shows description bullets (for PDF/XPS, three points: fonts, formatting and images are kept; the content is not easy to change; free viewers exist on the Web)
and a large **Create PDF/XPS** button.

## 3. Ribbon tabs

Tab order: File, Home, Insert, Page Design, Mailings, Review, View, Help. Contextual tabs follow in section 4.
KeyTips are shown as the Alt-key sequence the ribbon reports.

### Home




**Clipboard** (KeyTip Alt, H, Z L)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Paste | split button | large | Alt, H, V |  |
| 2 | Cut | button | small | Alt, H, X | Ctrl+X |
| 3 | Copy | button | small | Alt, H, C | Ctrl+C |
| 4 | Format Painter | button | small | Alt, H, F P | Ctrl+Shift+C |
| 5 | Office Clipboard... | dialog launcher | small | Alt, H, F O |  |

**Font** (KeyTip Alt, H, Z F)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Font | combo box | small | Alt, H, F F |  |
| 2 | Bold | button | small | Alt, H, 1 | Ctrl+B |
| 3 | Italic | button | small | Alt, H, 2 | Ctrl+I |
| 4 | Underline | button | small | Alt, H, 3 | Ctrl+U |
| 5 | Subscript | button | small | Alt, H, 5 | Ctrl+= |
| 6 | Superscript | button | small | Alt, H, 6 | Ctrl+Shift++ |
| 7 | Change Case | drop-down | small | Alt, H, 7 |  |
| 8 | Font Size | combo box | small | Alt, H, F S |  |
| 9 | Character Spacing | drop-down | small | Alt, H, M |  |
| 10 | Increase Font Size | button | small | Alt, H, F G | Ctrl+> |
| 11 | Decrease Font Size | button | small | Alt, H, F K | Ctrl+< |
| 12 | Font Color | split button | small | Alt, H, F C |  |
| 13 | Clear Formatting | button | small | Alt, H, E |  |
| 14 | Font... | dialog launcher | small | Alt, H, F N |  |

**Paragraph** (KeyTip Alt, H, Z P)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Bullets | drop-down | small | Alt, H, B U |  |
| 2 | Align Left | button | small | Alt, H, A L | Ctrl+L |
| 3 | Center | button | small | Alt, H, A C | Ctrl+E |
| 4 | Numbering | drop-down | small | Alt, H, N N |  |
| 5 | Align Right | button | small | Alt, H, A R | Ctrl+R |
| 6 | Justify | button | small | Alt, H, A J | Ctrl+J |
| 7 | Decrease Indent | button | small | Alt, H, D |  |
| 8 | Line Spacing | drop-down | small | Alt, H, K |  |
| 9 | Increase Indent | button | small | Alt, H, I |  |
| 10 | Columns | drop-down | small | Alt, H, J |  |
| 11 | Paragraph Spacing | drop-down | small | Alt, H, S P |  |
| 12 | Special Characters | button | small | Alt, H, 8 | Ctrl+Shift+Y |
| 13 | Paragraph... | dialog launcher | small | Alt, H, F A |  |

**Styles** (KeyTip Alt, H, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Styles | drop-down | large | Alt, H, S T |  |

**Objects** (KeyTip Alt, H, Z O)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Draw Text Box | button | large | Alt, H, T B |  |
| 2 | Pictures... | button | large | Alt, H, P |  |
| 3 | Table | drop-down | large | Alt, H, T A |  |
| 4 | Shapes | drop-down | large | Alt, H, S H |  |

**Arrange** (KeyTip Alt, H, Z A)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Wrap Text | drop-down | large | Alt, H, T W |  |
| 2 | Bring Forward | split button | small | Alt, H, A F |  |
| 3 | Send Backward | split button | small | Alt, H, A E |  |
| 4 | Align | drop-down | small | Alt, H, A A |  |
| 5 | Group | button | small | Alt, H, G | Ctrl+Shift+G |
| 6 | Ungroup | button | small | Alt, H, U | Ctrl+Shift+G |
| 7 | Rotate | drop-down | small | Alt, H, A Y |  |

**Editing** (KeyTip Alt, H, Z N)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Find... | split button | small | Alt, H, F D |  |
| 2 | Replace... | button | small | Alt, H, R P | Ctrl+H |
| 3 | Select | drop-down | small | Alt, H, S L |  |


### Insert




**Pages** (KeyTip Alt, N, Z P)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Page | split button | large | Alt, N, N P |  |
| 2 | Catalog Pages | button | large | Alt, N, C P |  |

**Tables** (KeyTip Alt, N, Z B)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Table | drop-down | large | Alt, N, T A |  |

**Illustrations** (KeyTip Alt, N, Z I)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Pictures... | button | large | Alt, N, P |  |
| 2 | Online Pictures... | button | large | Alt, N, F 1 |  |
| 3 | Shapes | drop-down | large | Alt, N, S H |  |
| 4 | Picture Placeholder | button | large | Alt, N, I |  |

**Building Blocks** (KeyTip Alt, N, Z 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Page Parts | drop-down | large | Alt, N, G |  |
| 2 | Calendars | drop-down | large | Alt, N, C A |  |
| 3 | Borders & Accents | drop-down | large | Alt, N, A |  |
| 4 | Advertisements | drop-down | large | Alt, N, R |  |
| 5 | Building Block Library | dialog launcher | small | Alt, N, L |  |

**Text** (KeyTip Alt, N, Z T)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Draw Text Box | button | large | Alt, N, T B |  |
| 2 | Business Information | drop-down | large | Alt, N, B I |  |
| 3 | WordArt | drop-down | large | Alt, N, W |  |
| 4 | Insert File | button | large | Alt, N, F 2 |  |
| 5 | Symbol | drop-down | large | Alt, N, U |  |
| 6 | Date & Time... | button | large | Alt, N, D |  |
| 7 | Object... | button | large | Alt, N, J |  |

**Links** (KeyTip Alt, N, Z L)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Link | button | large | Alt, N, H | Ctrl+K |
| 2 | Bookmark... | button | large | Alt, N, K |  |

**Header & Footer** (KeyTip Alt, N, Z H)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Header | button | large | Alt, N, E |  |
| 2 | Footer | button | large | Alt, N, O |  |
| 3 | Page Number | drop-down | large | Alt, N, M |  |


### Page Design

The Schemes group holds an in-ribbon colour-scheme gallery (see below), plus the Fonts button; the Colors drop-down arrow lives on the gallery.


**Template** (KeyTip Alt, P, Z T)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Change Template | button | large | Alt, P, C |  |
| 2 | Options | button | large | Alt, P, O P |  |

**Page Setup** (KeyTip Alt, P, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Margins | drop-down | large | Alt, P, M R |  |
| 2 | Orientation | drop-down | large | Alt, P, O R |  |
| 3 | Size | drop-down | large | Alt, P, S Z |  |
| 4 | Page Setup... | dialog launcher | small | Alt, P, S P |  |

**Layout** (KeyTip Alt, P, Z 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Guides | drop-down | large | Alt, P, G |  |
| 2 | Align To | label | small | Alt, P, A |  |
| 3 | Guides | check box | small | Alt, P, U | Ctrl+Shift+W |
| 4 | Objects | check box | small | Alt, P, O B |  |

**Pages** (KeyTip Alt, P, Z P)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Delete | button | small | Alt, P, D |  |
| 2 | Move | button | small | Alt, P, V |  |
| 3 | Rename... | button | small | Alt, P, N |  |

**Schemes** (KeyTip Alt, P, Z C)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Colors (gallery "more" button) | button | small | Alt, P, T C |  |
| 3 | Fonts | drop-down | large | Alt, P, T F |  |

**Page Background** (KeyTip Alt, P, Z B)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Apply Image | split button | large | Alt, P, A |  |
| 2 | Background | drop-down | large | Alt, P, B |  |
| 3 | Master Pages | drop-down | large | Alt, P, R |  |


### Mailings




**Start** (KeyTip Alt, M, Z 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Mail Merge | split button | large | Alt, M, T |  |
| 2 | E-mail Merge | split button | large | Alt, M, S |  |
| 3 | Select Recipients | drop-down | large | Alt, M, R |  |
| 4 | Edit Recipient List... | button | large | Alt, M, D |  |

**Write & Insert Fields** (KeyTip Alt, M, Z W)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Insert Merge Field | drop-down | large | Alt, M, I |  |
| 2 | Address Block... | button | large | Alt, M, A |  |
| 3 | Greeting Line... | button | large | Alt, M, G |  |
| 4 | Picture | drop-down | large | Alt, M, C |  |
| 5 | Insert Personalized Hyperlink... | button | large | Alt, M, H |  |
| 6 | Format | drop-down | large | Alt, M, O |  |

**Preview Results** (KeyTip Alt, M, Z P)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Preview Results | button | large | Alt, M, P |  |
| 2 | First | button | small | Alt, M, Q |  |
| 3 | Find Recipient... | button | small | Alt, M, J |  |
| 4 | Exclude This Recipient | button | small | Alt, M, E |  |
| 5 | Previous | button | small | Alt, M, M |  |
| 6 | Record Number | edit | small | Alt, M, V |  |
| 7 | Next | button | small | Alt, M, X |  |
| 8 | Last | button | small | Alt, M, L |  |

**Finish** (KeyTip Alt, M, Z F)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Finish & Merge | drop-down | large | Alt, M, F |  |
| 2 | Follow-up | drop-down | large | Alt, M, U |  |


### Review




**Proofing** (KeyTip Alt, R, Z P)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Spelling... | button | large | Alt, R, S |  |
| 2 | Research... | button | large | Alt, R, R |  |
| 3 | Thesaurus... | button | large | Alt, R, E |  |

**Language** (KeyTip Alt, R, Z L)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Translate Selected Text | button | large | Alt, R, L |  |
| 2 | Language | drop-down | large | Alt, R, U |  |


### View




**Views** (KeyTip Alt, W, Z V)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Normal | button | large | Alt, W, N |  |
| 2 | Master Page | button | large | Alt, W, M |  |

**Layout** (KeyTip Alt, W, Z 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Single Page | button | large | Alt, W, S P |  |
| 2 | Two-Page Spread | button | large | Alt, W, 2 |  |

**Show** (KeyTip Alt, W, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Boundaries | check box | small | Alt, W, U | Ctrl+Shift+O |
| 2 | Guides | check box | small | Alt, W, D |  |
| 3 | Fields | check box | small | Alt, W, F |  |
| 4 | Rulers | check box | small | Alt, W, R |  |
| 5 | Page Navigation | check box | small | Alt, W, P |  |
| 6 | Scratch Area | check box | small | Alt, W, S C |  |
| 7 | Graphics Manager... | check box | small | Alt, W, G |  |
| 8 | Baselines | check box | small | Alt, W, B | Ctrl+F7 |

**Zoom** (KeyTip Alt, W, Z Z)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | 100% | button | large | Alt, W, J |  |
| 2 | Whole Page | button | large | Alt, W, 1 | Ctrl+Shift+L |
| 3 | Page Width | button | small | Alt, W, I |  |
| 4 | Selected Objects | button | small | Alt, W, T |  |
| 5 | Zoom | combo box | small | Alt, W, O |  |

**Window** (KeyTip Alt, W, Z W)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Arrange All | button | large | Alt, W, A |  |
| 2 | Cascade | button | large | Alt, W, E |  |
| 3 | Switch Windows | drop-down | large | Alt, W, W |  |


### Help




**Help** (KeyTip Alt, Y, Z)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Help | button | large | Alt, Y, H |  |
| 2 | Contact Support... | button | large | Alt, Y, C |  |
| 3 | Feedback | button | large | Alt, Y, K |  |


#### Page Design notes: Schemes gallery
The in-ribbon gallery shows 3 rows x 6 columns of colour schemes (4 swatches plus the name), with
Row up / Row Down scroll buttons and a "More" drop-down arrow at its right (it is the whole Schemes group; the Fonts
button sits to its right). Visible order, column by column: Apex, Flow, Office 2007-2010 | Aspect, Foundry,
Opulent | Black & Gray, Median, Oriel | Civic, Metro, Origin | Concourse, Module, Paper | Equity, Office, Solstice;
the current scheme is highlighted.
The full list in the drop-down, in addition to the visible ones, includes (about 90 schemes total): Alpine,
Apex, Aqua, Aspect, Berry, Black & Gray, Black & White, Bluebird, Brown, Burgundy, Cavern, Cherry, Citrus,
Civic, Clay, Concourse, Cranberry, Crocus, Dark Blue, Desert, Eggplant, Equity, Field, Fjord, Floral, Flow,
Foundry, Garnet, Glacier, Green, Grove, Harbor, Heather, Iris, Island, Ivy, Lagoon, Lilac, Mahogany, Marine,
Maroon, Meadow, Median, Metro, Mist, Mistletoe, Module, Monarch, Moss, Mountain, Mulberry, Navy, Nutmeg,
Ocean, Office, Office 2007-2010, Olive, Opulent, Orange, Orchid, Oriel, Origin, Paper, Parrot, Peach, Pebbles,
Plum, Prairie, Rain Forest, Red, Redwood, Reef, Sagebrush, Sapphire, Shamrock, Sienna, Solstice, Spice,
Summer, Sunrise, Sunset, Teal, Technic, Tidepool, Trek, Tropics, Trout, Tuscany, Urban, Verve, Vineyard,
Waterfall, Wildflower. Note the new "Office" (2013+ palette) versus "Office 2007-2010".

## 4. Contextual tabs

Contextual tabs appear to the right of Help, in the accent colour, under a context label. Verified by selecting
a text box, a picture, an autoshape and a table in sample files.

| Selection | Context label | Tabs |
|---|---|---|
| Text box | Drawing Tools, Text Box Tools | Shape Format (Alt, J, D), Text Box (Alt, J, X) |
| Autoshape | Drawing Tools | Shape Format (Alt, J, D) |
| Picture | Picture Tools | Picture Format (Alt, J, P) |
| Table | Table Tools | Table Design (Alt, J, T), Layout (Alt, J, L) |

Gallery contents (Shape Styles, WordArt Styles, Table Formats, Picture Styles, Insert Shapes) are presented as
in-ribbon galleries with Row up / Row Down / "More" controls; only the visible items are listed below as
"(gallery)". Observed gallery contents: Picture Styles start with Simple Frame White, Beveled Matte White,
Metal Frame, Drop Shadow Rectangle, Reflected Rounded Rectangle, Soft Edge Rectangle, Double Frame Black,
Thick Matte Black, Simple Frame Black, Beveled Oval Black, Moderate Frame Black, Center Shadow Rectangle; Table
Formats are "Table Style 1" onward; Shape Styles start with "Colored Outline - Dark", "- Accent 1" ... "- Accent 4".
Note: when a gallery row is shown the gallery drop-down appears as an unnamed control in the dump; it is
listed as "(gallery drop-down arrow)".

### Text Box (context: Text Box Tools)


**Text** (KeyTip Alt, J X, Z X)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Text Fit | drop-down | large | Alt, J X, I |  |
| 2 | Text Direction | button | large | Alt, J X, X |  |
| 3 | Hyphenation | button | large | Alt, J X, H | Ctrl+Shift+H |
| 4 | Format Text Box... | dialog launcher | small | Alt, J X, T X |  |

**Font** (KeyTip Alt, J X, Z F)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Font | combo box | small | Alt, J X, F F |  |
| 2 | Bold | button | small | Alt, J X, 1 | Ctrl+B |
| 3 | Italic | button | small | Alt, J X, 2 | Ctrl+I |
| 4 | Underline | button | small | Alt, J X, 3 | Ctrl+U |
| 5 | Subscript | button | small | Alt, J X, 5 | Ctrl+= |
| 6 | Superscript | button | small | Alt, J X, 6 | Ctrl+Shift++ |
| 7 | Change Case | drop-down | small | Alt, J X, 7 |  |
| 8 | Font Size | combo box | small | Alt, J X, F S |  |
| 9 | Character Spacing | drop-down | small | Alt, J X, M |  |
| 10 | Increase Font Size | button | small | Alt, J X, F G | Ctrl+> |
| 11 | Decrease Font Size | button | small | Alt, J X, F K | Ctrl+< |
| 12 | Font Color | split button | small | Alt, J X, F C |  |
| 13 | Clear Formatting | button | small | Alt, J X, E |  |
| 14 | Font... | dialog launcher | small | Alt, J X, F N |  |

**Alignment** (KeyTip Alt, J X, Z G)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Align Top Left | button | small | Alt, J X, T L |  |
| 2 | Align Center Left | button | small | Alt, J X, C L |  |
| 3 | Align Bottom Left | button | small | Alt, J X, B L |  |
| 4 | Align Top Center | button | small | Alt, J X, T C |  |
| 5 | Align Center | button | small | Alt, J X, C C |  |
| 6 | Align Bottom Center | button | small | Alt, J X, B C |  |
| 7 | Align Top Right | button | small | Alt, J X, T R |  |
| 8 | Align Center Right | button | small | Alt, J X, C R |  |
| 9 | Align Bottom Right | button | small | Alt, J X, B R |  |
| 10 | Columns | drop-down | large | Alt, J X, J |  |
| 11 | Margins | drop-down | large | Alt, J X, R |  |

**Linking** (KeyTip Alt, J X, Z L)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Create Link | button | large | Alt, J X, C K |  |
| 2 | Break | button | small | Alt, J X, K |  |
| 3 | Next | button | small | Alt, J X, N |  |
| 4 | Previous | button | small | Alt, J X, P |  |

**WordArt Styles** (KeyTip Alt, J X, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Quick Styles (gallery "more" button) | button | small | Alt, J X, Q |  |
| 3 | Text Fill | split button | small | Alt, J X, T I |  |
| 4 | Text Outline | split button | small | Alt, J X, T O |  |
| 5 | Text Effects | drop-down | small | Alt, J X, T E |  |
| 6 | Fill Effects... | dialog launcher | small | Alt, J X, W |  |

**Typography** (KeyTip Alt, J X, Z T)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Drop Cap | drop-down | large | Alt, J X, D C |  |
| 2 | Number Style | drop-down | large | Alt, J X, U |  |
| 3 | Ligatures | drop-down | large | Alt, J X, L |  |
| 4 | Stylistic Sets | drop-down | small | Alt, J X, S T |  |
| 5 | Swash | button | small | Alt, J X, S W |  |
| 6 | Stylistic Alternates | drop-down | small | Alt, J X, A |  |
| 7 | Font... | dialog launcher | small | Alt, J X, F N |  |


### Shape Format (context: Drawing Tools; also shown for text boxes)


**Insert Shapes** (KeyTip Alt, J D, Z H)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Shapes | button | small | Alt, J D, S H |  |
| 3 | Edit Shape | drop-down | small | Alt, J D, E |  |
| 4 | Change Shape | drop-down | small | Alt, J D, I |  |
| 5 | Edit Text | button | small | Alt, J D, X |  |

**Shape Styles** (KeyTip Alt, J D, Z Y)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Quick Styles (gallery "more" button) | button | small | Alt, J D, S T |  |
| 3 | Shape Fill | split button | small | Alt, J D, S F |  |
| 4 | Shape Outline | split button | small | Alt, J D, S O |  |
| 5 | Shape Effects | drop-down | small | Alt, J D, S E |  |
| 6 | Format Text Box... | dialog launcher | small | Alt, J D, O |  |

**Arrange** (KeyTip Alt, J D, Z A)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Wrap Text | drop-down | large | Alt, J D, T W |  |
| 2 | Bring Forward | split button | large | Alt, J D, A F |  |
| 3 | Send Backward | split button | large | Alt, J D, A E |  |
| 4 | Align | drop-down | large | Alt, J D, A A |  |
| 5 | Group | button | large | Alt, J D, G | Ctrl+Shift+G |
| 6 | Ungroup | button | large | Alt, J D, U | Ctrl+Shift+G |
| 7 | Rotate | drop-down | large | Alt, J D, A Y |  |

**Size** (KeyTip Alt, J D, Z Z)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Measurement | button | small | Alt, J D, M |  |
| 2 | Height | label | small | Alt, J D, H |  |
| 3 | Width | label | small | Alt, J D, W |  |
| 4 | Shape Height | edit | small | Alt, J D, H |  |
| 5 | Shape Width | edit | small | Alt, J D, W |  |
| 6 | Size... | dialog launcher | small | Alt, J D, S Z |  |


### Picture Format (context: Picture Tools)


**Insert** (KeyTip Alt, J P, Z N)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Pictures... | button | large | Alt, J P, P 1 |  |

**Adjust** (KeyTip Alt, J P, Z 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Corrections | drop-down | large | Alt, J P, R |  |
| 2 | Recolor | drop-down | large | Alt, J P, L |  |
| 3 | Compress Pictures... | button | small | Alt, J P, M |  |
| 4 | Change Picture | drop-down | small | Alt, J P, E |  |
| 5 | Reset Picture | button | small | Alt, J P, Q |  |
| 6 | Format Picture... | dialog launcher | small | Alt, J P, A D |  |

**Swap** (KeyTip Alt, J P, Z T)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Swap | split button | large | Alt, J P, S W |  |

**Picture Styles** (KeyTip Alt, J P, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Quick Styles (gallery "more" button) | button | small | Alt, J P, K |  |
| 3 | Picture Border | split button | small | Alt, J P, S O |  |
| 4 | Picture Effects | drop-down | small | Alt, J P, P E |  |
| 5 | Caption | drop-down | small | Alt, J P, C |  |
| 6 | Format Picture... | dialog launcher | small | Alt, J P, O |  |

**Arrange** (KeyTip Alt, J P, Z A)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Arrange Thumbnails | button | large | Alt, J P, A T |  |
| 2 | Wrap Text | drop-down | large | Alt, J P, T W |  |
| 3 | Bring Forward | split button | small | Alt, J P, A F |  |
| 4 | Send Backward | split button | small | Alt, J P, A E |  |
| 5 | Align | drop-down | small | Alt, J P, A A |  |
| 6 | Group | button | small | Alt, J P, G | Ctrl+Shift+G |
| 7 | Ungroup | button | small | Alt, J P, U | Ctrl+Shift+G |
| 8 | Rotate | drop-down | small | Alt, J P, A Y |  |

**Crop** (KeyTip Alt, J P, Z C)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Crop | split button | large | Alt, J P, X |  |
| 2 | Fit | button | small | Alt, J P, F T |  |
| 3 | Fill | button | small | Alt, J P, F L |  |
| 4 | Clear Crop | button | small | Alt, J P, F C |  |

**Size** (KeyTip Alt, J P, Z Z)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Height | label | small | Alt, J P, H |  |
| 2 | Width | label | small | Alt, J P, W |  |
| 3 | Shape Height | edit | small | Alt, J P, H |  |
| 4 | Shape Width | edit | small | Alt, J P, W |  |
| 5 | Size... | dialog launcher | small | Alt, J P, S Z |  |


### Table Design (context: Table Tools)


**Table Formats** (KeyTip Alt, J T, Z T 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Table Formats (gallery "more" button) | button | small | Alt, J T, T F |  |
| 3 | Fill | split button | large | Alt, J T, T C |  |
| 4 | Format Table... | dialog launcher | small | Alt, J T, O |  |

**Borders** (KeyTip Alt, J T, Z B)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Line Weight | combo box | small | Alt, J T, W 1 |  |
| 2 | Line Color | drop-down | small | Alt, J T, C |  |
| 3 | Borders | split button | small | Alt, J T, B |  |

**WordArt Styles** (KeyTip Alt, J T, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | (gallery drop-down arrow) | drop-down | large |  |  |
| 2 | Quick Styles (gallery "more" button) | button | small | Alt, J T, Q |  |
| 3 | Text Fill | split button | small | Alt, J T, T I |  |
| 4 | Text Outline | split button | small | Alt, J T, T O |  |
| 5 | Text Effects | drop-down | small | Alt, J T, T E |  |
| 6 | Fill Effects... | dialog launcher | small | Alt, J T, W 2 |  |

**Typography** (KeyTip Alt, J T, Z T 2)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Drop Cap | drop-down | small | Alt, J T, D C |  |
| 2 | Number Style | drop-down | small | Alt, J T, U |  |
| 3 | Ligatures | drop-down | small | Alt, J T, L |  |
| 4 | Stylistic Sets | drop-down | small | Alt, J T, S T |  |
| 5 | Swash | button | small | Alt, J T, S W |  |
| 6 | Stylistic Alternates | drop-down | small | Alt, J T, A |  |
| 7 | Font... | dialog launcher | small | Alt, J T, F N |  |


### Layout (context: Table Tools)


**Table** (KeyTip Alt, J L, Z B)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Select | drop-down | large | Alt, J L, Q |  |
| 2 | View Gridlines | button | large | Alt, J L, T G |  |

**Rows & Columns** (KeyTip Alt, J L, Z R)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Delete | drop-down | large | Alt, J L, D L |  |
| 2 | Insert Above | button | large | Alt, J L, I A |  |
| 3 | Insert Below | button | large | Alt, J L, I B |  |
| 4 | Insert Left | button | large | Alt, J L, I L |  |
| 5 | Insert Right | button | large | Alt, J L, I R |  |

**Merge** (KeyTip Alt, J L, Z M)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Merge Cells | button | large | Alt, J L, M |  |
| 2 | Split Cells... | button | large | Alt, J L, P |  |
| 3 | Diagonals | drop-down | large | Alt, J L, D M |  |

**Alignment** (KeyTip Alt, J L, Z A 1)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Align Top Left | button | small | Alt, J L, T L |  |
| 2 | Align Center Left | button | small | Alt, J L, C L |  |
| 3 | Align Bottom Left | button | small | Alt, J L, B L |  |
| 4 | Align Top Center | button | small | Alt, J L, T C |  |
| 5 | Align Center | button | small | Alt, J L, C C |  |
| 6 | Align Bottom Center | button | small | Alt, J L, B C |  |
| 7 | Align Top Right | button | small | Alt, J L, T R |  |
| 8 | Align Center Right | button | small | Alt, J L, C R |  |
| 9 | Align Bottom Right | button | small | Alt, J L, B R |  |
| 10 | Text Direction | button | large | Alt, J L, X |  |
| 11 | Hyphenation | button | large | Alt, J L, H | Ctrl+Shift+H |
| 12 | Cell Margins | drop-down | large | Alt, J L, N |  |
| 13 | Format Table... | dialog launcher | small | Alt, J L, O |  |

**Arrange** (KeyTip Alt, J L, Z A 2)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Wrap Text | drop-down | large | Alt, J L, T W |  |
| 2 | Bring Forward | split button | large | Alt, J L, A F |  |
| 3 | Send Backward | split button | large | Alt, J L, A E |  |
| 4 | Align | drop-down | large | Alt, J L, A A |  |
| 5 | Group | button | large | Alt, J L, G | Ctrl+Shift+G |
| 6 | Ungroup | button | large | Alt, J L, U | Ctrl+Shift+G |
| 7 | Rotate | drop-down | large | Alt, J L, A Y |  |

**Size** (KeyTip Alt, J L, Z S)

| # | Control | Type | Size | KeyTip | Shortcut |
|---|---|---|---|---|---|
| 1 | Grow to Fit Text | check box | small | Alt, J L, F |  |
| 2 | Height | label | small | Alt, J L, T H |  |
| 3 | Width | label | small | Alt, J L, W |  |
| 4 | Height | edit | small | Alt, J L, T H |  |
| 5 | Width | edit | small | Alt, J L, W |  |
| 6 | Size... | dialog launcher | small | Alt, J L, S Z |  |


Autoshape Shape Format: same four groups as the text-box Shape Format tab (Insert Shapes, Shape Styles,
Arrange, Size); differences: the Shape Styles dialog launcher reads "Format AutoShape..." (text boxes:
"Format Text Box..."), and there is no Text Box tab.
In Arrange on all contextual tabs the controls are large (Wrap Text, Bring Forward, Send Backward, Align,
Group, Ungroup, Rotate) except on Picture Format, where Arrange Thumbnails and Wrap Text are large and the
rest are small, and on Home where all but Wrap Text are small.

## 5. Right-click (context) menus

Reached by selecting the object and pressing Shift+F10 (the same menu as right-click). Separators noted with "|".

Text box: Best Fit, Shrink Text On Overflow, Grow Text Box to Fit, Do Not Autofit (these four are the
Text Fit options; greyed when not applicable) | Create Text Box Link | Cut, Copy, Paste Options: (icon row) |
Delete Text, Delete Object | Save as Building Block..., Save as Picture... | Change Text (submenu) | Format Text Box...,
Look Up..., Translate | Zoom (submenu) | Hyperlink...

Autoshape: Cut, Copy, Paste Options: (icon row) | Delete Object | Save as Building Block..., Save as Picture... |
Add Text | Format AutoShape... | Zoom (submenu) | Hyperlink...

Menus use the modern flyout style: white, light border and shadow, icons in a left column, underlined
access keys, a chevron for submenus, and the first row shows a thick black keyboard focus rectangle.

## 6. Differences from Publisher 2010 (and from our current app)

Observed by running Publisher 2010 (14.0.7248) and 2021 side by side with the same method. The ribbon structure
is almost unchanged; what changed is mostly chrome, the backstage and a few labels.

Genuine 2010 vs 2021 changes:
- Chrome: 2021 has a Colorful (green) theme with flat line icons; 2010 is a white/blue-grey look with a smaller
  green File tab and a light status bar.
- Title bar: QAT is on the left above the tab row; centred title reads "name.pub - Publisher" (2010: "Microsoft
  Publisher"); right side has the signed-in user widget; Touch/Mouse Mode button added to the QAT.
- New **Help** tab (Alt, Y): Help, Contact Support..., Feedback.
- Backstage: completely redesigned: Home (templates plus recent), New (Office/Built-In tabs, online search),
  Open/Save As (places: Recent, OneDrive, This PC, Add a Place, Browse), Info, Print, Share, Export, Close, Account,
  Feedback, Options. 2010 had Save, Save As, Open, Close, Info, Recent, New, Print, Save & Send, Help, Options,
  Exit (and no Home/Share/Export/Account/Feedback).
- Home tab: "Picture..." became "Pictures..."; no other control changes.
- Insert tab: Illustrations now Pictures..., Online Pictures... (Clip Art removed); the Hyperlink... button is now
  "Link" (Ctrl+K); Building Block Library is the group's dialog launcher.
- Page Design: schemes list adds "Office" (new palette) and "Office 2007-2010"; a new **Apply Image** split button joins
  Background and Master Pages in the Page Background group.
- Combo box labels lost trailing colons ("Font:" -> "Font", "Font Size:" -> "Font Size", "Zoom:" -> "Zoom").
- Status bar: dark-green coloured (2010: light grey-blue); page indicator, object position and size are
  kept; "Whole Page" fit button is at the far right; zoom percent is a button.
- Page Navigation pane: titled "Pages" in large text (2010: "Page Navigation" in the pane header with a collapse arrow).
- Contextual tabs in 2021: Shape Format and Text Box (text box), Picture Format, Table Design and Layout; see section 4.
- Review, View and Mailings group contents are identical to 2010 apart from the colon removal above.

Differences from our current app (crates/kernpage/src, read-only inspection):
- Our ribbon has the same six tabs and (mostly) the same group names as Publisher; there is no Help tab and no
  contextual tabs (Shape Format, Text Box, Picture Format, Table Design/Layout).
- Our backstage (backstage.rs) follows the 2010 entry list (Save, Save As, Open, Close, Info, New, Print, Save & Send,
  Exit) with a blue accent; 2021 needs the green rail with Home/New/Open/Info/Save/Save As/Print/Share/Export/Close
  then Account/Feedback/Options and the new page layouts above.
- Our theme is blue-grey (RIBBON_BG #F4F6FA, STRIP_BG #DDE3EE, accent blue); 2021 Colorful is white ribbon, grey canvas
  (#F0F0F0) and a dark-green accent for File tab, status bar, backstage rail and selection highlights.
- Our Insert > Illustrations still reads "Picture / Clip Art / Shapes / Picture Placeholder" (2021: Pictures... / Online
  Pictures... / Shapes / Picture Placeholder); "Hyperlink" should read "Link".
- Status bar: ours is light with page, cursor position and zoom; 2021 is dark green with page button, position, size,
  Single Page / Two-Page Spread buttons, zoom slider and the Whole Page button.
- Title bar: add Quick Access Toolbar (Save, Undo, Redo, Touch/Mouse Mode, Customize), centred "name.pub - Publisher".
- Left pane heading should read "Pages" (ours: "Page Navigation").

## 7. Gaps and caveats

- Not opened: Share, Account, Feedback, Options (online/personal), and any online template or picture search.
- Contents of drop-down menus and galleries (Shapes, Table, Page Parts, Styles, Margins, etc.) were not expanded;
  only the ribbon surface was inventoried.
- Right-click menus were captured from screenshots, not UIA (popups were not exposed); item enabled/disabled state
  is as seen for the sample selection.
- Collapsed-ribbon layouts at narrow window widths were not examined (all dumps at 1920 px wide).
- Table Design shows the Table Formats group, the Borders group, WordArt Styles and Typography as observed; extra
  groups may appear when a table cell range is selected.
- Screenshots in local evidence files (git-ignored) include the signed-in user name in the title bar and recent file paths;
  they must not be committed or shared.
