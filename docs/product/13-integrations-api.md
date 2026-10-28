# 13. Точки интеграции и API

## Входящие и исходящие интеграции

| Система | Протокол | Сценарии | Отказоустойчивость |
| --- | --- | --- | --- |
| OIDC-провайдер (Keycloak — профиль по умолчанию) | OIDC (Authorization Code + PKCE, Device Grant, Client Credentials), Back-Channel Logout, JWKS; для Keycloak дополнительно admin events через event listener и Admin REST (только чтение); для других провайдеров SCIM 2.0 | Вход в UI (Auth Code + PKCE), проверка JWT для CLI, маппинг групп и ролей на команды и роли платформы (`group_mapping`), реакция на блокировку. | Кэш JWKS 1 ч; уже выданные сессии живут до exp; без каталога и push роль, снятая в IdP, живёт до следующего входа (ниже) |
| GitLab | OIDC ID token, REST API v4, webhooks | Workload identity, привязка пакета к проекту, создание issues, CI templates, приём SBOM, provenance и attestations из job. | Очередь с повторами для issues |
| Sigstore и upstream attestations | npm attestations endpoint, PyPI Integrity API (PEP 740), Maven Central `.asc` и `.sigstore.json`; Sigstore bundle, Rekor; TUF для trust root | Проверка provenance upstream (10.6.1), приём attestations из CI, gate provenance требует валидную attestation (настройка по репозиторию и стадии). | В закрытом контуре trust root зеркалируется (TUF-репозиторий или `trusted_root.json` переносится вручную), проверка идёт офлайн по inclusion proof и SET внутри bundle. Если trust root не обновился, используется последний валидный до истечения метаданных TUF; устаревание алертится. Ошибка проверки даёт `unverifiable`, а не `invalid` |
| Trivy | Trivy server (client/server mode) | Скан SBOM и распакованных артефактов, версия базы. | Circuit breaker; статус «ошибка анализа» |
| SonarQube | Webhook + Web API | Quality gate по commit SHA как evidence. | Pull при отсутствии webhook |
| AI-контур | systemone-совместимый API (скоринг), OpenAI-совместимый HTTP (генерация, embeddings), rerank endpoint; подробности в разделе «AI-контур» | Оценки exploit relevance, blast radius и вирусности лицензий; семантический поиск, объяснения, migration assistant, дубликаты, ответы A2A-агента. | Скоринг деградирует до правил, генерация до лексического поиска |
| Внешние агенты | A2A (Agent2Agent), JSON-RPC 2.0 over HTTPS, SSE для стриминга; подробности в разделе «A2A-агент CRANE» | Чтение каталога, evidence и объяснений; управляемые изменения (deprecate, исключения, issues) с подтверждением. | Агент недоступен — REST и UI работают без него |
| Фиды уязвимостей | OSV (API и выгрузки), NVD API 2.0, GitHub Advisory Database, CISA KEV, FIRST EPSS, CSAF 2.0 / VEX, собственные advisory организации | Инкрементальная синхронизация каждые 5–15 мин, сопоставление с инвентарём по PURL без повторного сканирования, обогащение и приоритизация (раздел 14). | Последнее состояние фида хранится локально; отставание фида алертится |
| SIEM | Syslog RFC 5424 поверх TLS (RFC 5425), HTTP JSON (пакетами), OTLP logs; схема событий OCSF, опционально CEF | Аудит с hash chain, решения доступа, аутентификация, findings с приоритетом и оценками, экстренные блокировки, действия A2A, состояние фидов (раздел 14.9). | Outbox и курсор доставки на каждый приёмник; at-least-once с `event_id`; алерт по отставанию |
| SOAR и тикеты | Входящий REST `/api/v1/response/*` с robot-credential `soar`; исходящие webhooks; GitLab Issues, Jira | Автоматические ответные действия: заблокировать версию, отозвать credential, создать исключение или задачу (раздел 14.10). | Идемпотентность по `Idempotency-Key`; опасные действия требуют approval |
| Почта, мессенджеры | SMTP, исходящие webhooks | Истечение токенов, новые риски, approvals. | Повторы, DLQ |
| Nexus | Nexus REST API | Разовая миграция hosted-пакетов. | Идемпотентный журнал |
| Наблюдаемость | OpenMetrics `/metrics`, OTLP traces и logs, JSON-журналы, health endpoints, `/api/v1/status` | Каталог метрик `crane_*`, SLO и burn-rate алерты, дашборды Perses (раздел 16.6). | Телеметрия не влияет на обработку запросов |

## Идентичность и токены доступа

OIDC-провайдер (IdP) отвечает на вопрос «кто это», CRANE на вопрос «что ему можно». Пакетные менеджеры (npm, pip, Maven, Gradle, dotnet, cargo) не умеют проходить OIDC-вход, поэтому CRANE выдаёт им собственные токены, привязанные к личности из IdP. Права в токен не зашиваются: каждый запрос заново проходит авторизацию Cedar по текущим ролям (`role_binding`, 14.15), поэтому изменение ролей действует без перевыпуска токенов; насколько быстро CRANE узнаёт об изменении в IdP, зависит от способа сверки (ниже).

### Идентичность: generic OIDC и профили

CRANE принимает любого провайдера, который отдаёт стандартный OIDC (discovery, JWKS). Keycloak не особый случай, а профиль по умолчанию: готовая конфигурация и два необязательных расширения (каталог и push). Вендорных адаптеров Entra, Google и GitLab нет (решение 34 в 19).

**Порты** (`crates/core/ports`; порт `IdentityProvider` из прежней схемы заменён тремя):

```rust
pub trait IdentityVerifier: Send + Sync {
    async fn verify(&self, bearer: &str) -> Result<VerifiedIdentity, VerifyError>;
}
// VerifiedIdentity { issuer, subject, username, display, email, groups: GroupClaim, auth: AuthStrength, expires_at }
// GroupClaim::{ Complete(CompleteGroups), Overage, Absent }  - «пусто» и «неизвестно» различаются типом; use case observe(p, &CompleteGroups, via) вызывают только для Complete
// AuthStrength { acr: Option<String>, amr: Vec<String>, auth_time: Option<Timestamp> }
pub trait Directory: Send + Sync {        // только pull, по группам из group_mapping
    async fn members(&self, group: &GroupRef) -> Result<Vec<DirectoryMember>, DirectoryError>;
}   // DirectoryMember { subject, active }; «нет каталога» = Option<Arc<dyn Directory>> == None
pub trait RoleBindings: Send + Sync {     // имя совпадает с таблицей role_binding
    async fn reconcile(&self, p: PrincipalId, source: Source, grants: &BTreeSet<Grant>, via: Via) -> Result<Diff, StoreError>; // атомарно с audit_event
    async fn set_local(&self, p: PrincipalId, ws: WorkspaceId, role: Role, on: bool, by: PrincipalId) -> Result<Diff, StoreError>;
    async fn active_at(&self, p: PrincipalId, at: Timestamp) -> Result<Vec<Grant>, StoreError>;
}
```

Чистые функции домена: `resolve(&CompleteGroups, &[GroupMapping]) -> BTreeSet<Grant>`, `attribute(...)` и `context_mfa(...)` (10.16).

**Адаптеры.** `adapters/identity/oidc` — единственный проверяющий: discovery, JWKS с кэшем 1 ч, нормализатор claims, device flow. `adapters/identity/keycloak` — только `Directory` через Admin REST (чтение) для опроса. Push принимают входящие адаптеры в `entry`: admin events Keycloak (event listener с HTTP-доставкой) и SCIM 2.0 (подмножество: `Users` с `active`, `Groups` с `members` через PATCH). Оба вызывают тот же use case `observe`, что и вход.

**Конфигурация** (`entry`, YAML; один провайдер на экземпляр в v0.4.0):

```yaml
oidc:
  issuer: https://kc.example/realms/acme
  audience: crane
  clients: { web: crane-web, cli: crane-cli }
  claims:
    subject: sub
    username: preferred_username
    email: email
    groups:
      - { claim: groups, format: path, prefix: /crane/ }     # format: path | name | id
      - { claim: resource_access.crane.roles, as: "role:" }   # роли нормализуются в GroupRef с префиксом
    nested: as_emitted     # as_emitted | expand_parents (только format: path: /a/b даёт и /a)
  mfa:
    acr_accepted: ["gold"]          # Keycloak LoA, Okta urn:okta:loa:2fa:any
    amr_accepted: ["mfa", "hwk", "otp"]
    max_age: 600s                   # свежесть: now - auth_time <= max_age
    assume_idp_enforced: false      # true: оператор утверждает, что IdP требует MFA на каждом входе
  device_flow: auto        # auto (по discovery device_authorization_endpoint) | off
  session_ttl_without_directory: 1h  # потолок crp_ при отсутствии каталога и push
access:
  local_membership: allow  # allow | deny
```

**Нормализация claims и маппинг групп.** Нормализатор превращает все настроенные claims (groups, roles, client roles) в одно множество `GroupRef`; это множество отображает только `group_mapping`: группа → команда и роль, либо → роль экземпляра при `workspace_id NULL`. Пример для Keycloak: группа `/crane/payments/maintainer` → (payments, maintainer); client role `crane:platform-admin` нормализуется в `role:platform-admin` → роль экземпляра. Команды создаются в CRANE, группа IdP команду не создаёт. Маппинги ведёт только platform-admin со step-up: это выдача прав (14.15).

-   **Префикс.** Учитываются только группы, начинающиеся с `prefix`; остальные отбрасываются до маппинга. Токен в памяти меньше, чужие группы не попадают в журнал.
-   **Вложенность.** `as_emitted` берёт группы как их отдал IdP. `expand_parents` (только `format: path`) добавляет родителей: `/a/b` даёт и `/a`. Нужен там, где провайдер отдаёт только прямые группы (Keycloak, по исходному коду mapper).
-   **Overage.** Если IdP не вместил группы в токен (Entra: `_claim_names`/`_claim_sources`), результат `GroupClaim::Overage`. Роли из токена не меняются: ничего не закрывается и не открывается (`observe` вызывается только для `Complete`; режима «взять группы из каталога» нет, overage всегда fail closed). Обход для Entra — «группы, назначенные приложению» или SCIM. «Пусто» и «неизвестно» различаются типом.
-   **Локальное членство.** Флаг `access.local_membership` (`allow` по умолчанию; `deny` — организация запрещает права мимо IdP) управляет только строками источника `local` (14.15).

**Свежесть MFA.** `context.mfa` для Cedar — чистая функция: `(acr ∈ acr_accepted ИЛИ amr ∩ amr_accepted ≠ ∅ ИЛИ assume_idp_enforced) И auth_time есть И now − auth_time ≤ max_age`. Нет `auth_time` → не свежо (fail closed). Step-up — повторный вход с `max_age=0` (и `acr_values`, если профиль их поддерживает); BFF и CLI запускают его по `401 insufficient_user_authentication`. Step-up всегда требует свежего MFA, а без `auth_time` подтвердить его нельзя: опасные действия недоступны. Правила выпуска токенов и список опасных действий описаны в разделах 13 и 14; здесь только вычисление `context.mfa`.

**Вход из CLI.** Если у IdP есть device flow, `cctl login` использует его (`device_flow: auto` читает `device_authorization_endpoint` из discovery). Иначе CLI открывает loopback Authorization Code + PKCE на `127.0.0.1:<случайный порт>` (RFC 8252). Если на машине нет браузера, вставка кода из браузера другого устройства не поддерживается (решение 39 в 19): личный токен выпускается в SPA, и `cctl login --token` принимает его.

**Сверка ролей с IdP.** Способы дополняют друг друга:

| Способ | Когда работает | Что делает |
| --- | --- | --- |
| JIT из токена | Всегда: вход, обмен, обновление `crp_` через refresh | `observe` по группам токена |
| Pull | Только если настроен `Directory` (Keycloak) | Задача `access.directory_sync` раз в 5 мин: проходит по группам из `group_mapping`, закрывает строки `idp` у тех, кого в группе больше нет, повышает эпоху неактивных (отзыв ≤ 5 с, 10.4); субъекты каталога без `principal` пропускаются, принципал создаётся при первом входе |
| Push | Keycloak admin events; SCIM (Entra, Okta, Authentik) | Приём события вызывает `observe`; SCIM: `Users.active`, `Groups.members` |

**Деградация без расширений Keycloak** (admin events, Admin REST). Без pull и push удаление человека из группы в IdP закрывает строку `role_binding` только при его следующем входе или обновлении токена; до этого роль остаётся открытой. Отключённый в IdP пользователь теряет доступ не позже `session_ttl_without_directory` (потолок `crp_`, по умолчанию 1 ч вместо 12 ч) или истечения refresh. Отзыв при блокировке идёт не за секунды, а за этот срок; отчёт о пересмотре доступа показывает «наблюдалось в момент» (`valid_from`), а не состояние IdP. Риск и митигирование: раздел 19.

**Матрица провайдеров.** Проверено по официальной документации 2026-10-28; «не проверено» означает, что факт не подтверждён и перед использованием проверяется на стенде.

| Провайдер | Группы в токене | Overage / лимит | MFA-сигнал | auth_time | Device flow | Каталог (pull) | Push | Рекомендуемый путь |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Keycloak | `groups` через Group Membership mapper (настроить); полный путь `/a/b` (по умолчанию) или имя; только прямые группы (по исходному коду mapper) → `nested: expand_parents` | нет | `acr` (ACR→LoA в realm), `amr` через AMR mapper; `acr_values`, `claims` поддерживаются | да | да (с 13.0, включить у клиента) | Admin REST (адаптер `keycloak`) | admin events через event listener (расширение SPI); SCIM в Keycloak только входящий | Родной профиль: JIT + опрос 5 мин + admin events |
| Authentik | `groups` в scope `profile` по умолчанию; в ID token — при «Include claims in id_token»; формат и вложенность не проверены | не проверено | не проверено → `assume_idp_enforced` или проверить на стенде | не проверено | да (`/application/o/device/`, нужен device-code flow бренда) | не используется | SCIM provider (исходящий push, событийно + полная синхронизация раз в час) | JIT + SCIM; второй IdP в compose-тесте |
| Zitadel | нативного `groups` нет; роли проекта `urn:zitadel:iam:org:project:roles` (по запросу) или группы через Action | — | `amr` (RFC 8176), `acr` | да | да | не используется | исходящего SCIM нет (только входящий) | JIT по ролям проекта (`as: "role:"`) + TTL |
| Okta | `groups` только с настроенным фильтром; имена | 100; при превышении запрос токена падает (не усечение) → узкий фильтр/префикс обязателен | `amr` всегда (`pwd`, `mfa`, `otp`, `hwk`…), `acr_values` `urn:okta:loa:2fa:any` | да | да (native-приложения) | не используется | не проверено (вероятно, через парное SAML/SWA-приложение): по документации SCIM provisioning в Okta рассчитан на SAML/SWA-интеграции из App Integration Wizard, а не на OIDC-приложение; Event Hooks | JIT; SCIM не проверено |
| Entra ID | `groups` = GUID объектов (имена только для синхронизированных из AD или `ApplicationGroup`); включает вложенные (кроме «групп, назначенных приложению») → `format: id` | 200 в JWT с учётом вложенных; при превышении `groups` нет, есть `_claim_names`/`_claim_sources` → `GroupClaim::Overage` → роли из токена не меняются | `amr` содержит `mfa`; step-up через `claims` с `acrs` (auth context, нужен P1); `acr` только v1 | да | да | не используется (Graph `transitiveMemberOf` не реализуем, решение 34) | SCIM provisioning (не-галерейное приложение) | JIT с «группами, назначенными приложению» (обходит overage) + SCIM |
| Google Workspace | групп в ID token нет | — | `amr` только если включено в настройках Google Auth Platform и запрошено через `claims`; может отсутствовать; `acr` нет | да | только «TV and limited input»-клиенты, scopes `openid email profile` | Cloud Identity Groups API требует платных редакций — не реализуем | исходящий SCIM только для каталожных приложений | Локальное членство (`local`), вход через Google |
| Dex | `groups` при scope `groups`; формат от коннектора | от коннектора | `acr`/`amr` нет | не проверено (в списке claims Dex нет) → считать «нет» | да, но не в `grantTypes` по умолчанию — включить | нет | нет | JIT + TTL; без `auth_time` опасные действия недоступны, `assume_idp_enforced` не помогает (нет свежести) |
| GitLab | `groups_direct` в ID token (прямые, пути); `groups` и `https://gitlab.org/claims/groups/{owner,maintainer,developer}` только в userinfo | нет | `acr`/`amr` нет → `assume_idp_enforced` (группа требует 2FA) | да | да: с 17.2 за флагом, GA в 17.9 (исправление CVE-2025-0549 в 17.9.8 / 17.10.6 / 17.11.2) | не используется | system hooks (`user_add_to_group`, `user_remove_from_group`), блокировки пользователя нет — не реализуем | JIT по `groups_direct` + TTL |

Back-channel logout: Keycloak да; Authentik да (preview с 2025.8); Zitadel да (с 2.65); Okta нет (Universal Logout); Entra только front-channel; Google, Dex, GitLab не документировано.

### LDAP, Active Directory и SAML: только через IdP

CRANE никогда не будет подключаться к LDAP или Active Directory напрямую. Это принцип продукта (решение 44 в 19), а не отложенная функция.

-   **Пароли и привилегированная учётная запись.** Прямой bind означает, что CRANE принимает пароли пользователей и хранит привилегированную учётную запись для поиска в каталоге. Реестр на пути сборки стал бы держателем секретов, а поверхность атаки выросла бы.
-   **Нет сигнала MFA и устойчивой к фишингу аутентификации.** В LDAP нет ни `acr`/`amr`/`auth_time`, ни WebAuthn и passkeys. Модель опасных действий CRANE (свежий MFA, step-up, раздел 13) при прямом подключении невыполнима.
-   **Каталоги запутаны и разрастаются.** Деревья OU и групп в корпоративных каталогах обычно непоследовательны. Вывод прав из них ведёт к избыточным правам и тихому дрейфу доступа. CRANE выдаёт права только явным `group_mapping` (14.15).
-   **Инъекции и цель атакующего.** LDAP-инъекции известны, а AD — главная цель при боковом перемещении и краже учётных данных. Реестр на пути сборки не должен быть ещё одним местом, где эти учётные данные вводятся.

Поддерживаемый путь — федерация: перед каталогом стоит Keycloak (User Federation LDAP/AD) или другой OIDC-провайдер, а CRANE говорит только по OIDC. SAML напрямую тоже не поддерживается: SAML-провайдер подключается через федерирующий OIDC-провайдер (в Keycloak — identity brokering). Адаптеров и «плагинов» LDAP и SAML не будет.

### Профиль Keycloak

| Объект в Keycloak | Тип | Потоки OIDC | Назначение |
| --- | --- | --- | --- |
| Клиент `crane-web` | confidential | Authorization Code + PKCE (S256) | Вход в UI. Токены хранит BFF на сервере, браузер получает только HttpOnly-cookie сессии |
| Клиент `crane-cli` | public | Device Authorization Grant (RFC 8628), refresh token | `cctl login` в терминале, на сервере без браузера и в IDE |
| Клиент `crane` | bearer-only (audience) | — | Аудитория `aud: crane` в токенах; client roles для ролей уровня экземпляра (раздел 14.12) |
| Сервисные аккаунты автоматизации | confidential | Client Credentials | Jenkins, Argo CD, скрипты: обмен токена на краткоживущий токен CRANE (ниже) |
| Группы `/crane/<workspace>/<роль>` | группы | — | Источник прав в команде: маппинг группы в `group_mapping` даёт роль `owner`, `maintainer`, `developer` или `reader`; сама группа команду не создаёт |

Мапперы в клиентах: аудитория `crane`, членство в группах (Group Membership mapper, полный путь, claim `groups`), client roles клиента `crane`, `preferred_username`, `email`, `acr` (Level of Authentication) и `amr` (AMR mapper). Свежесть MFA и step-up вычисляются по `acr`/`amr` и `auth_time` (выше). Имя группы `/crane/<slug>/<роль>` — только рекомендация по именованию: UI предзаполняет `group_ref` при создании маппинга, путь не разбирается, права идут только через явный `group_mapping` (14.15). Каталог (Admin REST) и admin events включаются как расширения профиля: без них действует деградация, описанная выше.

### Какие токены принимает CRANE

| Токен | Кто выпускает | Срок жизни | Кто предъявляет | Где хранится секрет |
| --- | --- | --- | --- | --- |
| Access token Keycloak (JWT) | Keycloak | 5 мин, продление refresh-токеном | BFF UI, CLI, REST API, A2A-агент | Нигде в CRANE: проверяется подпись по JWKS |
| `crp_` личный токен | CRANE по интерактивной сессии IdP человека | Сессионный: 12 ч; личный долгоживущий: до 90 дней по политике | Пакетные менеджеры разработчика | HMAC в `credential`; сам токен в конфигурации клиента или keychain |
| `crr_` токен робота | CRANE по решению владельца workspace | Обязательный срок, до 90 дней; ротация с перекрытием 24 ч | Сервисы и сборки, где нет workload identity | HMAC в `credential`; секрет показывается один раз |
| `crci_` токен нагрузки | CRANE по OIDC-токену платформы (обмен) | 15 мин по умолчанию, не больше 1 ч и не дольше job | GitLab CI, Kubernetes, сервисные аккаунты Keycloak | Нигде: выпускается на каждый job |

У всех токенов CRANE формат `<префикс><8 символов идентификатора><32 байта base62><crc32>` (раздел 10.4). Разные префиксы нужны secret-сканерам GitLab и SIEM: по префиксу видно, чей токен утёк и что с ним делать.

#### Правила выпуска

1.  Токены выпускает только человек со своей интерактивной сессией IdP: cookie BFF или JWT, полученный входом пользователя (authorization code + PKCE, в том числе loopback на 127.0.0.1, или device code) и его продлением refresh-токеном. Токен CRANE (`crp_`, `crr_`, `crci_`) и JWT сервисного аккаунта (client credentials) ничего не выпускают: `403 interactive-login-required`. Иначе postinstall-скрипт с `crp_` из `~/.npmrc` выпустил бы себе личный токен на 90 дней.
2.  Только сужение. Scopes нового `crp_` не шире текущих прав человека; credential робота получает ровно scopes робота; `crci_` не шире trust policy (10.5). Запрос шире даёт `403 scope-exceeds-principal`.
3.  Step-up. Личный долгоживущий `crp_` и секрет робота (выпуск и ротация) выпускаются только со step-up, как создание робота. Сессионный `crp_` step-up не требует, но живёт не дольше 12 часов и не дольше сессии IdP (срок refresh-сессии; выход из сессии отзывает токены через эпоху). Решение: step-up на сессионный токен означал бы MFA каждые 12 часов и полную недоступность CRANE при IdP без step-up.
4.  `POST /tokens/exchange` принимает `subject_token` только от доверенного внешнего издателя и выпускает только `crci_`; токены CRANE как `subject_token` не принимаются.
5.  Команда атрибуции фиксируется при выпуске: 14.15.

Алгоритм выпуска — 10.4.

### Разработчик: от входа до `npm install`

1.  **Вход.** `cctl login` запускает device flow: CLI показывает код и ссылку, пользователь входит в Keycloak (с MFA по политике realm). CLI получает access и refresh токены и кладёт refresh в системное хранилище секретов (Keychain, Secret Service, Credential Manager).
2.  **Выпуск токена для клиентов.** `cctl setup npm pip maven gradle nuget cargo` вызывает `POST /api/v1/tokens` (`kind: session`) с JWT пользователя. CRANE создаёт `crp_` на 12 часов, ограниченный scopes из запроса (например, только чтение или конкретные workspaces), и возвращает его один раз.
3.  **Настройка клиентов.** CLI записывает токен в конфигурацию каждого клиента (таблица ниже). Секрет не попадает в историю shell и в файлы проекта.
4.  **Работа.** Пакетный менеджер предъявляет `crp_`; CRANE находит запись по идентификатору, сверяет HMAC, проверяет эпоху пользователя и считает решение Cedar по текущим ролям. Скрытый пакет протокол отдаёт как несуществующий (404), причину видно только в `GET /api/v1/diagnostics/{request_id}`.
5.  **Продление.** `cctl login --refresh` (или хук IDE) выпускает новый `crp_` по refresh-токену Keycloak без повторного ввода пароля. Продление работает, пока жива сессия IdP; когда сессия Keycloak закончилась, нужен новый вход.

Для тех, кому сессионного токена мало (например, офлайн-сборка на ноутбуке), владелец политики может разрешить личный долгоживущий токен: явные scopes, срок до 90 дней, напоминания за 14 и 3 дня, отзыв из UI одним действием; выпуск только со step-up.

| Клиент | Куда CLI пишет токен | Как передаётся |
| --- | --- | --- |
| npm, pnpm, yarn | `~/.npmrc`: `//crane.example/npm/:_authToken=crp_…` | `Authorization: Bearer` |
| pip, uv, poetry | `~/.netrc` (или keyring) для хоста CRANE, `index-url` без секрета | Basic: пользователь `__token__`, пароль `crp_…` |
| Maven | `~/.m2/settings.xml`, `<server>` с идентификатором репозитория | Basic: пользователь `token`, пароль `crp_…` |
| Gradle | `~/.gradle/gradle.properties`: `craneToken=crp_…` | Basic или заголовок через `credentials(HttpHeaderCredentials)` |
| dotnet, NuGet | Credential provider `crane` (протокол NuGet credential providers) выдаёт токен по запросу | Basic; провайдер сам продлевает токен через CLI |
| cargo | `~/.cargo/config.toml`: `[registries.crane] index = "sparse+https://crane.example/cargo/internal/index/"` и `credential-provider`; токен в системном хранилище через credential provider cargo | Заголовок `Authorization: crp_…` (cargo передаёт токен без префикса схемы); в CI переменная `CARGO_REGISTRIES_CRANE_TOKEN` |

Если задан origin пакетов (`CRANE_PACKAGES_URL`, 16.1), адреса в конфигурации клиентов указывают на него, а не на консоль.

`cctl setup` настраивает два источника — hosted и proxy. Порядок источников не важен (19 #28); hosted-пакеты npm публикуются только со scope, unscoped-имена запрещены (10.3).

### Cargo: sparse-реестр

CRANE реализует только sparse-протокол cargo (RFC 2789, стабилен с cargo 1.68, по умолчанию с 1.70); git-индекс не поддерживается. Один репозиторий CRANE — один реестр cargo, hosted (внутренние крейты) или proxy (зеркало crates.io).

| Назначение | Запрос |
| --- | --- |
| Конфигурация реестра | `GET /cargo/{repo}/index/config.json`: `dl`, `api`, `auth-required: true` |
| Индекс крейта | `GET /cargo/{repo}/index/{путь}`: путь по схеме `1/`, `2/`, `3/{a}/`, `{ab}/{cd}/`; ответ NDJSON, одна строка на версию, поддерживает `ETag` и `If-None-Match` |
| Скачивание `.crate` | `GET /cargo/{repo}/api/v1/crates/{name}/{version}/download`: потоком или presigned redirect (раздел 11.2) |
| Публикация | `PUT /cargo/{repo}/api/v1/crates/new`: бинарный формат cargo (длина + JSON metadata, длина + `.crate`); проверка SHA-256, `Cargo.toml` внутри архива и namespace |
| Yank / unyank | `DELETE /cargo/{repo}/api/v1/crates/{name}/{version}/yank`, `PUT …/unyank` |
| Поиск, владельцы | `GET …/api/v1/crates?q=`, `GET/PUT/DELETE …/api/v1/crates/{name}/owners` (владельцы = `owner` и `maintainer` команды-владельца, 14.15) |

Для proxy-репозитория CRANE подставляет собственный `dl` в `config.json`, а файлы индекса и `.crate` берёт у `index.crates.io` и `static.crates.io` через общий кэш (раздел 11.1). Имена крейтов нормализуются без учёта регистра и с равенством `-` и `_`; индексный путь строится по нижнему регистру. Публичность имени при первой публикации проверяется по API crates.io (`/api/v1/crates/{каноническое имя}`, хост `crates.io` входит в egress-allowlist), поэтому любое написание публичного крейта (`serde-json` при публичном `serde_json`) получает отказ `PublicName`; ответ 404 означает «имени нет», сбой или таймаут — отказ 503. Внутренние имена защищены от dependency confusion так же, как в других форматах: namespace с приоритетом hosted над proxy. Поле `registry` зависимостей при публикации проверяется: зависимости из этого же реестра хранятся без `registry`; зависимости с crates.io приводятся к каноническому адресу crates.io, и потребитель направляет их через proxy CRANE с помощью `[source.crates-io] replace-with`; зависимости на любой другой реестр публикация отвергает (400) с событием аудита, разрешающего правила нет. Для SBOM используется `Cargo.lock` и PURL `pkg:cargo/{name}@{version}`.

### PyPI: Simple API и загрузка

Один репозиторий CRANE — один индекс PyPI. Hosted-репозиторий отвечает pip, uv и poetry на `/pypi/{repo}/simple/` и принимает загрузку от twine, poetry и `uv publish`. Proxy `pypi.org` появляется следующим шагом вехи v0.3.0.

| Назначение | Запрос |
| --- | --- |
| Страница проекта | `GET /pypi/{repo}/simple/{name}/`: HTML (PEP 503) или JSON (PEP 691, API 1.1) по `Accept`; JSON отдаётся, если клиент назвал `application/vnd.pypi.simple.v1+json` (или `…latest+json`), иначе HTML; `Vary: Accept`, `ETag`. Имя без нормализации (`Acme_Core`) и адрес без завершающего `/` перенаправляются на нормализованный адрес со слэшем (301). Проект без видимых версий и неизвестный проект — 404 |
| Корень индекса | `GET /pypi/{repo}/simple/` в v0.3.0 не обслуживается (404): клиенты для установки его не читают, а список проектов репозитория требует нового метода каталога |
| Скачивание файла | `GET\|HEAD /pypi/{repo}/files/{name}/{version}/{filename}`: имя в нормализованном виде, версия в канонической (`1.0.0` и `1.0` — одна версия `1`); yanked-файл скачивается, чтобы lock-файлы собирались |
| Загрузка | `POST /pypi/{repo}/legacy/` и `POST /pypi/{repo}/legacy` (оба написания без редиректа: twine и poetry падают на любой 3xx): legacy upload API, `multipart/form-data` с одним файлом `content`, который стоит после полей формы |
| Yank / unyank (PEP 592) | `PUT\|DELETE /pypi/{repo}/packages/{name}/versions/{version}/yank`: временный маршрут формата, операция переезжает в admin API `yankVersion` / `unyankVersion` (v0.4.0). Ответ 204, повтор идемпотентен |

Клиенты предъявляют токен через Basic (пользователь `__token__`, пароль `crp_…`); без токена ответ 401 с `WWW-Authenticate: Basic realm="crane"`, с неверным 403.

Загрузка принимает только `.whl` (`filetype=bdist_wheel`) и `.tar.gz` (`filetype=sdist`); `.zip`, `.egg` и `.exe` отвергаются (400). Проект и версия в имени файла сравниваются с полями `name` и `version` формы после нормализации (PEP 503 для имени, PEP 440 для версии), имя файла должно быть одним сегментом из латиницы, цифр и знаков `. _ - + !`. Заявленный `sha256_digest` проверяется при записи; `md5_digest` и `blake2_256_digest` принимаются, но не сверяются: хранилище адресуется по SHA-256 и считает его само. Метаданные версии (`requires_python`, `license_expression`) пишет первый файл версии, остальные файлы к ней прикрепляются отдельными запросами. Повтор того же файла отвечает 200; другой файл под тем же именем — 409 с текстом `File already exists: …`, по которому `--skip-existing` распознаёт занятое имя. Размер всего тела ограничен `CRANE_MAX_PUBLISH_BYTES` (превышение даёт 413), одно текстовое поле формы — 1 МиБ. Первая загрузка имени защищена так же, как в других форматах (namespace и проверка публичного реестра).

### Сборки и роботы

-   **GitLab CI (рекомендуется).** Секрет не хранится вообще: job предъявляет ID token с `aud: crane`, CRANE сверяет claims с trust policy и выдаёт `crci_` на время job (раздел 10.5, рис. 12.1). Scope записи выдаётся только для защищённых тегов и веток (`ref_protected`), пайплайны merge request и форков получают только чтение (10.5).
-   **Другие платформы с OIDC.** Тот же обмен работает для любого доверенного издателя: projected ServiceAccount token Kubernetes, GitHub Actions, сервисный аккаунт Keycloak (Client Credentials). Запрос `POST /api/v1/tokens/exchange` по RFC 8693: `subject_token` = JWT платформы. Trust policy задаёт издателя, допустимые claims (проект, namespace, ref, имя сервисного аккаунта) и scopes.
-   **Робот с секретом.** Для систем без OIDC владелец workspace создаёт робота в UI: имя, workspace, scopes (репозитории, namespaces, действия), обязательный срок. Токен `crr_` показывается один раз. Ротация: второй секрет выпускается заранее, оба действуют 24 часа, затем старый отзывается. Неиспользуемые роботы подсвечиваются через 30 дней и предлагаются к отзыву через 90 (раздел 11.5). Выпуск и ротация секрета требуют step-up, как создание робота, и интерактивной сессии IdP.

### Отзыв и синхронизация с IdP

-   **События IdP.** Обычный OIDC Back-Channel Logout одной сессии IdP (если провайдер его поддерживает, матрица выше; в Keycloak штатно) не трогает эпоху пользователя: `sid` из logout token выставляет `revoked_at` у каждого credential с этим `idp_sid`, то есть у сессионных `crp_`, выпущенных из этой сессии. Долгоживущие личные и роботные токены не затрагиваются. В профиле Keycloak блокировка пользователя и изменение групп приходят как admin events, если подключён event listener с HTTP-доставкой (расширение SPI); у других провайдеров push идёт по SCIM 2.0. По такому событию CRANE вызывает `observe` и увеличивает эпоху пользователя, и все его `crp_` перестают действовать на всех репликах за ≤ 5 секунд (раздел 10.4).
-   **Смена пароля, сброс учётных данных и «выйти из всех сессий».** Смена пароля или учётных данных в IdP (Keycloak: user events `UPDATE_PASSWORD` и `UPDATE_CREDENTIAL` через event listener, admin event для сброса credentials) и «выйти из всех сессий» (logout-all) увеличивают эпоху пользователя так же, как блокировка: все его `crp_` перестают действовать за ≤ 5 секунд. У других провайдеров работает back-channel logout там, где его показывает матрица, и SCIM `active=false`; иначе действует только потолок `session_ttl_without_directory`.
-   **Страховка от потерянных событий.** Если настроен каталог (Keycloak), раз в 5 минут CRANE сверяет активных пользователей и их группы через Admin REST (только чтение): отключённый в IdP пользователь теряет доступ не позже чем через 5 минут, даже если событие потерялось. Без каталога и push срок определяет `session_ttl_without_directory` или refresh (по умолчанию 1 ч для `crp_`).
-   **Отзыв в CRANE.** Пользователь видит свои токены и может отозвать любой; владелец workspace отзывает токены роботов; SecOps отзывает любой токен, в том числе по ответному действию SOAR (раздел 14.10).

## Публичный API платформы

-   **REST** `/api/v1`, спецификация OpenAPI 3.1 генерируется из кода (utoipa). Ошибки в формате `application/problem+json` (RFC 9457) с полями `rule_id`, `request_id`, `remediation`.
-   **Версионирование**: мажорная версия в пути; deprecation через заголовок `Sunset` минимум за 6 месяцев.
-   **Идемпотентность**: заголовок `Idempotency-Key` для всех мутаций, хранится 24 ч вместе с хэшем запроса. Ключ действует в пределах principal. Ответы с секретом не сохраняются: повтор даёт 409 без секрета со ссылкой на ресурс (секрет показывается один раз, 14.3).
-   **Пагинация** курсорами, фильтры как query-параметры, частичные ответы через `fields`.
-   **CLI** `cctl`: login (device flow), robots, tokens, publish, promote, diag, warm, readiness, export.
-   **Декларативная конфигурация** (этап после P3): YAML-манифесты workspaces, репозиториев, namespaces и политик, применяемые через `cctl apply` с dry run.

## AI-контур: скоринг systemone и генеративные модели

В продукте две разные функции ИИ, и обе обязательны. **systemone-модели считают**: выдают числовые оценки риска, по которым работают политики и очередь разбора. **LLM пишут**: объясняют решения, сводят находки, ведут диалог и разбирают намерение в A2A-агенте. Одна функция не заменяет другую: число без объяснения не принимают люди, а объяснение без числа нельзя положить в политику.

| | systemone-модели (скоринг) | LLM (генерация) |
| --- | --- | --- |
| Что делают | Числовая оценка 0..1 с уверенностью и главными факторами | Текст со ссылками на источники |
| Вход | Структурированные признаки: advisory, CVSS, EPSS, KEV, CWE, граф потребителей, тексты и SPDX-выражения лицензий, контекст использования | Заземлённый промпт: контекст отфильтрован ACL и очищен от секретов |
| Кто потребляет | Политики (пороги), приоритизация уязвимостей, license gate, очереди security и юристов, SIEM | Пользователь в UI, A2A-агент, migration assistant |
| Пример модели | JEV или другая systemone-совместимая | Gemma или любая модель с OpenAI-совместимым API |
| Порт в ядре | `RiskScorer` | `AiProvider` (`complete`) |
| Если недоступна | Приоритет и решения по правилам: CVSS, EPSS, KEV, категория SPDX | Нет объяснений и ассистентов; A2A отвечает структурированными данными |

### Оценки systemone

| Оценка | Какой вопрос закрывает | Признаки | Где используется |
| --- | --- | --- | --- |
| `exploit_relevance` | Насколько вероятна эксплуатация уязвимости в нашем контексте | Вектор CVSS, EPSS, присутствие в KEV, CWE, публичный эксплойт, тип зависимости (runtime, dev, test), формат и экосистема | Приоритет finding (раздел 14.6), gate promotion |
| `blast_radius` | Насколько широк возможный ущерб | Граф потребителей по уровням достоверности, глубина, число развёрнутых приложений и их критичность, число команд | Приоритет, адресаты уведомлений, экстренный режим (14.8) |
| `license_virality` | Насколько сильно обязательства лицензии распространяются на производное ПО в данном контексте | SPDX-выражение и тексты, категория, исключения (`WITH`), способ связывания (статическое, динамическое, отдельный процесс, сеть), контекст (internal, distributed, SaaS, embedded), нестандартные тексты `LicenseRef-*` | License gate (10.8), порядок очереди юристов, предупреждение в карточке пакета |

Правила работы с оценками:

-   **Оценка это evidence.** Хранится в `risk_score` вместе с моделью, её версией и хэшем признаков; пересчитывается при новом advisory, изменении графа, новой версии модели. Любое решение можно воспроизвести.
-   **Оценка только ужесточает.** Высокая оценка повышает приоритет или переводит решение в review. Низкая оценка не опускает решение ниже базовых правил: KEV, вредоносный пакет и deny-лицензия обрабатываются по правилам независимо от модели.
-   **Объяснимость.** Главные факторы оценки показываются в UI, передаются в SIEM и подаются LLM, которая пишет человеческое объяснение «почему P1».
-   **Качество.** Новая версия модели сначала работает в теневом режиме (оценки пишутся, но не влияют на решения); отслеживаются доля ручных переопределений security и юристов и дрейф распределения оценок.
-   **Границы.** Оценка не является юридическим заключением и не утверждает, что уязвимость эксплуатируема. Это сигнал для политики и очереди людей.

### Генеративные модели и поиск

| Роль | Пример модели | Протокол | Где используется |
| --- | --- | --- | --- |
| Генерация | Gemma, резерв: вторая LLM | OpenAI-совместимый HTTP | Объяснение ошибок и решений, сводки по уязвимостям и лицензиям, migration assistant, разбор намерения и ответы A2A-агента |
| Эмбеддинги | embedder (размерность 1024, раздел 9) | OpenAI-совместимый HTTP | Индексация описаний и README, семантический поиск, поиск дубликатов |
| Rerank | reranker | rerank endpoint | Переранжирование top 50 для запросов на естественном языке (10.10) |

-   **Заземление.** LLM получает только `GroundedPrompt`: контекст отфильтрован ACL пользователя и очищен от секретов; ответ содержит ссылки на источники, ответ без источников помечается как непроверенный.
-   **Границы.** LLM не выдаёт доступ, не принимает решений о promotion и не меняет состояние. Изменение совершается только отдельным явным действием principal (см. A2A-агент ниже).
-   **Маршрутизация и деградация.** Для каждой роли задан упорядоченный список провайдеров с circuit breaker; при отказе всех генеративных моделей поиск работает лексически, а скоринг продолжает работать, потому что не зависит от LLM.

### Конфигурация и наблюдаемость

Провайдеры задаются по ролям (`score`, `chat`, `embed`, `rerank`), а не по именам моделей: endpoint, модель, таймаут, лимиты и ключ (из Secret или файла 0400, никогда в логах и аудите). Метрики по роли и модели: latency, ошибки, токены, доля fallback, cache hit эмбеддингов, распределение оценок и доля переопределений.

## A2A-агент CRANE

CRANE сам предоставляет агента по протоколу A2A (Agent2Agent). Внешние агенты (ассистенты организации, агенты команд, CI-боты) обращаются к каталогу пакетов через единый диалоговый интерфейс вместо набора REST-вызовов. Агент работает внутри роли `serve` как входной адаптер ядра и не имеет собственного хранилища и обходных путей: все действия идут через те же use cases, что и REST `/api/v1`.

### Интерфейс

| Элемент | Значение |
| --- | --- |
| Agent Card | `GET /.well-known/agent-card.json`: имя, версия, URL, поддерживаемые навыки (skills), схемы аутентификации, capabilities (`streaming: true`, `pushNotifications: false` в P3) |
| Endpoint | `POST /a2a` (JSON-RPC 2.0): `message/send`, `message/stream` (SSE), `tasks/get`, `tasks/cancel` |
| Аутентификация | `Authorization: Bearer`: JWT Keycloak (агент действует от имени пользователя, OAuth2 token exchange) или robot credential `crci_` с ограниченными scopes |
| Идентичность | Каждый вызов выполняется как principal (пользователь или робот). Права агента не шире прав principal: Cedar оценивает тот же запрос, что и для REST |
| Scopes | `a2a:read` для read-навыков, `a2a:write` для write-навыков; без `a2a:write` write-навыки не показываются в Agent Card для этого вызывающего и отклоняются |
| Ограничения | Rate limit GCRA отдельно для read и write; размер сообщения; число одновременных задач на principal |

### Навыки: чтение (`a2a:read`)

| Навык | Что делает | Соответствие REST |
| --- | --- | --- |
| `search_packages` | Лексический и семантический поиск по каталогу с ACL-фильтром | `GET /api/v1/search` |
| `get_package`, `get_version` | Карточка пакета и версии: владельцы, состояние, лицензия, evidence | `GET /api/v1/packages/...` |
| `list_findings` | CVE и лицензионные findings версии: приоритет, оценки systemone с факторами, уровень достоверности, исключения, VEX | `GET /api/v1/findings` |
| `impact_analysis` | Потребители уязвимой или deprecated версии, уровни достоверности | `GET /api/v1/impact` |
| `explain_decision` | Объяснение отказа или результата promotion: сработавшие правила Cedar, remediation | `diag <request-id>` |
| `check_readiness` | Готовность lockfile или SBOM к офлайну: что в кэше, что отсутствует | `POST /api/v1/readiness` |

### Навыки: запись (`a2a:write`)

Запись ограничена обратимыми действиями управления метаданными. Операции с байтами и решения безопасности остаются за человеком.

| Навык | Что делает | Ограничения |
| --- | --- | --- |
| `deprecate_version` | Помечает версию deprecated, указывает замену | Только владелец пакета; обратимо |
| `request_approval` | Создаёт запрос approval у security reviewer | Сам approval агент выдать не может |
| `create_exception` | Создаёт исключение для finding | Обязательны TTL и обоснование; максимальный срок задаёт политика; требуется step-up |
| `create_issue` | Создаёт issue в GitLab владельцу по finding | Дедупликация по CVE и пакету |
| `subscribe_webhook` | Подписывает указанный URL на события | Только allowlist хостов; секрет показывается один раз |

Не поддерживаются через агента: публикация, удаление и перезапись версий, promotion в release, изменение политик, лицензий и ролей, управление хранилищем и retention.

### Модель подтверждения записи

1.  **Предложение.** Агент разбирает запрос (LLM помогает извлечь намерение), формирует точный план действия: операция, ресурс, параметры, ожидаемый эффект. Задача переходит в состояние `input-required`; ничего не изменено.
2.  **Подтверждение.** Действие выполняется только после явного подтверждения principal: только кнопка в UI CRANE (cookie BFF). Повторный вызов агентом с `confirmation_id` план не подтверждает: иначе агент, ведомый текстом README, подтвердил бы свой же план. Подтверждение привязано к хэшу плана, действует 10 минут и одноразово; для `create_exception` дополнительно требуется step-up Keycloak.
3.  **Исполнение.** Use case вызывается с `Idempotency-Key` = `confirmation_id`. Задача завершается в `completed` с ссылкой на созданный объект либо в `failed` с `problem+json` (`rule_id`, `request_id`, `remediation`).
4.  **Аудит.** Запись в журнал содержит actor = principal, канал `a2a`, идентификатор вызывающего агента, `task_id`, хэш плана и решение Cedar. Событие попадает в hash chain и SIEM так же, как действие из UI.

Текст сообщений вызывающего агента, README и metadata пакетов считаются недоверенными данными: они не могут расширить план действия и не могут подтвердить запись.

### Ошибки и деградация

| Ситуация | Поведение |
| --- | --- |
| Нет права или scope | Задача `rejected` с объяснением из policy trace, без утечки существования ресурса |
| LLM недоступны | read-навыки возвращают структурированный результат без текста; write-навыки возвращают `failed` с причиной «планирование недоступно», прямые REST-вызовы работают |
| Повтор подтверждения | Идемпотентный ответ с прежним результатом |
| Истёк `confirmation_id` | Новое предложение; изменения не применяются |

## Каталог событий webhooks

| Событие | Когда |
| --- | --- |
| `version.published`, `version.promoted`, `version.blocked`, `version.deleted` | Жизненный цикл версии |
| `security.finding.created`, `security.finding.resolved`, `security.finding.priority_changed` | Новые, закрытые и переприоритизированные CVE |
| `security.advisory.emergency`, `security.emergency_block.applied`, `security.emergency_block.lifted` | Экстренный режим: KEV, вредоносный пакет, zero-day |
| `security.vex.created`, `security.sla.breached` | VEX-заявление о неприменимости; нарушение SLA устранения |
| `feed.lagging`, `feed.recovered` | Отставание и восстановление фида уязвимостей |
| `license.violation`, `license.review_required` | Решения лицензионной политики |
| `exception.expiring`, `exception.expired` | Исключения с TTL |
| `credential.expiring`, `credential.revoked` | Роботы и токены |
| `quota.threshold`, `upstream.unavailable` | Эксплуатация |

## Готовые CI templates

```yaml
# .gitlab-ci.yml — include: { project: platform/crane-ci, file: npm.yml }
build:
  extends: .crane-npm-build        # npm ci --ignore-scripts, сборка; id_tokens нет, токена записи нет
publish:
  extends: .crane-npm-publish      # без установки зависимостей: берёт артефакт build
  needs: [build]
  id_tokens:
    CRANE_ID_TOKEN: { aud: crane }
  rules: [{ if: $CI_COMMIT_TAG }]  # защищённые теги
# publish обменивает ID token на crci_ (scope publish, только имена из trust policy, 5 мин),
# пишет .npmrc без секрета в логах, публикует, прикладывает SBOM (CycloneDX) и provenance
```

Почему так:

-   `id_tokens` в GitLab задаются на job, поэтому токен записи существует только в job публикации, где не исполняется чужой код.
-   Зависимости ставятся с `--ignore-scripts` (pip — `--only-binary :all:` где возможно, cargo — build-скрипты исполняются в build-job без токена); publish-job их не ставит.
-   Условия записи (`ref_protected`, защищённые теги) и содержимое provenance — 10.5.
