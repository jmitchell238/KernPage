# KernPage: UI familiarity checklist

Goal: someone who used Publisher 2010 can open KernPage and find things where they
expect them. Source: screenshots of Publisher 2010 taken by the project owner (2026-11-04).

**Rules.** We replicate *layout, workflow, groupings and plain-word command names*. We do
**not** copy icons, artwork, colors, template designs, color-scheme sets/names, the product
name or logos. All visuals are our own or from an open-licensed icon set.

## 1. Main window layout
- [ ] Title bar: `<document name> - KernPage`
- [ ] Quick-access buttons at top-left: Save, Undo (with history dropdown), Redo
- [ ] Tab strip: **File** (accent-colored, opens a full-window "backstage"), Home, Insert,
      Page Design, Review, View
- [ ] Ribbon under tabs: commands in labeled groups; a small launcher arrow at a group's
      bottom-right opens the full dialog for that group (Font, Paragraph, Page Setup…)
- [ ] Ribbon can be collapsed (chevron at top-right) and has a Help button
- [ ] Left pane: **Page Navigation**: page thumbnails with page numbers, current page
      highlighted, collapsible
- [ ] Horizontal and vertical rulers in inches, zero point at the page's top-left corner,
      ruler extends across the pasteboard
- [ ] Center: page on a gray **scratch area** (pasteboard); objects can be parked off-page
- [ ] Margin guides shown as dotted lines inside the page
- [ ] Status bar: `Page: 1 of 1`, live cursor position in inches (e.g. `10.44, 3.52 in.`),
      view toggles (single page / two-page spread), zoom percentage, zoom slider with −/+,
      fit-page button

## 2. File (backstage) view
Left column: Save, Save As, Open, Close, Info, Recent, **New**, Print, Save & Send, Help,
Options, Exit.

**New** screen (shown at startup):
- [ ] Source dropdown ("installed and online templates") + template search box
- [ ] Back/forward/home breadcrumb
- [ ] Blank page sizes first (Letter portrait, Letter landscape, more sizes, my templates)
- [ ] "Most Popular" category tiles: Brochures, Business Cards, Calendars, Greeting Cards,
      Labels, Newsletters, Postcards
- [ ] "More Templates" category tiles: Advertisements, Award Certificates, Banners,
      Business Forms, Catalogs, E-mail, Envelopes, Flyers, Gift Certificates, Import Word
      Documents, Invitation Cards, Letterhead, Menus, Paper Folding, Programs, Quick
      Publications, Resumes, Signs, With Compliments cards, More Categories
- [ ] Add for our audience: **Church Bulletin**, **Order of Service**, **Prayer List**

## 3. Home tab
| Group | Commands |
|---|---|
| Clipboard | Paste (with options), Cut, Copy, Format Painter |
| Font | font name, size, grow/shrink font, clear formatting; Bold, Italic, Underline, subscript, superscript, change case, character spacing, font color |
| Paragraph | bullets, numbering, decrease/increase indent, columns, show ¶ marks; align left/center/right/justify, line spacing, paragraph shading |
| Styles | Styles gallery |
| Objects | Draw Text Box, Picture, Table, Shapes |
| Arrange | Wrap Text, Bring Forward, Send Backward, Align, Group, Ungroup, Rotate |
| Editing | Find, Replace, Select |

## 4. Insert tab
| Group | Commands |
|---|---|
| Pages | Page (insert blank/duplicate), Catalog Pages |
| Tables | Table |
| Illustrations | Picture, Clip Art, Shapes, Picture Placeholder |
| Building Blocks | Page Parts, Calendars, Borders & Accents, Advertisements |
| Text | Draw Text Box, Business Information, WordArt, Insert File, Symbol, Date & Time, Object |
| Links | Hyperlink, Bookmark |
| Header & Footer | Header, Footer, Page Number |

## 5. Page Design tab
| Group | Commands |
|---|---|
| Template | Change Template, Options |
| Page Setup | Margins, Orientation, Size (+ dialog launcher) |
| Layout | Guides, Align To: Guides / Objects (checkboxes) |
| Pages | Delete, Move, Rename |
| Palettes | colour-palette gallery (each palette shown as colour swatches + name); our own names and colours |
| Page Background | Fonts (font scheme), Background, Master Pages |

## 6. Data tab (our own design, mock-up only; bead pw-3iuc.1)

This tab is our own layout, not Publisher's Mailings tab. Every control is greyed and inert for now.

| Group | Commands |
|---|---|
| List | Data Source (large, drop-down), Edit List |
| Fields | Insert Field (large, drop-down), Picture Field, Numbering |
| Layout | Make Copies, Repeating Area, Per Sheet (drop-down) |
| Check | Preview, row navigator (first, previous, "1 of 0" box, next, last), Find Row |
| Create | Create (drop-down): new publication, one PDF, one PDF per row, or print |

## 7. Review tab
| Group | Commands |
|---|---|
| Proofing | Spelling, Research, Thesaurus |
| Language | Translate Selected Text, Language |

## 8. View tab
| Group | Commands |
|---|---|
| Views | Normal, Master Page |
| Layout | Single Page, Two-Page Spread |
| Show | Boundaries, Guides, Fields, Rulers, Page Navigation, Scratch Area, Graphics Manager, Baselines (checkboxes) |
| Zoom | 100%, Whole Page, zoom box, Page Width, Selected Objects |
| Window | Arrange All, Cascade, Switch Windows |

## 9. Priorities for version 1 (church bulletins/newsletters)
Must: window layout (§1), New screen with blank sizes + our own templates, Home tab,
Insert → Page/Picture/Shapes/Draw Text Box/Table/Page Number, Page Design → Margins/
Orientation/Size/Guides/Master Pages, View → zoom + show toggles, open `.pub`, print, export PDF.
Later: Review, Building Blocks, WordArt, Clip Art, Business Information.
