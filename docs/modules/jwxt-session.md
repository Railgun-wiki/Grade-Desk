# JWXT session module

## Responsibility

Provide a macOS-only, controlled WebView flow for SYSU CAS/JWXT sign-in. It reads the authenticated JWXT WebView cookies, persists them in the macOS Keychain, and uses the saved session only when the user explicitly verifies or queries the official course-grade list.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `start_jwxt_login` | Rust/Tauri command | Opens or focuses a controlled JWXT WebView window at the CAS-backed student login endpoint. |
| `jwxt_status` | Rust/Tauri command | Reports only whether a locally persisted session exists; it never returns a Cookie. |
| `verify_jwxt_session` | Rust/Tauri command | Calls the official JWXT pull and grade-list endpoints using the Keychain session, then returns only course count and training type. |
| `sync_jwxt_grades` | Rust/Tauri command | Normalizes the official JWXT list into SQLite and creates a local history snapshot. |
| `jwxt-session-updated` | Tauri event | Announces that the controlled WebView has persisted a JWXT session; event payload contains no secret. |

## Data ownership

The macOS Keychain owns the serialized JWXT Cookie set under the service `edu.sysu.grade-desk`. SQLite does not store cookies, passwords, CAS tickets, or authorization headers.

## Security and privacy constraints

- Authentication occurs in a separate application-controlled WebView; the main UI never collects NetID or password fields.
- Tauri's macOS WebView Cookie API can include HttpOnly cookies. The module persists only cookies scoped to `jwxt.sysu.edu.cn`.
- Cookie values are never returned to TypeScript, rendered, logged, exported, or inserted into SQLite.
- Network requests occur only after the user selects “验证并查询课程”. The implementation does not run the numeric-score probing endpoint automatically.
- The feature is intentionally macOS-only. Windows/Linux behavior is not claimed or supported.

## Dependencies

- Tauri 2 WebviewWindow Cookie APIs on macOS.
- `keyring` for macOS Keychain persistence.
- `reqwest` with Rustls for the official HTTPS pull/list requests.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
CI=true pnpm build
CI=true pnpm tauri build --debug
```

## Known limitations

- Real login and request validation require an authorized student account and are not exercised by automated tests.
- Numeric-score probing for grade-only records is deliberately not automatic; it requires a separate explicit action and rate-limited policy.
- Exact session expiry and multi-factor behavior remain under the school's CAS policy.
