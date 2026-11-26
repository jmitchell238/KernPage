# Publisher 2021 object model and feature inventory

Source: the public automation interface of Publisher 2021 (what its COM object model exposes to scripts: objects,
properties, methods, events and enumerations), as seen through Windows' standard type-information interface. Almost
every user-visible feature has a counterpart there, so it is a good checklist. This file restates it in our own
words and groups it into user features. It is research input for ROADMAP.md; it is not a specification of the
`.pub` file format.

| What | Count |
|---|---|
| Automation interfaces (objects, collections, event sinks) | 131 (129 real objects and collections, 2 duplicate event interfaces) |
| Members on those interfaces (properties plus methods, counted per interface, so a collection's `Count`, `Item` and `Parent` appear on each) | 1638 (1227 properties, 411 methods) |
| Enumerations | 104, with 1129 named constants |
| Event sinks | ApplicationEvents (23 events), DocumentEvents (7 events), ModalBrowser (3), ICagNotifySink (2) |

Caveats:

- The library refers to many drawing enumerations by name without defining them. They live in Office's shared
  library (names starting `Mso`: auto-shape types, gradient styles, arrowheads, dash styles, 3-D presets,
  textures, patterns, shadow types, fill types, flip/align/distribute/z-order commands and so on). Section 3
  lists what each one offers from general Office knowledge and marks it as external. Dump the shared Office object model
  in a follow-up to pin the exact members.
- The automation model reflects what Publisher lets a script do. A few user features (spell check, Design
  Checker, Pack and Go, the Undo list, the object tabs' galleries, many dialogs) are not exposed to
  automation and so are absent here; the UI inventory (`publisher-2021-ui.md`) covers them. Part 2 of the
  inventory should cross-check the two.
- Object names ending in a number (`Add10`, `Execute10`, `SaveAsPicture10`, `EnterColorMode10`) are older
  signatures kept for compatibility, not separate features.

## 1. Objects and what each one represents

### Application, windows and documents

| Object | Feature it represents |
|---|---|
| Application | The program itself: open/new/quit, global options, unit conversion, template folder, installed printers, security level, wizard (template) catalog |
| Documents | The set of open publications |
| Document | One publication: pages, master pages, sections, stories, styles, colour mode, save/export/print, undo, find, business information |
| Window | A publication window: caption, size, position, state |
| View | Zoom, current page, scroll-into-view |
| Selection | What is selected (shapes, text, table cells) |
| Options | The Options dialog settings (units, autosave, hyphenation, typing helpers, envelope printing, text direction) |
| WebOptions | Options for saving as web pages and email (encoding, folder layout, web fonts, VML) |
| FindReplace | Find and Replace in text |
| ApplicationEvents / IApplicationEvents | Notifications to add-ins: window, new/open/close, print, mail merge steps, catalog UI |
| DocumentEvents / IDocumentEvents | Notifications: open, close, shapes added/removed, undo, redo, wizard changed |
| ModalBrowser, ICagNotifySink | Web-based content (online clip art / catalog) hosted in a window |

### Pages and layout

| Object | Feature it represents |
|---|---|
| Pages / Page | Pages of the publication: add, delete, duplicate, move, export a page as a picture or email HTML |
| MasterPages | Master pages (one or two-page) and which pages use them |
| ReaderSpread | The pair of pages shown side by side |
| Sections / Section | Page-numbering sections with their own start number and format |
| HeaderFooter | Header and footer area of a page |
| PageBackground | A background fill for a page |
| PageSetup | Page size, margins, orientation, label sheet and layout type |
| PageSizes / PageSize | The list of preset page sizes |
| Labels / Label | The list of label products and sheet geometry |
| LayoutGuides | Margin, column, row and baseline guides |
| RulerGuides / RulerGuide | Individual ruler guides |
| ScratchArea | The off-page area beside the page |
| WebPageOptions | Per-page web options: description, keywords, background sound, publish file name |

### Shapes and drawing

| Object | Feature it represents |
|---|---|
| Shapes | The objects on a page; every "Insert" command that creates an object |
| Shape / ShapeRange | A single object / several selected objects: position, size, rotation, flip, order, group, copy, alt text, tags |
| GroupShapes | Members of a group |
| Adjustments | Yellow-handle adjustments on an AutoShape |
| FreeformBuilder | Drawing a freeform (polyline/curve) shape point by point |
| ShapeNodes / ShapeNode | Edit Points: node position, segment and node type, insert, delete |
| ConnectorFormat | Connectors that stay attached to shapes |
| CalloutFormat | Callout (speech bubble) geometry |
| FillFormat | Solid, gradient, texture, pattern and picture fills |
| LineFormat | Outline: colour, weight, dash, arrowheads, caps, joins, compound styles, gradient lines |
| ShadowFormat | Shadow effect |
| GlowFormat | Glow effect |
| ReflectionFormat | Reflection effect |
| SoftEdgeFormat | Soft edges effect |
| ThreeDFormat | 3-D rotation, extrusion, bevel, material, lighting |
| WrapFormat | Text wrapping around an object |
| BorderArtFormat / BorderArts / BorderArt | BorderArt: decorative picture borders on shapes |
| TextEffectFormat | Classic WordArt shape and text-effect settings |
| PictureFormat | Picture properties and corrections: crop, brightness, contrast, recolour, transparency, links, resolution |
| LinkFormat | Link to an external file (linked pictures, linked OLE) |
| OLEFormat / ObjectVerbs | Embedded and linked OLE objects and their verbs (Edit, Open) |
| Hyperlinks / Hyperlink | Hyperlinks on text or objects (web, email, page, file) |
| InlineShapes | Objects anchored in text and flowing with it |
| Tags / Tag | Custom name/value tags attached to objects, pages, documents |
| BuildingBlocks / BuildingBlock | The Building Block library (page parts, calendars, borders and accents, ads, business information) |

### Text

| Object | Feature it represents |
|---|---|
| TextFrame | Text box: margins, columns, autofit, linking, overflow, vertical alignment, orientation |
| Story | A chain of linked text boxes (or a table's text); Stories is the list of all of them |
| TextRange | Any run of text: edit, select, count, find, insert symbols, fields, date/time, page numbers |
| Font | Character formatting, including OpenType features |
| ParagraphFormat | Paragraph formatting: alignment, indents, spacing, tabs, lists, keeps |
| TabStops / TabStop | Tab stops with alignment and leader |
| DropCap | Drop capitals |
| TextStyles / TextStyle | Named paragraph/character styles |
| Fields / Field | Live fields in text: page number, date/time, mail merge field, phonetic guide |
| PhoneticGuide | Phonetic guide (ruby) text over East Asian characters |

### Tables

| Object | Feature it represents |
|---|---|
| Table | A table: rows, columns, direction, auto-format, grow to fit |
| Columns / Column, Rows / Row | Column and row collections and single items (width, height, add, delete) |
| CellRange / Cell | Cell ranges and single cells: merge, split, borders, diagonals, fill, margins, vertical alignment, text orientation |
| CellBorder | One cell border: colour and weight |

### Colour, schemes and colour printing

| Object | Feature it represents |
|---|---|
| ColorSchemes / ColorScheme | The colour scheme list and a scheme's colours |
| ColorFormat | A colour: RGB, CMYK, scheme colour, spot ink, tint, transparency |
| ColorCMYK | A CMYK colour value |
| ColorsInUse | Colours used in the publication (Colours in Use / replace colour) |
| Plates / Plate | Colour separation plates (process and spot inks) with screen angle and frequency |
| PrintablePlates / PrintablePlate | The plates chosen for output |
| AdvancedPrintOptions | Commercial print settings: marks, bleed, separations, flips, halftone, resolution, fonts |

### Printing and output

| Object | Feature it represents |
|---|---|
| InstalledPrinters / Printer | Printers and their capabilities (colour, duplex, paper sizes, sources, driver kind) |
| PrintableRect | Printable area of a printer |

### Mail merge, catalog merge and email merge

| Object | Feature it represents |
|---|---|
| MailMerge | Merge setup: type (letter/catalog/email), data source, destination, wizard |
| MailMergeDataSource / DataSources | The recipient list and the list of sources |
| MailMergeDataFields / DataField | Columns of the recipient list and inserting them as fields |
| MailMergeMappedDataFields / MappedDataField | Mapping list columns to standard address fields |
| MailMergeFilters / FilterCriterion | Filter rows with conditions joined by And/Or |
| EmailMergeEnvelope | Email merge headers: To, Cc, Bcc, subject, priority |
| Attachments / Attachment | Files attached to merged emails |
| CatalogMergeShapes | The repeated merge area for a catalog merge |
| CaptionStyles / CaptionStyle | Picture caption styles |

### Web publishing, forms and templates

| Object | Feature it represents |
|---|---|
| WebNavigationBarSets / Set / Hyperlinks | Navigation bars shared across web pages |
| WebCommandButton, WebTextBox, WebListBox(Items), WebOptionButton, WebCheckBox, WebHiddenFields | Web form controls and how submitted data is sent |
| Wizard / WizardProperties / WizardProperty / WizardValues / WizardValue | Template ("wizard") design options on a publication, page or object, such as the Options panes in the Page Design pane |

## 2. Features per object (grouped, with member names as evidence)

### Application and Document

- **Open, create, close, save**: Application.NewDocument, Open, Quit, Documents.Add; Document.Save, SaveAs, Close, ReadOnly, Saved, FullName, Path, SaveFormat (publication, Publisher 98/2000 formats, HTML, web archive, RTF, plain/Unicode text).
- **Export**: Document.ExportAsFixedFormat (PDF or XPS, with intent minimum/standard/printing/commercial, page range, bookmarks, include non-printing, picture resolution), Page.SaveAsPicture, Shape.SaveAsPicture, Page.ExportEmailHTML, Document.WebPagePreview, Document.ConvertPublicationType (print to web/email and back).
- **Printing**: Document.PrintOut, PrintOutEx, ActivePrinter, PrintStyle (one per sheet, tiled, multiple copies, multiple pages, booklets, quarter/half folds, envelope), PrintPageBackgrounds, Application.PrintPreview, Options.PrintLineByLine, DisplayPrintTroubleshooter.
- **Colour modes**: Document.ColorMode (desktop, process, spot, black and white, spot plus process), EnterColorMode, CreatePlateCollection, Plates.
- **Undo and redo**: Document.Undo, Redo, UndoClear, UndoActionsAvailable, RedoActionsAvailable, BeginCustomUndoAction, EndCustomUndoAction.
- **Find and replace**: Document.Find, FindReplace (find text, replace with, direction, match case, whole word, replace one/all, East Asian match options: width, kashida, diacritics, Alef Hamza).
- **Views and guides on/off**: ViewBoundaries, ViewGuides, ViewBoundariesAndGuides, ViewTwoPageSpread, ViewHorizontalBaseLineGuides, ViewVerticalBaseLineGuides, ActiveView, View.Zoom, ZoomIn, ZoomOut, ScrollShapeIntoView, Application.SnapToGuides, SnapToObjects.
- **Business information**: Document.PersonalInformationSet, SetBusinessInformation, RemovePersonalInformation, Options.UpdatePersonalInfoOnSave (four sets: primary business, secondary business, other organisation, home).
- **Publication type and templates**: Document.PublicationType (print or web), IsWizard, Wizard, Application.ShowWizardCatalog, WizardCatalogVisible, TemplateFolderPath, Options.UseCatalogAtStartup, UseWizardForBlankPublication, ChangeDocument.
- **Fonts and colours**: Document.ColorScheme, ColorsInUse, TextStyles, DefaultTabStop.
- **Tags and find by tag**: Document.Tags, FindShapesByTag, FindShapeByWizardTag, SurplusShapes (overflow items).
- **Building blocks**: Document.AvailableBuildingBlocks.
- **Language and direction**: Application.Language, Document.DocumentDirection, Options.DefaultPubDirection, DefaultTextFlowDirection, AutoKeyboardSwitching, AddHebDoubleQuote.
- **Options dialog**: AllowBackgroundSave, AutoHyphenate and HyphenationZone, AutoSelectWord, AutoFormatWord, TypeNReplace, SequenceCheck, DragAndDropText, MeasurementUnit, PathForPictures, PathForPublications, SaveAutoRecoverInfo and interval, ShowBasicColors, ShowScreenTipsOnObjects, ShowTipPages, ShowStatusBar, UseHelpfulMousePointers, EnvelopePrintPlacement/Orientation, UseEnvelopePaperSizes, UseEnvelopePrintOptions.
- **Unit conversion**: Application.InchesToPoints and fourteen siblings (cm, mm, pica, pixel, twip, line, EMU, and back).
- **Online or add-in surfaces**: Application.LaunchWebService, ModalBrowser, ICagNotifySink, COMAddIns, CommandBars, FileSearch, Assistant, Assistance, OfficeDataSourceObject, MsoDebugOptions, AutomationSecurity.
- **Merge-related application flags**: ValidateAddressVisible, InsertBarcodeVisible, ShowFollowUpCustom.

### Window and View

- Window position, size, state (normal, minimised, maximised), activate, move, resize.
- View zoom presets (page width, whole page, fit selection), zoom in/out.

### Page, Pages, MasterPages, Sections, HeaderFooter, PageBackground

- **Pages**: Add (with before/after, blank or from wizard page type), AddWizardPage, FindByPageID, Delete, Duplicate, Move, PageNumber, PageIndex, PageType (left, right, scratch, master), IsLeading/IsTrailing (first and last of a spread), Name.
- **Master pages**: MasterPages.Add (one or two-page, with name), Page.Master (assign), IgnoreMaster (hide master items on a page), IsTwoPageMaster, MasterPageType (left or right).
- **Headers and footers**: Page.Header, Footer, HeaderFooter.TextRange, IsHeader, Delete.
- **Background**: PageBackground.Fill, Create, Delete, Exists; Document.PrintPageBackgrounds.
- **Sections**: Add, Delete, StartPageIndex, PageNumberStart, ContinueNumbersFromPreviousSection, PageNumberFormat (Arabic, upper/lower Roman, letters, ordinal, cardinal text, many East Asian/Hebrew/Arabic/Hindi/Thai/Vietnamese/Russian styles), ShowHeaderFooterOnFirstPage.
- **Reader spreads**: Page.ReaderSpread, X/YOffsetWithinReaderSpread, Pages, PageCount.
- **Web page options**: Description, Keywords, IncludePageOnNewWebNavigationBars, BackgroundSound with loop count/forever, PublishFileName.

### PageSetup, PageSizes, Labels, LayoutGuides, RulerGuides

- **Page size and orientation**: PageWidth, PageHeight, Orientation, PageSize, AvailablePageSizes, PageSizes (name, width, height, margins, gaps, has background image), PublicationLayout.
- **Multiple pages per sheet and label sheets**: MultiplePagesPerSheet, HorizontalGap, VerticalGap, LeftMargin, TopMargin, Label, AvailableLabels, Labels (name, width, height, margins, gaps).
- **Layout guides**: MarginLeft/Right/Top/Bottom, MirrorGuides, Columns, Rows, ColumnGutterWidth, RowGutterWidth, GutterCenterlines, HorizontalBaseLineOffset/Spacing, VerticalBaseLineOffset/Spacing.
- **Ruler guides**: Add (horizontal or vertical at a position), Position, Type, Delete.

### Shapes, Shape and ShapeRange

- **Creating objects** (Shapes.Add...): text box, label, line, connector, auto-shape, callout, curve, polyline, freeform (BuildFreeform), picture, empty picture frame, OLE object, table, WordArt/text effect, web control, web navigation bar, group wizard (design-gallery parts), building block, catalog merge area/field, paste, select all, range.
- **Geometry**: Left, Top, Width, Height, Rotation, HorizontalFlip, VerticalFlip, Flip, LockAspectRatio, ScaleHeight/ScaleWidth (relative to original or current), IncrementLeft/Top/Rotation, GetLeft/Top/Width/Height, MoveToPage.
- **Order and arrangement**: ZOrderPosition, ZOrder (front, back, forward, back one), Align, Distribute (ShapeRange), Group, Ungroup, Regroup, GroupItems, IsGroupMember, ParentGroupShape.
- **Clipboard and copy**: Cut, Copy, Delete, Duplicate, PickUp and Apply (copy formatting), SetShapesDefaultProperties (default for new shapes).
- **Identity and metadata**: Name, ID, AlternativeText, Tags, WizardTag, WizardTagInstance, Hyperlink, Type, HasTextFrame, HasTable, BlackWhiteMode (print in black and white: automatic, grey scale, light grey, black and white, dont show), IsExcess.
- **Inline objects**: IsInline, InlineAlignment (character, left, right), InlineTextRange, MoveIntoTextFlow, MoveOutOfTextFlow, InlineShapes.
- **Connectors**: Connector, ConnectorFormat (type straight/elbow/curve, begin/end connected shape and site), RerouteConnections, ConnectionSiteCount.
- **Vertices and editing**: Nodes, Vertices, ShapeNodes.Insert/Delete/SetPosition/SetSegmentType/SetEditingType.
- **Catalog merge membership**: AddToCatalogMergeArea, RemoveFromCatalogMergeArea, RemoveCatalogMergeArea, CatalogMergeItems, CatalogMergeShapes.HorizontalRepeat/VerticalRepeat.
- **Save objects**: SaveAsPicture (format and options), SaveAsBuildingBlock (name, category, keywords, description, gallery).
- **Captions**: SetCaption (with caption style), CaptionStyles.
- **Effects**: Fill, Line, Shadow, Glow, Reflection, SoftEdge, ThreeD, TextEffect, BorderArt, TextWrap (and WrapFormat), WebComponentFormat.
- **Web-only**: WebCommandButton, WebListBox, WebTextBox, WebOptionButton, WebCheckBox, WebNavigationBarSetName, Wizard.

### Fill, line and effects

- **Fill**: Visible, Type (solid, patterned, gradient, textured, picture, background), ForeColor, BackColor, Transparency, Solid, OneColorGradient and TwoColorGradient (style and variant, degree), PresetGradient (named gradients), GradientAngle, GradientColorType, Patterned (pattern list), PresetTextured and UserTextured (texture name/type/offset/scale/alignment), UserPicture, TextureHorizontalScale/VerticalScale, TextureOffsetX/Y, TextureAlignment, RotateWithObject.
- **Line**: Visible, ForeColor, BackColor, Pattern, Weight, DashStyle, Style (single, thin-thin, thick-thin and so on), Transparency, BeginArrowhead/EndArrowhead Style/Length/Width, CapStyle (flat, round, square), JoinStyle (round, bevel, mitre), InsetPen, line gradients (gradient colour type, style, variant, angle, preset), Type.
- **Shadow**: Visible, Type (preset shadows), ForeColor, Transparency, OffsetX/Y, Blur, Size, Obscured, RotateWithShape, IncrementOffsetX/Y.
- **Glow**: Radius, Color, Transparency, Visible. **Reflection**: Type, Transparency, Size, Offset, Blur, Visible. **Soft edges**: Type, Radius, Visible.
- **3-D**: Visible, Depth, ExtrusionColor and type, Perspective, FieldOfView, RotationX/Y with increments and ResetRotation, PresetExtrusionDirection, PresetLightingDirection and Softness, PresetMaterial, PresetThreeDFormat, SetThreeDFormat, SetExtrusionDirection, bevel top/bottom (type, inset, depth), contour width and colour.
- **Wrap**: Type (none, square, tight, through, top and bottom), Side (both, left, right, largest, neither), DistanceAuto, DistanceTop/Left/Bottom/Right; edit wrap points is exposed through Vertices.
- **Callouts**: Type, Angle, AutoAttach, AutoLength, Border, Accent, Drop and DropType, Gap, Length, PresetDrop, CustomDrop, CustomLength.
- **BorderArt**: Name, Weight, Color, StretchPictures, Exists, Set, Delete, RevertToDefaultWeight, RevertToOriginalColor; BorderArts lists the library.
- **WordArt (classic)**: TextEffect.Text, FontName, FontSize, FontBold, FontItalic, Alignment, KernedPairs, NormalizedHeight, RotatedChars, Tracking, PresetShape, PresetTextEffect, PresetWordArt, ToggleVerticalText; Shapes.AddTextEffect, AddWordArt.

### PictureFormat

- **Crop and fit**: CropTop/Bottom/Left/Right, ClearCrop, FillFrame, FitFrame, HorizontalPictureLocking, VerticalPictureLocking (anchor/stretch when the frame is resized), HorizontalScale, VerticalScale.
- **Corrections**: Brightness, Contrast, IncrementBrightness/Contrast, ColorType (automatic, greyscale, black and white, watermark), Recolor, RecoloredPictureColor, IsRecolored, LeaveBlackAsBlack, RestoreOriginalColors.
- **Transparency**: TransparencyColor, TransparentBackground, HasTransparencyColor, HasAlphaChannel.
- **Source and links**: Filename, Replace and ReplaceEx (embedded, linked or original state), Remove, IsLinked, LinkedFileStatus (OK, missing, modified), IsEmpty, Shapes.AddEmptyPictureFrame.
- **Info (read-only)**: ColorModel, ColorsInPalette, EffectiveResolution, FileSize, ImageFormat (EMF, WMF, PICT, JPEG, PNG, DIB, GIF, TIFF, CMYK JPEG), IsGreyScale, IsTrueColor, Original* (width, height, resolution, size, palette, alpha, true colour), Width, Height.
- Inserted picture fit choice: fit or fill (PictureInsertFit); picture resolution presets 96, 150, 300 dpi.

### TextFrame and Story

- **Text box geometry**: MarginLeft/Top/Right/Bottom, Columns, ColumnSpacing, VerticalTextAlignment, Orientation (horizontal, East Asian vertical, right to left).
- **Autofit**: AutoFitText (none, shrink on overflow, best fit, grow to fit).
- **Linking**: NextLinkedTextFrame, PreviousLinkedTextFrame, HasNextLink, HasPreviousLink, BreakForwardLink, ValidLinkTarget, Overflowing, IncludeContinuedOnPage, IncludeContinuedFromPage ("Continued on page" and "from" notes), Story, HasText.
- **Story**: Type (table, text frame, continued from, continued on), TextRange, TextFrame, Table; Stories is the document-wide list for text-flow operations.

### TextRange

- **Navigation and selection**: Start, End, Length, Collapse, Expand, Move, MoveStart, MoveEnd, Characters, Words, Lines, Paragraphs, Select, BoundLeft/Top/Width/Height, ContainingObject, Story.
- **Editing**: Text, InsertAfter, InsertBefore, Delete, Cut, Copy, Paste, Duplicate, InsertSymbol.
- **Insertion**: InsertDateTime (23 formats including East Asian; calendar types), InsertPageNumber (current, next in story, previous in story), InsertMailMergeField, InsertBarcode, Fields.
- **Counts**: WordsCount, LinesCount, ParagraphsCount.
- **Per-run properties**: Font, ParagraphFormat, MajorityFont, MajorityParagraphFormat, LanguageID, Script, DropCap, Hyperlinks, InlineShapes, Find.

### Font

- **Basic**: Name, Size, Bold, Italic, Underline (about 18 styles including double, dotted, dashed, wavy, thick, words only), Color, StrikeThrough, SuperScript, SubScript, AllCaps, SmallCaps, Outline, Shadow, Emboss, Engrave.
- **Spacing**: Tracking, TrackingPreset (very tight to very loose), Scaling, Kerning, AutomaticPairKerningThreshold, Position (baseline shift).
- **OpenType**: Ligature presets, NumberStyle (proportional/tabular, lining/oldstyle), Swash, StylisticAlternates, ContextualAlternates, StylisticSets.
- **Text effects**: Fill (text fill), Line (text outline), Glow, Reflection, TextShadow, ThreeD.
- **Complex scripts**: BoldBi, ItalicBi, SizeBi, ExpandUsingKashida, DiacriticColor, UseDiacriticColor, GetScriptName/SetScriptName (about 45 script slots).
- **Actions**: Grow, Shrink (bigger/smaller font step), Reset (clear character formatting), Duplicate, AttachedToText.

### ParagraphFormat, TabStops, DropCap, TextStyle

- **Alignment**: left, centre, right, justified, distributed, plus East Asian and Arabic kashida variants (14 values), KashidaPercentage.
- **Indents and spacing**: FirstLineIndent, LeftIndent, RightIndent, CharBasedFirstLineIndent, SpaceBefore, SpaceAfter, LineSpacing, LineSpacingRule (single, 1.5, double, exactly, multiple), SetLineSpacing.
- **Lists**: ListType (bullet, none, Arabic, Roman, letters, ordinals, text numbers, and many East Asian, Hebrew, Arabic, Hindi, Thai, Vietnamese, Russian styles), ListBulletText, ListBulletFontName, ListBulletFontSize, ListIndent, ListNumberSeparator (period, parenthesis, both, colon, square, hyphen and more), ListNumberStart, SetListType.
- **Flow control**: KeepLinesTogether, KeepWithNext, WidowControl, StartInNextTextBox, LockToBaseLine (snap to baseline grid).
- **Direction and style**: TextDirection, TextStyle (apply a named style), Reset (clear paragraph formatting).
- **Tabs**: TabStops.Add, Item, Clear, ClearAll; Position, Alignment (left, centre, right, decimal), Leader (none, dots, dashes, line, bullet).
- **Drop caps**: ApplyCustomDropCap (font, colour, bold, italic, lines up, size, span), Clear.
- **Styles**: TextStyles.Add (name, based on, next style, description), TextStyle.Font, ParagraphFormat, BaseStyle, NextParagraphStyle, Delete.

### Fields

- Field types: page number (current, next, previous), date and time, mail merge field, "IHIV" (information fields from Business Information), phonetic guide, wizard sample text, hyperlink fields (URL, relative page, absolute page, email, file, personalised URL).
- Fields.Item, Unlink, AddHorizontalInVertical (East Asian horizontal text inside vertical text), AddPhoneticGuide (font, size, raise, alignment, base text).

### Tables

- **Structure**: Rows.Add, Columns.Add, Row.Delete, Column.Delete, Row.Height, Column.Width, Cell.Merge, CellRange.Merge, Cell.Split, CellRange.Select, Cell.Select.
- **Cell look**: BorderTop/Bottom/Left/Right (weight, colour), BorderDiagonal and Diagonal direction (up, down), Fill, MarginTop/Bottom/Left/Right, VerticalTextAlignment, CellTextOrientation (horizontal, vertical, right-to-left).
- **Table-wide**: ApplyAutoFormat (about 24 named table formats such as Checkbook Register, List 1 to 7, Numbers 1 to 6, Table of Contents 1 to 3, Checkerboard, each with five option switches), GrowToFitText, TableDirection (left-to-right, right-to-left).

### Hyperlinks, Tags, Wizard, BuildingBlock

- **Hyperlinks**: Add (address, text, screen tip, sub-address, target frame), Address, EmailSubject, TextToDisplay, PageID and SetPageRelative (jump to page), Type (none, URL, email, first/last/next/previous page, page ID, personalised), Shape or Range target, Delete, Move.
- **Tags**: Add, Name, Value, Delete on shapes, pages and documents.
- **Wizard/Design options**: Wizard.ID, Name, Properties; WizardProperty.ID, Name, CurrentValueId, Enabled, Values; SetId. These are the template option panes (colour scheme, font scheme, layout options, business information sets).
- **Building blocks**: Type (built-in, downloaded, user, work group), Name, Description, Gallery (advertisements, accents, calendars, business info, page parts), Category, Keywords, ShowInGallery, Delete.

### Colour

- ColorFormat: RGB, CMYK, SchemeColor (ten scheme slots: main, accent 1 to 5, hyperlink, followed hyperlink, none), Type (RGB, scheme, CMYK, colour-managed, ink), Ink (spot ink 1 to 12), TintAndShade, Transparency, BaseRGB/BaseCMYK.
- ColorSchemes: about 68 named schemes plus custom and user-defined slots; ColorScheme.Colors, Name.
- ColorsInUse: the list to inspect and replace; replace-tint rules (default, keep tints, maintain luminosity).
- Plates and inks: cyan, magenta, yellow, black, 12 spot slots; Plate.Angle, Frequency, Luminance, InUse, ConvertToProcess, Delete; PrintablePlate.PrintPlate (choose which inks to print).

### Print options and printers

- **Printer**: PrinterName, DriverType (non-PostScript, three PostScript levels, XPS), IsColor, IsDuplex, PaperSize, PaperHeight/Width, PaperOrientation, PaperSource, PrintMode (composite RGB, composite CMYK, composite greyscale, separations), PrintableRect, IsActivePrinter.
- **Advanced**: HorizontalFlip, VerticalFlip, NegativeImage, UseOnlyPublicationFonts, PrintCropMarks, PrintRegistrationMarks, PrintJobInformation, PrintDensityBars, PrintColorBars, AllowBleeds, PrintBleedMarks, PrintBlankPlates, GraphicsResolution (full, high, low, none), Resolution, InksToPrint (all, used, convert spot to process), UseCustomHalftone, PrintCMYKByDefault, IsPostscriptPrinter, BackSideInsertFaceUp, ManualFeedAlign, ManualFeedDirection, PageRotated.
- **Font license**: FontType (TrueType, printer, raster), FontSource (system, document, missing), FontLicenseLimitations (embeddable, print/preview, not embeddable).

### Mail merge

- **Types and flow**: MailMerge.Type (mail merge, catalog merge, email merge), ShowWizard and ShowWizardEx, Execute, WizardState, Destination (printer, new publication, existing publication, email), SuppressBlankLines, ViewMailMergeFieldCodes, DocumentUpdating, ShowSendToCustom, CreateShortcut.
- **Data**: OpenDataSource (file, connection, table, query), DataSource.ConnectString, TableName, Type, RecordCount, ActiveRecord, FirstRecord, LastRecord, Included (per record), InvalidAddress, InvalidComments, EverValidated, ValidatedClean, FindRecord, EditRecord, OpenRecipientsDialog, SetSortOrder (up to three keys), SetAllIncludedFlags, SetAllErrorFlags, ApplyFilter, Close, MergeInfo (ODSO or sub-ODSO), IsMaster, ExportRecipientList (to .mdb or .csv).
- **Fields**: DataFields (name, index, value, type string or picture, IsMapped, MappedTo), Insert, AddToRecipientFields, MapToRecipientField, UnMapRecipientField; MappedDataFields maps to standard columns (courtesy title, first, middle, last, suffix, nickname, job title, company, address 1 to 3, city, state, postal code, country, phones, faxes, email, web page, spouse name parts, ruby names, department).
- **Filters**: MailMergeFilters.Add/Delete with comparison (equal, not equal, less, greater, less/greater or equal, blank, not blank) and conjunction (and, or).
- **Email merge**: EmailMergeEnvelope To, Cc, Bcc, Subject, Priority (none, high, low), Attachments.Add/ClearAll.
- **Barcodes**: TextRange.InsertBarcode, MailMergeInsertBarcode and MailMergeGenerateBarcode events, BarCodePictureHolder shape type.
- **Address validation and follow-up**: ValidateAddressVisible, ShowFollowUpCustom, MailMergeWizardFollowUpCustom, MailMergeWizardSendToCustom events (hooks for third-party mailing services).
- **Catalog merge**: Shapes.AddCatalogMergeArea, AddCatalogMergeFieldToCanvas (text or picture field), CanvasArrangementType (one canvas, columns, rows), CanvasesCount, repeat counts, wizard page types for catalog pages.

### Web publishing

- Web navigation bars: AddSet, Design (25 styles), ButtonStyle (small, large, text), Orientation, HorizontalButtonCount, HorizontalAlignment, AutoUpdate, ShowSelected, Links (add page or URL links), AddToEveryPage, DeleteSetAndInstances, ChangeOrientation.
- Form controls: command button (submit or reset, action URL, email address and subject, data file name, retrieval method save on server/email/program, data format HTML/rich text/CSV/tab, post form data, hidden fields), single-line and multi-line text boxes (default text, asterisks, limit, required), list box (items, multi-select, selected), option button, check box (value, selected), return data label.
- HTML fragment, hot spot and web component shape types; WebOptions (encoding, folder, VML, web fonts only, incremental upload, email as image).
- Page-level web options: description, keywords, background sound; web template wizard page types (about us, FAQ, jobs, photo gallery, product list and so on).
- Web publication layouts: web page small and large, web site.

### OLE and linked objects

- OLEFormat: Object, ProgId, Activate, DoVerb, ObjectVerbs; LinkFormat: SourceFullName, Update; AddOLEObject (class, file, link, display as icon); shape types for embedded and linked OLE objects and OLE controls; Document.UpdateOLEObjects.

## 3. Enumerations: what choices they give the user

Names below drop the `pb`/`Pb` prefix. Items marked (Office) are defined in Office's shared library, not in this
one; their members here come from general knowledge and need a follow-up dump.

| Enumeration | Choices |
|---|---|
| SchemeColorIndex | Scheme slots: main (text), accent 1 to 5, hyperlink, followed hyperlink, none |
| ColorType | Mixed, RGB, scheme, CMYK, colour-managed, ink (spot) |
| ColorMode | Desktop, process, spot, black and white, spot plus process |
| ColorModel | RGB, CMYK, greyscale, unknown (picture info) |
| InkName / SpotColor | Cyan, magenta, yellow, black, spot 1 to 12 / none |
| ReplaceTint | Use default, keep tints, maintain luminosity |
| UnitType | Inch, cm, pica, point, EMU, twip, foot, metre, kyu, ha, pixel |
| OrientationType | Portrait, landscape |
| DirectionType, TextDirection, TableDirectionType | Left to right, right to left |
| PlacementType | Left, right, centre (envelope placement) |
| SelectionType | None, shapes, text, table cells, shape sub-selection |
| PictureInsertAs, PictureInsertFit | Embedded, linked, original state; fit, fill |
| PictureResolution | Default, web 96 dpi, desktop print 150 dpi, commercial print 300 dpi |
| LinkedFileStatus | OK, missing, modified |
| ImageFormat | EMF, WMF, PICT, JPEG, PNG, DIB, GIF, TIFF, CMYK JPEG |
| VerticalPictureLocking, HorizontalPictureLocking | None, top/bottom or left/right, stretch |
| PresetWordArt | 60 classic WordArt styles plus mixed |
| TrackingPresetType | Custom, very loose, loose, normal, tight, very tight |
| UnderlineType | None, single, words only, double, dotted, thick, dash, dot-dash, dot-dot-dash, wavy, wavy heavy, dot heavy, dash heavy, dot-dash heavy, dot-dot-dash heavy, long dash, long dash heavy, wavy double |
| FontScriptType | 45 script slots (Latin, Greek, Cyrillic, Hebrew, Arabic, Devanagari, Thai, Hangul, Han, Kana and many more) |
| NumberStylesType | Default, proportional lining, tabular lining, proportional oldstyle, tabular oldstyle |
| LigaturePresetType | Standard, standard plus optional, standard plus historical, all, none |
| TextUnit, CollapseDirection | Character, word, sentence, paragraph, line, story, screen, section, column, row, window, cell, table and so on; collapse to start or end |
| ParagraphAlignmentType | Left, centre, right, justified, inter-word, distribute, distribute East Asian, inter-ideograph, inter-cluster, distribute all, distribute centre last, kashida |
| LineSpacingRule | Single, 1.5 lines, double, exactly, multiple |
| TabAlignmentType, TabLeaderType | Left, centre, right, decimal; none, dots, dashes, line, bullet |
| ListType, ListSeparator | About 47 numbering/bullet styles (bullet, Arabic, upper/lower Roman, upper/lower letters, ordinal, cardinal text, ordinal text, leading-zero, circled, and many regional systems); separators period, parenthesis, double parenthesis, plain, square, colon, double square, hyphen, wide comma |
| DateTimeFormat | 23 date/time formats (short, long, ISO, month and year, 12 and 24 hour times with or without seconds, East Asian eras) |
| CalendarType | Western, Arabic Hijri, Hebrew lunar, Chinese national, Japanese emperor, Thai Buddhist, Korean Danki, Saka era, transliterated English/French, Arabic Umalqura |
| FieldType | Page number (current/next/previous), date/time, mail merge, business-info field, phonetic guide, sample text, six hyperlink kinds |
| PhoneticGuideAlignmentType | Default, 0-1-0, 1-2-1, centre, left, right |
| TextAutoFitType | None, shrink on overflow, best fit, grow to fit |
| VerticalTextAlignmentType | Top, centre, bottom |
| TextOrientation | Horizontal, vertical East Asian, right to left, mixed |
| CellDiagonalType | None, up, down |
| TableAutoFormatType | 24 named table formats |
| StoryType | Table, text frame, continued from, continued on |
| HlinkTargetType | None, URL, email, first/last/next/previous page, page ID, personalised |
| ShapeType | AutoShape, callout, chart, comment, freeform, group, embedded/linked OLE, form control, line, linked picture, OLE control, picture, placeholder, text effect, media, text frame, table, web check box, command button, list box, multi-line and single-line text, option button, web component, HTML fragment, group wizard, hot spot, catalog merge area, navigation bar, barcode picture holder |
| WrapType, WrapSideType | None, square, tight, through, top and bottom; both, left, right, larger side, neither |
| InlineAlignment | Character, left, right |
| WizardTag | About 60 roles for template parts (logo, organisation name, address, tag line, phone/fax/email, story, masthead, table of contents, date, picture caption and so on) |
| Wizard | About 62 publication types: business cards, letterheads, envelopes, brochures, newsletters, postcards, advertisements, calendars, flyers, signs, resumes, labels, business forms, banners, origami, airplanes, menus, certificates, gift certificates, compliments cards, programs, catalogs, quick publications, Word document, web sites, greeting cards, invitations, email newsletters/letters/events/products, and Japanese-market equivalents |
| WizardGroup | About 51 design gallery part families (logos, borders, attention getters, mastheads, sidebars, pull quotes, tables of contents, coupons, reply forms, phone tear-offs, accent boxes, linear accents, navigation bars, web buttons, calendars, marquees and more) |
| WizardPageType | About 57 starter page layouts for newsletters, catalogs and web sites |
| WizardNavBarDesign, ButtonStyle, Alignment, NavBarOrientation | 25 navigation bar designs; small, large, text buttons; left, centre, right; horizontal, vertical |
| PublicationLayout | About 60 publication layouts: full page, book, fold card, greeting cards, index card, business cards (four regions), posters, banners, postcards, label, envelope, web page, advertisement, certificate, brochure, calendar, catalog, email, flyer, gift certificate, invitation, letterhead, menu, newsletter, paper folding project, program, resume, sign, compliments card, web site, quick publication, and East Asian paper standards |
| PublicationType | Print, web |
| PageType, MasterPageType | Left, right, scratch, master; left, right |
| PageNumberType, PageNumberFormat | Current, next in story, previous in story; about 44 numbering formats |
| Zoom, WindowState | Page width, whole page, fit selection; maximise, minimise, normal |
| RulerGuideType | Vertical, horizontal |
| CanvasArrangementType, CatalogMergeFieldType | One canvas, columns, rows; text, picture |
| MergeType, MailMergeDestination | Mail merge, catalog merge, email merge; printer, new publication, existing publication, email |
| RecipientListFileType, EmailMergePriority | .mdb, .csv; none, high, low |
| MappedDataFields | 28 standard recipient columns (name parts, address parts, phone/fax, email, web, spouse, ruby) |
| FilterComparison, FilterConjunction | Eight comparisons; and, or |
| PersonalInfoSet | Primary business, secondary business, other organisation, home |
| ColorScheme | About 68 named built-in schemes plus custom and first-user-defined |
| FileFormat | Publication, Publisher 98, Publisher 2000, publication HTML, web archive, RTF, filtered HTML, plain text, Unicode text |
| FixedFormatType, FixedFormatIntent | XPS, PDF; minimum size, standard, printing, commercial printing |
| PrintStyle | Default, one page per sheet, tiled, multiple copies per sheet, multiple pages per sheet, booklet side fold, booklet top fold, half fold side/top, quarter fold side/top, envelope |
| PrintMode, DriverType, InksToPrint, PrintGraphics | Composite RGB/CMYK/greyscale, separations; non-PostScript, PostScript 1 to 3, XPS; all, used, convert spot to process; high, low, none |
| SaveOptions | Prompt, save, do not save |
| BuildingBlockType, BuildingBlockGallery | None, built-in, downloaded, user, work group; advertisements, accents, calendars, business info, page parts |
| FontType, FontSource, FontLicenseLimitations | TrueType, printer, raster; system, document, missing; embeddable, print/preview embeddable, not embeddable |
| Submit* and CommandButtonType, WebControlType | Web form submit choices (see Web publishing) |
| ShowDialog, HelpType, OriginalFormat | Default, show, suppress; help kinds; publication or Publisher file |
| AutoShapeType (Office) | About 180 shapes: rectangles, rounded and snipped rectangles, ovals, triangles, polygons, stars and banners, arrows, flowchart symbols, callouts, action buttons, brackets, braces, math signs, hearts, lightning, sun, moon, clouds and more |
| FillType, GradientStyle, GradientColorType, PresetGradientType, PatternType, PresetTexture, TextureType, TextureAlignment (Office) | Solid, patterned, gradient, textured, picture, background; horizontal, vertical, diagonal up/down, from corner, from title, from centre; one colour, two colours, preset; about 24 preset gradients; about 48 patterns; 24 textures |
| LineStyle, LineDashStyle, LineCapStyle, LineJoinStyle, ArrowheadStyle/Length/Width (Office) | Single, thin-thin, thin-thick, thick-thin, thick-between-thin; solid, square dot, round dot, dash, dash-dot, dash-dot-dot, long dash, long dash-dot, long dash-dot-dot; flat, round, square; round, bevel, mitre; none, triangle, open, stealth, diamond, oval; short, medium, long; narrow, medium, wide |
| ShadowType, ReflectionType, SoftEdgeType, BevelType, PresetMaterial, PresetLighting*, PresetExtrusionDirection, PresetThreeDFormat (Office) | Preset shadows (about 20), preset reflections, preset soft edge sizes, 12 bevels, materials, lighting, extrusion directions, 3-D presets |
| FlipCmd, ZOrderCmd, AlignCmd, DistributeCmd, ScaleFrom (Office) | Horizontal/vertical; bring to front, send to back, forward, backward; left, centre, right, top, middle, bottom; horizontal/vertical distribute; scale from top-left, middle, bottom-right |
| ConnectorType, CalloutType, CalloutAngleType, CalloutDropType, EditingType, SegmentType (Office) | Straight, elbow, curve; four callout styles; automatic, 30/45/60/90 degrees; top, centre, bottom, custom; auto, corner, smooth, symmetric; line, curve |
| BlackWhiteMode, PictureColorType, TriState, RGBType, LanguageID, Encoding, FileDialogType, AutomationSecurity (Office) | Print black-and-white options; automatic, greyscale, black and white, watermark; true, false, mixed, toggle; colour values; proofing language; web encodings; open/save/folder pickers; macro security levels |

## 4. Flat feature list

One row per distinct user-visible feature. `[ONLINE]` marks features that depend on a Microsoft service or on an
external online service in Publisher 2021 (to be replaced by open equivalents, ROADMAP Phase 2). Evidence is the
object model members that show the feature exists.

### FILE: publication files, export, print

| ID | Feature | Evidence |
|---|---|---|
| FILE-NEW | Create a new publication, blank or from a template | Application.NewDocument, Documents.Add |
| FILE-OPEN | Open a publication (several versions) | Application.Open, ChangeFileOpenDirectory, ReadOnly |
| FILE-SAVE | Save and Save As in the publication format | Document.Save, SaveAs, Saved, FullName, Path |
| FILE-SAVE-OLD | Save as Publisher 98 / 2000 formats | FileFormat Publisher98, Publisher2000, SaveFormat |
| FILE-SAVE-WEB | Save as web page, filtered HTML, web archive | FileFormat PublicationHTML, HTMLFiltered, WebArchive, WebOptions |
| FILE-SAVE-TEXT | Save as RTF, plain text or Unicode text | FileFormat RTF, PlainText, UnicodeText |
| FILE-AUTORECOVER | Auto-recover information and interval | Options.SaveAutoRecoverInfo, SaveAutoRecoverInfoInterval |
| FILE-BGSAVE | Background saving | Options.AllowBackgroundSave |
| FILE-CLOSE | Close with prompt, save or discard | Document.Close, SaveOptions |
| FILE-EXPORT-PDF | Export PDF with intent (minimum, standard, print, commercial) and page range | Document.ExportAsFixedFormat, FixedFormatType PDF, FixedFormatIntent |
| FILE-EXPORT-XPS | Export XPS | ExportAsFixedFormat, FixedFormatType XPS |
| FILE-EXPORT-PIC-PAGE | Save a page as a picture | Page.SaveAsPicture |
| FILE-EXPORT-PIC-SHAPE | Save a selected object as a picture | Shape.SaveAsPicture, ShapeRange.SaveAsPicture |
| FILE-EXPORT-HTMLMAIL | Export a page as email HTML | Page.ExportEmailHTML, WebOptions.EmailAsImg |
| FILE-WEB-PREVIEW | Preview a web publication in a browser | Document.WebPagePreview |
| FILE-CONVERT-TYPE | Convert print publication to web or email and back | Document.ConvertPublicationType, PublicationType |
| FILE-WORD-IMPORT | Create a publication from a Word document | Wizard WordDocument, PublicationLayout WordImport |
| FILE-INFO-PERSONAL | Remove personal information on save | Document.RemovePersonalInformation, Options.UpdatePersonalInfoOnSave |
| FILE-READONLY | Read-only handling | Document.ReadOnly |
| FILE-PATHS | Default folders for pictures and publications | Options.PathForPictures, PathForPublications |
| FILE-RECENT-PREVIEW | Template and file preview in the start screen | Options.UseCatalogAtStartup, ShowTipPages |

### PRT: printing and commercial output

| ID | Feature | Evidence |
|---|---|---|
| PRT-PRINT | Print with copies, page range and collation | Document.PrintOut, PrintOutEx |
| PRT-PRINTER-SELECT | Choose a printer and see its capabilities | Document.ActivePrinter, InstalledPrinters, Printer.* |
| PRT-DUPLEX | Duplex capability and back-side options | Printer.IsDuplex, AdvancedPrintOptions.BackSideInsertFaceUp |
| PRT-PAPER | Paper size, source, orientation, printable area | Printer.PaperSize, PaperSource, PaperOrientation, PrintableRect |
| PRT-COLOR-MODE | Print composite RGB, CMYK, greyscale or separations | Printer.PrintMode, PrintMode enum |
| PRT-STYLE-ONE | One page per sheet | PrintStyle OnePagePerSheet |
| PRT-STYLE-TILE | Tile large pages across sheets | PrintStyle Tiled |
| PRT-STYLE-COPIES | Multiple copies per sheet | PrintStyle MultipleCopiesPerSheet |
| PRT-STYLE-MULTI-PAGES | Multiple pages per sheet | PrintStyle MultiplePagesPerSheet, PageSetup.MultiplePagesPerSheet |
| PRT-BOOKLET | Booklet printing (side fold, top fold) | PrintStyle BookletSideFold, BookletTopFold |
| PRT-FOLD | Half-fold and quarter-fold printing | PrintStyle HalfFoldSide/Top, QuarterFoldSide/Top |
| PRT-ENVELOPE | Envelope printing options | PrintStyle Envelope, Options.EnvelopePrint*, UseEnvelopePaperSizes |
| PRT-PREVIEW | Print preview | Application.PrintPreview |
| PRT-BACKGROUNDS | Choose whether to print page backgrounds | Document.PrintPageBackgrounds |
| PRT-LINE-BY-LINE | Print line by line (troubleshooting) | Options.PrintLineByLine |
| PRT-TROUBLESHOOT | Print troubleshooter prompt | Options.DisplayPrintTroubleshooter |
| PRT-BW-MODE | Per-object black-and-white printing mode | Shape.BlackWhiteMode |
| PRT-MARKS-CROP | Crop marks | AdvancedPrintOptions.PrintCropMarks |
| PRT-MARKS-REG | Registration marks | PrintRegistrationMarks |
| PRT-MARKS-BLEED | Bleed marks and allow bleeds | PrintBleedMarks, AllowBleeds |
| PRT-MARKS-INFO | Job information, density bars, colour bars | PrintJobInformation, PrintDensityBars, PrintColorBars |
| PRT-FLIP | Print mirrored (horizontal or vertical) | AdvancedPrintOptions.HorizontalFlip, VerticalFlip |
| PRT-NEGATIVE | Print as negative | NegativeImage |
| PRT-FONTS-PUB-ONLY | Use only publication fonts when printing | UseOnlyPublicationFonts |
| PRT-GFX-RES | Graphics resolution: full, low, none | GraphicsResolution, PrintGraphics enum |
| PRT-RESOLUTION | Output resolution | AdvancedPrintOptions.Resolution |
| PRT-HALFTONE | Custom halftone | UseCustomHalftone |
| PRT-PS | PostScript printer options and detection | IsPostscriptPrinter, DriverType |
| PRT-MANUAL-FEED | Manual feed alignment and direction, page rotation | ManualFeedAlign, ManualFeedDirection, PageRotated |
| PRT-INKS-SELECT | Choose inks to print; convert spot to process | InksToPrint, PrintablePlates, PrintPlate |
| PRT-BLANK-PLATES | Print blank plates | PrintBlankPlates |
| PRT-CMYK-DEFAULT | Print CMYK by default | PrintCMYKByDefault |
| PRT-PLATE-SCREEN | Per-plate screen angle and frequency | Plate.Angle, Frequency, PrintablePlate.Angle |
| PRT-FONT-LICENSE | Font embedding permission check and font source report | FontLicenseLimitations, FontSource, FontType |

### APP: application, windows, views, options

| ID | Feature | Evidence |
|---|---|---|
| APP-WINDOW | Multiple publication windows; move, resize, state | Window.*, Documents |
| APP-ZOOM | Zoom presets, in, out | View.Zoom, ZoomIn, ZoomOut, Zoom enum |
| APP-VIEW-GUIDES | Show or hide boundaries and guides | Document.ViewBoundaries, ViewGuides, ViewBoundariesAndGuides |
| APP-VIEW-TWOPAGE | Two-page spread view | Document.ViewTwoPageSpread, ReaderSpread |
| APP-VIEW-BASELINE | Show baseline guides | ViewHorizontalBaseLineGuides, ViewVerticalBaseLineGuides |
| APP-SCROLL-TO | Scroll an object into view | View.ScrollShapeIntoView |
| APP-SNAP | Snap to guides and to objects | Application.SnapToGuides, SnapToObjects |
| APP-UNITS | Measurement units (inch, cm, mm, pica, point and more) | Options.MeasurementUnit, UnitType, *ToPoints methods |
| APP-SELECT | Selection of shapes, text and cells | Selection.Type, ShapeRange, TextRange, TableCellRange, ChildShapeRange |
| APP-SELECT-WORD | Auto-select entire word | Options.AutoSelectWord |
| APP-DRAGDROP | Drag-and-drop text editing | Options.DragAndDropText |
| APP-UNDO | Undo and redo with availability queries and grouped actions | Document.Undo, Redo, UndoClear, BeginCustomUndoAction |
| APP-HYPHEN | Automatic hyphenation with zone | Options.AutoHyphenate, HyphenationZone |
| APP-AUTOFORMAT | Auto-format words, typing replacements, sequence checking | Options.AutoFormatWord, TypeNReplace, SequenceCheck |
| APP-STATUSBAR | Show or hide status bar | Options.ShowStatusBar |
| APP-TIPS | Screen tips on objects, helpful mouse pointers, show tip pages | Options.ShowScreenTipsOnObjects, UseHelpfulMousePointers, ShowTipPages, ResetTips |
| APP-BASIC-COLORS | Show basic colours in colour pickers | Options.ShowBasicColors |
| APP-FILEDIALOG | Standard open/save/folder pickers | Application.FileDialog |
| APP-CMDBARS | Customisable command bars and add-in toolbars | Application.CommandBars |
| APP-ADDINS | COM add-in loading | Application.COMAddIns |
| APP-SCRIPT-EVENTS | Document and application events for automation | ApplicationEvents, DocumentEvents |
| APP-SECURITY | Macro security level | Application.AutomationSecurity |
| APP-FILESEARCH | File search service | Application.FileSearch |
| APP-HELP | Help pane / Office Assistant | Application.Help, Assistant, Assistance, HelpType |
| APP-ONLINE-WEBSERVICE | Launch Microsoft web service pages [ONLINE] | Application.LaunchWebService |
| APP-DIRECTION-DEFAULT | Default publication direction and text flow (right-to-left support) | Options.DefaultPubDirection, DefaultTextFlowDirection, AddHebDoubleQuote, AutoKeyboardSwitching |

### TPL: templates, wizards, building blocks, business information

| ID | Feature | Evidence |
|---|---|---|
| TPL-CATALOG | Template catalog on start and on demand | Application.ShowWizardCatalog, WizardCatalogVisible, UseCatalogAtStartup |
| TPL-ONLINE | Online template and clip content window [ONLINE] | ModalBrowser, ICagNotifySink, ShowCatalogUI event |
| TPL-WIZARD-TYPES | Publication types (business cards, newsletters, brochures and so on) | Wizard enum |
| TPL-WIZARD-OPTIONS | Per-template design options pane (layout, colour, font, business info) | Wizard.Properties, WizardProperty, WizardValue, SetId |
| TPL-WIZARD-PAGE | Add a starter page layout to newsletters, catalogs, web sites | Pages.AddWizardPage, WizardPageType |
| TPL-WIZARD-GROUP | Design Gallery parts (borders, mastheads, sidebars, calendars, coupons and so on) | Shapes.AddGroupWizard, WizardGroup enum |
| TPL-WIZARD-TAGS | Template roles of objects, used to refill business information | Shape.WizardTag, WizardTagInstance, FindShapeByWizardTag, WizardTag enum |
| TPL-CHANGE-TEMPLATE | Change template without losing content | Document.ChangeDocument |
| TPL-ISWIZARD | Know whether a publication is template-based | Document.IsWizard, Page.IsWizardPage |
| TPL-WIZARD-AFTER | React when a template option changes | DocumentEvents.WizardAfterChange |
| TPL-FOLDER | Template folder location | Application.TemplateFolderPath |
| BB-GALLERY | Building block galleries (advertisements, accents, calendars, business info, page parts) | BuildingBlockGallery, Document.AvailableBuildingBlocks |
| BB-INSERT | Insert a building block onto a page | Shapes.AddBuildingBlock |
| BB-SAVE | Save selected objects as a building block with name, category, keywords | Shape.SaveAsBuildingBlock, BuildingBlock.* |
| BB-DELETE | Delete a user building block | BuildingBlock.Delete |
| BB-SHOW | Show or hide a block in its gallery | BuildingBlock.ShowInGallery |
| BB-DOWNLOADED | Downloaded building blocks [ONLINE] | BuildingBlockType Downloaded |
| BB-WORKGROUP | Shared workgroup building blocks | BuildingBlockType WorkGroup |
| BIZINFO-SETS | Four business-information sets (primary, secondary, other, home) | PersonalInfoSet, Document.PersonalInformationSet |
| BIZINFO-EDIT | Edit business information and apply to publication | Document.SetBusinessInformation |
| BIZINFO-FIELDS | Insert business-information fields into text | FieldType IHIV |
| BIZINFO-UPDATE | Update business info on save | Options.UpdatePersonalInfoOnSave |

### PG: pages, master pages, sections, layout

| ID | Feature | Evidence |
|---|---|---|
| PG-ADD | Insert blank, duplicate or starter pages before or after | Pages.Add, Add10 |
| PG-DELETE | Delete a page | Page.Delete |
| PG-DUPLICATE | Duplicate a page | Page.Duplicate |
| PG-MOVE | Move a page | Page.Move |
| PG-NAVIGATE | Go to page, active page | View.ActivePage, Pages.Item, FindByPageID |
| PG-ID | Stable page identifier | Page.PageID, PageIndex, PageNumber |
| PG-NAME | Page name | Page.Name |
| PG-SPREAD | Facing pages and reader spreads | Page.ReaderSpread, IsLeading, IsTrailing, X/YOffsetWithinReaderSpread |
| PG-SCRATCH | Scratch area beside pages | ScratchArea, PageType Scratch |
| PG-MASTER-ADD | Create one-page or two-page master pages | MasterPages.Add, IsTwoPageMaster |
| PG-MASTER-APPLY | Apply a master to a page | Page.Master |
| PG-MASTER-IGNORE | Hide master objects on one page | Page.IgnoreMaster |
| PG-MASTER-EDIT | Edit master shapes (page numbers, headers) | MasterPages.Item, Shapes |
| PG-HEADER-FOOTER | Header and footer areas | Page.Header, Footer, HeaderFooter |
| PG-BACKGROUND | Page background fill or picture | PageBackground.Fill, Create, Delete |
| PG-SECTIONS | Section breaks with independent numbering | Sections.Add, Section.StartPageIndex, Delete |
| PG-NUM-START | Start number and continue from previous | Section.PageNumberStart, ContinueNumbersFromPreviousSection |
| PG-NUM-FORMAT | Number format (Arabic, Roman, letters, ordinals, regional) | Section.PageNumberFormat, PageNumberFormat enum |
| PG-NUM-FIRSTPAGE | Show header and footer on first page | Section.ShowHeaderFooterOnFirstPage |
| PG-SIZE | Page size presets and custom size | PageSetup.PageSize, PageWidth, PageHeight, PageSizes |
| PG-ORIENT | Portrait or landscape | PageSetup.Orientation |
| PG-LAYOUT-TYPE | Publication layout (card, poster, banner, booklet, book and so on) | PageSetup.PublicationLayout, PublicationLayout enum |
| PG-MARGINS | Margin guides | LayoutGuides.MarginLeft/Right/Top/Bottom |
| PG-MIRROR | Mirror guides for facing pages | LayoutGuides.MirrorGuides |
| PG-GRID | Column and row guides with gutters | LayoutGuides.Columns, Rows, ColumnGutterWidth, RowGutterWidth, GutterCenterlines |
| PG-BASELINE | Baseline grid | HorizontalBaseLineOffset/Spacing, VerticalBaseLineOffset/Spacing |
| PG-RULER-GUIDES | Draggable ruler guides | RulerGuides.Add, RulerGuide.Position, Type, Delete |
| PG-LABEL-SHEETS | Label sheet definitions (size, margins, gaps) | PageSetup.Label, AvailableLabels, Labels |
| PG-SHEET-LAYOUT | Multiple pages per sheet with gaps and margins | PageSetup.HorizontalGap, VerticalGap, LeftMargin, TopMargin |
| PG-SIZE-BACKGROUND | Page sizes that carry a background image | PageSize.HasBackgroundImage |
| PG-BLEED | Bleed area (allow bleeds) | AdvancedPrintOptions.AllowBleeds |
| PG-REPEAT-NEWSLETTER | Continued-on and continued-from notes | TextFrame.IncludeContinuedOnPage, IncludeContinuedFromPage |

### SHP: shapes and objects

| ID | Feature | Evidence |
|---|---|---|
| SHP-ADD-TEXTBOX | Draw a text box | Shapes.AddTextbox |
| SHP-ADD-LABEL | Add a label (auto-sized text) | Shapes.AddLabel |
| SHP-ADD-LINE | Draw a line | Shapes.AddLine |
| SHP-ADD-AUTOSHAPE | Draw any AutoShape | Shapes.AddShape, AutoShapeType |
| SHP-AUTOSHAPE-ADJUST | Drag adjust handles on AutoShapes | Shape.Adjustments |
| SHP-ADD-CALLOUT | Callout shapes with leader line | Shapes.AddCallout, CalloutFormat |
| SHP-ADD-CONNECTOR | Connectors that follow shapes | Shapes.AddConnector, ConnectorFormat, RerouteConnections |
| SHP-ADD-CURVE | Draw a curve | Shapes.AddCurve |
| SHP-ADD-POLYLINE | Draw a polyline or polygon | Shapes.AddPolyline |
| SHP-FREEFORM | Build a freeform shape point by point | Shapes.BuildFreeform, FreeformBuilder |
| SHP-EDIT-POINTS | Edit points: add, delete, move, change segment or node type | ShapeNodes.*, ShapeNode.EditingType, SegmentType |
| SHP-ADD-OLE | Insert an OLE object (embedded or linked) | Shapes.AddOLEObject, OLEFormat |
| SHP-OLE-VERBS | Edit or open an OLE object | OLEFormat.Activate, DoVerb, ObjectVerbs |
| SHP-OLE-LINK-UPDATE | Update linked objects | LinkFormat.Update, Document.UpdateOLEObjects |
| SHP-MOVE-SIZE | Position and size, with lock of aspect ratio | Left, Top, Width, Height, LockAspectRatio, IncrementLeft/Top |
| SHP-SCALE | Scale height or width relative to original or current | ScaleHeight, ScaleWidth, ScaleFrom |
| SHP-ROTATE | Rotation to angle or by increments | Rotation, IncrementRotation |
| SHP-FLIP | Flip horizontal or vertical | Flip, HorizontalFlip, VerticalFlip |
| SHP-ZORDER | Bring forward, send backward, to front, to back | ZOrder, ZOrderPosition |
| SHP-ALIGN | Align to edges and centres, relative to page or selection | ShapeRange.Align |
| SHP-DISTRIBUTE | Distribute horizontally or vertically | ShapeRange.Distribute |
| SHP-GROUP | Group, ungroup, regroup | Group, Ungroup, Regroup, GroupItems |
| SHP-GROUP-MEMBER | Select inside a group | ChildShapeRange, IsGroupMember, ParentGroupShape |
| SHP-CUT-COPY-PASTE | Cut, copy, paste and delete objects | Cut, Copy, Shapes.Paste, Delete |
| SHP-DUPLICATE | Duplicate with offset | Shape.Duplicate |
| SHP-FORMAT-PAINTER | Copy formatting (Pick Up, Apply) | PickUp, Apply |
| SHP-DEFAULTS | Set object defaults for new shapes | SetShapesDefaultProperties |
| SHP-SELECT-ALL | Select all objects on a page | Shapes.SelectAll |
| SHP-MOVE-TO-PAGE | Move object to another page | Shape.MoveToPage |
| SHP-NAME | Name an object | Shape.Name, ID |
| SHP-ALT-TEXT | Alternative text | Shape.AlternativeText |
| SHP-TAG | Custom tags on objects | Tags.Add, Tag.Name, Value |
| SHP-INLINE | Anchor object inline in text (character, left, right) | IsInline, InlineAlignment, MoveIntoTextFlow, MoveOutOfTextFlow |
| SHP-LOCK | Lock aspect ratio | LockAspectRatio |
| SHP-OVERFLOW-LIST | Find objects excess to the page or text overflow | Document.SurplusShapes, Shape.IsExcess |
| SHP-CANVAS | Canvas arrangement for merged catalog items | Shapes.CanvasArrangementType, CanvasesCount |

### FMT: fill, outline and effects

| ID | Feature | Evidence |
|---|---|---|
| FMT-FILL-SOLID | Solid fill with scheme, RGB, CMYK or spot colour | FillFormat.Solid, ForeColor |
| FMT-FILL-TRANSPARENCY | Fill transparency | FillFormat.Transparency |
| FMT-FILL-GRADIENT | One- and two-colour gradients with style, variant and angle | OneColorGradient, TwoColorGradient, GradientStyle/Variant/Angle |
| FMT-FILL-GRADIENT-PRESET | Preset gradients | PresetGradient, PresetGradientType |
| FMT-FILL-PATTERN | Pattern fills | Patterned, Pattern |
| FMT-FILL-TEXTURE | Preset textures | PresetTextured, TextureType |
| FMT-FILL-TEXTURE-USER | Picture textures (tile with offset, scale, alignment) | UserTextured, TextureOffsetX/Y, TextureHorizontalScale, TextureAlignment |
| FMT-FILL-PICTURE | Picture fill | UserPicture |
| FMT-FILL-ROTATE | Rotate fill with object | RotateWithObject |
| FMT-FILL-NONE | No fill | FillFormat.Visible |
| FMT-LINE-COLOR | Outline colour and transparency | LineFormat.ForeColor, Transparency |
| FMT-LINE-WEIGHT | Outline weight | LineFormat.Weight |
| FMT-LINE-DASH | Dash styles | DashStyle |
| FMT-LINE-COMPOUND | Compound line styles (double, thick-thin) | LineFormat.Style |
| FMT-LINE-PATTERN | Patterned line | LineFormat.Pattern, BackColor |
| FMT-LINE-ARROW | Arrowheads: style, length, width at both ends | Begin/EndArrowheadStyle/Length/Width |
| FMT-LINE-CAP-JOIN | Cap and join style | CapStyle, JoinStyle |
| FMT-LINE-INSET | Inset pen (outline inside) | InsetPen |
| FMT-LINE-GRADIENT | Gradient outlines | LineFormat gradient members |
| FMT-BORDERART | BorderArt picture borders with weight, colour, stretch | BorderArtFormat, BorderArts |
| FMT-SHADOW | Shadows: presets, colour, offset, blur, size, transparency | ShadowFormat.* |
| FMT-GLOW | Glow | GlowFormat.* |
| FMT-REFLECTION | Reflection | ReflectionFormat.* |
| FMT-SOFTEDGE | Soft edges | SoftEdgeFormat.* |
| FMT-3D-ROTATE | 3-D rotation X/Y, perspective, field of view | ThreeDFormat.RotationX/Y, Perspective, FieldOfView |
| FMT-3D-EXTRUDE | Extrusion depth, colour, direction | Depth, ExtrusionColor, SetExtrusionDirection |
| FMT-3D-BEVEL | Bevel top and bottom, contour | BevelTop*, BevelBottom*, ContourWidth, ContourColor |
| FMT-3D-MATERIAL-LIGHT | Material and lighting presets | PresetMaterial, PresetLighting* |
| FMT-3D-PRESET | 3-D presets | PresetThreeDFormat |
| FMT-WRAP-TYPE | Text wrap type | WrapFormat.Type |
| FMT-WRAP-SIDE | Wrap sides | WrapFormat.Side |
| FMT-WRAP-DISTANCE | Wrap distance, automatic or per side | DistanceAuto, DistanceTop/Left/Bottom/Right |
| FMT-WRAP-EDIT | Edit wrap points | Shape.Vertices |
| FMT-CALLOUT-FORMAT | Callout angle, drop, length, border, accent bar | CalloutFormat.* |
| FMT-CONNECTOR-TYPE | Straight, elbow, curved connectors | ConnectorFormat.Type |

### WA: WordArt and text effects

| ID | Feature | Evidence |
|---|---|---|
| WA-INSERT | Insert WordArt with a preset style | Shapes.AddWordArt, AddTextEffect, PresetWordArt |
| WA-SHAPE | WordArt warp shape | TextEffect.PresetShape, PresetTextEffect |
| WA-TEXT-FORMAT | WordArt font, size, bold, italic, alignment | TextEffect.FontName, FontSize, FontBold, FontItalic, Alignment |
| WA-SPACING | Tracking, kerned pairs, equalise letter height | TextEffect.Tracking, KernedPairs, NormalizedHeight |
| WA-ROTATE-CHARS | Rotate characters 90 degrees | TextEffect.RotatedChars |
| WA-VERTICAL | Vertical WordArt | TextEffect.ToggleVerticalText |

### TXT: text, character and paragraph formatting

| ID | Feature | Evidence |
|---|---|---|
| TXT-EDIT | Type, insert, delete, cut, copy, paste text | TextRange.Text, InsertAfter, InsertBefore, Delete, Paste |
| TXT-SELECT-UNITS | Select by character, word, line, sentence, paragraph, story | TextRange.Characters, Words, Lines, Paragraphs, TextUnit |
| TXT-COUNT | Word, line, paragraph counts (word count) | WordsCount, LinesCount, ParagraphsCount |
| TXT-SYMBOL | Insert symbols and special characters | TextRange.InsertSymbol |
| TXT-DATETIME | Insert date and time (updating or fixed) | InsertDateTime, DateTimeFormat, CalendarType |
| TXT-PAGENUM-FIELD | Insert page number fields (current, next, previous) | InsertPageNumber, FieldType PageNumber* |
| TXT-FIELDS | Unlink fields to plain text | Fields.Unlink, Field.Unlink |
| TXT-FONT-FACE | Font name | Font.Name |
| TXT-FONT-SIZE | Font size, grow and shrink | Font.Size, Grow, Shrink |
| TXT-BOLD-ITALIC | Bold and italic | Font.Bold, Italic |
| TXT-UNDERLINE | Underline with 18 styles | Font.Underline |
| TXT-STRIKE | Strikethrough | Font.StrikeThrough |
| TXT-SCRIPT | Superscript and subscript | Font.SuperScript, SubScript, Position |
| TXT-CASE | All caps and small caps | Font.AllCaps, SmallCaps |
| TXT-EFFECTS-LEGACY | Outline, shadow, emboss, engrave | Font.Outline, Shadow, Emboss, Engrave |
| TXT-COLOR | Text colour (scheme, RGB, CMYK, spot) | Font.Color |
| TXT-TRACKING | Character spacing presets and custom | Font.Tracking, TrackingPreset |
| TXT-SCALE | Character width scaling | Font.Scaling |
| TXT-KERN | Pair kerning with minimum size | Font.Kerning, AutomaticPairKerningThreshold |
| TXT-OT-LIGATURE | OpenType ligatures | Font.Ligature |
| TXT-OT-NUMSTYLE | OpenType number styles | Font.NumberStyle |
| TXT-OT-STYLISTIC | Stylistic sets and alternates, swashes, contextual alternates | StylisticSets, StylisticAlternates, Swash, ContextualAlternates |
| TXT-FILL-TEXT | Text fill and outline (gradient and picture fills) | Font.Fill, Font.Line |
| TXT-EFFECTS-MODERN | Text shadow, glow, reflection, 3-D on text | Font.TextShadow, Glow, Reflection, ThreeD |
| TXT-CLEAR-CHAR | Clear character formatting | Font.Reset |
| TXT-COMPLEX-SCRIPT | Complex-script bold, italic, size, kashida, diacritic colour | BoldBi, ItalicBi, SizeBi, ExpandUsingKashida, DiacriticColor |
| TXT-SCRIPT-FONTS | Per-script font assignment | Font.GetScriptName, SetScriptName, FontScriptType |
| TXT-LANGUAGE | Proofing language of text | TextRange.LanguageID |
| TXT-ALIGN | Left, centre, right, justify, distribute | ParagraphFormat.Alignment |
| TXT-ALIGN-EASTASIAN | East Asian and Arabic alignment options | Alignment values, KashidaPercentage |
| TXT-INDENT | Left, right and first-line indents | LeftIndent, RightIndent, FirstLineIndent |
| TXT-INDENT-CHAR | Indents in characters | CharBasedFirstLineIndent, UseCharBasedFirstLineIndent |
| TXT-SPACING | Space before and after paragraphs | SpaceBefore, SpaceAfter |
| TXT-LINESPACING | Line spacing single, 1.5, double, exact, multiple | LineSpacing, LineSpacingRule, SetLineSpacing |
| TXT-TABS | Tab stops with alignment and leaders | TabStops.Add/Clear/ClearAll, TabStop.Position/Alignment/Leader |
| TXT-TAB-DEFAULT | Default tab stop distance | Document.DefaultTabStop |
| TXT-BULLET | Bullets with custom character and font | ListType Bullet, ListBulletText, ListBulletFontName, ListBulletFontSize |
| TXT-NUMBERING | Numbered lists in many numbering systems | ListType values, SetListType |
| TXT-NUM-START | Number start value | ListNumberStart |
| TXT-NUM-SEPARATOR | Separator style after number | ListNumberSeparator |
| TXT-LIST-INDENT | List indent | ListIndent |
| TXT-KEEP-TOGETHER | Keep lines together | KeepLinesTogether |
| TXT-KEEP-NEXT | Keep with next paragraph | KeepWithNext |
| TXT-WIDOW | Widow and orphan control | WidowControl |
| TXT-START-NEXT-BOX | Start paragraph in next text box | StartInNextTextBox |
| TXT-BASELINE-LOCK | Align paragraph to baseline grid | LockToBaseLine |
| TXT-DIRECTION | Paragraph direction left to right or right to left | ParagraphFormat.TextDirection |
| TXT-CLEAR-PARA | Clear paragraph formatting | ParagraphFormat.Reset |
| TXT-DROPCAP | Drop cap presets and custom (lines, size, font, colour) | DropCap.*, ApplyCustomDropCap, Clear |
| TXT-STYLE-APPLY | Apply a named style | ParagraphFormat.TextStyle |
| TXT-STYLE-CREATE | Create, edit, delete styles with base and next style | TextStyles.Add, TextStyle.* |
| TXT-STYLE-DESCR | Style description text | TextStyle.Description |
| TXT-FINDREPLACE | Find, find next, replace, replace all | FindReplace.*, Document.Find, TextRange.Find |
| TXT-FIND-OPTIONS | Match case, whole word, direction; East Asian match options | FindReplace.Match* |
| TXT-HYPHENATION | Hyphenation (auto) | Options.AutoHyphenate |
| TXT-PHONETIC | Phonetic guides over East Asian text | PhoneticGuide.*, Fields.AddPhoneticGuide |
| TXT-HORIZONTAL-IN-VERTICAL | Horizontal text inside vertical lines (tate-chu-yoko) | Fields.AddHorizontalInVertical |
| TXT-BARCODE | Insert a barcode | TextRange.InsertBarcode, Application.InsertBarcodeVisible |
| TXT-RANGE-BOUNDS | Geometry of a text range | BoundLeft/Top/Width/Height |
| TXT-MAJORITY | Formatting shown for mixed selections | MajorityFont, MajorityParagraphFormat |

### TB: text boxes and story flow

| ID | Feature | Evidence |
|---|---|---|
| TB-MARGINS | Internal margins | TextFrame.MarginLeft/Top/Right/Bottom |
| TB-COLUMNS | Columns with spacing | Columns, ColumnSpacing |
| TB-VALIGN | Vertical alignment | VerticalTextAlignment |
| TB-ORIENT | Text direction (horizontal, vertical, right to left) | Orientation |
| TB-AUTOFIT | Shrink on overflow, best fit, grow to fit | AutoFitText |
| TB-LINK | Create link to next text box | NextLinkedTextFrame, ValidLinkTarget |
| TB-UNLINK | Break forward link | BreakForwardLink |
| TB-OVERFLOW | Detect text overflow | Overflowing, Story |
| TB-CONTINUED | Continued-on and continued-from notes | IncludeContinuedOnPage/FromPage |
| TB-STORY | Whole-story operations across linked boxes | Story, Stories |
| TB-HAS-TEXT | Know if a frame has text | HasText |

### PIC: pictures

| ID | Feature | Evidence |
|---|---|---|
| PIC-INSERT | Insert picture from file, embedded or linked | Shapes.AddPicture, PictureInsertAs |
| PIC-EMPTY-FRAME | Empty picture frame | Shapes.AddEmptyPictureFrame, IsEmpty |
| PIC-REPLACE | Change picture, keep size and formatting | PictureFormat.Replace, ReplaceEx |
| PIC-REMOVE | Remove picture, keep frame | PictureFormat.Remove |
| PIC-CROP | Crop on all sides | CropTop/Bottom/Left/Right, ClearCrop |
| PIC-FIT | Fill frame or fit frame | FillFrame, FitFrame, PictureInsertFit |
| PIC-LOCK | Picture anchoring when frame resizes | Horizontal/VerticalPictureLocking |
| PIC-SCALE | Picture scale percent | HorizontalScale, VerticalScale |
| PIC-BRIGHTNESS | Brightness | Brightness, IncrementBrightness |
| PIC-CONTRAST | Contrast | Contrast, IncrementContrast |
| PIC-RECOLOR | Recolour with a colour; keep black | Recolor, RecoloredPictureColor, LeaveBlackAsBlack |
| PIC-COLORTYPE | Automatic, greyscale, black and white, washout | PictureFormat.ColorType |
| PIC-RESTORE | Reset picture colours | RestoreOriginalColors |
| PIC-TRANSPARENT | Set transparent colour | TransparencyColor, TransparentBackground |
| PIC-LINK-STATUS | Linked picture status and relink | IsLinked, LinkedFileStatus, LinkFormat.SourceFullName |
| PIC-INFO | Picture information: resolution, size, format, colour depth | EffectiveResolution, FileSize, ImageFormat, ColorModel, Original* |
| PIC-FORMATS | Support EMF, WMF, PICT, JPEG, PNG, DIB, GIF, TIFF, CMYK JPEG | ImageFormat enum |
| PIC-RESOLUTION-EXPORT | Resolution presets for output (96, 150, 300 dpi) | PictureResolution |
| PIC-CAPTION | Picture captions with styles | Shape.SetCaption, CaptionStyles |
| PIC-ALPHA | Alpha channel awareness | HasAlphaChannel |
| PIC-ONLINE | Insert online pictures [ONLINE] | ModalBrowser, ICagNotifySink, InsertClip event |

### TBL: tables

| ID | Feature | Evidence |
|---|---|---|
| TBL-INSERT | Insert table of rows and columns | Shapes.AddTable |
| TBL-ROW-ADD | Add rows | Rows.Add |
| TBL-COL-ADD | Add columns | Columns.Add |
| TBL-ROW-DELETE | Delete rows | Row.Delete |
| TBL-COL-DELETE | Delete columns | Column.Delete |
| TBL-ROW-HEIGHT | Row height | Row.Height, CellRange.Height |
| TBL-COL-WIDTH | Column width | Column.Width |
| TBL-MERGE | Merge cells | Cell.Merge, CellRange.Merge |
| TBL-SPLIT | Split cell | Cell.Split |
| TBL-SELECT | Select cells, rows, columns, table | Cell.Select, CellRange.Select, Selection.TableCellRange |
| TBL-BORDERS | Cell borders with weight and colour | Cell.BorderTop/Bottom/Left/Right, CellBorder |
| TBL-DIAGONAL | Diagonal borders | BorderDiagonal, Diagonal |
| TBL-FILL | Cell fill | Cell.Fill |
| TBL-MARGINS | Cell margins | Cell.MarginTop/Bottom/Left/Right |
| TBL-VALIGN | Cell vertical alignment | Cell.VerticalTextAlignment |
| TBL-TEXT-ORIENT | Cell text direction | Cell.CellTextOrientation |
| TBL-AUTOFORMAT | Table formats (24 named) | Table.ApplyAutoFormat, TableAutoFormatType |
| TBL-GROW | Grow table to fit text | Table.GrowToFitText |
| TBL-DIRECTION | Table direction left-to-right or right-to-left | Table.TableDirection |
| TBL-CELL-TEXT | Text, font and paragraph formatting inside cells | Cell.TextRange |

### LNK: hyperlinks and tags

| ID | Feature | Evidence |
|---|---|---|
| LNK-ADD | Add hyperlink to text or object | Hyperlinks.Add |
| LNK-URL | Web address link | Hyperlink.Address, HlinkTargetType URL |
| LNK-EMAIL | Email link with subject | Hyperlink.EmailSubject, TargetType Email |
| LNK-PAGE | Link to a page, first, last, next, previous | PageID, SetPageRelative, HlinkTargetType |
| LNK-PERSONALIZED | Personalised URL using merge data | HlinkTargetType Personalized, FieldType PersonalizedHyperlinkURL |
| LNK-EDIT-REMOVE | Edit, move, remove hyperlink | Hyperlink.Move, Delete, TextToDisplay |
| LNK-COLORS | Hyperlink and followed-hyperlink colours | SchemeColorHyperlink, SchemeColorFollowedHyperlink |
| LNK-FILE | Link to a file | FieldType HyperlinkFile |

### CLR: colour

| ID | Feature | Evidence |
|---|---|---|
| CLR-SCHEME-APPLY | Apply one of ~68 built-in colour schemes | Document.ColorScheme, ColorSchemes, ColorScheme enum |
| CLR-SCHEME-CUSTOM | Create and edit a custom colour scheme | ColorScheme.Colors, ColorSchemeCustom, FirstUserDefined |
| CLR-RGB | RGB colours | ColorFormat.RGB, BaseRGB |
| CLR-CMYK | CMYK colours | ColorFormat.CMYK, ColorCMYK.SetCMYK |
| CLR-SPOT | Spot colours (12 spot inks) | ColorFormat.Ink, InkName |
| CLR-TINT | Tints and shades | ColorFormat.TintAndShade |
| CLR-TRANSPARENCY | Colour transparency | ColorFormat.Transparency |
| CLR-SCHEME-SLOTS | Scheme-linked colours that follow the scheme | ColorFormat.SchemeColor, SchemeColorIndex |
| CLR-IN-USE | Colours in use list | ColorsInUse |
| CLR-REPLACE | Replace a colour throughout, with tint rules | ReplaceTint |
| CLR-MODE | Document colour mode: desktop, process, spot, black and white | Document.ColorMode, EnterColorMode |
| CLR-PLATES | Create and manage ink plates | Plates.Add, Plate.*, CreatePlateCollection, FindPlateByInkName |
| CLR-SPOT-TO-PROCESS | Convert spot to process | Plate.ConvertToProcess |
| CLR-COLOR-MANAGED | Colour-managed colour type | ColorType CMS |

### MM: mail merge, catalog merge, email merge

| ID | Feature | Evidence |
|---|---|---|
| MM-WIZARD | Mail merge step-by-step pane | MailMerge.ShowWizard, ShowWizardEx, WizardState |
| MM-TYPE | Choose letter/catalog/email merge | MailMerge.Type, MergeType |
| MM-SOURCE-OPEN | Connect to recipient list (Excel, Access, text, Outlook, other) | OpenDataSource, DataSource.ConnectString, TableName |
| MM-SOURCE-CREATE | Create a new recipient list | OpenRecipientsDialog, RecipientListFileType |
| MM-RECIPIENTS | Select and exclude recipients | DataSource.Included, SetAllIncludedFlags, OpenRecipientsDialog |
| MM-SORT | Sort recipients | SetSortOrder |
| MM-FILTER | Filter recipients by conditions | MailMergeFilters.Add/Delete, ApplyFilter, FilterComparison |
| MM-EDIT-RECORD | Edit a recipient | EditRecord, ActiveRecord, FindRecord |
| MM-FIELD-INSERT | Insert merge fields in text | TextRange.InsertMailMergeField, DataField.Insert |
| MM-FIELD-PICTURE | Insert picture merge fields | MailMergeDataFieldType Picture |
| MM-FIELD-MAP | Match fields to standard address fields | MappedDataFields, MapToRecipientField, UnMapRecipientField |
| MM-FIELD-ADD | Add a recipient column | AddToRecipientFields |
| MM-ADDRESS-BLOCK | Address block and greeting line insertion | MappedDataFields, FieldType MailMerge |
| MM-PREVIEW | Preview merged results | ViewMailMergeFieldCodes, ActiveRecord, FirstRecord/LastRecord |
| MM-SUPPRESS-BLANK | Suppress blank lines | MailMerge.SuppressBlankLines |
| MM-MERGE-PRINT | Merge to printer | Destination SendToPrinter, Execute |
| MM-MERGE-NEW-PUB | Merge to a new publication | Destination MergeToNewPublication |
| MM-MERGE-EXISTING | Merge to an existing publication | Destination MergeToExistingPublication |
| MM-MERGE-EMAIL | Merge to email with To, subject, priority | EmailMergeEnvelope, Destination SendEmail |
| MM-EMAIL-ATTACH | Attach files to merged email | Attachments.Add, ClearAll |
| MM-EMAIL-CCBCC | Cc and Bcc | EmailMergeEnvelope.Cc, Bcc |
| MM-EXPORT-LIST | Export recipient list to .mdb or .csv | ExportRecipientList |
| MM-SHORTCUT | Create shortcut to merge | MailMerge.CreateShortcut |
| MM-VALIDATE | Validate addresses; flag invalid with comments [ONLINE] | ValidateAddressVisible, InvalidAddress, InvalidComments, EverValidated, ValidatedClean, MailMergeDataSourceValidate event |
| MM-FOLLOWUP | Third-party send and follow-up hooks [ONLINE] | ShowSendToCustom, ShowFollowUpCustom, MailMergeWizardSendToCustom, MailMergeWizardFollowUpCustom |
| MM-BARCODE | Merge barcodes and postal codes | MailMergeInsertBarcode, MailMergeGenerateBarcode, BarCodePictureHolder |
| MM-EVENTS | Hooks before and after merge and records | MailMergeBefore/AfterMerge, BeforeRecordMerge |
| CAT-AREA | Catalog merge area on a page | Shapes.AddCatalogMergeArea, CatalogMergeShapes |
| CAT-FIELD | Insert catalog text and picture fields | AddCatalogMergeFieldToCanvas, CatalogMergeFieldType |
| CAT-REPEAT | Repeat counts across and down | HorizontalRepeat, VerticalRepeat |
| CAT-MEMBERS | Add and remove objects from the merge area | AddToCatalogMergeArea, RemoveFromCatalogMergeArea, CatalogMergeItems |
| CAT-PAGES | Catalog starter page layouts | WizardPageTypeCatalog* |

### WEB: web publishing

| ID | Feature | Evidence |
|---|---|---|
| WEB-PAGE-OPTIONS | Page title/description/keywords | WebPageOptions.Description, Keywords |
| WEB-FILENAME | Published file name per page | WebPageOptions.PublishFileName |
| WEB-SOUND | Background sound with loop | BackgroundSound, SetBackgroundSoundRepeat |
| WEB-NAVBAR | Navigation bars with 25 designs, orientation, button style | WebNavigationBarSets.AddSet, Design, ButtonStyle, ChangeOrientation |
| WEB-NAVBAR-LINKS | Navigation bar links and auto-update | WebNavigationBarHyperlinks.Add, AutoUpdate, ShowSelected |
| WEB-NAVBAR-EVERY | Add navigation bar to every page | AddToEveryPage, IncludePageOnNewWebNavigationBars |
| WEB-FORM-BUTTON | Submit and reset buttons | WebCommandButton.ButtonType |
| WEB-FORM-SUBMIT | Where form data goes: server, email, program, with format | DataRetrievalMethod, DataFileFormat, EmailAddress, ActionURL |
| WEB-FORM-HIDDEN | Hidden form fields | WebHiddenFields.* |
| WEB-FORM-TEXT | Single and multi-line text input | WebTextBox.* |
| WEB-FORM-LIST | List box with items | WebListBox, WebListBoxItems |
| WEB-FORM-OPTION | Option buttons | WebOptionButton |
| WEB-FORM-CHECK | Check boxes | WebCheckBox |
| WEB-HTML-FRAGMENT | HTML code fragment | ShapeType WebHTMLFragment, WebControlType |
| WEB-HOTSPOT | Image hot spot | ShapeType WebHotSpot |
| WEB-COMPONENT | Web components | WebComponentFormat, WebControlWebComponent |
| WEB-ENCODING | Save web pages with chosen encoding | WebOptions.Encoding, AlwaysSaveInDefaultEncoding |
| WEB-FOLDER | Organise files in a folder | WebOptions.OrganizeInFolder |
| WEB-VML | Rely on VML for shapes | WebOptions.RelyOnVML |
| WEB-WEBFONTS | Show only web fonts | WebOptions.ShowOnlyWebFonts |
| WEB-INCREMENTAL | Incremental upload | WebOptions.EnableIncrementalUpload |
| WEB-TEMPLATES | Web site and email templates and page types | Wizard WebSite*, Email*, WizardPageTypeWeb* |

### Features in the object model that may be absent from the UI inventory

These appear only in scripting and should be checked against the UI before planning: BuildingBlock galleries per
type, Tags on documents, shape `BlackWhiteMode`, `FindShapesByTag`, `SurplusShapes`, `Assistance`, `FileSearch`,
and `OfficeDataSourceObject`.
