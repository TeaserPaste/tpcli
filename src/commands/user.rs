use anyhow::Result;
use comfy_table::Table;
use log::info;

use crate::api::ApiClient;
use crate::cli_args::{UserArgs, UserCmd};

pub fn handle_user(args: UserArgs, token: Option<String>, json_output: bool) -> Result<()> {
    match args.command {
        UserCmd::View { s } => {
            let client = ApiClient::new(token.clone());
            let user: crate::types::User =
                client.request("/getUserInfo", "GET", Option::<()>::None.as_ref())?;

            if json_output {
                if s {
                    // if s (snippets) is requested, we should probably include them in JSON?
                    // Or maybe just output user info + snippets list.
                    // The existing struct doesn't bundle them.
                    // Let's fetch snippets and create ad-hoc JSON.
                    let snippets: Vec<crate::types::Snippet> = client.request(
                        "/getUserPublicSnippets",
                        "POST",
                        Some(&serde_json::json!({
                            "userId": user.user_id
                        })),
                    )?;

                    let result = serde_json::json!({
                        "user": user,
                        "snippets": snippets
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&user)?);
                }
                return Ok(());
            }

            println!("\n=====================================");
            println!("USER PROFILE: {}", user.user_id);
            println!("=====================================");
            println!("Display Name: {}", user.display_name);
            println!("Photo URL: {}", user.photo_url.as_deref().unwrap_or("N/A"));
            println!("-------------------------------------\n");

            if s {
                println!("Loading public snippets for {}...", user.display_name);
                let snippets: Vec<crate::types::Snippet> = client.request(
                    "/getUserPublicSnippets",
                    "POST",
                    Some(&serde_json::json!({
                        "userId": user.user_id
                    })),
                )?;

                if snippets.is_empty() {
                    println!("\nThis user has no public snippets.\n");
                } else {
                    println!("\nPublic Snippets:");
                    let mut table = Table::new();
                    table.set_header(vec!["ID", "TITLE", "LANGUAGE"]);
                    for s in snippets {
                        table.add_row(vec![s.id, s.title, s.language]);
                    }
                    println!("{}", table);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_stats(token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token.clone());
    info!("Loading statistics...");

    let user_info: crate::types::User =
        client.request("/getUserInfo", "GET", Option::<()>::None.as_ref())?;
    let all_snippets: Vec<crate::types::Snippet> = client.request(
        "/listSnippets",
        "POST",
        Some(&serde_json::json!({
            "limit": 500,
            "includeDeleted": true
        })),
    )?;

    if all_snippets.is_empty() {
        println!("You do not have any snippets to display stats for.");
        return Ok(());
    }

    let total_snippets = all_snippets.len();

    let mut visibility_counts = std::collections::HashMap::new();
    let mut language_counts = std::collections::HashMap::new();

    for snippet in &all_snippets {
        *visibility_counts
            .entry(snippet.visibility.clone())
            .or_insert(0) += 1;
        *language_counts.entry(snippet.language.clone()).or_insert(0i32) += 1;
    }

    let mut top_languages: Vec<_> = language_counts.into_iter().collect();
    top_languages.sort_by(|a, b| b.1.cmp(&a.1));
    top_languages.truncate(5);

    println!("\n--- STATISTICS FOR USER: {} ---", user_info.display_name);
    println!("\n📊 Overview");

    let mut table = Table::new();
    table.set_header(vec![
        "Total Snippets",
        "Public",
        "Unlisted",
        "Private",
        "Deleted (In Trash)",
    ]); 
    table.add_row(vec![
        total_snippets.to_string(),
        visibility_counts.get("public").unwrap_or(&0).to_string(),
        visibility_counts.get("unlisted").unwrap_or(&0).to_string(),
        visibility_counts.get("private").unwrap_or(&0).to_string(),
        visibility_counts.get("deleted").unwrap_or(&0).to_string(), 
    ]);
    println!("{}", table);

    println!("\n🌐 Top 5 Languages");
    let mut lang_table = Table::new();
    lang_table.set_header(vec!["Language", "Count"]);
    for (lang, count) in top_languages {
        lang_table.add_row(vec![lang, count.to_string()]);
    }
    println!("{}", lang_table);

    Ok(())
}
