# Grade repository module

## Responsibility

Own the local SQLite database for Grade Desk. It creates the version-1 schema, seeds an anonymous demo profile on a fresh database, and returns a typed dashboard summary to the desktop UI.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `get_dashboard` | Rust/Tauri command | Opens the app-data SQLite database, applies idempotent schema setup, seeds only an empty database, and returns typed all-course and professional-course GPA summaries. |
| `list_course_attempts` | Rust/Tauri command | Returns typed read-only course attempts ordered by course code. |
| `get_course_detail(attemptId)` | Rust/Tauri command | Returns one typed attempt with term, class number, and score components. |
| `numeric_probe_target(attemptId)` | Rust repository interface | Returns the minimum local course-number and official-grade metadata required for an explicitly requested remote probe; it never returns cookies or credentials. |
| `save_verified_numeric_score(attemptId, score)` | Rust repository interface | Stores a remote-confirmed numeric score as `official_numeric`, updates the local timestamp, and captures a local history snapshot. |
| `grade-desk.db` | Rust repository | Local SQLite database stored only under the Tauri application-data directory. |

## Data ownership

The module owns `profiles`, `terms`, `courses`, `course_attempts`, and `score_components`. The archive-workflow module owns `sync_runs`, immutable `grade_snapshots`, and `grade_changes`. `course_attempts.score_kind` makes official numbers, official grades, local derivations, and unavailable values mutually explicit.

## Security and privacy constraints

- SQLite is opened solely in Rust; no generic SQL command is exposed through Tauri IPC.
- The seeded record is anonymous demo data and contains no credentials or real student identifiers.
- Connections enable foreign keys and WAL mode. All seed writes run in one transaction.
- Errors identify only the local repository operation; they do not expose query parameters or data payloads.
- A remotely verified numeric score is distinct from an inferred conversion: it is saved only as `official_numeric`; absent confirmation leaves the official grade intact.
- GPA aggregation excludes `P` and `NP` from both the weighted-grade-point numerator and the credit denominator. Professional-course GPA includes only `专业必修`, `专业选修`, and `公共必修`; it excludes `公共选修` and other categories.

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

- Schema version 2 is established with idempotent table creation; later changes still require explicit incremental migrations.
- Real synchronization remains a later module.
- The demo profile is intentionally not a real synchronization result.
