mod config;
mod indexer;
mod resolver;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_plugin_updater::UpdaterExt;

struct AppState {
    prompts: Arc<Mutex<Vec<indexer::Prompt>>>,
}

fn app_version() -> &'static str {
    const CONF: &str = include_str!("../tauri.conf.json");
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        serde_json::from_str::<serde_json::Value>(CONF)
            .ok()
            .and_then(|v| v["version"].as_str().map(String::from))
            .unwrap_or_else(|| "0.0.0".to_string())
    })
}

fn app_identifier() -> &'static str {
    const CONF: &str = include_str!("../tauri.conf.json");
    static IDENTIFIER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    IDENTIFIER.get_or_init(|| {
        serde_json::from_str::<serde_json::Value>(CONF)
            .ok()
            .and_then(|v| v["identifier"].as_str().map(String::from))
            .unwrap_or_else(|| "com.jonasgantner.promptpicker".to_string())
    })
}

/// PID of the app that was frontmost before we showed our window.
static PREVIOUS_APP_PID: AtomicI32 = AtomicI32::new(-1);
static LAST_PASTE_REPORT: OnceLock<Mutex<String>> = OnceLock::new();

const PASTE_HANDOFF_DELAY_MS: u64 = 80;
const PASTE_FOCUS_RETRY_MS: u64 = 20;
const PASTE_FOCUS_TIMEOUT_MS: u64 = 1_000;

#[derive(Debug, Clone)]
struct PasteDiagnostics {
    accessibility_trusted: bool,
    app_identifier: String,
    app_version: String,
    executable: String,
    previous_pid: i32,
    previous_app: Option<String>,
    frontmost_pid: Option<i32>,
    frontmost_app: Option<String>,
    last_paste_report: String,
}

fn last_paste_report_store() -> &'static Mutex<String> {
    LAST_PASTE_REPORT.get_or_init(|| Mutex::new("No paste attempt recorded yet.".to_string()))
}

fn set_last_paste_report(report: String) {
    if let Ok(mut value) = last_paste_report_store().lock() {
        *value = report;
    }
}

fn get_last_paste_report() -> String {
    last_paste_report_store()
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| "Paste diagnostics lock was poisoned.".to_string())
}

fn paste_diagnostics() -> PasteDiagnostics {
    let previous_pid = PREVIOUS_APP_PID.load(Ordering::Relaxed);

    PasteDiagnostics {
        accessibility_trusted: accessibility_trusted(),
        app_identifier: app_identifier().to_string(),
        app_version: app_version().to_string(),
        executable: std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|e| format!("unknown: {e}")),
        previous_pid,
        previous_app: app_summary(previous_pid),
        frontmost_pid: frontmost_pid(),
        frontmost_app: frontmost_pid().and_then(app_summary),
        last_paste_report: get_last_paste_report(),
    }
}

fn paste_diagnostics_text() -> String {
    let diagnostics = paste_diagnostics();
    format!(
        "Prompt Picker paste diagnostics\n\
         app_identifier: {}\n\
         app_version: {}\n\
         executable: {}\n\
         accessibility_trusted: {}\n\
         previous_pid: {}\n\
         previous_app: {}\n\
         frontmost_pid: {}\n\
         frontmost_app: {}\n\
         launch_at_login_enabled: {}\n\n\
         last_paste_report:\n{}",
        diagnostics.app_identifier,
        diagnostics.app_version,
        diagnostics.executable,
        diagnostics.accessibility_trusted,
        diagnostics.previous_pid,
        diagnostics
            .previous_app
            .unwrap_or_else(|| "unknown".to_string()),
        diagnostics
            .frontmost_pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        diagnostics
            .frontmost_app
            .unwrap_or_else(|| "unknown".to_string()),
        launch_at_login_enabled(),
        diagnostics.last_paste_report,
    )
}

#[cfg(target_os = "macos")]
mod macos_focus {
    use super::*;
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> u8;
    }

    fn nsstring_to_string(value: *mut Object) -> Option<String> {
        if value.is_null() {
            return None;
        }

        unsafe {
            let ptr: *const c_char = msg_send![value, UTF8String];
            if ptr.is_null() {
                return None;
            }
            Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    }

    pub fn get_frontmost_pid() -> Option<i32> {
        unsafe {
            let cls = Class::get("NSWorkspace")?;
            let workspace: *mut Object = msg_send![cls, sharedWorkspace];
            let app: *mut Object = msg_send![workspace, frontmostApplication];
            if app.is_null() {
                return None;
            }
            let pid: i32 = msg_send![app, processIdentifier];
            Some(pid)
        }
    }

    pub fn running_app_summary(pid: i32) -> Option<String> {
        if pid <= 0 {
            return None;
        }

        unsafe {
            let cls = Class::get("NSRunningApplication")?;
            let app: *mut Object = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
            if app.is_null() {
                return None;
            }

            let name: *mut Object = msg_send![app, localizedName];
            let bundle_id: *mut Object = msg_send![app, bundleIdentifier];
            let name = nsstring_to_string(name).unwrap_or_else(|| "unknown app".to_string());
            let bundle_id =
                nsstring_to_string(bundle_id).unwrap_or_else(|| "unknown bundle".to_string());
            Some(format!("{name} ({bundle_id})"))
        }
    }

    pub fn is_accessibility_trusted() -> bool {
        unsafe { AXIsProcessTrusted() != 0 }
    }

    pub fn activate_pid(pid: i32) -> bool {
        unsafe {
            let Some(cls) = Class::get("NSRunningApplication") else {
                return false;
            };
            let app: *mut Object = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
            if app.is_null() {
                return false;
            }
            // NSApplicationActivateAllWindows | NSApplicationActivateIgnoringOtherApps
            let activated: bool = msg_send![app, activateWithOptions: 3u64];
            activated
        }
    }

    /// Simulate Cmd+V keystroke via CGEvent to paste clipboard contents.
    pub fn simulate_paste() -> Result<(), String> {
        use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        // Virtual keycode for 'V' on macOS
        const KV_V: CGKeyCode = 9;

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|e| format!("Failed to create CGEventSource: {e:?}"))?;

        let key_down = CGEvent::new_keyboard_event(source.clone(), KV_V, true)
            .map_err(|e| format!("Failed to create key down event: {e:?}"))?;
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);
        key_down.post(core_graphics::event::CGEventTapLocation::HID);

        let key_up = CGEvent::new_keyboard_event(source, KV_V, false)
            .map_err(|e| format!("Failed to create key up event: {e:?}"))?;
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);
        key_up.post(core_graphics::event::CGEventTapLocation::HID);
        Ok(())
    }
}

fn frontmost_pid() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        macos_focus::get_frontmost_pid()
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn app_summary(pid: i32) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        macos_focus::running_app_summary(pid)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

fn accessibility_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_focus::is_accessibility_trusted()
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
mod macos_launch_agent {
    use std::path::PathBuf;

    fn plist_path(label: &str) -> Result<PathBuf, String> {
        let home =
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
        Ok(home
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{label}.plist")))
    }

    fn escape_xml(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn plist_contents(label: &str, executable: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>LimitLoadToSessionType</key>
  <string>Aqua</string>
</dict>
</plist>
"#,
            escape_xml(label),
            escape_xml(executable),
        )
    }

    pub fn is_enabled(label: &str) -> bool {
        plist_path(label).map(|path| path.exists()).unwrap_or(false)
    }

    pub fn enable(label: &str) -> Result<(), String> {
        let path = plist_path(label)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create LaunchAgents directory: {e}"))?;
        }

        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to determine current executable: {e}"))?;
        let executable = exe.to_string_lossy();
        std::fs::write(&path, plist_contents(label, &executable))
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        Ok(())
    }

    pub fn disable(label: &str) -> Result<(), String> {
        let path = plist_path(label)?;
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {e}", path.display()))?;
        }
        Ok(())
    }
}

fn launch_at_login_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_launch_agent::is_enabled(app_identifier())
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

fn toggle_launch_at_login() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let label = app_identifier();
        let next_enabled = !macos_launch_agent::is_enabled(label);
        if next_enabled {
            macos_launch_agent::enable(label)?;
        } else {
            macos_launch_agent::disable(label)?;
        }
        Ok(next_enabled)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Launch at Login is only supported on macOS.".to_string())
    }
}

/// Center the window on whichever monitor currently contains the mouse cursor.
fn center_on_active_screen(window: &tauri::WebviewWindow) {
    if let Ok(cursor) = window.cursor_position() {
        if let Ok(monitors) = window.available_monitors() {
            for monitor in &monitors {
                let pos = monitor.position();
                let size = monitor.size();
                let left = pos.x as f64;
                let top = pos.y as f64;
                let right = left + size.width as f64;
                let bottom = top + size.height as f64;

                if cursor.x >= left && cursor.x < right && cursor.y >= top && cursor.y < bottom {
                    let win_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
                        width: 900,
                        height: 680,
                    });
                    let x = pos.x + (size.width as i32 - win_size.width as i32) / 2;
                    let y = pos.y + (size.height as i32 - win_size.height as i32) / 2;
                    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                    return;
                }
            }
        }
    }
    // Fallback: use the positioner plugin's center (primary monitor)
    let _ = window.move_window(Position::Center);
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            #[cfg(target_os = "macos")]
            {
                let pid = PREVIOUS_APP_PID.load(Ordering::Relaxed);
                if pid > 0 {
                    macos_focus::activate_pid(pid);
                }
            }
        } else {
            #[cfg(target_os = "macos")]
            {
                if let Some(pid) = macos_focus::get_frontmost_pid() {
                    PREVIOUS_APP_PID.store(pid, Ordering::Relaxed);
                }
            }
            center_on_active_screen(&window);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn parse_shortcut(shortcut_str: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = shortcut_str.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let key_str = parts.last()?;

    for &part in &parts[..parts.len() - 1] {
        match part.trim() {
            "Cmd" | "Super" | "Command" => modifiers |= Modifiers::SUPER,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Ctrl" | "Control" => modifiers |= Modifiers::CONTROL,
            "Alt" | "Option" => modifiers |= Modifiers::ALT,
            _ => {}
        }
    }

    let code = match key_str.trim().to_uppercase().as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "SPACE" => Code::Space,
        _ => return None,
    };

    let mods = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Some(Shortcut::new(mods, code))
}

#[tauri::command]
fn get_config() -> Result<config::Config, String> {
    config::load_config()
}

#[tauri::command]
fn open_config() -> Result<(), String> {
    config::open_config_file()
}

#[tauri::command]
fn get_prompts(state: tauri::State<'_, AppState>) -> Vec<indexer::Prompt> {
    state.prompts.lock().unwrap().clone()
}

#[tauri::command]
fn rescan(state: tauri::State<'_, AppState>) -> Result<Vec<indexer::Prompt>, String> {
    let cfg = config::load_config()?;
    let prompts = indexer::scan(&cfg);
    *state.prompts.lock().unwrap() = prompts.clone();
    Ok(prompts)
}

#[tauri::command]
fn get_resolved_chain(
    path: String,
    repo: String,
    state: tauri::State<'_, AppState>,
) -> resolver::ResolvedChain {
    let prompts = state.prompts.lock().unwrap().clone();
    resolver::resolve_chain(&path, &repo, &prompts)
}

#[tauri::command]
fn get_prompt_content(path: String, repo: String) -> Result<String, String> {
    let cfg = config::load_config()?;
    let repo_config = cfg
        .repos
        .iter()
        .find(|r| r.name == repo)
        .ok_or_else(|| format!("Repo not found: {repo}"))?;
    let full_path = config::expand_path(&repo_config.path).join(&path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read {}: {e}", full_path.display()))?;
    Ok(indexer::strip_frontmatter(&content))
}

#[tauri::command]
fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_version() -> String {
    format!("v{}", app_version())
}

#[tauri::command]
fn restore_previous_focus() {
    #[cfg(target_os = "macos")]
    {
        let pid = PREVIOUS_APP_PID.load(Ordering::Relaxed);
        if pid > 0 {
            let _ = macos_focus::activate_pid(pid);
        }
    }
}

#[tauri::command]
fn paste_to_app(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    {
        let pid = PREVIOUS_APP_PID.load(Ordering::Relaxed);
        let frontmost_at_command = macos_focus::get_frontmost_pid();
        let trusted_at_command = macos_focus::is_accessibility_trusted();

        std::thread::spawn(move || {
            use std::time::{Duration, Instant};

            let mut report = vec![
                "deferred_paste: true".to_string(),
                format!("paste_handoff_delay_ms: {PASTE_HANDOFF_DELAY_MS}"),
                format!("accessibility_trusted_at_command: {trusted_at_command}"),
                format!("previous_pid: {pid}"),
                format!(
                    "previous_app: {}",
                    app_summary(pid).unwrap_or_else(|| "unknown".to_string())
                ),
                format!(
                    "frontmost_at_command: {}",
                    frontmost_at_command
                        .map(|pid| format!(
                            "{pid} {}",
                            app_summary(pid).unwrap_or_else(|| "unknown".to_string())
                        ))
                        .unwrap_or_else(|| "unknown".to_string())
                ),
            ];

            std::thread::sleep(Duration::from_millis(PASTE_HANDOFF_DELAY_MS));
            let frontmost_after_delay = macos_focus::get_frontmost_pid();
            let trusted_before_paste = macos_focus::is_accessibility_trusted();
            report.push(format!(
                "accessibility_trusted_before_paste: {trusted_before_paste}"
            ));
            report.push(format!(
                "frontmost_after_delay: {}",
                frontmost_after_delay
                    .map(|pid| format!(
                        "{pid} {}",
                        app_summary(pid).unwrap_or_else(|| "unknown".to_string())
                    ))
                    .unwrap_or_else(|| "unknown".to_string())
            ));

            if pid > 0 {
                let activated = macos_focus::activate_pid(pid);
                report.push(format!("activate_previous_pid_returned: {activated}"));
                let wait_started = Instant::now();
                let deadline = wait_started + Duration::from_millis(PASTE_FOCUS_TIMEOUT_MS);
                let mut frontmost_after = macos_focus::get_frontmost_pid();

                while frontmost_after != Some(pid) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(PASTE_FOCUS_RETRY_MS));
                    let _ = macos_focus::activate_pid(pid);
                    frontmost_after = macos_focus::get_frontmost_pid();
                }

                report.push(format!(
                    "focus_wait_elapsed_ms: {}",
                    wait_started.elapsed().as_millis()
                ));
                report.push(format!(
                    "frontmost_before_paste: {}",
                    frontmost_after
                        .map(|pid| format!(
                            "{pid} {}",
                            app_summary(pid).unwrap_or_else(|| "unknown".to_string())
                        ))
                        .unwrap_or_else(|| "unknown".to_string())
                ));

                if frontmost_after == Some(pid) {
                    if trusted_before_paste {
                        match macos_focus::simulate_paste() {
                            Ok(()) => report.push("posted_cmd_v_event: true".to_string()),
                            Err(e) => report.push(format!("posted_cmd_v_event_error: {e}")),
                        }
                    } else {
                        report.push("skipped_paste: accessibility not trusted".to_string());
                    }
                } else {
                    report.push("skipped_paste: previous app did not become frontmost".to_string());
                }
            } else {
                report.push("skipped_paste: no previous pid captured".to_string());
            }

            let report = report.join("\n");
            eprintln!("{report}");
            set_last_paste_report(report);
        });
    }
    Ok(())
}

pub fn run() {
    let cfg = config::ensure_config().expect("Failed to load config");
    let shortcut = parse_shortcut(&cfg.shortcut)
        .unwrap_or_else(|| Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyP));

    // Initial scan
    let prompts = indexer::scan(&cfg);
    println!("Indexed {} prompts", prompts.len());

    let state = AppState {
        prompts: Arc::new(Mutex::new(prompts)),
    };

    let expected_shortcut = shortcut.clone();

    let mut app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, sc, event| {
                    if sc == &expected_shortcut && event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                })
                .build(),
        )
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_config,
            open_config,
            get_prompts,
            rescan,
            get_resolved_chain,
            get_prompt_content,
            copy_to_clipboard,
            restore_previous_focus,
            paste_to_app,
            get_version
        ])
        .setup(move |app| {
            let _window = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Prompt Picker")
                .inner_size(900.0, 680.0)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .build()?;

            app.global_shortcut().register(shortcut)?;

            let open_config_i =
                MenuItem::with_id(app, "open_config", "Open Config", true, None::<&str>)?;
            let open_prompts_i =
                MenuItem::with_id(app, "open_prompts", "Open Prompt Folder", true, None::<&str>)?;
            let reload_i =
                MenuItem::with_id(app, "reload", "Reload", true, None::<&str>)?;
            let launch_at_login_i = CheckMenuItem::with_id(
                app,
                "launch_at_login",
                "Launch at Login",
                true,
                launch_at_login_enabled(),
                None::<&str>,
            )?;
            let about_i = MenuItem::with_id(
                app,
                "about",
                &format!("About Prompt Picker v{}", app_version()),
                true,
                None::<&str>,
            )?;
            let copy_frontmatter_i = MenuItem::with_id(
                app,
                "copy_frontmatter",
                "Copy Prompt Frontmatter Template",
                true,
                None::<&str>,
            )?;
            let copy_paste_diagnostics_i = MenuItem::with_id(
                app,
                "copy_paste_diagnostics",
                "Copy Paste Diagnostics",
                true,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &open_config_i,
                    &open_prompts_i,
                    &reload_i,
                    &launch_at_login_i,
                    &copy_frontmatter_i,
                    &copy_paste_diagnostics_i,
                    &about_i,
                    &quit_i,
                ],
            )?;

            let app_handle_for_reload = app.handle().clone();
            let launch_at_login_item = launch_at_login_i.clone();
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open_config" => {
                        let _ = config::open_config_file();
                    }
                    "open_prompts" => {
                        if let Ok(cfg) = config::load_config() {
                            if let Some(repo) = cfg.repos.first() {
                                let _ = open::that(config::expand_path(&repo.path));
                            }
                        }
                    }
                    "launch_at_login" => match toggle_launch_at_login() {
                        Ok(enabled) => {
                            let _ = launch_at_login_item.set_checked(enabled);
                        }
                        Err(e) => {
                            eprintln!("Failed to toggle Launch at Login: {e}");
                            let _ = launch_at_login_item
                                .set_checked(launch_at_login_enabled());
                        }
                    },
                    "reload" => {
                        if let Ok(cfg) = config::load_config() {
                            let prompts = indexer::scan(&cfg);
                            if let Some(state) =
                                app_handle_for_reload.try_state::<AppState>()
                            {
                                *state.prompts.lock().unwrap() = prompts.clone();
                                let _ = app_handle_for_reload.emit("prompts-changed", &prompts);
                            }
                        }
                    }
                    "copy_frontmatter" => {
                        use tauri_plugin_clipboard_manager::ClipboardExt;
                        let example = "---\ntype: prompt\nname: \"New prompt\"\nsection: start\nsection_name: Start\nsection_icon: play-circle\nsection_order: 10\norder: 10\ntags:\n  - agent\npinned: true\n---\n";
                        let _ = app.clipboard().write_text(example);
                    }
                    "copy_paste_diagnostics" => {
                        use tauri_plugin_clipboard_manager::ClipboardExt;
                        let _ = app.clipboard().write_text(paste_diagnostics_text());
                    }
                    "about" => {
                        let _ = open::that("https://github.com/jonasgantner/prompt-picker");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Start watchers
            config::watch_config(app.handle().clone());
            indexer::watch_repos(&cfg, app.handle().clone());

            // Background update check
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // Delay to let the app finish starting up
                std::thread::sleep(std::time::Duration::from_secs(5));
                tauri::async_runtime::block_on(async {
                    match handle.updater() {
                        Ok(updater) => match updater.check().await {
                            Ok(Some(update)) => {
                                println!("Update available: {}", update.version);
                                if let Err(e) =
                                    update.download_and_install(|_, _| {}, || {}).await
                                {
                                    eprintln!("Failed to install update: {e}");
                                }
                            }
                            Ok(None) => {}
                            Err(e) => eprintln!("Update check failed: {e}"),
                        },
                        Err(e) => eprintln!("Failed to create updater: {e}"),
                    }
                });
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    app.run(|_app_handle, _event| {});
}
