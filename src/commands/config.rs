use anyhow::{Result, anyhow};
use colored::Colorize;

use crate::cli_args::{ConfigArgs, ConfigCmd};
use crate::config::ConfigManager;
use crate::utils::normalize_lang;

pub fn handle_config(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCmd::Set { key, value } => {
            if key == "token" {
                ConfigManager::set_token(&value)?;
                println!("\n{}\n", "✅ Token has been saved securely!".green());
            } else if key == "default_language" {
                let mut config = ConfigManager::load_config()?;
                config.default_language = Some(normalize_lang(&value));
                ConfigManager::save_config(&config)?;
                println!(
                    "\n{}\n",
                    format!(
                        "✅ Default language set to '{}'.",
                        config.default_language.unwrap()
                    )
                    .green()
                );
            } else if key == "default_visibility" {
                let valid_visibilities = ["public", "private", "unlisted"];
                if !valid_visibilities.contains(&value.as_str()) {
                    return Err(anyhow!(
                        "Invalid visibility. Allowed values: public, private, unlisted"
                    ));
                }
                let mut config = ConfigManager::load_config()?;
                config.default_visibility = Some(value.clone());
                ConfigManager::save_config(&config)?;
                println!(
                    "\n{}\n",
                    format!("✅ Default visibility set to '{}'.", value).green()
                );
            } else {
                return Err(anyhow!(
                    "Invalid config key. Supported keys: 'token', 'default_language', 'default_visibility'."
                ));
            }
        }
        ConfigCmd::Get { key } => {
            if key == "token" {
                match ConfigManager::get_token()? {
                    Some(token) => println!("\n🔑 Current Token: {}\n", token),
                    None => println!("\nYou have not set a token.\n"),
                }
            } else if key == "default_language" {
                let config = ConfigManager::load_config()?;
                match config.default_language {
                    Some(lang) => println!("\nDefault Language: {}\n", lang),
                    None => println!("\nDefault language is not set.\n"),
                }
            } else if key == "default_visibility" {
                let config = ConfigManager::load_config()?;
                match config.default_visibility {
                    Some(vis) => println!("\nDefault Visibility: {}\n", vis),
                    None => println!("\nDefault visibility is not set.\n"),
                }
            } else {
                return Err(anyhow!(
                    "Invalid config key. Supported keys: 'token', 'default_language', 'default_visibility'."
                ));
            }
        }
        ConfigCmd::Clear { key } => {
            if key == "token" {
                ConfigManager::clear_token()?;
                println!("\n{}\n", "✅ Token has been cleared.".green());
            } else if key == "default_language" {
                let mut config = ConfigManager::load_config()?;
                config.default_language = None;
                ConfigManager::save_config(&config)?;
                println!("\n{}\n", "✅ Default language cleared.".green());
            } else if key == "default_visibility" {
                let mut config = ConfigManager::load_config()?;
                config.default_visibility = None;
                ConfigManager::save_config(&config)?;
                println!("\n{}\n", "✅ Default visibility cleared.".green());
            } else {
                return Err(anyhow!(
                    "Invalid config key. Supported keys: 'token', 'default_language', 'default_visibility'."
                ));
            }
        }
    }
    Ok(())
}
