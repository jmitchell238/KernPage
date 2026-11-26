//! Tray icon for "keep running in the tray when closed" (Windows only).

#[cfg(all(windows, not(test)))]
mod imp {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    pub struct Tray(#[allow(dead_code)] TrayIcon);

    pub fn create() -> Option<Tray> {
        let menu = Menu::new();
        let open = MenuItem::with_id("open", "Open KernPage Launcher", true, None);
        let quit = MenuItem::with_id("quit", "Quit", true, None);
        menu.append_items(&[&open, &quit]).ok()?;
        let (rgba, size) = crate::mark::rgba(32);
        let icon = Icon::from_rgba(rgba, size, size).ok()?;
        TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                crate::instance::show_main_window();
            }
        }));
        MenuEvent::set_event_handler(Some(|e: MenuEvent| match e.id.0.as_str() {
            "open" => crate::instance::show_main_window(),
            "quit" => std::process::exit(0),
            _ => {}
        }));
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("KernPage Launcher")
            .with_icon(icon)
            .build()
            .ok()?;
        Some(Tray(tray))
    }
}

#[cfg(all(not(windows), not(test)))]
mod imp {
    pub struct Tray;
    pub fn create() -> Option<Tray> {
        None
    }
}

pub use imp::{create, Tray};

#[cfg(test)]
mod imp {
    /// Test build: no icon is made; the fake records the request (see `testhooks`).
    pub struct Tray;
    pub fn create() -> Option<Tray> {
        crate::testhooks::with(|f| {
            f.tray_creates += 1;
            (!f.tray_fails).then_some(Tray)
        })
    }
}

