# Publisher after 2010: what changed (public-sources history)

Research date: November 2026. Scope: Publisher 2013, 2016, 2019, 2021, LTSC 2024 /
Microsoft 365, and the retirement. Baseline for KernPage today: Publisher 2010
(14.0).

Legend: ✔ = Microsoft source, ~ = third-party source, ? = unconfirmed / not found.

## Summary

- **Does the `.pub` format differ after 2010?** Probably, but this is the weakest-evidenced
  point. The only direct statement found is third-party: files from Publisher 2013 and
  later share one format, "the main format change came with 2013", and older versions
  "may have trouble" opening them ~
  ([Office Watch](https://office-watch.com/2026/best-office-version-for-publisher/)). A
  Microsoft Press sample chapter says `.pub` is "directly" compatible across Publisher
  2003 through 2013 ~
  ([Microsoft Press](https://www.microsoftpressstore.com/articles/article.aspx?p=3131575)),
  which conflicts with the Office Watch claim as far as 2010 vs 2013 goes. Microsoft has
  never published a `.pub` specification that we could find ?. No Microsoft page we
  found says Publisher 2010 can or cannot open a 2013+ file, and none describes a new
  2013 file version or a "Save as Publisher 2010" option ?.
- **One confirmed 2013 file-content change:** Publisher 2013 removed colour modes
  (CMYK / spot / black-and-white). Files that used them are converted to RGB on open
  ✔ ([Changes in Office 2013](https://learn.microsoft.com/en-us/previous-versions/office/office-2013-resource-kit/cc178954(v=office.15))).
  So a 2013+ file is always RGB-only; a 2010 file with colour modes loses them when
  opened in 2013+.
- **Biggest user-visible additions** (all 2013): pictures that are easy to move/swap and
  new picture and text effects, a redesigned Backstage / start screen, new templates,
  online picture search ✔/~ (details below). After 2013, sources agree there were
  essentially no feature changes through 2021 ~.
- **Retirement:** Microsoft ended Publisher on 1 October 2026 for Microsoft 365, with
  perpetual support ending 13 October 2026 (Office LTSC 2021 / Office 2021). Office 2024
  and LTSC 2024 do not appear to ship Publisher. Microsoft's advice for `.pub` files is
  "convert to PDF before 1 Oct 2026" (PDF to Word to edit).

## Publisher 2010 (14.0, 15 Jun 2010) - baseline

- Released 15 Jun 2010; first Publisher with the ribbon ~
  ([Wikipedia](https://en.wikipedia.org/wiki/Microsoft_Publisher)).
- Save & Send > Change File Type offers Publisher 2000 / Publisher 98 compatible formats,
  template, image types, web page ✔-ish: the Microsoft VBA enum lists
  `pbFilePublisher2000` and `pbFilePublisher98` only (no 2002/2003/2007/2010 members)
  ✔ ([PbFileFormat](https://learn.microsoft.com/en-us/office/vba/api/publisher.pbfileformat)).
  Publisher 2007 used the same format as 2003 and 2002 ✔ (Microsoft Office support page
  as relayed by search results; page itself not re-fetched) ?.
- Office 2010 end of support: 13 Oct 2020 ✔
  ([Microsoft Tech Community](https://techcommunity.microsoft.com/blog/officeeos/reminder-of-changes-coming-to-office-support-in-october/1302951)).
  SP2 for Office 2010 is KB2687455; whether it contained Publisher-specific fixes could
  not be confirmed ?. The Publisher 2010 installation we observed (version 14.0.7011.1000)
  is not matched to a specific update in any source found ?.

## Publisher 2013 (15.0, 29 Jan 2013)

- Release date, version 15.0 (version 13 skipped) ~
  ([Wikipedia](https://en.wikipedia.org/wiki/Microsoft_Publisher)).
- **Pictures:** new ways to move, swap and add effects to pictures; effects include
  shadows, reflections, glow, soft edges, bevel, 3-D rotation ✔ (Microsoft
  [Publisher 2013 page](https://support.office.com/en-ie/article/publisher-2013-8a665c00-8497-4f17-8e2a-8fca3da92683),
  now retired on Microsoft's site; wording seen via search snippet) and ~
  ([Neowin preview coverage](https://www.neowin.net/news/microsoft-talks-about-publisher-in-office-2013/)).
  Multiple pictures inserted at once are placed in the scratch area ✔ (same page).
- **Online pictures:** search across computer, Office.com clip art and the web; SkyDrive
  access ~ (Neowin, same link; individual product pages seen via search snippets only).
  Exact "photo as background" feature: not found ?.
- **Backstage / start screen:** redesigned Backstage and a new landing screen for opening
  or creating publications ~ (training material surfaced by search; Microsoft page for
  this was retired) ?.
- **Templates:** new template set and improved graphic editing ~ (Neowin).
- **Removed:** colour-mode management (CMYK, spot colour, black and white). Files using
  colour modes are auto-converted to RGB with a notice dialog. Microsoft's reason: low
  usage; printers use PDF anyway ✔
  ([Changes in Office 2013](https://learn.microsoft.com/en-us/previous-versions/office/office-2013-resource-kit/cc178954(v=office.15))).
  This is the only Publisher item in Microsoft's 2013 changed/removed list.
- **File format:** see Summary. Third-party says new shared format from 2013 ~; Microsoft
  Press says 2003-2013 are directly compatible ~; Save As options for 2013 (does it still
  list Publisher 98/2000?) not confirmed ?. A third-party blog claims the older options
  disappear in newer versions; treated as unreliable ?.
- Other ribbon/UI differences vs 2010 beyond Backstage: not found ?.

## Publisher 2016 (16.0, 22 Sep 2015)

- Release date ~ (Wikipedia, link above).
- Feature changes vs 2013: none documented; sources describe 2016, 2019 and 2021 as
  functionally the same ~
  ([Office Watch](https://office-watch.com/2026/best-office-version-for-publisher/)).
- Runs on Windows 7 / 8.1, unlike Office 2019+ ~ (Office Watch, same link).
- Office 2016 stopped receiving security fixes 14 Oct 2025 ~ (Office Watch, same link).
- File format: same as 2013 per the third-party claim ~.

## Publisher 2019 (24 Sep 2018)

- Release date ~ (Wikipedia). Last standalone Publisher sold; hard to find ~
  ([Office Watch 2019](https://office-watch.com/2019/yes-there-is-publisher-2019-heres-how-to-find-it/)).
- Said to get security and bug fixes only; no specific new features listed ~ (same link).
- Office 2019 support ended 14 Oct 2025 ~ (Office Watch 2026 link).

## Publisher 2021 (5 Oct 2021)

- Release date ~ (Wikipedia). Microsoft lists no changes between 2019 and 2021 ~
  ([Office Watch 2024](https://office-watch.com/2024/microsoft-publisher-is-ending-what-you-need-to-know)).
- Publisher LTSC 2021 and consumer Office 2021: support ends 13 Oct 2026 ✔
  ([Microsoft retirement notice](https://support.microsoft.com/en-US/publisher)); lifecycle
  page: <https://learn.microsoft.com/en-us/lifecycle/products/publisher-ltsc-2021>
  (cited by Wikipedia; not fetched) ?.

## Office 2024 / LTSC 2024

- Office 2024 does not include Publisher in any bundle; installing it over Office
  2021/2019 removes Publisher ~ ([Office Watch 2024](https://office-watch.com/2024/microsoft-publisher-is-ending-what-you-need-to-know)).
  Whether LTSC 2024 specifically includes Publisher: not addressed by that source ?.
  Wikipedia lists an "Office 2024 (LTSC)" final Publisher build (2408), which suggests
  some LTSC 2024 availability ~ (conflicting; unconfirmed ?).

## Microsoft 365 / Current Channel

- Always the newest Publisher, Windows only ~ (search results of Microsoft marketing page).
- Microsoft Message Center MC1469564: Publisher no longer available to Microsoft 365
  subscribers from 1 Oct 2026; removal starts on Current Channel with version 2611
  (mid-Nov 2026) and Monthly Enterprise Channel in early Dec 2026 ~
  ([Message Center mirror](https://mc.merill.net/message/MC1469564); mirror of a
  Microsoft source, not Microsoft itself).
- Microsoft 365 builds are not documented as adding Publisher features after 2013 ?.

## Retirement

- Announced: Wikipedia says 15 Feb 2024 ~
  ([Wikipedia](https://en.wikipedia.org/wiki/Microsoft_Publisher)); Microsoft's own
  announcement page was not located by date ?.
- Dates ✔ ([Microsoft support notice](https://support.microsoft.com/en-US/publisher)):
  - 1 Oct 2026: Publisher no longer available to Microsoft 365 subscribers.
  - 13 Oct 2026: perpetual support ends (Office LTSC 2021 and consumer Office 2021).
  - Affected: Publisher for Microsoft 365, 2021, 2019, 2016, 2013, 2010, 2007.
- Microsoft's advice for `.pub` files ✔ (same page): convert before 1 Oct 2026. To view,
  Save As PDF. To edit, PDF then open in Word (layout may change). Third-party converters
  are unsupported. Microsoft supplies `Convert-PubFileToPDF.ps1` (needs licensed
  Publisher installed).
- Files themselves have no deadline; only the software does ~
  ([Office Watch 2024](https://office-watch.com/2024/microsoft-publisher-is-ending-what-you-need-to-know)).
  After 13 Oct 2026 no security fixes for any version ~.
- Microsoft's FAQ mentions Affinity as a third party that may import `.pub` ✔; LibreOffice
  and Scribus import `.pub` ~.

## Change table

| Change | First version | Affects file format? | KernPage today (match 2010) | Suggested action |
|---|---|---|---|---|
| Colour modes (CMYK/spot/B&W) removed, files forced to RGB | 2013 ✔ | Yes: 2013+ files carry no colour mode; 2010 files with a mode are converted on open ✔ | not checked | Keep reading 2010 colour-mode data; do not require it when writing |
| Shared "2013+" `.pub` format (claimed) | 2013 ~ | Claimed yes; no Microsoft confirmation ? | not checked | Obtain a real 2013+ file and diff against a 2010 file before deciding |
| Cross-version compatibility 2003-2013 (claimed direct) | 2013 ~ | Contradicts the claim above ? | not checked | Test with real files |
| Save As Publisher 98 / 2000 | present in 2010 ✔ | Yes (older formats) | not checked | Out of scope unless needed |
| Picture move/swap, picture and text effects (shadow, glow, bevel, 3-D) | 2013 ✔ | Probably (effects in Escher properties) ? | not checked | Look for effect properties in 2013 sample files |
| Online picture search (Office.com, web, SkyDrive) | 2013 ~ | No | not checked | Low priority; local file insert is enough |
| Redesigned Backstage / start screen | 2013 ~ | No | not checked | UI polish later |
| New templates | 2013 ~ | No | not checked | Project cannot ship Microsoft templates |
| No documented feature changes 2016 / 2019 / 2021 | 2016-2021 ~ | Unknown ? | not checked | Nothing to add from sources |
| Publisher removed from Office 2024 (LTSC 2024 unclear) | 2024 ~ | No | not checked | Informational |
| Retirement: Microsoft 365 1 Oct 2026, perpetual 13 Oct 2026 | 2026 ✔ | No | not checked | Position project as a `.pub` viewer/editor for post-retirement users |
| Microsoft guidance: convert to PDF, then Word | 2026 ✔ | No | not checked | Validate PDF export fidelity against Publisher PDFs |

## Not found / open questions

- Any Microsoft statement of the `.pub` version number or of 2010-vs-2013 forward
  compatibility. Best next step: get a 2013+ `.pub` and open it in Publisher 2010 and in
  our parser.
- The actual Publisher 2013 Save As type list.
- Publisher-specific content of Office 2010 SP2 or the final 2010 updates.
- Whether LTSC 2024 ships Publisher.
- Microsoft's original announcement date and text.
