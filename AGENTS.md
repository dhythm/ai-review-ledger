# AGENTS.md

## Guideline

- follow TDD, follow t-wada method
- read the latest code when starting a new task, because it might be updated independently of your work

### Monorepo

JavaScript is a pnpm workspace. Rust stays a Cargo workspace. Do not add Nx or Turbo.

- `apps/web` — React + TypeScript (Vite). pnpm workspace member (`pnpm-workspace.yaml`). Tests use Vitest and Testing Library.
- `apps/api` — Axum + SQLite. Cargo workspace member (`Cargo.toml`). Tests use `cargo test`.
- Root `package.json` — cross-cutting `dev:*`, `build:web`, and `test*` scripts.

Prerequisites: Rust stable 1.85+ (the lockfile includes edition 2024 crates) and Cargo, Node.js 24.15+, pnpm 10 (`packageManager` in the root `package.json`). If pnpm is missing: `corepack enable` then `corepack install`.

Install from the repository root. Do not run `npm install` inside `apps/web`; the lockfile is `pnpm-lock.yaml`.

```bash
pnpm install
```

### Tests (TDD, t-wada)

Write a failing test for the behavior first, make the smallest change that passes, then refactor without changing behavior.

```bash
pnpm test            # web, then API
pnpm test:web        # Vitest in apps/web
pnpm test:web:watch  # Vitest watch, for the red-green loop
pnpm test:api        # cargo test --workspace --locked
```

`pnpm test:api` runs from the repository root. `cargo test --workspace --locked` at the root is the same suite. Do not drop existing API tests.

GitHub Actions runs `pnpm test:web` and `pnpm test:api` on push to `main` and on pull requests.

Put web tests next to the code they cover (`apps/web/src/labels.test.ts`, `apps/web/src/components/Badges.test.tsx`). Date formatting tests assume `TZ=Asia/Tokyo`, set in `apps/web/vite.config.ts`. API tests stay in the Rust modules under `#[cfg(test)]`.

### Naming Rules

- Variable names, function names, and database column names should be written in English.
- Romanized Japanese (romaji) should be avoided and only used when absolutely necessary.

Example:

- Good: `last_name_kana`
- Bad: `sei_kana`

- Use plural names only for arrays.
- Use singular names for non-array values.

### UI Rules

- Do not add explanatory helper copy that restates what the control already implies (e.g. "optional", "works without selection", "failure falls back to X"). Prefer clear labels and option text; surface status in results when needed, not as instructional paragraphs under every field.
- Keep UI copy concise. Avoid tutorial-style descriptions unless the user explicitly asks for onboarding help.

---

## Workflow

### 1. Plan Mode Default

- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately – don't keep pushing
- Use plan mode for verication steps, not just building
- Write detailed specs upfront to reduce ambiguity

### 2. Subagent Strategy

- Use subagents liberally to keep main context window clean
  fad research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution

### 3. Self-Improvement Loop

- After ANY correction from the user: update `tasks/lessons.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start forelevant project

### 4. Verification Before Done

- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

### 5. Demand Elegance

- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes – don't over-engineer
- Challenge your o work before presenting it

### 6. Autonomo Bug Fing

- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests – then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

---

## Task Management

1. Plan First: Write plan to `tasks/todo.md` with checkable items
2. Verify Plan: Check in before starting implementation
3. Track Progress: Mark items complete as you go
4. ExplChanges: High-level summary at each step
5. Document Results: Add review section to `tasks/todo.md`
6. Capture Lessons: Update `tasks/lessons.md` after corrections

---

## Core Principles

- Simplicity First: Make every change as simple as possible. Impact minimal code.
- No Laziness: Find root causes. No temporary fixes. Senior developer standards.
- Minimal Impact: Changes should only touch what's necessary. Avontroducing bugs.

---
