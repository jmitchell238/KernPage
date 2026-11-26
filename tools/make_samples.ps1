# Generate controlled .pub samples by automating the locally installed Publisher (COM).
# Usage (Windows PowerShell 5.1):  powershell -NoProfile -ExecutionPolicy Bypass -File make_samples.ps1 -OutDir <dir>
# Writes <name>.pub files plus manifest.json describing exactly what was set (the test oracle).
# Units: Publisher's object model uses points (72 per inch).
param([Parameter(Mandatory = $true)][string]$OutDir, [string[]]$Only)
# -Only name1,name2 regenerates just those samples and keeps the other manifest entries.
$ErrorActionPreference = 'Stop'

$msoTrue = -1; $msoFalse = 0
$horizontal = 1                       # pbTextOrientationHorizontal
$picture = Join-Path $PSScriptRoot 'assets\test-picture.png'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

if (Get-Process MSPUB -ErrorAction SilentlyContinue) { throw 'Publisher is already running; close it first so this script does not touch your open documents.' }
$app = New-Object -ComObject Publisher.Application
$manifest = [ordered]@{}
$manifestPath = Join-Path $OutDir 'manifest.json'
if ($Only -and (Test-Path $manifestPath)) {
    (Get-Content $manifestPath -Raw | ConvertFrom-Json).PSObject.Properties | ForEach-Object { $manifest[$_.Name] = $_.Value }
}
$Only = @($Only | ForEach-Object { $_ -split ',' } | Where-Object { $_ })
function Want($n) { (-not $Only) -or ($Only -contains $n) }
$log = Join-Path $OutDir 'progress.log'; Set-Content $log ''
function Step($m) { Add-Content $log "$(Get-Date -Format HH:mm:ss) $m" }
Step 'started'

# Publisher 2010's SaveAs hangs under automation for new documents, so each sample starts as a
# copy of a blank publication that is opened, edited, and saved in place with Save().
$template = Join-Path $PSScriptRoot '..\samples\blank.pub'
function New-Doc($name) {
    $path = Join-Path $OutDir "$name.pub"
    Copy-Item $template $path -Force
    Step "open $name"; $d = $app.Open($path); Step "opened $name"; return $d
}
function Save-Doc($doc, $name) { Step "saving $name"; $doc.Save(); Step "saved $name"; $doc.Close() }
function Box($page, $left, $top, $width, $height, $text) {
    $s = $page.Shapes.AddTextbox($horizontal, $left, $top, $width, $height)
    $s.TextFrame.TextRange.Text = $text
    return $s
}
function Describe-Box($s) {
    $paras = @()
    $tr = $s.TextFrame.TextRange
    $count = ($tr.Text.TrimEnd("`r") -split "`r").Count
    for ($i = 1; $i -le $count; $i++) {
        $p = $tr.Paragraphs($i)
        $paras += [ordered]@{ text = $p.Text; font = $p.Font.Name; size = $p.Font.Size; bold = $p.Font.Bold; italic = $p.Font.Italic;
                              underline = $p.Font.Underline; color = $p.Font.Color.RGB; alignment = $p.ParagraphFormat.Alignment }
    }
    [ordered]@{ name = $s.Name; left_pt = $s.Left; top_pt = $s.Top; width_pt = $s.Width; height_pt = $s.Height; paragraphs = $paras }
}

try {
    if (Want 'apple-banana') {
        Step 'sample 1'
        # 1. Two boxes with different text, created in the opposite order to their position on the page.
        $doc = New-Doc 'apple-banana'; $pg = $doc.Pages(1)
        $a = Box $pg 72 576 216 72 'Apple'        # created first, lower on the page
        $b = Box $pg 72 72 216 72 'Banana'        # created second, near the top
        $manifest['apple-banana'] = [ordered]@{ note = 'Apple created first (lower), Banana second (upper)'; boxes = @((Describe-Box $a), (Describe-Box $b)) }
        Save-Doc $doc 'apple-banana'
    }

    if (Want 'formatting') {
        Step 'sample 2'
        # 2. Character and paragraph formatting, one paragraph per variation.
        $doc = New-Doc 'formatting'; $pg = $doc.Pages(1)
        $lines = @('Arial 12', 'Times New Roman 18 italic', 'Courier New 10 underline', 'Calibri 14 bold italic', 'Red Calibri 11', 'Centered', 'Right aligned', 'Justified paragraph with enough words to wrap across more than one line of the box.')
        $s = Box $pg 72 72 432 360 ($lines -join "`r")
        $tr = $s.TextFrame.TextRange
        $p = $tr.Paragraphs(1); $p.Font.Name = 'Arial'; $p.Font.Size = 12
        $p = $tr.Paragraphs(2); $p.Font.Name = 'Times New Roman'; $p.Font.Size = 18; $p.Font.Italic = $msoTrue
        $p = $tr.Paragraphs(3); $p.Font.Name = 'Courier New'; $p.Font.Size = 10; $p.Font.Underline = 1
        $p = $tr.Paragraphs(4); $p.Font.Name = 'Calibri'; $p.Font.Size = 14; $p.Font.Bold = $msoTrue; $p.Font.Italic = $msoTrue
        $p = $tr.Paragraphs(5); $p.Font.Name = 'Calibri'; $p.Font.Size = 11; $p.Font.Color.RGB = 0x0000FF   # RGB() is BGR-ordered: red
        $tr.Paragraphs(6).ParagraphFormat.Alignment = 2
        $tr.Paragraphs(7).ParagraphFormat.Alignment = 3
        $tr.Paragraphs(8).ParagraphFormat.Alignment = 4
        $manifest['formatting'] = [ordered]@{ boxes = @((Describe-Box $s)) }
        Save-Doc $doc 'formatting'
    }

    if (Want 'picture') {
        Step 'sample 3'
        # 3. One picture.
        $doc = New-Doc 'picture'; $pg = $doc.Pages(1)
        $pic = $pg.Shapes.AddPicture($picture, $msoFalse, $msoTrue, 144, 144, 200, 120)
        $manifest['picture'] = [ordered]@{ picture = [ordered]@{ name = $pic.Name; left_pt = $pic.Left; top_pt = $pic.Top; width_pt = $pic.Width; height_pt = $pic.Height; source_png = 'tools/assets/test-picture.png' } }
        Save-Doc $doc 'picture'
    }

    if (Want 'legal') {
        Step 'sample 4'
        # 4. Legal page size.
        $doc = New-Doc 'legal'
        $doc.PageSetup.PageWidth = 612; $doc.PageSetup.PageHeight = 1008
        $s = Box $doc.Pages(1) 72 72 216 72 'Legal page'
        $manifest['legal'] = [ordered]@{ page_width_pt = $doc.PageSetup.PageWidth; page_height_pt = $doc.PageSetup.PageHeight; boxes = @((Describe-Box $s)) }
        Save-Doc $doc 'legal'
    }

    if (Want 'bulletin') {
        Step 'sample 5'
        # 5. A realistic two-page church bulletin (original placeholder content).
        $doc = New-Doc 'bulletin'
        $doc.Pages.Add(1, 1) | Out-Null
        $p1 = $doc.Pages(1); $p2 = $doc.Pages(2)
        $title = Box $p1 54 54 504 72 "Grace Community Church`rSunday Morning Worship"
        $title.TextFrame.TextRange.ParagraphFormat.Alignment = 2
        $t = $title.TextFrame.TextRange.Paragraphs(1); $t.Font.Name = 'Georgia'; $t.Font.Size = 24; $t.Font.Bold = $msoTrue
        $t = $title.TextFrame.TextRange.Paragraphs(2); $t.Font.Name = 'Georgia'; $t.Font.Size = 14; $t.Font.Italic = $msoTrue
        $pic = $p1.Shapes.AddPicture($picture, $msoFalse, $msoTrue, 206, 144, 200, 120)
        $order = Box $p1 54 288 504 360 ("Order of Service`rPrelude`rCall to Worship`rHymn: Amazing Grace`rScripture Reading: Psalm 23`rPrayer`rSermon: Walking by Faith`rHymn: Blessed Assurance`rBenediction")
        $h = $order.TextFrame.TextRange.Paragraphs(1); $h.Font.Size = 16; $h.Font.Bold = $msoTrue
        $ann = Box $p2 54 54 504 288 ("Announcements`rWednesday Bible Study meets at 6:30 PM in the fellowship hall.`rThe food pantry needs canned vegetables and cereal this month.`rChoir practice is Thursday at 7:00 PM.")
        $h = $ann.TextFrame.TextRange.Paragraphs(1); $h.Font.Size = 16; $h.Font.Bold = $msoTrue
        $pray = Box $p2 54 378 504 216 ("Prayer List`rThose who are sick or recovering`rOur missionaries`rOur community and its leaders")
        $h = $pray.TextFrame.TextRange.Paragraphs(1); $h.Font.Size = 16; $h.Font.Bold = $msoTrue
        $manifest['bulletin'] = [ordered]@{ pages = 2; boxes = @((Describe-Box $title), (Describe-Box $order), (Describe-Box $ann), (Describe-Box $pray));
                                           picture = [ordered]@{ left_pt = $pic.Left; top_pt = $pic.Top; width_pt = $pic.Width; height_pt = $pic.Height } }
        Save-Doc $doc 'bulletin'
    }

    if (Want 'colors') {
        Step 'sample colors'
        # 6. Text colours (Publisher's RGB values are BGR-ordered integers).
        $doc = New-Doc 'colors'; $pg = $doc.Pages(1)
        $s = Box $pg 72 72 432 216 ("Black default`rRed text`rGreen text`rBlue text`rOrange text`rGray text")
        $tr = $s.TextFrame.TextRange
        $tr.Paragraphs(2).Font.Color.RGB = 0x0000FF
        $tr.Paragraphs(3).Font.Color.RGB = 0x00FF00
        $tr.Paragraphs(4).Font.Color.RGB = 0xFF0000
        $tr.Paragraphs(5).Font.Color.RGB = 0x0080FF
        $tr.Paragraphs(6).Font.Color.RGB = 0x808080
        $manifest['colors'] = [ordered]@{ note = 'color values are BGR integers: 255 = red'; boxes = @((Describe-Box $s)) }
        Save-Doc $doc 'colors'
    }

    if (Want 'shapes') {
        Step 'sample shapes'
        # 7. Basic drawn shapes with fills and borders.
        $doc = New-Doc 'shapes'; $pg = $doc.Pages(1)
        $r = $pg.Shapes.AddShape(1, 72, 72, 144, 72)             # rectangle
        $r.Fill.Visible = $msoTrue; $r.Fill.ForeColor.RGB = 0xE6D8AD   # light blue
        $r.Line.Visible = $msoTrue; $r.Line.ForeColor.RGB = 0x8B0000; $r.Line.Weight = 2   # dark blue, 2 pt
        $o = $pg.Shapes.AddShape(9, 288, 72, 144, 72)            # oval
        $o.Fill.Visible = $msoTrue; $o.Fill.ForeColor.RGB = 0x00FFFF   # yellow
        $o.Line.Visible = $msoFalse
        $l = $pg.Shapes.AddLine(72, 216, 504, 216)
        $l.Line.ForeColor.RGB = 0x0000FF; $l.Line.Weight = 3       # red, 3 pt
        $q = $pg.Shapes.AddShape(1, 72, 288, 144, 72)            # outline-only rectangle
        $q.Fill.Visible = $msoFalse
        $q.Line.Visible = $msoTrue; $q.Line.ForeColor.RGB = 0x000000; $q.Line.Weight = 1
        $list = @()
        foreach ($x in @($r, $o, $l, $q)) {
            $list += [ordered]@{ name = $x.Name; type = $x.Type; autoshape = $x.AutoShapeType; left_pt = $x.Left; top_pt = $x.Top; width_pt = $x.Width; height_pt = $x.Height;
                                 fill_visible = $x.Fill.Visible; fill_rgb = $x.Fill.ForeColor.RGB; line_visible = $x.Line.Visible; line_rgb = $x.Line.ForeColor.RGB; line_weight_pt = $x.Line.Weight }
        }
        $manifest['shapes'] = [ordered]@{ note = 'rgb values are BGR integers; order: rectangle, oval, line, outline rectangle'; shapes = $list }
        Save-Doc $doc 'shapes'
    }

    if (Want 'table') {
        Step 'sample table'
        # 8. A 2 x 2 table with text in each cell.
        $doc = New-Doc 'table'; $pg = $doc.Pages(1)
        $t = $pg.Shapes.AddTable(2, 2, 72, 72, 288, 72)
        $cells = [ordered]@{}
        foreach ($rc in @(@(1,1,'A1'), @(1,2,'B1'), @(2,1,'A2'), @(2,2,'B2'))) {
            $t.Table.Rows($rc[0]).Cells($rc[1]).TextRange.Text = $rc[2]; $cells["r$($rc[0])c$($rc[1])"] = $rc[2]
        }
        $manifest['table'] = [ordered]@{ left_pt = $t.Left; top_pt = $t.Top; width_pt = $t.Width; height_pt = $t.Height; rows = 2; columns = 2; cells = $cells }
        Save-Doc $doc 'table'
    }
    if (Want 'schemes') {
        Step 'sample schemes'
        # 9. A non-default colour scheme, with one rectangle per scheme slot filled from that slot.
        $doc = New-Doc 'schemes'; $pg = $doc.Pages(1)
        $doc.ColorScheme = $app.ColorSchemes.Item(5)
        $slots = @()
        $x = 36; $y = 36
        for ($k = 1; $k -le 16; $k++) {
            try { $rgb = $doc.ColorScheme.Colors($k).RGB } catch { continue }
            $r = $pg.Shapes.AddShape(1, $x, $y, 60, 40)
            try { $r.Fill.ForeColor.SchemeColor = $k } catch { $r.Delete(); continue }
            $r.Line.Visible = $msoFalse
            $slots += [ordered]@{ slot = $k; rgb = $rgb; left_pt = $x; top_pt = $y }
            $x += 72; if ($x -gt 500) { $x = 36; $y += 60 }
        }
        $manifest['schemes'] = [ordered]@{ scheme_name = $doc.ColorScheme.Name; note = 'rgb values are BGR integers; one 60x40 pt rectangle per scheme slot'; slots = $slots }
        Save-Doc $doc 'schemes'
    }
    if (Want 'linked') {
        Step 'sample linked'
        # 10. One story flowing through three linked text boxes (two on page 1, one on page 2),
        #     plus an unrelated box created between them so story order != box order.
        $doc = New-Doc 'linked'
        $doc.Pages.Add(1, 1) | Out-Null
        $p1 = $doc.Pages(1); $p2 = $doc.Pages(2)
        $paras = @(); for ($i = 1; $i -le 30; $i++) { $paras += "Paragraph $i of the linked story, long enough to take up a full line or two in a narrow box." }
        $a = Box $p1 72 72 216 144 ($paras -join "`r")
        $lone = Box $p1 324 72 216 72 'Standalone box'
        $b = Box $p1 72 288 216 144 ''
        $c = Box $p2 72 72 216 288 ''
        $a.TextFrame.NextLinkedTextFrame = $b.TextFrame
        $b.TextFrame.NextLinkedTextFrame = $c.TextFrame
        $frames = @()
        foreach ($f in @($a, $b, $c)) {
            $tr = $f.TextFrame.TextRange
            $frames += [ordered]@{ name = $f.Name; left_pt = $f.Left; top_pt = $f.Top; width_pt = $f.Width; height_pt = $f.Height;
                                   first_chars = $tr.Text.Substring(0, [Math]::Min(40, $tr.Text.Length)); length = $tr.Length;
                                   has_next = [bool]$f.TextFrame.HasNextLink; has_previous = [bool]$f.TextFrame.HasPreviousLink; overflowing = [bool]$f.TextFrame.Overflowing }
        }
        $manifest['linked'] = [ordered]@{ pages = 2; note = 'one story in frames a->b->c; first_chars/length are the part of the story shown in each frame'; story_paragraphs = 30;
                                          frames = $frames; standalone = (Describe-Box $lone) }
        Save-Doc $doc 'linked'
    }
    if (Want 'spacing') {
        Step 'sample spacing'
        # 11. Paragraph spacing: defaults first, then one explicit change per paragraph.
        $doc = New-Doc 'spacing'; $pg = $doc.Pages(1)
        $lines = @('Default paragraph one.', 'Default paragraph two.', 'Space before 18 pt.', 'Space after 24 pt.', 'Line spacing 2 lines, long enough to wrap onto a second line inside this box so the spacing shows.', 'Line spacing exactly 30 pt, long enough to wrap onto a second line inside this box so the spacing shows.', 'Last default paragraph.')
        $s = Box $pg 72 72 288 576 ($lines -join "`r")
        $tr = $s.TextFrame.TextRange
        $defaults = @()
        for ($i = 1; $i -le 2; $i++) { $pf = $tr.Paragraphs($i).ParagraphFormat
            $defaults += [ordered]@{ space_before = $pf.SpaceBefore; space_after = $pf.SpaceAfter; line_spacing = $pf.LineSpacing; line_spacing_rule = $pf.LineSpacingRule } }
        $tr.Paragraphs(3).ParagraphFormat.SpaceBefore = 18
        $tr.Paragraphs(4).ParagraphFormat.SpaceAfter = 24
        $p = $tr.Paragraphs(5).ParagraphFormat; $p.LineSpacingRule = 2      # pbLineSpacingDouble
        $p = $tr.Paragraphs(6).ParagraphFormat; $p.LineSpacingRule = 4; $p.LineSpacing = 30   # pbLineSpacingExactly
        $after = @()
        for ($i = 1; $i -le $lines.Count; $i++) { $pf = $tr.Paragraphs($i).ParagraphFormat
            $after += [ordered]@{ text = $lines[$i-1]; space_before = $pf.SpaceBefore; space_after = $pf.SpaceAfter; line_spacing = $pf.LineSpacing; line_spacing_rule = $pf.LineSpacingRule;
                                   top_pt = $tr.Paragraphs($i).BoundTop; height_pt = $tr.Paragraphs($i).BoundHeight } }
        $manifest['spacing'] = [ordered]@{ note = 'space/line values in points; line_spacing_rule is the Publisher enum; top_pt/height_pt are where Publisher laid each paragraph out';
                                           box = (Describe-Box $s); defaults = $defaults; paragraphs = $after }
        Save-Doc $doc 'spacing'
    }
    if (Want 'rotate') {
        Step 'sample rotate'
        # 12. Rotated shapes: text boxes at 90, 270 and 30 degrees, plus a rotated rectangle.
        $doc = New-Doc 'rotate'; $pg = $doc.Pages(1)
        $items = @()
        foreach ($spec in @(@(90, 72, 72), @(270, 288, 72), @(30, 72, 360))) {
            $b = Box $pg $spec[1] $spec[2] 180 54 "Rotated $($spec[0]) degrees"
            $b.Rotation = $spec[0]
            $items += [ordered]@{ kind = 'textbox'; rotation = $b.Rotation; left_pt = $b.Left; top_pt = $b.Top; width_pt = $b.Width; height_pt = $b.Height; text = "Rotated $($spec[0]) degrees" }
        }
        $r = $pg.Shapes.AddShape(1, 360, 400, 120, 60); $r.Rotation = 45
        $items += [ordered]@{ kind = 'rectangle'; rotation = $r.Rotation; left_pt = $r.Left; top_pt = $r.Top; width_pt = $r.Width; height_pt = $r.Height }
        $manifest['rotate'] = [ordered]@{ note = 'left/top/width/height are the unrotated frame as Publisher reports it; rotation clockwise in degrees'; shapes = $items }
        Save-Doc $doc 'rotate'
    }
    if (Want 'columns') {
        Step 'sample columns'
        # 13. A text box with 3 columns and a 0.25 in gap, and one with 2 columns and default gap.
        $doc = New-Doc 'columns'; $pg = $doc.Pages(1)
        $paras = @(); for ($i = 1; $i -le 24; $i++) { $paras += "Column paragraph $i with enough words to wrap across a narrow column." }
        $a = Box $pg 72 72 468 288 ($paras -join "`r")
        $a.TextFrame.Columns = 3; $a.TextFrame.ColumnSpacing = 18
        $b = Box $pg 72 432 468 216 ($paras[0..11] -join "`r")
        $b.TextFrame.Columns = 2
        $manifest['columns'] = [ordered]@{ boxes = @(
            [ordered]@{ name = $a.Name; left_pt = $a.Left; top_pt = $a.Top; width_pt = $a.Width; height_pt = $a.Height; columns = $a.TextFrame.Columns; column_spacing_pt = $a.TextFrame.ColumnSpacing },
            [ordered]@{ name = $b.Name; left_pt = $b.Left; top_pt = $b.Top; width_pt = $b.Width; height_pt = $b.Height; columns = $b.TextFrame.Columns; column_spacing_pt = $b.TextFrame.ColumnSpacing }) }
        Save-Doc $doc 'columns'
    }
    if (Want 'table2') {
        Step 'sample table2'
        # 14. A 3x3 table with uneven column widths and row heights, and cells holding several paragraphs.
        $doc = New-Doc 'table2'; $pg = $doc.Pages(1)
        $t = $pg.Shapes.AddTable(3, 3, 72, 72, 432, 216)
        $tb = $t.Table
        $tb.Columns(1).Width = 72; $tb.Columns(2).Width = 216; $tb.Columns(3).Width = 144
        $tb.Rows(1).Height = 36; $tb.Rows(2).Height = 108; $tb.Rows(3).Height = 72
        $cells = [ordered]@{}
        for ($r = 1; $r -le 3; $r++) { for ($c = 1; $c -le 3; $c++) {
            $txt = "R$r C$c"
            if ($r -eq 2 -and $c -eq 2) { $txt = "First paragraph in B2`rSecond paragraph in B2`rThird" }
            if ($r -eq 3 -and $c -eq 1) { $txt = "Two`rLines" }
            $tb.Rows($r).Cells($c).TextRange.Text = $txt; $cells["r$($r)c$($c)"] = $txt } }
        $widths = @(); for ($c = 1; $c -le 3; $c++) { $widths += $tb.Columns($c).Width }
        $heights = @(); for ($r = 1; $r -le 3; $r++) { $heights += $tb.Rows($r).Height }
        $manifest['table2'] = [ordered]@{ left_pt = $t.Left; top_pt = $t.Top; width_pt = $t.Width; height_pt = $t.Height; rows = 3; columns = 3;
                                          column_widths_pt = $widths; row_heights_pt = $heights; cells = $cells; note = 'cell text uses CR between paragraphs' }
        Save-Doc $doc 'table2'
    }
    if (Want 'group') {
        Step 'sample group'
        # 15. A group of a text box + rectangle, and an ungrouped text box for comparison.
        $doc = New-Doc 'group'; $pg = $doc.Pages(1)
        $t = Box $pg 144 144 180 54 'Grouped caption'
        $r = $pg.Shapes.AddShape(1, 144, 216, 180, 108)
        $lone = Box $pg 360 144 144 54 'Not grouped'
        $before = @((Describe-Box $t), [ordered]@{ name = $r.Name; left_pt = $r.Left; top_pt = $r.Top; width_pt = $r.Width; height_pt = $r.Height })
        $g = $pg.Shapes.Range(@($t.Name, $r.Name)).Group()
        $kids = @(); for ($k = 1; $k -le $g.GroupItems.Count; $k++) { $c = $g.GroupItems.Item($k)
            $kids += [ordered]@{ name = $c.Name; left_pt = $c.Left; top_pt = $c.Top; width_pt = $c.Width; height_pt = $c.Height } }
        $manifest['group'] = [ordered]@{ group = [ordered]@{ name = $g.Name; left_pt = $g.Left; top_pt = $g.Top; width_pt = $g.Width; height_pt = $g.Height; children = $kids };
                                         standalone = (Describe-Box $lone); note = 'children positions are page coordinates as Publisher reports them' }
        Save-Doc $doc 'group'
    }
    if (Want 'wrap') {
        Step 'sample wrap'
        # 16. Text wrap: a long story with a rectangle (square wrap) and an oval (tight wrap) over it, and one with no wrap.
        $doc = New-Doc 'wrap'; $pg = $doc.Pages(1)
        $paras = @(); for ($i = 1; $i -le 20; $i++) { $paras += "Wrap paragraph $i, written with enough words to fill several lines across the box." }
        $s = Box $pg 72 72 468 648 ($paras -join "`r")
        $sq = $pg.Shapes.AddShape(1, 216, 144, 144, 108); $sq.TextWrap.Type = 2      # pbWrapTypeSquare
        $ov = $pg.Shapes.AddShape(9, 216, 360, 144, 108); $ov.TextWrap.Type = 3      # pbWrapTypeTight
        $nw = $pg.Shapes.AddShape(1, 216, 576, 144, 72); $nw.TextWrap.Type = 0       # pbWrapTypeNone
        $wr = @(); foreach ($x in @($sq, $ov, $nw)) {
            $w = $x.TextWrap; $wr += [ordered]@{ name = $x.Name; left_pt = $x.Left; top_pt = $x.Top; width_pt = $x.Width; height_pt = $x.Height; wrap_type = $w.Type; side = $w.Side;
                                                 dist_left = $w.DistanceLeft; dist_top = $w.DistanceTop; dist_right = $w.DistanceRight; dist_bottom = $w.DistanceBottom } }
        $manifest['wrap'] = [ordered]@{ box = (Describe-Box $s); wrappers = $wr; note = 'wrap_type: Publisher PbWrapType enum; distances in points' }
        Save-Doc $doc 'wrap'
    }
    # ---- Master pages, page numbers, headers/footers (research for beads pw-s81; spec "Master pages, page numbers and headers/footers") ----
    # Names start with master-. Regenerate with -Only master-box-1p,... ; use -OutDir out\research\masters.
    # Each recipe changes ONE thing against blank.pub (1 page) or against the 2/3-page version (Pages.Add) so Contents/Escher/Quill can be diffed.
    function Master-Doc($name, $pages = 1) {
        $d = New-Doc $name
        if ($pages -gt 1) { $d.Pages.Add($pages - 1, 1) | Out-Null }
        return $d
    }
    function Master-Box($d, $text, $left = 72, $top = 72) {
        $b = $d.MasterPages(1).Shapes.AddTextbox($horizontal, $left, $top, 216, 36)
        if ($text) { $b.TextFrame.TextRange.Text = $text }
        return $b
    }
    function Add-PageNumber($box, $prefix) {
        if ($prefix) { $box.TextFrame.TextRange.Text = $prefix }
        $r = $box.TextFrame.TextRange.InsertAfter("")
        $null = $r.InsertPageNumber(1)          # pbPageNumberCurrent (0 is rejected)
        return $box.TextFrame.TextRange.Text    # Publisher shows the field as '#'
    }
    function Master-Info($d) {
        $o = [ordered]@{ pages = $d.Pages.Count; masters = @(); sections = @() }
        for ($i = 1; $i -le $d.MasterPages.Count; $i++) { $m = $d.MasterPages($i); $o.masters += [ordered]@{ name = $m.Name; two_page = $m.IsTwoPageMaster; shapes = $m.Shapes.Count } }
        for ($i = 1; $i -le $d.Sections.Count; $i++) { $c = $d.Sections($i); $o.sections += [ordered]@{ start_page = $c.StartPageIndex; number_start = $c.PageNumberStart; format = $c.PageNumberFormat; continue = $c.ContinueNumbersFromPreviousSection } }
        $o.page_masters = @(); $o.page_ignore = @(); $o.page_numbers = @()
        for ($i = 1; $i -le $d.Pages.Count; $i++) { $pg = $d.Pages($i); $o.page_masters += $pg.Master.Name; $o.page_ignore += $pg.IgnoreMaster; $o.page_numbers += $pg.PageNumber }
        return $o
    }
    function Master-Run($n, $pages, $note, [scriptblock]$body) {
        if (-not (Want $n)) { return }
        Step "sample $n"; $doc = Master-Doc $n $pages; $err = ''; $extra = $null
        try { $extra = & $body $doc } catch { $err = $_.Exception.Message }
        $e = [ordered]@{ note = $note; error = $err; result = $extra; info = (Master-Info $doc) }
        $script:manifest[$n] = $e
        Save-Doc $doc $n
    }
    Master-Run 'master-base-2p' 2 'control: 2 blank pages' { param($d) }
    Master-Run 'master-base-3p' 3 'control: 3 blank pages' { param($d) }
    Master-Run 'master-box-1p' 1 'one text box MASTERTEXT at 72,72 216x36 on master page A' { param($d) $null = Master-Box $d 'MASTERTEXT' }
    Master-Run 'master-box-2p' 2 'same, 2 pages' { param($d) $null = Master-Box $d 'MASTERTEXT' }
    Master-Run 'master-boxpage-1p' 1 'control: the same box on document page 1 (not on the master)' { param($d)
        $b = $d.Pages(1).Shapes.AddTextbox($horizontal, 72, 72, 216, 36); $b.TextFrame.TextRange.Text = 'MASTERTEXT' }
    Master-Run 'master-pn-1p' 1 'master text box holding only a current-page-number field' { param($d) $b = Master-Box $d ''; Add-PageNumber $b '' }
    Master-Run 'master-pn-2p' 2 'same, 2 pages' { param($d) $b = Master-Box $d ''; Add-PageNumber $b '' }
    Master-Run 'master-pn-text-2p' 2 'master box "Page " + field' { param($d) $b = Master-Box $d ''; Add-PageNumber $b 'Page ' }
    Master-Run 'master-pn-boxpage-1p' 1 'page-number field in a normal page text box' { param($d)
        $b = $d.Pages(1).Shapes.AddTextbox($horizontal, 72, 72, 216, 36); Add-PageNumber $b '' }
    Master-Run 'master-box-2p-ignore' 2 'master box; Pages(2).IgnoreMaster = true' { param($d) $null = Master-Box $d 'MASTERTEXT'; $d.Pages(2).IgnoreMaster = $msoTrue }
    # page-number formats / start / sections: 3 pages, master page-number box in every case
    foreach ($f in 1..6) {
        Master-Run "master-pnfmt-$f" 3 "Sections(1).PageNumberFormat = $f" { param($d) $b = Master-Box $d ''; $null = Add-PageNumber $b ''; $d.Sections(1).PageNumberFormat = $f; $d.Sections(1).PageNumberFormat }.GetNewClosure()
    }
    Master-Run 'master-pn-start5' 3 'Sections(1).PageNumberStart = 5' { param($d)
        $b = Master-Box $d ''; $null = Add-PageNumber $b ''; $d.Sections(1).ContinueNumbersFromPreviousSection = $msoFalse; $d.Sections(1).PageNumberStart = 5 }
    Master-Run 'master-pn-section2' 3 'Sections.Add(2) only' { param($d) $b = Master-Box $d ''; $null = Add-PageNumber $b ''; $null = $d.Sections.Add(2) }
    Master-Run 'master-pn-section2-fmt' 3 'Sections.Add(2); section 2 start 1, format 2' { param($d)
        $b = Master-Box $d ''; $null = Add-PageNumber $b ''; $null = $d.Sections.Add(2); $s2 = $d.Sections(2)
        $s2.ContinueNumbersFromPreviousSection = $msoFalse; $s2.PageNumberStart = 1; $s2.PageNumberFormat = 2 }
    Master-Run 'master-pn-first-hf' 3 'Sections(1).ShowHeaderFooterOnFirstPage = false' { param($d)
        $b = Master-Box $d ''; $null = Add-PageNumber $b ''; $d.Sections(1).ShowHeaderFooterOnFirstPage = $msoFalse }
    # second master page
    Master-Run 'master-two-add' 2 'MasterPages.Add(0) (one-page master), not applied' { param($d) $null = $d.MasterPages.Add(0) }
    Master-Run 'master-two-apply' 2 'second master; Pages(2).Master = second' { param($d) $m2 = $d.MasterPages.Add(0); $d.Pages(2).Master = $m2 }
    Master-Run 'master-two-box' 2 'second master with box SECONDTEXT applied to page 2' { param($d)
        $m2 = $d.MasterPages.Add(0); $b = $m2.Shapes.AddTextbox($horizontal, 72, 72, 216, 36); $b.TextFrame.TextRange.Text = 'SECONDTEXT'; $d.Pages(2).Master = $m2 }
    Master-Run 'master-two-rename' 2 'second master renamed to Renamed, applied to page 2' { param($d)
        $m2 = $d.MasterPages.Add(0); $d.Pages(2).Master = $m2; $m2.Name = 'Renamed' }
    Master-Run 'master-rename-a' 1 'only master A renamed to Front' { param($d) $d.MasterPages(1).Name = 'Front' }
    Master-Run 'master-twopage-add' 2 'two-page master added and applied to page 2' { param($d) $m2 = $d.MasterPages.Add(1); $d.Pages(2).Master = $m2 }
    Master-Run 'master-bg' 1 'master A background solid red (BGR 0x0000FF)' { param($d)
        $bg = $d.MasterPages(1).Background; $bg.Create(); $f = $bg.Fill; $f.Visible = $msoTrue; $f.Solid(); $f.ForeColor.RGB = 0x0000FF }
    Master-Run 'master-bg-page' 2 'page 1 background solid green (BGR 0x00FF00)' { param($d)
        $bg = $d.Pages(1).Background; $bg.Create(); $f = $bg.Fill; $f.Visible = $msoTrue; $f.Solid(); $f.ForeColor.RGB = 0x00FF00 }
    Master-Run 'master-header' 1 'probe: Pages(1).Header / Footer on a blank page' { param($d)
        "header null: " + ($null -eq $d.Pages(1).Header) + "; footer null: " + ($null -eq $d.Pages(1).Footer) }

    # NOTE: AutoFitText does not take effect when Publisher runs hidden under automation
    # (values 0-3 are accepted but read back as 0), so this sample is not generated by default.
    if ($Only -contains 'autofit') {
        Step 'sample autofit'
        # 17. AutoFit: same overflowing text in three boxes - no autofit, shrink on overflow, best fit.
        $doc = New-Doc 'autofit'; $pg = $doc.Pages(1)
        $txt = 'Living and Learning in Class X'
        $out = @(); $y = 72
        foreach ($mode in @(0, 1, 2)) {          # pbTextAutoFitNone, pbTextAutoFitShrinkOnOverflow, pbTextAutoFitBestFit
            $b = Box $pg 72 $y 216 54 $txt
            $b.TextFrame.TextRange.Font.Size = 36
            try { $b.TextFrame.AutoFitText = $mode } catch { }
            $out += [ordered]@{ name = $b.Name; top_pt = $b.Top; mode = $b.TextFrame.AutoFitText; font_size_after = $b.TextFrame.TextRange.Font.Size }
            $y += 108
        }
        $manifest['autofit'] = [ordered]@{ boxes = $out; note = 'mode: PbTextAutoFitType; font_size_after = size Publisher reports after autofit; boxes 216x54 pt; text set at 36 pt' }
        Save-Doc $doc 'autofit'
    }
}
finally {
    $app.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($app) | Out-Null
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 $manifestPath
Write-Output "done: $((Get-ChildItem $OutDir -Filter *.pub).Count) files"
