use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use anyhow::{Result, anyhow};
use arboard::Clipboard;
use colored::Colorize;
use comfy_table::Table;
use inquire::{Confirm, Password, Select, Text};
use log::{debug, info};
use tempfile::tempdir;

use crate::api::ApiClient;
use crate::cli_args::{
    CloneArgs, CopyArgs, CreateArgs, DeleteArgs, EditArgs, ListArgs,
    RestoreArgs, SearchArgs, StarArgs, UpdateArgs, ViewArgs,
};
use crate::config::ConfigManager;
use crate::types::{
    CreateSnippetRequest, ListSnippetsRequest, SearchSnippetsRequest, UpdateSnippetRequest,
};
use crate::utils::{
    get_file_extension, get_lang_from_extension, normalize_lang, sanitize_filename,
};

pub fn handle_edit(args: EditArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);

    // 1. Get Snippet
    let snippet: crate::types::Snippet = client.request(
        "/getSnippet",
        "POST",
        Some(&serde_json::json!({
            "snippetId": args.id,
            "password": args.password
        })),
    )?;

    // 2. Create temp file
    let ext = get_file_extension(&snippet.language);
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join(format!("snippet{}", ext));

    debug!("Creating temp file for editing: {}", file_path.display());
    fs::write(&file_path, &snippet.content)?;

    // 3. Edit file
    info!("Opening external editor...");
    edit::edit_file(&file_path)?;

    let edited = fs::read_to_string(&file_path)?;

    // 4. Compare and Update
    if edited != snippet.content {
        // It changed
        let req = UpdateSnippetRequest {
            snippet_id: args.id,
            updates: serde_json::json!({
                "content": edited
            }),
        };
        client.request::<UpdateSnippetRequest, crate::types::Snippet>(
            "/updateSnippet",
            "PATCH",
            Some(&req),
        )?;
        println!("\n{}\n", "✅ Snippet content updated successfully!".green());
    } else {
        println!("\n{}\n", "ℹ️ No changes detected.".blue());
    }

    Ok(())
}

pub fn handle_view(args: ViewArgs, token: Option<String>, json_output: bool) -> Result<()> {
    if args.url {
        println!("\nhttps://paste.teaserverse.online/snippet/{}\n", args.id);
        return Ok(());
    }

    let client = ApiClient::new(token);
    let snippet: crate::types::Snippet = client.request(
        "/getSnippet",
        "POST",
        Some(&serde_json::json!({
            "snippetId": args.id,
            "password": args.password
        })),
    )?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&snippet)?);
        return Ok(());
    }

    if args.raw {
        print!("{}", snippet.content);
        io::stdout().flush()?;
        return Ok(());
    }

    if args.copy {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(&snippet.content)?;
        println!("\n{}\n", "✅ Copied snippet content to clipboard!".green());
        return Ok(());
    }

    println!("\n=====================================");
    println!("TEASERPASTE SNIPPET: {}", snippet.id);
    println!("=====================================");
    println!("Title: {}", snippet.title);
    if let Some(true) = snippet.is_verified {
        println!("⭐ VERIFIED SNIPPET");
    }
    if let Some(true) = snippet.password_bypassed {
        println!("🔑 Password bypassed because you are the owner.");
    }
    println!(
        "Creator: {}",
        snippet.creator_name.as_deref().unwrap_or("Unknown")
    );
    println!("Language: {}", snippet.language);
    if let Some(tags) = snippet.tags {
        println!("Tags: {}", tags.join(", "));
    }
    println!("Visibility: {}", snippet.visibility);
    println!("-------------------------------------");
    println!("{}", snippet.content);
    println!("-------------------------------------\n");

    Ok(())
}

pub fn handle_clone(args: CloneArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    let snippet: crate::types::Snippet = client.request(
        "/getSnippet",
        "POST",
        Some(&serde_json::json!({
            "snippetId": args.id,
            "password": args.password
        })),
    )?;

    let correct_extension = get_file_extension(&snippet.language);
    let base_filename = if let Some(filename) = args.filename {
        let path = Path::new(&filename);
        let user_extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e));

        if let Some(ext) = user_extension {
            if ext != correct_extension {
                println!("\n{}\n", format!("⚠️ Warning: File extension ('{}') doesn't match language ('{}'). Saving with correct extension '{}'.", ext, snippet.language, correct_extension).yellow());
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("snippet")
                    .to_string()
            } else {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("snippet")
                    .to_string()
            }
        } else {
            filename
        }
    } else {
        sanitize_filename(&snippet.title)
    };

    let output_filename = format!("{}{}", base_filename, correct_extension);
    debug!("Writing snippet content to file: {}", output_filename);
    fs::write(&output_filename, snippet.content)?;
    println!(
        "\n{}\n",
        format!("✅ Snippet successfully saved to file: {}", output_filename).green()
    );
    Ok(())
}

pub fn handle_create(args: CreateArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    let config = ConfigManager::load_config().unwrap_or_default();
    let snippet_data: CreateSnippetRequest;

    if args.interactive {
        let title = Text::new("Snippet title:")
            .with_initial_value("Untitled")
            .prompt()?;
        let language_input = Text::new("Language:")
            .with_initial_value("plaintext")
            .prompt()?;
        let language = normalize_lang(&language_input);
        let visibility = Select::new(
            "Visibility:",
            vec![
                "Unlisted (Link only)",
                "Public (Searchable)",
                "Private (You only)",
            ],
        )
        .prompt()?;

        let visibility_val = match visibility {
            "Unlisted (Link only)" => "unlisted",
            "Public (Searchable)" => "public",
            "Private (You only)" => "private",
            _ => "unlisted",
        };

        let password = if visibility_val == "unlisted" {
            let p = Password::new("Password (optional):").prompt()?;
            if p.is_empty() { None } else { Some(p) }
        } else {
            None
        };

        let tags_str = Text::new("Tags (comma-separated):")
            .with_placeholder("e.g., code, tutorial")
            .prompt()?;
        let tags = if tags_str.is_empty() {
            None
        } else {
            Some(tags_str.split(',').map(|s| s.trim().to_string()).collect())
        };

        let expires_str = Text::new("Expiration (e.g., 1h, 7d, 2w):")
            .with_placeholder("Leave empty for never")
            .prompt()?;
        let expires = if expires_str.is_empty() {
            None
        } else {
            Some(expires_str)
        };

        let content_source = Select::new(
            "Content source:",
            vec!["Open default editor", "Import from file"],
        )
        .prompt()?;

        let content = if content_source == "Import from file" {
            let file_path = Text::new("Path to file:").prompt()?;
            fs::read_to_string(file_path)?
        } else {
            // Edit crate
            edit::edit("")?
        };

        snippet_data = CreateSnippetRequest {
            title,
            language,
            visibility: visibility_val.to_string(),
            password,
            tags,
            expires,
            content,
        };
    } else if let Some(file_path) = args.file {
        let content = fs::read_to_string(&file_path)?;
        let ext = Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang_from_file = get_lang_from_extension(&format!(".{}", ext));

        let language = args
            .language
            .map(|l| normalize_lang(&l))
            .unwrap_or_else(|| {
                println!("ℹ️ Auto-detected language: {}", lang_from_file);
                lang_from_file.to_string()
            });

        snippet_data = CreateSnippetRequest {
            title: args.title.unwrap_or_else(|| "Untitled".to_string()),
            content,
            language,
            visibility: args.visibility.unwrap_or_else(|| "unlisted".to_string()),
            password: args.password,
            tags: args.tags,
            expires: args.expires,
        };
    } else {
        // Checking if stdin has data is hard in a cross-platform reliable way without blocking.
        // But clap doesn't easily support "if not interactive and no file, read stdin".
        // We will assume if title and content are missing, and no interactive flag, we error or read stdin if provided.
        // Since `ureq` and `inquire` are blocking, let's just try to read stdin if content is missing.
        // Actually, let's just error if required args are missing for non-interactive.

        // Wait, the JS version reads from stdin if !isTTY.
        // In Rust, we can check `atty::is(Stream::Stdin)`.
        let content = if let Some(c) = args.content {
            c
        } else if !atty::is(atty::Stream::Stdin) {
            debug!("Reading content from stdin...");
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer.trim().to_string()
        } else {
            return Err(anyhow!(
                "Missing --title and --content. Use -i (interactive) or --file <path>."
            ));
        };

        if content.is_empty() {
            return Err(anyhow!("Content cannot be empty."));
        }

        let language = args
            .language
            .map(|l| normalize_lang(&l))
            .or(config.default_language)
            .unwrap_or_else(|| "plaintext".to_string());

        let visibility = args
            .visibility
            .or(config.default_visibility)
            .unwrap_or_else(|| "unlisted".to_string());

        snippet_data = CreateSnippetRequest {
            title: args.title.unwrap_or_else(|| "Untitled".to_string()),
            content,
            language,
            visibility,
            password: args.password,
            tags: args.tags,
            expires: args.expires,
        };
    }

    let response: crate::types::CreateSnippetResponse =
        client.request("/createSnippet", "POST", Some(&snippet_data))?;
    println!(
        "\n{}\n",
        format!("✅ Successfully created snippet! ID: {}", response.id).green()
    );
    Ok(())
}

pub fn handle_list(args: ListArgs, token: Option<String>, json_output: bool) -> Result<()> {
    let client = ApiClient::new(token);
    let req = ListSnippetsRequest {
        limit: args.limit.unwrap_or(20),
        visibility: args.visibility,
        include_deleted: args.include_deleted,
    };

    let snippets: Vec<crate::types::Snippet> =
        client.request("/listSnippets", "POST", Some(&req))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&snippets)?);
        return Ok(());
    }

    if snippets.is_empty() {
        println!("\nNo snippets found.\n");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["ID", "TITLE", "VISIBILITY", "LANGUAGE"]);
    for s in snippets {
        table.add_row(vec![s.id, s.title, s.visibility, s.language]);
    }
    println!("{}", table);
    Ok(())
}

pub fn handle_update(args: UpdateArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);

    let mut updates = serde_json::Map::new();
    if let Some(v) = args.title {
        updates.insert("title".to_string(), v.into());
    }
    if let Some(v) = args.content {
        updates.insert("content".to_string(), v.into());
    }
    if let Some(v) = args.language {
        updates.insert("language".to_string(), normalize_lang(&v).into());
    }
    if let Some(v) = args.visibility {
        updates.insert("visibility".to_string(), v.into());
    }
    if let Some(v) = args.password {
        updates.insert("password".to_string(), v.into());
    }
    if let Some(v) = args.tags {
        updates.insert("tags".to_string(), v.into());
    }
    if let Some(v) = args.expires {
        updates.insert("expires".to_string(), v.into());
    }

    if updates.is_empty() {
        return Err(anyhow!(
            "Must provide at least one field to update (e.g., --title \"New Title\")."
        ));
    }

    let req = UpdateSnippetRequest {
        snippet_id: args.id,
        updates: serde_json::Value::Object(updates),
    };

    let updated_snippet: crate::types::Snippet =
        client.request("/updateSnippet", "PATCH", Some(&req))?;
    println!("\n✅ Snippet successfully updated!");

    // Print snippet details
    println!("Title: {}", updated_snippet.title);
    println!("Language: {}", updated_snippet.language);
    println!("Visibility: {}", updated_snippet.visibility);
    Ok(())
}

pub fn handle_delete(args: DeleteArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);

    if atty::is(atty::Stream::Stdin) {
        let ans = Confirm::new(&format!(
            "Are you sure you want to delete snippet '{}'?",
            args.id
        ))
        .with_default(false)
        .prompt()?;

        if !ans {
            println!("\nDelete operation canceled.\n");
            return Ok(());
        }
    }

    let res: crate::types::DeleteSnippetResponse = client.request(
        "/deleteSnippet",
        "DELETE",
        Some(&serde_json::json!({ "snippetId": args.id })),
    )?;
    println!("\n{}\n", format!("✅ {}", res.message).green());
    Ok(())
}

pub fn handle_restore(args: RestoreArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    if atty::is(atty::Stream::Stdin) {
        let ans = Confirm::new(&format!(
            "Are you sure you want to restore snippet '{}' from the trash?",
            args.id
        ))
        .with_default(true)
        .prompt()?;

        if !ans {
            println!("\nOperation canceled.\n");
            return Ok(());
        }
    }

    let res: crate::types::RestoreSnippetResponse = client.request(
        "/restoreSnippet",
        "POST",
        Some(&serde_json::json!({ "snippetId": args.id })),
    )?;
    println!("\n{}\n", format!("✅ {}", res.message).green());
    Ok(())
}

pub fn handle_star(args: StarArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    let star = !args.unstar;
    let res: crate::types::StarSnippetResponse = client.request(
        "/starSnippet",
        "POST",
        Some(&serde_json::json!({ "snippetId": args.id, "star": star })),
    )?;

    match res.status.as_str() {
        "starred" => println!(
            "\n{}\n",
            format!("⭐ Snippet starred! (Total stars: {})", res.star_count).yellow()
        ),
        "unstarred" => println!(
            "\n{}\n",
            format!("💔 Snippet unstarred. (Total stars: {})", res.star_count).yellow()
        ),
        "already_starred" | "already_unstarred" => println!(
            "\n{}\n",
            format!(
                "ℹ️ Snippet was already in this state. (Total stars: {})",
                res.star_count
            )
            .blue()
        ),
        _ => println!(
            "\n{}\n",
            format!("✅ Star status updated. (Total stars: {})", res.star_count).green()
        ),
    }
    Ok(())
}

pub fn handle_search(args: SearchArgs, token: Option<String>, json_output: bool) -> Result<()> {
    let client = ApiClient::new(token);

    if !json_output {
        info!(
            "Searching for \"{}\" (limit: {}, from: {})...",
            args.term, args.limit, args.from
        );
    }

    let res: crate::types::SearchSnippetsResponse = client.request(
        "/searchSnippets",
        "POST",
        Some(&SearchSnippetsRequest {
            term: args.term,
            size: args.limit,
            from: args.from,
        }),
    )?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

    if res.hits.is_empty() {
        println!("\nNo matching results found.\n");
        return Ok(());
    }

    println!(
        "\nFound {} total results. Displaying {} results:",
        res.total,
        res.hits.len()
    );
    let mut table = Table::new();
    table.set_header(vec!["ID", "TITLE", "CREATOR", "LANGUAGE"]);
    for hit in res.hits {
        table.add_row(vec![
            hit.id,
            hit.title,
            hit.creator_name.unwrap_or_default(),
            hit.language,
        ]);
    }
    println!("{}", table);
    Ok(())
}

pub fn handle_copy(args: CopyArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    println!(
        "\nSending \"copy\" (fork) request for snippet '{}'...",
        args.id
    );

    let res: crate::types::CopySnippetResponse = client.request(
        "/copySnippet",
        "POST",
        Some(&serde_json::json!({
            "snippetId": args.id,
            "password": args.password
        })),
    )?;

    println!("\n{}", format!("✅ {}", res.message).green());
    println!("New snippet ID (private): {}", res.new_snippet_id);
    println!(
        "URL: https://paste.teaserverse.online/snippet/{}\n",
        res.new_snippet_id
    );
    Ok(())
}
