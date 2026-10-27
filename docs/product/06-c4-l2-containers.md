# 6. C4 · Уровень 2: контейнеры

Один исполняемый образ `crane` запускается в разных ролях. Состояние живёт только в PostgreSQL (истина), S3 (байты) и Valkey (эфемерный кэш и координация).

```mermaid
flowchart LR
  classDef person fill:#6a135c,stroke:#6a135c,color:#fff
  classDef app fill:#951b81,stroke:#6a135c,color:#fff
  classDef store fill:#f9f1f7,stroke:#951b81,color:#29303e
  classDef ext fill:#f2f3f3,stroke:#80858e,color:#29303e
  users["Пользователи<br>и package-клиенты"]:::person
  ci["GitLab CI jobs"]:::ext
  subgraph SR["CRANE"]
    direction LR
    gw["<b>Edge proxy</b><br>Caddy или Envoy Gateway<br>TLS, HTTP/2, лимиты тела"]:::ext
    api["<b>API server</b><br>Rust, роль serve<br>package-протоколы, REST /api/v1,<br>A2A-агент, SPA, авторизация, горячий путь"]:::app
    wrk["<b>Worker</b><br>Rust, роль worker<br>сканирование, индексация, GC,<br>webhooks, rescan, прогрев"]:::app
    sbx["<b>Analyzer sandbox</b><br>изолированный под/контейнер<br>распаковка и разбор архивов"]:::app
    pg[("<b>PostgreSQL</b><br>metadata, политики, аудит,<br>outbox, очередь задач,<br>FTS и pgvector")]:::store
    vk[("<b>Valkey</b><br>кэш metadata, singleflight,<br>rate limit, pub/sub инвалидаций")]:::store
    s3[("<b>S3 хранилище</b><br>CAS blobs, staging,<br>evidence, бэкапы")]:::store
  end
  trivy["Trivy server"]:::ext
  kc["Keycloak"]:::ext
  ai["AI-контур<br>systemone, LLM"]:::ext
  feeds["Фиды уязвимостей"]:::ext
  siem["SIEM / SOAR"]:::ext
  agents["Внешние агенты"]:::ext
  up["Upstream-реестры"]:::ext
  users --> gw
  ci --> gw
  agents -->|"A2A"| gw
  api --> ai
  gw --> api
  api <--> pg
  api <--> vk
  api <--> s3
  api --> up
  api --> kc
  wrk <--> pg
  wrk <--> vk
  wrk <--> s3
  wrk --> sbx
  sbx --> trivy
  wrk --> ai
  wrk --> feeds
  wrk --> siem
  wrk --> up
```

*Рис. 6.1. Containers. API server и Worker собираются из одного бинарника.*

| Контейнер | Технология | Ответственность | Масштабирование |
| --- | --- | --- | --- |
| Edge proxy | Caddy (single node), Envoy Gateway / любой Gateway API (k8s) | TLS, HTTP/2, ограничение размера тела, ранний отказ, доступ-логи. | Горизонтально |
| API server | Rust: tokio, axum, hyper, rustls | Протоколы пяти форматов, REST API, A2A-агент (`/a2a`, Agent Card), вызовы LLM для интерактивных ответов, раздача SPA, аутентификация и авторизация, кэш, proxy fetch, приём публикаций. | Stateless, HPA по RPS и CPU |
| Worker | тот же бинарник | Фоновые задачи из очереди в PostgreSQL: синхронизация фидов уязвимостей, сопоставление и скоринг systemone, экспорт в SIEM, сканы, индекс, эмбеддинги, GC, retention, webhooks, rescan, прогрев кэша, импорт из Nexus. | Горизонтально, по длине очереди |
| Analyzer sandbox | тот же бинарник в роли `analyze`, distroless, seccomp, read-only FS, без сети кроме Trivy | Распаковка недоверенных архивов с лимитами, извлечение metadata и license-файлов, генерация SBOM. | Пул подов, по очереди |
| PostgreSQL | PostgreSQL 17+, pgvector, pg\_trgm | Единственный источник истины. | Primary + синхронные реплики, PgBouncer |
| Valkey | Valkey 8+ | Кэш, распределённые блокировки singleflight, GCRA rate limit, pub/sub. | Sentinel или Cluster |
| S3 | SeaweedFS / RustFS / Ceph RGW / облачный S3, либо локальная ФС | Неизменяемые blobs по SHA-256, staging загрузок, evidence, бэкапы. По умолчанию одно хранилище; классы `primary`, `proxy-cache`, `archive`, `restricted` могут указывать на разные (раздел 9). | По возможностям хранилища |

> **Почему не микросервисы.** Домен реестра компактный, а горячий путь (авторизация → кэш → blob) выигрывает от отсутствия сетевых переходов. Роли процесса дают то же независимое масштабирование, что и сервисы, без распределённых транзакций и десятка деплойментов.
