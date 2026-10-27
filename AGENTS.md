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

- One feature branch, one worktree. The main directory is read-only: nobody commits in it. It stays on `main`; while a milestone is in progress it tracks the current `release/vX.Y.Z` branch (updated with `git pull --ff-only` after each merge) and returns to `main` after the release, because Orca reads `.worktreeinclude`, `worktree.sharedDirectories` and their sources from the main directory.
- Worktrees are created with Orca (`orca worktree create`): `orca.yaml` builds the code graphs and links `node_modules` from the main directory, `.worktreeinclude` copies the local `.claude/` setup. Other environments use their own directories; worktree roots are in `.gitignore`.
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
| PostgreSQL tests | `task rust:test:pg` (needs `.env` from `.env.dist` and Docker) |
| Web: types, Biome, Steiger, knip, depcruise | `task web:check` · unit tests: `task web:test` |
| File structure, file size and crate layers | `task structure` · non-gating size report: `task structure:report` |
| Dogfood: throwaway compose stand + real cargo | `task dogfood` (latest stable) · `task e2e` (two latest stable) · `task dogfood:full` (publishes CRANE to its own registry, builds it from there, then builds offline; `task gate` runs it in the same stand after the e2e scenarios). Needs Docker and Compose with repeated `--env-file` (tested on v5.6.1) |
| Checks before a PR to `release/*` | `task check` + the tests the change touches (`task rust:test:pg` for storage, `task e2e` for HTTP, server and delivery) + a manual mutants table in the PR description |
| Full gate before the release PR `release/*` → `main` | `task gate` (unit tests + cargo-mutants and Stryker mutations over the whole release diff against `main` + e2e + Playwright) |
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
4. **Big files are forbidden: no file longer than 400 lines** (physical lines, as `wc -l` counts them; comments, blank lines and tests included). A file over 400 lines cannot be read in one sitting or reviewed properly, and it hides several concepts. Rules 1-3 apply to `crates/`, `web/src/`, `cli/`; rule 4 applies to `.rs`, `.ts`, `.tsx` under `crates/`, `web/src/`, `web/screens/`, `web/test/`, `cli/`, `tools/`. Only `generated/` and `migrations/` are exempt (`web/e2e` and `web/.storybook` are outside the roots by the owner's decision). `task structure` fails on it and lists offenders largest first; `task structure:report` (non-gating) lists the 20 largest files and everything from 320 lines (80% of the limit) so that the queue is visible before it breaks.

Not counted: manifests and configs (`Cargo.toml`, `package.json`, `tsconfig*.json`, …), test files (`*.test.*`, `*.spec.*`), the `tests/`, `migrations/` and `generated/` directories, dotfiles. Entry files (`lib.rs`, `main.rs`, `mod.rs`, `index.ts`) do not make a folder "mixed" but do count toward the limit of 4 files. Consequence for Rust: a module with submodules is `foo/mod.rs` + `foo/…`, not `foo.rs` next to `foo/`. If you exceed the limit, extract a subfolder by meaning; do not hide files in `misc/`.

Splitting a file that is too long:

- Split by meaning (one concept, one behaviour or one section per file), never `part1`/`misc`/`utils`.
- **Rust unit tests.** Keep `mod tests` inline while the file fits. When it does not, move the test module into a sibling `tests/` folder under the same file name, declared as the last top-level item of the module: `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;` (`foo/egress.rs` -> `foo/tests/egress.rs`, `foo/mod.rs` -> `foo/tests/mod.rs`). The body moves unchanged (`use super::*;` keeps access to private items); the `#[allow(..., reason = ...)]` becomes `#![allow(...)]` at the top of the file. `tests/` is exempt from the rule of four and from mutation testing, and is still checked by AAA. Never put `#[path]` inside an inline `mod x { }`.
- **Rust integration tests.** Every top-level file in `tests/` is a separate binary that links all dependencies and recompiles `common`. Do not split by adding top-level files: turn `tests/<name>.rs` into `tests/<name>/main.rs` (shared attributes, `#[path = "../common/mod.rs"] mod common;`, `mod` declarations) plus one module per behaviour. Split oversized helpers inside `tests/common/` by meaning.
- **TypeScript tests.** Split `x.test.ts` by subject into `x.<subject>.test.ts`; helpers shared by those files live in `x.fixtures.ts`.

## Code style

- **Rust:** "boring Rust" (section 3): no custom procedural macros and no type-level magic. Lints live in `[workspace.lints]` in `Cargo.toml` (`unsafe_code = forbid`; `unwrap`, `panic`, `todo`, `dbg!`, `println!` are forbidden); `rustfmt.toml`, `clippy.toml`. Errors: `thiserror` in `domain` and `app`, `problem+json` at the HTTP boundary.
- **Function length.** clippy `too-many-lines-threshold = 60` (code lines only: clippy skips blank lines and comments); Biome `noExcessiveLinesPerFunction` at 80 lines with blank lines skipped (Biome counts comments, and JSX is more verbose, hence 80 and not 60), off for `*.test.*` (`describe` is a container Biome cannot skip; AAA and the file limit already bound test bodies). Stories are not exempt. Cut a long function into named steps by meaning, not into "first half / second half". The only allowed exemption is a table-driven Rust test whose long `cases` table is data: `#[expect(clippy::too_many_lines, reason = "...")]` (`expect`, so the attribute fails when the table shrinks); extracting the table into a function would break AAA.
- **serde:** `#[derive(Serialize, Deserialize)]` is allowed only at the wire-format boundary: in `formats` (protocols) and `entry` (the HTTP protocol, configuration). It is forbidden in `domain`, `ports` and `app`; errors there use `thiserror`. Reason: parsing the JSON and XML of five formats by hand is thousands of lines.
- **TypeScript wherever possible:** scripts, configs and tools are written in `.ts` (Node 24 runs them directly, erasable syntax only: no `enum` and no `namespace`). `.mjs` files are forbidden in the repository; `.js`/`.cjs` only where a tool does not support TS.
- **TypeScript:** strict, Biome (format and lint), no `any`, no `console`, no `!`. The `~/` alias points to `src/`.
- **No warnings, strict lint.** Biome runs with `--error-on-warnings`, all active rules are errors (`biome.json`): `noMagicNumbers`, `noJsxLiterals`, `useBlockStatements`, `useTopLevelRegex`, type-aware domains (`types`, `project`, `react`, `test`) and cognitive complexity ≤ 15. There is no second linter (ESLint/SonarJS were deliberately rejected). Biome has no magic-strings rule, so name repeated string literals yourself. `knip` fails on unused files, exports, dependencies and stale config hints; `dependency-cruiser` also fails on unresolved imports and imports missing from `package.json`. Do not weaken a rule to make code pass: fix the code, or discuss the rule change in its own PR.
- Dependencies: permissive licenses only (`cargo deny`); a new dependency is justified in the PR. In `web`, `knip` catches unused code and dependencies.
- Comments explain "why", not "what". The language of comments and documentation is Russian.

## Testing

- Unit tests: Rust uses `#[cfg(test)]` next to the code and `tests/` for integration tests; web uses Vitest + Testing Library. AAA structure; a test checks behavior, not implementation.
- **Strict AAA, logic only (all non-e2e tests; checked by `task structure`, see `tools/lint-structure/aaa.ts`).**
  - Each test has the line comments `// Arrange`, `// Act`, `// Assert` exactly once, in this order. Act is exactly one top-level statement. No assertions before `// Assert`. Assert observes the outcome through read-only queries and never calls the operation under test again. Several assertions are allowed only when they are facets of one outcome (for example, the error plus "state unchanged"); split tests by action, not by assertion. When a runner wraps the test in a closure, the markers may sit inside it, but all three must be at the same nesting level.
  - Table-driven tests: in Rust the cases live in Arrange, Act is `let actual = cases.map(f);` and Assert is one comparison with the expected collection (no loops with act/assert); in TS each case is its own `it` (`it.each` or a loop at module top level with one AAA body per case).
  - Contract scenarios in `crates/core/ports/src/contract/` are tests and follow the same rules. The only exception is the binding test of an adapter: one statement calling `*_contract(`, without markers.
  - `#[should_panic]` is forbidden; assert on the returned error instead.
  - Test logic, not code. Do not write tests for getters and accessors, `Debug`/`Display`-only output, restatements of the implementation, interaction-only spies, test doubles, or tests whose only purpose is to kill a mutant. Secret-redaction tests stay.
  - A mutant in non-logic code is excluded in `.cargo/mutants.toml` with a Russian "why" comment; it is never killed by a tautological test.

  ```rust
  #[test]
  fn ensure_reports_both_states_of_forbidden_transition() {
      // Arrange
      let (from, to) = (Deleted, Released);

      // Act
      let result = from.ensure(to);

      // Assert
      assert_eq!(
          result,
          Err(TransitionError {
              from: Deleted,
              to: Released
          })
      );
  }
  ```
- Mutation testing: `cargo-mutants` and Stryker. The critical path (format protocols, access control, CAS and publishing, tokens) must hold the threshold. A feature PR to `release/*` carries a **manual mutants table** in its description (mutation → test that caught it), produced by the coder and the lead during review. The full gate runs **locally** (`task gate`) once per milestone, before the release PR `release/*` → `main`; it is not run in CI. Gaps it finds are fixed by PRs into the same release branch before the release PR merges. Playwright and Storybook tests are also local.

## Git flow

The full description (branch diagram, versioning, tag rules, release procedure) is in [GITFLOW.md](GITFLOW.md); below is the short version.

- `main` is the primary (production) branch. `release/vX.Y.Z` and feature branches are created from it.
- A feature branch from `main` → PR to `release/vX.Y.Z`, merged with **squash**. The PR title is a Conventional Commit (it becomes the squash message). The PR description contains the manual mutants table; `task gate` output (`Gate: passed`, `Gate-SHA`) goes into the release PR to `main`.
- **Rebasing feature branches onto release branches is allowed, and sometimes required.** A feature branch is created from `main`, but its PR targets a release branch, so rebase it onto `origin/release/vX.Y.Z` (`git rebase origin/release/vX.Y.Z`):
  - when the release branch received changes your work depends on (for example a fix your checks need), or that conflict with it;
  - before opening the PR and whenever the release branch moves ahead, so that the PR diff contains only your own changes.

  Rules: rebase only your own feature branches and push them with `git push --force-with-lease` (never plain `--force`); never rebase or force-push `main` or `release/*`; the rebase changes commit SHAs, so rerun your checks. Do not merge release branches into feature branches; use rebase.
- The owner approves the release → PR `release/vX.Y.Z` → `main`, merged with a **merge commit** (not squash: the commit history is needed for the CHANGELOG).
- After the merge into `main`, the `release` workflow bumps the versions (`Cargo.toml`, `package.json`, `Chart.yaml`), rebuilds `CHANGELOG.md` from commits (window "previous tag..HEAD", `git-cliff`), makes the `chore(release): vX.Y.Z` commit and creates the `vX.Y.Z` tag. The tag triggers the `publish` workflow: a GitHub Release with notes from the CHANGELOG. Images are not built for ghcr yet.
- Commit types: `feat fix perf refactor docs test build ci chore revert style`; the scope is kebab-case, the header is ≤ 100 characters.

## Documentation

- `docs/product/` is maintained in Russian; the root files (README, CONTRIBUTING, SECURITY, code of conduct) are Russian with an `-EN` mirror. `AGENTS.md` and `CLAUDE.md` are English only. After edits, build the guidebook (`tools/build-guidebook.sh`, `--strict` mode).
- If you add or remove functionality, update `tools/estimate-agent-hours.py` and the numbers in sections 1 and 18.
