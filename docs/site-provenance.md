# Происхождение фактов публичной документации

Реестр для сайта getcrane.dev (публичный репозиторий `getcrane.dev`, сабмодуль `docs/site`). Сам сайт не содержит ссылок на внутренние разделы, решения и код; здесь записано, на чём держится каждая страница, какие расхождения продукта найдены при написании и какие вопросы открыты. Ведёт руководитель документации; файл живёт только в этом репозитории.

Порядок доверия к источникам: код и тесты, решения раздела 19, `AGENTS.md` и `GITFLOW.md`, `docs/api/*`, `docs/ui/*`, остальные разделы `docs/product/`. При конфликте в текст сайта идёт подтверждённое кодом, расхождение — сюда.

## Страницы и источники

Пути страниц — от корня языка (`content/ru/…` и `content/en/…`, структура 1:1). `target` — целевое состояние (Features, концепции), `current` — текущее состояние кода.

| Страница | Состояние | Источники |
| --- | --- | --- |
| `start/_index` | target | `README.md`, разделы 01, 02 (границы), 05, 13 (OIDC), 14.12; `LICENSE` |
| `start/quickstart` | current | `deploy/compose/docker-compose.yaml`, `.env.dist`, `deploy/docker/Dockerfile`, `Taskfile.yml` (`secrets:init`), `tools/secrets-init.ts`, `tools/dogfood/*` (команды cargo и npm, `proxy.ts`), `crates/formats/cargo` (`render_config`, `repo_url`), `crates/entry/http` (401/403) |
| `start/concepts` | target | 09 (жизненный цикл версии), 10.1–10.3, 13, 14.15, 99 |
| `start/first-run` | current, в разработке | 20.4–20.5; в `crates/entry/http` и `web/src/pages` маршрутов и экрана мастера нет |
| `features/_index` | target | 01, 02 (P1, P5), 14, 16.4 |
| `features/security/_index` | target | 14.1–14.3, 02 (P3–P5), решения 17, 46; `LICENSE` п. 7 (смысл слова «гарантия») |
| `features/security/supply-chain-threats` | target | 14.2, 11.5 |
| `features/security/immutable-versions` | target | 09 (жизненный цикл, семантика по форматам), 10.1, решения 50, 51 |
| `features/security/digest-verification` | target | 09 (blob), 10.1, 10.2, 10.6.1, 07 (выборочная проверка), 16.1 (остаточный риск Valkey), 16.6, решение 52; `crates/entry/http/src/error/status.rs` (502) |
| `features/security/namespace-reservation` | target | 10.3, решения 28, 42, 43 |
| `features/security/upstream-quarantine` | target | 10.2 (видимость proxy), 14.2, 14.8 (блокировки, break-glass, снятие), 21.7, 21.9 (окно 24 ч, `unknown_age = release`), решения 25, 27, 62 |
| `features/security/egress-guard` | target | 14.2, 14.3, 10.2; `crates/adapters/external/upstream/src/guard.rs` (`MAX_REDIRECTS = 5`, диапазоны адресов), `lib.rs` (`record_egress`, поле `failure`) |
| `features/security/public-upstreams-only` | target | Утверждение владельца от 29.10.2026 (см. ниже); `crates/formats/cargo/src/proxy/mod.rs:13` (`INDEX_HOST`), `crates/formats/npm/src/proxy/mod.rs:17` (`REGISTRY_HOST`), `crates/entry/server/src/config/proxy`, `guard.rs`; 10.15 и решение 9 (перенос из Nexus — импорт в hosted) |
| `features/operations/deployment-model` | target | 01, 02 (P1, P5), 04 (режимы деградации), 06, 16.1–16.4, решение 46 |
| `deploy/_index`, `deploy/requirements` | current | `deploy/compose/*`, `.env.dist`, `deploy/docker/Dockerfile`, `crates/adapters/persistence/pg` (расширения), `AGENTS.md` (версия Compose) |
| `deploy/docker-compose` | current | `deploy/compose/docker-compose.yaml`, `deploy/caddy/Caddyfile`, `.env.dist`, `tools/secrets-init.ts`, `crates/entry/server/src/main.rs` (`healthcheck`), `crates/entry/http/src/admin/mod.rs` |
| `deploy/s3-storage` | current | `deploy/compose/docker-compose.s3.yaml`, `crates/adapters/persistence/objstore`, `config/storage` |
| `deploy/valkey-cache` | current | `deploy/compose/docker-compose.cache.yaml`, `crates/adapters/persistence/valkey`, `crates/entry/server/src/config/proxy` |
| `deploy/kubernetes-helm` | current, в разработке | До исправления чарта (расхождения 1–4) страница помечена «в разработке»; полный черновик сохранён в истории ветки `site/initial` сайта |

Страницы второй партии (оставшаяся безопасность, Настройка, Справочники, Управление реестром, Обслуживание) добавляются после приёмки.

## Утверждения владельца, которых ещё нет в разделе 19

| Дата | Утверждение | Где использовано | Что нужно |
| --- | --- | --- | --- |
| 29.10.2026 | CRANE не поддерживает приватные и произвольные реестры в качестве upstream и не будет поддерживать: это канал атаки на цепочку поставки, а ответственность за содержимое, пришедшее автоматически, нести некому. Пакеты из других реестров попадают только через hosted-публикацию человеком или его скриптом, под его ответственность | `features/security/public-upstreams-only`, `features/security/_index` | Записать решением в раздел 19. Сверить с пунктом «импорт из других реестров» в ROADMAP v0.6.0: импорт в hosted не противоречит, подключение upstream противоречит |

## Реестр расхождений продукта

Статус «открыто» означает, что в тексте сайта использовано подтверждённое кодом, а спорное не написано.

| # | Страница | Расхождение | Источник | Статус |
| --- | --- | --- | --- | --- |
| 1 | `deploy/kubernetes-helm` | Шаблон worker запускает роль `worker`, бинарник принимает только `serve`, `all`, `migrate`, `healthcheck` и завершается с кодом 2 | `deploy/helm/crane/templates/worker.yaml:1`; `crates/entry/server/src/main.rs:20,32-41`, тест `anything_but_exactly_one_known_role_is_rejected` | открыто, дефект продукта |
| 2 | `deploy/kubernetes-helm` | `startupProbe` пода api запрашивает `/healthz/startup`, порт admin отдаёт только `/healthz/live` и `/healthz/ready`; поды api не доходят до `Ready` | `templates/api.yaml:6`; `crates/entry/http/src/admin/mod.rs:30-31` | открыто, дефект продукта |
| 3 | `deploy/kubernetes-helm` | `values.yaml` называет `CRANE_CACHE` ключом секрета, но шаблон всегда задаёт его из `config.cache`; адрес Valkey с паролем возможен только открытым текстом в values и манифесте | `values.yaml:12`; `templates/_helpers.tpl:41-42`; `config/source/mod.rs:81-87` | открыто, дефект продукта (безопасность) |
| 4 | `deploy/kubernetes-helm` | `values.yaml` называет обязательной только `CRANE_DATABASE_URL`; код требует ещё `CRANE_PUBLIC_URL`, `CRANE_TOKEN`, `CRANE_BLOBS` | `values.yaml:12-13`; `config/network/mod.rs:67`, `config/mod.rs:267`, `config/storage/blobs.rs:33` | открыто |
| 5 | `deploy/*` | Профиль `scan` в compose запускает роль `analyze`, которой нет в бинарнике; `CRANE_TRIVY_URL` код не читает | `deploy/compose/docker-compose.yaml:86-103`; `main.rs` | открыто, в тексте сайта профиля нет |
| 6 | `deploy/*`, справочники | `CRANE_PACKAGES_URL`, `CRANE_BOOTSTRAP_TOKEN(_FILE)` в таблице 16.1 и `CRANE_REQUIRE_SECURE_CONFIG` в 14.3 и 16.8 код не читает | 16.1, 14.3, 16.8; `crates/entry/server/src/config` | открыто, в текущих страницах не упоминаются |
| 7 | `start/quickstart` | 16.1: образ «публикуется проектом» (`crane/crane:1.0.0`); `.env.dist`: образ собирается локально и не публикуется | 16.1:44; `.env.dist:10` | открыто; в тексте локальная сборка |
| 8 | `deploy/docker-compose` | Комментарий: пути секретов «относительно каталога compose-файла»; фактически от корня репозитория (`--project-directory .`) | `.env.dist:25`; `docker-compose.yaml:4`; `tools/secrets-init.ts` | открыто |
| 9 | `features/security/egress-guard` | 14.3: весь исходящий трафик (фиды, webhooks, SIEM, импорт, модели) через один клиент с SSRF-guard; в коде guard только у адаптера upstream (остальных клиентов ещё нет) | 14.3; `crates/adapters/external/upstream` | открыто; страница целевая |
| 10 | `features/security/egress-guard` | Поле `failure` журнала egress: в 16.6 два значения, в коде ещё `throttled` | 16.6; `upstream/src/lib.rs` (`record_egress`) | открыто; в тексте три значения |
| 11 | `features/security/immutable-versions` | 09 называет `unlisted` флагом NuGet; решение 51 — колонки нет, unlist = `yanked` | 09; решение 51 | открыто; в тексте решение 51 |
| 12 | `features/security/namespace-reservation` | 02: grouped endpoint «заложен в маршрутизации, включается после P3»; решение 28: групповых репозиториев нет | 02 (границы); решение 28 | открыто; в тексте «CRANE не сливает hosted и proxy под одним адресом» |
| 13 | `start/concepts` | Состояние версии `Quarantined` (принята и ждёт проверок) совпадает по названию с карантином upstream | 09; 10.2 | открыто; в тексте разведены явно |
| 14 | `features/operations/deployment-model` | В таблице 16.3 у строки worker два значения на три профиля | 16.3 | открыто; для S указано «в роли `all`» |
| 15 | `start/*` | README: «кода пока нет», `curl …/compose.yaml`, `helm install`, `cctl` | `README.md` | открыто |
| 16 | `features/operations/deployment-model` | На диаграмме 16.1 есть узел backup (pgBackRest, rclone), а 16.5 — готовых профилей нет | 16.1; 16.5 | открыто; узел на сайте не показан |

## Открытые вопросы

| # | Вопрос | Где влияет |
| --- | --- | --- |
| Q1 | Откуда ожидаемый дайджест при proxy для PyPI, Maven и NuGet (в решениях и коде нет; подтверждены Cargo `cksum` и npm `dist.integrity`) | `features/security/digest-verification` |
| Q2 | Проверяются ли attestations upstream для Cargo и NuGet (10.6.1: «не проверяются» с пометкой версии) | `digest-verification` |
| Q3 | Исходящий HTTP(S)_PROXY: клиент upstream вызывает `.no_proxy()`, 10.2 называет это ограничением версии. Постоянное ли это свойство (тогда граница гарантии) | `features/security/egress-guard` |
| Q4 | Критерий slopsquatting «пакет без истории (возраст, скачивания, maintainer) попадает в quarantine» шире окна по времени публикации. Есть ли отдельный механизм | `features/security/supply-chain-threats` |
| Q5 | Публичный адрес исходников и опубликованный образ: без них quickstart не воспроизводится (в тексте плейсхолдер `<repository-url>` и локальная сборка). Публикация образа и репозитория — условие публикации сайта | `start/quickstart`, `deploy/docker-compose` |
| Q6 | Минимальные версии `cargo` и `npm` для `credential-provider` и токена; минимальные версии Kubernetes и Helm | `start/quickstart`, `deploy/requirements` |
| Q7 | Перенос существующих blob между локальным томом и S3 не описан ни в коде, ни в тексте | `deploy/s3-storage` |
