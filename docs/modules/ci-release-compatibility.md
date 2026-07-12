# Platform Compatibility and CI/Release Module

## Responsibility

This module is responsible for:
1. Ensuring the application window, title bars, layout bounds, dragging regions, and transparent visual effects adapt natively to macOS, Windows, and Linux.
2. Maintaining a clean, uniform Apple-inspired UI design for all page contents (fonts, colors, cards, and buttons) across all operating systems.
3. Defining and orchestrating automated GitHub Actions workflows for continuous integration (type checks, lints, formats, and build verification) and draft release publishing.

## Public interfaces

| Interface | Owner | Contract |
|---|---|---|
| `application_status` | Rust command (modified) | Includes the platform OS type (`macos`, `windows`, or `linux`) in the returned status payload. |
| `.os-macos`, `.os-windows`, `.os-linux` | CSS classes | Applied to `document.body` to override window-level visual settings and layout geometry. |
| `.github/workflows/ci.yml` | GitHub Actions | Automatically triggers on pushes and PRs to verify the project builds on macOS, Windows, and Linux. |
| `.github/workflows/release.yml` | GitHub Actions | Automatically triggers on version tags to build installers and compile production draft releases. |

## Data ownership

This module does not own any student, credential, session, or grade database tables. It only references structural application metadata (i.e. the current operating system).

## Security and privacy constraints

- The GitHub Actions workflows run in isolated runners.
- The Release workflow uses the default `GITHUB_TOKEN` to publish releases, preventing exposure of personal access tokens or keys.
- Code signing (macOS/Windows certificates) is skipped by default in the basic workflow, as distribution is intended for manual side-loading or local builds. Secrets should be added in GitHub settings if signing is enabled in the future.
- The platform metadata exposed by `application_status` contains only general operating system names and does not collect or transmit unique hardware, device, or user identifiers.

## Dependencies

- **pnpm-managed**: `@tauri-apps/api`, `react`, `react-dom`.
- **Cargo-managed**: `tauri` (v2), `serde`.
- **GitHub-managed Actions**:
  - `actions/checkout@v4`
  - `actions/setup-node@v4`
  - `pnpm/action-setup@v3`
  - `dtolnay/rust-toolchain@stable`
  - `tauri-apps/tauri-action@v0`

## Verification

### Local Layout Check
Inject different class names in the DOM using devtools (`os-macos`, `os-windows`, `os-linux`) and verify that:
- On macOS, the top drag header is 25px tall and the sidebar padding is 40px.
- On Windows and Linux, the top drag header is hidden, and the sidebar padding-top is 20px.
- On Linux, the background color of the body and sidebar is solid opaque to prevent transparency bugs.

### Workflow Linting & Execution
- Verify syntax with `actionlint` if installed.
- Pushes to branches or pull requests trigger the CI job.

## Known limitations

- Real Windows/macOS app code signing requires dedicated developer profiles and code-signing certificates which must be configured as repository secrets.
- Auto-updating is not configured in the basic workflow.
