//! API keys in the OS keychain (macOS Keychain, Windows Credential
//! Manager, Linux kernel keyring). Never written to disk by Sapu.

use crate::{AiError, Result};

const SERVICE: &str = "sapu";

fn entry(provider: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, provider).map_err(|e| AiError::Keychain(e.to_string()))
}

pub fn set_api_key(provider: &str, key: &str) -> Result<()> {
    entry(provider)?
        .set_password(key)
        .map_err(|e| AiError::Keychain(e.to_string()))
}

pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AiError::Keychain(e.to_string())),
    }
}

pub fn delete_api_key(provider: &str) -> Result<()> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AiError::Keychain(e.to_string())),
    }
}
