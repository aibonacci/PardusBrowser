mod browser_window;
mod challenge;
mod commands;
mod cookie_bridge;
mod instance;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use tauri::{Listener, Manager};

pub struct AppState {
    pub instances: Mutex<HashMap<String, instance::ManagedInstance>>,
    pub next_id: Mutex<u32>,
    pub resolver: Mutex<Option<Arc<challenge::TauriChallengeResolver>>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            instances: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            resolver: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_instances,
            commands::spawn_instance,
            commands::kill_instance,
            commands::kill_all_instances,
            commands::open_challenge_window,
            commands::submit_challenge_resolution,
            commands::cancel_challenge,
            commands::open_browser_window,
            commands::navigate_browser_window,
            commands::close_browser_window,
        ])
        .setup(|app| {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info".into()),
                )
                .init();

            let app_handle = app.handle().clone();
            let resolver = Arc::new(challenge::TauriChallengeResolver::new(app_handle));

            // Store resolver in state
            let state = app.state::<AppState>();
            *state.resolver.lock().unwrap() = Some(resolver.clone());

            // Listen for cookie events from challenge webviews
            let r_cookies = resolver.clone();
            app.listen("challenge-cookies", move |event| {
                let payload = event.payload();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
                    let url = data["url"].as_str().unwrap_or("").to_string();
                    let cookies = data["cookies"].as_str().unwrap_or("").to_string();
                    let r = r_cookies.clone();
                    tauri::async_runtime::spawn(async move {
                        r.handle_cookies(url, cookies).await;
                    });
                }
            });

            // Listen for timeout events from challenge webviews
            let r_timeout = resolver.clone();
            app.listen("challenge-timeout", move |event| {
                let payload = event.payload();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
                    let url = data["url"].as_str().unwrap_or("").to_string();
                    let r = r_timeout.clone();
                    tauri::async_runtime::spawn(async move {
                        r.handle_failed(url, "challenge timed out (5 minutes)".to_string()).await;
                    });
                }
            });

            // Listen for browser-navigate events from browser window toolbars
            let nav_handle = app.handle().clone();
            app.listen("browser-navigate", move |event| {
                let payload = event.payload();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
                    let instance_id = data["instance_id"].as_str().unwrap_or("").to_string();
                    let url = data["url"].as_str().unwrap_or("").to_string();
                    let h = nav_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let label = format!("browser-{}", instance_id);
                        // Close and reopen with new URL
                        if let Some(window) = h.get_webview_window(&label) {
                            let _ = window.close();
                        }
                        if let Ok(_new_label) = browser_window::open_browser_window(&h, &instance_id, &url) {
                            // Update instance state
                            let state = h.state::<AppState>();
                            let mut instances = state.instances.lock().unwrap();
                            if let Some(inst) = instances.get_mut(&instance_id) {
                                inst.current_url = Some(url);
                            }
                        }
                    });
                }
            });

            // Listen for browser-url-changed events to track current URL
            let url_handle = app.handle().clone();
            app.listen("browser-url-changed", move |event| {
                let payload = event.payload();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
                    let instance_id = data["instance_id"].as_str().unwrap_or("").to_string();
                    let url = data["url"].as_str().unwrap_or("").to_string();
                    let h = url_handle.clone();
                    let state = h.state::<AppState>();
                    let mut instances = state.instances.lock().unwrap();
                    if let Some(inst) = instances.get_mut(&instance_id) {
                        inst.current_url = Some(url.to_string());
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
