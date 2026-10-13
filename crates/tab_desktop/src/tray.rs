use std::borrow::Cow;

use muda::{
    IconMenuItem,
    Menu,
    MenuEvent,
    MenuId,
    MenuItemBuilder,
    PredefinedMenuItem,
    Submenu,
};
use tab_install::InstallComponents;
use tab_os_shim::Context;
use tab_remote_ipc::figterm::FigtermState;
use tab_util::consts::PRODUCT_NAME;
use tab_util::url::USER_MANUAL;
use tao::event_loop::ControlFlow;
use tracing::{
    error,
    trace,
};
use tray_icon::{
    Icon,
    TrayIcon,
    TrayIconBuilder,
};

use crate::event::{
    Event,
    WindowEvent,
};
use crate::{
    AUTOCOMPLETE_ID,
    DASHBOARD_ID,
    EventLoopProxy,
    EventLoopWindowTarget,
};

// macro_rules! icon {
//     ($icon:literal) => {{
//         #[cfg(target_os = "macos")]
//         {
//             Some(include_bytes!(concat!(
//                 env!("TRAY_ICONS_PROCESSED"),
//                 "/",
//                 $icon,
//                 ".png"
//             )))
//         }
//         #[cfg(not(target_os = "macos"))]
//         {
//             None
//         }
//     }};
// }

pub fn handle_event(menu_event: &MenuEvent, proxy: &EventLoopProxy) {
    match &*menu_event.id().0 {
        "dashboard-devtools" => {
            proxy
                .send_event(Event::WindowEvent {
                    window_id: DASHBOARD_ID,
                    window_event: WindowEvent::Devtools,
                })
                .unwrap();
        },
        "autocomplete-devtools" => {
            proxy
                .send_event(Event::WindowEvent {
                    window_id: AUTOCOMPLETE_ID,
                    window_event: WindowEvent::Devtools,
                })
                .unwrap();
        },
        "quit" => {
            proxy.send_event(Event::ControlFlow(ControlFlow::Exit)).unwrap();
        },
        "dashboard" => {
            proxy
                .send_event(Event::WindowEvent {
                    window_id: DASHBOARD_ID.clone(),
                    window_event: WindowEvent::Batch(vec![
                        WindowEvent::NavigateRelative { path: "/".into() },
                        WindowEvent::Show,
                    ]),
                })
                .unwrap();
        },
        "settings" => {
            proxy
                .send_event(Event::WindowEvent {
                    window_id: DASHBOARD_ID.clone(),
                    window_event: WindowEvent::Batch(vec![
                        WindowEvent::NavigateRelative {
                            path: "/autocomplete".into(),
                        },
                        WindowEvent::Show,
                    ]),
                })
                .unwrap();
        },
        "not-working" => {
            proxy
                .send_event(Event::WindowEvent {
                    window_id: DASHBOARD_ID.clone(),
                    window_event: WindowEvent::Batch(vec![
                        WindowEvent::NavigateRelative { path: "/help".into() },
                        WindowEvent::Show,
                    ]),
                })
                .unwrap();
        },
        "uninstall" => {
            tokio::runtime::Handle::current().spawn(async {
                tab_install::uninstall(InstallComponents::all(), Context::new())
                    .await
                    .ok();
                #[allow(clippy::exit)]
                std::process::exit(0);
            });
        },
        "user-manual" => {
            if let Err(err) = tab_util::open_url(USER_MANUAL) {
                error!(%err, "Failed to open user manual url");
            }
        },
        id => {
            trace!(?id, "Unhandled tray event");
        },
    }
}

pub async fn build_tray(
    _event_loop_window_target: &EventLoopWindowTarget,
    _figterm_state: &FigtermState,
) -> tray_icon::Result<TrayIcon> {
    TrayIconBuilder::new()
        .with_icon(get_icon())
        .with_icon_as_template(true)
        .with_menu(Box::new(get_context_menu()))
        .build()
}

pub fn get_icon() -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let bytes: &[u8] = if cfg!(target_os = "linux") {
            include_bytes!("../icons/icon-monochrome-light.png")
        } else {
            include_bytes!("../icons/icon-monochrome.png")
        };
        let image = image::load_from_memory(bytes)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

pub fn get_context_menu() -> Menu {
    let mut tray_menu = Menu::new();

    let elements = menu();
    for elem in elements {
        elem.add_to_menu(&mut tray_menu);
    }

    tray_menu
}

enum MenuElement {
    Info {
        image_icon: Option<muda::Icon>,
        text: Cow<'static, str>,
    },
    Entry {
        emoji_icon: Option<Cow<'static, str>>,
        image_icon: Option<muda::Icon>,
        text: Cow<'static, str>,
        id: Cow<'static, str>,
    },
    Separator,
    #[allow(dead_code)]
    SubMenu {
        title: Cow<'static, str>,
        elements: Vec<MenuElement>,
    },
}

impl MenuElement {
    fn info(image_icon: Option<(Vec<u8>, u32, u32)>, text: impl Into<Cow<'static, str>>) -> Self {
        Self::Info {
            image_icon: image_icon.and_then(|(bytes, width, height)| muda::Icon::from_rgba(bytes, width, height).ok()),
            text: text.into(),
        }
    }

    fn entry(
        emoji_icon: Option<Cow<'static, str>>,
        image_icon: Option<(Vec<u8>, u32, u32)>,
        text: impl Into<Cow<'static, str>>,
        id: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::Entry {
            emoji_icon,
            image_icon: image_icon.and_then(|(bytes, width, height)| muda::Icon::from_rgba(bytes, width, height).ok()),
            text: text.into(),
            id: id.into(),
        }
    }

    // fn sub_menu(title: impl Into<Cow<'static, str>>, elements: Vec<MenuElement>) -> Self {
    //     Self::SubMenu {
    //         title: title.into(),
    //         elements,
    //     }
    // }

    fn add_to_menu(&self, menu: &mut Menu) {
        match self {
            MenuElement::Info { text, image_icon } => {
                let menu_item = IconMenuItem::new(
                    text,
                    false,
                    image_icon.clone(), // Some(muda::Icon::from_rgba(bytes, width, height).unwrap()),
                    None,
                );
                menu.append(&menu_item).unwrap();
            },
            MenuElement::Entry {
                emoji_icon,
                image_icon,
                text,
                id,
                ..
            } => {
                let text = match (std::env::consts::OS, emoji_icon) {
                    ("linux", Some(emoji_icon)) => format!("{emoji_icon} {text}"),
                    _ => text.to_string(),
                };
                let menu_item = muda::IconMenuItemBuilder::new()
                    .text(text)
                    .id(MenuId::new(id))
                    .enabled(true)
                    .icon(image_icon.clone())
                    .build();
                menu.append(&menu_item).unwrap();
            },
            MenuElement::Separator => {
                menu.append(&PredefinedMenuItem::separator()).unwrap();
            },
            MenuElement::SubMenu { title, elements } => {
                let sub_menu = Submenu::new(title, true);
                for element in elements {
                    element.add_to_submenu(&sub_menu);
                }

                menu.append(&sub_menu).unwrap();
            },
        }
    }

    fn add_to_submenu(&self, submenu: &Submenu) {
        match self {
            MenuElement::Info { image_icon, text } => {
                // menu.append(MenuItemAttributes::new(info).with_enabled(false));
                let menu_item = IconMenuItem::new(
                    text,
                    false,
                    image_icon.clone(), // Some(muda::Icon::from_rgba(bytes, width, height).unwrap()),
                    None,
                );
                submenu.append(&menu_item).unwrap();
            },
            MenuElement::Entry {
                emoji_icon, text, id, ..
            } => {
                let text: String = match (std::env::consts::OS, emoji_icon) {
                    ("linux", Some(emoji_icon)) => format!("{emoji_icon} {text}"),
                    _ => text.to_string(),
                };
                let menu_item = MenuItemBuilder::new()
                    .text(text)
                    .id(MenuId::new(id))
                    .enabled(true)
                    .build();
                submenu.append(&menu_item).unwrap();
            },
            MenuElement::Separator => {
                submenu.append(&PredefinedMenuItem::separator()).unwrap();
            },
            MenuElement::SubMenu { title, elements } => {
                let sub_menu = Submenu::new(title, true);
                for element in elements {
                    element.add_to_submenu(&sub_menu);
                }

                submenu.append(&sub_menu).unwrap();
            },
        }
    }
}

fn menu() -> Vec<MenuElement> {
    let not_working = MenuElement::entry(None, None, format!("{PRODUCT_NAME} not working?"), "not-working");
    let manual = MenuElement::entry(None, None, "User Guide", "user-manual");
    let version = MenuElement::info(None, format!("Version: {}", env!("CARGO_PKG_VERSION")));
    let quit = MenuElement::entry(None, None, format!("Quit {PRODUCT_NAME}"), "quit");
    // let dashboard = MenuElement::entry(None, None, "Dashboard", "dashboard");
    let settings = MenuElement::entry(None, None, "Settings", "settings");
    // let developer = MenuElement::sub_menu("Developer", vec![
    //     MenuElement::entry(None, None, "Dashboard Devtools", "dashboard-devtools"),
    //     MenuElement::entry(None, None, "Autocomplete Devtools", "autocomplete-devtools"),
    //     MenuElement::entry(None, None, "Companion Devtools", "companion-devtools"),
    // ]);

    vec![
        settings,
        MenuElement::Separator,
        manual,
        not_working,
        MenuElement::Separator,
        version,
        MenuElement::Separator,
        quit,
    ]
}
