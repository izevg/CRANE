use std::str::FromStr;

/// Формат пакетного реестра; значения совпадают с SQL enum `pkg_format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Format {
    Maven,
    Npm,
    PyPi,
    NuGet,
    Cargo,
}

impl Format {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Maven => "maven",
            Self::Npm => "npm",
            Self::PyPi => "pypi",
            Self::NuGet => "nuget",
            Self::Cargo => "cargo",
        }
    }
}

/// Неизвестное название формата.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    #[error("неизвестный формат: {0}")]
    Unknown(String),
}

impl FromStr for Format {
    type Err = FormatError;

    // Регистр не нормализуем: строка приходит из SQL enum и должна совпасть точно.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "maven" => Ok(Self::Maven),
            "npm" => Ok(Self::Npm),
            "pypi" => Ok(Self::PyPi),
            "nuget" => Ok(Self::NuGet),
            "cargo" => Ok(Self::Cargo),
            other => Err(FormatError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Format; 5] = [
        Format::Maven,
        Format::Npm,
        Format::PyPi,
        Format::NuGet,
        Format::Cargo,
    ];

    #[test]
    fn every_format_round_trips_through_its_string_form() {
        // Arrange
        let formats = ALL;

        // Act
        let parsed: Vec<_> = formats
            .iter()
            .map(|f| f.as_str().parse::<Format>())
            .collect();

        // Assert
        let expected: Vec<_> = formats.iter().copied().map(Ok).collect();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn unknown_name_is_rejected() {
        // Arrange
        let raw = "rubygems";

        // Act
        let result = raw.parse::<Format>();

        // Assert
        assert_eq!(result, Err(FormatError::Unknown("rubygems".to_owned())));
    }

    #[test]
    fn name_with_different_case_is_rejected() {
        // Arrange
        let raw = "Cargo";

        // Act
        let result = raw.parse::<Format>();

        // Assert
        assert_eq!(result, Err(FormatError::Unknown("Cargo".to_owned())));
    }
}
