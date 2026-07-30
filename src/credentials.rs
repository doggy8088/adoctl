use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::error::{AdoctlError, Result};

const SERVICE_NAME: &str = "adoctl";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialKey {
    organization: String,
    profile: String,
}

impl CredentialKey {
    pub fn new(organization: impl Into<String>, profile: impl Into<String>) -> Self {
        Self {
            organization: organization.into(),
            profile: profile.into(),
        }
    }

    pub fn account_name(&self) -> String {
        format!("{}:{}", self.organization, self.profile)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StoredCredential {
    Pat {
        token: String,
    },
    AzureCli,
    DeviceCode {
        client_id: String,
        tenant: String,
        refresh_token: String,
        access_token: Option<String>,
        expires_at: Option<u64>,
    },
}

impl StoredCredential {
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Pat { .. } => "PAT",
            Self::AzureCli => "Azure CLI",
            Self::DeviceCode { .. } => "OAuth device code",
        }
    }
}

pub trait CredentialStore: Send + Sync {
    fn save(&self, key: &CredentialKey, credential: &StoredCredential) -> Result<()>;
    fn load(&self, key: &CredentialKey) -> Result<Option<StoredCredential>>;
    fn delete(&self, key: &CredentialKey) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct KeyringCredentialStore;

impl KeyringCredentialStore {
    pub fn new() -> Self {
        Self
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn save(&self, key: &CredentialKey, credential: &StoredCredential) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, &key.account_name())
            .map_err(|error| AdoctlError::CredentialStore(error.to_string()))?;
        let value = serde_json::to_string(credential)?;
        entry
            .set_password(&value)
            .map_err(|error| AdoctlError::CredentialStore(error.to_string()))
    }

    fn load(&self, key: &CredentialKey) -> Result<Option<StoredCredential>> {
        let entry = keyring::Entry::new(SERVICE_NAME, &key.account_name())
            .map_err(|error| AdoctlError::CredentialStore(error.to_string()))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(serde_json::from_str(&value)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AdoctlError::CredentialStore(error.to_string())),
        }
    }

    fn delete(&self, key: &CredentialKey) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, &key.account_name())
            .map_err(|error| AdoctlError::CredentialStore(error.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AdoctlError::CredentialStore(error.to_string())),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct MemoryCredentialStore {
    values: Arc<Mutex<HashMap<CredentialKey, StoredCredential>>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn save(&self, key: &CredentialKey, credential: &StoredCredential) -> Result<()> {
        self.values
            .lock()
            .expect("memory credential store mutex poisoned")
            .insert(key.clone(), credential.clone());
        Ok(())
    }

    fn load(&self, key: &CredentialKey) -> Result<Option<StoredCredential>> {
        Ok(self
            .values
            .lock()
            .expect("memory credential store mutex poisoned")
            .get(key)
            .cloned())
    }

    fn delete(&self, key: &CredentialKey) -> Result<()> {
        self.values
            .lock()
            .expect("memory credential store mutex poisoned")
            .remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CredentialKey, CredentialStore, MemoryCredentialStore, StoredCredential};

    #[test]
    fn memory_store_round_trips_credentials() {
        let store = MemoryCredentialStore::default();
        let key = CredentialKey::new("org", "default");
        let credential = StoredCredential::Pat {
            token: "secret".into(),
        };

        store.save(&key, &credential).unwrap();
        assert_eq!(store.load(&key).unwrap(), Some(credential));
        store.delete(&key).unwrap();
        assert_eq!(store.load(&key).unwrap(), None);
    }
}
