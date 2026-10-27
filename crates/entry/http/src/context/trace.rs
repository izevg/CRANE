//! W3C Trace Context: идентификатор трассы из `traceparent`.

const VERSION: &str = "00";
const TRACE_ID_LEN: usize = 32;
const PARENT_ID_LEN: usize = 16;
const FLAGS_LEN: usize = 2;

/// `00-<32 hex>-<16 hex>-<2 hex>`, все символы нижнего регистра, ни trace id, ни parent id
/// не из одних нулей. Иное значение — `None`: заголовок приходит от клиента и в журнал
/// попадёт только проверенным.
#[must_use]
pub(super) fn trace_id(header: &str) -> Option<String> {
    let mut parts = header.split('-');
    let (version, trace, parent, flags) =
        (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || version != VERSION {
        return None;
    }
    let well_formed = |part: &str, len: usize| {
        part.len() == len
            && part
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    };
    let not_zero = |part: &str| part.bytes().any(|byte| byte != b'0');
    (well_formed(trace, TRACE_ID_LEN)
        && well_formed(parent, PARENT_ID_LEN)
        && well_formed(flags, FLAGS_LEN)
        && not_zero(trace)
        && not_zero(parent))
    .then(|| trace.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACE: &str = "4bf92f3577b34da6a3ce929d0e0e4736";
    const PARENT: &str = "00f067aa0ba902b7";

    #[test]
    fn only_a_well_formed_non_zero_traceparent_yields_a_trace_id() {
        // Arrange
        let zero_trace = format!("00-{}-{PARENT}-01", "0".repeat(32));
        let zero_parent = format!("00-{TRACE}-{}-01", "0".repeat(16));
        let cases = [
            (format!("00-{TRACE}-{PARENT}-01"), Some(TRACE)),
            (format!("01-{TRACE}-{PARENT}-01"), None),
            (format!("00-{}-{PARENT}-01", TRACE.to_uppercase()), None),
            (format!("00-{}-{PARENT}-01", &TRACE[1..]), None),
            (format!("00-{TRACE}0-{PARENT}-01"), None),
            (format!("00-{TRACE}-{}-01", &PARENT[1..]), None),
            (format!("00-{TRACE}-{PARENT}-1"), None),
            (format!("00-{TRACE}-{PARENT}-01-extra"), None),
            (format!("00-{TRACE}-{PARENT}"), None),
            (format!("00-{}-{PARENT}-01", TRACE.replace('4', "g")), None),
            (zero_trace, None),
            (zero_parent, None),
            (String::new(), None),
        ];

        // Act
        let actual = cases.clone().map(|(header, _)| trace_id(&header));

        // Assert
        assert_eq!(
            actual,
            cases.map(|(_, expected)| expected.map(str::to_owned))
        );
    }
}
