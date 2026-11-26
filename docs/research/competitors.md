# Competitive landscape for KernPage

*Research date: 2026-11-12. Prices and versions change; every price below should be re-checked before it is quoted publicly. Items marked **(verify)** could not be confirmed from a public source during this research. Complaint summaries come from review sites and forums and are the reviewers' opinions.*

## 1. The moment: Publisher is ending

- Microsoft ends Publisher in **October 2026**. Microsoft's final notice (message centre item MC1469564) says Publisher will no longer be included with Microsoft 365 subscriptions after that date; most sources give **1 October 2026** for subscribers, and one gives **13 October 2026** as end of support for the perpetual Office 2021 edition **(verify exact dates)**. Sources: [MC716267](https://mc.merill.net/message/MC716267), [MC1469564](https://mc.merill.net/message/MC1469564), [Windows Report](https://windowsreport.com/microsoft-gives-publisher-users-a-final-october-deadline/), [Office Watch](https://office-watch.com/2026/microsoft-publisher-is-ending-what-you-need-to-know/).
- Subscribers lose the ability to open and edit `.pub` files in Publisher; the files themselves are not deleted or converted. [W&M IT notice](https://www.wm.edu/offices/it/announcements/microsoft-publisher-retiring-oct-2026.php), [Neowin](https://www.neowin.net/reports/people-are-furious-that-microsoft-is-killing-off-publisher/).
- **Microsoft's advice:** use **Word**, **PowerPoint** and **Microsoft Create/Designer templates**; convert existing files to PDF (or Word) before the deadline; bulk-export to PDF with automation. Microsoft itself notes that Word and PowerPoint do not reproduce every part of the desktop publishing workflow. Sources: [Neowin on Microsoft's support page](https://www.neowin.net/amp/microsoft-updates-support-page-with-alternatives-to-the-soon-to-be-retired-publisher/), [Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/5855781/how-can-i-continue-to-use-microsoft-publisher-afte). One Microsoft Q&A commenter says there is no migration path for organisations holding thousands of editable `.pub` files (a user's view, not an official statement).
- **How many users are stranded?** No public figure was found. Publisher has shipped inside Microsoft 365 Business and Home/Family plans and in Office 2021 for years, so the number is large but unmeasured. Do not quote a number in marketing. What can be said safely: there is a deadline, Microsoft's answer is "use Word or PowerPoint", users are publicly angry about it ([Neowin](https://www.neowin.net/reports/people-are-furious-that-microsoft-is-killing-off-publisher/)), and nobody offers a drop-in replacement that opens and saves `.pub`.
- Practical fact: the perpetual Publisher 2016/2019/2021 keep running after October 2026, just without security updates, and Microsoft 365 users who need it must keep a perpetual copy. KernPage's opportunity is the large group that wants to *keep editing* old files.

## 2. Competitors at a glance

| Product | Price / licence | Platforms | `.pub` support | In one line |
|---|---|---|---|---|
| Scribus | Free; GPL-2.0-or-later | Windows, Mac, Linux | Import only, via libmspub | Powerful professional DTP, steep learning curve |
| LibreOffice Draw | Free; MPL-2.0 | Windows, Mac, Linux | Import only (via libmspub; Publisher 98-2010 officially); no save to `.pub` | Free and offline; basic layout |
| Affinity (by Canva) | Free (Affinity Layout in the 2025 relaunch); optional paid Canva AI features | Windows, Mac | No `.pub` support found | Modern pro layout, free, closed source, needs a Canva account |
| Canva | Free tier; Pro about 144 USD/year (re-check) | Web, apps | No `.pub` | Template-driven design, online |
| VivaDesigner | Freemium, one-off purchase (re-check) | Windows, Mac, Linux | None found **(verify)** | Cross-platform DTP, small user base |
| Swift Publisher | About 20 USD one-off (v5) **(verify v6)** | Mac only | None | Simple Mac publishing |
| Marq (Lucidpress) | Free tier; paid from about 10 USD/month (third-party figure) | Web | None | Brand-template platform for teams |
| QuarkXPress, Adobe InDesign | Commercial (Quark one-off or subscription; InDesign subscription) | Windows, Mac | None | Industry standard print layout |
| Microsoft Word / PowerPoint | Part of Microsoft 365 | Windows, Mac, web | Cannot open `.pub` | Microsoft's recommendation |

## 3. Details per product

### Scribus
- **Licence/price:** free, [GPL-2.0-or-later](https://en.wikipedia.org/wiki/Scribus); stable 1.6.x with a 1.7 development series ([comparison](https://www.tutkit.com/en/comparison/layout-software/scribus)).
- **Platforms:** Windows, macOS, Linux.
- **`.pub`:** import through libmspub (written independently by the Document Liberation Project); added around Scribus 1.5 ([LibreArts, 2013](https://librearts.org/2013/04/scribus-gets-basic-support-for-microsoft-publisher-documents/), [LinuxFr](https://linuxfr.org/users/oliver_h/journaux/merci-scribus-v1-5-pour-l-import-ms-publisher)). Quality is basic; no saving to `.pub`.
- **Strengths:** real prepress features (CMYK, spot colours, ICC, PDF/X, bleed, imposition-style tools), free, long history, stores its own documented format (SLA).
- **Weaknesses / complaints:** steep learning curve and unfamiliar interface for newcomers ([G2](https://www.g2.com/products/scribus/reviews?page=3), [Capterra](https://www.capterra.com/p/230120/Scribus/reviews)); no mail merge or Publisher-style templates **(verify)**; dated look.

### LibreOffice Draw (and libmspub)
- **Licence/price:** free, open source.
- **`.pub`:** opens Publisher files through libmspub; officially versions 98 to 2010 **(verify)**; cannot save back to `.pub`; edits go to ODF or PDF. Linked text boxes and automatic text wrap are reported weak, missing images happen in some 2013 files ([bug 121800](https://www.libreoffice.org/bugzilla/show_bug.cgi?id=121800), [bug 67299](https://bugs.libreoffice.org/show_bug.cgi?id=67299)). Coverage after the retirement: [Born City](https://borncity.com/news/microsoft-publisher-eingestellt-libreoffice-draw-oeffnet-nur-dateien-bis-2010/), [Cybernews](https://cybernews.com/tech/microsoft-publisher-libreoffice/), [Office Watch](https://office-watch.com/?p=77956).
- **Strengths:** free, offline, familiar office feel, exports PDF.
- **Weaknesses:** Draw is a drawing tool more than a page-layout tool; no real master-page/linked-story workflow like Publisher; fidelity "check every page"; no write-back to `.pub`.
- **Takeaway:** libmspub is the best existing reading code and a useful cross-check; KernPage can exceed it by also *saving* and by supporting 2013-2021 files.

### Affinity (by Canva)
- **Price/status:** in late 2025 the standalone Affinity V2 apps were replaced by a single free app combining photo, vector and layout, for Windows and Mac; the free app needs a (free) Canva account; optional AI features need a paid Canva plan; V2 is no longer sold and its update future is unstated ([DigitalProduction](https://digitalproduction.com/2025/10/30/affinity-free-all-in-one-design-suite/), [Engadget](https://www.engadget.com/apps/affinity-resurfaces-as-an-all-in-one-illustration-photo-editing-and-layout-app-204833289.html), [ToolRadar](https://toolradar.com/tools/affinity-by-canva)). **(verify current state and terms)**
- **`.pub`:** none found; can import InDesign IDML/INDD per a comparison site.
- **Strengths:** polished, fast, professional layout and typography, free.
- **Weaknesses:** closed source, vendor owned by a SaaS company (account, AI upsell, risk that terms change), no Linux, no `.pub`, not Publisher-like.
- **Complaints:** users worry about the account requirement and the business model after the buyout **(verify with forum sources)**.

### Canva
- Free tier; Pro about 144 USD/year for one person (down from 180 in September 2026 per one tracker; sources disagree) ([Piktochart](https://piktochart.com/blog/canva-pricing-2026-costs-raised/), [Pricing tracker](https://pricingsaas.com/companies/canva/diffs/2026D254)).
- Web-based, template-first; strong for social posts and flyers; no `.pub`; print tools exist but limited prepress control; requires an account and internet; user content and AI features raise privacy questions. Often named as the replacement for simple Publisher jobs ([Krumzi guide](https://www.krumzi.com/blog/microsoft-publisher-alternative)).

### VivaDesigner
- Freemium with a free version and a one-off purchase; runs on Windows, macOS (Rosetta on Apple Silicon) and Linux; v11.5 July 2026 ([Capterra](https://www.capterra.co.uk/software/1056296/vivadesigner), [comparison](https://www.tutkit.com/en/comparison/layout-software/vivadesigner)). Closed source, small community, `.pub` not supported **(verify)**.

### Swift Publisher
- Mac only; about 20 USD (version 5) with an Extras pack; sold via the Mac App Store; one-time purchase ([Software Advice](https://www.softwareadvice.com/product/477285-Swift-Publisher/), [Swift Publisher's own guide](https://www.swiftpublisher.com/useful-articles/ms-publisher-discontinued-try-these-alternatives-for-mac)). Not an option for Windows users; no `.pub` **(verify)**.

### Marq (Lucidpress)
- Web-based brand-template service for teams; free tier and paid plans (about 10 USD/month in one third-party list). No `.pub`. Subscription and cloud-bound **(verify current plans)**.

### Others
- **QuarkXPress** and **Adobe InDesign**: industry standards, paid, no `.pub`; InDesign is subscription-only.
- **Word and PowerPoint:** Microsoft's own recommendation; labels and mail merge are the strong point in Word; layout control is weaker ([Microsoft Q&A](https://learn.microsoft.com/answers/a/12746694)).
- **Open-source or free DTP projects** to examine further **(verify)**: Scribus (above), LibreOffice Draw, Inkscape (vector, single page, SVG; not DTP), Gravit Designer / Corel Vector (free tier), Penpot (open-source design tool, web), Sozi/others are not relevant. The honest conclusion: **no free, open-source, Windows-first, Publisher-like application exists.** That is the gap.

## 4. What Publisher users actually need (from the sources above)

1. Open the old `.pub` files and edit them, not just look at them.
2. Mail merge, labels, catalogue merge; Microsoft's advice is "use Word" for these.
3. Familiar ribbon, text boxes, linked stories, master pages, simple templates.
4. Reliable print output and PDF, including for a commercial printer.
5. No subscription, no account, runs offline on a typical office or school PC.

## 5. Where KernPage can be the best

Ranked by (importance to stranded users) x (how little the competition offers). Parity with Publisher 2021 comes first (ROADMAP Phase 1); these are what makes KernPage worth choosing *over* the alternatives.

1. **Faithful `.pub` open *and save*, for Publisher 98 through 2021.** Why: it is the single thing nobody else does; LibreOffice and Scribus only import, and quality is "check every page" ([Born City](https://borncity.com/news/microsoft-publisher-eingestellt-libreoffice-draw-oeffnet-nur-dateien-bis-2010/)). Microsoft offers no migration path ([Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/5855781/how-can-i-continue-to-use-microsoft-publisher-afte)). Proof KernPage can give: a public fidelity report (page-by-page image comparison against real Publisher output on a shared sample set, and the `.pub` files round-trip into Publisher 2021 without repair prompts).
2. **Batch `.pub` to PDF (and to images, text and a searchable index), command line and a drag-and-drop window.** Why: organisations have folders of thousands of files and Microsoft's own advice is a home-made PowerShell script that needs Publisher installed. KernPage's `pub2pdf` already exists; add folder recursion, parallelism, a log of problems, and the same results on machines without Office. Evidence: [Microsoft Q&A bulk export advice](https://learn.microsoft.com/answers/a/12746694), no-migration-path comment above.
3. **Print-shop-ready output: PDF/X-1a/X-4, CMYK and spot colours, bleed and crop marks, font embedding and a preflight report.** Why: Publisher's PDF/X support is limited and Scribus is the only free tool with real prepress features but is hard to learn; Affinity does it but closed and account-bound **(verify Publisher's PDF/X support)**. This makes KernPage the easy-to-use free tool for print jobs.
4. **Accessible, tagged PDF (PDF/UA), reading order, alt text for pictures and an accessibility checker.** Why: schools, councils and non-profits increasingly must publish accessible documents; most free DTP tools produce untagged PDFs **(verify per tool)**. Differentiator Microsoft never finished in Publisher.
5. **Local, private and account-free by design: no telemetry, no cloud requirement, offline spelling and offline translation (Hunspell, Bergamot models), AI-free by default.** Why: Canva and Affinity require accounts and push AI; Marq is cloud-bound. Mozilla's Bergamot models run fully on the computer ([Project Bergamot](https://en.wikipedia.org/wiki/Project_Bergamot)). Pair this with the Launcher's honest privacy page (see `launcher-and-addons.md`).
6. **Mail merge and catalogue merge that works with CSV and Excel files, with labels, envelopes and name-tag templates.** Why: Word is Microsoft's answer for merge ([Microsoft support](https://support.microsoft.com/en-us/topic/d06aaf48-9d55-4420-a138-fe581aac344a), [Krumzi](https://www.krumzi.com/blog/microsoft-publisher-alternative)); no free DTP tool has a Publisher-grade merge **(verify Scribus)**. Add repeatable data-driven layouts (a directory, a menu, a calendar) that Publisher never had.
7. **A real, open template and add-on library through the Launcher** (flyers, newsletters, bulletins, calendars, labels), all freely licensed, with an easy "Available to install" catalogue and signed packages. Why: Publisher's templates were online-only and Microsoft's online library goes away with the product; Canva's strength is templates and it is paid or account-bound; Scribus has few. Also let users keep and share their own template packs.
8. **Publisher-familiar interface plus modern essentials**: the same ribbon, shortcuts and dialogs, with also real autosave and crash recovery, unlimited undo with history list, light/dark mode, high-DPI, touch, and keyboard-only operation. Why: switching cost is the main barrier; Scribus's steep learning curve is the most repeated complaint ([G2](https://www.g2.com/products/scribus/reviews?page=3)).
9. **Open, documented formats and an exit path**: our own independent `.pub` spec published, an open native save format, export to PDF, SVG, images, IDML/ODF later. Why: Publisher users are being burned by a closed format; trust comes from being able to leave. Affinity and Canva are closed.
10. **Cross-platform later (Mac and Linux)**: Windows-first, but the Rust/egui stack allows it. Why: Swift Publisher is Mac-only, Affinity lacks Linux, Scribus covers all three but is hard to learn; a Publisher-like app on all three has no competitor. Lower priority than the items above.

### Suggested positioning
"The Publisher you can keep: opens and saves your `.pub` files, prints properly, free forever, nothing leaves your computer."

### Evidence gaps to close before any public claim
- A measured fidelity comparison on a shared sample set (vs libmspub and Publisher).
- Exact Publisher retirement dates (Microsoft's own page) and the Microsoft 365 vs Office 2021 distinction.
- Current prices and terms for Affinity, Canva, VivaDesigner, Swift Publisher and Marq (read each vendor's pricing page).
- What PDF/X and tagged-PDF support Publisher 2021 actually has.
- Real user complaint threads (forums, reviews): this research found mostly vendor guides, review-site comments and Microsoft Q&A; direct community sources should be added.
