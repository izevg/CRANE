use crane_domain::registry::Format;
use crane_ports::contract::{Samples, ecosystem_contract};

use crate::ecosystem::MAVEN;

#[test]
fn maven_satisfies_ecosystem_contract() {
    ecosystem_contract(
        &MAVEN,
        &Samples {
            format: Format::Maven,
            bases: &["acme", "Com.Acme", "org.apache.maven"],
            names: &[
                "com.acme:core",
                "Org.Apache:commons-lang3",
                "io.github.user-name:lib.api",
            ],
            versions: &[
                "0.9-SNAPSHOT",
                "1.0-alpha-1",
                "1.0-beta-2",
                "1.0-rc-1",
                "1.0",
                "1.0.1",
                "1.0.10",
                "1.1-SNAPSHOT",
                "1.1",
            ],
        },
    );
}
