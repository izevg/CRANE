/// Результат проб связи одного шага мастера. Пробы ничего не сохраняют.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub checks: Vec<ProbeCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCheck {
    pub id: &'static str,
    pub status: ProbeStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStatus {
    Pass,
    /// Вперёд можно с подтверждением.
    Warn,
    Fail,
    Skipped,
}

impl ProbeReport {
    /// Вперёд можно, пока нет ни одного `Fail`.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status != ProbeStatus::Fail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(statuses: &[ProbeStatus]) -> ProbeReport {
        ProbeReport {
            checks: statuses
                .iter()
                .map(|&status| ProbeCheck {
                    id: "check",
                    status,
                    detail: None,
                })
                .collect(),
        }
    }

    #[test]
    fn passed_is_false_only_when_some_check_failed() {
        // Arrange
        let cases = [
            report(&[]),
            report(&[ProbeStatus::Pass, ProbeStatus::Warn, ProbeStatus::Skipped]),
            report(&[ProbeStatus::Pass, ProbeStatus::Fail]),
            report(&[ProbeStatus::Fail]),
        ];

        // Act
        let actual = cases.map(|r| r.passed());

        // Assert
        assert_eq!(actual, [true, true, false, false]);
    }
}
