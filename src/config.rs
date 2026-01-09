use keyring::Entry;
use anyhow::{Result, anyhow};

const SERVICE_NAME: &str = "TeaserPasteCLI";
const ACCOUNT_NAME: &str = "api_token";

pub struct ConfigManager;

impl ConfigManager {
    pub fn set_token(token: &str) -> Result<()> {
        if !token.starts_with("priv_") {
            return Err(anyhow!("Invalid token. Private token must start with \"priv_\"."));
        }
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
        entry.set_password(token)?;
        Ok(())
    }

    pub fn get_token() -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn clear_token() -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
        match entry.delete_password() {
            Ok(_) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
