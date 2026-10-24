# Platform module

## Responsibility

`src-tauri/src/platform/` centralizes operating-system differences behind `PlatformService`. It owns platform identity, capability declarations, native main-window effects, and JWXT session-file protection. Business modules must not add their own OS-specific `cfg` branches.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `Platform` | `platform/mod.rs` | Stable OS identifiers: `macos`, `windows`, `linux`, `android`, and `ios`. |
| `PlatformCapabilities` | `platform/mod.rs` | Declares desktop effects, JWXT login-window support, and file-session storage support. |
| `PlatformService` | `platform/mod.rs` | Provides startup configuration, session protection, session-file permissions, and support guards. |
| `application_status` | Rust/Tauri command | Returns only the current general OS identifier; its existing payload fields remain compatible. |

## Data ownership

The module owns no grades, credentials, Cookies, or database records. It owns only platform policy. JWXT retains responsibility for its app-data session file and calls this module before reading or writing it.

## Security and privacy constraints

- Windows uses current-user DPAPI for JWXT session payloads.
- macOS and Linux restrict desktop session and diagnostic files to `0600`.
- Android and iOS explicitly reject file-backed JWXT sessions and desktop login windows. They never fall back to plaintext files.
- Platform metadata contains no device identifiers, hardware properties, or user data.

## Dependencies

- Tauri 2 for lifecycle and native-window APIs.
- `windows-sys` only on Windows for DPAPI.
- Rust standard library for Unix permissions and platform selection.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
CI=true pnpm build
CI=true pnpm tauri build --debug
```

Automated tests cover platform names, capability matrix, IPC status mapping, Unix `0600` permissions, desktop pass-through payloads, mobile-session rejection, and (on Windows) the versioned DPAPI payload prefix.

## Known limitations

- This module does not initialize Android or iOS native projects and does not make JWXT available on mobile.
- A future mobile JWXT implementation must use system-browser authentication (Android Custom Tabs or iOS `ASWebAuthenticationSession`) and Keystore/Keychain storage; it must not reuse the desktop multi-window Cookie flow.
