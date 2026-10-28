-- 0005: порядок версий и признак pre-release хранятся фактами, а не выводятся из строки.
-- Раньше каталог собирал версию заново из version_display через грамматику SemVer в domain;
-- теперь грамматики живут в formats за трейтом Ecosystem, и адаптер хранилища их не знает.
-- Нумерация с 0005: 0004 занята миграцией параллельного потока (её может не быть в этой ветке).
--
-- Отклонение от README (NOT NULL без backfill): postgres-каталог в v0.2.0 ни разу не стоял
-- на постоянной установке, а старые строки заполнить нечем без грамматики экосистемы
-- в SQL. Поэтому миграция только на пустой package_version и падает на непустой: dev-БД до
-- v0.3.0 пересоздаётся. Решение принято владельцем проекта; нового кода, читающего эти
-- колонки как nullable, нет.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM package_version) THEN
        RAISE EXCEPTION '0005: в package_version уже есть строки; это dev-БД до v0.3.0 — пересоздайте её (docker compose ... down -v)';
    END IF;
END
$$;

-- Version::order_key(): байты OrderKey, сравниваются побайтно. Обслуживает: чтение версий
-- (load/list собирают Version::from_parts) и будущий ORDER BY.
-- Version::is_prerelease(): отдельная колонка, чтобы фильтровать pre-release без разбора ключа.
ALTER TABLE package_version
    ADD COLUMN version_order bytea   NOT NULL,
    ADD COLUMN prerelease    boolean NOT NULL;
