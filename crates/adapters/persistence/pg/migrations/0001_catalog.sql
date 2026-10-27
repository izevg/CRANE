-- 0001, фаза expand: исходная схема каталога. Таблицы пусты, поэтому обычный CREATE INDEX
-- допустим; в последующих миграциях индексы на непустых таблицах строятся только CONCURRENTLY
-- (см. README.md). Расчёт на профиль L: 1-5 млн пакетов, десятки млн версий и файлов.

CREATE TYPE pkg_format AS ENUM ('maven', 'npm', 'pypi', 'nuget', 'cargo');
CREATE TYPE repo_kind AS ENUM ('hosted', 'proxy');
CREATE TYPE ver_state AS ENUM ('quarantined', 'released', 'blocked', 'deleted');

-- Обслуживает: ensure_repository, find_repository (поиск по (format, name)).
CREATE TABLE repository (
    id     uuid       PRIMARY KEY,
    format pkg_format NOT NULL,
    kind   repo_kind  NOT NULL,
    name   text       NOT NULL,
    CONSTRAINT repository_format_name_key UNIQUE (format, name)
);

-- Внутренний bigint-ключ делает индексы и FK версий и файлов вдвое уже, чем uuid;
-- за пределы адаптера он не выходит. Обслуживает: publish_version, versions, find_version
-- (уникальный индекс по (repository_id, name_norm) - единственный путь к пакету).
CREATE TABLE package (
    id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    repository_id uuid   NOT NULL,
    name_norm     text   NOT NULL,
    -- Первое написание имени: его возвращают клиентам.
    name_display  text   NOT NULL,
    -- Имя ограничения читает адаптер: нарушение этого FK означает «нет репозитория».
    CONSTRAINT package_repository_id_fkey FOREIGN KEY (repository_id) REFERENCES repository (id),
    CONSTRAINT package_repository_id_name_norm_key UNIQUE (repository_id, name_norm)
);

-- Самая большая таблица. state и yanked не проиндексированы и меняются на месте, поэтому
-- fillfactor 90 оставляет место под HOT-обновления. Отдельного индекса под versions() нет:
-- диапазон по префиксу уникального индекса (package_id, version) читает версии одного пакета,
-- сортировка по (published_at, id) идёт над их небольшим числом.
-- Обслуживает: publish_version, versions, find_version, set_yanked и transition (по id).
CREATE TABLE package_version (
    id              uuid        PRIMARY KEY,
    package_id      bigint      NOT NULL REFERENCES package (id),
    -- Version::canonical(): ключ уникальности.
    version         text        NOT NULL,
    -- Version::as_written(): отдаётся клиентам как есть.
    version_display text        NOT NULL,
    state           ver_state   NOT NULL,
    yanked          boolean     NOT NULL DEFAULT false,
    published_at    timestamptz NOT NULL,
    -- json, а не jsonb: jsonb переписывает текст, а метаданные формата - непрозрачные байты
    -- (индекс cargo, ETag), которые обязаны вернуться побайтно.
    metadata        json        NOT NULL,
    CONSTRAINT package_version_package_id_version_key UNIQUE (package_id, version)
) WITH (fillfactor = 90);

-- Содержимое CAS по (хранилище, sha256). Обслуживает: publish_version (вставка, размер для
-- load_version через JOIN). store_id задаёт адаптер явно: значения по умолчанию нет.
CREATE TABLE blob (
    store_id   text        NOT NULL,
    sha256     bytea       NOT NULL CHECK (octet_length(sha256) = 32),
    size       bigint      NOT NULL CHECK (size >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (store_id, sha256)
);

-- Файлы версии. Обслуживает: load_version (префикс первичного ключа version_id) и
-- publish_version. Индекса по (store_id, blob_sha256) нет: он нужен только сборке мусора
-- и будет добавлен вместе с ней отдельной миграцией CREATE INDEX CONCURRENTLY.
CREATE TABLE asset (
    version_id  uuid  NOT NULL REFERENCES package_version (id),
    path        text  NOT NULL,
    store_id    text  NOT NULL,
    blob_sha256 bytea NOT NULL,
    PRIMARY KEY (version_id, path),
    FOREIGN KEY (store_id, blob_sha256) REFERENCES blob (store_id, sha256)
);

-- Не в этом срезе (добавляются позже как nullable expand): namespace (v0.2.0), workspace_id,
-- upstream_url, config, purl, sort_key, mutable, published_by, source, sha512/sha1/md5,
-- media_type, таблица blob_store.
