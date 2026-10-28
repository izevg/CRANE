-- 0002, фаза expand: устаревание версий и dist-теги (npm). Колонка nullable и без default,
-- поэтому ALTER не переписывает package_version; таблица тегов новая и пуста.

-- NULL - версия не устарела, иначе сообщение deprecate. Обслуживает: load_version/versions/
-- find_version (читают колонку вместе с версией) и set_deprecated (по id).
ALTER TABLE package_version ADD COLUMN deprecated text;

-- Тег указывает на версию того же пакета: package_id хранится рядом, а set_tag выводит его
-- из версии, так что тег чужого пакета не выразить. Обслуживает: tags (диапазон по префиксу
-- первичного ключа package_id), set_tag и publish_version (upsert по (package_id, tag)),
-- remove_tag (удаление по первичному ключу). Индекса по version_id нет: версии не удаляются
-- физически, а обратного поиска «теги версии» нет ни в одном запросе.
CREATE TABLE dist_tag (
    package_id bigint NOT NULL REFERENCES package (id),
    tag        text   NOT NULL,
    version_id uuid   NOT NULL REFERENCES package_version (id),
    PRIMARY KEY (package_id, tag)
);
