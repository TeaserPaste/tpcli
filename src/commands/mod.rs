use anyhow::Result;
use crate::config::ConfigManager;

pub mod snippet;
pub mod run;
pub mod user;
pub mod config;
pub mod system;

pub fn resolve_token(arg_token: Option<String>) -> Result<Option<String>> {
    if let Some(t) = arg_token {
        return Ok(Some(t));
    }
    ConfigManager::get_token()
}
