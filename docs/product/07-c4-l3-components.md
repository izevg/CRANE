# 7. C4 · Уровень 3: компоненты

Внутри бинарника модули разбиты по bounded context. Слои показывают разрешённые зависимости: адаптеры зависят от портов ядра, но не наоборот.

| Слой | Элементы |
|---|---|
| Входные адаптеры | **Maven** (protocol), **npm** (protocol), **PyPI** (protocol), **NuGet** (protocol), **Cargo** (sparse protocol), **REST /api/v1** (OpenAPI), **A2A agent** (`/a2a`, Agent Card), **SPA + BFF** (UI kit) |
| Доменное ядро (use cases) | **Identity** (OIDC, роботы, GitLab exchange), **Access** (Cedar, кэш, trace), **Catalog** (repos, namespaces), **Publishing** (upload, commit), **Proxy** (TTL, SWR, coalescing), **Evidence** (SBOM, findings), **Vulnerability intel** (фиды, сопоставление, приоритет, VEX), **Risk scoring** (systemone), **License** (SPDX, категории), **Promotion** (gates, approvals), **Usage graph** (impact), **Search** (FTS + vector), **Storage mgmt** (квоты, GC), **Audit** (hash chain), **Events** (outbox, webhooks), **AI assist** (LLM, RAG с ACL), **Security events** (OCSF, SIEM, SOAR), **Diagnostics** (problem+json) |
| Порты (трейты) | `BlobStore`, `Repositories`, `Cache`, `JobQueue`, `Scanner`, `AdvisoryFeed`, `RiskScorer`, `AiProvider`, `SiemSink`, `UpstreamClient`, `IdentityProvider`, `VcsIntegration`, `Notifier` |
| Выходные адаптеры (вкомпилированы) | **S3 / FS** (object_store), **PostgreSQL** (sqlx), **Valkey + moka** (2 уровня), **PG queue** (SKIP LOCKED), **Trivy** (server API), **systemone** (скоринг риска), **LLM, embedder, rerank** (OpenAI-совм.), **Фиды** (OSV, NVD, KEV, EPSS, CSAF), **SIEM** (syslog TLS, HTTP, OTLP; OCSF), **HTTP upstream** (SSRF guard), **Keycloak** (OIDC/JWKS), **GitLab** (REST, OIDC), **SMTP, webhooks** (уведомления) |

*Рис. 7.1. Components. Зависимости направлены внутрь: адаптеры знают о портах, ядро не знает об адаптерах.*

## Ответственность компонентов

| Компонент | Ответственность | Ключевой инвариант |
| --- | --- | --- |
| Format protocols | Разбор URL и тел запросов каждого формата в канонические команды ядра; рендер metadata (packument, Simple API, maven-metadata.xml, NuGet V3 resources, Cargo sparse index и `config.json`); переписывание ссылок на адрес CRANE. | Никакой бизнес-логики, только трансляция протокола |
| Identity | Проверка OIDC JWT от Keycloak, выпуск личных токенов и токенов роботов, обмен OIDC-токенов платформ (GitLab, Kubernetes, сервисные аккаунты) на краткоживущий credential, эпохи и отзыв. | Секрет показывается один раз, хранится только HMAC |
| Access | Вычисление решения по политикам Cedar с атрибутами principal, ресурса и контекста; кэш решений; trace сработавших правил. | Default deny; каждое решение объяснимо |
| Catalog | Workspaces, repositories, namespaces, packages, владельцы, резервирование имён. | Внутреннее имя никогда не резолвится в публичный upstream |
| Publishing | Приём загрузок, потоковое хэширование, атомарная фиксация версии, state machine версии. | Незавершённая загрузка невидима; release неизменяем |
| Proxy | Pull-through кэш upstream: TTL, negative cache, stale-while-revalidate, stale-if-error, singleflight. | Один upstream-запрос на объект при любой конкуренции |
| Vulnerability intel | Синхронизация фидов, нормализация и дедупликация advisory, сопоставление с инвентарём по PURL, приоритет P0–P4, VEX, экстренный режим, SLA. | Модель только повышает приоритет; P0 не зависит от доступности модели |
| Risk scoring | Сбор признаков, запрос оценок `exploit_relevance`, `blast_radius`, `license_virality` у systemone, хранение оценки как evidence с версией модели. | Оценка воспроизводима: модель, версия и хэш признаков сохраняются |
| Security events | Формирование событий OCSF из аудита и доменных событий, доставка в SIEM с курсором на приёмник, приём ответных действий SOAR. | At-least-once, событие не теряется при недоступности SIEM |
| Evidence | Единая модель доказательств по `predicate_type`: SBOM, сканы, attestations, подписи, quality gate; проверка Sigstore/PGP и trust root; findings; исключения с TTL. | Отчёт привязан к digest, версии сканера и базы |
| License | Разбор SPDX expressions, определение лицензии из нескольких источников, оценка по политике с учётом контекста. | Unknown означает review required |
| Promotion | Publish → Quarantine → Checks → Approval → Release, повторная оценка при новых рисках. | Promotion не меняет байты |
| Usage graph | Рёбра зависимостей из SBOM и lockfile, evidence использования с уровнем достоверности, impact analysis. | Уровни достоверности не смешиваются |
| Search | Лексический и семантический поиск, гибридное ранжирование, ACL-фильтр до выдачи. | Пользователь не видит то, что не может читать |
| Storage mgmt | Классы хранения и размещение blob, онлайн-перенос между хранилищами, квоты, retention с dry run и архивированием, holds, двухфазный GC, проверка целостности. | Blob удаляется только после grace period без ссылок |
| A2A agent | Приём задач A2A, выбор навыка, формирование плана записи, ожидание подтверждения, вызов use cases от имени principal. | Агент не имеет прав шире principal; запись только после подтверждения плана |
| Audit | Неизменяемый журнал с hash chain, экспорт в SIEM. | Изменение записи обнаруживается |
| Events | Transactional outbox для внешней доставки (P2), доменные события, webhooks с подписью и повторами. Внутренние задачи идут через `job`, не через outbox. | Событие публикуется тогда и только тогда, когда закоммичена транзакция |

## Компоненты Worker

Worker использует те же модули ядра, но запускает обработчики задач. Каждый тип задачи имеет свой лимит параллелизма, таймаут и политику повторов.

| Задача | Триггер | Параллелизм по умолчанию | Повторы |
| --- | --- | --- | --- |
| `analyze.package` | Новая версия (строка `job` в транзакции публикации) | 2 на под sandbox | 3, экспоненциально |
| `scan.sbom` | Новый SBOM, новая версия базы Trivy | 8 | 5 |
| `advisory.sync` | Cron каждые 5–15 мин на фид | 1 на фид | следующий запуск, алерт при отставании |
| `advisory.match` | Новое или изменённое advisory | 16 | 5 |
| `score.risk` | Новый finding, обновление EPSS/KEV, изменение графа, новая версия модели | 8 | 5, затем приоритет по правилам |
| `siem.export` | Outbox, курсор на приёмник | 1 на приёмник | до успеха, алерт при отставании > 5 мин |
| `index.package` / `embed.package` | Изменение metadata или README | 16 / 4 | 5 |
| `promotion.evaluate` | Новое evidence, решение approval, истечение exception | 16 | до успеха |
| `webhook.deliver` | Outbox | 32 | до 24 ч, затем DLQ |
| `gc.mark` / `gc.sweep` | Cron, по умолчанию раз в сутки | 1 (лидер) | следующий запуск |
| `cache.warm` | Lockfile или SBOM от пользователя | 8 | 3 |
| `storage.move` | Смена класса репозитория, архивирование по retention, вывод хранилища из эксплуатации | 4 | 5, перенос идемпотентен |
| `integrity.verify` | Cron, выборочная проверка 1% blobs в сутки | 2 | следующий запуск |
| `import.nexus` | Ручной запуск миграции | 8 | 5, с журналом |
