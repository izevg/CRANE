-- 0003, фаза expand: namespace - зарезервированные паттерны имён. Новая таблица, поэтому
-- обычные индексы допустимы. Нумерация с 0003: 0002 занята миграцией ветки npm.
-- Защита от dependency confusion опирается и на строки `package` hosted-репозиториев
-- (они не удаляются никогда), но отдельного индекса под это не нужно: `hosted_exists`
-- идёт по уникальному (repository_id, name_norm) из 0001.

-- Обслуживает: namespace_for (поиск по (format, pattern) из списка покрывающих паттернов),
-- ensure_namespace.
CREATE TABLE namespace (
    -- Домен id не знает: он нужен API v0.4.0 (`/namespaces/{id}`). Колонка создаётся сейчас
    -- и заполняется базой: добавить uuid в непустую таблицу позже нельзя (README: без
    -- volatile default в ADD COLUMN). gen_random_uuid() встроен в PostgreSQL >= 13.
    id           uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Владелец - workspace. Таблицы workspace до v0.4.0 нет, поэтому FK добавит её миграция;
    -- до тех пор все строки принадлежат WorkspaceId::DEFAULT.
    workspace_id uuid        NOT NULL,
    format       pkg_format  NOT NULL,
    -- Нормализован правилами формата; префикс оканчивается разделителем и '*'.
    pattern      text        NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT namespace_format_pattern_key UNIQUE (format, pattern)
);
