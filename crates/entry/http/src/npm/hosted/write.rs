//! Запись: публикация, устаревание, снятие, dist-tags.

use std::future::ready;
use std::io;

use axum::response::Response;
use bytes::Bytes;
use crane_app::registry::{Publish, Upload};
use crane_domain::registry::{Format, PackageName, PackageRef};
use crane_format_npm::{
    UNPUBLISH, Write, classify, parse_tag, parse_tag_body, removable, render_ok, tarball_version,
};
use crane_ports::registry::{BlobStore, Catalog};
use futures_util::stream;

use super::read::coordinate;
use super::{Request, Shared};
use crate::error::{ClientError, chain};
use crate::npm::json_ok;

const UNSUPPORTED: &str = "операция не поддерживается";

fn bad_request(error: &(dyn std::error::Error + 'static)) -> ClientError {
    ClientError::bad_request(chain(error))
}

/// `PUT` документа: публикация, устаревание или ничего из поддерживаемого.
pub(super) async fn put<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
) -> Result<Response, ClientError> {
    let current = shared
        .registry
        .package(&request.caller, Format::Npm, &request.repo, package)
        .await?
        .versions;
    match classify(package, &request.body, &current).map_err(|e| bad_request(&e))? {
        Write::Publish(publish) => {
            let tarball: Result<Bytes, io::Error> = Ok(publish.tarball);
            shared
                .registry
                .publish(
                    &request.caller,
                    Publish {
                        coordinate: coordinate(request, package, publish.version),
                        file: Upload {
                            path: publish.file,
                            body: Box::pin(stream::once(ready(tarball))),
                            declared: publish.declared,
                        },
                        metadata: Ok(publish.metadata),
                        tags: vec![publish.tag],
                    },
                )
                .await?;
        }
        Write::Deprecate(changes) => {
            // Первая ошибка останавливает цепочку; повтор команды идемпотентен.
            for (version, message) in changes {
                shared
                    .registry
                    .set_deprecated(
                        &request.caller,
                        &coordinate(request, package, version),
                        message,
                    )
                    .await?;
            }
        }
        Write::Unsupported => return Err(ClientError::bad_request(UNSUPPORTED)),
    }
    Ok(json_ok(render_ok()))
}

pub(super) async fn delete_tarball<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
    file: &str,
) -> Result<Response, ClientError> {
    let version = tarball_version(package, file).ok_or_else(ClientError::not_found)?;
    shared
        .registry
        .delete(
            &request.caller,
            &coordinate(request, package, version),
            &UNPUBLISH,
        )
        .await?;
    Ok(json_ok(render_ok()))
}

pub(super) async fn delete_package<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
) -> Result<Response, ClientError> {
    shared
        .registry
        .delete_package(
            &request.caller,
            Format::Npm,
            &request.repo,
            package,
            &UNPUBLISH,
        )
        .await?;
    Ok(json_ok(render_ok()))
}

pub(super) async fn set_dist_tag<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
    tag: &str,
) -> Result<Response, ClientError> {
    let tag = parse_tag(tag).map_err(|e| bad_request(&e))?;
    let version = parse_tag_body(&request.body).map_err(|e| bad_request(&e))?;
    shared
        .registry
        .set_tag(
            &request.caller,
            &coordinate(request, package, version),
            &tag,
        )
        .await?;
    Ok(json_ok(render_ok()))
}

pub(super) async fn remove_dist_tag<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
    tag: &str,
) -> Result<Response, ClientError> {
    let tag = parse_tag(tag).map_err(|e| bad_request(&e))?;
    removable(&tag).map_err(|e| bad_request(&e))?;
    let target = PackageRef {
        format: Format::Npm,
        repository: request.repo.clone(),
        package: package.clone(),
    };
    shared
        .registry
        .remove_tag(&request.caller, &target, &tag)
        .await?;
    Ok(json_ok(render_ok()))
}
