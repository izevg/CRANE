-- 0004, фаза expand: setting (значения, заданные в консоли) и onboarding (состояние мастера
-- первого запуска). Новые таблицы, поэтому обычные индексы допустимы. Нумерация закреплена
-- за потоком настроек: 0005 и 0006 заняты потоком форматов.
-- Время записи ставит адаптер из Clock, а не DEFAULT now(): так контракт с ручными часами
-- одинаков для memory и pg.

-- Обслуживает: list (полный обход, ORDER BY key по первичному ключу), put и delete (CAS по
-- key и version). Секрет лежит только в value_sealed: nonce || шифртекст || тег (AES-256-GCM),
-- key_id - отпечаток ключа, которым он запечатан.
CREATE TABLE setting (
    key          text        PRIMARY KEY CHECK (key ~ '^[a-z][a-z0-9_]*$' AND length(key) <= 96),
    value_text   text,
    value_sealed bytea,
    key_id       text,
    version      bigint      NOT NULL CHECK (version >= 1),
    updated_at   timestamptz NOT NULL,
    updated_by   text        NOT NULL,
    CONSTRAINT setting_value_xor CHECK ((value_text IS NULL) <> (value_sealed IS NULL)),
    CONSTRAINT setting_sealed_key CHECK ((value_sealed IS NULL) = (key_id IS NULL))
);

-- Обслуживает: get, begin (INSERT ON CONFLICT по singleton), rotate, reset, complete.
-- Одна строка на установку; «завершён, но с токеном» CHECK-ом непредставимо.
CREATE TABLE onboarding (
    singleton        boolean     PRIMARY KEY DEFAULT true CHECK (singleton),
    token_hash       bytea CHECK (octet_length(token_hash) = 32),
    token_expires_at timestamptz,
    completed_at     timestamptz,
    completed_by     text,
    CONSTRAINT onboarding_state CHECK (
           (completed_at IS NULL AND token_hash IS NOT NULL AND token_expires_at IS NOT NULL AND completed_by IS NULL)
        OR (completed_at IS NOT NULL AND token_hash IS NULL AND token_expires_at IS NULL AND completed_by IS NOT NULL))
);
