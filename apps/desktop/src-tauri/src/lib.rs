use std::{
    env,
    ffi::{c_char, c_void},
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use tauri::{
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tokio::sync::oneshot;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use vaexcore_api::{
    default_auth_from_env, default_bind_addr, generate_token, serve_listener_with_shutdown,
    ApiServerConfig, AuthConfig, ProfileStore, SharedAuthConfig,
};
use vaexcore_core::{
    AppSettings, CaptureSourceCandidate, CaptureSourceInventory, CaptureSourceKind,
    CaptureSourceSelection, PreflightCheck, PreflightSnapshot, PreflightStatus, ProfileBundle,
};
use vaexcore_media::{MediaRunnerConfig, MediaRunnerSupervisor};

const APP_NAME: &str = "vaexcore studio";
const MAIN_WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_CLOSE_WINDOW: &str = "close-window";
const MENU_QUIT_APP: &str = "quit-app";
const MENU_LAUNCH_SUITE: &str = "launch-suite";
const MENU_SHOW_MAIN_WINDOW: &str = "show-main-window";
const MENU_RELOAD_WINDOW: &str = "reload-window";
const MENU_VIEW_DASHBOARD: &str = "view-dashboard";
const MENU_VIEW_DESTINATIONS: &str = "view-destinations";
const MENU_VIEW_PROFILES: &str = "view-profiles";
const MENU_VIEW_CONTROLS: &str = "view-controls";
const MENU_VIEW_CONNECTED_APPS: &str = "view-connected-apps";
const MENU_VIEW_LOGS: &str = "view-logs";
const FRONTEND_OPEN_SECTION_EVENT: &str = "vaexcore://open-section";
const VAEXCORE_SUITE_APPS: &[&str] = &["vaexcore studio", "vaexcore pulse", "vaexcore console"];
const SUITE_DISCOVERY_SCHEMA_VERSION: u8 = 1;
const SUITE_DISCOVERY_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const SUITE_DISCOVERY_STALE_AFTER: Duration = Duration::from_secs(45);

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut u32,
        display_count: *mut u32,
    ) -> i32;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> *const c_void;
    static kCGWindowName: *const c_void;
    static kCGWindowNumber: *const c_void;
    static kCGWindowOwnerName: *const c_void;
    static kCGWindowLayer: *const c_void;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
    fn CFDictionaryGetValueIfPresent(
        dictionary: *const c_void,
        key: *const c_void,
        value: *mut *const c_void,
    ) -> u8;
    fn CFNumberGetValue(number: *const c_void, number_type: i32, value: *mut c_void) -> u8;
    fn CFRelease(value: *const c_void);
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut i8,
        buffer_size: isize,
        encoding: u32,
    ) -> u8;
    fn CFStringGetLength(string: *const c_void) -> isize;
}

#[cfg(target_os = "macos")]
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVMediaTypeAudio: *const c_void;
    static AVMediaTypeVideo: *const c_void;
}

#[cfg(target_os = "macos")]
#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendApiConfig {
    api_url: String,
    ws_url: String,
    configured_api_url: String,
    configured_ws_url: String,
    bind_addr: String,
    configured_bind_addr: String,
    port_fallback_active: bool,
    discovery_file: String,
    token: Option<String>,
    dev_auth_bypass: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendAppSettings {
    settings: AppSettings,
    api_url: String,
    ws_url: String,
    configured_api_url: String,
    configured_ws_url: String,
    port_fallback_active: bool,
    data_dir: String,
    database_path: String,
    discovery_file: String,
    log_dir: String,
    pipeline_plan_path: String,
    pipeline_config_path: String,
    restart_required: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiDiscoveryDocument {
    service: String,
    api_url: String,
    ws_url: String,
    bind_addr: String,
    configured_bind_addr: String,
    port_fallback_active: bool,
    auth_required: bool,
    dev_auth_bypass: bool,
    pid: u32,
    updated_at: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendMediaRunnerInfo {
    bundled: bool,
    running: bool,
    fallback_dry_run: bool,
    status_addr: Option<String>,
    executable_path: Option<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendProfileBundleResult {
    path: String,
    recording_profiles: usize,
    stream_destinations: usize,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendPermissionStatus {
    service: String,
    status: String,
    detail: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteLaunchResult {
    app_name: String,
    ok: bool,
    detail: String,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteDiscoveryDocument {
    schema_version: u8,
    app_id: String,
    app_name: String,
    bundle_identifier: String,
    version: String,
    pid: u32,
    started_at: String,
    updated_at: String,
    api_url: Option<String>,
    ws_url: Option<String>,
    health_url: Option<String>,
    capabilities: Vec<String>,
    launch_name: String,
    suite_session_id: Option<String>,
    activity: Option<String>,
    activity_detail: Option<String>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteSessionDocument {
    schema_version: u8,
    session_id: String,
    title: String,
    status: String,
    owner_app: String,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuiteCommandInput {
    target_app: String,
    command: String,
    payload: serde_json::Value,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteCommandDocument {
    schema_version: u8,
    command_id: String,
    source_app: String,
    source_app_name: String,
    target_app: String,
    command: String,
    requested_at: String,
    payload: serde_json::Value,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteTimelineEvent {
    schema_version: u8,
    event_id: String,
    source_app: String,
    source_app_name: String,
    kind: String,
    title: String,
    detail: String,
    created_at: String,
    metadata: serde_json::Value,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuiteTimelineInput {
    kind: String,
    title: String,
    detail: String,
    metadata: serde_json::Value,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PulseRecordingHandoffInput {
    session_id: String,
    output_path: String,
    profile_id: Option<String>,
    profile_name: Option<String>,
    stopped_at: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PulseRecordingHandoffDocument {
    schema_version: u8,
    request_id: String,
    source_app: String,
    source_app_name: String,
    target_app: String,
    requested_at: String,
    recording: PulseRecordingHandoffRecording,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleTwitchStreamKey {
    stream_key: String,
    broadcaster_login: Option<String>,
    broadcaster_user_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleTwitchStreamKeyResponse {
    ok: bool,
    stream_key: Option<String>,
    broadcaster_login: Option<String>,
    broadcaster_user_id: Option<String>,
    error: Option<String>,
}

type ConsoleTwitchBroadcastReadiness = serde_json::Value;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PulseRecordingHandoffRecording {
    session_id: String,
    output_path: String,
    profile_id: Option<String>,
    profile_name: Option<String>,
    stopped_at: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteAppStatus {
    app_id: String,
    app_name: String,
    launch_name: String,
    bundle_identifier: String,
    installed: bool,
    running: bool,
    reachable: bool,
    stale: bool,
    discovery_file: String,
    pid: Option<u32>,
    api_url: Option<String>,
    health_url: Option<String>,
    updated_at: Option<String>,
    capabilities: Vec<String>,
    suite_session_id: Option<String>,
    activity: Option<String>,
    activity_detail: Option<String>,
    detail: String,
}

struct SuiteAppDefinition {
    app_id: &'static str,
    app_name: &'static str,
    launch_name: &'static str,
    bundle_identifier: &'static str,
}

#[derive(Clone)]
struct DailyLogWriter {
    directory: PathBuf,
}

struct AppRuntimeState {
    bind_addr: SocketAddr,
    configured_bind_addr: SocketAddr,
    port_fallback_active: bool,
    auth: SharedAuthConfig,
    settings_store: ProfileStore,
    data_dir: PathBuf,
    database_path: PathBuf,
    discovery_file: PathBuf,
    log_dir: PathBuf,
    pipeline_plan_path: PathBuf,
    pipeline_config_path: PathBuf,
    media_runner: Option<MediaRunnerSupervisor>,
    api_shutdown: Mutex<Option<oneshot::Sender<()>>>,
}

#[tauri::command]
fn api_config(state: tauri::State<'_, AppRuntimeState>) -> FrontendApiConfig {
    frontend_api_config(
        state.bind_addr,
        state.configured_bind_addr,
        state.port_fallback_active,
        &state.discovery_file,
        &state.auth.get(),
    )
}

#[tauri::command]
fn app_settings(state: tauri::State<'_, AppRuntimeState>) -> Result<FrontendAppSettings, String> {
    frontend_app_settings(&state).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_app_settings(
    state: tauri::State<'_, AppRuntimeState>,
    settings: AppSettings,
) -> Result<FrontendAppSettings, String> {
    let settings = state
        .settings_store
        .save_app_settings(settings)
        .map_err(|error| error.to_string())?;
    state.auth.update(AuthConfig {
        token: settings.api_token.clone(),
        dev_mode: settings.dev_auth_bypass,
    });
    write_api_discovery_file(
        &state.discovery_file,
        state.bind_addr,
        state.configured_bind_addr,
        state.port_fallback_active,
        &state.auth.get(),
    )
    .map_err(|error| error.to_string())?;
    write_app_log(
        &state.log_dir,
        "settings.saved",
        serde_json::json!({
            "restart_required": settings_restart_required(&settings, state.bind_addr),
            "dev_auth_bypass": settings.dev_auth_bypass,
        }),
    );
    frontend_app_settings(&state).map_err(|error| error.to_string())
}

#[tauri::command]
fn regenerate_api_token(
    state: tauri::State<'_, AppRuntimeState>,
) -> Result<FrontendAppSettings, String> {
    let mut settings = state
        .settings_store
        .app_settings()
        .map_err(|error| error.to_string())?;
    settings.api_token = Some(generate_token());
    let settings = state
        .settings_store
        .save_app_settings(settings)
        .map_err(|error| error.to_string())?;
    state.auth.update(AuthConfig {
        token: settings.api_token.clone(),
        dev_mode: settings.dev_auth_bypass,
    });
    write_api_discovery_file(
        &state.discovery_file,
        state.bind_addr,
        state.configured_bind_addr,
        state.port_fallback_active,
        &state.auth.get(),
    )
    .map_err(|error| error.to_string())?;
    write_app_log(
        &state.log_dir,
        "settings.api_token_regenerated",
        serde_json::json!({
            "dev_auth_bypass": settings.dev_auth_bypass,
        }),
    );
    frontend_app_settings(&state).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_data_directory(state: tauri::State<'_, AppRuntimeState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&state.data_dir)
            .spawn()
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("opening the data directory is not implemented on this platform".to_string())
    }
}

#[tauri::command]
fn launch_vaexcore_suite() -> Vec<SuiteLaunchResult> {
    VAEXCORE_SUITE_APPS
        .iter()
        .map(|app_name| launch_macos_app(app_name))
        .collect()
}

#[tauri::command]
fn suite_status() -> Vec<SuiteAppStatus> {
    suite_app_definitions()
        .iter()
        .map(suite_app_status)
        .collect()
}

#[tauri::command]
fn suite_session() -> Option<SuiteSessionDocument> {
    read_suite_session_document()
}

#[tauri::command]
fn start_suite_session(title: Option<String>) -> Result<SuiteSessionDocument, String> {
    let document = build_suite_session_document(title);
    write_suite_session_document(&document).map_err(|error| error.to_string())?;
    if let Err(error) = append_suite_timeline_event(
        "suite.session",
        "Suite session started",
        &format!("{} is active.", document.title),
        serde_json::json!({
            "sessionId": document.session_id,
            "ownerApp": document.owner_app,
        }),
    ) {
        tracing::warn!(%error, "failed to append suite timeline event");
    }
    Ok(document)
}

#[tauri::command]
fn send_suite_command(input: SuiteCommandInput) -> Result<SuiteCommandDocument, String> {
    write_suite_command(input).map_err(|error| error.to_string())
}

#[tauri::command]
fn suite_timeline(limit: Option<usize>) -> Vec<SuiteTimelineEvent> {
    read_suite_timeline_events(limit.unwrap_or(50))
}

#[tauri::command]
fn append_suite_timeline(input: SuiteTimelineInput) -> Result<(), String> {
    append_suite_timeline_event(&input.kind, &input.title, &input.detail, input.metadata)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn twitch_stream_key_from_console() -> Result<ConsoleTwitchStreamKey, String> {
    let discovery = read_suite_discovery_document(&suite_discovery_file("vaexcore-console"))
        .ok_or_else(|| "Console is not publishing a suite heartbeat yet.".to_string())?;
    let api_url = discovery
        .api_url
        .ok_or_else(|| "Console heartbeat does not include an API URL.".to_string())?;
    let endpoint = format!("{}/api/twitch/stream-key", api_url.trim_end_matches('/'));
    let (status, body) = local_http_get(&endpoint)?;
    let parsed = serde_json::from_str::<ConsoleTwitchStreamKeyResponse>(&body)
        .map_err(|error| format!("Console returned an unreadable stream key response: {error}"))?;

    if !parsed.ok || status != 200 {
        return Err(parsed
            .error
            .unwrap_or_else(|| format!("Console stream key request failed with HTTP {status}")));
    }

    let stream_key = parsed
        .stream_key
        .ok_or_else(|| "Console did not return a stream key.".to_string())?;
    if let Err(error) = append_suite_timeline_event(
        "twitch.stream_key",
        "Twitch key imported",
        "Studio imported a Twitch stream key from Console.",
        serde_json::json!({
            "broadcasterLogin": parsed.broadcaster_login,
            "broadcasterUserId": parsed.broadcaster_user_id,
        }),
    ) {
        tracing::warn!(%error, "failed to append suite timeline event");
    }

    Ok(ConsoleTwitchStreamKey {
        stream_key,
        broadcaster_login: parsed.broadcaster_login,
        broadcaster_user_id: parsed.broadcaster_user_id,
    })
}

#[tauri::command]
fn twitch_broadcast_readiness_from_console() -> Result<ConsoleTwitchBroadcastReadiness, String> {
    let discovery = read_suite_discovery_document(&suite_discovery_file("vaexcore-console"))
        .ok_or_else(|| "Console is not publishing a suite heartbeat yet.".to_string())?;
    let api_url = discovery
        .api_url
        .ok_or_else(|| "Console heartbeat does not include an API URL.".to_string())?;
    let endpoint = format!(
        "{}/api/twitch/broadcast-readiness",
        api_url.trim_end_matches('/')
    );
    let (status, body) = local_http_get(&endpoint)?;
    let parsed = serde_json::from_str::<serde_json::Value>(&body)
        .map_err(|error| format!("Console returned unreadable Twitch readiness: {error}"))?;

    if status != 200 {
        return Err(parsed
            .get("error")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("Console Twitch readiness failed with HTTP {status}")));
    }

    Ok(parsed)
}

#[tauri::command]
fn handoff_recording_to_pulse(recording: PulseRecordingHandoffInput) -> Vec<SuiteLaunchResult> {
    if let Err(error) = write_pulse_recording_handoff(recording) {
        return vec![SuiteLaunchResult {
            app_name: "vaexcore pulse".to_string(),
            ok: false,
            detail: format!("Could not write Pulse handoff: {error}"),
        }];
    }

    vec![launch_macos_app("vaexcore pulse")]
}

fn launch_macos_app(app_name: &str) -> SuiteLaunchResult {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("open")
            .args(["-a", app_name])
            .output()
        {
            Ok(output) if output.status.success() => SuiteLaunchResult {
                app_name: app_name.to_string(),
                ok: true,
                detail: "Launch requested.".to_string(),
            },
            Ok(output) => {
                let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
                SuiteLaunchResult {
                    app_name: app_name.to_string(),
                    ok: false,
                    detail: if detail.is_empty() {
                        format!("open exited with status {}.", output.status)
                    } else {
                        detail
                    },
                }
            }
            Err(error) => SuiteLaunchResult {
                app_name: app_name.to_string(),
                ok: false,
                detail: error.to_string(),
            },
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        SuiteLaunchResult {
            app_name: app_name.to_string(),
            ok: false,
            detail: "Launch Suite is only implemented for macOS Applications.".to_string(),
        }
    }
}

fn start_suite_discovery_heartbeat(bind_addr: SocketAddr) {
    let started_at = chrono::Utc::now().to_rfc3339();
    let api_url = format!("http://{bind_addr}");
    let ws_url = format!("ws://{bind_addr}/events");
    let health_url = format!("{api_url}/health");

    std::thread::spawn(move || loop {
        let session = read_suite_session_document();
        let document = SuiteDiscoveryDocument {
            schema_version: SUITE_DISCOVERY_SCHEMA_VERSION,
            app_id: "vaexcore-studio".to_string(),
            app_name: APP_NAME.to_string(),
            bundle_identifier: "com.vaexcore.studio".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
            started_at: started_at.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            api_url: Some(api_url.clone()),
            ws_url: Some(ws_url.clone()),
            health_url: Some(health_url.clone()),
            capabilities: vec![
                "studio.api".to_string(),
                "recording.control".to_string(),
                "pulse.recording.handoff".to_string(),
                "suite.commands".to_string(),
                "suite.session.owner".to_string(),
                "suite.status".to_string(),
                "suite.launcher".to_string(),
                "suite.timeline".to_string(),
                "twitch.stream_key.import".to_string(),
            ],
            launch_name: APP_NAME.to_string(),
            suite_session_id: session.as_ref().map(|session| session.session_id.clone()),
            activity: Some("control-room".to_string()),
            activity_detail: session
                .as_ref()
                .map(|session| format!("Coordinating {}", session.title))
                .or_else(|| Some("Ready to coordinate the suite".to_string())),
        };

        if let Err(error) = write_suite_discovery_document(&document) {
            tracing::warn!(%error, "failed to write suite discovery document");
        }

        std::thread::sleep(SUITE_DISCOVERY_HEARTBEAT_INTERVAL);
    });
}

fn build_suite_session_document(title: Option<String>) -> SuiteSessionDocument {
    let now = chrono::Utc::now().to_rfc3339();
    let session_id = read_suite_session_document()
        .map(|session| session.session_id)
        .unwrap_or_else(|| format!("suite-{}", chrono::Utc::now().timestamp_millis()));
    SuiteSessionDocument {
        schema_version: SUITE_DISCOVERY_SCHEMA_VERSION,
        session_id,
        title: title
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "VaexCore Suite Session".to_string()),
        status: "active".to_string(),
        owner_app: "vaexcore-studio".to_string(),
        created_at: read_suite_session_document()
            .map(|session| session.created_at)
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    }
}

fn ensure_suite_session() {
    if read_suite_session_document().is_some() {
        return;
    }
    if let Err(error) = write_suite_session_document(&build_suite_session_document(None)) {
        tracing::warn!(%error, "failed to initialize suite session");
    }
}

fn read_suite_session_document() -> Option<SuiteSessionDocument> {
    let contents = fs::read(suite_session_file()).ok()?;
    serde_json::from_slice(&contents).ok()
}

fn write_suite_session_document(document: &SuiteSessionDocument) -> std::io::Result<()> {
    fs::create_dir_all(suite_discovery_dir())?;
    fs::write(suite_session_file(), serde_json::to_vec_pretty(document)?)
}

fn write_suite_command(input: SuiteCommandInput) -> std::io::Result<SuiteCommandDocument> {
    let target_app = sanitize_suite_file_component(&input.target_app);
    let directory = suite_command_dir().join(&target_app);
    fs::create_dir_all(&directory)?;
    let requested_at = chrono::Utc::now().to_rfc3339();
    let command_id = format!(
        "{}-{}",
        sanitize_suite_file_component(&input.command),
        chrono::Utc::now().timestamp_millis()
    );
    let document = SuiteCommandDocument {
        schema_version: SUITE_DISCOVERY_SCHEMA_VERSION,
        command_id: command_id.clone(),
        source_app: "vaexcore-studio".to_string(),
        source_app_name: APP_NAME.to_string(),
        target_app: input.target_app,
        command: input.command,
        requested_at,
        payload: input.payload,
    };

    fs::write(
        directory.join(format!("{command_id}.json")),
        serde_json::to_vec_pretty(&document)?,
    )?;
    if let Err(error) = append_suite_timeline_event(
        "suite.command",
        "Suite command sent",
        &format!(
            "Studio sent {} to {}.",
            document.command, document.target_app
        ),
        serde_json::json!({
            "commandId": document.command_id,
            "targetApp": document.target_app,
            "command": document.command,
        }),
    ) {
        tracing::warn!(%error, "failed to append suite timeline event");
    }
    Ok(document)
}

fn write_suite_discovery_document(document: &SuiteDiscoveryDocument) -> std::io::Result<()> {
    let directory = suite_discovery_dir();
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.json", document.app_id));
    let serialized = serde_json::to_vec_pretty(document)?;
    fs::write(path, serialized)
}

fn write_pulse_recording_handoff(recording: PulseRecordingHandoffInput) -> std::io::Result<()> {
    let directory = suite_handoff_dir();
    fs::create_dir_all(&directory)?;
    let requested_at = chrono::Utc::now().to_rfc3339();
    let request_id = format!(
        "studio-recording-{}-{}",
        sanitize_handoff_id(&recording.session_id),
        chrono::Utc::now().timestamp_millis()
    );
    let document = PulseRecordingHandoffDocument {
        schema_version: SUITE_DISCOVERY_SCHEMA_VERSION,
        request_id,
        source_app: "vaexcore-studio".to_string(),
        source_app_name: APP_NAME.to_string(),
        target_app: "vaexcore-pulse".to_string(),
        requested_at,
        recording: PulseRecordingHandoffRecording {
            session_id: recording.session_id,
            output_path: recording.output_path,
            profile_id: recording.profile_id,
            profile_name: recording.profile_name,
            stopped_at: recording.stopped_at,
        },
    };

    let serialized = serde_json::to_vec_pretty(&document)?;
    fs::write(directory.join("pulse-recording-intake.json"), serialized)?;
    let payload = serde_json::to_value(&document).map_err(std::io::Error::other)?;
    write_suite_command(SuiteCommandInput {
        target_app: "vaexcore-pulse".to_string(),
        command: "open-review".to_string(),
        payload,
    })
    .map(|_| ())
}

fn suite_app_definitions() -> [SuiteAppDefinition; 3] {
    [
        SuiteAppDefinition {
            app_id: "vaexcore-studio",
            app_name: "vaexcore studio",
            launch_name: "vaexcore studio",
            bundle_identifier: "com.vaexcore.studio",
        },
        SuiteAppDefinition {
            app_id: "vaexcore-pulse",
            app_name: "vaexcore pulse",
            launch_name: "vaexcore pulse",
            bundle_identifier: "com.vaexil.vaexcore.pulse",
        },
        SuiteAppDefinition {
            app_id: "vaexcore-console",
            app_name: "vaexcore console",
            launch_name: "vaexcore console",
            bundle_identifier: "com.vaexil.vaexcore.console",
        },
    ]
}

fn suite_app_status(definition: &SuiteAppDefinition) -> SuiteAppStatus {
    let discovery_file = suite_discovery_file(definition.app_id);
    let installed = Path::new("/Applications")
        .join(format!("{}.app", definition.launch_name))
        .exists();
    let discovery = read_suite_discovery_document(&discovery_file);
    let pid = discovery.as_ref().map(|document| document.pid);
    let running = pid.is_some_and(process_is_running);
    let stale = suite_discovery_is_stale(&discovery_file);
    let reachable = discovery
        .as_ref()
        .and_then(|document| document.health_url.as_deref())
        .is_some_and(health_url_is_reachable);
    let detail = suite_status_detail(installed, discovery.is_some(), running, stale, reachable);

    SuiteAppStatus {
        app_id: definition.app_id.to_string(),
        app_name: discovery
            .as_ref()
            .map(|document| document.app_name.clone())
            .unwrap_or_else(|| definition.app_name.to_string()),
        launch_name: definition.launch_name.to_string(),
        bundle_identifier: definition.bundle_identifier.to_string(),
        installed,
        running,
        reachable,
        stale,
        discovery_file: discovery_file.display().to_string(),
        pid,
        api_url: discovery
            .as_ref()
            .and_then(|document| document.api_url.clone()),
        health_url: discovery
            .as_ref()
            .and_then(|document| document.health_url.clone()),
        updated_at: discovery
            .as_ref()
            .map(|document| document.updated_at.clone()),
        capabilities: discovery
            .as_ref()
            .map(|document| document.capabilities.clone())
            .unwrap_or_default(),
        suite_session_id: discovery
            .as_ref()
            .and_then(|document| document.suite_session_id.clone()),
        activity: discovery
            .as_ref()
            .and_then(|document| document.activity.clone()),
        activity_detail: discovery
            .as_ref()
            .and_then(|document| document.activity_detail.clone()),
        detail,
    }
}

fn read_suite_discovery_document(path: &Path) -> Option<SuiteDiscoveryDocument> {
    let contents = fs::read(path).ok()?;
    serde_json::from_slice(&contents).ok()
}

fn suite_status_detail(
    installed: bool,
    discovered: bool,
    running: bool,
    stale: bool,
    reachable: bool,
) -> String {
    if !installed {
        return "Install this app in /Applications.".to_string();
    }
    if !discovered {
        return "No suite heartbeat has been published yet.".to_string();
    }
    if !running {
        return "Heartbeat exists, but the app process is not running.".to_string();
    }
    if stale {
        return "The suite heartbeat is stale.".to_string();
    }
    if !reachable {
        return "The app is running, but its local health endpoint is not reachable.".to_string();
    }
    "Ready.".to_string()
}

fn suite_discovery_file(app_id: &str) -> PathBuf {
    suite_discovery_dir().join(format!("{app_id}.json"))
}

fn local_http_get(endpoint: &str) -> Result<(u16, String), String> {
    let without_scheme = endpoint
        .strip_prefix("http://")
        .ok_or_else(|| "Only local http:// Console endpoints are supported.".to_string())?;
    let (host_port, path) = without_scheme
        .split_once('/')
        .map(|(host_port, path)| (host_port, format!("/{path}")))
        .unwrap_or((without_scheme, "/".to_string()));
    let address = host_port
        .to_socket_addrs()
        .map_err(|error| format!("Could not resolve Console endpoint {host_port}: {error}"))?
        .next()
        .ok_or_else(|| format!("Could not resolve Console endpoint {host_port}"))?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("Could not connect to Console: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("Could not configure Console read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("Could not configure Console write timeout: {error}"))?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("Could not request Console stream key: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("Could not read Console response: {error}"))?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "Console returned a malformed HTTP response.".to_string())?;
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .ok_or_else(|| "Console returned a malformed HTTP status.".to_string())?;

    Ok((status, body.to_string()))
}

fn suite_handoff_dir() -> PathBuf {
    suite_discovery_dir().join("handoffs")
}

fn suite_session_file() -> PathBuf {
    suite_discovery_dir().join("session.json")
}

fn suite_timeline_file() -> PathBuf {
    suite_discovery_dir().join("timeline.jsonl")
}

fn suite_command_dir() -> PathBuf {
    suite_discovery_dir().join("commands")
}

fn append_suite_timeline_event(
    kind: &str,
    title: &str,
    detail: &str,
    metadata: serde_json::Value,
) -> std::io::Result<()> {
    fs::create_dir_all(suite_discovery_dir())?;
    let now = chrono::Utc::now().to_rfc3339();
    let event = SuiteTimelineEvent {
        schema_version: SUITE_DISCOVERY_SCHEMA_VERSION,
        event_id: format!(
            "studio-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            std::process::id()
        ),
        source_app: "vaexcore-studio".to_string(),
        source_app_name: APP_NAME.to_string(),
        kind: kind.to_string(),
        title: title.to_string(),
        detail: detail.to_string(),
        created_at: now,
        metadata,
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(suite_timeline_file())?;
    writeln!(file, "{}", serde_json::to_string(&event)?)?;
    Ok(())
}

fn read_suite_timeline_events(limit: usize) -> Vec<SuiteTimelineEvent> {
    let contents = match fs::read_to_string(suite_timeline_file()) {
        Ok(contents) => contents,
        Err(_) => return Vec::new(),
    };
    let mut events = contents
        .lines()
        .filter_map(|line| serde_json::from_str::<SuiteTimelineEvent>(line).ok())
        .collect::<Vec<_>>();
    events.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    events.truncate(limit);
    events
}

fn sanitize_suite_file_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "suite-command".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn suite_discovery_dir() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_data_dir())
        .join("Library")
        .join("Application Support")
        .join("vaexcore")
        .join("suite")
}

fn sanitize_handoff_id(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "recording".to_string()
    } else {
        trimmed.chars().take(80).collect()
    }
}

fn suite_discovery_is_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .map(|elapsed| elapsed > SUITE_DISCOVERY_STALE_AFTER)
        .unwrap_or(true)
}

fn process_is_running(pid: u32) -> bool {
    let pid_arg = pid.to_string();
    std::process::Command::new("ps")
        .args(["-p", pid_arg.as_str()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn health_url_is_reachable(url: &str) -> bool {
    let Some(authority) = http_url_authority(url) else {
        return false;
    };
    let Ok(mut addresses) = authority.to_socket_addrs() else {
        return false;
    };
    addresses
        .any(|address| TcpStream::connect_timeout(&address, Duration::from_millis(450)).is_ok())
}

fn http_url_authority(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    rest.split('/')
        .next()
        .filter(|authority| !authority.is_empty())
}

#[tauri::command]
fn capture_source_inventory(
    state: tauri::State<'_, AppRuntimeState>,
) -> Result<CaptureSourceInventory, String> {
    let settings = state
        .settings_store
        .app_settings()
        .map_err(|error| error.to_string())?;
    Ok(CaptureSourceInventory {
        candidates: capture_source_candidates(),
        selected: settings.capture_sources,
    })
}

#[tauri::command]
fn camera_permission_status() -> FrontendPermissionStatus {
    permission_status_response("camera")
}

#[tauri::command]
fn microphone_permission_status() -> FrontendPermissionStatus {
    permission_status_response("microphone")
}

#[tauri::command]
fn open_camera_privacy_settings() -> Result<(), String> {
    open_macos_privacy_settings("Privacy_Camera")
}

#[tauri::command]
fn open_microphone_privacy_settings() -> Result<(), String> {
    open_macos_privacy_settings("Privacy_Microphone")
}

#[tauri::command]
fn open_screen_recording_privacy_settings() -> Result<(), String> {
    open_macos_privacy_settings("Privacy_ScreenCapture")
}

#[tauri::command]
async fn preflight_snapshot(
    state: tauri::State<'_, AppRuntimeState>,
) -> Result<PreflightSnapshot, String> {
    let settings = state
        .settings_store
        .app_settings()
        .map_err(|error| error.to_string())?;
    let runner = state.media_runner.clone();
    let mut checks = vec![
        api_preflight_check(state.bind_addr),
        token_preflight_check(&state.auth.get()),
        output_folder_preflight_check(&settings.default_recording_profile.output_folder),
        screen_recording_preflight_check(&settings.capture_sources),
        camera_preflight_check(&settings.capture_sources),
        microphone_preflight_check(&settings.capture_sources),
        system_audio_preflight_check(&settings.capture_sources),
    ];

    checks.push(match runner {
        Some(runner) => match runner.health().await {
            Ok(()) => PreflightCheck {
                id: "media.sidecar".to_string(),
                label: "Media Runner".to_string(),
                status: PreflightStatus::Ready,
                detail: format!("Sidecar is reachable at {}.", runner.status_addr()),
            },
            Err(error) => PreflightCheck {
                id: "media.sidecar".to_string(),
                label: "Media Runner".to_string(),
                status: PreflightStatus::Warning,
                detail: format!("Sidecar is configured but not healthy: {error}"),
            },
        },
        None => PreflightCheck {
            id: "media.sidecar".to_string(),
            label: "Media Runner".to_string(),
            status: PreflightStatus::Warning,
            detail: "Sidecar is not running; Studio will use in-process dry-run media.".to_string(),
        },
    });

    Ok(PreflightSnapshot {
        overall: aggregate_preflight_status(&checks),
        checked_at: chrono::Utc::now(),
        checks,
    })
}

#[tauri::command]
fn export_profile_bundle(
    state: tauri::State<'_, AppRuntimeState>,
) -> Result<FrontendProfileBundleResult, String> {
    let bundle = state
        .settings_store
        .export_profile_bundle()
        .map_err(|error| error.to_string())?;
    let path = profile_bundle_path(&state);
    let result = FrontendProfileBundleResult {
        path: path.display().to_string(),
        recording_profiles: bundle.recording_profiles.len(),
        stream_destinations: bundle.stream_destinations.len(),
    };
    let serialized = serde_json::to_vec_pretty(&bundle).map_err(|error| error.to_string())?;
    std::fs::write(path, serialized).map_err(|error| error.to_string())?;
    write_app_log(
        &state.log_dir,
        "profiles.bundle_exported",
        serde_json::json!({
            "recording_profiles": result.recording_profiles,
            "stream_destinations": result.stream_destinations,
            "path": &result.path,
        }),
    );
    Ok(result)
}

#[tauri::command]
fn import_profile_bundle(
    state: tauri::State<'_, AppRuntimeState>,
) -> Result<FrontendProfileBundleResult, String> {
    let path = profile_bundle_path(&state);
    let contents = std::fs::read(&path).map_err(|error| error.to_string())?;
    let bundle: ProfileBundle =
        serde_json::from_slice(&contents).map_err(|error| error.to_string())?;
    let result = state
        .settings_store
        .import_profile_bundle(bundle)
        .map_err(|error| error.to_string())?;

    Ok(FrontendProfileBundleResult {
        path: path.display().to_string(),
        recording_profiles: result.recording_profiles,
        stream_destinations: result.stream_destinations,
    })
    .inspect(|result| {
        write_app_log(
            &state.log_dir,
            "profiles.bundle_imported",
            serde_json::json!({
                "recording_profiles": result.recording_profiles,
                "stream_destinations": result.stream_destinations,
                "path": &result.path,
            }),
        );
    })
}

#[tauri::command]
async fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    show_settings_window(&app).map_err(|error| error.to_string())
}

#[tauri::command]
async fn media_runner_info(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppRuntimeState>,
) -> Result<FrontendMediaRunnerInfo, String> {
    let runner = state.media_runner.clone();
    drop(state);

    let running = match &runner {
        Some(runner) => runner.health().await.is_ok(),
        None => false,
    };
    let executable_path = runner
        .as_ref()
        .map(MediaRunnerSupervisor::executable_path)
        .or_else(|| resolve_media_runner_path(&app));
    let bundled = executable_path
        .as_ref()
        .is_some_and(|path| is_bundled_media_runner_path(&app, path));

    Ok(FrontendMediaRunnerInfo {
        bundled,
        running,
        fallback_dry_run: !running,
        status_addr: runner
            .as_ref()
            .map(|runner| runner.status_addr().to_string()),
        executable_path: executable_path.map(|path| path.display().to_string()),
    })
}

pub fn run() {
    let log_dir = init_logging(&default_data_dir());

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
            let launch_suite = MenuItem::with_id(
                handle,
                MENU_LAUNCH_SUITE,
                "Launch vaexcore Suite",
                true,
                None::<&str>,
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
                    &launch_suite,
                    &PredefinedMenuItem::separator(handle)?,
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
            MENU_LAUNCH_SUITE => {
                std::thread::spawn(|| {
                    for result in launch_vaexcore_suite() {
                        if !result.ok {
                            tracing::warn!(
                                app_name = %result.app_name,
                                detail = %result.detail,
                                "suite app launch failed"
                            );
                        }
                    }
                });
            }
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
        .setup(move |app| {
            let default_auth = default_auth_from_env();
            let data_dir = app.path().app_data_dir()?;
            let database_path = data_dir.join("studio.sqlite");
            let settings_store = ProfileStore::open(&database_path)?;
            let settings = settings_store.initialize_app_settings(AppSettings {
                api_token: default_auth.token.clone(),
                dev_auth_bypass: default_auth.dev_mode,
                ..AppSettings::default()
            })?;
            let configured_bind_addr =
                settings_bind_addr(&settings).unwrap_or_else(default_bind_addr);
            let (api_listener, bind_addr, port_fallback_active) =
                bind_api_listener(configured_bind_addr)?;
            let auth = SharedAuthConfig::new(AuthConfig {
                token: settings.api_token.clone(),
                dev_mode: settings.dev_auth_bypass,
            });
            let (api_shutdown, shutdown_rx) = oneshot::channel::<()>();
            let discovery_file = data_dir.join("api-discovery.json");
            let pipeline_plan_path = data_dir.join("pipeline-plan.json");
            let pipeline_config_path = data_dir.join("pipeline-config.json");
            write_seed_pipeline_config(&pipeline_config_path)?;
            let media_runner = start_media_runner(app.handle(), &pipeline_config_path);
            write_api_discovery_file(
                &discovery_file,
                bind_addr,
                configured_bind_addr,
                port_fallback_active,
                &auth.get(),
            )?;
            ensure_suite_session();
            start_suite_discovery_heartbeat(bind_addr);
            write_app_log(
                &log_dir,
                "app.api.ready",
                serde_json::json!({
                    "api_url": format!("http://{bind_addr}"),
                    "ws_url": format!("ws://{bind_addr}/events"),
                    "configured_bind_addr": configured_bind_addr.to_string(),
                    "port_fallback_active": port_fallback_active,
                    "discovery_file": discovery_file.display().to_string(),
                    "pipeline_plan_path": pipeline_plan_path.display().to_string(),
                    "pipeline_config_path": pipeline_config_path.display().to_string(),
                }),
            );

            app.manage(AppRuntimeState {
                bind_addr,
                configured_bind_addr,
                port_fallback_active,
                auth: auth.clone(),
                settings_store,
                data_dir,
                database_path: database_path.clone(),
                discovery_file,
                log_dir: log_dir.clone(),
                pipeline_plan_path: pipeline_plan_path.clone(),
                pipeline_config_path: pipeline_config_path.clone(),
                media_runner: media_runner.clone(),
                api_shutdown: Mutex::new(Some(api_shutdown)),
            });

            tauri::async_runtime::spawn(async move {
                let config = ApiServerConfig {
                    bind_addr,
                    database_path,
                    auth,
                    media_runner,
                    pipeline_plan_path: Some(pipeline_plan_path),
                    pipeline_config_path: Some(pipeline_config_path),
                };

                let listener = match tokio::net::TcpListener::from_std(api_listener) {
                    Ok(listener) => listener,
                    Err(error) => {
                        tracing::error!(%error, "could not create async API listener");
                        return;
                    }
                };

                if let Err(error) = serve_listener_with_shutdown(config, listener, async {
                    let _ = shutdown_rx.await;
                })
                .await
                {
                    tracing::error!(%error, "local API stopped");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api_config,
            app_settings,
            save_app_settings,
            regenerate_api_token,
            open_data_directory,
            capture_source_inventory,
            camera_permission_status,
            microphone_permission_status,
            open_camera_privacy_settings,
            open_microphone_privacy_settings,
            open_screen_recording_privacy_settings,
            preflight_snapshot,
            export_profile_bundle,
            import_profile_bundle,
            open_settings_window,
            media_runner_info,
            launch_vaexcore_suite,
            suite_status,
            suite_session,
            start_suite_session,
            suite_timeline,
            append_suite_timeline,
            send_suite_command,
            twitch_stream_key_from_console,
            twitch_broadcast_readiness_from_console,
            handoff_recording_to_pulse
        ])
        .build(tauri::generate_context!())
        .expect("failed to build vaexcore studio")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                shutdown_runtime(app);
            }
            _ => {}
        });
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
    .inner_size(720.0, 820.0)
    .min_inner_size(560.0, 620.0)
    .resizable(true)
    .maximizable(false)
    .center()
    .focused(true)
    .build()?;

    Ok(())
}

fn quit_app(app: &tauri::AppHandle) {
    shutdown_runtime(app);
    app.exit(0);
}

fn shutdown_runtime(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppRuntimeState>() else {
        return;
    };

    if let Some(media_runner) = &state.media_runner {
        media_runner.shutdown();
    }

    let shutdown = {
        let mut guard = state
            .api_shutdown
            .lock()
            .expect("api shutdown mutex poisoned");
        guard.take()
    };

    if let Some(shutdown) = shutdown {
        let _ = shutdown.send(());
    }
}

fn start_media_runner(
    app: &tauri::AppHandle,
    pipeline_config_path: &Path,
) -> Option<MediaRunnerSupervisor> {
    let Some(executable_path) = resolve_media_runner_path(app) else {
        tracing::warn!("media-runner sidecar not found; using in-process dry-run media engine");
        return None;
    };
    let Some(status_addr) = reserve_sidecar_status_addr() else {
        tracing::warn!(
            "could not reserve a media-runner status port; using in-process dry-run media engine"
        );
        return None;
    };

    let dry_run = env_flag_enabled("VAEXCORE_MEDIA_RUNNER_DRY_RUN");
    let mut config = MediaRunnerConfig::dry_run(executable_path.clone(), status_addr);
    config.dry_run = dry_run;
    config.config_path = Some(pipeline_config_path.to_path_buf());
    match MediaRunnerSupervisor::start(config) {
        Ok(supervisor) => {
            tracing::info!(
                path = %executable_path.display(),
                %status_addr,
                dry_run,
                "media-runner sidecar started"
            );
            Some(supervisor)
        }
        Err(error) => {
            tracing::warn!(%error, "media-runner sidecar unavailable; using in-process dry-run media engine");
            None
        }
    }
}

fn env_flag_enabled(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn resolve_media_runner_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(path) = env::var("VAEXCORE_MEDIA_RUNNER_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
        tracing::warn!(
            path = %path.display(),
            "VAEXCORE_MEDIA_RUNNER_PATH does not point to a file"
        );
    }

    let executable_names = media_runner_executable_names();
    let mut candidates = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        push_media_runner_candidates(&mut candidates, &resource_dir, &executable_names);
        push_media_runner_candidates(
            &mut candidates,
            &resource_dir.join("binaries"),
            &executable_names,
        );
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_media_runner_candidates(&mut candidates, exe_dir, &executable_names);
            push_media_runner_candidates(
                &mut candidates,
                &exe_dir.join("binaries"),
                &executable_names,
            );
            push_media_runner_candidates(
                &mut candidates,
                &exe_dir.join("../Resources"),
                &executable_names,
            );
            push_media_runner_candidates(
                &mut candidates,
                &exe_dir.join("../Resources/binaries"),
                &executable_names,
            );
        }
    }

    if let Some(workspace_root) = workspace_root_from_manifest() {
        let unsuffixed_name = media_runner_unsuffixed_name();
        candidates.push(workspace_root.join("target/debug").join(&unsuffixed_name));
        candidates.push(workspace_root.join("target/release").join(&unsuffixed_name));
        push_media_runner_candidates(
            &mut candidates,
            &workspace_root.join("apps/desktop/src-tauri/binaries"),
            &executable_names,
        );
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn push_media_runner_candidates(
    candidates: &mut Vec<PathBuf>,
    directory: &std::path::Path,
    executable_names: &[String],
) {
    for executable_name in executable_names {
        candidates.push(directory.join(executable_name));
    }
}

fn media_runner_executable_names() -> Vec<String> {
    let extension = if cfg!(windows) { ".exe" } else { "" };
    vec![
        format!("media-runner-{}{}", media_runner_target_triple(), extension),
        format!("media-runner{extension}"),
    ]
}

fn media_runner_unsuffixed_name() -> String {
    if cfg!(windows) {
        "media-runner.exe".to_string()
    } else {
        "media-runner".to_string()
    }
}

fn media_runner_target_triple() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else {
        "unknown"
    }
}

fn is_bundled_media_runner_path(app: &tauri::AppHandle, path: &std::path::Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };

    if let Ok(resource_dir) = app.path().resource_dir() {
        if canonicalized_contains(&resource_dir, &path) {
            return true;
        }
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            return canonicalized_contains(exe_dir, &path);
        }
    }

    false
}

fn canonicalized_contains(base: &std::path::Path, path: &std::path::Path) -> bool {
    base.canonicalize()
        .map(|base| path.starts_with(base))
        .unwrap_or(false)
}

fn workspace_root_from_manifest() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.ancestors().nth(3).map(PathBuf::from)
}

fn init_logging(data_dir: &Path) -> PathBuf {
    let log_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let file_writer = DailyLogWriter {
        directory: log_dir.clone(),
    };
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(file_writer);
    let stderr_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(std::io::stderr);

    match tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
    {
        Ok(()) => tracing::info!(log_dir = %log_dir.display(), "structured logging initialized"),
        Err(error) => eprintln!("failed to initialize structured logging: {error}"),
    };
    write_app_log(
        &log_dir,
        "app.logging.initialized",
        serde_json::json!({
            "log_dir": log_dir.display().to_string(),
        }),
    );

    log_dir
}

fn default_data_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "vaexcore", "studio")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".vaexcore-studio"))
}

impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for DailyLogWriter {
    type Writer = Box<dyn Write + Send + 'writer>;

    fn make_writer(&'writer self) -> Self::Writer {
        let _ = std::fs::create_dir_all(&self.directory);
        let path = daily_log_path(&self.directory);
        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(file) => Box::new(file),
            Err(_) => Box::new(std::io::sink()),
        }
    }
}

fn write_app_log(log_dir: &Path, event: &str, fields: serde_json::Value) {
    let _ = std::fs::create_dir_all(log_dir);
    let entry = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "level": "info",
        "target": "vaexcore_studio_desktop",
        "event": event,
        "fields": fields,
    });

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(daily_log_path(log_dir))
    {
        let _ = writeln!(file, "{entry}");
    }
}

fn daily_log_path(log_dir: &Path) -> PathBuf {
    let date = chrono::Utc::now().format("%Y-%m-%d");
    log_dir.join(format!("studio-{date}.jsonl"))
}

fn capture_source_candidates() -> Vec<CaptureSourceCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(display_source_candidates());
    candidates.extend(window_source_candidates());
    candidates.extend(camera_source_candidates());
    candidates.extend(microphone_source_candidates());
    candidates.push(CaptureSourceCandidate {
        id: "system-audio:placeholder".to_string(),
        kind: CaptureSourceKind::SystemAudio,
        name: "System Audio".to_string(),
        available: false,
        notes: Some("System audio capture is a future macOS pipeline milestone.".to_string()),
    });
    candidates
}

#[cfg(target_os = "macos")]
fn display_source_candidates() -> Vec<CaptureSourceCandidate> {
    let mut displays = [0_u32; 16];
    let mut display_count = 0_u32;
    let result = unsafe {
        CGGetActiveDisplayList(
            displays.len() as u32,
            displays.as_mut_ptr(),
            &mut display_count,
        )
    };
    if result != 0 || display_count == 0 {
        return fallback_display_candidates();
    }

    let main_display = unsafe { CGMainDisplayID() };
    displays
        .iter()
        .copied()
        .take(display_count as usize)
        .enumerate()
        .map(|(index, display)| {
            let width = unsafe { CGDisplayPixelsWide(display) };
            let height = unsafe { CGDisplayPixelsHigh(display) };
            let is_main = display == main_display;
            CaptureSourceCandidate {
                id: if is_main {
                    "display:main".to_string()
                } else {
                    format!("display:{display}")
                },
                kind: CaptureSourceKind::Display,
                name: if is_main {
                    format!("Main Display ({width}x{height})")
                } else {
                    format!("Display {} ({width}x{height})", index + 1)
                },
                available: true,
                notes: None,
            }
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn display_source_candidates() -> Vec<CaptureSourceCandidate> {
    fallback_display_candidates()
}

fn fallback_display_candidates() -> Vec<CaptureSourceCandidate> {
    vec![CaptureSourceCandidate {
        id: "display:main".to_string(),
        kind: CaptureSourceKind::Display,
        name: "Main Display".to_string(),
        available: true,
        notes: None,
    }]
}

#[cfg(target_os = "macos")]
fn window_source_candidates() -> Vec<CaptureSourceCandidate> {
    const KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
    const KCG_NULL_WINDOW_ID: u32 = 0;

    let array = unsafe {
        CGWindowListCopyWindowInfo(KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY, KCG_NULL_WINDOW_ID)
    };
    if array.is_null() {
        return fallback_window_candidates();
    }

    let count = unsafe { CFArrayGetCount(array) };
    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for index in 0..count.min(64) {
        let dictionary = unsafe { CFArrayGetValueAtIndex(array, index) };
        if dictionary.is_null() {
            continue;
        }

        let layer = unsafe { cf_dictionary_i32(dictionary, kCGWindowLayer) }.unwrap_or(0);
        if layer != 0 {
            continue;
        }

        let Some(window_number) = (unsafe { cf_dictionary_i32(dictionary, kCGWindowNumber) })
        else {
            continue;
        };
        let Some(owner) = (unsafe { cf_dictionary_string(dictionary, kCGWindowOwnerName) }) else {
            continue;
        };
        let title = unsafe { cf_dictionary_string(dictionary, kCGWindowName) };
        let name = match title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(title) => format!("{owner} - {title}"),
            None => format!("{owner} Window"),
        };

        if !seen.insert(name.clone()) {
            continue;
        }

        candidates.push(CaptureSourceCandidate {
            id: format!("window:{window_number}"),
            kind: CaptureSourceKind::Window,
            name,
            available: macos_screen_recording_granted(),
            notes: if macos_screen_recording_granted() {
                None
            } else {
                Some("Grant Screen Recording permission to capture this window.".to_string())
            },
        });
    }

    unsafe { CFRelease(array) };

    let mut all_candidates = fallback_window_candidates();
    all_candidates.extend(candidates);
    all_candidates
}

#[cfg(not(target_os = "macos"))]
fn window_source_candidates() -> Vec<CaptureSourceCandidate> {
    fallback_window_candidates()
}

fn fallback_window_candidates() -> Vec<CaptureSourceCandidate> {
    vec![CaptureSourceCandidate {
        id: "window:selected".to_string(),
        kind: CaptureSourceKind::Window,
        name: "Window Capture".to_string(),
        available: true,
        notes: Some("Window selection is provided by the active media backend.".to_string()),
    }]
}

#[cfg(target_os = "macos")]
fn camera_source_candidates() -> Vec<CaptureSourceCandidate> {
    let names = system_profiler_device_names("SPCameraDataType", |item| {
        item.get("_name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    });
    let mut candidates = fallback_camera_candidates();
    if names.is_empty() {
        return candidates;
    }

    candidates.extend(
        names
            .into_iter()
            .enumerate()
            .map(|(index, name)| CaptureSourceCandidate {
                id: format!("camera:{index}"),
                kind: CaptureSourceKind::Camera,
                name,
                available: true,
                notes: permission_note("camera"),
            }),
    );
    candidates
}

#[cfg(not(target_os = "macos"))]
fn camera_source_candidates() -> Vec<CaptureSourceCandidate> {
    fallback_camera_candidates()
}

fn fallback_camera_candidates() -> Vec<CaptureSourceCandidate> {
    vec![CaptureSourceCandidate {
        id: "camera:default".to_string(),
        kind: CaptureSourceKind::Camera,
        name: "Default Camera".to_string(),
        available: true,
        notes: permission_note("camera"),
    }]
}

#[cfg(target_os = "macos")]
fn microphone_source_candidates() -> Vec<CaptureSourceCandidate> {
    let names = system_profiler_device_names("SPAudioDataType", |item| {
        let inputs = item
            .get("coreaudio_device_input")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default();
        if inputs <= 0 {
            return None;
        }
        item.get("_name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    });
    let mut candidates = fallback_microphone_candidates();
    if names.is_empty() {
        return candidates;
    }

    candidates.extend(
        names
            .into_iter()
            .enumerate()
            .map(|(index, name)| CaptureSourceCandidate {
                id: format!("microphone:{index}"),
                kind: CaptureSourceKind::Microphone,
                name,
                available: true,
                notes: permission_note("microphone"),
            }),
    );
    candidates
}

#[cfg(not(target_os = "macos"))]
fn microphone_source_candidates() -> Vec<CaptureSourceCandidate> {
    fallback_microphone_candidates()
}

fn fallback_microphone_candidates() -> Vec<CaptureSourceCandidate> {
    vec![CaptureSourceCandidate {
        id: "microphone:default".to_string(),
        kind: CaptureSourceKind::Microphone,
        name: "Default Microphone".to_string(),
        available: true,
        notes: permission_note("microphone"),
    }]
}

fn permission_note(service: &str) -> Option<String> {
    let status = media_permission_status(service);
    (status.status != "authorized").then_some(status.detail)
}

#[cfg(target_os = "macos")]
fn system_profiler_device_names<F>(data_type: &str, map_item: F) -> Vec<String>
where
    F: Fn(&serde_json::Value) -> Option<String>,
{
    let output = match std::process::Command::new("system_profiler")
        .args(["-json", data_type])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return Vec::new(),
    };
    let json = match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
        Ok(json) => json,
        Err(_) => return Vec::new(),
    };

    let mut names = Vec::new();
    collect_system_profiler_names(json.get(data_type), &map_item, &mut names);
    names.sort();
    names.dedup();
    names
}

#[cfg(target_os = "macos")]
fn collect_system_profiler_names<F>(
    value: Option<&serde_json::Value>,
    map_item: &F,
    names: &mut Vec<String>,
) where
    F: Fn(&serde_json::Value) -> Option<String>,
{
    match value {
        Some(serde_json::Value::Array(items)) => {
            for item in items {
                if let Some(name) = map_item(item)
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                {
                    names.push(name);
                }
                collect_system_profiler_names(item.get("_items"), map_item, names);
            }
        }
        Some(serde_json::Value::Object(object)) => {
            for nested in object.values() {
                collect_system_profiler_names(Some(nested), map_item, names);
            }
        }
        _ => {}
    }
}

fn api_preflight_check(bind_addr: SocketAddr) -> PreflightCheck {
    match TcpStream::connect_timeout(&bind_addr, Duration::from_millis(250)) {
        Ok(_) => PreflightCheck {
            id: "api.local".to_string(),
            label: "Local API".to_string(),
            status: PreflightStatus::Ready,
            detail: format!("Listening at http://{bind_addr}."),
        },
        Err(error) => PreflightCheck {
            id: "api.local".to_string(),
            label: "Local API".to_string(),
            status: PreflightStatus::Blocked,
            detail: format!("Could not connect to http://{bind_addr}: {error}"),
        },
    }
}

fn token_preflight_check(auth: &AuthConfig) -> PreflightCheck {
    if auth.dev_mode {
        return PreflightCheck {
            id: "api.auth".to_string(),
            label: "API Token".to_string(),
            status: PreflightStatus::Warning,
            detail: "Dev auth bypass is enabled.".to_string(),
        };
    }

    match auth
        .token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
    {
        Some(_) => PreflightCheck {
            id: "api.auth".to_string(),
            label: "API Token".to_string(),
            status: PreflightStatus::Ready,
            detail: "Token auth is configured.".to_string(),
        },
        None => PreflightCheck {
            id: "api.auth".to_string(),
            label: "API Token".to_string(),
            status: PreflightStatus::Blocked,
            detail: "Dev auth bypass is disabled and no API token is configured.".to_string(),
        },
    }
}

fn output_folder_preflight_check(output_folder: &str) -> PreflightCheck {
    let path = expand_user_path(output_folder);
    if let Err(error) = std::fs::create_dir_all(&path) {
        return PreflightCheck {
            id: "recording.output_folder".to_string(),
            label: "Output Folder".to_string(),
            status: PreflightStatus::Blocked,
            detail: format!("Could not create '{}': {error}", path.display()),
        };
    }

    let probe = path.join(".vaexcore-preflight-write-test");
    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(mut file) => {
            let write_result = file.write_all(b"ok");
            drop(file);
            let _ = std::fs::remove_file(&probe);
            match write_result {
                Ok(()) => PreflightCheck {
                    id: "recording.output_folder".to_string(),
                    label: "Output Folder".to_string(),
                    status: PreflightStatus::Ready,
                    detail: format!("Writable: {}", path.display()),
                },
                Err(error) => PreflightCheck {
                    id: "recording.output_folder".to_string(),
                    label: "Output Folder".to_string(),
                    status: PreflightStatus::Blocked,
                    detail: format!("Could not write to '{}': {error}", path.display()),
                },
            }
        }
        Err(error) => PreflightCheck {
            id: "recording.output_folder".to_string(),
            label: "Output Folder".to_string(),
            status: PreflightStatus::Blocked,
            detail: format!(
                "Could not open write probe in '{}': {error}",
                path.display()
            ),
        },
    }
}

fn screen_recording_preflight_check(sources: &[CaptureSourceSelection]) -> PreflightCheck {
    if !source_enabled(
        sources,
        &[CaptureSourceKind::Display, CaptureSourceKind::Window],
    ) {
        return not_required_check("macos.screen_recording", "Screen Recording");
    }

    if macos_screen_recording_granted() {
        PreflightCheck {
            id: "macos.screen_recording".to_string(),
            label: "Screen Recording".to_string(),
            status: PreflightStatus::Ready,
            detail: "macOS reports screen capture permission is available.".to_string(),
        }
    } else {
        PreflightCheck {
            id: "macos.screen_recording".to_string(),
            label: "Screen Recording".to_string(),
            status: PreflightStatus::Blocked,
            detail: "Grant Screen Recording permission in macOS Privacy & Security.".to_string(),
        }
    }
}

fn camera_preflight_check(sources: &[CaptureSourceSelection]) -> PreflightCheck {
    if !source_enabled(sources, &[CaptureSourceKind::Camera]) {
        return not_required_check("macos.camera", "Camera");
    }

    permission_preflight_check("macos.camera", "Camera", "camera")
}

fn microphone_preflight_check(sources: &[CaptureSourceSelection]) -> PreflightCheck {
    if !source_enabled(sources, &[CaptureSourceKind::Microphone]) {
        return not_required_check("macos.microphone", "Microphone");
    }

    permission_preflight_check("macos.microphone", "Microphone", "microphone")
}

fn permission_preflight_check(id: &str, label: &str, service: &str) -> PreflightCheck {
    let permission = media_permission_status(service);
    let status = match permission.status.as_str() {
        "authorized" => PreflightStatus::Ready,
        "denied" | "restricted" => PreflightStatus::Blocked,
        "not_determined" => PreflightStatus::Warning,
        _ => PreflightStatus::Unknown,
    };

    PreflightCheck {
        id: id.to_string(),
        label: label.to_string(),
        status,
        detail: permission.detail,
    }
}

fn system_audio_preflight_check(sources: &[CaptureSourceSelection]) -> PreflightCheck {
    if !source_enabled(sources, &[CaptureSourceKind::SystemAudio]) {
        return not_required_check("macos.system_audio", "System Audio");
    }

    PreflightCheck {
        id: "macos.system_audio".to_string(),
        label: "System Audio".to_string(),
        status: PreflightStatus::Blocked,
        detail: "System audio capture is not implemented in the MVP media pipeline.".to_string(),
    }
}

fn not_required_check(id: &str, label: &str) -> PreflightCheck {
    PreflightCheck {
        id: id.to_string(),
        label: label.to_string(),
        status: PreflightStatus::NotRequired,
        detail: "No enabled capture source requires this permission.".to_string(),
    }
}

fn source_enabled(sources: &[CaptureSourceSelection], kinds: &[CaptureSourceKind]) -> bool {
    sources
        .iter()
        .any(|source| source.enabled && kinds.iter().any(|kind| kind == &source.kind))
}

fn permission_status_response(service: &str) -> FrontendPermissionStatus {
    media_permission_status(service)
}

fn media_permission_status(service: &str) -> FrontendPermissionStatus {
    let label = match service {
        "camera" => "Camera",
        "microphone" => "Microphone",
        _ => "Media",
    };

    match macos_media_permission_status(service) {
        Some(status) => {
            let detail = match status.as_str() {
                "authorized" => format!("{label} permission is authorized."),
                "denied" => format!("{label} permission is denied in macOS Privacy & Security."),
                "restricted" => {
                    format!("{label} permission is restricted by macOS policy.")
                }
                "not_determined" => {
                    format!("{label} permission has not been requested yet.")
                }
                _ => format!("{label} permission status is unknown."),
            };
            FrontendPermissionStatus {
                service: service.to_string(),
                status,
                detail,
            }
        }
        None => FrontendPermissionStatus {
            service: service.to_string(),
            status: "unknown".to_string(),
            detail: format!("{label} permission status is unavailable on this platform."),
        },
    }
}

#[cfg(target_os = "macos")]
fn macos_media_permission_status(service: &str) -> Option<String> {
    let media_type = match service {
        "camera" => unsafe { AVMediaTypeVideo },
        "microphone" => unsafe { AVMediaTypeAudio },
        _ => return None,
    };
    if media_type.is_null() {
        return None;
    }

    let class = unsafe { objc_getClass(b"AVCaptureDevice\0".as_ptr().cast::<c_char>()) };
    let selector = unsafe {
        sel_registerName(
            b"authorizationStatusForMediaType:\0"
                .as_ptr()
                .cast::<c_char>(),
        )
    };
    if class.is_null() || selector.is_null() {
        return None;
    }

    let send: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void) -> isize =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    let status = unsafe { send(class, selector, media_type) };
    Some(
        match status {
            0 => "not_determined",
            1 => "restricted",
            2 => "denied",
            3 => "authorized",
            _ => "unknown",
        }
        .to_string(),
    )
}

#[cfg(not(target_os = "macos"))]
fn macos_media_permission_status(_service: &str) -> Option<String> {
    None
}

fn open_macos_privacy_settings(pane: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(format!(
                "x-apple.systempreferences:com.apple.preference.security?{pane}"
            ))
            .spawn()
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = pane;
        Err("privacy settings are only implemented on macOS".to_string())
    }
}

#[cfg(target_os = "macos")]
unsafe fn cf_dictionary_value(
    dictionary: *const c_void,
    key: *const c_void,
) -> Option<*const c_void> {
    if dictionary.is_null() || key.is_null() {
        return None;
    }

    let mut value = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dictionary, key, &mut value) == 0 || value.is_null() {
        None
    } else {
        Some(value)
    }
}

#[cfg(target_os = "macos")]
unsafe fn cf_dictionary_i32(dictionary: *const c_void, key: *const c_void) -> Option<i32> {
    const K_CF_NUMBER_SINT32_TYPE: i32 = 3;

    let value = cf_dictionary_value(dictionary, key)?;
    let mut number = 0_i32;
    if CFNumberGetValue(
        value,
        K_CF_NUMBER_SINT32_TYPE,
        (&mut number as *mut i32).cast::<c_void>(),
    ) == 0
    {
        None
    } else {
        Some(number)
    }
}

#[cfg(target_os = "macos")]
unsafe fn cf_dictionary_string(dictionary: *const c_void, key: *const c_void) -> Option<String> {
    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    let value = cf_dictionary_value(dictionary, key)?;
    let length = CFStringGetLength(value);
    if length <= 0 {
        return None;
    }

    let mut buffer = vec![0_i8; (length as usize * 4) + 1];
    if CFStringGetCString(
        value,
        buffer.as_mut_ptr(),
        buffer.len() as isize,
        K_CF_STRING_ENCODING_UTF8,
    ) == 0
    {
        return None;
    }

    Some(
        std::ffi::CStr::from_ptr(buffer.as_ptr())
            .to_string_lossy()
            .trim()
            .to_string(),
    )
    .filter(|value| !value.is_empty())
}

fn aggregate_preflight_status(checks: &[PreflightCheck]) -> PreflightStatus {
    if checks
        .iter()
        .any(|check| check.status == PreflightStatus::Blocked)
    {
        PreflightStatus::Blocked
    } else if checks
        .iter()
        .any(|check| check.status == PreflightStatus::Warning)
    {
        PreflightStatus::Warning
    } else if checks
        .iter()
        .any(|check| check.status == PreflightStatus::Unknown)
    {
        PreflightStatus::Unknown
    } else {
        PreflightStatus::Ready
    }
}

fn expand_user_path(value: &str) -> PathBuf {
    if value == "~" {
        return home_dir();
    }

    if let Some(rest) = value.strip_prefix("~/") {
        return home_dir().join(rest);
    }

    PathBuf::from(value)
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(target_os = "macos")]
fn macos_screen_recording_granted() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

#[cfg(not(target_os = "macos"))]
fn macos_screen_recording_granted() -> bool {
    true
}

fn profile_bundle_path(state: &AppRuntimeState) -> PathBuf {
    state.data_dir.join("profile-bundle.json")
}

fn write_seed_pipeline_config(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = serde_json::json!({
        "dry_run": false,
        "status_addr": null,
        "pipeline_name": "ffmpeg-rtmp",
        "pipeline": null,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&config)?)?;
    Ok(())
}

fn reserve_sidecar_status_addr() -> Option<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    listener.local_addr().ok()
}

fn bind_api_listener(
    configured_bind_addr: SocketAddr,
) -> std::io::Result<(TcpListener, SocketAddr, bool)> {
    match TcpListener::bind(configured_bind_addr) {
        Ok(listener) => {
            listener.set_nonblocking(true)?;
            Ok((listener, configured_bind_addr, false))
        }
        Err(error) => {
            tracing::warn!(
                %configured_bind_addr,
                %error,
                "configured API port unavailable; binding fallback port"
            );
            let fallback_addr = SocketAddr::new(configured_bind_addr.ip(), 0);
            let listener = TcpListener::bind(fallback_addr)?;
            listener.set_nonblocking(true)?;
            let active_addr = listener.local_addr()?;
            tracing::info!(%active_addr, "API fallback port selected");
            Ok((listener, active_addr, true))
        }
    }
}

fn write_api_discovery_file(
    discovery_file: &std::path::Path,
    bind_addr: SocketAddr,
    configured_bind_addr: SocketAddr,
    port_fallback_active: bool,
    auth: &AuthConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = discovery_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let document = ApiDiscoveryDocument {
        service: APP_NAME.to_string(),
        api_url: format!("http://{bind_addr}"),
        ws_url: format!("ws://{bind_addr}/events"),
        bind_addr: bind_addr.to_string(),
        configured_bind_addr: configured_bind_addr.to_string(),
        port_fallback_active,
        auth_required: auth.auth_required(),
        dev_auth_bypass: auth.dev_mode,
        pid: std::process::id(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    std::fs::write(discovery_file, serde_json::to_vec_pretty(&document)?)?;
    Ok(())
}

fn frontend_api_config(
    bind_addr: std::net::SocketAddr,
    configured_bind_addr: std::net::SocketAddr,
    port_fallback_active: bool,
    discovery_file: &std::path::Path,
    auth: &AuthConfig,
) -> FrontendApiConfig {
    let api_url = format!("http://{bind_addr}");
    let ws_url = format!("ws://{bind_addr}/events");
    let configured_api_url = format!("http://{configured_bind_addr}");
    let configured_ws_url = format!("ws://{configured_bind_addr}/events");
    FrontendApiConfig {
        api_url,
        ws_url,
        configured_api_url,
        configured_ws_url,
        bind_addr: bind_addr.to_string(),
        configured_bind_addr: configured_bind_addr.to_string(),
        port_fallback_active,
        discovery_file: discovery_file.display().to_string(),
        token: auth.token.clone(),
        dev_auth_bypass: auth.dev_mode,
    }
}

fn frontend_app_settings(
    state: &tauri::State<'_, AppRuntimeState>,
) -> Result<FrontendAppSettings, vaexcore_api::StoreError> {
    let settings = state.settings_store.app_settings()?;
    let api = frontend_api_config(
        state.bind_addr,
        state.configured_bind_addr,
        state.port_fallback_active,
        &state.discovery_file,
        &state.auth.get(),
    );
    Ok(FrontendAppSettings {
        restart_required: settings_restart_required(&settings, state.bind_addr),
        settings,
        api_url: api.api_url,
        ws_url: api.ws_url,
        configured_api_url: api.configured_api_url,
        configured_ws_url: api.configured_ws_url,
        port_fallback_active: api.port_fallback_active,
        data_dir: state.data_dir.display().to_string(),
        database_path: state.database_path.display().to_string(),
        discovery_file: state.discovery_file.display().to_string(),
        log_dir: state.log_dir.display().to_string(),
        pipeline_plan_path: state.pipeline_plan_path.display().to_string(),
        pipeline_config_path: state.pipeline_config_path.display().to_string(),
    })
}

fn settings_bind_addr(settings: &AppSettings) -> Option<SocketAddr> {
    format!("{}:{}", settings.api_host, settings.api_port)
        .parse()
        .ok()
}

fn settings_restart_required(settings: &AppSettings, active_addr: SocketAddr) -> bool {
    settings_bind_addr(settings)
        .map(|settings_addr| settings_addr != active_addr)
        .unwrap_or(true)
}
