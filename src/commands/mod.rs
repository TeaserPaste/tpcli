use crate::config::ConfigManager;
use anyhow::Result;

pub mod config;
pub mod run;
pub mod snippet;
pub mod user;

pub fn resolve_token(arg_token: Option<String>) -> Result<Option<String>> {
    if let Some(t) = arg_token {
        return Ok(Some(t));
    }
    ConfigManager::get_token()
}
