use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectStateFilter {
    #[default]
    All,
    New,
    CreatePending,
    WellFormed,
    Deleting,
}

impl ProjectStateFilter {
    pub fn cli_name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::New => "new",
            Self::CreatePending => "create-pending",
            Self::WellFormed => "well-formed",
            Self::Deleting => "deleting",
        }
    }

    pub fn api_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::New => "new",
            Self::CreatePending => "createPending",
            Self::WellFormed => "wellFormed",
            Self::Deleting => "deleting",
        }
    }
}

impl fmt::Display for ProjectStateFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.cli_name())
    }
}

impl FromStr for ProjectStateFilter {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "all" => Ok(Self::All),
            "new" => Ok(Self::New),
            "create-pending" | "createpending" => Ok(Self::CreatePending),
            "well-formed" | "wellformed" => Ok(Self::WellFormed),
            "deleting" => Ok(Self::Deleting),
            _ => {
                Err("專案狀態無效；可用值：all、new、create-pending、well-formed、deleting".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectStateFilter;

    #[test]
    fn maps_cli_values_to_api_values() {
        assert_eq!(ProjectStateFilter::All.api_value(), "all");
        assert_eq!(ProjectStateFilter::New.api_value(), "new");
        assert_eq!(
            ProjectStateFilter::CreatePending.api_value(),
            "createPending"
        );
        assert_eq!(ProjectStateFilter::WellFormed.api_value(), "wellFormed");
        assert_eq!(ProjectStateFilter::Deleting.api_value(), "deleting");
    }

    #[test]
    fn parses_cli_and_api_style_values() {
        assert_eq!(
            "all".parse::<ProjectStateFilter>().unwrap(),
            ProjectStateFilter::All
        );
        assert_eq!(
            "create-pending".parse::<ProjectStateFilter>().unwrap(),
            ProjectStateFilter::CreatePending
        );
        assert_eq!(
            "createPending".parse::<ProjectStateFilter>().unwrap(),
            ProjectStateFilter::CreatePending
        );
        assert_eq!(
            "well_formed".parse::<ProjectStateFilter>().unwrap(),
            ProjectStateFilter::WellFormed
        );
    }
}
