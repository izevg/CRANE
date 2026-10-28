# Contributing to CRANE

[Русский](CONTRIBUTING.md) · **English**

Thank you for wanting to help. This document explains what kind of contribution the project needs right now, how the repository is organized, and which rules apply to documentation and code.

By participating, you agree to the [Code of Conduct](CODE_OF_CONDUCT-EN.md). Do not report vulnerabilities in public issues; see [SECURITY-EN.md](SECURITY-EN.md).

## Contents

- [Where the project is now](#where-the-project-is-now)
- [How to help](#how-to-help)
- [Repository layout](#repository-layout)
- [Documentation guidelines](#documentation-guidelines)
- [Code guidelines](#code-guidelines)
- [Commits and DCO sign-off](#commits-and-dco-sign-off)
- [Pull requests and review](#pull-requests-and-review)
- [Contribution license](#contribution-license)

## Where the project is now

CRANE is in the design stage: the repository holds the architecture, the effort estimate and a guidebook, and there is no code yet. The most valuable contribution right now is a critical look at the architecture. The code guidelines below take effect with the first code commit; they are published early so expectations are clear.

## How to help

| What | How |
| --- | --- |
| You found a flaw or weak spot in the architecture | Open an issue labeled `architecture`: the documentation section, what is wrong, what it leads to, your proposal |
| You know a simpler way | Open an issue labeled `proposal`: the problem you solve, alternatives, the trade-offs |
| An inaccuracy, typo or outdated fact | Open a pull request against `docs/product/`; cite a source for facts |
| You have a question | Open an issue labeled `question`. A question the docs do not answer is a signal that the docs need work |
| You have run Nexus, Artifactory or other package registries | Open an issue labeled `field-experience`: real scenarios and pain points beat assumptions |

Search for similar issues first. One topic per issue.

## Repository layout

```
docs/product/        solution architecture, the source of truth (20 sections)
guidebook/           manifest.yaml: PDF guidebook styling on top of docs/product
brand/               logo and mark in SVG
crates/ web/ cli/    code (arrives with the first code); layout in section 8
tools/
  lint-structure/          rule of four and crate layers (TypeScript)
  bump-version.ts          version bump on release
  build-guidebook.sh       builds the PDF with guidebook-render
  estimate-agent-hours.py  the effort model from section 18
Taskfile.yml         single entry point: `task --list`
out/                 build artifacts, not committed
```

## Documentation guidelines

- **Language.** Documentation in `docs/product/` is written in Russian. Root files (README, CONTRIBUTING, SECURITY, code of conduct) exist in Russian with an English mirror suffixed `-EN`; if you change one, update both.
- **Style.** Short sentences, active voice, terms from the [glossary](docs/product/99-glossary.md). Numbers are backed by a calculation or a source, and estimates are called estimates. No marketing, no superlatives.
- **Diagrams.** Mermaid inline in Markdown, captioned `*Рис. N.M. …*` on the line below, so they render on GitLab and GitHub and flow into the guidebook.
- **Cross-references** use section numbers ("section 14.9"), never page positions.
- **Guidebook.** After editing, build the PDF with `tools/build-guidebook.sh`. The build runs with `--strict`: if you rewrote a passage that styling in `guidebook/manifest.yaml` is anchored to, the build stops with a warning; update the manifest.
- **Effort estimate.** If you add or remove functionality, update the modules in `tools/estimate-agent-hours.py` and the figures in sections 1 and 18.

## Code guidelines

These take effect with the first code.

- **Single entry point:** `task check` (see `AGENTS.md`). `lefthook` hooks are installed by `pnpm install`; do not bypass them.
- **Code architecture:** Rust is DDD + hexagonal + clean architecture, web is Feature-Sliced Design ([section 8](docs/product/08-c4-l4-code.md), [section 15](docs/product/15-frontend.md)).
- **Secrets for compose and PostgreSQL tests:** `cp .env.dist .env`, then `task secrets:init` creates the secret files in `./secrets/` (gitignored; existing files are never overwritten).
- **Rule of four:** at most 4 files and 8 subfolders per folder; mixed folders (files plus subfolders) are forbidden. Checked by `task structure`.
- **TypeScript wherever possible:** scripts and configs are `.ts`; `.mjs` files are not accepted in the repository.
- **Rust:** stable toolchain; `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass cleanly. The style is the "boring Rust" of [section 3](docs/product/03-load-forecast-rust-vs-fastapi.md): no custom procedural macros, no clever type gymnastics.
- **Dependencies:** permissive licenses only; `cargo deny check` (licenses and advisories) is mandatory. Justify every new dependency in the pull request description.
- **Frontend:** strict TypeScript; the linter and tests (Vitest, Testing Library, Playwright) pass cleanly.
- **Storybook.** A kit component (`shared/ui`) ships with a `*.stories.tsx` next to it; axe violations fail the run. One-time setup per machine: `pnpm --filter web exec playwright install chromium`. Story tests (headless Chromium) run together with the unit tests in `task web:test`; the showcase is `task web:storybook` (port 6006), the static build is `task web:storybook:build`. How to write stories: [web/.storybook/README](web/.storybook/README.md).
- **Tests.** Every behavior change comes with a test. Tests follow strict AAA (`// Arrange`, `// Act`, `// Assert`, a single statement in Act; several assertions only as facets of one outcome) and cover logic only, not code: no tests for getters, `Debug`/`Display`, or tests that exist only to kill a mutant in code without logic. `task structure` checks compliance; the full rules are in [AGENTS.md](AGENTS.md). The critical path (format protocols, access control, CAS and publishing, tokens) is additionally covered by mutation testing (cargo-mutants, Stryker). The full gate `task gate` runs locally before a PR to `release/*`.
- **UI text (i18n).** Visible strings are Russian (the source language) and written with Lingui macros (``t`…` ``, `plural`, `msg`) at the point of use; the EN translation lives in `web/src/shared/lib/i18n/locales/en.po`. A Cyrillic literal outside a macro is caught by `tools/lint-i18n` (part of `task web:check`); the only exception is showcase content in `web/screens` (package names, people, descriptions, journal events): its declaration carries the line `// i18n-fixture: <reason>` above it, and the service parts of stories (`play`, `title`, `name`) are not checked. Refresh catalogs with `pnpm --filter web run i18n:extract`. Unit tests get `I18nProvider` automatically: `@testing-library/react` is aliased to `web/test/rtl.tsx`. One message convention: `t` from `useLingui` (`@lingui/react/macro`) at the point of use, `msg` only in tables keyed by an enum (theme modes, problem kinds), `i18n._(msg…)` outside render; there are no separate `messages.ts` files. Logic (branching, calculations, selection) does not live next to macros: it goes into pure `.ts` modules without `@lingui/*/macro`, and the component only calls `t` and `plural` on ready data. Stryker mutates everything except macros: the `lingui-macros` ignorer (`tools/stryker-lingui/ignorer.ts`, wired in `web/stryker.config.json`) creates no mutants inside `t`, `msg`, `plural` and in function bodies that call `useLingui()`, because the instrumenter wrappers would otherwise break macro expansion.
- **Security.** No secrets in code, fixtures or logs; secrets are wrapped in a type that cannot be printed. Untrusted data (package contents, metadata, model output) never influences permissions.
- **Compatibility.** Protocol changes are checked against the matrix of real clients; database schema changes follow expand / migrate / contract (section 16.4).

## Commits and DCO sign-off

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `feat:`, `fix:`, `perf:`, `refactor:`, `docs:`, `test:`, `build:`, `ci:`, `chore:`. Imperative subject line, up to 100 characters. Enforced by commitlint (`commit-msg` hook and CI).
- **AI-agent attribution is forbidden**: no `Co-Authored-By: <agent>`, "Generated with …" or 🤖 in commits and pull request descriptions. The author of a contribution is the human who answers for it.
- One commit, one logical change.
- Every commit is signed off under the [Developer Certificate of Origin](https://developercertificate.org/): `git commit -s` adds a `Signed-off-by: Name <email>` line. By signing off, you certify that you have the right to submit the contribution under the project license. No CLA is required.

## Pull requests and review

1. Branch from `main`; keep pull requests small and focused. Open the PR against `release/vX.Y.Z` (not `main`); only the release itself goes to `main`.
2. In the description: what changed and why, links to issues and documentation sections, how you verified it.
3. All CI checks are green.
4. A maintainer approval is required; changes to access control, tokens, cryptography, protocols or audit need two, one of them from the security owner.
5. Review comments are discussed in the pull request, and the decision is recorded there.
6. Merges into `release/*` are squashed; the PR title follows Conventional Commits and the description carries a manual mutants table; the full `task gate` runs before the release PR to `main`.
7. After the release is approved, the `release/vX.Y.Z` → `main` PR is merged with a merge commit; the `release` workflow sets versions, CHANGELOG and the `vX.Y.Z` tag ([section 16.7](docs/product/16-deployment.md)).

## Contribution license

The project is licensed under the [Apache License 2.0](LICENSE). Under section 5 of the license, any contribution submitted to the project is accepted under the same terms.
