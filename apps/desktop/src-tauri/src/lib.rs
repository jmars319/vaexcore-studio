use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager,
};
use tokio::sync::oneshot;
use vaexcore_api::{
    default_auth_from_env, default_bind_addr, serve_with_shutdown, ApiServerConfig, AuthConfig,
};

const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_CLOSE_WINDOW: &str = "close-window";
const MENU_QUIT_APP: &str = "quit-app";
const FRONTEND_OPEN_SETTINGS_EVENT: &str = "vaexcore://open-settings";

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

pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vaexcore_api=info,vaexcore_studio_desktop=info".into()),
        )
        .try_init();

    tauri::Builder::default()
        .menu(|handle| {
            let settings = MenuItem::with_id(
                handle,
                MENU_OPEN_SETTINGS,
                "Settings...",
                true,
                Some("CmdOrCtrl+,"),
            )?;
            let close_window = MenuItem::with_id(
                handle,
                MENU_CLOSE_WINDOW,
                "Close Window",
                true,
                Some("CmdOrCtrl+W"),
            )?;
            let quit = MenuItem::with_id(
                handle,
                MENU_QUIT_APP,
                "Quit vaexcore-studio",
                true,
                Some("CmdOrCtrl+Q"),
            )?;
            let separator = PredefinedMenuItem::separator(handle)?;
            let edit_separator_one = PredefinedMenuItem::separator(handle)?;
            let edit_separator_two = PredefinedMenuItem::separator(handle)?;

            let app_menu = Submenu::with_items(
                handle,
                "vaexcore-studio",
                true,
                &[&settings, &separator, &quit],
            )?;
            let file_menu = Submenu::with_items(handle, "File", true, &[&close_window])?;
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
            let window_menu = Submenu::with_items(
                handle,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(handle, None)?,
                    &PredefinedMenuItem::fullscreen(handle, None)?,
                    &PredefinedMenuItem::bring_all_to_front(handle, None)?,
                ],
            )?;

            Menu::with_items(handle, &[&app_menu, &file_menu, &edit_menu, &window_menu])
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_SETTINGS => open_settings(app),
            MENU_CLOSE_WINDOW => close_main_window(app),
            MENU_QUIT_APP => quit_app(app),
            _ => {}
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
        .invoke_handler(tauri::generate_handler![api_config])
        .run(tauri::generate_context!())
        .expect("failed to run vaexcore-studio");
}

fn open_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(FRONTEND_OPEN_SETTINGS_EVENT, ());
        let _ = window.set_focus();
    }
}

fn close_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
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
