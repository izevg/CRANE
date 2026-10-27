@AGENTS.md

## Claude Code

- Все правила проекта — в `AGENTS.md` (импортируется выше). Здесь только то, что специфично для Claude Code.
- Атрибуция в коммитах и PR выключена на уровне проекта (`.claude/settings.json`: `attribution.commit` и `attribution.pr` пустые). Не добавляйте `Co-Authored-By` и «Generated with» вручную.
- Перед коммитом прогоняйте проверки из таблицы команд; хуки `lefthook` выполнятся сами, не обходите их.
