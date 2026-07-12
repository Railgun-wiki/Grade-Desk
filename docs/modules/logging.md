# Logging module

## Responsibility

Provide structured Rust-side diagnostic logging. Console verbosity is controlled by the standard `RUST_LOG` environment variable and defaults to `debug` when it is absent or invalid.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `logging::init` | Rust application startup | Installs the process-wide `tracing` subscriber using `RUST_LOG`, with a `debug` fallback. |
| `tracing::{debug, info, warn, error}` | Rust modules | Emit structured, levelled diagnostic events without credentials, cookies, response bodies, or student records. |

## Data ownership

The logging module owns no application data. The JWXT session module continues to own its restricted local diagnostics file.

## Security and privacy constraints

- Default diagnostic events are metadata-only. Credentials, Cookies, CAS tickets, response bodies, and grade records must never be passed to a logging macro.
- `RUST_LOG` changes console filtering only; it does not authorize new requests or change JWXT behavior.
- The JWXT local diagnostics file remains restricted to the current macOS user.

## Dependencies

- `tracing` for Rust-native structured event macros.
- `tracing-subscriber` and its `EnvFilter` for standard `RUST_LOG` filtering.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
CI=true pnpm build
```

## Known limitations

- Changing `RUST_LOG` requires restarting the development process or app.
- The current release writes structured events to the process console; application-specific file diagnostics remain limited to JWXT response metadata.
