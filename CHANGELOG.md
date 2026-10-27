# Журнал изменений

Формат — [Conventional Commits](https://www.conventionalcommits.org/ru/v1.0.0/), версии — [SemVer](https://semver.org/lang/ru/).

## [v0.1.0] — 2026-10-27

### Возможности
- **core:** Add workspace skeleton, registry domain and ports (#8) (3acbdeb)
- **app:** Add publish, fetch and yank use cases (#10) (a6490e7)
- **adapters:** Add PostgreSQL catalog and object_store CAS (#12) (199cf7c)
- **entry:** Serve cargo sparse registry over HTTP with audit seams (#15) (da27cdd)


### Документация
- Add roadmap with release calendar and debt register (#2) (8f8b3b4)
- **agents:** Require following ROADMAP milestones, working in worktrees and allow rebasing (#3) (4735cc2)
- **gitflow:** Add GITFLOW.md, worktree cleanup rule and roadmap Gantt (#4) (bb3f1c9)
- **roadmap:** Move full dogfood build check to v0.2.0 (#11) (f689603)
- **gitflow:** Run the full gate once per milestone before the release PR to main (#14) (4f6e485)
- **plan:** Record v0.1.0 decisions and recalibrate the release split (#18) (6d719ac)
- **plan:** Verify supply-chain attestations and make evidence extensible (#21) (0d6e1b8)
- **ui:** Add web console design docs and admin/CLI OpenAPI contracts (#22) (55cf362)


### Исправления
- **gate:** Load stryker vitest plugin and skip web mutations without tests (0c884fa)
- **gate:** Load stryker vitest plugin and skip web mutations without tests (88c6488)
- **entry:** Keep security journal on regardless of RUST_LOG (#17) (9306f20)


### Служебное
- **deploy:** Add compose, helm chart and stricter lint config (#5) (ef95453)
- **tools:** Wire graft and graphify code graphs for agents (#6) (178a512)
- **tools:** Prepare Orca worktrees with orca.yaml and .worktreeinclude (#7) (a066981)
- **tools:** Ignore node_modules symlinks in Orca worktrees (#9) (4d614b2)
- Enforce strict AAA and logic-only tests (#13) (36ed161)
- **dogfood:** Prove cargo publish, fetch and yank against compose stand (#16) (86d5006)
- **entry:** Close mutants left by the v0.1.0 release gate (#19) (8321a40)
- **entry:** Cover healthcheck role and exclude process plumbing from mutants (#20) (03e5c91)

## [v0.0.0] — 2026-10-27

### Служебное
- Initial commit (787f801)

