# AGENTS.md

Instructions for AI agents and humans working in the CRANE repository. The product architecture lives in `docs/product/` (the source of truth); this file covers only what is needed to work with the code and the repository.

This file and `CLAUDE.md` are written in English. Product documentation (`docs/product/`) and code comments stay in Russian.

## Prohibitions (always in force)

- **No attribution of AI agents.** Not in commits, pull request descriptions, code or documents: no `Co-Authored-By: <agent>`, `Generated with …`, `🤖`. This is a product rule; `commitlint` enforces it (locally and in CI).
- No secrets in code, test data or logs. Secrets are wrapped in a type without `Debug`/`Display`.
- Do not commit to `main` or `release/*` directly, do not push with `--force` to protected branches, do not bypass hooks (`--no-verify`).

## Project stage

Design plus skeleton. Code starts with `crates/core/domain`. The commands in the README describe the target interface.

## Roadmap

Until all features are fully released (the P0–P3 scope of [ROADMAP.md](ROADMAP.md) is complete, the last milestone being the Nexus migration), agents **must follow the milestones** of `ROADMAP.md`:

- Work is taken only from the current milestone (the nearest version not marked "done"). Tasks of later versions are not started, even if they look simple or "on the way".
- The order and composition of milestones, the completion criteria and the dates are not changed on an agent's initiative. If a task does not fit a milestone, or a milestone hits a blocker, stop and ask the owner.
- A change to the plan is a separate PR to `ROADMAP.md` (with the dates recalculated by `python3 tools/estimate-agent-hours.py --start …`) and takes effect after the owner approves it.
- A PR references the milestone (the `release/vX.Y.Z` version) it goes into; a PR to a different release branch is not opened without agreement.

## Worktrees

All work is done in separate git worktrees, not in the main working directory:

- One feature branch, one worktree. The main directory stays on `main`: nobody commits or switches branches in it.
- The location and the way of creating a worktree are defined by the settings of your IDE or agent environment; paths are not hardcoded in the repository (Claude Code uses `.claude/worktrees/`, others use their own directories; worktree roots are in `.gitignore`).
- After creating a worktree, run `task install` (every worktree has its own `node_modules` and lefthook hooks).
- Do not use the shared `git stash` (the stack is shared by all worktrees); set work aside with a WIP commit.
- **Automatic cleanup.** Before starting any task, and again whenever a PR you opened has been merged, remove the worktrees whose branch was merged through a PR into `main` or a `release/*` branch:
  - Determine the PR state through `gh` (`gh pr list --head <branch> --state merged --json number,baseRefName,headRefOid`) or the GitHub MCP, never through `git branch --merged`: a squash merge leaves the branch "unmerged" from git's point of view.
  - Remove a worktree only if its PR is `MERGED` into `main` or `release/*`, the worktree is clean, and its `HEAD` equals the PR's `headRefOid` (no commits after the merge). Never remove the main directory, the current worktree, `main` or `release/*` worktrees, or a detached `HEAD`.
  - Removal deletes the worktree and its local branch (`git worktree remove <path>` and `git branch -D <branch>`; the remote branch is deleted by GitHub on merge).
  - `task worktrees:prune` does exactly this: dry-run by default, `task worktrees:prune -- --apply` to remove.

## Commands

The single entry point is the [Taskfile](Taskfile.yml) (`task --list`).

| Task | Command |
| --- | --- |
| Dependencies and hooks | `task install` (Node ≥ 24, see `.nvmrc`) |
| All fast checks | `task check` |
| Rust: fmt, clippy, test, deny | `task rust:check` |
| Web: types, Biome, Steiger, knip, depcruise | `task web:check` · unit tests: `task web:test` |
| File structure and crate layers | `task structure` |
| Full gate before a PR to `release/*` | `task gate` (unit tests + cargo-mutants and Stryker mutations + Playwright) |
| Prune worktrees of merged branches | `task worktrees:prune` (dry-run) · `task worktrees:prune -- --apply` |
| Version bump, CHANGELOG, guidebook | `task bump TAG=vX.Y.Z` · `task changelog` · `task guidebook` |

## Code architecture

- **Rust: DDD + hexagonal + clean architecture.** Dependencies point inward:
  `entry → adapters | formats | access | app → ports → domain`.
  `domain` has no IO and no dependencies on the rest of the project; `ports` are traits (ports) on top of `domain`; `app` holds the use cases; `adapters` and `formats` implement the ports and do not know about each other; `entry` (http, worker, analyzer, server) is the only place where dependencies are assembled. The matrix of allowed dependencies is `tools/lint-structure/rules.json`, checked against `cargo metadata`.
- **Web: Feature-Sliced Design.** Layers `app → pages → widgets → features → entities → shared`; imports go only downward, no cross-slice imports, a slice is accessed only through `index.ts`. Steiger and dependency-cruiser enforce this.

## The rule of four (file structure)

Applies to `crates/`, `web/src/`, `cli/`. Checked by `tools/lint-structure` in the pre-commit hook and in CI.

1. No more than **4 files** per folder.
2. No more than **8 subfolders** per folder.
3. **Mixed folders are forbidden**: a folder contains either files or subfolders.

Not counted: manifests and configs (`Cargo.toml`, `package.json`, `tsconfig*.json`, …), test files (`*.test.*`, `*.spec.*`), the `tests/`, `migrations/` and `generated/` directories, dotfiles. Entry files (`lib.rs`, `main.rs`, `mod.rs`, `index.ts`) do not make a folder "mixed" but do count toward the limit of 4 files. Consequence for Rust: a module with submodules is `foo/mod.rs` + `foo/…`, not `foo.rs` next to `foo/`. If you exceed the limit, extract a subfolder by meaning; do not hide files in `misc/`.

## Code style

- **Rust:** "boring Rust" (section 3): no procedural macros and no type-level magic. Lints live in `[workspace.lints]` in `Cargo.toml` (`unsafe_code = forbid`; `unwrap`, `panic`, `todo`, `dbg!`, `println!` are forbidden); `rustfmt.toml`, `clippy.toml`. Errors: `thiserror` in `domain` and `app`, `problem+json` at the HTTP boundary.
- **TypeScript wherever possible:** scripts, configs and tools are written in `.ts` (Node 24 runs them directly, erasable syntax only: no `enum` and no `namespace`). `.mjs` files are forbidden in the repository; `.js`/`.cjs` only where a tool does not support TS.
- **TypeScript:** strict, Biome (format and lint), no `any`, no `console`, no `!`. The `~/` alias points to `src/`.
- Dependencies: permissive licenses only (`cargo deny`); a new dependency is justified in the PR. In `web`, `knip` catches unused code and dependencies.
- Comments explain "why", not "what". The language of comments and documentation is Russian.

## Testing

- Unit tests: Rust uses `#[cfg(test)]` next to the code and `tests/` for integration tests; web uses Vitest + Testing Library. AAA structure; a test checks behavior, not implementation.
- Mutation testing: `cargo-mutants` and Stryker. The critical path (format protocols, access control, CAS and publishing, tokens) must hold the threshold. The full gate runs **locally** (`task gate`) before a PR to `release/*`; it is not run in CI. Playwright and Storybook tests are also local.

## Git flow

The full description (branch diagram, versioning, tag rules, release procedure) is in [GITFLOW.md](GITFLOW.md); below is the short version.

- `main` is the primary (production) branch. `release/vX.Y.Z` and feature branches are created from it.
- A feature branch from `main` → PR to `release/vX.Y.Z`, merged with **squash**. The PR title is a Conventional Commit (it becomes the squash message). The PR description contains the output of `task gate` (`Gate: passed`, `Gate-SHA`).
- **Rebasing feature branches onto release branches is allowed, and sometimes required.** A feature branch is created from `main`, but its PR targets a release branch, so rebase it onto `origin/release/vX.Y.Z` (`git rebase origin/release/vX.Y.Z`):
  - when the release branch received changes your work depends on (for example a fix that `task gate` needs), or that conflict with it;
  - before opening the PR and whenever the release branch moves ahead, so that the PR diff contains only your own changes.

  Rules: rebase only your own feature branches and push them with `git push --force-with-lease` (never plain `--force`); never rebase or force-push `main` or `release/*`; the rebase changes commit SHAs, so rerun `task gate` and update `Gate-SHA` in the PR description (CI requires the `Gate-SHA` to be in the PR history). Do not merge release branches into feature branches; use rebase.
- The owner approves the release → PR `release/vX.Y.Z` → `main`, merged with a **merge commit** (not squash: the commit history is needed for the CHANGELOG).
- After the merge into `main`, the `release` workflow bumps the versions (`Cargo.toml`, `package.json`, `Chart.yaml`), rebuilds `CHANGELOG.md` from commits (window "previous tag..HEAD", `git-cliff`), makes the `chore(release): vX.Y.Z` commit and creates the `vX.Y.Z` tag. The tag triggers the `publish` workflow: a GitHub Release with notes from the CHANGELOG. Images are not built for ghcr yet.
- Commit types: `feat fix perf refactor docs test build ci chore revert style`; the scope is kebab-case, the header is ≤ 100 characters.

## Documentation

- `docs/product/` is maintained in Russian; the root files (README, CONTRIBUTING, SECURITY, code of conduct) are Russian with an `-EN` mirror. `AGENTS.md` and `CLAUDE.md` are English only. After edits, build the guidebook (`tools/build-guidebook.sh`, `--strict` mode).
- If you add or remove functionality, update `tools/estimate-agent-hours.py` and the numbers in sections 1 and 18.
