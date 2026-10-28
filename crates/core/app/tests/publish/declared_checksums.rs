//! Проверка заявленных контрольных сумм.

use crane_app::registry::RegistryError;
use crane_domain::registry::{Checksum, Md5, Sha1, Sha256};
use futures_executor::block_on;

use crate::common::faults::{RecordingBlobs, commit_log, committed};
use crate::common::{
    Seed, caller, hello_md5, hello_sha1, hello_sha256, hello_sha512, publish_request,
};

use super::{HELLO, listed_foo};

#[test]
fn publish_accepts_matching_declared_sha256() {
    // Arrange
    let registry = Seed::new().registry();
    let request = publish_request(
        "foo",
        "1.0.0",
        &HELLO,
        vec![Checksum::Sha256(hello_sha256())],
    );

    // Act
    let result = block_on(registry.publish(&caller(), request));

    // Assert
    assert!(
        result.is_ok(),
        "совпавший заявленный sha256 не мешает публикации: {result:?}"
    );
}

#[test]
fn publish_rejects_mismatching_declared_sha256_leaving_cas_and_catalog_untouched() {
    // Arrange
    let declared = Checksum::Sha256(Sha256::from([9; 32]));
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, vec![declared]);

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(
            &error,
            RegistryError::DigestMismatch { declared: d, actual } if **d == declared && **actual == Checksum::Sha256(hello_sha256())
        ),
        "ожидался DigestMismatch с точными значениями: {error:?}"
    );
    assert!(
        committed(&log).is_empty(),
        "неверный blob не должен попасть в CAS"
    );
    assert!(
        listed_foo(&registry).is_empty(),
        "записи каталога быть не должно"
    );
}

#[test]
fn publish_rejects_when_any_declared_checksum_mismatches_leaving_cas_untouched() {
    // Arrange
    let declared = [
        Checksum::Sha256(hello_sha256()),
        Checksum::Sha1(Sha1::from([9; 20])),
    ];
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, declared.to_vec());

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(&error, RegistryError::DigestMismatch { declared: d, actual }
            if **d == declared[1] && **actual == Checksum::Sha1(hello_sha1())),
        "{error:?}"
    );
    assert!(committed(&log).is_empty(), "неверный blob не попал в CAS");
}

#[test]
fn publish_rejects_mismatching_declared_md5_leaving_cas_untouched() {
    // Arrange
    let declared = Checksum::Md5(Md5::from([9; 16]));
    let log = commit_log();
    let registry = Seed::new().registry_with(
        |catalog| catalog,
        |blobs| RecordingBlobs::recording(blobs, log.clone()),
    );
    let request = publish_request("foo", "1.0.0", &HELLO, vec![declared]);

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(&error, RegistryError::DigestMismatch { declared: d, actual }
            if **d == declared && **actual == Checksum::Md5(hello_md5())),
        "{error:?}"
    );
    assert!(committed(&log).is_empty(), "неверный blob не попал в CAS");
}

#[test]
fn publish_accepts_all_matching_declared_checksums() {
    // Arrange
    let registry = Seed::new().registry();
    let declared = vec![
        Checksum::Sha256(hello_sha256()),
        Checksum::Sha512(hello_sha512()),
        Checksum::Sha1(hello_sha1()),
        Checksum::Md5(hello_md5()),
    ];
    let request = publish_request("foo", "1.0.0", &HELLO, declared);

    // Act
    let result = block_on(registry.publish(&caller(), request));

    // Assert
    assert!(result.is_ok(), "{result:?}");
}
