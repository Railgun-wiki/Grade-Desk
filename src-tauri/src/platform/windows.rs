use std::{ffi::c_void, path::Path, ptr};

use tauri::{App, Manager};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

use super::{desktop, Platform, PlatformService};

const DPAPI_SESSION_PREFIX: &[u8] = b"GRADE_DESK_DPAPI_V1\0";

pub(super) static SERVICE: WindowsPlatform = WindowsPlatform;

pub(super) struct WindowsPlatform;

impl PlatformService for WindowsPlatform {
    fn kind(&self) -> Platform {
        Platform::Windows
    }

    fn configure_app(&self, app: &App) -> tauri::Result<()> {
        if let Some(window) = app.get_webview_window("main") {
            let effects = tauri::window::EffectsBuilder::new()
                .effects(vec![tauri::window::Effect::Mica])
                .build();
            let _ = window.set_effects(effects);
        }
        Ok(())
    }

    fn protect_session_payload(&self, payload: &[u8]) -> Result<Vec<u8>, String> {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: payload
                .len()
                .try_into()
                .map_err(|error| error.to_string())?,
            pbData: payload.as_ptr() as *mut u8,
        };
        let mut encrypted = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        let protected = unsafe {
            CryptProtectData(
                &mut input,
                ptr::null(),
                ptr::null(),
                ptr::null::<c_void>(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut encrypted,
            )
        };
        if protected == 0 {
            return Err("无法使用 Windows DPAPI 加密教务会话。".into());
        }
        let encrypted_bytes = unsafe {
            std::slice::from_raw_parts(encrypted.pbData, encrypted.cbData as usize).to_vec()
        };
        unsafe { LocalFree(encrypted.pbData as *mut c_void) };
        let mut stored = DPAPI_SESSION_PREFIX.to_vec();
        stored.extend(encrypted_bytes);
        Ok(stored)
    }

    fn unprotect_session_payload(&self, payload: &[u8]) -> Result<Vec<u8>, String> {
        if !payload.starts_with(DPAPI_SESSION_PREFIX) {
            return Ok(payload.to_vec());
        }
        let encrypted = &payload[DPAPI_SESSION_PREFIX.len()..];
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: encrypted
                .len()
                .try_into()
                .map_err(|error| error.to_string())?,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut decrypted = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        let unprotected = unsafe {
            CryptUnprotectData(
                &mut input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null::<c_void>(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut decrypted,
            )
        };
        if unprotected == 0 {
            return Err("无法使用 Windows DPAPI 解密教务会话。".into());
        }
        let decrypted_bytes = unsafe {
            std::slice::from_raw_parts(decrypted.pbData, decrypted.cbData as usize).to_vec()
        };
        unsafe { LocalFree(decrypted.pbData as *mut c_void) };
        Ok(decrypted_bytes)
    }

    fn restrict_session_file_permissions(&self, path: &Path) -> Result<(), String> {
        desktop::restrict_file_permissions(path)
    }
}

#[cfg(test)]
mod tests {
    use super::DPAPI_SESSION_PREFIX;

    #[test]
    fn dpapi_payload_prefix_is_versioned() {
        assert_eq!(DPAPI_SESSION_PREFIX, b"GRADE_DESK_DPAPI_V1\0");
    }
}
