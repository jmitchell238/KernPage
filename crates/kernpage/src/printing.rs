//! Direct printing (File > Print): the page-range parser, the print request, and the Windows
//! back end that sends the pages to a printer.
//!
//! The pages come from the same PDF that File > Save As PDF creates. On Windows each chosen
//! page is drawn at the printer's resolution by the system's own PDF renderer
//! (`Windows.Data.Pdf`, public API, Windows 10 and later) in horizontal strips, so even a
//! 600 dpi page never needs one huge bitmap, and each strip is sent with `StretchDIBits`
//! between `StartPage` and `EndPage`. The whole job runs on a worker thread; the Print page
//! shows its progress. Everything that touches the operating system is behind
//! [`list_printers`], [`printer_properties`] and [`submit`], which tests replace with recorders.

use std::sync::{Arc, Mutex};

/// Which pages the Print page's "Pages" dropdown means.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PageScope {
    #[default]
    All,
    Current,
    Custom,
}

impl PageScope {
    pub fn label(self) -> &'static str {
        match self {
            PageScope::All => "Print All Pages",
            PageScope::Current => "Print Current Page",
            PageScope::Custom => "Custom Print",
        }
    }
}

/// Everything the Print page decided: what a click on Print asks the system to do.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PrintJob {
    pub printer: String,
    pub copies: u32,
    /// 0-based page numbers, in print order, each once.
    pub pages: Vec<usize>,
    pub collate: bool,
    pub landscape: bool,
    pub doc_name: String,
    /// The printer's own settings (a DEVMODE) from Printer Properties, if the user opened it.
    pub devmode: Option<Vec<u8>>,
}

/// Parse the text of the "Pages:" box ("1-3, 5") against a publication of `total` pages.
/// Returns the 0-based page numbers, ascending and without repeats. A range that runs past the
/// last page is cut short and a page wholly past the end is ignored; if nothing is left, or the
/// text is not a list of pages and ranges, the error is the message to show.
pub fn parse_page_ranges(text: &str, total: usize) -> Result<Vec<usize>, String> {
    let how = "Enter page numbers and ranges separated by commas, for example 1-3, 5.";
    if text.trim().is_empty() {
        return Err(format!("There are no pages to print.\n\n{how}"));
    }
    let number = |s: &str| -> Result<u64, String> {
        let s = s.trim();
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("\"{}\" is not a valid page number.\n\n{how}", s));
        }
        // A number too long for u64 is certainly past the end of the publication.
        Ok(s.parse::<u64>().unwrap_or(u64::MAX))
    };
    let mut pages = Vec::new();
    for part in text.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            return Err(format!("The page list has an empty entry.\n\n{how}"));
        }
        let (from, to) = match part.split_once('-') {
            Some((a, b)) => (number(a)?, number(b)?),
            None => {
                let n = number(part)?;
                (n, n)
            }
        };
        if from == 0 || to == 0 {
            return Err(format!("Page numbers start at 1.\n\n{how}"));
        }
        if from > to {
            return Err(format!("\"{part}\" is not a valid range: the first page is after the last.\n\n{how}"));
        }
        let last = to.min(total as u64);
        if from <= last {
            pages.extend((from..=last).map(|n| n as usize - 1));
        }
    }
    pages.sort_unstable();
    pages.dedup();
    if pages.is_empty() {
        return Err(if total == 1 {
            "The publication has only 1 page. Enter 1.".to_owned()
        } else {
            format!("The publication has only {total} pages. Enter page numbers from 1 to {total}.")
        });
    }
    Ok(pages)
}

/// The printers installed on this computer and the name of the default one.
#[derive(Clone, Default, Debug)]
pub struct PrinterList {
    pub names: Vec<String>,
    pub default: Option<String>,
}

/// Progress of a running job, shared with its worker thread.
#[derive(Default, Debug)]
pub struct Progress {
    pub sheet: usize,
    pub sheets: usize,
    pub finished: bool,
    pub error: Option<String>,
}

pub type Shared = Arc<Mutex<Progress>>;

/// Sheets a job prints: every page, once per copy.
pub fn sheet_count(job: &PrintJob) -> usize {
    job.pages.len() * job.copies.max(1) as usize
}

/// The page order for a job: collated prints the whole range once per copy, uncollated prints
/// each page `copies` times in a row.
pub fn sheet_order(job: &PrintJob) -> Vec<usize> {
    let copies = job.copies.max(1) as usize;
    if job.collate {
        (0..copies).flat_map(|_| job.pages.iter().copied()).collect()
    } else {
        job.pages.iter().flat_map(|&p| std::iter::repeat_n(p, copies)).collect()
    }
}

pub fn list_printers() -> PrinterList {
    #[cfg(test)]
    {
        test_os::printers()
    }
    #[cfg(not(test))]
    {
        list_printers_now()
    }
}

/// Printer Properties: the printer's own dialog. Returns the chosen settings, or None if the
/// user cancelled or the printer has no dialog.
pub fn printer_properties(printer: &str, current: Option<&[u8]>, landscape: bool) -> Option<Vec<u8>> {
    #[cfg(test)]
    {
        let _ = (current, landscape);
        test_os::PROPS.with(|p| p.borrow_mut().push(printer.to_owned()));
        None
    }
    #[cfg(not(test))]
    {
        printer_properties_now(printer, current, landscape)
    }
}

/// Start a job on a worker thread; `repaint` is called whenever the progress changes.
pub fn submit(job: PrintJob, pdf: Vec<u8>, repaint: impl Fn() + Send + 'static) -> Shared {
    let progress: Shared = Arc::new(Mutex::new(Progress { sheets: sheet_count(&job), ..Progress::default() }));
    #[cfg(test)]
    {
        let _ = (pdf, repaint);
        test_os::PRINTED.with(|p| p.borrow_mut().push(job));
        if let Ok(mut p) = progress.lock() {
            p.sheet = p.sheets;
            p.finished = true;
        }
    }
    #[cfg(not(test))]
    {
        let shared = progress.clone();
        std::thread::spawn(move || {
            let result = print_now(&job, &pdf, &shared, &repaint);
            if let Ok(mut p) = shared.lock() {
                p.finished = true;
                p.error = result.err();
            }
            repaint();
        });
    }
    progress
}

/// Test seam for the operating system: the printers on offer, and what would have been printed.
#[cfg(test)]
pub mod test_os {
    use super::{PrintJob, PrinterList};
    use std::cell::RefCell;
    thread_local! {
        pub(super) static PRINTED: RefCell<Vec<PrintJob>> = const { RefCell::new(Vec::new()) };
        pub(super) static PROPS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    pub fn printers() -> PrinterList {
        PrinterList {
            names: vec!["Test Laser".to_owned(), "Test Inkjet".to_owned()],
            default: Some("Test Inkjet".to_owned()),
        }
    }
    /// Every job the app asked the system to print since the last call.
    pub fn take_printed() -> Vec<PrintJob> {
        PRINTED.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }
    /// Every printer whose Printer Properties dialog was asked for since the last call.
    pub fn take_properties() -> Vec<String> {
        PROPS.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }
}

#[cfg(not(test))]
fn print_now(job: &PrintJob, pdf: &[u8], progress: &Shared, repaint: &dyn Fn()) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::print(job, pdf, progress, repaint)
    }
    #[cfg(not(windows))]
    {
        let _ = (job, pdf, progress, repaint);
        Err("Printing directly is only available on Windows. Use \"Open as PDF instead\".".to_owned())
    }
}

#[cfg(not(test))]
fn list_printers_now() -> PrinterList {
    #[cfg(windows)]
    {
        win::list_printers()
    }
    #[cfg(not(windows))]
    {
        PrinterList::default()
    }
}

#[cfg(not(test))]
fn printer_properties_now(printer: &str, current: Option<&[u8]>, landscape: bool) -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        win::properties(printer, current, landscape)
    }
    #[cfg(not(windows))]
    {
        let _ = (printer, current, landscape);
        None
    }
}

/// Whether a page of this (width, height) is printed in landscape orientation.
pub fn is_landscape<T: PartialOrd>(size: Option<(T, T)>) -> bool {
    size.is_some_and(|(w, h)| w > h)
}
#[cfg(windows)]
#[cfg_attr(test, allow(dead_code))]
pub(crate) mod win {
    use super::{sheet_order, PrintJob, PrinterList, Shared};
    use std::ptr::{null, null_mut};
    use windows::core::Interface;
    use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
    use windows::Foundation::Rect;
    use windows::Storage::Streams::{IOutputStream, DataReader, DataWriter, InMemoryRandomAccessStream};
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Graphics::Printing::*;
    use windows_sys::Win32::Storage::Xps::*;

    /// The most resolution the PDF renderer is asked for; the printer scales the rest.
    const MAX_RENDER_DPI: f64 = 600.0;
    /// Rows drawn and sent at a time.
    const STRIP_ROWS: u32 = 768;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn from_wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        // SAFETY: the spooler returns NUL-terminated names.
        unsafe {
            while *p.add(n) != 0 {
                n += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
        }
    }

    pub fn list_printers() -> PrinterList {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut needed, mut count) = (0u32, 0u32);
        // SAFETY: the first call only asks for the buffer size; the second fills our buffer.
        unsafe {
            EnumPrintersW(flags, null(), 4, null_mut(), 0, &mut needed, &mut count);
        }
        let mut names = Vec::new();
        if needed > 0 {
            let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
            // SAFETY: `buf` is at least `needed` bytes and 8-aligned.
            let ok = unsafe {
                EnumPrintersW(flags, null(), 4, buf.as_mut_ptr().cast(), needed, &mut needed, &mut count)
            };
            if ok != 0 {
                let infos = buf.as_ptr().cast::<PRINTER_INFO_4W>();
                for i in 0..count as usize {
                    // SAFETY: the spooler wrote `count` PRINTER_INFO_4W records into `buf`.
                    let name = from_wide(unsafe { (*infos.add(i)).pPrinterName });
                    if !name.is_empty() {
                        names.push(name);
                    }
                }
            }
        }
        names.sort_by_key(|n| n.to_lowercase());
        let mut len = 0u32;
        // SAFETY: size query, then fill.
        unsafe { GetDefaultPrinterW(null_mut(), &mut len) };
        let default = if len > 1 {
            let mut b = vec![0u16; len as usize];
            // SAFETY: `b` holds `len` UTF-16 units.
            (unsafe { GetDefaultPrinterW(b.as_mut_ptr(), &mut len) } != 0).then(|| from_wide(b.as_ptr()))
        } else {
            None
        };
        PrinterList { names, default }
    }

    /// A printer's DEVMODE: its defaults (or `base`), set to the wanted orientation and one copy.
    /// With `prompt` the printer's own Properties dialog edits it first.
    fn devmode(printer: &str, base: Option<&[u8]>, landscape: bool, prompt: bool) -> Option<Vec<u8>> {
        let name = wide(printer);
        let mut h: PRINTER_HANDLE = unsafe { std::mem::zeroed() };
        // SAFETY: plain spooler calls with buffers sized by the spooler's own answer.
        unsafe {
            if OpenPrinterW(name.as_ptr().cast_mut(), &mut h, null()) == 0 {
                return None;
            }
            let size = DocumentPropertiesW(null_mut(), h, name.as_ptr().cast_mut(), null_mut(), null(), 0);
            if size <= 0 {
                ClosePrinter(h);
                return None;
            }
            let mut out = vec![0u64; (size as usize).div_ceil(8)];
            let mut input = vec![0u64; (size as usize).div_ceil(8)];
            let mut mode = DM_OUT_BUFFER;
            if let Some(b) = base.filter(|b| b.len() <= size as usize) {
                std::ptr::copy_nonoverlapping(b.as_ptr(), input.as_mut_ptr().cast::<u8>(), b.len());
                mode |= DM_IN_BUFFER;
            }
            if prompt {
                mode |= DM_IN_PROMPT;
            }
            let r = DocumentPropertiesW(
                null_mut(),
                h,
                name.as_ptr().cast_mut(),
                out.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                mode,
            );
            ClosePrinter(h);
            if r != 1 {
                // IDOK; anything else is Cancel or a failure.
                return None;
            }
            let dm = out.as_mut_ptr().cast::<DEVMODEW>();
            (*dm).dmFields |= DM_ORIENTATION | DM_COPIES;
            (*dm).Anonymous1.Anonymous1.dmOrientation =
                if landscape { DMORIENT_LANDSCAPE as i16 } else { DMORIENT_PORTRAIT as i16 };
            // This program prints every copy itself, so the driver prints one.
            (*dm).Anonymous1.Anonymous1.dmCopies = 1;
            Some(std::slice::from_raw_parts(out.as_ptr().cast::<u8>(), size as usize).to_vec())
        }
    }

    pub fn properties(printer: &str, current: Option<&[u8]>, landscape: bool) -> Option<Vec<u8>> {
        devmode(printer, current, landscape, true)
    }

    fn err<T: std::fmt::Display>(what: &str) -> impl Fn(T) -> String + '_ {
        move |e| format!("{what} ({e})")
    }

    /// Draw page `index` of the PDF at `width` x `height` pixels, `rows` rows at a time, and give
    /// each strip (top row, rows, BGRA pixels) to `sink`.
    pub fn render_strips(
        doc: &PdfDocument,
        index: u32,
        width: u32,
        height: u32,
        sink: &mut dyn FnMut(u32, u32, &[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        let page = doc.GetPage(index).map_err(err("The PDF page could not be opened"))?;
        let size = page.Size().map_err(err("The PDF page has no size"))?;
        let mut y = 0;
        while y < height {
            let rows = STRIP_ROWS.min(height - y);
            // Windows.Data.Pdf can return a size other than the one asked for (it applies the
            // process's display scaling when the process is not DPI aware); ask again for the
            // size that makes it come back at the wanted one.
            let (mut ask_w, mut ask_h) = (width, rows);
            let mut bgra = Vec::new();
            for attempt in 0..3 {
                let opts = PdfPageRenderOptions::new().map_err(err("The PDF renderer is not available"))?;
                opts.SetDestinationWidth(ask_w).map_err(err("render"))?;
                opts.SetDestinationHeight(ask_h).map_err(err("render"))?;
                opts.SetBackgroundColor(windows::UI::Color { A: 255, R: 255, G: 255, B: 255 })
                    .map_err(err("render"))?;
                opts.SetSourceRect(Rect {
                    X: 0.0,
                    Y: size.Height * y as f32 / height as f32,
                    Width: size.Width,
                    Height: size.Height * rows as f32 / height as f32,
                })
                .map_err(err("render"))?;
                let stream = InMemoryRandomAccessStream::new().map_err(err("render"))?;
                page.RenderWithOptionsToStreamAsync(&stream, &opts)
                    .and_then(|a| a.join())
                    .map_err(err("The page could not be drawn"))?;
                let len = stream.Size().map_err(err("render"))? as usize;
                let input = stream.GetInputStreamAt(0).map_err(err("render"))?;
                let reader = DataReader::CreateDataReader(&input).map_err(err("render"))?;
                reader.LoadAsync(len as u32).and_then(|a| a.join()).map_err(err("render"))?;
                let mut png_bytes = vec![0u8; len];
                reader.ReadBytes(&mut png_bytes).map_err(err("render"))?;
                let (px, got_w, got_h) = decode_png_bgra(&png_bytes)?;
                if (got_w, got_h) == (width, rows) {
                    bgra = px;
                    break;
                }
                if attempt == 2 || got_w == 0 || got_h == 0 {
                    return Err(format!(
                        "The PDF renderer returned {got_w}x{got_h} pixels instead of {width}x{rows}."
                    ));
                }
                ask_w = (f64::from(ask_w) * f64::from(width) / f64::from(got_w)).round().max(1.0) as u32;
                ask_h = (f64::from(ask_h) * f64::from(rows) / f64::from(got_h)).round().max(1.0) as u32;
            }            sink(y, rows, &mut bgra)?;
            y += rows;
        }
        Ok(())
    }

    fn decode_png_bgra(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = dec.read_info().map_err(err("The drawn page could not be read"))?;
        let mut buf = vec![0; r.output_buffer_size().ok_or("The drawn page is too large")?];
        let info = r.next_frame(&mut buf).map_err(err("The drawn page could not be read"))?;
        let (width, rows) = (info.width, info.height);

        let ch = info.color_type.samples();
        let mut out = Vec::with_capacity((width * rows * 4) as usize);
        for px in buf[..info.buffer_size()].chunks_exact(ch) {
            let (r, g, b) = match ch {
                1 | 2 => (px[0], px[0], px[0]),
                _ => (px[0], px[1], px[2]),
            };
            out.extend_from_slice(&[b, g, r, 255]);
        }
        Ok((out, width, rows))
    }

    pub fn load_pdf(pdf: &[u8]) -> Result<PdfDocument, String> {
        // SAFETY: initialising the WinRT apartment of this worker thread; "already initialised"
        // is not an error here.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        let stream = InMemoryRandomAccessStream::new().map_err(err("The PDF renderer is not available"))?;
        let writer = DataWriter::CreateDataWriter(&stream.cast::<IOutputStream>().map_err(err("stream"))?)
            .map_err(err("The PDF renderer is not available"))?;
        writer.WriteBytes(pdf).map_err(err("The PDF could not be loaded"))?;
        writer.StoreAsync().and_then(|a| a.join()).map_err(err("The PDF could not be loaded"))?;
        writer.DetachStream().map_err(err("The PDF could not be loaded"))?;
        stream.Seek(0).map_err(err("The PDF could not be loaded"))?;
        PdfDocument::LoadFromStreamAsync(&stream)
            .and_then(|a| a.join())
            .map_err(err("Windows could not read the PDF"))
    }

    struct Dc(HDC);
    impl Drop for Dc {
        fn drop(&mut self) {
            // SAFETY: the DC was created by CreateDCW and is deleted once.
            unsafe { DeleteDC(self.0) };
        }
    }

    /// Where a PDF page (size in 1/96 inch) lands on the paper (printer pixels): the page at
    /// 100%, or scaled down keeping its aspect when it is bigger than the paper. Returns the
    /// printed size (w, h) and the size to render it at (rw, rh), capped at MAX_RENDER_DPI.
    pub(crate) fn page_layout(page: (f64, f64), dpi: (f64, f64), paper: (f64, f64)) -> (f64, f64, u32, u32) {
        let (w, h) = (page.0 / 96.0 * dpi.0, page.1 / 96.0 * dpi.1);
        let fit = (paper.0 / w).min(paper.1 / h).min(1.0);
        let (w, h) = ((w * fit).round().max(1.0), (h * fit).round().max(1.0));
        let k = (MAX_RENDER_DPI / dpi.0.max(dpi.1)).min(1.0);
        (w, h, ((w * k).round() as u32).max(1), ((h * k).round() as u32).max(1))
    }

    pub fn print(job: &PrintJob, pdf: &[u8], progress: &Shared, repaint: &dyn Fn()) -> Result<(), String> {
        let doc = load_pdf(pdf)?;
        let page_count = doc.PageCount().map_err(err("The PDF has no pages"))?;
        let dm = devmode(&job.printer, job.devmode.as_deref(), job.landscape, false)
            .ok_or_else(|| format!("KernPage cannot reach the printer \"{}\".", job.printer))?;
        let printer = wide(&job.printer);
        let driver = wide("WINSPOOL");
        // SAFETY: `dm` is a DEVMODE the driver produced; the DC is owned by `Dc`.
        let hdc = unsafe { CreateDCW(driver.as_ptr(), printer.as_ptr(), null(), dm.as_ptr().cast()) };
        if hdc.is_null() {
            return Err(format!("Windows cannot open the printer \"{}\".", job.printer));
        }
        let dc = Dc(hdc);
        let name = wide(&job.doc_name);
        let info = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: name.as_ptr(),
            lpszOutput: null(),
            lpszDatatype: null(),
            fwType: 0,
        };
        // SAFETY: valid DC and DOCINFOW.
        if unsafe { StartDocW(dc.0, &info) } <= 0 {
            return Err(format!("The printer \"{}\" did not accept the job.", job.printer));
        }
        let result = (|| {
            let caps = |c| unsafe { GetDeviceCaps(dc.0, c) };
            let (dpi_x, dpi_y) = (caps(LOGPIXELSX as i32).max(1) as f64, caps(LOGPIXELSY as i32).max(1) as f64);
            let (paper_w, paper_h) = (caps(PHYSICALWIDTH as i32) as f64, caps(PHYSICALHEIGHT as i32) as f64);
            let (off_x, off_y) = (caps(PHYSICALOFFSETX as i32), caps(PHYSICALOFFSETY as i32));
            for (n, &p) in sheet_order(job).iter().enumerate() {
                if let Ok(mut s) = progress.lock() {
                    s.sheet = n + 1;
                }
                repaint();
                if p as u32 >= page_count {
                    continue;
                }
                let size = doc.GetPage(p as u32).and_then(|pg| pg.Size()).map_err(err("The PDF page has no size"))?;
                // The PDF page size is in 1/96 inch. Print at 100%, or smaller if it is bigger
                // than the paper. The top-left of the page is the top-left of the paper.
                let (w, h, rw, rh) =
                    page_layout((f64::from(size.Width), f64::from(size.Height)), (dpi_x, dpi_y), (paper_w, paper_h));
                // SAFETY: a page is started, filled by the sink and ended on this DC.
                if unsafe { StartPage(dc.0) } <= 0 {
                    return Err("The printer stopped the job.".to_owned());
                }
                render_strips(&doc, p as u32, rw, rh, &mut |top, rows, bgra| {
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: rw as i32,
                            biHeight: -(rows as i32),
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB,
                            ..unsafe { std::mem::zeroed() }
                        },
                        bmiColors: [unsafe { std::mem::zeroed() }],
                    };
                    let y0 = (f64::from(top) * h / f64::from(rh)).round() as i32;
                    let y1 = (f64::from(top + rows) * h / f64::from(rh)).round() as i32;
                    // SAFETY: `bgra` holds rw x rows 32-bit pixels, as `bmi` says.
                    let n = unsafe {
                        StretchDIBits(
                            dc.0,
                            -off_x,
                            y0 - off_y,
                            w as i32,
                            (y1 - y0).max(1),
                            0,
                            0,
                            rw as i32,
                            rows as i32,
                            bgra.as_ptr().cast(),
                            &bmi,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        )
                    };
                    if n == 0 {
                        return Err("The printer did not accept the page.".to_owned());
                    }
                    Ok(())
                })?;
                // SAFETY: matches StartPage above.
                if unsafe { EndPage(dc.0) } <= 0 {
                    return Err("The printer stopped the job.".to_owned());
                }
            }
            Ok(())
        })();
        // SAFETY: the document was started above.
        unsafe {
            if result.is_ok() {
                EndDoc(dc.0);
            } else {
                AbortDoc(dc.0);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(pages: &[usize], copies: u32, collate: bool) -> PrintJob {
        PrintJob {
            printer: "P".into(),
            copies,
            pages: pages.to_vec(),
            collate,
            landscape: false,
            doc_name: "D".into(),
            devmode: None,
        }
    }

    #[test]
    fn ranges_and_single_pages_are_parsed() {
        assert_eq!(parse_page_ranges("1-3, 5", 10), Ok(vec![0, 1, 2, 4]));
        assert_eq!(parse_page_ranges("2", 3), Ok(vec![1]));
        assert_eq!(parse_page_ranges(" 3 ; 1 ", 3), Ok(vec![0, 2]));
        assert_eq!(parse_page_ranges("1 - 2", 3), Ok(vec![0, 1]));
    }

    #[test]
    fn pages_are_sorted_and_not_repeated() {
        assert_eq!(parse_page_ranges("5, 1-3, 2", 9), Ok(vec![0, 1, 2, 4]));
    }

    #[test]
    fn pages_past_the_end_are_cut_or_ignored() {
        assert_eq!(parse_page_ranges("2-9", 4), Ok(vec![1, 2, 3]));
        assert_eq!(parse_page_ranges("1, 7", 4), Ok(vec![0]));
        assert_eq!(parse_page_ranges("1, 99999999999999999999999", 4), Ok(vec![0]));
    }

    #[test]
    fn nothing_left_is_an_error_naming_the_page_count() {
        let e = parse_page_ranges("7", 4).unwrap_err();
        assert!(e.contains("only 4 pages"), "{e}");
        assert!(parse_page_ranges("2", 1).unwrap_err().contains("only 1 page"));
    }

    #[test]
    fn bad_text_is_an_error_with_an_example() {
        for bad in ["", "  ", "abc", "1-", "-3", "1,,2", "1-2-3", "0", "0-2", "3-1", "1.5", "1 2", "-"] {
            let e = parse_page_ranges(bad, 5).unwrap_err();
            assert!(!e.is_empty(), "{bad:?}");
        }
        assert!(parse_page_ranges("x", 5).unwrap_err().contains("1-3, 5"));
        assert!(parse_page_ranges("3-1", 5).unwrap_err().contains("first page is after the last"));
    }

    #[test]
    fn collated_copies_repeat_the_whole_range_uncollated_repeat_each_page() {
        assert_eq!(sheet_order(&job(&[0, 2], 2, true)), vec![0, 2, 0, 2]);
        assert_eq!(sheet_order(&job(&[0, 2], 2, false)), vec![0, 0, 2, 2]);
        assert_eq!(sheet_count(&job(&[0, 2], 3, true)), 6);
        assert_eq!(sheet_order(&job(&[1], 0, true)), vec![1], "zero copies means one");
    }
    #[cfg(windows)]
    #[test]
    fn letter_page_on_letter_paper_at_600_dpi_fills_the_paper_rendered_at_600() {
        use super::win::page_layout;
        // 8.5 x 11 in = 816 x 1056 (1/96 in); paper 5100 x 6600 px at 600 dpi. Drawn at
        // (-offset_x, -offset_y) in printable-area coordinates, the page origin is the paper corner.
        assert_eq!(page_layout((816.0, 1056.0), (600.0, 600.0), (5100.0, 6600.0)), (5100.0, 6600.0, 5100, 6600));
    }

    #[cfg(windows)]
    #[test]
    fn small_pages_print_at_100_percent_and_big_pages_shrink_keeping_aspect() {
        use super::win::page_layout;
        let (w, h, ..) = page_layout((528.0, 816.0), (300.0, 300.0), (2550.0, 3300.0));
        assert_eq!((w, h), (1650.0, 2550.0), "half-letter is not scaled up");
        let (w, h, ..) = page_layout((1056.0, 1632.0), (300.0, 300.0), (2550.0, 3300.0));
        assert_eq!(h, 3300.0, "tabloid on letter scales to the height");
        assert!((w / h - 11.0 / 17.0).abs() < 0.001 && w <= 2550.0, "{w}x{h}");
        let (w, _, rw, _) = page_layout((816.0, 1056.0), (1200.0, 1200.0), (10200.0, 13200.0));
        assert_eq!((w, rw), (10200.0, 5100), "1200 dpi is rendered at 600 and scaled by the printer");
    }

    #[test]
    fn wider_than_tall_publications_are_landscape() {
        assert!(is_landscape(Some((1056.0, 816.0))));
        assert!(!is_landscape(Some((816.0, 1056.0))));
        assert!(!is_landscape(Some((816.0, 816.0))));
        assert!(!is_landscape::<u32>(None));
    }
}

/// Manual check of the Windows PDF renderer: `KP_RENDER_PDF=in.pdf KP_RENDER_OUT=out.png
/// cargo test -p kernpage render_page_to_png -- --ignored` draws page 1 in strips into a PNG.
#[cfg(all(test, windows))]
mod render_check {
    #[test]
    #[ignore = "manual: needs KP_RENDER_PDF and KP_RENDER_OUT"]
    fn render_page_to_png() {
        let (Ok(input), Ok(out)) = (std::env::var("KP_RENDER_PDF"), std::env::var("KP_RENDER_OUT")) else {
            return;
        };
        let dpi: f64 = std::env::var("KP_RENDER_DPI").ok().and_then(|d| d.parse().ok()).unwrap_or(150.0);
        let doc = super::win::load_pdf(&std::fs::read(input).unwrap()).unwrap();
        let size = doc.GetPage(0).unwrap().Size().unwrap();
        let (w, h) = ((f64::from(size.Width) / 96.0 * dpi) as u32, (f64::from(size.Height) / 96.0 * dpi) as u32);
        let mut rgb = vec![0u8; (w * h * 3) as usize];
        let mut strips = 0;
        super::win::render_strips(&doc, 0, w, h, &mut |top, rows, bgra| {
            strips += 1;
            for (i, px) in bgra.chunks_exact(4).enumerate() {
                let o = (top * w) as usize * 3 + i * 3;
                rgb[o..o + 3].copy_from_slice(&[px[2], px[1], px[0]]);
            }
            let _ = rows;
            Ok(())
        })
        .unwrap();
        let mut enc = png::Encoder::new(std::fs::File::create(out).unwrap(), w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&rgb).unwrap();
        eprintln!("rendered {w}x{h} in {strips} strips");
        // The WinRT PDF objects can keep the test process from exiting; this is a manual check.
        std::process::exit(0);
    }
}
