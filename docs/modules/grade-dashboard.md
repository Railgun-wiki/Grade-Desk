# Grade dashboard module

## Responsibility

Render the local, read-only grade experience: cumulative overview, searchable transcript, and a course-detail panel. It translates the Apple design reference into an information-dense desktop tool without copying branding or using decorative product imagery.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `get_dashboard` | Rust repository command | Supplies all-course GPA, professional-course GPA, earned credits, and the local archive timestamp. |
| `list_course_attempts` | Rust repository command | Supplies the flattened, read-only transcript list. |
| `get_course_detail(attemptId)` | Rust repository command | Supplies one attempt, its academic term, class number, and available score components. |
| Overview / transcript navigation | React state | Changes only the displayed local view; it does not mutate data. |

## Data ownership

The module owns no data. It reads typed records from the grade repository and uses in-memory UI state only for search text, active view, selected course, and per-detail numeric-probe state.

## Security and privacy constraints

- No direct HTTP, filesystem, credential, cookie, or SQL access exists in the frontend.
- The browser-preview fallback is anonymous seeded data only; a real Tauri command supersedes it when available.
- Numeric scores are labelled `教务数值`; grade-only records remain `官方等级` and are not converted into guessed scores.
- GPA cards label their scope explicitly. `P`/`NP` records are excluded from both GPA formulas rather than treated as zero points or zero-credit grades.

## Design system translation

- Apple-derived surfaces: black 44px global bar, frosted parchment context bar, white utility cards, and one near-black information panel.
- Apple-derived tokens: Action Blue `#0066CC`, Sky Link Blue `#2997FF` on dark surfaces only, 18px cards, pill actions, 17px body copy, and SF system font stack.
- The application uses no gradients or card/button shadows. Focus rings and 44px targets support keyboard and touch use.
- The app shell is a single viewport: global/context navigation and the sidebar remain fixed while non-overview content scrolls in its own region. The overview page itself does not scroll; its variable-length course list scrolls inside its card.
- Course-detail panels are keyed by attempt ID and reset their numeric-probe confirmation/result state whenever the user selects another course.

## Dependencies

- React state and Tauri typed invokes.
- The grade-repository module for all records.
- No additional UI or chart library.

## Verification

```sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
CI=true pnpm build
CI=true pnpm tauri build --debug
```

## Known limitations

- Course detail only exposes the available seeded components; unavailable components are stated plainly.
- Search is client-side and only covers the loaded transcript fields.
- On very short windows, overview cards retain their viewport layout and only the course-list card becomes scrollable.
- Analysis, snapshot comparison, exports, real synchronization, and account connection remain out of scope for this module.
