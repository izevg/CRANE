use crane_domain::registry::Format;
use crane_ports::contract::{Samples, ecosystem_contract};

use crate::ecosystem::NPM;

#[test]
fn npm_satisfies_ecosystem_contract() {
    ecosystem_contract(
        &NPM,
        &Samples {
            format: Format::Npm,
            bases: &["acme", "Acme"],
            names: &["left-pad", "JSONStream", "@babel/core", "@Scope/Name"],
            versions: &[
                "0.0.1",
                "1.0.0-alpha",
                "1.0.0-alpha.1",
                "1.0.0-rc.1",
                "1.0.0",
                "1.0.1",
                "1.10.0",
            ],
        },
    );
}
