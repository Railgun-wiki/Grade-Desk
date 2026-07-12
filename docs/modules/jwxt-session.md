# JWXT session module

## Responsibility

Provide a macOS-only, controlled WebView flow for SYSU CAS/JWXT sign-in. It reads the authenticated JWXT WebView cookies, persists them in the app-data directory, verifies a saved session independently of grade availability, and exposes user-selected official JWXT grade-query methods.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `start_jwxt_login` | Rust/Tauri command | Opens or focuses a controlled JWXT WebView window at the CAS-backed student login endpoint. |
| `jwxt_status` | Rust/Tauri command | Reports only whether a locally persisted session exists; it never returns a Cookie. |
| `verify_jwxt_session` | Rust/Tauri command | Calls only the official JWXT pull endpoint using the locally saved session. A successful response means authentication succeeded, regardless of grade-list policy. |
| `sync_jwxt_grades(method)` | Rust/Tauri command | Uses a user-selected official JWXT grade-query method, normalizes its course results into SQLite, and creates a local history snapshot. |
| `probe_jwxt_numeric_score(attemptId)` | Rust/Tauri command | Explicitly probes one locally stored grade-only course through the official graduation-course endpoint, then stores a returned verified numeric score locally. |
| `save_jwxt_session` | Rust/Tauri command | Reads the completed login window's JWXT Cookie on an explicit user action and saves it to the app-data directory. |

## Data ownership

The app-data directory owns the serialized JWXT Cookie set in `jwxt-session.json`, with macOS file permission `0600`. SQLite does not store cookies, passwords, CAS tickets, or authorization headers.

## Security and privacy constraints

- Authentication occurs in a separate application-controlled WebView; the main UI never collects NetID or password fields.
- Tauri's macOS WebView Cookie API can include HttpOnly cookies. Cookies are persisted to a local app-data file with `0600` permission; reading happens from an explicit command rather than a page-load callback to avoid WebKit main-thread contention.
- Cookie values are never returned to TypeScript, rendered, logged, exported, or inserted into SQLite.
- HTTP diagnostics record only operation, status, Content-Type, response shape, and byte length in `jwxt-diagnostics.log`; they never record Cookie values or response bodies.
- JWXT events use Rust `tracing` levels: successful session actions are `info`, response metadata is `debug`, and unexpected HTTP or business states are `warn`. Console filtering follows `RUST_LOG` and defaults to `debug`.
- Official JWXT requests include the JWXT homepage Referer and a browser-compatible Accept/User-Agent header, matching the request context expected by the service.
- For a JSON response, the JWXT business `code` is authoritative even if the service sends a nonstandard HTTP status (such as `600`). That status remains in diagnostics; malformed or HTML responses are still rejected.
- Authentication verification occurs only after the user selects “验证会话”; it calls no grade endpoint. Grade queries occur only after the user selects a query method and then requests synchronization.
- Supported grade-list query methods are the official score-check list and the official achievement search list. The UI requires an explicit user choice.
- Numeric-score probing is never run during login, verification, synchronization, or page load. It requires an explicit foreground confirmation for one selected course, makes bounded sequential requests, and only persists a score when the official endpoint confirms it.
- The feature is intentionally macOS-only. Windows/Linux behavior is not claimed or supported.

## Dependencies

- Tauri 2 WebviewWindow Cookie APIs on macOS.
- Standard-library app-data file I/O with restrictive macOS file permissions.
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
- Numeric-score probing is available only as a per-course explicit action. It can be rejected by JWXT policy, its score-range convention may change, and it should not be used as a bulk collection mechanism.
- Per-term list queries use the same `score-check/list` endpoint and share its server-side policy. The separate `score-check/getSortByYear` statistics endpoint does not provide importable course records and is not exposed as a synchronization method. Numeric-score probing remains unavailable.
- Exact session expiry and multi-factor behavior remain under the school's CAS policy.
