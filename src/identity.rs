use serde::{Deserialize, Serialize};

use crate::error::{AdoctlError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum UserIdentifier {
    Upn(String),
    Id(String),
}

impl UserIdentifier {
    pub fn from_parts(upn: Option<String>, id: Option<String>) -> Result<Self> {
        match (upn, id) {
            (Some(upn), None) if !upn.trim().is_empty() => Ok(Self::Upn(upn.trim().to_owned())),
            (None, Some(id)) if !id.trim().is_empty() => Ok(Self::Id(id.trim().to_owned())),
            _ => Err(AdoctlError::MissingUserIdentifier),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Upn(value) | Self::Id(value) => value,
        }
    }

    pub fn is_upn_match(&self, candidate: &str) -> bool {
        match self {
            Self::Upn(upn) => upn.eq_ignore_ascii_case(candidate),
            Self::Id(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UserIdentifier;

    #[test]
    fn prefers_single_explicit_identifier() {
        assert_eq!(
            UserIdentifier::from_parts(Some(" user@example.com ".into()), None).unwrap(),
            UserIdentifier::Upn("user@example.com".into())
        );
        assert_eq!(
            UserIdentifier::from_parts(None, Some(" 123 ".into())).unwrap(),
            UserIdentifier::Id("123".into())
        );
    }

    #[test]
    fn rejects_missing_identifier() {
        assert!(UserIdentifier::from_parts(None, None).is_err());
    }
}
