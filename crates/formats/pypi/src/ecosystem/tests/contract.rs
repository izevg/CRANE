use crane_domain::registry::Format;
use crane_ports::contract::{Samples, ecosystem_contract};

use crate::ecosystem::PYPI;

#[test]
fn pypi_satisfies_ecosystem_contract() {
    ecosystem_contract(
        &PYPI,
        &Samples {
            format: Format::PyPi,
            bases: &["acme", "Acme.Pay", "acme_pay_core"],
            names: &["requests", "Zope.Interface", "my_pkg-Name", "A", "a1"],
            versions: &[
                "0.0.1",
                "1.0.dev1",
                "1.0a1",
                "1.0b2",
                "1.0rc1",
                "1.0",
                "1.0.post1",
                "1.0.1",
                "1.10",
                "1!0.1",
            ],
        },
    );
}
