# Критичные пути для секьюрити-ревью

Список путей, изменения в которых проходят ревью роли `sec-lead` (белошляпный аудитор, определение в `~/.claude/agents/sec-lead.md`). Лид вызывает её **после зелёных гейтов** своего PR; если дифф касается хотя бы одного пути из списка, ревью обязательно. Пустой список изменений в этих путях означает, что секьюрити-ревью не нужно. Если путь переименован или перенесён, список обновляется в том же PR.

Правило: PR без пометки `sec-review: CLEAR | CONDITIONAL | n/a` в описании, затронувший критичный путь, не вливается.

## Аутентификация, права, журнал

- `crates/entry/http/src/auth/**`, `crates/entry/http/src/console/**` (слой аутентификации консоли)
- `crates/core/domain/src/access/**`, `crates/core/domain/src/audit/**`
- `crates/adapters/external/siem/**` (журнал ИБ)
- `crates/entry/server/src/config/token/**`, `crates/entry/server/src/config/source/**`
- `crates/core/domain/src/settings/**`, `crates/core/app/src/settings/**` (настройки, онбординг, bootstrap-токен)

## Секреты и криптография

- `crates/adapters/persistence/pg/src/settings/crypto/**`
- Любой код, создающий, хранящий, сравнивающий или логирующий секрет (`Secret`, `TokenHash`)

## Недоверенные входы: парсеры, пути, форматы

- `crates/formats/*/src/**` (грамматики имён и версий, разбор индексов и документов, загрузка)
- `crates/entry/http/src/formats/**` (маршруты и схемы токена по форматам)
- `crates/core/domain/src/registry/**` (координаты, namespace, жизненный цикл, дайджесты)
- `crates/core/domain/src/proxy/**` (`UpstreamPath`, политика egress, SSRF-guard)
- `crates/wire/**` (разбор внешних документов: фиды, сканер, SBOM)

## Proxy, кэш, целостность

- `crates/core/app/src/proxy/**`, `crates/adapters/upstream/**`
- `crates/adapters/persistence/valkey/**` (кодек, ключи, инвалидация)
- Ключи кэша (`ProxyKey`), политика stale и negative cache

## Хранилище и публикация

- `crates/core/app/src/registry/**` (publish, attach, yank, delete, ретенция)
- `crates/adapters/persistence/pg/**` (SQL, миграции `migrations/**`, CAS)
- `crates/adapters/persistence/*/blob*`, хранилище blob

## Анализ недоверенных артефактов, SecOps

- `crates/entry/analyzer/**` и любая распаковка архивов
- `crates/core/*/src/supply_chain/**`

## Веб-консоль

- `web/src/shared/api/**` (транспорт, заглушки, токены), `web/src/app/**`
- Любое место с `dangerouslySetInnerHTML`, хранением токена, рендером данных пакета
- Флаги демо-режима и сборки заглушек

## Инструменты, сборка, поставка (место закладок)

- `lefthook.yml`, `.github/workflows/**`, `Taskfile.yml`, `commitlint.config.ts`
- `tools/**` (линтеры структуры, FIXME, AAA)
- `biome.json`, `clippy.toml`, `rustfmt.toml`, `deny.toml`, `.cargo/**` (в том числе исключения мутаций)
- `Cargo.toml`, `Cargo.lock`, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, любые `build.rs` и процедурные макросы
- `deploy/**` (compose, Dockerfile, Helm), `.env.dist`
- Окружение агентов: `.claude/**`, `orca.yaml`, `.mcp.json`, `.worktreeinclude`

## Всегда на ревью, независимо от пути

- Любое добавление зависимости, `unsafe`, `build.rs`, сетевого вызова или чтения окружения вне конфигурации.
- Любое ослабление проверки: `allow`/`expect`, исключение мутации, понижение порога, `--no-verify`, пропуск теста.
- Любой «временный» маршрут, флаг, заглушка или `FIXME`, достижимые из production.
