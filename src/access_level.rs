use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Stakeholder,
    Basic,
    BasicTestPlans,
}

impl AccessLevel {
    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Stakeholder => "stakeholder",
            Self::Basic => "basic",
            Self::BasicTestPlans => "basic-test-plans",
        }
    }

    pub fn api_account_license_type(self) -> &'static str {
        match self {
            Self::Stakeholder => "stakeholder",
            Self::Basic => "express",
            Self::BasicTestPlans => "advanced",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Stakeholder => "Stakeholder",
            Self::Basic => "Basic",
            Self::BasicTestPlans => "Basic + Test Plans",
        }
    }

    pub fn matches_api_value(self, value: &str) -> bool {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        normalized == self.cli_name() || normalized == self.api_account_license_type()
    }
}

impl FromStr for AccessLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "stakeholder" => Ok(Self::Stakeholder),
            "basic" => Ok(Self::Basic),
            "basic-test-plans" => Ok(Self::BasicTestPlans),
            _ => Err("accessLevel 無效；可用值：stakeholder、basic、basic-test-plans".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccessLevel;

    #[test]
    fn maps_safe_cli_values_to_azure_devops_values() {
        assert_eq!(
            AccessLevel::Stakeholder.api_account_license_type(),
            "stakeholder"
        );
        assert_eq!(AccessLevel::Basic.api_account_license_type(), "express");
        assert_eq!(
            AccessLevel::BasicTestPlans.api_account_license_type(),
            "advanced"
        );
    }

    #[test]
    fn matches_aliases_and_api_values() {
        assert!(AccessLevel::Basic.matches_api_value("basic"));
        assert!(AccessLevel::Basic.matches_api_value("express"));
        assert!(AccessLevel::BasicTestPlans.matches_api_value("basic_test_plans"));
        assert!(AccessLevel::BasicTestPlans.matches_api_value("advanced"));
    }
}
