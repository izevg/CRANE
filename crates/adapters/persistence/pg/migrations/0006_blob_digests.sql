-- 0006: blob хранит все дайджесты содержимого, а не только sha256. Контрольные суммы
-- протоколов клиентов (Maven .sha1/.md5/.sha256/.sha512, npm и NuGet sha512) отдаются
-- строкой каталога без чтения самого blob; значения считаются один раз при записи потоком.
--
-- Отклонение от README (NOT NULL без backfill): постоянных установок нет, а восстановить
-- дайджесты старых строк можно только перечитав содержимое CAS, чего SQL не умеет. Поэтому
-- миграция только на пустой blob и падает на непустом: dev-БД до v0.3.0 пересоздаётся.
-- Решение принято владельцем проекта (как в 0005); кода, читающего эти колонки
-- как nullable, нет.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM blob) THEN
        RAISE EXCEPTION '0006: в blob уже есть строки; это dev-БД до v0.3.0 — пересоздайте её (docker compose --project-directory . -f deploy/compose/docker-compose.dev.yaml down -v)';
    END IF;
END
$$;

-- Обслуживает: чтение версии (load_version, find_version, versions) собирает Digests
-- файла из blob; publish_version и add_asset пишут все колонки.
ALTER TABLE blob
    ADD COLUMN sha512 bytea NOT NULL CHECK (octet_length(sha512) = 64),
    ADD COLUMN sha1   bytea NOT NULL CHECK (octet_length(sha1) = 20),
    ADD COLUMN md5    bytea NOT NULL CHECK (octet_length(md5) = 16);
