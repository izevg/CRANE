# 16. Развёртывание и наблюдаемость

Оба способа поддерживаются как равноправные. Используются один и тот же образ, одна и та же конфигурация (файл TOML и переменные окружения) и одни и те же миграции. Различаются только топология и число реплик.

## 16.1 Docker single node

```mermaid
flowchart LR
  classDef app fill:#951b81,stroke:#6a135c,color:#fff
  classDef store fill:#f9f1f7,stroke:#951b81,color:#29303e
  classDef ext fill:#f2f3f3,stroke:#80858e,color:#29303e
  cl["Клиенты, CI"]:::ext
  subgraph H["Хост: Docker Compose"]
    cd["caddy<br>TLS, ACME или собственный сертификат"]:::ext
    sr["crane<br>роль all: serve + worker"]:::app
    an["crane-analyzer<br>роль analyze, без сети"]:::app
    pg[("postgres 17<br>+ pgvector")]:::store
    vk[("valkey<br>опционально в профиле S")]:::store
    ob[("seaweedfs или rustfs<br>или том /data/blobs")]:::store
    tv["trivy server<br>опционально"]:::ext
    bk["backup<br>pgBackRest + rclone"]:::ext
  end
  cl --> cd --> sr
  sr --> pg
  sr --> vk
  sr --> ob
  sr --> an
  an --> tv
  bk --> pg
  bk --> ob
```

*Рис. 16.1. Single node. Минимальный вариант: caddy, crane, postgres и локальный том.*

```yaml
services:
  crane:
    image: crane/crane:1.0.0     # образ публикуется проектом; адрес реестра задаётся при релизе
    command: ["all"]
    environment:
      CRANE_DATABASE_URL: postgres://crane@postgres/crane
      CRANE_CACHE: valkey://valkey:6379        # или "memory" для профиля S; URL с паролем — через CRANE_CACHE_FILE
      CRANE_BLOBS: s3://blobs?endpoint=http://seaweedfs:8333&allow_http=true   # или file:///data/blobs
    read_only: true
    deploy: { resources: { limits: { cpus: "2", memory: 1g } } }
  analyzer:
    image: crane/crane:1.0.0
    command: ["analyze"]
    networks: [scan]                            # изолированная сеть только с trivy
    read_only: true
    security_opt: ["no-new-privileges:true"]
  # точный тег и digest образов — в `.env.dist`
  postgres: { image: "${POSTGRES_IMAGE}", volumes: [pg:/var/lib/postgresql/data] }
  valkey:   { image: "${VALKEY_IMAGE}", command: ["--save", "", "--appendonly", "no", "--maxmemory", "${VALKEY_MAXMEMORY}", "--maxmemory-policy", "allkeys-lru", "--aclfile", "<ACL из пароля>"] }
```

Ключи артефактов в Valkey живут без TTL, поэтому ему нужны `maxmemory` и `maxmemory-policy allkeys-lru`: без них кэш растёт, пока не упрётся в память хоста. Предел задаёт переменная `VALKEY_MAXMEMORY` (по умолчанию `512mb`), политика `allkeys-lru` зафиксирована в compose-файлах. Для внешнего Valkey обе настройки нужно задать самостоятельно.

**Valkey: изоляция и аутентификация.** Кэш хранит тела документов proxy (metadata upstream, которую CRANE отдаёт клиентам) и указатели на blob в CAS (дайджесты), поэтому это не безобидные данные. Valkey обязан быть сетево изолирован (доступен только репликам CRANE) и требовать AUTH; в Kubernetes для этого служат NetworkPolicy и пароль из Secret. TLS (схема `valkeys`) нужен, когда трафик выходит за пределы узла или доверенной сети. Эталонный compose включает Valkey override-файлом `docker-compose.cache.yaml` (по умолчанию single-node работает с `CRANE_CACHE=memory` и секретов Valkey не требует): `COMPOSE_FILE=docker-compose.yaml:docker-compose.cache.yaml docker compose up -d` либо `-f docker-compose.yaml -f docker-compose.cache.yaml`. Valkey стартует с `--aclfile`, который скрипт запуска собирает из пароля (секрет `valkey_password`) во временный файл с правами 0600, поэтому политика живёт в compose и обновляется вместе с ним: пользователь `default` выключен, пользователь `crane` не имеет команд группы `@dangerous` (`KEYS`, `CONFIG`, `SHUTDOWN` и т.п.), а сеть `cache` объявлена `internal: true` и содержит только crane и valkey.

**Остаточный риск.** Тот, кто может писать в Valkey, может подменить документы proxy и указатели на blob. Для артефакта с ожидаемым sha256 из индекса указатель на чужой blob отвергается (`Target::freshness`), но при одновременной подмене индекса эта защита не работает; когда ожидаемого дайджеста нет, проверки тоже нет. Подпись записей кэша (HMAC) остаётся открытым решением.

**Требования к PostgreSQL.** PostgreSQL 17+, pgvector ≥ 0.8 (итеративные сканы индекса HNSW, раздел 10.10), pg_trgm. Образы в `.env.dist` и Dockerfile закреплены тегом и digest мульти-арх индекса (`тег@sha256:…`; тег для человека, образ выбирает digest); в Helm digest образа CRANE задаётся значением `image.digest`. Перед миграциями `crane migrate` гарантирует расширения `vector` (≥ 0.8) и `pg_trgm`: создаёт недостающие, если хватает прав, и применяет миграции только после этого. Если расширения нет и прав на его создание нет, команда сразу завершается с сообщением, где названо расширение и готов SQL для администратора; миграции при этом не применяются. Если `vector` старше 0.8, сообщение подсказывает `ALTER EXTENSION vector UPDATE;`. В управляемом PostgreSQL `vector` не помечено доверенным и требует суперпользователя, `pg_trgm` доверенное и требует лишь права `CREATE` на базе, поэтому администратор заранее выполняет в базе каталога `CREATE EXTENSION vector; CREATE EXTENSION pg_trgm;`; с уже созданными расширениями `crane migrate` работает под ролью без этих прав.

**Конфигурация (v0.1.0).** Источники: переменные окружения и необязательный файл TOML, путь к которому задаёт только `CRANE_CONFIG`. Приоритет: переменная окружения, затем файл, затем значение по умолчанию. Ключ файла — имя переменной без префикса `CRANE_` в нижнем регистре (`CRANE_HTTP_PORT` — `http_port`); значение — строка или целое число, а для любого ключа допустим ещё и массив строк (склеивается запятыми). Неизвестный ключ — ошибка запуска. Значения `memory` пишутся явно.

| Переменная | По умолчанию | Назначение |
| --- | --- | --- |
| `CRANE_CONFIG` | нет файла | путь к файлу TOML; только из окружения |
| `CRANE_DATABASE_URL` или `CRANE_DATABASE_URL_FILE` | одна из двух | каталог: `memory` (данные теряются при рестарте) или `postgres://…` (`postgresql://…`); файл читается без завершающего перевода строки |
| `CRANE_CACHE` или `CRANE_CACHE_FILE` | `memory` | в compose не задаётся (без обеих переменных берётся `memory`), адрес Valkey задаёт override `deploy/compose/docker-compose.cache.yaml` через `CRANE_CACHE_FILE`. Кэш proxy и координатор выборок: `memory` (ограничен по байтам, в процессе) или Valkey; схемы `valkey`, `valkeys` (TLS), `valkey-cluster`, `valkeys-cluster`, `valkey-sentinel`, `valkeys-sentinel` (`redis://` не принимается). Необязательна, в отличие от хранилищ: кэш не источник истины, его потеря данных не теряет, поэтому правило «`memory` пишется явно» выше на неё не распространяется. Но содержимое кэша отдаётся клиентам, поэтому Valkey требует AUTH и сетевой изоляции (см. выше). URL может содержать пароль, поэтому есть `_FILE` (читается без завершающего перевода строки, как `CRANE_DATABASE_URL_FILE`) |
| `CRANE_BLOBS` | обязательна | хранилище blob: `memory`, `file:///абсолютный/путь` или `s3://bucket[/prefix]?endpoint=…` |
| `CRANE_SETTINGS_KEY` или `CRANE_SETTINGS_KEY_FILE` | необязательна | ключ шифрования секретов, сохранённых в консоли (AES-256-GCM): base64 от 32 байт (`openssl rand -base64 32`); файл читается без завершающего перевода строки. Без ключа секреты в БД не записываются, а ранее записанные не читаются: сервер при старте называет ключ настройки и отпечатки ключей (записи и текущего), значение не выводится. Только окружение или файл конфигурации, из консоли не меняется. При `CRANE_DATABASE_URL=memory` заданный ключ — ошибка старта |
| `AWS_ACCESS_KEY_ID_FILE` и `AWS_SECRET_ACCESS_KEY_FILE` | нет | статический ключ S3 из файлов (секреты Docker/Kubernetes), только окружение (в TOML их нет). Задаются парой: один файл без второго — ошибка запуска; переменная `AWS_ACCESS_KEY_ID` (`AWS_SECRET_ACCESS_KEY`) вместе со своим файлом — конфликт; при `memory` и `file://` ключ не нужен, и сервер отказывается стартовать. Без обоих файлов ключи, роли и метаданные берутся из обычных `AWS_*`. Файл читается без завершающего перевода строки |
| `CRANE_PUBLIC_URL` | обязательна | внешний адрес консоли и `/api` (`http://` или `https://`); в `config.json` cargo попадает, если `CRANE_PACKAGES_URL` не задан |
| `CRANE_PACKAGES_URL` | пусто | origin пакетов для протоколов (`/cargo`, `/npm`, `/pypi`, `/maven`, `/nuget`) и presigned-ссылок. Если задан, консоль на `CRANE_PUBLIC_URL` не отдаёт протоколы, а origin пакетов не отдаёт консоль и `/api`; `config.json` cargo и `cctl setup` берут адрес отсюда. Лучше отдельный регистрируемый домен; поддомен допустим: cookie сессии host-only (`__Host-`), мутации требуют CSRF-токена и Origin |
| `CRANE_TOKEN` или `CRANE_TOKEN_FILE` | одна из двух | временный статический токен до v0.4.0; файл читается без завершающего перевода строки |
| `CRANE_TOKEN_PRINCIPAL` | `crane` | имя личности токена (тип «робот»): попадает в журнал аудита, `access`-журнал и ответ cargo `owners` |
| `CRANE_TRUSTED_PROXIES` | пусто | IP-адреса и CIDR (IPv4, IPv6) через запятую, чьим `X-Forwarded-For` и `X-Request-Id` верить; compose обязан указать сеть Caddy, иначе во всех событиях будет адрес прокси. Каждый прокси из списка обязан перезаписывать или удалять присланный клиентом `X-Request-Id` и дописывать адрес своего пира в конец `X-Forwarded-For` (сервер разбирает его справа налево, левые записи клиента не влияют) |
| `CRANE_HTTP_PORT` | `8080` | порт API |
| `CRANE_ADMIN_PORT` | `9464` | порт проб `/healthz/live` и `/healthz/ready`; мастер первого запуска на нём не живёт, он обслуживается на публичном порту (20.2) |
| `CRANE_SETTINGS_KEY_FILE` | — | файл с ключом шифрования секретов настроек в БД: base64 от 32 байт, права 0400 (20.2). Без него секреты из консоли не хранятся |
| `CRANE_BOOTSTRAP_TOKEN`, `CRANE_BOOTSTRAP_TOKEN_FILE` | — | токен `crb_…` входа в мастер первого запуска (20.5). Без них и без `CRANE_TOKEN` сервер выпускает токен сам и печатает его в цель `crane::bootstrap` |
| `CRANE_CARGO_REPOS` | `internal` | hosted cargo-репозитории через запятую |
| `CRANE_CARGO_PROXY_REPOS` | пусто | proxy-репозитории crates.io через запятую (например `crates-io`): `/cargo/<имя>/`, индекс и архивы через кэш; имя не должно совпадать с hosted. Пустое значение выключает proxy |
| `CRANE_NPM_REPOS` | пусто | hosted npm-репозитории через запятую (например `web`): `/npm/<имя>/`; те же правила имён, что у cargo |
| `CRANE_NPM_PROXY_REPOS` | пусто | proxy-репозитории registry.npmjs.org через запятую (например `npmjs`): `/npm/<имя>/`, packument и tarball через кэш с проверкой sha512; имя не должно совпадать с hosted npm. Пустое значение выключает proxy npm; движок и кэш общие с cargo |
| `CRANE_NAMESPACES` | пусто | резервирование имён (10.3): записи `формат:шаблон` или `формат:шаблон=uuid-workspace` через запятую, владелец по умолчанию — workspace по умолчанию. Пример: `cargo:acme-*,cargo:acme-payments-*=<uuid>`. В TOML-файле ключ `namespaces` — массив строк. Формат — строчное имя (`cargo`, `maven`, `npm`, `nuget`, `pypi`), шаблон нормализуется правилами формата (10.3); неизвестный формат, некорректный шаблон, пустая запись и повтор шаблона — ошибка конфигурации. Применяется при старте идемпотентно; шаблон, уже принадлежащий другому workspace, обрывает старт (владение молча не переходит); каждая применённая запись аудируется (`Reserve`). Удаление записи из списка резервирование не снимает: резервирования только добавляются |
| `CRANE_MAX_PUBLISH_BYTES` | `10485760` | предел тела публикации в байтах (cargo и npm). npm шлёт tarball внутри JSON в base64, то есть на треть больше: пакет до 7,5 МиБ при значении по умолчанию |
| `CRANE_PROXY_MAX_ARTIFACT_BYTES` | `2147483648` | предел размера одного проксируемого архива в байтах (2 ГиБ); больше предела — 502, в CAS ничего не попадает. Предохранитель от сбежавшего потока, а не политика формата |

`memory` нельзя смешивать с постоянным хранилищем: пара `CRANE_DATABASE_URL` и `CRANE_BLOBS` либо вся в памяти, либо вся постоянная. Иначе каталог в PostgreSQL после рестарта ссылался бы на потерянные blob, и сервер отказывается стартовать. Адрес базы с паролем в журналы и тексты ошибок не попадает. При недоступной БД `crane serve` падает на старте, `crane migrate` применяет миграции и идемпотентен.

`/healthz/ready` проверяет PostgreSQL и хранилище blob и укладывается в 1,5 с: на отказ или таймаут он отвечает 503, а `/healthz/live` хранилищ не трогает. Учётные данные в query `CRANE_BLOBS` запрещены (только `AWS_*`; статический ключ из файлов читают `AWS_ACCESS_KEY_ID_FILE` и `AWS_SECRET_ACCESS_KEY_FILE`). Учётной записи S3 нужно право `s3:ListBucket`: без него отсутствующий ключ даёт 403 вместо 404, и готовность остаётся неуспешной.

`crane healthcheck` опрашивает `/healthz/ready` на `CRANE_ADMIN_PORT` и завершается с кодом 0 или 1 (для healthcheck в compose: в образе нет curl).

**S3-режим.** Вместо тома `blobs` CAS можно держать в S3 (в поставке SeaweedFS). Режим целиком описан override-файлом `deploy/compose/docker-compose.s3.yaml`, а не профилем: он меняет настройку самого crane. Включение (команды compose запускаются из корня репозитория с `--project-directory .`): строка `COMPOSE_FILE=deploy/compose/docker-compose.yaml:deploy/compose/docker-compose.s3.yaml` в `.env` (или `-f` в команде), `CRANE_BLOBS=s3://<S3_BUCKET>?endpoint=http://seaweedfs:8333&allow_http=true` и два файла секретов: `S3_ACCESS_KEY_ID_SECRET` и `S3_SECRET_ACCESS_KEY_SECRET` (ключ crane; их создаёт `task secrets:init`). SeaweedFS собирает из них identities при старте (политика доступа живёт в compose, а не в файле, который не перезаписывается). Одноразовый сервис `seaweedfs-init` создаёт бакет `S3_BUCKET` до старта crane, потому что `/healthz/ready` проверяет бакет и без него crane не станет готов. SeaweedFS живёт в отдельной сети `s3` (`internal`), в которой только он и crane; анонимного доступа нет, ключи лежат в файлах и в окружение не попадают. Без override-файла single-node не меняется и новых секретов не требует.

**Секреты и сети в compose.** Токен, пароль PostgreSQL и URL каталога (с паролем) лежат в файлах, пути к которым задают `CRANE_TOKEN_SECRET`, `POSTGRES_PASSWORD_SECRET` и `CRANE_DATABASE_URL_SECRET` (для Valkey добавляются `VALKEY_PASSWORD_SECRET` и `CRANE_CACHE_URL_SECRET`); compose монтирует их в `/run/secrets`, а сервер читает через `CRANE_TOKEN_FILE` и `CRANE_DATABASE_URL_FILE`. Ни в переменных окружения, ни в compose-файле, ни в `.env` значений секретов нет. Сеть `edge` с фиксированной подсетью `CRANE_EDGE_SUBNET` соединяет только caddy и crane; та же подсеть передаётся как `CRANE_TRUSTED_PROXIES`, поэтому в аудите виден адрес клиента, а не Caddy. PostgreSQL в `edge` не входит, порт admin наружу не публикуется. Эталонный Caddy удаляет клиентский `X-Request-Id` (`header_up -X-Request-Id`) и выставляет адрес своего пира в `X-Forwarded-For`. Valkey живёт в override `deploy/compose/docker-compose.cache.yaml`: аутентификация через ACL, который собирается при старте из пароля (политика в compose), адрес с паролем только в секрете `cache_url`, который сервер читает через `CRANE_CACHE_FILE` (`CRANE_CACHE` в compose не задаётся), сеть `cache` с `internal: true` только для crane и valkey. Healthcheck Valkey читает пароль из секрета внутри контейнера, в argv и `docker inspect` его нет. В `docker-compose.dev.yaml` ACL дополнен пользователем `crane-test` без ограничений: он нужен только тестам адаптера (очистка БД, `CLIENT LIST`), права `crane` там такие же, как в проде.

## 16.2 Kubernetes (Helm chart)

```mermaid
flowchart TB
  classDef app fill:#951b81,stroke:#6a135c,color:#fff
  classDef store fill:#f9f1f7,stroke:#951b81,color:#29303e
  classDef ext fill:#f2f3f3,stroke:#80858e,color:#29303e
  gw["Gateway API<br>Envoy Gateway / Traefik / Cilium"]:::ext
  subgraph NS["namespace crane"]
    api["Deployment api<br>3..N реплик, HPA по RPS и CPU<br>topology spread по зонам, PDB"]:::app
    wrk["Deployment worker<br>2..N, KEDA или HPA по длине очереди"]:::app
    anz["Deployment analyzer<br>restricted PSS, NetworkPolicy deny-all кроме trivy"]:::app
    mig["Job migrate<br>pre-upgrade hook"]:::app
    pgb["PgBouncer"]:::ext
  end
  subgraph DATA["Данные"]
    pg[("PostgreSQL<br>Percona Operator или CloudNativePG<br>1 primary + 2 sync реплики")]:::store
    vk[("Valkey<br>Sentinel 3 узла или Cluster")]:::store
    s3[("S3<br>SeaweedFS / RustFS / Ceph / облако")]:::store
  end
  obs["VictoriaMetrics, VictoriaLogs, Jaeger,<br>Perses или существующий стек"]:::ext
  gw --> api
  api --> pgb --> pg
  wrk --> pgb
  api --> vk
  wrk --> vk
  api --> s3
  wrk --> s3
  wrk --> anz
  api -.-> obs
  wrk -.-> obs
```

*Рис. 16.2. Kubernetes. Все stateful-компоненты могут быть внешними управляемыми сервисами организации.*

**Доверенные прокси и Ingress.** По умолчанию чарт `CRANE_TRUSTED_PROXIES` не задаёт, поэтому `X-Request-Id` и `X-Forwarded-For` от ingress игнорируются, а адрес клиента в событиях равен адресу ingress-контроллера. Если ingress-контроллер включён в `CRANE_TRUSTED_PROXIES`, он должен выполнять требование из строки этой переменной в таблице конфигурации (раздел 16.1).

## 16.3 Размеры по профилям

| Компонент | S · single node | M · single node или k8s | L · k8s, 3 зоны |
| --- | --- | --- | --- |
| CRANE api | 1 контейнер: 1–2 vCPU, 0.5–1 ГБ | 2 × (2 vCPU, 1 ГБ) | 6–12 × (4 vCPU, 2–4 ГБ) |
| CRANE worker | 2 × (1 vCPU, 1 ГБ) | 4–8 × (2 vCPU, 2 ГБ) |
| Analyzer + Trivy | опционально, 1 vCPU, 2 ГБ | 2 × (2 vCPU, 4 ГБ) | 4–8 × (2 vCPU, 4 ГБ) |
| PostgreSQL | 1 vCPU, 1 ГБ | 4 vCPU, 16 ГБ + реплика | 16 vCPU, 64 ГБ × 3, NVMe |
| Valkey | не обязателен | 1 × 2 ГБ | 3 × (2 vCPU, 8 ГБ) |
| Хранилище blobs | локальный диск 50–200 ГБ | S3, 2–10 ТБ | S3 с EC, 100–500 ТБ |
| Итого вычислений | **2 vCPU / 4 ГБ** | **~16 vCPU / 48 ГБ** | **~120–180 vCPU / ~350 ГБ** |

*В профиле L основная доля CPU приходится на PostgreSQL, сканирование и хранилище; сами API-реплики на Rust занимают 24–48 vCPU. Для FastAPI этот слой составил бы 150–250 vCPU (раздел 3).*

## 16.4 Обновления и миграции

-   Миграции схемы по схеме expand / migrate / contract: новая версия приложения работает со старой и новой схемой, поэтому rolling update идёт без простоя.
-   Миграции запускаются отдельной ролью `migrate`: Helm pre-upgrade hook в k8s, одноразовый сервис в Compose. Advisory lock не даёт запустить две миграции сразу.
-   Поддерживается обновление через одну минорную версию; мажорные версии с документированным путём и проверкой в CI на снапшоте реальной схемы.

## 16.5 Резервное копирование и проверяемое восстановление

Если вам это нужно — CRANE это поддерживает, но готовых профилей мы не предоставляем: конфигурация каждого проекта и предприятия, как и их бюджет на инфраструктуру, сильно различаются.

CRANE хранит metadata в PostgreSQL и неизменяемые content-addressed blob, поэтому подходит любой согласованный бэкап PostgreSQL (pgBackRest с PITR, снимки томов, управляемые бэкапы облака) вместе с версионированием, репликацией или Object Lock бакета. Эти средства работают на стороне оператора и совместимы с CRANE; CRANE их не поставляет и не настраивает.

| Данные | Что подходит (на стороне оператора) | Проверка |
| --- | --- | --- |
| PostgreSQL | pgBackRest (MIT): полные и инкрементальные бэкапы + непрерывный WAL-архив в отдельный bucket; PITR. Подходят также снимки томов и управляемые бэкапы облака | Регулярное восстановление в изолированный экземпляр (рекомендация) |
| Blobs | Неизменяемы, поэтому достаточно репликации bucket или rclone (MIT) копии новых объектов; версионирование и Object Lock. Класс `proxy-cache` не бэкапится: он восстанавливается прогревом; `restricted` бэкапится отдельно в своём контуре | `crane restore verify`: каждый asset в БД имеет blob с верным SHA-256 |
| Конфигурация | Helm values / compose-файлы в Git; секреты в хранилище секретов организации | — |

Процедура восстановления (команда `crane restore verify` — план v0.5.0; процедура закрывает критерий приёмки «backup восстановлен в тестовом окружении с проверкой metadata и blobs»):

1.  Восстановить PostgreSQL из бэкапа в изолированный экземпляр (на нужный момент, если используется PITR).
2.  Подключить хранилище blob: исходный bucket, его реплику или версию.
3.  Выполнить `crane restore verify`: она сверяет восстановленную БД с хранилищем blob (каждый asset имеет blob с верным SHA-256) и формирует отчёт. Проверить отчёт: расхождения metadata и blob устраняются до переключения трафика.
4.  Переключить CRANE на восстановленные PostgreSQL и хранилище; кэш класса `proxy-cache` прогреть заново.

## 16.6 Наблюдаемость: что CRANE отдаёт наружу

CRANE отдаёт телеметрию только в открытых форматах: метрики Prometheus/OpenMetrics, трассы и журналы OTLP, журналы JSON в stdout. Привязки к конкретному стеку нет: подходят VictoriaMetrics, Prometheus, Jaeger, VictoriaLogs, любой OpenTelemetry Collector или существующий стек организации.

### Точки съёма

| Endpoint | Порт | Назначение | Доступ |
| --- | --- | --- | --- |
| `GET /metrics` | служебный (9464) | Метрики OpenMetrics с exemplars (`trace_id`) | Только из сети мониторинга; в Helm есть ServiceMonitor и PodMonitor |
| `GET /healthz/live` | служебный | Процесс жив и event loop не завис | Kubelet |
| `GET /healthz/ready` | служебный | Готов принимать трафик: PostgreSQL и хранилище доступны; Valkey и upstream на готовность не влияют | Kubelet, балансировщик |
| `GET /healthz/startup` | служебный | Миграции применены, кэши прогреты. В v0.3.0 не реализован: `startupProbe` чарта использует `/healthz/live` | Kubelet |
| `GET /api/v1/status` | основной | Версия, режим (normal, read-only, degraded), состояние компонентов и фидов, отставание экспорта в SIEM | Пользователи, CLI `cctl diag`, внешние статус-страницы |
| OTLP export | исходящий | Трассы, журналы и, по желанию, метрики в OpenTelemetry Collector (gRPC или HTTP) | Настраивается endpoint и заголовки |

### Каталог метрик

Все метрики имеют префикс `crane_`. Длительности в секундах, размеры в байтах.

| Группа | Метрики | Метки |
| --- | --- | --- |
| Протоколы и API | `http_requests_total`, `http_request_duration_seconds` (histogram), `http_inflight_requests`, `http_response_bytes_total` | `format`, `op`, `status_class` |
| Кэш и proxy | `cache_requests_total`, `singleflight_coalesced_total`, `upstream_requests_total`, `upstream_duration_seconds`, `upstream_up` | `layer` (l1, l2), `result` (hit, miss, stale), `upstream` |
| Публикация и хранилище | `publish_total`, `publish_bytes_total`, `blob_objects`, `blob_bytes`, `gc_reclaimed_bytes_total`, `integrity_failures_total`, `storage_move_bytes_total`, `quota_usage_ratio` | `format`, `result`, `store`, `class`, `workspace` (top N + `other`) |
| Скачивания и след команды | `downloads_total`, `download_bytes_total` (`source`: hosted, hit, stale, miss, coalesced), `metadata_requests_total` (metadata и индексы, в строки потока не пишутся) | `format`, `source` (кроме `metadata_requests_total`), `workspace` (top N + `other` + `none` для не распределённых) |
| Доступ | `authz_decisions_total`, `auth_failures_total`, `authz_epoch_invalidations_total`, `credentials_expiring` | `decision`, `method`, `days` (7, 14, 30) |
| Очереди и события | `jobs_queued`, `job_oldest_age_seconds` (`now() - min(run_at)` среди видимых задач), `job_dead_total` (рост `job_dead`), `job_duration_seconds`, `outbox_lag_seconds` (только внешняя доставка), `webhook_deliveries_total` | `kind`, `outcome` |
| Уязвимости | `findings_open`, `findings_sla_breached`, `advisories_ingested_total`, `feed_lag_seconds`, `detection_time_seconds` (histogram: advisory → finding), `emergency_blocks_active`, `exceptions_expiring`, `vuln_db_age_seconds` | `priority`, `feed`, `scanner`, `days` |
| AI и скоринг | `ai_requests_total`, `ai_duration_seconds`, `ai_tokens_total`, `ai_fallback_total`, `score_value` (histogram), `score_overrides_total` | `role` (score, chat, embed, rerank), `provider`, `kind`, `direction` |
| A2A | `a2a_tasks_total`, `a2a_confirmations_total`, `a2a_task_duration_seconds` | `skill`, `state`, `outcome` |
| SIEM | `siem_events_total`, `siem_export_lag_seconds` | `sink`, `result` |
| Зависимости и рантайм | `db_pool_connections`, `db_query_duration_seconds`, `valkey_errors_total`, `s3_request_duration_seconds`, `tokio_busy_ratio`, `build_info` | `state`, `query_class`, `op`, `version`, `commit` |

**Бюджет кардинальности.** Имена пакетов, пользователи, IP и токены в метки не попадают. Метки по workspace ограничены top N с корзиной `other` (и `none` для не распределённых), N = 10 по умолчанию. Пакет, версия, принципал, токен и IP никогда не метки. Бюджет скачиваний: 5 форматов × 5 `source` × 12 значений `workspace` × 2 (`downloads_total` и `download_bytes_total`) + 5 × 12 (`metadata_requests_total`) = 660 рядов из 5 000. Цель не больше 5 000 рядов на реплику; превышение видно по самой метрике `crane_metric_series`.

### Журналы

-   **Формат.** JSON в stdout, одна строка на событие. Обязательные поля: `ts`, `level`, `msg`, `target`, `request_id`, `trace_id`, `span_id`; для запросов также `format`, `op`, `status`, `duration_ms`, `principal_kind` (человек, робот, агент), `workspace`, `rule_id` при отказе.
-   **Потоки.** Журнал приложения и access log разделены полем `stream`. Access log в stdout — операционный журнал: он пишется на 100% для ошибок и может прореживать успешные скачивания (доля настраивается, по умолчанию 10%), чтобы CI-трафик не топил журналы. Полная запись скачиваний — поток скачиваний (14.13 «Аудит скачиваний»): 100% с v0.4.0.
-   **Поля proxy.** В строке access log ответов proxy-репозиториев есть `cache` (`hit`, `miss`, `stale`, `stale-on-error`: откуда пришёл ответ), `upstream_failure` (`refused`, `unavailable`, `throttled`, `digest-mismatch`, `too-large`: почему upstream отказал) и `upstream_reason` (причина: для `refused` правило egress, для `digest-mismatch` ожидаемый и полученный sha256). Для 404 на скачивании, когда версии нет в индексе, поле `cache` относится к индексу. Отказ egress и несовпадение суммы — события безопасности, поэтому они пишутся именно в этот журнал (14.9), клиенту же уходит только «upstream недоступен». Ответ несёт заголовок `Age` (секунды в кэше) и `Warning: 110` для устаревшего или `111` для ответа при сбое upstream; `Warning` отменён RFC 9111 и оставлен для старых клиентов, основной сигнал — `Age` и поле `cache`.
-   **Egress.** Каждая реальная выборка из upstream пишется отдельной строкой с целью `crane::upstream` (журнал безопасности, как аудит и access: уровень `RUST_LOG` его не глушит). Поля: `host`, `path` (путь upstream без query), `status` при ответе либо `failure` (`refused` для отказа SSRF-guard, `unavailable` для сетевой ошибки), `elapsed_ms` (время до получения заголовков). Заголовков, тел и `ETag` в строке нет. Запрос, обслуженный из кэша или присоединившийся к чужой выборке, строки egress не порождает, поэтому по числу строк с одним `path` видно, сколько раз CRANE реально ходил к upstream.
-   **Аудит не журнал.** Действия пользователей и решения доступа идут в неизменяемый аудит и в SIEM (раздел 14.9), а не в журнал приложения: у них другие гарантии доставки и хранения.
-   **Редактирование.** Секреты не реализуют вывод в журнал на уровне типов (`Secret<T>`); дополнительно включён фильтр известных форматов токенов и заголовков `Authorization`.

### Трассировка

-   **Протокол.** OTLP, распространение контекста по W3C Trace Context. Входящий `traceparent` от CI и внешних агентов принимается; к публичным upstream заголовки трассировки не отправляются.
-   **Spans.** `http.server` (операция протокола; атрибут `crane.workspace` со slug команды атрибуции, идентификаторов людей в трассах нет), `authz.decide`, `cache.lookup`, `upstream.fetch`, `blob.get` и `blob.put`, `db.query`, `job.run`, `ai.score`, `ai.complete`, `siem.deliver`, `a2a.task`.
-   **Сэмплирование.** Parent-based, 1% по умолчанию и 100% для ошибок и медленных запросов (через tail sampling в Collector). Exemplars связывают точки гистограмм с трассами.

### SLO и алерты

SLI считаются из метрик, а алерты строятся по скорости сжигания бюджета ошибок в двух окнах (multi-window burn rate): быстрое окно ловит острые аварии, медленное ловит тлеющие.

| Алерт | Условие | Серьёзность |
| --- | --- | --- |
| `CraneDownloadErrorBudgetBurn` | Бюджет доступности скачивания сжигается в 14.4 раза быстрее нормы за 1 ч и 5 мин | page |
| `CraneMetadataLatencyHigh` | p99 metadata при кэш-хите выше SLO (раздел 4) 15 минут | page |
| `CraneReadOnlyMode` | Сервис в режиме read-only больше 5 минут | page |
| `CraneUpstreamDown` | `upstream_up == 0` 10 минут | ticket |
| `CraneQueueBacklog` | Возраст старейшей задачи больше 10 минут | ticket |
| `CraneJobDeadGrowth` | Рост `job_dead` за 15 минут | ticket |
| `CraneOutboxLag` | Отставание outbox внешней доставки (webhooks, SIEM) больше 5 минут | page |
| `CraneFeedLagging` | Фид уязвимостей не синхронизировался 30 минут | page |
| `CraneP0WithoutAction` | P0 открыт больше 1 часа без блокировки или решения | page |
| `CraneVulnDbStale` | База сканера старше 24 часов | ticket |
| `CraneSiemExportLag` | Отставание экспорта в SIEM больше 5 минут | page |
| `CraneIntegrityFailure` | Любой blob с неверным SHA-256 | page |
| `CraneScoreDrift` | Распределение оценок systemone сместилось относительно базовой недели | ticket |

Правила поставляются как PrometheusRule в Helm chart и как отдельный YAML для других систем.

### Дашборды

В поставке дашборды Perses (Apache-2.0) и их JSON-версии для совместимых систем: «Обзор и SLO», «Протоколы», «Proxy и upstream», «Хранилище и GC», «Очереди и outbox», «Безопасность: findings, SLA, фиды», «AI, скоринг и A2A», «Экспорт в SIEM», «PostgreSQL и Valkey». Каждая панель ссылается на соответствующий раздел runbook.

## 16.7 Ветвление и выпуск версий

```mermaid
flowchart LR
  classDef br fill:#f9f1f7,stroke:#951b81,color:#29303e
  classDef auto fill:#951b81,stroke:#6a135c,color:#fff
  feat["feature/*<br>от main"]:::br
  rel["release/vX.Y.Z"]:::br
  main["main<br>production"]:::br
  bot["бамп версий, CHANGELOG,<br>тег vX.Y.Z"]:::auto
  gh["GitHub Release"]:::auto
  feat -- "PR, squash,<br>Conventional Commits" --> rel
  rel -- "PR после одобрения,<br>merge commit" --> main
  main --> bot --> gh
```

*Рис. 16.7. Ветвление и выпуск.*

- `main` — основная ветка (production). Feature-ветки создаются от `main`, сливаются в `release/vX.Y.Z` через PR со **squash**; заголовок PR следует Conventional Commits и проверяется commitlint.
- PR в `release/*` проходит `task check`, тесты затронутой части и ручные мутанты ревью (таблица в описании PR). Перед PR `release/*` → `main` владелец релиза локально запускает полный `task gate` по всему дифу релиза: юниты, e2e, мутационное тестирование (cargo-mutants, Stryker) и Playwright. Результат (`Gate: passed`, `Gate-SHA`) вставляется в описание релизного PR. В CI полный гейт не гоняется; на PR в `main` идут юнит-тесты backend и frontend, линтеры и проверки структуры.
- После одобрения владельцем открывается PR `release/vX.Y.Z` → `main` со слиянием **merge commit**: история коммитов нужна для CHANGELOG.
- После слияния workflow `release` от имени GitHub App (bypass защиты `main`) бампит версии в `Cargo.toml`, `package.json` и `Chart.yaml`, пересобирает `CHANGELOG.md` по Conventional Commits (окно сборки — между тегами), коммитит `chore(release): vX.Y.Z` и ставит тег `vX.Y.Z`.
- Тег запускает workflow `publish`: он создаёт GitHub Release с заметками из CHANGELOG. Первый коммит выпущен с тегом `v0.0.0`. Сборка образов в ghcr пока не входит в процесс.
- Если запись в `main` от бота недоступна, бамп и тег выполняются вручную тем же скриптом (`tools/bump-version.ts`) и командой `git-cliff`.
- **Dogfooding.** Внутренние крейты CRANE публикуются в собственный Cargo-реестр CRANE и собираются из него: это первое применение продукта на себе и проверка sparse-протокола настоящим клиентом `cargo`. Полную проверку выполняет `task dogfood:full` (и `task gate`, где она идёт в том же стенде после e2e-сценариев); она доказывает пять утверждений.
    1. **Публикация.** Все члены воркспейса публикуются в hosted-репозиторий `internal` одним `cargo publish --workspace`; аудит содержит событие publish на каждый крейт. Манифесты правятся только в копии исходников, настоящие остаются пригодными для обычной сборки.
    2. **Схлопывание.** Восемь параллельных запросов одного холодного ключа proxy дают восемь ответов 200 с `cache=miss` и ровно одну строку egress-журнала для этого пути. Ведомые запросы одного полёта получают тот же `miss`, что и ведущий, поэтому условие доказывает именно перекрытие запросов.
    3. **Сборка из реестра.** `cargo install crane-server --registry crane --locked` с пустым `CARGO_HOME` собирает бинарник `crane`: внутренние крейты идут из hosted, внешние через proxy crates.io.
    4. **Сборка без upstream.** После отключения исходящей сети crane повторная сборка с пустым кэшем cargo проходит. Все ответы proxy-репозитория в этой фазе имеют статус ниже 500 и `cache` из `hit`, `stale`, `stale-on-error`; распределение значений печатается в лог как сведения. Какая зона свежести попадётся, зависит от времени сборки, поэтому проверяется свойство «ни одного промаха и ни одной 5xx», а ветви `stale` и `stale-on-error` покрыты тестами движка.
    5. **Недоступность upstream подтверждена.** Попадания в кэш в upstream не ходят, поэтому одних их мало: запрос холодного ключа, которого в кэше нет, получает 502 с `upstream_failure=unavailable`, а в egress-журнале с момента отключения нет ни одной успешной выборки и есть неудачная.

    **Сценарии отказов** идут в каждом прогоне на последнем cargo, до полного сценария; сервис останавливается на время проверки и всегда возвращается. Проверяет сервер (access-журнал, журнал crane, аудит, число объектов бакета), а не вывод клиента. Valkey остановлен или заморожен (`pause`: соединение открыто, ответов нет): тёплый ключ отвечает, холодный выбирается из upstream без L2 (`cache=miss`), 5xx нет, при остановке в журнале есть предупреждение адаптера; паник и ERROR нет; время запросов печатается в лог без порогов. PostgreSQL остановлен (fail closed): каждый протокольный запрос, включая попадание в кэш, получает 503, к upstream proxy не ходит, `/healthz/ready` красный, публикации нет; после возврата тот же ключ отвечает, повтор публикации проходит (не 409) и добавляет ровно один объект. S3 остановлен: чтение закэшированного ключа без blob и публикация получают 503 (в аудите Failure), число объектов не меняется, после возврата повтор публикации проходит.

    Офлайн необратим, поэтому полный сценарий идёт в стенде последним. Стенд = Valkey + S3 + PostgreSQL: Valkey L2 (`deploy/compose/docker-compose.cache.yaml`: AUTH по паролю из файла секрета, адрес с паролем в `CRANE_CACHE_FILE`, как у оператора) и CAS на SeaweedFS (`docker-compose.s3.yaml`, ключи S3 в файлах секретов, как у оператора). Число объектов бакета печатается до рестарта и после офлайна: оно должно быть больше нуля и не уменьшиться. Перед отключением сети crane перезапускается: L1 в памяти обнуляется, поэтому попадания в офлайне доказывают, что кэш переживает процесс.

## 16.8 Шифрование и проверка конфигурации

С v0.5.0. Если вам это нужно — CRANE это поддерживает, но готовых профилей мы не предоставляем: конфигурация каждого проекта и предприятия, как и их бюджет на инфраструктуру, сильно различаются.

### Что поддерживается

-   **PostgreSQL, канал.** TLS задаётся в URL каталога: `sslmode=verify-full`, `sslrootcert=<путь>`; для mTLS добавляются `sslcert` и `sslkey` (пути к файлам, ключ лежит в файле).
-   **PostgreSQL, диск.** LUKS/dm-crypt, шифрованные диски облака, шифрованный StorageClass в Kubernetes, сборки с TDE от третьих сторон (например, Percona pg_tde) — на стороне оператора и совместимы с CRANE.
-   **S3.** Серверное шифрование SSE-S3, SSE-KMS и DSSE-KMS задаётся несекретными параметрами `CRANE_BLOBS`: `aws_server_side_encryption` (`AES256`, `aws:kms`, `aws:kms:dsse`), `aws_sse_kms_key_id`, `aws_sse_bucket_key_enabled`; любой ключ вне списка несекретных параметров сервер отвергает при старте. Пример: `s3://blobs?aws_server_side_encryption=aws:kms&aws_sse_kms_key_id=<arn или id>&aws_sse_bucket_key_enabled=true`. SSE-C (`sse-c`) тоже поддерживается, но ключ `aws_sse_customer_key_base64` — секрет: его задают только через окружение `AWS_SSE_CUSTOMER_KEY_BASE64`, не в URL. Подходит и шифрование бакета по умолчанию на стороне хранилища.
-   **Бэкапы pgBackRest.** Шифрование репозитория (`repo1-cipher-type`) настраивается на стороне оператора.
-   **Valkey.** В конфигурации CRANE персистентности нет (только кэш), поэтому шифрования данных на диске нет. Но в кэше лежат тела документов proxy и указатели на CAS, поэтому нужны AUTH и сетевая изоляция, а за пределами узла или сети ещё и TLS (`valkeys`).
-   **Класс `restricted`.** Свой bucket и свой ключ KMS задаются параметрами хранилища этого класса (раздел 9).
-   **Секреты и pepper.** Как в 14.3.

### Проверка безопасной конфигурации

Параметр `CRANE_REQUIRE_SECURE_CONFIG` (по умолчанию `false`). При `true` `crane serve` отказывается стартовать, если:

-   публичный URL начинается с `http://`;
-   URL PostgreSQL без `sslmode=verify-full`;
-   endpoint S3 задан по `http://` или указан `aws_allow_http=true`;
-   режим разделения обязанностей `warn` (14.12);
-   `CRANE_PACKAGES_URL` не задан или его хост совпадает с хостом `CRANE_PUBLIC_URL`.

Без `true` те же находки дают предупреждения и событие `config.insecure`. Шифрование на диске CRANE не проверяет: это зона оператора.

### Чек-лист для оператора

Если нужно усиление, проверьте:

-   TLS везде;
-   `sslmode=verify-full` для PostgreSQL;
-   серверное шифрование S3 (SSE-KMS) через параметры `CRANE_BLOBS` или шифрование бакета по умолчанию;
-   шифрованные тома и бэкапы;
-   разделение обязанностей `strict`;
-   заданы доверенные прокси (`CRANE_TRUSTED_PROXIES`);
-   порт admin не опубликован наружу;
-   `CRANE_REQUIRE_SECURE_CONFIG=true`.
