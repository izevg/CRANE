use crane_domain::registry::Format;
use crane_ports::contract::{Samples, ecosystem_contract};

use crate::ecosystem::CARGO;

#[test]
fn cargo_satisfies_ecosystem_contract() {
    ecosystem_contract(
        &CARGO,
        &Samples {
            format: Format::Cargo,
            names: &["serde", "Serde-Json", "tokio_util", "A", "a1-b_2"],
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
