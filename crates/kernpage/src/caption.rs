//! The window's one top bar: the quick-access row doubles as the title bar.
//!
//! On Windows the window keeps its native frame (shadow, rounded corners, Aero Snap, resize
//! edges, Alt+Space menu) but its client area is extended over the caption (`WM_NCCALCSIZE`),
//! and a window-procedure subclass answers `WM_NCHITTEST` from the rectangles the last frame
//! recorded here: empty bar space is the caption (drag, double-click to maximise), the
//! minimise / maximise / close buttons answer with their own hit codes (so the Windows 11
//! snap-layout flyout appears over the maximise button) and widgets stay client area.
//! egui draws the three caption buttons. Windows delivers the mouse over them as non-client
//! messages, so the subclass keeps their hover and pressed state ([`nc_hover`], [`nc_pressed`])
//! and runs the action; when egui receives the clicks itself (tests, other systems) the same
//! buttons send `ViewportCommand`s.

use eframe::egui::{self, Color32, Context, Pos2, Rect, Stroke, Ui, ViewportCommand};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

/// Height of the bar, logical px.
pub const BAR_H: f32 = 32.0;
/// Width of each caption button, logical px.
pub const BUTTON_W: f32 = 46.0;

// Hit-test codes (the values of the Win32 `HT*` constants).
pub const HTCLIENT: u32 = 1;
pub const HTCAPTION: u32 = 2;
pub const HTMINBUTTON: u32 = 8;
pub const HTMAXBUTTON: u32 = 9;
pub const HTTOP: u32 = 12;
pub const HTTOPLEFT: u32 = 13;
pub const HTTOPRIGHT: u32 = 14;
pub const HTCLOSE: u32 = 20;

/// One of the three caption buttons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Minimize = 1,
    Maximize = 2,
    Close = 3,
}

impl Button {
    fn from_u8(v: u8) -> Option<Button> {
        match v {
            1 => Some(Button::Minimize),
            2 => Some(Button::Maximize),
            3 => Some(Button::Close),
            _ => None,
        }
    }
}

/// Where things are in the bar, as laid out by the last frame (logical px, client coordinates).
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub ppp: f32,
    /// Bottom edge of the bar.
    pub bar_h: f32,
    /// Quick-access buttons, the search box: clicks here belong to egui.
    pub interactive: Vec<Rect>,
    pub minimize: Rect,
    pub maximize: Rect,
    pub close: Rect,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            ppp: 1.0,
            bar_h: 0.0,
            interactive: Vec::new(),
            minimize: Rect::NOTHING,
            maximize: Rect::NOTHING,
            close: Rect::NOTHING,
        }
    }
}

/// What Windows should treat the point as. `pos` is in client pixels, `border_px` is the width
/// of the top resize edge.
pub fn hit_test(pos: Pos2, l: &Layout, maximised: bool, client_w_px: f32, border_px: f32) -> u32 {
    let ppp = l.ppp.max(0.1);
    if !maximised && pos.y < border_px {
        return if pos.x < border_px {
            HTTOPLEFT
        } else if pos.x >= client_w_px - border_px {
            HTTOPRIGHT
        } else {
            HTTOP
        };
    }
    let p = Pos2::new(pos.x / ppp, pos.y / ppp);
    if p.y < 0.0 || p.y >= l.bar_h {
        return HTCLIENT;
    }
    if l.minimize.contains(p) {
        return HTMINBUTTON;
    }
    if l.maximize.contains(p) {
        return HTMAXBUTTON;
    }
    if l.close.contains(p) {
        return HTCLOSE;
    }
    if l.interactive.iter().any(|r| r.contains(p)) {
        return HTCLIENT;
    }
    HTCAPTION
}

static LAYOUT: Mutex<Option<Layout>> = Mutex::new(None);
static HOVER: AtomicU8 = AtomicU8::new(0);
static PRESSED: AtomicU8 = AtomicU8::new(0);

#[cfg(test)]
thread_local! {
    static LAST: std::cell::RefCell<Layout> = std::cell::RefCell::new(Layout::default());
    static SENT: std::cell::RefCell<Vec<ViewportCommand>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Store this frame's layout for the window procedure.
pub fn publish(l: Layout) {
    #[cfg(test)]
    LAST.with(|c| *c.borrow_mut() = l.clone());
    if let Ok(mut g) = LAYOUT.lock() {
        *g = Some(l);
    }
}

/// The recorded layout (the window procedure's view).
#[cfg(any(windows, test))]
pub fn current() -> Layout {
    LAYOUT.lock().ok().and_then(|g| g.clone()).unwrap_or_default()
}

/// Test seam: the layout published by this thread's last frame.
#[cfg(test)]
pub fn last_layout() -> Layout {
    LAST.with(|c| c.borrow().clone())
}

/// Test seam: the viewport commands the caption buttons sent on this thread.
#[cfg(test)]
pub fn take_sent() -> Vec<ViewportCommand> {
    SENT.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// The button the system reports the mouse over (non-client messages).
pub fn nc_hover() -> Option<Button> {
    Button::from_u8(HOVER.load(Ordering::Relaxed))
}

/// The button the system reports pressed.
pub fn nc_pressed() -> Option<Button> {
    Button::from_u8(PRESSED.load(Ordering::Relaxed))
}

fn send(ctx: &Context, cmd: ViewportCommand) {
    #[cfg(test)]
    SENT.with(|c| c.borrow_mut().push(cmd.clone()));
    ctx.send_viewport_cmd(cmd);
}

/// Draw the three caption buttons at the right end of `bar` (full height, 46 px wide each) and
/// return their rectangles (minimise, maximise, close). Clicks egui receives send the matching
/// viewport command.
pub fn show_buttons(ui: &mut Ui, bar: Rect) -> [Rect; 3] {
    let ppp = ui.ctx().pixels_per_point();
    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    let snap = |v: f32| (v * ppp).round() / ppp;
    // Pixel-snapped so fills and glyphs are crisp at any scale.
    let top = snap(bar.min.y);
    let bottom = snap(bar.max.y);
    let right = snap(bar.max.x);
    let at = |i: f32| {
        Rect::from_min_max(
            Pos2::new(snap(right - (3.0 - i) * BUTTON_W), top),
            Pos2::new(snap(right - (2.0 - i) * BUTTON_W), bottom),
        )
    };
    let rects = [at(0.0), at(1.0), at(2.0)];
    let kinds = [Button::Minimize, Button::Maximize, Button::Close];
    let names = ["Minimize", "Maximize", "Close"];
    for i in 0..3 {
        let kind = kinds[i];
        let name = if kind == Button::Maximize && maximized { "Restore Down" } else { names[i] };
        let resp = ui
            .interact(rects[i], ui.id().with(("caption", i)), egui::Sense::click())
            .on_hover_text(name);
        let hovered = resp.hovered() || nc_hover() == Some(kind);
        let pressed = resp.is_pointer_button_down_on() || nc_pressed() == Some(kind);
        let close = kind == Button::Close;
        let mut glyph = crate::theme::text();
        if hovered || pressed {
            let fill = if close {
                let red = Color32::from_rgb(0xC4, 0x2B, 0x1C);
                glyph = Color32::WHITE;
                if pressed { crate::theme::mix(red, Color32::BLACK, 0.12) } else { red }
            } else if pressed {
                crate::theme::mix(crate::theme::hover_bg(), crate::theme::text(), 0.10)
            } else {
                crate::theme::hover_bg()
            };
            ui.painter().rect_filled(rects[i], 0.0, fill);
        }
        paint_glyph(ui, rects[i], kind, maximized, glyph, ppp);
        if resp.clicked() {
            match kind {
                Button::Minimize => send(ui.ctx(), ViewportCommand::Minimized(true)),
                Button::Maximize => send(ui.ctx(), ViewportCommand::Maximized(!maximized)),
                Button::Close => send(ui.ctx(), ViewportCommand::Close),
            }
        }
        #[cfg(test)]
        crate::widgets::probe::record(crate::widgets::probe::Seen {
            label: format!("caption:{}", names[i]),
            tip: name.to_owned(),
            rect: rects[i],
            enabled: true,
            selected: false,
        });
    }
    rects
}

/// The centre of a device pixel nearest `v`, so a 1 px line there is sharp.
fn crisp(v: f32, ppp: f32) -> f32 {
    ((v * ppp - 0.5).round() + 0.5) / ppp
}

fn paint_glyph(ui: &Ui, rect: Rect, kind: Button, maximized: bool, color: Color32, ppp: f32) {
    let p = ui.painter();
    let stroke = Stroke::new(1.0 / ppp, color);
    let size = 10.0_f32;
    let x0 = crisp(rect.center().x - size / 2.0, ppp);
    let y0 = crisp(rect.center().y - size / 2.0, ppp);
    let x1 = x0 + size;
    let y1 = y0 + size;
    match kind {
        Button::Minimize => {
            let y = crisp(rect.center().y, ppp);
            p.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], stroke);
        }
        Button::Maximize if !maximized => {
            p.rect_stroke(
                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)),
                0.0,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Button::Maximize => {
            // Two overlapping squares: the front one low-left, the back one's outline above it.
            let s = size - 2.0;
            p.rect_stroke(
                Rect::from_min_max(Pos2::new(x0, y0 + 2.0), Pos2::new(x0 + s, y0 + 2.0 + s)),
                0.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(x0 + 2.0, y0 + 2.0),
                    Pos2::new(x0 + 2.0, y0),
                    Pos2::new(x1, y0),
                    Pos2::new(x1, y0 + s),
                    Pos2::new(x0 + s, y0 + s),
                ],
                stroke,
            ));
        }
        Button::Close => {
            p.line_segment([Pos2::new(x0, y0), Pos2::new(x1, y1)], stroke);
            p.line_segment([Pos2::new(x0, y1), Pos2::new(x1, y0)], stroke);
        }
    }
}

#[cfg(windows)]
pub use win::install;

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, ScreenToClient, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;
    use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT,
    };
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    static CTX: OnceLock<Context> = OnceLock::new();
    const SUBCLASS_ID: usize = 0x4B50; // "KP"

    /// Extend the client area over the caption and subclass the window. Call once, on the
    /// window's thread, from the creation callback. `inner` is the wanted inner size in logical
    /// px: the window was sized for a caption that no longer exists, so it is trimmed to match.
    pub fn install(cc: &eframe::CreationContext<'_>, inner: (f32, f32)) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(handle) = cc.window_handle() else { return };
        let RawWindowHandle::Win32(h) = handle.as_raw() else { return };
        let hwnd = h.hwnd.get() as HWND;
        let _ = CTX.set(cc.egui_ctx.clone());
        // SAFETY: `hwnd` is this process's live top-level window, used on its own thread.
        unsafe {
            if SetWindowSubclass(hwnd, Some(proc_), SUBCLASS_ID, 0) == 0 {
                return;
            }
            let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 };
            DwmExtendFrameIntoClientArea(hwnd, &margins);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            if IsZoomed(hwnd) == 0 {
                let scale = GetDpiForWindow(hwnd) as f32 / 96.0;
                let mut client: RECT = std::mem::zeroed();
                let mut outer: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut client);
                GetWindowRect(hwnd, &mut outer);
                let extra = client.bottom - (inner.1 * scale).round() as i32;
                if extra > 0 {
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        outer.right - outer.left,
                        outer.bottom - outer.top - extra,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }

    fn set(hover: bool, b: u8) {
        let a = if hover { &HOVER } else { &PRESSED };
        if a.swap(b, Ordering::Relaxed) != b {
            if let Some(c) = CTX.get() {
                c.request_repaint();
            }
        }
    }

    fn button_for(ht: u32) -> u8 {
        match ht {
            super::HTMINBUTTON => 1,
            super::HTMAXBUTTON => 2,
            super::HTCLOSE => 3,
            _ => 0,
        }
    }

    unsafe extern "system" fn proc_(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        // SAFETY: a window procedure called by the system with its documented arguments.
        unsafe {
            match msg {
                WM_NCCALCSIZE if wparam != 0 => {
                    let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
                    if IsZoomed(hwnd) != 0 {
                        // Maximised: exactly the monitor's work area (no frame overhang).
                        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                        let mut mi: MONITORINFO = std::mem::zeroed();
                        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                        if GetMonitorInfoW(mon, &mut mi) != 0 {
                            params.rgrc[0] = mi.rcWork;
                            return 0;
                        }
                        return DefSubclassProc(hwnd, msg, wparam, lparam);
                    }
                    // Normal: keep the side and bottom borders, drop the whole top.
                    let top = params.rgrc[0].top;
                    let r = DefSubclassProc(hwnd, msg, wparam, lparam);
                    params.rgrc[0].top = top;
                    r
                }
                WM_NCHITTEST => {
                    let def = DefSubclassProc(hwnd, msg, wparam, lparam);
                    if def != super::HTCLIENT as LRESULT {
                        return def;
                    }
                    let mut pt = POINT {
                        x: (lparam & 0xFFFF) as i16 as i32,
                        y: ((lparam >> 16) & 0xFFFF) as i16 as i32,
                    };
                    ScreenToClient(hwnd, &mut pt);
                    let mut rc: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut rc);
                    let dpi = GetDpiForWindow(hwnd);
                    let border = (GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi))
                        as f32;
                    hit_test(
                        Pos2::new(pt.x as f32, pt.y as f32),
                        &current(),
                        IsZoomed(hwnd) != 0,
                        rc.right as f32,
                        border,
                    ) as LRESULT
                }
                WM_NCMOUSEMOVE => {
                    let b = button_for(wparam as u32);
                    set(true, b);
                    if b != 0 {
                        let mut t = TRACKMOUSEEVENT {
                            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE | TME_NONCLIENT,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        TrackMouseEvent(&mut t);
                    }
                    DefSubclassProc(hwnd, msg, wparam, lparam)
                }
                WM_NCMOUSELEAVE => {
                    set(true, 0);
                    set(false, 0);
                    DefSubclassProc(hwnd, msg, wparam, lparam)
                }
                WM_MOUSEMOVE => {
                    set(true, 0);
                    DefSubclassProc(hwnd, msg, wparam, lparam)
                }
                WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK if button_for(wparam as u32) != 0 => {
                    set(false, button_for(wparam as u32));
                    0
                }
                WM_NCLBUTTONUP if button_for(wparam as u32) != 0 => {
                    let b = button_for(wparam as u32);
                    let was = PRESSED.load(Ordering::Relaxed);
                    set(false, 0);
                    if was == b {
                        let cmd = match b {
                            1 => SC_MINIMIZE,
                            2 if IsZoomed(hwnd) != 0 => SC_RESTORE,
                            2 => SC_MAXIMIZE,
                            _ => SC_CLOSE,
                        };
                        PostMessageW(hwnd, WM_SYSCOMMAND, cmd as WPARAM, 0);
                    }
                    0
                }
                WM_NCDESTROY => {
                    RemoveWindowSubclass(hwnd, Some(proc_), SUBCLASS_ID);
                    DefSubclassProc(hwnd, msg, wparam, lparam)
                }
                _ => DefSubclassProc(hwnd, msg, wparam, lparam),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        let r = |x0, x1| Rect::from_min_max(Pos2::new(x0, 0.0), Pos2::new(x1, 32.0));
        Layout {
            ppp: 1.0,
            bar_h: 32.0,
            interactive: vec![
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(120.0, 32.0)),
                Rect::from_min_max(Pos2::new(400.0, 5.0), Pos2::new(700.0, 27.0)),
            ],
            minimize: r(862.0, 908.0),
            maximize: r(908.0, 954.0),
            close: r(954.0, 1000.0),
        }
    }

    fn ht(x: f32, y: f32, maximised: bool) -> u32 {
        hit_test(Pos2::new(x, y), &layout(), maximised, 1000.0, 8.0)
    }

    #[test]
    fn empty_bar_and_title_are_caption() {
        assert_eq!(ht(200.0, 16.0, false), HTCAPTION);
        assert_eq!(ht(800.0, 16.0, false), HTCAPTION);
    }

    #[test]
    fn widgets_are_client() {
        assert_eq!(ht(50.0, 16.0, false), HTCLIENT);
        assert_eq!(ht(500.0, 16.0, false), HTCLIENT);
    }

    #[test]
    fn caption_buttons_have_their_own_codes() {
        assert_eq!(ht(880.0, 16.0, false), HTMINBUTTON);
        assert_eq!(ht(930.0, 16.0, false), HTMAXBUTTON);
        assert_eq!(ht(990.0, 16.0, false), HTCLOSE);
    }

    #[test]
    fn below_the_bar_is_client() {
        assert_eq!(ht(200.0, 40.0, false), HTCLIENT);
    }

    #[test]
    fn top_edge_resizes_unless_maximised() {
        assert_eq!(ht(500.0, 2.0, false), HTTOP);
        assert_eq!(ht(3.0, 2.0, false), HTTOPLEFT);
        assert_eq!(ht(996.0, 2.0, false), HTTOPRIGHT);
        assert_eq!(ht(500.0, 2.0, true), HTCAPTION);
        // Maximised, the buttons reach the very top edge.
        assert_eq!(ht(990.0, 0.0, true), HTCLOSE);
    }

    #[test]
    fn scaled_points_are_converted() {
        let mut l = layout();
        l.ppp = 1.25;
        // 990 logical px = 1237.5 device px.
        assert_eq!(hit_test(Pos2::new(1237.0, 20.0), &l, false, 1250.0, 8.0), HTCLOSE);
        assert_eq!(hit_test(Pos2::new(250.0, 20.0), &l, false, 1250.0, 8.0), HTCAPTION);
    }

    #[test]
    fn layout_before_the_first_frame_is_all_client() {
        let l = Layout::default();
        assert_eq!(hit_test(Pos2::new(500.0, 20.0), &l, false, 1000.0, 8.0), HTCLIENT);
    }
}
