use std::path::Path;

use tauri::App;

use super::{desktop, Platform, PlatformService};

pub(super) static SERVICE: LinuxPlatform = LinuxPlatform;

pub(super) struct LinuxPlatform;

impl PlatformService for LinuxPlatform {
    fn kind(&self) -> Platform {
        Platform::Linux
    }

    fn configure_app(&self, _: &App) -> tauri::Result<()> {
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
