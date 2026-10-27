# 17. OSS-стек и лицензии

Требование: только OSS с пермиссивными лицензиями, допускающими коммерческое использование. Copyleft (GPL, AGPL, LGPL) и source-available (BUSL, SSPL, RSAL, Elastic License) исключены из поставки.

## Рекомендованный стек

| Компонент | Лицензия | Роль и комментарий |
| --- | --- | --- |
| Rust toolchain, tokio, axum, hyper, tower | MIT / Apache-2.0 | Рантайм и HTTP |
| rustls | Apache-2.0 / MIT / ISC | TLS без OpenSSL |
| sqlx | MIT / Apache-2.0 | PostgreSQL; запросы без макросов, во время выполнения («скучный Rust») |
| object\_store (Apache Arrow) | Apache-2.0 | Абстракция S3, GCS, Azure, локальной ФС |
| sha2, uuid | MIT / Apache-2.0 | SHA-256 для content-addressable хранилища, идентификаторы UUIDv7 |
| cedar-policy | Apache-2.0 | Политики доступа с объяснением решений |
| moka, fred (Valkey-клиент) | MIT / Apache-2.0 | Кэш в процессе и клиент Valkey |
| utoipa, clap, serde, quick-xml, semver | MIT / Apache-2.0 | OpenAPI, CLI, сериализация, XML для Maven и NuGet, разбор `.crate` и `Cargo.toml` (toml, tar, flate2) |
| pep440\_rs, spdx, askalono | Apache-2.0 / MIT | Версии PyPI, SPDX expressions, определение лицензий по тексту |
| opentelemetry-rust, tracing | Apache-2.0 / MIT | Наблюдаемость |
| PostgreSQL | PostgreSQL License | Источник истины |
| pgvector, PgBouncer | PostgreSQL / ISC | Векторный поиск, пул соединений |
| Valkey | BSD-3-Clause | Кэш и координация |
| SeaweedFS | Apache-2.0 | S3 в комплекте, зрелый проект |
| RustFS | Apache-2.0 | S3 в комплекте, GA с сентября 2026, молодой проект: проверять в PoC |
| Percona Operator for PostgreSQL | Apache-2.0 | HA PostgreSQL в k8s: Patroni (MIT) + pgBackRest (MIT) |
| CloudNativePG | Apache-2.0 | Альтернатива; штатный backup-плагин использует Barman (GPL-3), поэтому бэкап через VolumeSnapshot или pgBackRest-плагин (пока экспериментальный) |
| Caddy | Apache-2.0 | Edge proxy в single node |
| Envoy Gateway, Traefik, Cilium | Apache-2.0 / MIT | Gateway API в k8s (ingress-nginx снят с поддержки в марте 2026) |
| Trivy, Syft | Apache-2.0 | Сканирование и SBOM (часто уже развёрнут) |
| VictoriaMetrics, VictoriaLogs, Jaeger, Perses | Apache-2.0 | Метрики, логи, трейсы, дашборды |
| KEDA, External Secrets Operator | Apache-2.0 | Автомасштабирование по очереди, секреты |
| oha, Gatling | MIT / Apache-2.0 | Нагрузочное тестирование |
| React, Vite, TanStack, Base UI, Tailwind | MIT | Frontend |
| Keycloak | Apache-2.0 | IdP по умолчанию; любой OIDC-провайдер подключается так же |

## Инструменты разработки

В поставку не входят. Лицензии указаны по данным проектов, проверяются при добавлении.

| Инструмент | Лицензия | Роль |
| --- | --- | --- |
| cargo-deny, cargo-mutants | MIT / Apache-2.0 | Лицензии и advisories зависимостей; мутационное тестирование Rust |
| Biome, Steiger, dependency-cruiser, knip | MIT / ISC | Формат и линт, структура FSD, направление импортов, неиспользуемый код |
| Vitest, Stryker | MIT / Apache-2.0 | Юнит-тесты и мутационное тестирование frontend |
| commitlint, lefthook, git-cliff | MIT / Apache-2.0 | Conventional Commits, git hooks, CHANGELOG |
| pnpm | MIT | Менеджер пакетов frontend и инструментов |

## Исключено из поставки

| Компонент | Лицензия | Замена |
| --- | --- | --- |
| **MinIO** | AGPL-3.0 | SeaweedFS или RustFS. Кроме лицензии: в 2025 убраны бинарники и консоль из community edition, в феврале 2026 репозиторий архивирован. Если у организации уже есть MinIO как отдельный сервис, CRANE работает с ним по S3 API без изменения кода, но в поставку он не входит. |
| Grafana, Loki, Tempo, Mimir | AGPL-3.0 | Perses, VictoriaLogs, Jaeger, VictoriaMetrics |
| k6 | AGPL-3.0 | oha, Gatling |
| Redis 7.4+ | RSAL / SSPL / AGPL | Valkey |
| Elasticsearch | SSPL / ELv2 / AGPL | PostgreSQL FTS + pgvector; при необходимости OpenSearch (Apache-2.0) |
| Meilisearch Enterprise | BUSL-1.1 | PostgreSQL FTS |
| HashiCorp Vault | BUSL-1.1 | External Secrets Operator + существующее хранилище секретов |
| Barman | GPL-3.0 | pgBackRest |
| Образы Bitnami | условия распространения изменены в 2025 | Официальные образы проектов |

> **Контроль в CI.** `cargo-deny` с allowlist лицензий (MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode-3.0, MPL-2.0 только по согласованию) и аналогичная проверка npm-зависимостей frontend ломают сборку при появлении новой copyleft-зависимости. Тот же движок лицензий, что в продукте, проверяет сам продукт.
