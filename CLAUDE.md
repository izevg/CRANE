@AGENTS.md

## Claude Code

- Все правила проекта — в `AGENTS.md` (импортируется выше). Здесь только то, что специфично для Claude Code.
- Атрибуция в коммитах и PR выключена на уровне проекта (`.claude/settings.json`: `attribution.commit` и `attribution.pr` пустые). Не добавляйте `Co-Authored-By` и «Generated with» вручную.
- Вехи `ROADMAP.md` обязательны (см. «Дорожная карта» в `AGENTS.md`). Работу ведите в worktree: инструмент `EnterWorktree` (каталог `.claude/worktrees/`) или `claude --worktree`; не правьте файлы в основном каталоге.
- Перед коммитом прогоняйте проверки из таблицы команд; хуки `lefthook` выполнятся сами, не обходите их.
