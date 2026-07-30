use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Stakeholder,
    Basic,
    BasicTestPlans,
    VisualStudioSubscriber,
    VisualStudioEnterprise,
    GitHubEnterprise,
}

impl AccessLevel {
    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Stakeholder => "stakeholder",
            Self::Basic => "basic",
            Self::BasicTestPlans => "basic-test-plans",
            Self::VisualStudioSubscriber => "visual-studio-subscriber",
            Self::VisualStudioEnterprise => "visual-studio-enterprise",
            Self::GitHubEnterprise => "github-enterprise",
        }
    }

    fn api_account_license_type(self) -> &'static str {
        match self {
            Self::Stakeholder => "stakeholder",
            Self::Basic => "express",
            Self::BasicTestPlans => "advanced",
            Self::VisualStudioSubscriber
            | Self::VisualStudioEnterprise
            | Self::GitHubEnterprise => "none",
        }
    }

    fn api_licensing_source(self) -> &'static str {
        match self {
            Self::Stakeholder | Self::Basic | Self::BasicTestPlans => "account",
            Self::VisualStudioSubscriber | Self::VisualStudioEnterprise => "msdn",
            Self::GitHubEnterprise => "gitHub",
        }
    }

    fn api_msdn_license_type(self) -> Option<&'static str> {
        match self {
            Self::VisualStudioSubscriber => Some("eligible"),
            Self::VisualStudioEnterprise => Some("enterprise"),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Stakeholder => "Stakeholder",
            Self::Basic => "Basic",
            Self::BasicTestPlans => "Basic + Test Plans",
            Self::VisualStudioSubscriber => "Visual Studio Subscriber",
            Self::VisualStudioEnterprise => "Visual Studio Enterprise",
            Self::GitHubEnterprise => "GitHub Enterprise",
        }
    }

    pub fn matches_api_values(
        self,
        account_license_type: &str,
        licensing_source: &str,
        msdn_license_type: &str,
        github_license_type: &str,
    ) -> bool {
        let account_license_type = normalize(account_license_type);
        let licensing_source = normalize(licensing_source);
        let msdn_license_type = normalize(msdn_license_type);
        let github_license_type = normalize(github_license_type);

        match self {
            Self::Stakeholder | Self::Basic | Self::BasicTestPlans => {
                account_license_type == self.cli_name()
                    || account_license_type == self.api_account_license_type()
            }
            Self::VisualStudioSubscriber | Self::VisualStudioEnterprise => {
                account_license_type == "none"
                    && licensing_source == "msdn"
                    && self
                        .api_msdn_license_type()
                        .is_some_and(|value| msdn_license_type == value)
            }
            Self::GitHubEnterprise => {
                account_license_type == "none"
                    && licensing_source == "github"
                    && github_license_type == "enterprise"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignableAccessLevel(AccessLevel);

impl AssignableAccessLevel {
    pub fn cli_name(self) -> &'static str {
        self.0.cli_name()
    }

    pub fn api_account_license_type(self) -> &'static str {
        self.0.api_account_license_type()
    }

    pub fn api_licensing_source(self) -> &'static str {
        self.0.api_licensing_source()
    }

    pub fn api_msdn_license_type(self) -> Option<&'static str> {
        self.0.api_msdn_license_type()
    }

    pub fn display_name(self) -> &'static str {
        self.0.display_name()
    }
}

impl FromStr for AssignableAccessLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let level = match normalize(value).as_str() {
            "stakeholder" => AccessLevel::Stakeholder,
            "basic" | "express" => AccessLevel::Basic,
            "basic-test-plans" | "advanced" => AccessLevel::BasicTestPlans,
            "visual-studio-subscriber" => AccessLevel::VisualStudioSubscriber,
            "visual-studio-enterprise" => AccessLevel::VisualStudioEnterprise,
            _ => {
                return Err(
                    "可設定的 accessLevel 無效；可用值：stakeholder、basic（別名 express）、basic-test-plans（別名 advanced）、visual-studio-subscriber、visual-studio-enterprise"
                        .into(),
                );
            }
        };

        Ok(Self(level))
    }
}

impl FromStr for AccessLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "stakeholder" => Ok(Self::Stakeholder),
            "basic" | "express" => Ok(Self::Basic),
            "basic-test-plans" | "advanced" => Ok(Self::BasicTestPlans),
            "visual-studio-subscriber" => Ok(Self::VisualStudioSubscriber),
            "visual-studio-enterprise" => Ok(Self::VisualStudioEnterprise),
            "github-enterprise" => Ok(Self::GitHubEnterprise),
            _ => Err(
                "accessLevel 無效；可用值：stakeholder、basic（別名 express）、basic-test-plans（別名 advanced）、visual-studio-subscriber、visual-studio-enterprise、github-enterprise"
                    .into(),
            ),
        }
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{AccessLevel, AssignableAccessLevel};

    #[test]
    fn maps_assignable_cli_values_to_azure_devops_values() {
        assert_eq!(
            AssignableAccessLevel::from_str("stakeholder")
                .unwrap()
                .api_account_license_type(),
            "stakeholder"
        );
        assert_eq!(
            AssignableAccessLevel::from_str("basic")
                .unwrap()
                .api_account_license_type(),
            "express"
        );
        assert_eq!(
            AssignableAccessLevel::from_str("basic-test-plans")
                .unwrap()
                .api_account_license_type(),
            "advanced"
        );
        assert_eq!(
            AssignableAccessLevel::from_str("visual-studio-subscriber")
                .unwrap()
                .api_account_license_type(),
            "none"
        );
        assert_eq!(
            AssignableAccessLevel::from_str("visual-studio-subscriber")
                .unwrap()
                .api_licensing_source(),
            "msdn"
        );
        assert_eq!(
            AssignableAccessLevel::from_str("visual-studio-subscriber")
                .unwrap()
                .api_msdn_license_type(),
            Some("eligible")
        );
        assert_eq!(
            AssignableAccessLevel::from_str("visual-studio-enterprise")
                .unwrap()
                .api_msdn_license_type(),
            Some("enterprise")
        );
    }

    #[test]
    fn matches_aliases_and_api_values() {
        assert!(AccessLevel::Basic.matches_api_values("basic", "", "", ""));
        assert!(AccessLevel::Basic.matches_api_values("express", "account", "none", "none"));
        assert!(AccessLevel::BasicTestPlans.matches_api_values("basic_test_plans", "", "", ""));
        assert!(
            AccessLevel::BasicTestPlans.matches_api_values("advanced", "account", "none", "none")
        );
        assert!(
            AccessLevel::VisualStudioSubscriber
                .matches_api_values("none", "msdn", "eligible", "none")
        );
        assert!(AccessLevel::VisualStudioEnterprise.matches_api_values(
            "none",
            "msdn",
            "enterprise",
            "none"
        ));
        assert!(AccessLevel::GitHubEnterprise.matches_api_values(
            "none",
            "gitHub",
            "none",
            "enterprise"
        ));
    }

    #[test]
    fn parses_every_publicly_supported_access_level() {
        let supported = [
            ("stakeholder", AccessLevel::Stakeholder),
            ("basic", AccessLevel::Basic),
            ("basic-test-plans", AccessLevel::BasicTestPlans),
            (
                "visual-studio-subscriber",
                AccessLevel::VisualStudioSubscriber,
            ),
            (
                "visual-studio-enterprise",
                AccessLevel::VisualStudioEnterprise,
            ),
            ("github-enterprise", AccessLevel::GitHubEnterprise),
        ];

        for (value, expected) in supported {
            assert_eq!(AccessLevel::from_str(value), Ok(expected));
            assert_eq!(expected.cli_name(), value);
        }

        assert_eq!(AccessLevel::from_str("express"), Ok(AccessLevel::Basic));
        assert_eq!(
            AccessLevel::from_str("advanced"),
            Ok(AccessLevel::BasicTestPlans)
        );
        assert_eq!(
            AssignableAccessLevel::from_str("express")
                .unwrap()
                .cli_name(),
            "basic"
        );
        assert_eq!(
            AssignableAccessLevel::from_str("advanced")
                .unwrap()
                .cli_name(),
            "basic-test-plans"
        );
    }

    #[test]
    fn rejects_internal_or_undocumented_raw_account_license_types() {
        for value in ["none", "early-adopter", "earlyAdopter", "professional"] {
            assert!(AccessLevel::from_str(value).is_err());
        }
    }

    #[test]
    fn rejects_automatically_detected_github_enterprise_for_direct_assignment() {
        assert!(AccessLevel::from_str("github-enterprise").is_ok());
        assert!(AssignableAccessLevel::from_str("github-enterprise").is_err());
    }
}
