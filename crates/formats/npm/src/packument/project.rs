//! Сокращённая проекция (corgi): одна для upstream и hosted.

use super::{DocError, Fields, Raw, model::Packument};

const TOP_KEEP: [&str; 3] = ["name", "dist-tags", "modified"];
const VERSION_KEEP: [&str; 21] = [
    "name",
    "version",
    "deprecated",
    "dependencies",
    "optionalDependencies",
    "devDependencies",
    "bundleDependencies",
    "bundledDependencies",
    "peerDependencies",
    "peerDependenciesMeta",
    "acceptDependencies",
    "bin",
    "directories",
    "dist",
    "engines",
    "_hasShrinkwrap",
    "hasInstallScript",
    "cpu",
    "os",
    "libc",
    "funding",
];
const INSTALL_HOOKS: [&str; 3] = ["preinstall", "install", "postinstall"];
const HAS_INSTALL_SCRIPT: &str = "hasInstallScript";

pub(super) fn abbreviate(doc: &mut Packument<'_>) -> Result<(), DocError> {
    // Строка, а не заимствование: `time` разбирается во временную карту.
    let modified = match doc.fields.get("modified").and_then(Raw::as_string) {
        Some(modified) => Some(modified),
        None => doc
            .fields
            .get("time")
            .and_then(Raw::as_object)
            .and_then(|time| time.get("modified").and_then(Raw::as_string)),
    };
    doc.fields.retain(|key, _| TOP_KEEP.contains(&key.as_ref()));
    if let Some(modified) = modified {
        doc.fields.insert("modified".into(), Raw::owned(&modified)?);
    }
    doc.versions.values_mut().try_for_each(abbreviate_version)
}

fn abbreviate_version(version: &mut Fields<'_>) -> Result<(), DocError> {
    // Клиенты решают по этому полю, запускать ли скрипты: выводим его, если upstream не указал.
    if !version.contains_key(HAS_INSTALL_SCRIPT) && runs_install_script(version) {
        version.insert(HAS_INSTALL_SCRIPT.into(), Raw::owned(&true)?);
    }
    version.retain(|key, _| VERSION_KEEP.contains(&key.as_ref()));
    Ok(())
}

fn runs_install_script(version: &Fields<'_>) -> bool {
    version
        .get("scripts")
        .and_then(Raw::as_object)
        .is_some_and(|scripts| INSTALL_HOOKS.iter().any(|hook| scripts.contains_key(*hook)))
}
