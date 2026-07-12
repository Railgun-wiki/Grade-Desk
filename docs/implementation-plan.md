# Grade Desk implementation plan

> Status: construction active — JWXT session integration in progress
> Workspace: `/Users/yuki/Documents/coding/Sysuer`  
> Reference only: `./Sysuer` (Android application, separate Git repository)

## Documentation structure

- `AGENTS.md`: workspace rules, pnpm requirement, and documentation policy.
- `docs/implementation-plan.md`: cross-module scope, sequencing, and decisions.
- `docs/modules/<module-name>.md`: one maintained document for every implementation module. Each records ownership, interfaces, data, security, dependencies, verification, and limitations.
- [`docs/modules/app-shell.md`](modules/app-shell.md): Tauri, Vite, and Rust application shell.
- [`docs/modules/grade-repository.md`](modules/grade-repository.md): local SQLite schema, seed data, and typed repository commands.
- [`docs/modules/grade-dashboard.md`](modules/grade-dashboard.md): read-only overview, transcript, and course detail interface.
- [`docs/modules/archive-workflow.md`](modules/archive-workflow.md): local snapshots, review queue, controlled export, and deletion.
- [`docs/modules/jwxt-session.md`](modules/jwxt-session.md): macOS controlled WebView, local session persistence, and official query verification.
- [`docs/modules/logging.md`](modules/logging.md): Rust-side structured diagnostics and standard `RUST_LOG` filtering.
- [`docs/modules/ci-release-compatibility.md`](modules/ci-release-compatibility.md): Platform-native window styling overrides and GitHub Actions workflows.

## Confirmed product direction

- A student-facing desktop application for viewing, analyzing, archiving, and exporting personal grades.
- Tauri 2 + Vite + TypeScript frontend, Rust command layer, and local SQLite storage.
- Apple-inspired interaction language: restrained surfaces, one blue action color, system typography, accessible data tables, no decorative gradients.
- The original Sysuer design document is retained in the reference repository; this document is the independent project's execution record.

## Planned delivery steps

| Step | Deliverable | Verification | Commit |
|---|---|---|---|
| 1 | Repository governance, scope, and build plan | `git diff --check` | `docs: establish grade desk workspace plan` |
| 2 | Tauri/Vite/Rust project shell and toolchain scripts; `docs/modules/app-shell.md` | pnpm frontend type check + Tauri build | `chore: scaffold grade desk desktop app` |
| 3 | SQLite schema, migrations, repository commands, and seeded local demo profile; `docs/modules/grade-repository.md` | Rust tests + migration test | `feat(data): add local grade repository` |
| 4 | Overview, transcript, and course-detail UI using local data; `docs/modules/grade-dashboard.md` | type check + production build | `feat(ui): add grade dashboard` |
| 5 | Snapshot history, change review, export, and local-data deletion; `docs/modules/archive-workflow.md` | tests + manual acceptance checklist | `feat(sync): add grade history workflow` |
| 6 | CAS/JWXT adapter; `docs/modules/jwxt-session.md` | integration tests against approved environment | `feat(auth): add authorized jwxt sync` |
| 7 | Multi-platform native window compatibility and GitHub Actions CI/Release; `docs/modules/ci-release-compatibility.md` | local build on macOS/Windows/Linux + verification | `feat(ci): add platform-native window support and actions` |

## Scope guardrails

- Step 2–5 must work without a school account and use local seeded data only.
- The frontend invokes only typed Rust commands; it does not receive raw credentials, cookies, tickets, or a generic SQL channel.
- Numeric-score resolution remains opt-in, rate-limited, and unavailable by default. When no verified value exists, the UI keeps the official grade rather than guessing.
- The `Sysuer/` directory stays untouched by all new-project commits.

## Decisions pending before Step 6

1. Whether CAS login must use the system browser + callback or an approved embedded flow.
2. Whether real numeric-score resolution is authorized for release, including request-rate limits.
3. Default GPA and retake calculation policies.
4. First release target: macOS only or macOS + Windows.
