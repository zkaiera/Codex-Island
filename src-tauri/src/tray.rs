use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Emitter, Runtime};

use crate::startup;

const AUTOSTART_MENU_ID: &str = "autostart";
const QUIT_MENU_ID: &str = "quit";

pub fn setup_tray<R: Runtime>(app: &App<R>) -> tauri::Result<()> {
    let autostart = CheckMenuItem::with_id(
        app,
        AUTOSTART_MENU_ID,
        "开机自启",
        true,
        startup::is_enabled(),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&autostart, &quit])?;
    let autostart_for_menu = autostart.clone();

    let mut tray = TrayIconBuilder::with_id("codex-island")
        .tooltip("Codex Island")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event: MenuEvent| match event.id().as_ref() {
            AUTOSTART_MENU_ID => toggle_autostart(app, &autostart_for_menu),
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

fn toggle_autostart<R: Runtime>(app: &tauri::AppHandle<R>, item: &CheckMenuItem<R>) {
    let next_enabled = !startup::is_enabled();
    let result = std::env::current_exe()
        .map_err(Into::into)
        .and_then(|path| startup::set_enabled(next_enabled, &path));

    let checked = result
        .map(|_| next_enabled)
        .unwrap_or_else(|_| startup::is_enabled());
    let _ = item.set_checked(checked);
    let _ = app.emit("autostart:changed", checked);
}
