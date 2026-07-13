use std::path::Path;

use tauri::{App, Manager};

use super::{desktop, Platform, PlatformService};

pub(super) static SERVICE: MacosPlatform = MacosPlatform;

pub(super) struct MacosPlatform;

impl PlatformService for MacosPlatform {
    fn kind(&self) -> Platform {
        Platform::Macos
    }

    fn configure_app(&self, app: &App) -> tauri::Result<()> {
        if let Some(window) = app.get_webview_window("main") {
            let effects = tauri::window::EffectsBuilder::new()
                .effects(vec![tauri::window::Effect::Sidebar])
                .build();
            let _ = window.set_effects(effects);
        }
        Ok(())
    }

    fn protect_session_payload(&self, payload: &[u8]) -> Result<Vec<u8>, String> {
        desktop::passthrough(payload)
    }

    fn unprotect_session_payload(&self, payload: &[u8]) -> Result<Vec<u8>, String> {
        desktop::passthrough(payload)
    }

    fn restrict_session_file_permissions(&self, path: &Path) -> Result<(), String> {
        desktop::restrict_file_permissions(path)
    }
}
