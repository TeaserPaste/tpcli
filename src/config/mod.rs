use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use keyring::Entry;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::fs;

const SERVICE_NAME: &str = "TeaserPasteCLI";
const ACCOUNT_NAME: &str = "api_token";

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub default_language: Option<String>,
    pub default_visibility: Option<String>,
    pub runners: Option<std::collections::HashMap<String, String>>,
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn validate_token(token: &str) -> Result<()> {
        if !token.starts_with("priv_") {
            return Err(anyhow!(
                "Invalid token. Private token must start with \"priv_\"."
            ));
        }
        Ok(())
    }

    pub fn set_token(token: &str) -> Result<()> {
        Self::validate_token(token)?;
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

    pub fn load_config() -> Result<Config> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "teaserpaste", "tpcli") {
            let config_path = proj_dirs.config_dir().join("config.json");
            debug!("Loading config from: {}", config_path.display());
            if config_path.exists() {
                let content = fs::read_to_string(config_path)?;
                let config: Config = serde_json::from_str(&content)?;
                return Ok(config);
            } else {
                warn!(
                    "Config file not found at: {}. Using default.",
                    config_path.display()
                );
            }
        } else {
            warn!("Could not determine project directory. Using default config.");
        }
        Ok(Config::default())
    }

    pub fn save_config(config: &Config) -> Result<()> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "teaserpaste", "tpcli") {
            let config_dir = proj_dirs.config_dir();
            fs::create_dir_all(config_dir)?;
            let config_path = config_dir.join("config.json");
            let content = serde_json::to_string_pretty(config)?;
            fs::write(config_path, content)?;
            return Ok(());
        }
        Err(anyhow!("Could not determine config directory"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_token() {
        assert!(ConfigManager::validate_token("priv_12345").is_ok());
        assert!(ConfigManager::validate_token("public_token").is_err());
        assert!(ConfigManager::validate_token("").is_err());
        assert!(ConfigManager::validate_token("priv_").is_ok()); // technically valid prefix, though maybe useless
    }
}
