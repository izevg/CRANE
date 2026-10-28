# Журнал изменений

Формат — [Conventional Commits](https://www.conventionalcommits.org/ru/v1.0.0/), версии — [SemVer](https://semver.org/lang/ru/).

## [v0.2.0] — 2026-10-28

### Возможности
- **tools:** Make agent capacity explicit in the estimate and record the observed scenario (#29) (782f203)
- **web:** Add UI kit atoms, molecules, shell and Storybook (#30) (6f3272d)
- **proxy:** Add format-agnostic proxy engine with fetch-through cache and SSRF guard (#31) (671e8aa)
- **formats:** Proxy crates.io index and archives through the proxy engine (#33) (05eb28e)
- **web:** Localize the UI kit, switch to Onest and JetBrains Mono, add screen mocks (#34) (38871de)
- **adapters:** Add Valkey L2 cache and singleflight coordinator with moka L1 (#35) (e8c307f)
- **web:** Add P0 domain blocks, step-up dialog and install block with generated API types (#40) (09d74e2)
- **formats:** Add npm hosted and proxy with dist-tags, deprecate and unpublish policy (#43) (277f30d)
- **app:** Namespaces and dependency-confusion protection: hosted beats proxy (#52) (270d505)


### Документация
- **security:** Record security-first decisions and design hybrid vector search (#24) (ed3019a)
- **roadmap:** Move load testing and pentest post-v1, support backup and encryption via configuration (#26) (4573798)
- **security:** Close design-level attack surfaces in the spec and place the work in milestones (#42) (93e7875)
- **identity:** Design teams, download trail by team and generic OIDC (#44) (444ab4f)


### Исправления
- **api:** Drop allowReserved from the package name path parameter (#25) (15b336e)
- **entry:** Accept only non-secret keys in the CRANE_BLOBS query (#27) (4dea041)
- **api:** Name shared enums, type action rels and justifications (#28) (6bb7151)
- **web:** Let story tests load addons from symlinked node_modules (#32) (d88ac96)
- **proxy:** Bound proxied archive size before staging into CAS (#36) (3db8e27)
- **deploy:** Make S3 mode work with secret key files and a compose override (#45) (2c9db91)
- **storage:** Answer 503 with Retry-After when the catalog or blob store is unreachable (#49) (ab4c3bc)
- **deploy:** Drop client X-Request-Id at the Caddy edge (#50) (84a0bb3)
- **web:** Follow HAL links and server install clients in domain blocks (#51) (fbe5d79)
- **proxy:** Stop serving stale documents on upstream takedown and bound it on throttling (#53) (35b0bcc)
- **cargo:** Reject dependencies on foreign registries at publish (#54) (e281862)
- **deploy:** Require Valkey authentication and isolate it in its own network (#55) (363638b)
- **upstream:** Ping idle HTTP/2 connections and cap pool idle time (#58) (532016d)
- **valkey:** Detect a silent Valkey, log availability changes and cap L1 age (#59) (62c2e9d)


### Рефакторинг
- Forbid big files and split config and large tests into small modules (#48) (4cf1659)


### Служебное
- **http:** Capture logs via one global subscriber routed by thread (#37) (d98f0c5)
- **deploy:** Pin images by digest and require vector and pg_trgm in crane migrate (#38) (9ea916e)
- **dogfood:** Publish CRANE to its own registry and build it through the proxy (#39) (cd122e2)
- **dogfood:** Run the stand on Valkey and restart crane before going offline (#41) (b093f3c)
- **dogfood:** Run the stand on S3 and count bucket objects across the restart (#46) (1fdbcd0)
- Run only one cheap job on PRs to release branches while the repository is private (#47) (975a10b)
- **server:** Check the log filter without the global callsite cache (#56) (e4f15c8)
- **dogfood:** Exercise Valkey, PostgreSQL and S3 outages on the stand (#57) (30bed97)
- **web:** Wait for overlays to close before axe runs in stories (#60) (beb587c)
- **gate:** Read .env only in the mutation step so the dogfood stand keeps its own image (#61) (3d049f6)
- **mutants:** Cover cargo and npm policy windows, exclude equivalent mutants (#62) (9814520)
- **mutants:** Pin memory cache budget, egress host, proxy repo detection and artifact cap (#63) (61110a9)
- **valkey:** Pin codec capacity, wait patience, fast failure and availability log changes (#64) (9659c8e)
- **gate:** Run mutations in parallel via JOBS and keep test PostgreSQL off fsync (#65) (8998636)
- **mutants:** Pin proxy mounting, L1 weight, task abort on drop and valkey timeouts (#66) (adc6917)
- **web:** Mutate only logic modules with Stryker, cover time series helpers (#67) (7fb831f)

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

