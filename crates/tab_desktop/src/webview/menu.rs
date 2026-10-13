#[allow(unused_imports)]
use muda::{
    Menu,
    MenuEvent,
    Submenu,
};
#[allow(unused_imports)]
use tab_util::consts::PRODUCT_NAME;

use crate::event::{
    Event,
    WindowEvent,
};
use crate::{
    DASHBOARD_ID,
    EventLoopProxy,
};

const DASHBOARD_QUIT: &str = "dashboard-quit";
const DASHBOARD_RELOAD: &str = "dashboard-reload";
const DASHBOARD_BACK: &str = "dashboard-back";
const DASHBOARD_FORWARD: &str = "dashboard-forward";

pub fn menu_bar() -> Menu {
    use muda::{
        MenuItemBuilder,
        PredefinedMenuItem,
        Submenu,
    };

    let menu_bar = Menu::new();

    let app_submenu = Submenu::new(PRODUCT_NAME, true);
    app_submenu
        .append_items(&[
            &MenuItemBuilder::new()
                .text("Backward")
                .id(DASHBOARD_BACK.into())
                .enabled(true)
                .accelerator(Some("super+["))
                .unwrap()
                .build(),
            &MenuItemBuilder::new()
                .text("Forward")
                .id(DASHBOARD_FORWARD.into())
                .enabled(true)
                .accelerator(Some("super+]"))
                .unwrap()
                .build(),
            &MenuItemBuilder::new()
                .text("Reload")
                .id(DASHBOARD_RELOAD.into())
                .enabled(true)
                .accelerator(Some("super+r"))
                .unwrap()
                .build(),
            &MenuItemBuilder::new()
                .text("Close Window")
                .id(DASHBOARD_QUIT.into())
                .enabled(true)
                .accelerator(Some("super+w"))
                .unwrap()
                .build(),
            &MenuItemBuilder::new()
                .text(format!("Quit {PRODUCT_NAME} (UI)"))
                .id(DASHBOARD_QUIT.into())
                .enabled(true)
                .accelerator(Some("super+q"))
                .unwrap()
                .build(),
        ])
        .unwrap();

    menu_bar.append(&app_submenu).unwrap();

    let edit_submenu = Submenu::new("Edit", true);
    edit_submenu
        .append_items(&[
            &PredefinedMenuItem::undo(Some("Undo")),
            &PredefinedMenuItem::redo(Some("Redo")),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::copy(Some("Copy")),
            &PredefinedMenuItem::paste(Some("Paste")),
            &PredefinedMenuItem::cut(Some("Cut")),
            &PredefinedMenuItem::select_all(Some("Select All")),
        ])
        .unwrap();

    menu_bar.append(&edit_submenu).unwrap();

    let window_submenu = Submenu::new("Window", true);
    window_submenu
        .append_items(&[&PredefinedMenuItem::minimize(Some("Minimize"))])
        .unwrap();
    menu_bar.append(&window_submenu).unwrap();

    let help_submenu = Submenu::new("Help", true);
    menu_bar.append(&help_submenu).unwrap();

    menu_bar
}

// TODO(chay): add whatever is ergonomic for Windows

pub fn handle_event(menu_event: &MenuEvent, proxy: &EventLoopProxy) {
    match &menu_event.id().0 {
        menu_id if menu_id == DASHBOARD_QUIT => proxy
            .send_event(Event::WindowEvent {
                window_id: DASHBOARD_ID,
                window_event: WindowEvent::Hide,
            })
            .unwrap(),
        menu_id if menu_id == DASHBOARD_RELOAD => proxy
            .send_event(Event::WindowEvent {
                window_id: DASHBOARD_ID,
                window_event: WindowEvent::Reload,
            })
            .unwrap(),
        menu_id if menu_id == DASHBOARD_BACK => proxy
            .send_event(Event::WindowEvent {
                window_id: DASHBOARD_ID,
                window_event: WindowEvent::NavigateBack,
            })
            .unwrap(),
        menu_id if menu_id == DASHBOARD_FORWARD => proxy
            .send_event(Event::WindowEvent {
                window_id: DASHBOARD_ID,
                window_event: WindowEvent::NavigateForward,
            })
            .unwrap(),
        _ => (),
    }
}
