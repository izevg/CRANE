# Технический долг

Закрываем после того, как проект станет публичным (минимум — полное ядро по плану).

| # | Долг | Условие | Что сделать |
| --- | --- | --- | --- |
| 1 | Защита веток `main` и `release/**` | Публичный репозиторий или GitHub Pro | Rulesets: PR обязателен, `main` — только merge commit, `release/**` — только squash, required checks (`branch-flow`, `title`, `structure`, `rust`, `web`, плюс `gate-mark` для `release/**`), запрет force-push и удаления; GitHub App `crane-release` в bypass для `main`. Файлы в `.github/rulesets/` и задача `task gh:rulesets`. |
| 2 | GitHub App `crane-release` | Оплата Actions | Создать App, положить `RELEASE_APP_ID` (variable) и `RELEASE_APP_PRIVATE_KEY` (secret); проверить `release` и `publish` на реальном релизе. |
| 3 | GitHub Release для `v0.0.0` | Оплата Actions | Перезапустить workflow `Publish` для тега или создать релиз вручную. |
| 4 | Storybook и Playwright | Первый UI | Настроить, подключить `e2e` в `task gate` и Storybook lint/tests в CI на PR в `main`. |
| 5 | Сборка гайдбука | Появится `../guidebook-render` | Собрать PDF после правок разделов 2, 8, 13, 15–19. `guidebook/` вне git. |
