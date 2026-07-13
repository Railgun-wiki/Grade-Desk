# Analysis module

## Responsibility

Provide local, explainable academic analysis. It aggregates GPA, earned credits, and course counts by academic term; ranks course GPA contributions; and displays verified numeric-score distribution. It never predicts results, queries JWXT, or treats snapshots as a trend source.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `get_analysis_overview` | Rust/Tauri command | Returns term trends, GPA contribution records, numeric-score distribution, data-coverage counts, and local archive timestamp. |
| `list_terms` | Rust/Tauri command | Returns locally known terms in descending academic-term order for transcript paging. |

## Data ownership

The module owns no persistent tables. It reads `terms`, `course_attempts`, `courses`, and completed local archive timestamps from the grade repository.

## Security and privacy constraints

- All aggregation runs in Rust against SQLite; the frontend receives typed, read-only results only.
- Analysis performs no CAS/JWXT request, credential access, or automatic synchronization.
- Numeric distribution counts only `official_numeric`. Official grade-only records are reported as excluded, never converted to guessed scores.
- `P`/`NP` are excluded from GPA calculations and identified in data coverage.

## Dependencies

- Grade repository typed records and SQLite indexes.
- Grade dashboard UI for course-detail navigation.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
CI=true pnpm build
CI=true pnpm tauri build --debug
```

## Known limitations

- Trends are per academic term only; they are not predictions, official rank trends, or snapshot timelines.
- GPA contribution is relative to the current local cumulative GPA and is an explanatory metric, not an official calculation.
