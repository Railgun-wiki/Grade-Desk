# JWXT session module

## Responsibility

Provide a controlled WebView flow for CAS/JWXT sign-in on macOS, Windows 11, and supported Linux desktop environments. It reads the authenticated JWXT WebView cookies, persists them in the app-data directory, verifies a saved session independently of grade availability, and exposes user-selected official JWXT grade-query methods.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `start_jwxt_login` | Async Rust/Tauri command | Opens or focuses a controlled JWXT WebView window at the CAS-backed student login endpoint. The asynchronous command path avoids the WebView2 deadlock that can occur when Windows creates a window synchronously. |
| `jwxt_status` | Rust/Tauri command | Reports only whether a locally persisted session exists; it never returns a Cookie. |
| `verify_jwxt_session` | Rust/Tauri command | Calls only the official JWXT pull endpoint using the locally saved session. A successful response means authentication succeeded, regardless of grade-list policy. |
| `sync_jwxt_grades(method)` | Rust/Tauri command | Uses a user-selected official JWXT grade-query method, normalizes its course results into SQLite, and creates a local history snapshot. |
| `query_jwxt_rank_summary` | Rust/Tauri command | Explicitly queries the official JWXT ranking/GPA summary and returns aggregate statistics only; it does not alter local course records. |
| `probe_jwxt_numeric_score(attemptId)` | Rust/Tauri command | Explicitly probes one locally stored grade-only course through the official graduation-course endpoint, then stores a returned verified numeric score locally. |
| `save_jwxt_session` | Async Rust/Tauri command | Reads the completed login window's JWXT Cookie on an explicit user action in a blocking worker, saves it to the app-data directory, then closes the login window. |

## Data ownership

The app-data directory owns the serialized JWXT Cookie set in `jwxt-session.json`, with Unix file permission `0600`. On Windows, the Cookie payload is encrypted with DPAPI for the current user. SQLite does not store cookies, passwords, CAS tickets, or authorization headers.

## Security and privacy constraints

- Authentication occurs in a separate application-controlled WebView; the main UI never collects NetID or password fields.
- Tauri's WebView Cookie API can include HttpOnly cookies. Cookies are persisted to a local app-data file with `0600` permission on macOS. Reading happens only after an explicit user action, and runs in a blocking worker so Windows WebView2's UI thread is never blocked.
- On Windows, the serialized Cookie payload is protected with DPAPI and can only be decrypted by the same Windows user. Existing plaintext session files remain readable for migration and are encrypted on the next successful save.
- Cookie values are never returned to TypeScript, rendered, logged, exported, or inserted into SQLite.
- HTTP diagnostics record only operation, status, Content-Type, response shape, and byte length in `jwxt-diagnostics.log`; they never record Cookie values or response bodies.
- JWXT events use Rust `tracing` levels: successful session actions are `info`, response metadata is `debug`, and unexpected HTTP or business states are `warn`. Console filtering follows `RUST_LOG` and defaults to `debug`.
- Official JWXT requests include the JWXT homepage Referer and a browser-compatible Accept/User-Agent header, matching the request context expected by the service.
- For a JSON response, the JWXT business `code` is authoritative even if the service sends a nonstandard HTTP status (such as `600`). That status remains in diagnostics; malformed or HTML responses are still rejected.
- Authentication verification occurs only after the user selects “验证会话”; it calls no grade endpoint. Grade queries occur only after the user selects a query method and then requests synchronization.
- Supported grade-list query methods are the official score-check list and the official achievement search list. The UI requires an explicit user choice.
- Ranking statistics use the separate official `score-check/getSortByYear` endpoint and are queried only after the user selects the ranking action. They are displayed as aggregate values and are not imported as course attempts.
- Numeric-score probing is never run during login, verification, synchronization, or page load. It requires two explicit in-app actions for one selected course, makes bounded sequential requests, and only persists a score when the official endpoint confirms it.
- Numeric-score probing writes a lifecycle diagnostic before local validation, then records each pre-request, request, response, persistence, or no-result failure without including course identifiers, grades, scores, Cookies, or response bodies.
- The graduation-course endpoint receives the official course number (`scoCourseNumber`/local `course_code`), matching the reference implementation; it does not receive the teaching-class number.
- macOS, Windows 11, and Linux are supported. Login-window creation and Cookie reads stay asynchronous; this avoids WebView2 deadlocks on Windows and prevents the synchronous WebKitGTK Cookie API from blocking the UI thread on Linux.

## Dependencies

- Tauri 2 WebviewWindow Cookie APIs on macOS, Windows 11 (WebView2 Runtime), and Linux (WebKitGTK).
- Windows: DPAPI through `windows-sys` for current-user session encryption.
- Standard-library app-data file I/O with restrictive Unix file permissions.
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
- Windows login requires Windows 11 with WebView2 Runtime. CAS navigation and Cookie persistence must be verified on a physical Windows device because automated macOS checks cannot exercise WebView2.
- Linux login requires the supported WebKitGTK system dependencies. CAS navigation and Cookie persistence must be verified on a physical Linux desktop because automated builds do not exercise a real desktop session.
- Numeric-score probing is available only as a per-course explicit action. It can be rejected by JWXT policy, its score-range convention may change, and it should not be used as a bulk collection mechanism.
- Per-term list queries use the same `score-check/list` endpoint and share its server-side policy. The separate `score-check/getSortByYear` statistics endpoint does not provide importable course records and is not exposed as a synchronization method.
- `getSortByYear` is under the same JWXT score-check service family; its current evaluation-policy behavior must be treated as server-controlled and can differ from the grade list.
- Exact session expiry and multi-factor behavior remain under the host's CAS policy.
