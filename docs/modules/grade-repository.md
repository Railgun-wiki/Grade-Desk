# Grade repository module

## Responsibility

Own the local SQLite database for Grade Desk. It creates the version-1 schema, seeds an anonymous demo profile on a fresh database, and returns a typed dashboard summary to the desktop UI.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `get_dashboard` | Rust/Tauri command | Opens the app-data SQLite database, applies idempotent schema setup, seeds only an empty database, and returns one typed summary. |
| `list_course_attempts` | Rust/Tauri command | Returns typed read-only course attempts ordered by course code. |
| `get_course_detail(attemptId)` | Rust/Tauri command | Returns one typed attempt with term, class number, and score components. |
| `grade-desk.db` | Rust repository | Local SQLite database stored only under the Tauri application-data directory. |

## Data ownership

The module owns `profiles`, `terms`, `courses`, `course_attempts`, `score_components`, `sync_runs`, and immutable `grade_snapshots`. `course_attempts.score_kind` makes official numbers, official grades, local derivations, and unavailable values mutually explicit.

## Security and privacy constraints

- SQLite is opened solely in Rust; no generic SQL command is exposed through Tauri IPC.
- The seeded record is anonymous demo data and contains no credentials or real student identifiers.
- Connections enable foreign keys and WAL mode. All seed writes run in one transaction.
- Errors identify only the local repository operation; they do not expose query parameters or data payloads.

## Dependencies

- `rusqlite` with the bundled SQLite feature, avoiding a machine-specific SQLite dependency.
- Tauri's app-data path service.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
pnpm build
```

## Known limitations

- Schema migration is currently version 1 only; later changes require explicit incremental migrations.
- Snapshot comparison, export, deletion, and real synchronization arrive in later modules.
- The demo profile is intentionally not a real synchronization result.
