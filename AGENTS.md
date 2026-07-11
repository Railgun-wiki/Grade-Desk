# Grade Desk workspace instructions

This directory is the independent workspace for the student grade-management desktop application. The `Sysuer/` directory is an existing Android reference project and a separate Git repository; treat it as read-only reference material unless the user explicitly asks to change it.

## Product boundary

- Build the new application at this workspace root, using Tauri 2, Vite, TypeScript, Rust, and SQLite.
- Use `pnpm` exclusively for JavaScript and frontend dependency management. Commit `pnpm-lock.yaml`; do not introduce npm, Yarn, or Bun lockfiles.
- Do not place new desktop-app code inside `Sysuer/`.
- Use Sysuer only to understand public UI behavior, data fields, and authentication/session handling. Do not copy credentials, cookies, secrets, or user data.
- Keep CAS and real-score lookup behind a Rust-side adapter. The frontend must never receive credentials, session cookies, or arbitrary SQL access.

## Delivery sequence

1. Maintain the design and implementation plan in `docs/` before creating application code.
2. Create the Tauri/Vite shell and a local SQLite repository layer.
3. Build the read-only grade dashboard with seeded local data.
4. Add an explicit, permissioned synchronization adapter and snapshot/change-history flow.
5. Add real CAS/JWXT integration only after the user confirms the authorization and deployment constraints.

## Working agreements

- Use Conventional Commits: `type(scope): imperative summary`.
- Make one focused commit after every completed delivery step. Do not mix reference-project changes into these commits.
- Run formatting, type checking, and the applicable test/build command before each commit; report any environment blocker plainly.
- Do not commit `node_modules`, `dist`, `target`, local SQLite databases, `.env*`, credentials, cookies, or generated application keys.
- Preserve the distinction between official grades, verified numeric scores, and locally derived calculations in both schema and UI.
- Default to local-only storage, opt-in diagnostics, accessible keyboard navigation, and a minimum 44×44px interactive target.

## Documentation requirements

- Every implemented module must have its own Markdown document in `docs/modules/<module-name>.md` before or alongside its first code commit.
- A module document must state: responsibility, public interfaces/commands, data ownership, security and privacy constraints, dependencies, verification commands, and known limitations.
- Update the module document whenever its interface, data schema, behavior, or verification procedure changes. Include the documentation update in the same focused Conventional Commit as the module change.
- Keep `docs/implementation-plan.md` as the cross-module roadmap; it must link to all module documents once they exist.

## Current delivery state

Construction is active. Complete the documented delivery steps in order and commit each verified step separately.
