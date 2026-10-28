//! Единственная таблица «формат → поведение HTTP».
//!
//! Ключ — `Format`: каждая функция ниже содержит исчерпывающий `match` по нему, одна строка на
//! формат. Новый формат — свой модуль, по строке в каждой функции и строка в `REPO_VARS` сервера;
//! `api`, `serve` и `run` не правятся. Строки ещё не реализованных форматов намеренно не склеены
//! через `|`: иначе потоки форматов переписывали бы одну и ту же строку.

#![expect(
    clippy::match_same_arms,
    reason = "строка на формат, а не `A | B | C`: потоки форматов правят разные строки и не конфликтуют"
)]

mod cargo;
mod npm;

use std::sync::Arc;

use axum::Router;
use axum::http::HeaderValue;
use crane_app::proxy::ProxyEngine;
use crane_app::registry::Registry;
use crane_domain::proxy::{EgressPolicy, EgressRefusal, MetadataPolicy, Origin, UpstreamPath};
use crane_domain::registry::{Format, PackageName, RepositoryId};
use crane_format_cargo::{DL_HOST, INDEX, INDEX_HOST, index_path};
use crane_format_npm::{PACKUMENT, REGISTRY_HOST, packument_path};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobStore, Catalog};

use crate::auth::Credential;

const JSON: &str = "application/json";
const BASIC_CHALLENGE: HeaderValue = HeaderValue::from_static("Basic realm=\"crane\"");

/// Настройки маршрутов форматов: общие для всех.
#[derive(Debug, Clone)]
pub struct FormatSettings {
    /// Внешний адрес без завершающего '/': от него строятся ссылки, которые клиент получает в ответах.
    pub public_url: String,
    /// Предел тела публикации: оно буферизуется в памяти целиком. У npm вложение приходит в base64
    /// внутри JSON, то есть на треть больше самого tarball: предел применяется к документу.
    pub max_publish_bytes: usize,
}

impl FormatSettings {
    /// Точка монтирования репозитория; префикс совпадает с `Format::as_str`.
    pub(crate) fn repo_url(&self, format: Format, repo: &str) -> String {
        format!("{}/{}/{repo}", self.public_url, format.as_str())
    }
}

/// Что нужно формату, чтобы построить источник и router proxy-репозитория.
#[derive(Debug, Clone, Copy)]
pub struct ProxySource<'a> {
    pub repository: RepositoryId,
    /// Имя репозитория: совпадает с сегментом, под которым router монтируется.
    pub name: &'a str,
    pub policy: &'a EgressPolicy,
}

/// Hosted-router формата без слоёв аутентификации и формата; `None` — hosted ещё не реализован.
pub(crate) fn hosted_router<C: Catalog + 'static, B: BlobStore + 'static>(
    format: Format,
    registry: &Arc<Registry<C, B>>,
    settings: &FormatSettings,
) -> Option<Router> {
    match format {
        Format::Cargo => Some(cargo::hosted_router(Arc::clone(registry), settings)),
        Format::Npm => Some(npm::hosted_router(Arc::clone(registry), settings)),
        Format::PyPi => None,
        Format::Maven => None,
        Format::NuGet => None,
    }
}

/// Proxy-router репозитория над общим движком; `None` — proxy формата ещё не реализован.
/// Отказ означает, что хост upstream вне политики egress.
pub fn proxy_router<U, PC, L, B, RC, RB>(
    format: Format,
    engine: ProxyEngine<U, PC, L, B>,
    registry: Arc<Registry<RC, RB>>,
    source: &ProxySource<'_>,
    settings: &FormatSettings,
) -> Result<Option<Router>, EgressRefusal>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
    RC: Catalog + 'static,
    RB: BlobStore + 'static,
{
    match format {
        Format::Cargo => cargo::proxy_router(engine, registry, source, settings).map(Some),
        Format::Npm => npm::proxy_router(engine, registry, source, settings).map(Some),
        Format::PyPi => Ok(None),
        Format::Maven => Ok(None),
        Format::NuGet => Ok(None),
    }
}

/// Источник документа, по которому проверяется присутствие имени в публичном реестре.
pub fn presence_origin(
    format: Format,
    source: &ProxySource<'_>,
) -> Result<Option<Origin>, EgressRefusal> {
    match format {
        Format::Cargo => cargo::presence_origin(source).map(Some),
        Format::Npm => npm::presence_origin(source).map(Some),
        Format::PyPi => Ok(None),
        Format::Maven => Ok(None),
        Format::NuGet => Ok(None),
    }
}

/// Путь и политика документа, существование которого доказывает, что имя публично. `None` — формат
/// не умеет спрашивать или путь по имени не построился.
#[must_use]
pub(crate) fn presence_probe(
    format: Format,
    name: &PackageName,
) -> Option<(UpstreamPath, &'static MetadataPolicy)> {
    match format {
        Format::Cargo => UpstreamPath::parse(&index_path(name))
            .ok()
            .map(|path| (path, &INDEX)),
        Format::Npm => packument_path(name).ok().map(|path| (path, &PACKUMENT)),
        Format::PyPi => None,
        Format::Maven => None,
        Format::NuGet => None,
    }
}

/// Хосты upstream формата: proxy ходит только на них.
#[must_use]
pub fn upstream_hosts(format: Format) -> &'static [&'static str] {
    match format {
        Format::Cargo => &[INDEX_HOST, DL_HOST],
        Format::Npm => &[REGISTRY_HOST],
        Format::PyPi => &[],
        Format::Maven => &[],
        Format::NuGet => &[],
    }
}

/// Тело ошибки в проводной форме формата с типом содержимого. `None` — формат отвечает
/// `application/problem+json` (RFC 9457). Клиенты cargo и npm показывают пользователю только текст
/// ошибки, поэтому id запроса уходит суффиксом.
pub(crate) fn error_body(
    format: Format,
    detail: &str,
    request_id: &str,
) -> Option<(&'static str, String)> {
    let text = || format!("{detail} (request_id {request_id})");
    match format {
        Format::Cargo => Some((JSON, crane_format_cargo::render_errors(&text()))),
        Format::Npm => Some((JSON, crane_format_npm::render_error(&text()))),
        Format::PyPi => None,
        Format::Maven => None,
        Format::NuGet => None,
    }
}

/// Что клиент формата показывает в `WWW-Authenticate` на 401. Pip, Maven и dotnet присылают учётные
/// данные только в ответ на 401 с Basic-challenge.
pub(crate) fn challenge(format: Format) -> HeaderValue {
    match format {
        Format::Cargo => HeaderValue::from_static("Cargo"),
        Format::Npm => HeaderValue::from_static("Bearer realm=\"crane\""),
        Format::PyPi => BASIC_CHALLENGE,
        Format::Maven => BASIC_CHALLENGE,
        Format::NuGet => BASIC_CHALLENGE,
    }
}

/// Способы предъявить токен в порядке предпочтения.
pub(crate) fn credentials(format: Format) -> &'static [Credential] {
    match format {
        Format::Cargo => &[Credential::Raw],
        Format::Npm => &[Credential::Bearer],
        Format::PyPi => &[Credential::BasicPassword, Credential::Bearer],
        Format::Maven => &[Credential::BasicPassword, Credential::Bearer],
        Format::NuGet => &[
            Credential::Header("X-NuGet-ApiKey"),
            Credential::BasicPassword,
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn challenge_names_the_scheme_the_client_of_the_format_answers_to() {
        // Arrange
        let formats = Format::ALL;

        // Act
        let actual: BTreeMap<_, _> = formats
            .into_iter()
            .map(|format| (format, challenge(format)))
            .collect();

        // Assert
        let basic = HeaderValue::from_static("Basic realm=\"crane\"");
        let expected = BTreeMap::from([
            (Format::Maven, basic.clone()),
            (
                Format::Npm,
                HeaderValue::from_static("Bearer realm=\"crane\""),
            ),
            (Format::PyPi, basic.clone()),
            (Format::NuGet, basic),
            (Format::Cargo, HeaderValue::from_static("Cargo")),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn only_cargo_and_npm_render_their_own_error_body() {
        // Arrange
        let formats = Format::ALL;

        // Act
        let actual: BTreeMap<_, _> = formats
            .into_iter()
            .map(|format| (format, error_body(format, "плохо", "req-1")))
            .collect();

        // Assert
        let text = "плохо (request_id req-1)";
        let expected = BTreeMap::from([
            (Format::Maven, None),
            (
                Format::Npm,
                Some((JSON, format!(r#"{{"error":"{text}"}}"#))),
            ),
            (Format::PyPi, None),
            (Format::NuGet, None),
            (
                Format::Cargo,
                Some((JSON, format!(r#"{{"errors":[{{"detail":"{text}"}}]}}"#))),
            ),
        ]);
        assert_eq!(actual, expected);
    }
}
