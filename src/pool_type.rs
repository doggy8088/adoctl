use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolTypeFilter {
    Automation,
    Deployment,
}

impl PoolTypeFilter {
    pub fn api_value(self) -> &'static str {
        match self {
            Self::Automation => "automation",
            Self::Deployment => "deployment",
        }
    }
}

impl fmt::Display for PoolTypeFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.api_value())
    }
}

impl FromStr for PoolTypeFilter {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "automation" => Ok(Self::Automation),
            "deployment" => Ok(Self::Deployment),
            _ => Err("代理程式集區型別無效；可用值：automation、deployment".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PoolTypeFilter;

    #[test]
    fn maps_pool_types_to_api_values() {
        assert_eq!(PoolTypeFilter::Automation.api_value(), "automation");
        assert_eq!(PoolTypeFilter::Deployment.api_value(), "deployment");
    }

    #[test]
    fn parses_pool_types_case_insensitively() {
        assert_eq!(
            "Automation".parse::<PoolTypeFilter>().unwrap(),
            PoolTypeFilter::Automation
        );
        assert_eq!(
            "deployment".parse::<PoolTypeFilter>().unwrap(),
            PoolTypeFilter::Deployment
        );
    }
}
