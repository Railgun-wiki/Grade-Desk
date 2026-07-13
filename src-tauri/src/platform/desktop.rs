use std::path::Path;

pub(super) fn passthrough(payload: &[u8]) -> Result<Vec<u8>, String> {
    Ok(payload.to_vec())
}

#[cfg(unix)]
pub(super) fn restrict_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
pub(super) fn restrict_file_permissions(_: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::passthrough;

    #[test]
    fn desktop_passthrough_does_not_add_a_dpapi_prefix() {
        let payload = b"desktop-session";
        assert_eq!(passthrough(payload).unwrap(), payload);
    }

    #[cfg(unix)]
    #[test]
    fn unix_session_permissions_are_restricted() {
        use std::os::unix::fs::PermissionsExt;

        let path =
            std::env::temp_dir().join(format!("grade-desk-platform-test-{}", std::process::id()));
        std::fs::write(&path, b"session").unwrap();
        super::restrict_file_permissions(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mode, 0o600);
    }
}
