use std::error::Error;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::icon;

const ICON_SIZE: u32 = 32;
const TOOLTIP: &str = "sushot";
const AUTOSTART_LABEL: &str = "Iniciar con el sistema";
const QUIT_LABEL: &str = "Salir";

#[cfg(windows)]
/// tray-icon numbers its icons from 1 and this process creates exactly one.
const FIRST_ICON_ID: u32 = 1;

pub(crate) struct Tray {
    icon: TrayIcon,
    normal: Icon,
    flash: Icon,
    autostart: CheckMenuItem,
    quit: MenuItem,
}

impl Tray {
    pub(crate) fn new(autostart_enabled: bool) -> Result<Self, Box<dyn Error>> {
        let normal = Icon::from_rgba(icon::rgba(ICON_SIZE, false), ICON_SIZE, ICON_SIZE)?;
        let flash = Icon::from_rgba(icon::rgba(ICON_SIZE, true), ICON_SIZE, ICON_SIZE)?;

        let title = MenuItem::new(format!("sushot {}", env!("CARGO_PKG_VERSION")), false, None);
        let autostart = CheckMenuItem::new(AUTOSTART_LABEL, true, autostart_enabled, None);
        let quit = MenuItem::new(QUIT_LABEL, true, None);
        let menu = Menu::new();
        menu.append_items(&[&title, &autostart, &PredefinedMenuItem::separator(), &quit])?;

        let icon = TrayIconBuilder::new()
            .with_icon(normal.clone())
            .with_tooltip(TOOLTIP)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()?;
        Ok(Self {
            icon,
            normal,
            flash,
            autostart,
            quit,
        })
    }

    pub(crate) fn autostart_id(&self) -> &MenuId {
        self.autostart.id()
    }

    pub(crate) fn quit_id(&self) -> &MenuId {
        self.quit.id()
    }

    pub(crate) fn autostart_checked(&self) -> bool {
        self.autostart.is_checked()
    }

    pub(crate) fn set_autostart_checked(&self, checked: bool) {
        self.autostart.set_checked(checked);
    }

    pub(crate) fn flash(&self) {
        self.show(&self.flash);
    }

    pub(crate) fn restore(&self) {
        self.show(&self.normal);
    }

    #[cfg(windows)]
    pub(crate) fn notify(&self, title: &str, text: &str) {
        let window = self.icon.window_handle() as isize;
        if let Err(error) =
            sushot_platform_windows::show_tray_notification(window, FIRST_ICON_ID, title, text)
        {
            log::warn!("{error}");
        }
    }

    #[cfg(not(windows))]
    pub(crate) fn notify(&self, title: &str, text: &str) {
        log::info!("{title}: {text}");
    }

    fn show(&self, icon: &Icon) {
        if let Err(error) = self.icon.set_icon(Some(icon.clone())) {
            log::warn!("tray icon was not updated: {error}");
        }
    }
}
