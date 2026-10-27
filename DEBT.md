# Технический долг

Закрываем после того, как репозиторий станет публичным (минимум — полное ядро по плану, веха v0.5.0 в [ROADMAP.md](ROADMAP.md)).

| # | Долг | Условие | Что сделать |
| --- | --- | --- | --- |
| 1 | Защита веток `main` и `release/**` | Репозиторий публичный | Rulesets: PR обязателен, `main` — только merge commit, `release/**` — только squash, required checks (`branch-flow`, `title`, `structure`, `rust`, `web`, плюс `gate-mark` для `main`), запрет force-push и удаления; GitHub App `crane-release` в bypass для `main`. Файлы в `.github/rulesets/` и задача `task gh:rulesets`. |
| 2 | Автовыпуск версий | Включена защита веток | Создать GitHub App `crane-release`, положить `RELEASE_APP_ID` (variable) и `RELEASE_APP_PRIVATE_KEY` (secret); проверить `release` и `publish` на реальном релизе; опубликовать GitHub Release для `v0.0.0`. |
| 3 | Storybook и Playwright | Первый UI (v0.4.0) | Настроить, подключить `e2e` в `task gate` и Storybook lint/tests в CI на PR в `main`. |
| 4 | Сборка гайдбука | Появится `../guidebook-render` | Собрать PDF после правок разделов 2, 8, 13, 15–19. `guidebook/` вне git. |
| 5 | Sweeper staging-объектов на локальной ФС | Адаптер `BlobStore` на локальной ФС (v0.1.0) | В S3 брошенные `staging/<uuid>` убирает lifecycle по префиксу и возрасту; на файловой системе чистить некому. Добавить периодическую задачу: удалять `staging/*` старше 24 ч; покрыть тестом на возраст и на то, что закоммиченные blob не затрагиваются. |
