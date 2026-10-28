//! Протокол Maven-репозитория: чистый sans-IO код без HTTP и async.
//!
//! Каталог CRANE остаётся единственным источником истины: `maven-metadata.xml` и контрольные
//! суммы здесь только проекции его фактов, а SNAPSHOT — представление над неизменяемыми
//! timestamp-сборками.

mod ecosystem;
mod layout;

pub use ecosystem::{Build, MAVEN, Maven};
pub use layout::{Algorithm, FileName, Resource, Route, RouteError};
