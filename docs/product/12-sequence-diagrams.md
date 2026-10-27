# 12. Sequence-диаграммы ключевых сценариев

## 12.1 Публикация npm-пакета из GitLab CI

```mermaid
sequenceDiagram
  autonumber
  participant J as GitLab CI job
  participant A as API server
  participant G as GitLab JWKS
  participant P as PostgreSQL
  participant S as S3
  participant V as Valkey
  J->>A: POST /api/v1/auth/gitlab (ID token)
  A->>G: JWKS (кэшируется)
  A->>A: проверка подписи, iss, aud, exp и trust policy
  A-->>J: crci_ credential, TTL 15 мин, scope publish:@acme/*
  J->>A: npm publish (PUT /npm/@acme/ui, tarball)
  A->>A: Cedar decide(Publish, @acme/ui) = allow
  A->>S: stream в staging + sha512/sha256
  A->>P: BEGIN, advisory lock, проверка версии
  A->>S: commit в CAS (идемпотентно)
  A->>P: INSERT version (quarantined), assets, outbox, job, audit
  A->>P: COMMIT
  A->>V: PUBLISH inv md:npm-hosted:@acme/ui
  A-->>J: 201 Created + request id
  Note over A,P: Worker забирает analyze.package и scan.sbom асинхронно
```

*Рис. 12.1. Публикация с provenance из claims ID token.*

## 12.2 Скачивание через proxy при промахе кэша (конкурентные запросы)

```mermaid
sequenceDiagram
  autonumber
  participant C1 as Клиент 1
  participant C2 as Клиент 2..N
  participant A as API server
  participant V as Valkey
  participant U as npmjs.org
  participant S as S3
  C1->>A: GET /npm-proxy/lodash
  C2->>A: GET /npm-proxy/lodash
  A->>A: L1 промах, общий future в процессе
  A->>V: GET md:... (промах)
  A->>V: SET NX sf:lodash (лидер)
  A->>U: GET /lodash (If-None-Match)
  U-->>A: 200 packument
  A->>A: rewrite tarball URL на адрес CRANE
  A->>V: SET md:... TTL 5 мин + PUBLISH ready
  A-->>C1: 200 (ETag)
  A-->>C2: 200 из того же future
  C1->>A: GET tarball lodash-4.17.21.tgz
  A->>S: HEAD blob по пути (промах)
  A->>U: GET tarball
  U-->>A: поток байт
  A-->>C1: поток (tee) с проверкой integrity
  A->>S: запись в CAS
```

*Рис. 12.2. Один запрос в upstream независимо от числа клиентов.*

## 12.3 Отказ в доступе с диагностикой

```mermaid
sequenceDiagram
  autonumber
  participant D as Разработчик (pip)
  participant A as API server
  participant UI as Web UI
  participant P as PostgreSQL
  D->>A: GET /pypi/internal/simple/acme-billing/
  A->>A: decide(Read, acme-billing) = deny, rule ws-billing-readers
  A->>P: audit_event (deny, rule, request id)
  A-->>D: 403 и текст: нет доступа к workspace billing, правило ws-billing-readers, request id R-7f3a
  D->>UI: открыть страницу диагностики R-7f3a
  UI->>A: GET /api/v1/diagnostics/R-7f3a
  A->>A: проверка права видеть событие
  A-->>UI: причина, правило, trace политик, кому писать, кнопка «запросить доступ»
```

*Рис. 12.3. Отказ, который можно исправить без обращения в поддержку.*

## 12.4 Новая CVE: от фида до задач командам и SIEM

```mermaid
sequenceDiagram
  autonumber
  participant F as Фид (OSV, KEV, EPSS)
  participant W as Worker
  participant P as PostgreSQL
  participant S as systemone
  participant GL as GitLab
  participant SI as SIEM
  W->>F: изменения с курсора
  F-->>W: advisory GHSA-… (CVE-…), диапазоны, fix
  W->>P: нормализация, слияние псевдонимов
  W->>P: версии по (экосистема, имя), проверка диапазонов
  P-->>W: затронутые версии и граф потребителей
  W->>S: признаки: CVSS, EPSS, KEV, граф
  S-->>W: exploit_relevance 0.82, blast_radius 0.64, факторы
  W->>P: finding, приоритет P1 = max(правила, оценки), SLA 3 дня
  W->>GL: issue владельцу (дедупликация по CVE и пакету)
  W->>SI: Vulnerability Finding (OCSF), severity high
  Note over W,P: байты не пересканируются
```

*Рис. 12.4. Advisory сопоставляется с инвентарём по PURL; оценки systemone могут только повысить приоритет.*

## 12.5 Promotion релиза

```mermaid
sequenceDiagram
  autonumber
  participant O as Владелец библиотеки
  participant A as API server
  participant W as Worker
  participant SQ as SonarQube
  participant P as PostgreSQL
  SQ->>A: webhook quality gate (commit SHA)
  A->>P: evidence quality_gate, trust = integration
  W->>P: promotion.evaluate(version)
  P-->>W: cve ok, license review, provenance ok, quality ok
  W->>P: verdict pending approval (license review)
  O->>A: запрос approval у security reviewer
  A->>P: approval granted, обоснование
  W->>P: state = released, outbox version.promoted
  Note over P: байты и digest не меняются
```

*Рис. 12.5. Publish, quarantine, checks, approval, release.*

## 12.6 Запись через A2A-агента с подтверждением

```mermaid
sequenceDiagram
  autonumber
  participant X as Внешний агент
  participant A as API server (A2A)
  participant M as LLM
  participant P as PostgreSQL
  participant U as Владелец (UI)
  X->>A: «deprecate foo 1.2.3, замена 1.2.4» (a2a:write)
  A->>M: извлечь намерение (контекст по ACL)
  M-->>A: deprecate_version(foo, 1.2.3, 1.2.4)
  A->>A: Cedar: право principal на deprecate
  A->>P: план (хэш), confirmation_id, TTL 10 мин
  A-->>X: input-required: план и эффект
  U->>A: подтверждение плана
  A->>P: deprecate + audit (a2a, task_id) + outbox
  A-->>X: completed: ссылка на версию
```

*Рис. 12.6. Агент предлагает, principal подтверждает, use case исполняет. До подтверждения состояние не меняется.*

## 12.7 Разработчик получает токен для пакетных менеджеров

```mermaid
sequenceDiagram
  autonumber
  participant D as Разработчик (CLI)
  participant K as Keycloak
  participant A as API server
  participant P as PostgreSQL
  participant N as npm
  D->>K: device authorization (crane-cli)
  K-->>D: user_code, ссылка для входа
  D->>K: вход в браузере, MFA
  K-->>D: access token (aud crane, groups), refresh token
  D->>A: POST /api/v1/tokens/session (Bearer JWT, scopes)
  A->>P: credential crp_, HMAC, TTL 12 ч, audit
  A-->>D: crp_… (показан один раз)
  D->>D: запись в ~/.npmrc, settings.xml, ~/.netrc
  N->>A: GET /npm/@acme/ui (Bearer crp_)
  A->>A: HMAC, эпоха, Cedar по текущим ролям
  A-->>N: packument
  K-->>A: back-channel logout или admin event: пользователь заблокирован
  A->>P: эпоха пользователя + 1
  N->>A: следующий запрос с тем же crp_
  A-->>N: 401, токен отозван
```

*Рис. 12.7. Права не зашиты в токен: блокировка в Keycloak отзывает все токены пользователя за секунды.*
