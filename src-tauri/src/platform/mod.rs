//! Platform boundaries for native window behavior and sensitive session storage.

use std::path::Path;

use tauri::App;

mod desktop;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile;
#[cfg(target_os = "windows")]
mod windows;

#[allow(dead_code)] // A target compiles only its own variant; tests cover the full matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Platform {
    Macos,
    Windows,
    Linux,
    Android,
    Ios,
}

impl Platform {
    pub(crate) const fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::Macos
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(target_os = "android")]
        {
            Self::Android
        }
        #[cfg(target_os = "ios")]
        {
            Self::Ios
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Android => "android",
            Self::Ios => "ios",
        }
    }

    pub(crate) const fn capabilities(self) -> PlatformCapabilities {
        match self {
            Self::Macos | Self::Windows | Self::Linux => PlatformCapabilities {
                desktop_window_effects: true,
                jwxt_login_window: true,
                file_session_storage: true,
            },
            Self::Android | Self::Ios => PlatformCapabilities {
                desktop_window_effects: false,
                jwxt_login_window: false,
                file_session_storage: false,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlatformCapabilities {
    pub(crate) desktop_window_effects: bool,
    pub(crate) jwxt_login_window: bool,
    pub(crate) file_session_storage: bool,
}

pub(crate) trait PlatformService: Sync {
    fn kind(&self) -> Platform;

    fn configure_app(&self, app: &App) -> tauri::Result<()>;

    fn protect_session_payload(&self, payload: &[u8]) -> Result<Vec<u8>, String>;

    fn unprotect_session_payload(&self, payload: &[u8]) -> Result<Vec<u8>, String>;

    fn restrict_session_file_permissions(&self, path: &Path) -> Result<(), String>;

    fn ensure_jwxt_login_window_supported(&self) -> Result<(), String> {
        if self.kind().capabilities().jwxt_login_window {
            Ok(())
        } else {
            Err(
                "当前移动端暂不支持教务登录与会话同步；后续将使用系统浏览器认证和安全凭据存储。"
                    .into(),
            )
        }
    }

    fn ensure_file_session_storage_supported(&self) -> Result<(), String> {
        if self.kind().capabilities().file_session_storage {
            Ok(())
        } else {
            Err("当前移动端不允许将教务会话写入文件；请等待移动端安全认证支持。".into())
        }
    }
}

pub(crate) fn current() -> &'static dyn PlatformService {
    #[cfg(target_os = "macos")]
    {
        &macos::SERVICE
    }
    #[cfg(target_os = "windows")]
    {
        &windows::SERVICE
    }
    #[cfg(target_os = "linux")]
    {
        &linux::SERVICE
    }
    #[cfg(target_os = "android")]
    {
        &mobile::ANDROID_SERVICE
    }
    #[cfg(target_os = "ios")]
    {
        &mobile::IOS_SERVICE
    }
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn platform_names_are_stable_for_ipc() {
        assert_eq!(Platform::Macos.as_str(), "macos");
        assert_eq!(Platform::Windows.as_str(), "windows");
        assert_eq!(Platform::Linux.as_str(), "linux");
        assert_eq!(Platform::Android.as_str(), "android");
        assert_eq!(Platform::Ios.as_str(), "ios");
    }

    #[test]
    fn desktop_and_mobile_capabilities_are_explicit() {
        for platform in [Platform::Macos, Platform::Windows, Platform::Linux] {
            let capabilities = platform.capabilities();
            assert!(capabilities.desktop_window_effects);
            assert!(capabilities.jwxt_login_window);
            assert!(capabilities.file_session_storage);
        }
        for platform in [Platform::Android, Platform::Ios] {
            let capabilities = platform.capabilities();
            assert!(!capabilities.desktop_window_effects);
            assert!(!capabilities.jwxt_login_window);
            assert!(!capabilities.file_session_storage);
        }
    }

    #[test]
    fn current_platform_matches_the_compile_target() {
        #[cfg(target_os = "linux")]
        assert_eq!(Platform::current(), Platform::Linux);
        #[cfg(target_os = "macos")]
        assert_eq!(Platform::current(), Platform::Macos);
        #[cfg(target_os = "windows")]
        assert_eq!(Platform::current(), Platform::Windows);
        #[cfg(target_os = "android")]
        assert_eq!(Platform::current(), Platform::Android);
        #[cfg(target_os = "ios")]
        assert_eq!(Platform::current(), Platform::Ios);
    }
}
