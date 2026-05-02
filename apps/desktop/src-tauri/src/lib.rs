use std::sync::Mutex;

use tauri::{
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tokio::sync::oneshot;
use vaexcore_api::{
    default_auth_from_env, default_bind_addr, serve_with_shutdown, ApiServerConfig, AuthConfig,
};

const APP_NAME: &str = "vaexcore studio";
const MAIN_WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_CLOSE_WINDOW: &str = "close-window";
const MENU_QUIT_APP: &str = "quit-app";
const MENU_SHOW_MAIN_WINDOW: &str = "show-main-window";
const MENU_RELOAD_WINDOW: &str = "reload-window";
const MENU_VIEW_DASHBOARD: &str = "view-dashboard";
const MENU_VIEW_DESTINATIONS: &str = "view-destinations";
const MENU_VIEW_PROFILES: &str = "view-profiles";
const MENU_VIEW_CONTROLS: &str = "view-controls";
const MENU_VIEW_CONNECTED_APPS: &str = "view-connected-apps";
const MENU_VIEW_LOGS: &str = "view-logs";
const FRONTEND_OPEN_SECTION_EVENT: &str = "vaexcore://open-section";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendApiConfig {
    api_url: String,
    ws_url: String,
    token: Option<String>,
    dev_auth_bypass: bool,
}

struct AppRuntimeState {
    api: FrontendApiConfig,
    api_shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

#[tauri::command]
fn api_config(state: tauri::State<'_, AppRuntimeState>) -> FrontendApiConfig {
    state.api.clone()
}

#[tauri::command]
async fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    show_settings_window(&app).map_err(|error| error.to_string())
}

pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vaexcore_api=info,vaexcore_studio_desktop=info".into()),
        )
        .try_init();

    tauri::Builder::default()
        .menu(|handle| {
            let about = PredefinedMenuItem::about(
                handle,
                Some("About vaexcore studio"),
                Some(AboutMetadata {
                    name: Some(APP_NAME.to_string()),
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    copyright: Some("Copyright 2026 vaexcore studio".to_string()),
                    ..Default::default()
                }),
            )?;
            let settings = MenuItem::with_id(
                handle,
                MENU_OPEN_SETTINGS,
                "Configuration Settings...",
                true,
                Some("CmdOrCtrl+,"),
            )?;
            let close_window = MenuItem::with_id(
                handle,
                MENU_CLOSE_WINDOW,
                "Close Window (App Keeps Running)",
                true,
                Some("CmdOrCtrl+W"),
            )?;
            let quit = MenuItem::with_id(
                handle,
                MENU_QUIT_APP,
                "Quit App (Stops Local Server)",
                true,
                Some("CmdOrCtrl+Q"),
            )?;
            let show_main_window = MenuItem::with_id(
                handle,
                MENU_SHOW_MAIN_WINDOW,
                "Show Main Window",
                true,
                None::<&str>,
            )?;
            let reload_window = MenuItem::with_id(
                handle,
                MENU_RELOAD_WINDOW,
                "Reload Window",
                true,
                Some("CmdOrCtrl+R"),
            )?;
            let view_dashboard =
                MenuItem::with_id(handle, MENU_VIEW_DASHBOARD, "Dashboard", true, None::<&str>)?;
            let view_destinations = MenuItem::with_id(
                handle,
                MENU_VIEW_DESTINATIONS,
                "Stream Destinations",
                true,
                None::<&str>,
            )?;
            let view_profiles = MenuItem::with_id(
                handle,
                MENU_VIEW_PROFILES,
                "Recording Profiles",
                true,
                None::<&str>,
            )?;
            let view_controls =
                MenuItem::with_id(handle, MENU_VIEW_CONTROLS, "Controls", true, None::<&str>)?;
            let view_connected_apps = MenuItem::with_id(
                handle,
                MENU_VIEW_CONNECTED_APPS,
                "Connected Apps",
                true,
                None::<&str>,
            )?;
            let view_logs = MenuItem::with_id(handle, MENU_VIEW_LOGS, "Logs", true, None::<&str>)?;
            let app_separator_one = PredefinedMenuItem::separator(handle)?;
            let app_separator_two = PredefinedMenuItem::separator(handle)?;
            let view_separator = PredefinedMenuItem::separator(handle)?;
            let window_separator = PredefinedMenuItem::separator(handle)?;
            let edit_separator_one = PredefinedMenuItem::separator(handle)?;
            let edit_separator_two = PredefinedMenuItem::separator(handle)?;

            let app_menu = Submenu::with_items(
                handle,
                APP_NAME,
                true,
                &[
                    &about,
                    &app_separator_one,
                    &settings,
                    &app_separator_two,
                    &close_window,
                    &quit,
                ],
            )?;
            let edit_menu = Submenu::with_items(
                handle,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(handle, None)?,
                    &PredefinedMenuItem::redo(handle, None)?,
                    &edit_separator_one,
                    &PredefinedMenuItem::cut(handle, None)?,
                    &PredefinedMenuItem::copy(handle, None)?,
                    &PredefinedMenuItem::paste(handle, None)?,
                    &edit_separator_two,
                    &PredefinedMenuItem::select_all(handle, None)?,
                ],
            )?;
            let view_menu = Submenu::with_items(
                handle,
                "View",
                true,
                &[
                    &view_dashboard,
                    &view_destinations,
                    &view_profiles,
                    &view_controls,
                    &view_connected_apps,
                    &view_logs,
                    &view_separator,
                    &reload_window,
                ],
            )?;
            let window_menu = Submenu::with_items(
                handle,
                "Window",
                true,
                &[
                    &show_main_window,
                    &window_separator,
                    &PredefinedMenuItem::minimize(handle, None)?,
                    &PredefinedMenuItem::fullscreen(handle, None)?,
                    &PredefinedMenuItem::bring_all_to_front(handle, None)?,
                ],
            )?;

            Menu::with_items(handle, &[&app_menu, &edit_menu, &view_menu, &window_menu])
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_SETTINGS => open_settings(app),
            MENU_CLOSE_WINDOW => close_active_window(app),
            MENU_QUIT_APP => quit_app(app),
            MENU_SHOW_MAIN_WINDOW => show_main_window(app),
            MENU_RELOAD_WINDOW => reload_main_window(app),
            MENU_VIEW_DASHBOARD => open_section(app, "dashboard"),
            MENU_VIEW_DESTINATIONS => open_section(app, "destinations"),
            MENU_VIEW_PROFILES => open_section(app, "profiles"),
            MENU_VIEW_CONTROLS => open_section(app, "controls"),
            MENU_VIEW_CONNECTED_APPS => open_section(app, "apps"),
            MENU_VIEW_LOGS => open_section(app, "logs"),
            _ => {}
        })
        .on_window_event(|window, event| {
            if window.label() == MAIN_WINDOW_LABEL {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let auth = default_auth_from_env();
            let bind_addr = default_bind_addr();
            let api = frontend_api_config(bind_addr, &auth);
            let database_path = app.path().app_data_dir()?.join("studio.sqlite");
            let (api_shutdown, shutdown_rx) = oneshot::channel::<()>();

            app.manage(AppRuntimeState {
                api,
                api_shutdown: Mutex::new(Some(api_shutdown)),
            });

            tauri::async_runtime::spawn(async move {
                let config = ApiServerConfig {
                    bind_addr,
                    database_path,
                    auth,
                };

                if let Err(error) = serve_with_shutdown(config, async {
                    let _ = shutdown_rx.await;
                })
                .await
                {
                    tracing::error!(%error, "local API stopped");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api_config, open_settings_window])
        .run(tauri::generate_context!())
        .expect("failed to run vaexcore studio");
}

fn open_settings(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(error) = show_settings_window(&app) {
            tracing::error!(%error, "failed to open settings window");
        }
    });
}

fn open_section(app: &tauri::AppHandle, section: &str) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.emit(FRONTEND_OPEN_SECTION_EVENT, section);
        let _ = window.set_focus();
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn close_active_window(app: &tauri::AppHandle) {
    if let Some(window) = app
        .webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
    {
        if window.label() == MAIN_WINDOW_LABEL {
            let _ = window.hide();
        } else {
            let _ = window.close();
        }
        return;
    }

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

fn reload_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.reload();
    }
}

fn show_settings_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW_LABEL,
        WebviewUrl::App("index.html?window=settings".into()),
    )
    .title("Configuration Settings")
    .inner_size(560.0, 640.0)
    .min_inner_size(500.0, 520.0)
    .resizable(true)
    .maximizable(false)
    .center()
    .focused(true)
    .build()?;

    Ok(())
}

fn quit_app(app: &tauri::AppHandle) {
    if let Some(shutdown) = app
        .state::<AppRuntimeState>()
        .api_shutdown
        .lock()
        .expect("api shutdown mutex poisoned")
        .take()
    {
        let _ = shutdown.send(());
    }

    app.exit(0);
}

fn frontend_api_config(bind_addr: std::net::SocketAddr, auth: &AuthConfig) -> FrontendApiConfig {
    let api_url = format!("http://{bind_addr}");
    let ws_url = format!("ws://{bind_addr}/events");
    FrontendApiConfig {
        api_url,
        ws_url,
        token: auth.token.clone(),
        dev_auth_bypass: auth.dev_mode,
    }
}
