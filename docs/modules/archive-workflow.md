# Archive workflow module

## Responsibility

Create immutable local grade snapshots, compare each snapshot to the preceding one, present pending grade changes for review, export the local transcript, and delete the local database only after explicit user confirmation.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `archive_current_data` | Rust/Tauri command | Captures every current course attempt into a new `sync_runs` and `grade_snapshots` record, then creates change records only where normalized fields differ. |
| `list_sync_runs` / `list_pending_changes` | Rust/Tauri commands | Return typed history and unreviewed changes only. |
| `review_pending_changes` | Rust/Tauri command | Marks all pending changes as reviewed with a local timestamp. |
| `export_grade_data(format)` | Rust/Tauri command | Writes a CSV or JSON export to the application's own `exports/` directory and returns a receipt. |
| `clear_local_data` | Rust/Tauri command | Deletes the local SQLite database plus WAL/SHM sidecar files. |

## Data ownership

The module owns derived history data in `sync_runs`, `grade_snapshots`, and `grade_changes`. It does not alter the current course-attempt records. Schema version 2 introduces `grade_changes` with links to its before and after snapshots.

## Security and privacy constraints

- “Create snapshot” is offline-only in this release; it does not contact CAS, JWXT, WebVPN, or any external service.
- Exports use a fixed application-data `exports/` destination. The frontend cannot supply paths or invoke general filesystem operations.
- Deletion is initiated only after a native confirmation in the UI. It removes only Grade Desk's database files, never school-side data.
- Snapshots remain append-only. A review only marks a change record; it never edits historical payloads.

## Dependencies

- Grade-repository SQLite schema and typed course reads.
- `serde_json` for JSON exports; standard-library file I/O for controlled local export and deletion.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
CI=true pnpm build
CI=true pnpm tauri build --debug
```

## Known limitations

- Export location is the app-data directory rather than a user-selected folder; a native save dialog is a future enhancement.
- The timestamps are Unix-epoch seconds for deterministic local ordering; formatted local dates will be added later.
- A fresh start after deletion intentionally recreates anonymous demo data until account synchronization is available.
