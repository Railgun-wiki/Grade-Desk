# App shell module

## Responsibility

Provide the desktop runtime boundary and the minimal TypeScript user interface for Grade Desk. It starts Tauri, serves the Vite frontend, exposes a typed health command, and establishes the build/tooling conventions for subsequent modules.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `application_status` | Rust/Tauri command | Returns app name, semantic version, and the `local-only` storage mode. |
| `get_dashboard` | Rust/Tauri command | Delegates to the grade-repository module for a typed local summary. |
| `pnpm dev` | Vite | Starts the browser frontend development server. |
| `pnpm tauri dev` | Tauri CLI | Starts the desktop development application. |
| `pnpm build` | TypeScript + Vite | Performs strict TypeScript validation and produces `dist/`. |
| `pnpm tauri build` | Tauri CLI | Builds the desktop bundle after the frontend build. |

## Data ownership

This module owns no student, credential, session, or grade data. SQLite and academic records belong to the grade-repository module.

## Security and privacy constraints

- The frontend may invoke only explicit typed commands; no generic SQL or network command is available.
- No network capability, credential storage, or raw SQL interface is introduced.
- The window uses Tauri's isolation boundary; no global Tauri object is enabled for browser code.

## Dependencies

- pnpm-managed: React, React DOM, Vite, TypeScript, Tauri API, and Tauri CLI.
- Cargo-managed: Tauri runtime and Tauri build helper.
- System: Rust toolchain and the platform WebView required by Tauri.
- The Vite macOS build target is Safari 15, matching the compiler baseline used by the current Vite toolchain.

## Verification

```sh
pnpm install
pnpm build
pnpm tauri build --debug
```

## Known limitations

- The landing view is intentionally a shell and has no transcript or course-detail layout.
- Authentication, analytics, export, and synchronization have not yet been implemented.
