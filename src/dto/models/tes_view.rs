use serde::Deserialize;

/// Controls how much data to return for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TesView {
    #[default]
    Minimal,
    Basic,
    Full,
}

impl std::str::FromStr for TesView {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "BASIC" => Self::Basic,
            "FULL" => Self::Full,
            _ => Self::Minimal,
        })
    }
}

impl<'de> Deserialize<'de> for TesView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.parse().unwrap_or_default())
    }
}
