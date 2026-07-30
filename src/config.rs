use url::Url;

use crate::{
    cli::{AuthMethodArg, Cli, OutputArg},
    error::{AdoctlError, Result},
};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub organization: Organization,
    pub profile: String,
    pub output: OutputArg,
    pub auth_method: Option<AuthMethodArg>,
}

impl AppConfig {
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let org = cli.org.as_deref().ok_or(AdoctlError::MissingOrganization)?;

        Ok(Self {
            organization: Organization::parse(org, cli.base_url.clone())?,
            profile: cli.profile.clone(),
            output: cli.output,
            auth_method: cli.auth_method,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub name: String,
    base_url_override: Option<String>,
}

impl Organization {
    pub fn parse(input: &str, base_url_override: Option<String>) -> Result<Self> {
        let trimmed = input.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AdoctlError::InvalidOrganization(input.to_owned()));
        }

        let name = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            extract_org_name_from_url(trimmed)?
        } else {
            trimmed.to_owned()
        };

        if name.is_empty() || name.contains('/') {
            return Err(AdoctlError::InvalidOrganization(input.to_owned()));
        }

        Ok(Self {
            name,
            base_url_override,
        })
    }

    pub fn service_base(&self, service: AdoService) -> Result<Url> {
        if let Some(base_url) = &self.base_url_override {
            return Ok(Url::parse(ensure_trailing_slash(base_url).as_str())?);
        }

        let base = match service {
            AdoService::Core => format!("https://dev.azure.com/{}/", self.name),
            AdoService::Entitlements => format!("https://vsaex.dev.azure.com/{}/", self.name),
            AdoService::Graph => format!("https://vssps.dev.azure.com/{}/", self.name),
        };
        Ok(Url::parse(&base)?)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AdoService {
    Core,
    Entitlements,
    Graph,
}

fn extract_org_name_from_url(input: &str) -> Result<String> {
    let url = Url::parse(input)?;
    let host = url
        .host_str()
        .ok_or_else(|| AdoctlError::InvalidOrganization(input.to_owned()))?;

    if host.eq_ignore_ascii_case("dev.azure.com")
        || host.eq_ignore_ascii_case("vsaex.dev.azure.com")
        || host.eq_ignore_ascii_case("vssps.dev.azure.com")
    {
        return url
            .path_segments()
            .and_then(|mut segments| segments.find(|segment| !segment.is_empty()))
            .map(str::to_owned)
            .ok_or_else(|| AdoctlError::InvalidOrganization(input.to_owned()));
    }

    if let Some(org) = host.strip_suffix(".visualstudio.com") {
        return Ok(org.to_owned());
    }

    Err(AdoctlError::InvalidOrganization(input.to_owned()))
}

fn ensure_trailing_slash(value: &str) -> String {
    if value.ends_with('/') {
        value.to_owned()
    } else {
        format!("{value}/")
    }
}

#[cfg(test)]
mod tests {
    use super::{AdoService, Organization};

    #[test]
    fn parses_organization_name_from_common_forms() {
        assert_eq!(Organization::parse("my-org", None).unwrap().name, "my-org");
        assert_eq!(
            Organization::parse("https://dev.azure.com/my-org", None)
                .unwrap()
                .name,
            "my-org"
        );
        assert_eq!(
            Organization::parse("https://my-org.visualstudio.com", None)
                .unwrap()
                .name,
            "my-org"
        );
    }

    #[test]
    fn supports_test_base_url_override() {
        let org = Organization::parse("my-org", Some("http://127.0.0.1:3000".into())).unwrap();
        assert_eq!(
            org.service_base(AdoService::Entitlements).unwrap().as_str(),
            "http://127.0.0.1:3000/"
        );
    }
}
