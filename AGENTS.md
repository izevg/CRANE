# AGENTS.md

Инструкции для ИИ-агентов и людей, работающих в репозитории CRANE. Продуктовая архитектура — `docs/product/` (источник истины); здесь только то, что нужно для работы с кодом и репозиторием.

## Запреты (действуют всегда)

- **Никакой атрибуции ИИ-агентов.** Ни в коммитах, ни в описаниях PR, ни в коде и документах: без `Co-Authored-By: <агент>`, `Generated with …`, `🤖`. Это правило продукта; его проверяют `commitlint` (локально и в CI).
- Никаких секретов в коде, тестовых данных и журналах. Секреты оборачиваются в тип без `Debug`/`Display`.
- Не коммитьте в `main` и `release/*` напрямую, не пушьте с `--force` в защищённые ветки, не обходите хуки (`--no-verify`).

## Стадия проекта

Проектирование + каркас. Код начинается с `crates/core/domain`. Команды в README описывают целевой интерфейс.

## Дорожная карта

До полного релиза всех фич (завершён объём P0–P3 из [ROADMAP.md](ROADMAP.md), последняя веха — миграция из Nexus) агенты **обязаны следовать вехам** `ROADMAP.md`:

- Работа берётся только из текущей вехи (текущая — ближайшая версия без пометки «готово»). Задачи следующих версий не начинаются, даже если они кажутся простыми или «по пути».
- Порядок и состав вех, критерии готовности и даты не меняются по инициативе агента. Если задача не вписывается в веху или веха упирается в блокер, остановитесь и спросите владельца.
- Изменение плана оформляется отдельным PR в `ROADMAP.md` (с пересчётом `python3 tools/estimate-agent-hours.py --start …`) и вступает в силу после одобрения владельца.
- PR ссылается на веху (версию `release/vX.Y.Z`), в которую он идёт; PR в другую релизную ветку без согласования не открывается.

## Рабочие деревья (worktrees)

Вся работа ведётся в отдельных git worktree, а не в основном рабочем каталоге:

- Одна feature-ветка — один worktree. Основной каталог остаётся на `main`: в нём не коммитят и не переключают ветки.
- Расположение и способ создания определяются настройками вашей IDE или агентского окружения, пути в репозитории не зашиваются (Claude Code — `.claude/worktrees/`, остальные — свои каталоги; корни worktree в `.gitignore`).
- После создания worktree выполните `task install` (`node_modules` и хуки lefthook у каждого worktree свои).
- Не используйте общий `git stash` (стек общий для всех worktree); откладывайте работу WIP-коммитом.
- После слияния ветки удалите worktree и ветку: `git worktree remove <путь>`.

## Команды

Единая точка входа — [Taskfile](Taskfile.yml) (`task --list`).

| Задача | Команда |
| --- | --- |
| Зависимости и хуки | `task install` (Node ≥ 24, см. `.nvmrc`) |
| Все быстрые проверки | `task check` |
| Rust: fmt, clippy, test, deny | `task rust:check` |
| Web: типы, Biome, Steiger, knip, depcruise | `task web:check` · юниты: `task web:test` |
| Файловая структура и слои крейтов | `task structure` |
| Полный гейт перед PR в `release/*` | `task gate` (юниты + мутации cargo-mutants и Stryker + Playwright) |
| Бамп версий, CHANGELOG, гайдбук | `task bump TAG=vX.Y.Z` · `task changelog` · `task guidebook` |

## Архитектура кода

- **Rust: DDD + hexagonal + clean architecture.** Зависимости направлены внутрь:
  `entry → adapters | formats | access | app → ports → domain`.
  `domain` без IO и без зависимостей от проекта; `ports` — трейты (порты) поверх `domain`; `app` — use cases; `adapters` и `formats` реализуют порты и не знают друг о друге; `entry` (http, worker, analyzer, server) — единственное место сборки зависимостей. Матрица разрешённых зависимостей — `tools/lint-structure/rules.json`, проверяется на `cargo metadata`.
- **Web: Feature-Sliced Design.** Слои `app → pages → widgets → features → entities → shared`, импорт только вниз, без перекрёстных слайсов, доступ к слайсу только через `index.ts`. Проверяют Steiger и dependency-cruiser.

## Правило четырёх (файловая структура)

Действует для `crates/`, `web/src/`, `cli/`. Проверяет `tools/lint-structure`, в хуке pre-commit и в CI.

1. Не больше **4 файлов** в папке.
2. Не больше **8 подпапок** в папке.
3. **Смешанные папки запрещены**: в папке либо файлы, либо подпапки.

Не считаются: манифесты и конфиги (`Cargo.toml`, `package.json`, `tsconfig*.json`, …), файлы тестов (`*.test.*`, `*.spec.*`), каталоги `tests/`, `migrations/`, `generated/`, dotfiles. Файлы-входы (`lib.rs`, `main.rs`, `mod.rs`, `index.ts`) не создают «смешанность», но входят в лимит в 4 файла. Следствие для Rust: модуль с подмодулями — это `foo/mod.rs` + `foo/…`, а не `foo.rs` рядом с `foo/`. Превысили лимит — выделяйте подпапку по смыслу, не прячьте файлы в `misc/`.

## Стиль кода

- **Rust:** «скучный Rust» (раздел 3): без процедурных макросов и магии типов. Линты — `[workspace.lints]` в `Cargo.toml` (`unsafe_code = forbid`, запрет `unwrap`, `panic`, `todo`, `dbg!`, `println!`); `rustfmt.toml`, `clippy.toml`. Ошибки: `thiserror` в `domain` и `app`, `problem+json` на границе HTTP.
- **TypeScript везде, где можно:** скрипты, конфиги и tools пишутся на `.ts` (Node 24 запускает их напрямую, только erasable-синтаксис: без `enum` и `namespace`). Файлы `.mjs` в репозитории запрещены; `.js`/`.cjs` только там, где инструмент не поддерживает TS.
- **TypeScript:** strict, Biome (формат и линт), без `any`, без `console`, без `!`. Алиас `~/` для `src/`.
- Зависимости: только пермиссивные лицензии (`cargo deny`), новая зависимость объясняется в PR. `web`: `knip` ловит неиспользуемое.
- Комментарии объясняют «почему», не «что». Язык комментариев и документации — русский.

## Тестирование

- Юниты: Rust — `#[cfg(test)]` рядом с кодом и `tests/` для интеграционных; web — Vitest + Testing Library. Структура AAA, тест проверяет поведение, не реализацию.
- Мутационное тестирование: `cargo-mutants` и Stryker. Критичный путь (протоколы форматов, доступ, CAS и публикация, токены) обязан держать порог. Полный гейт запускается **локально** (`task gate`) перед PR в `release/*`; в CI он не гоняется. Playwright и Storybook-тесты тоже локально.

## Git flow

- `main` — основная ветка (production). От неё: `release/vX.Y.Z` и feature-ветки.
- Feature-ветка от `main` → PR в `release/vX.Y.Z`, **squash**-мерж. Заголовок PR — Conventional Commits (он становится сообщением сквоша). В описание PR вставляется вывод `task gate` (`Gate: passed`, `Gate-SHA`).
- Владелец одобряет релиз → PR `release/vX.Y.Z` → `main`, **merge commit** (не squash: история коммитов нужна для CHANGELOG).
- После мержа в `main` workflow `release` бампит версии (`Cargo.toml`, `package.json`, `Chart.yaml`), пересобирает `CHANGELOG.md` из коммитов (окно «предыдущий тег..HEAD», `git-cliff`), делает коммит `chore(release): vX.Y.Z` и ставит тег `vX.Y.Z`. Тег запускает workflow `publish`: GitHub Release с заметками из CHANGELOG. Образы в ghcr пока не собираются.
- Типы коммитов: `feat fix perf refactor docs test build ci chore revert style`; scope — kebab-case, заголовок ≤ 100 символов.

## Документация

- `docs/product/` ведётся на русском; корневые файлы — русский + зеркало `-EN`. После правок собирайте гайдбук (`tools/build-guidebook.sh`, режим `--strict`).
- Добавили или убрали функциональность — обновите `tools/estimate-agent-hours.py` и цифры в разделах 1 и 18.
