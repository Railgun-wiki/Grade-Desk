use std::path::Path;

use tauri::App;

use super::{Platform, PlatformService};

pub(super) static ANDROID_SERVICE: MobilePlatform = MobilePlatform(Platform::Android);
pub(super) static IOS_SERVICE: MobilePlatform = MobilePlatform(Platform::Ios);

pub(super) struct MobilePlatform(Platform);

impl PlatformService for MobilePlatform {
    fn kind(&self) -> Platform {
        self.0
    }

    fn configure_app(&self, _: &App) -> tauri::Result<()> {
        Ok(())
    }

    fn protect_session_payload(&self, _: &[u8]) -> Result<Vec<u8>, String> {
        self.ensure_file_session_storage_supported()?;
        unreachable!("mobile session storage support is intentionally disabled")
    }

    fn unprotect_session_payload(&self, _: &[u8]) -> Result<Vec<u8>, String> {
        self.ensure_file_session_storage_supported()?;
        unreachable!("mobile session storage support is intentionally disabled")
    }

    fn restrict_session_file_permissions(&self, _: &Path) -> Result<(), String> {
        self.ensure_file_session_storage_supported()
    }
}
