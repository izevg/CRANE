//! Консоль над `Settings` на памяти с известным токеном и схемой из нескольких ключей.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use axum::Router;
use axum::http::Method;
use crane_app::settings::Settings;
use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::audit::AuditEvent;
use crane_domain::settings::{
    Kind, NewValue, SettingDef, Stored, StoredValue, Unit, Unreadable, Version,
};
use crane_http::{Section, StaticToken, TrustedProxies, console_router};
use crane_memory::{ManualClock, MemoryAudit, MemorySettings};
use crane_ports::settings::{
    Inherited, Invalid, InvalidKey, SettingsError, SettingsSchema, SettingsStore,
};
use http_body_util::BodyExt;
use tokio::runtime::{Builder, Runtime};
use tower::ServiceExt;

use crate::common::{Reply, TOKEN, request};

pub(crate) const PRINCIPAL: &str = "crane";
/// Значение, которое тестовая схема считает непроходящим разбор секции.
pub(crate) const BAD_VALUE: &str = "bad";
pub(crate) const ENV_KEY: &str = "http_port";
pub(crate) const UNREADABLE_KEY: &str = "registry_token";
/// Секрет из окружения; схема-двойник нарушает контракт и отдаёт его значение, консоль обязана его скрыть.
pub(crate) const ENV_SECRET_KEY: &str = "env_secret";
pub(crate) const ENV_SECRET: &str = "leak-me";
pub(crate) const ENV_SECRET_LOCATOR: &str = "valkey://env.internal:6379";
const ROW_TIME: Duration = Duration::from_secs(1_700_000_000);

const fn def(
    key: &'static str,
    env_var: &'static str,
    group: &'static str,
    kind: Kind,
    unit: Option<Unit>,
) -> SettingDef {
    SettingDef {
        key,
        env_var,
        group,
        kind,
        unit,
        console: true,
    }
}

const DEFS: [SettingDef; 6] = [
    def("blobs", "CRANE_BLOBS", "storage", Kind::Text, None),
    def(
        "max_publish_bytes",
        "CRANE_MAX_PUBLISH_BYTES",
        "limits",
        Kind::Number,
        Some(Unit::Bytes),
    ),
    def(
        "s3_secret",
        "CRANE_S3_SECRET",
        "storage",
        Kind::Secret,
        None,
    ),
    def(
        UNREADABLE_KEY,
        "CRANE_REGISTRY_TOKEN",
        "token",
        Kind::Secret,
        None,
    ),
    def(ENV_KEY, "CRANE_HTTP_PORT", "network", Kind::Number, None),
    def(
        ENV_SECRET_KEY,
        "CRANE_ENV_SECRET",
        "token",
        Kind::Secret,
        None,
    ),
];

/// Схема, в которой `http_port` задан окружением, а значение `bad` не проходит разбор.
struct Schema;

impl SettingsSchema for Schema {
    fn definitions(&self) -> &[SettingDef] {
        &DEFS
    }

    fn inherited(&self, key: &str) -> Option<Inherited> {
        match key {
            ENV_KEY => Some(Inherited {
                var: "CRANE_HTTP_PORT".to_owned(),
                value: Some("8080".to_owned()),
                locator: None,
            }),
            ENV_SECRET_KEY => Some(Inherited {
                var: "CRANE_ENV_SECRET".to_owned(),
                value: Some(ENV_SECRET.to_owned()),
                locator: Some(ENV_SECRET_LOCATOR.to_owned()),
            }),
            _ => None,
        }
    }

    fn check(
        &self,
        _group: &str,
        _stored: &BTreeMap<String, Stored>,
        changes: &[(&str, Option<&NewValue>)],
    ) -> Result<(), Invalid> {
        let errors: Vec<InvalidKey> = changes
            .iter()
            .filter(|(_, value)| matches!(value, Some(NewValue::Plain(text)) if text == BAD_VALUE))
            .map(|(key, _)| InvalidKey {
                key: (*key).to_owned(),
                detail: "значение не разобрано".to_owned(),
            })
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Invalid { errors })
        }
    }
}

/// Память плюс то, чего память не умеет: нечитаемая строка секрета и «хранилище недоступно».
struct Store {
    inner: Arc<MemorySettings>,
    down: Arc<AtomicBool>,
    unreadable: bool,
}

impl Store {
    fn guard(&self) -> Result<(), SettingsError> {
        if self.down.load(Ordering::SeqCst) {
            Err(SettingsError::Unreachable(Box::new(std::io::Error::other(
                "db.internal:5432 refused",
            ))))
        } else {
            Ok(())
        }
    }
}

impl SettingsStore for Store {
    async fn list(&self) -> Result<BTreeMap<String, Stored>, SettingsError> {
        self.guard()?;
        let mut rows = self.inner.list().await?;
        if self.unreadable {
            rows.insert(
                UNREADABLE_KEY.to_owned(),
                Stored {
                    value: StoredValue::Unreadable(Unreadable::WrongKey {
                        stored_key_id: "k-old".to_owned(),
                        current_key_id: "k-new".to_owned(),
                    }),
                    version: Version::FIRST,
                    updated_at: SystemTime::UNIX_EPOCH + ROW_TIME,
                    updated_by: "admin".to_owned(),
                },
            );
        }
        Ok(rows)
    }

    async fn put(
        &self,
        key: &str,
        value: NewValue,
        expected: Option<Version>,
        by: &Principal,
    ) -> Result<Version, SettingsError> {
        self.guard()?;
        self.inner.put(key, value, expected, by).await
    }

    async fn delete(&self, key: &str, expected: Version) -> Result<(), SettingsError> {
        self.guard()?;
        self.inner.delete(key, expected).await
    }
}

pub(crate) struct Console {
    runtime: Runtime,
    store: Arc<MemorySettings>,
    router: Router,
    audit: Arc<MemoryAudit>,
    down: Arc<AtomicBool>,
}

impl Console {
    pub(crate) fn new() -> Self {
        Self::build(true, Vec::new(), false)
    }

    /// Режим memory: писать некуда.
    pub(crate) fn read_only() -> Self {
        Self::build(false, Vec::new(), false)
    }

    pub(crate) fn with_sections(sections: Vec<Section>) -> Self {
        Self::build(true, sections, false)
    }

    /// В хранилище лежит строка ключа, который перекрыт окружением (`ENV_KEY`).
    pub(crate) fn with_shadowed_row() -> Self {
        let console = Self::build(true, Vec::new(), false);
        let by = Principal {
            id: "admin".to_owned(),
            kind: PrincipalKind::Human,
        };
        console
            .runtime
            .block_on(
                console
                    .store
                    .put(ENV_KEY, NewValue::Plain("9090".to_owned()), None, &by),
            )
            .unwrap();
        console
    }

    /// В хранилище лежит секрет, зашифрованный другим ключом.
    pub(crate) fn with_unreadable_secret() -> Self {
        Self::build(true, Vec::new(), true)
    }

    fn build(persistent: bool, sections: Vec<Section>, unreadable: bool) -> Self {
        let audit = Arc::new(MemoryAudit::default());
        let down = Arc::new(AtomicBool::new(false));
        let clock = Arc::new(ManualClock::new(SystemTime::UNIX_EPOCH + ROW_TIME));
        let memory = Arc::new(MemorySettings::new(clock));
        let store = Store {
            inner: memory.clone(),
            down: down.clone(),
            unreadable,
        };
        let settings = Arc::new(Settings::new(
            store,
            Arc::new(Schema),
            audit.clone(),
            persistent,
            BTreeMap::new(),
        ));
        let token = StaticToken::new(
            TOKEN.as_bytes(),
            Principal {
                id: PRINCIPAL.to_owned(),
                kind: PrincipalKind::Robot,
            },
        );
        let router = console_router(
            settings,
            persistent,
            sections,
            token,
            audit.clone(),
            TrustedProxies::default(),
        );
        Self {
            store: memory,
            runtime: Builder::new_current_thread().enable_all().build().unwrap(),
            router,
            audit,
            down,
        }
    }

    /// Хранилище настроек перестаёт отвечать.
    pub(crate) fn store_down(&self) {
        self.down.store(true, Ordering::SeqCst);
    }

    pub(crate) fn audit_events(&self) -> Vec<AuditEvent> {
        self.audit.events()
    }

    pub(crate) fn call(
        &self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Reply {
        let request = request(method, uri, headers, body.as_bytes().to_vec(), None);
        self.runtime.block_on(async {
            let response = self.router.clone().oneshot(request).await.unwrap();
            let (parts, body) = response.into_parts();
            Reply {
                status: parts.status,
                headers: parts.headers,
                body: body.collect().await.unwrap().to_bytes(),
            }
        })
    }

    /// Запрос с верным токеном.
    pub(crate) fn authed(
        &self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Reply {
        let bearer = format!("Bearer {TOKEN}");
        let mut all = vec![("authorization", bearer.as_str())];
        all.extend_from_slice(headers);
        self.call(method, uri, &all, body)
    }

    pub(crate) fn get(&self, uri: &str) -> Reply {
        self.authed(Method::GET, uri, &[], "")
    }

    /// Создание (`If-None-Match: *`).
    pub(crate) fn create(&self, key: &str, value: &str) -> Reply {
        self.put(key, &[("if-none-match", "*")], value)
    }

    pub(crate) fn put(&self, key: &str, conditions: &[(&str, &str)], value: &str) -> Reply {
        let body = serde_json::json!({"value": value}).to_string();
        self.authed(
            Method::PUT,
            &format!("/api/v1/settings/{key}"),
            conditions,
            &body,
        )
    }
}
