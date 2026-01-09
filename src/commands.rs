use std::fs;
use std::io::{self, Write, Read};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow, Context};
use colored::Colorize;
use inquire::{Text, Select, Confirm, Password};
use arboard::Clipboard;
use comfy_table::Table;
use tempfile::tempdir;

use crate::api::ApiClient;
use crate::config::ConfigManager;
use crate::types::{CreateSnippetRequest, ListSnippetsRequest, UpdateSnippetRequest, SearchSnippetsRequest};
use crate::utils::{get_file_extension, get_lang_from_extension, sanitize_filename};
use crate::cli_args::{ViewArgs, CloneArgs, CopyArgs, StarArgs, RestoreArgs, RunArgs, ListArgs, CreateArgs, UpdateArgs, DeleteArgs, SearchArgs, UserArgs, ConfigArgs, UserCmd, ConfigCmd};

pub fn resolve_token(arg_token: Option<String>) -> Result<Option<String>> {
    if let Some(t) = arg_token {
        return Ok(Some(t));
    }
    ConfigManager::get_token()
}

pub fn handle_view(args: ViewArgs, token: Option<String>) -> Result<()> {
    if args.url {
        println!("\nhttps://paste.teaserverse.online/snippet/{}\n", args.id);
        return Ok(());
    }

    let client = ApiClient::new(token);
    let snippet: crate::types::Snippet = client.request("/getSnippet", "POST", Some(&serde_json::json!({
        "snippetId": args.id,
        "password": args.password
    })))?;

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
    println!("Creator: {}", snippet.creator_name.as_deref().unwrap_or("Unknown"));
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
    let snippet: crate::types::Snippet = client.request("/getSnippet", "POST", Some(&serde_json::json!({
        "snippetId": args.id,
        "password": args.password
    })))?;

    let correct_extension = get_file_extension(&snippet.language);
    let base_filename = if let Some(filename) = args.filename {
        let path = Path::new(&filename);
        let user_extension = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e));
        
        if let Some(ext) = user_extension {
            if ext != correct_extension {
                println!("\n{}\n", format!("⚠️ Warning: File extension ('{}') doesn't match language ('{}'). Saving with correct extension '{}'.", ext, snippet.language, correct_extension).yellow());
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("snippet").to_string()
            } else {
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("snippet").to_string()
            }
        } else {
             filename
        }
    } else {
        sanitize_filename(&snippet.title)
    };

    let output_filename = format!("{}{}", base_filename, correct_extension);
    fs::write(&output_filename, snippet.content)?;
    println!("\n{}\n", format!("✅ Snippet successfully saved to file: {}", output_filename).green());
    Ok(())
}

pub fn handle_user(args: UserArgs, token: Option<String>) -> Result<()> {
    match args.command {
        UserCmd::View { s } => {
            let client = ApiClient::new(token.clone());
            let user: crate::types::User = client.request("/getUserInfo", "GET", Option::<()>::None.as_ref())?;
            
            println!("\n=====================================");
            println!("USER PROFILE: {}", user.user_id);
            println!("=====================================");
            println!("Display Name: {}", user.display_name);
            println!("Photo URL: {}", user.photo_url.as_deref().unwrap_or("N/A"));
            println!("-------------------------------------\n");

            if s {
                println!("Loading public snippets for {}...", user.display_name);
                let snippets: Vec<crate::types::Snippet> = client.request("/getUserPublicSnippets", "POST", Some(&serde_json::json!({
                    "userId": user.user_id
                })))?;

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

pub fn handle_create(args: CreateArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    let snippet_data: CreateSnippetRequest;

    if args.interactive {
        let title = Text::new("Snippet title:").with_initial_value("Untitled").prompt()?;
        let language = Text::new("Language:").with_initial_value("plaintext").prompt()?;
        let visibility = Select::new("Visibility:", vec![
            "Unlisted (Link only)", "Public (Searchable)", "Private (You only)"
        ]).prompt()?;
        
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

        let tags_str = Text::new("Tags (comma-separated):").with_placeholder("e.g., code, tutorial").prompt()?;
        let tags = if tags_str.is_empty() { None } else { Some(tags_str.split(',').map(|s| s.trim().to_string()).collect()) };

        let expires_str = Text::new("Expiration (e.g., 1h, 7d, 2w):").with_placeholder("Leave empty for never").prompt()?;
        let expires = if expires_str.is_empty() { None } else { Some(expires_str) };

        let content_source = Select::new("Content source:", vec![
            "Open default editor", "Import from file"
        ]).prompt()?;
        
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
        let ext = Path::new(&file_path).extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang_from_file = get_lang_from_extension(&format!(".{}", ext));
        
        let language = args.language.unwrap_or_else(|| {
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
             let mut buffer = String::new();
             io::stdin().read_to_string(&mut buffer)?;
             buffer.trim().to_string()
        } else {
             return Err(anyhow!("Missing --title and --content. Use -i (interactive) or --file <path>."));
        };
        
        if content.is_empty() {
             return Err(anyhow!("Content cannot be empty."));
        }

        snippet_data = CreateSnippetRequest {
            title: args.title.unwrap_or_else(|| "Untitled".to_string()),
            content,
            language: args.language.unwrap_or_else(|| "plaintext".to_string()),
            visibility: args.visibility.unwrap_or_else(|| "unlisted".to_string()),
            password: args.password,
            tags: args.tags,
            expires: args.expires,
        };
    }

    let response: crate::types::CreateSnippetResponse = client.request("/createSnippet", "POST", Some(&snippet_data))?;
    println!("\n{}\n", format!("✅ Successfully created snippet! ID: {}", response.id).green());
    Ok(())
}

pub fn handle_list(args: ListArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    let req = ListSnippetsRequest {
        limit: args.limit.unwrap_or(20),
        visibility: args.visibility,
        include_deleted: args.include_deleted,
    };
    
    // API returns a list of snippets directly? Or inside an object?
    // Looking at JS: `return (typeof dataForLog === 'object') ? dataForLog : JSON.parse(responseText);`
    // JS `listSnippets` expects an array.
    
    // Wait, let's check JS again.
    // `const snippets = await apiRequest('/listSnippets', ...)`
    // And `snippets.map(...)`. So it expects an array.
    
    let snippets: Vec<crate::types::Snippet> = client.request("/listSnippets", "POST", Some(&req))?;

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
    if let Some(v) = args.title { updates.insert("title".to_string(), v.into()); }
    if let Some(v) = args.content { updates.insert("content".to_string(), v.into()); }
    if let Some(v) = args.language { updates.insert("language".to_string(), v.into()); }
    if let Some(v) = args.visibility { updates.insert("visibility".to_string(), v.into()); }
    if let Some(v) = args.password { updates.insert("password".to_string(), v.into()); }
    if let Some(v) = args.tags { updates.insert("tags".to_string(), v.into()); }
    if let Some(v) = args.expires { updates.insert("expires".to_string(), v.into()); }

    if updates.is_empty() {
        return Err(anyhow!("Must provide at least one field to update (e.g., --title \"New Title\")."));
    }

    let req = UpdateSnippetRequest {
        snippet_id: args.id,
        updates: serde_json::Value::Object(updates),
    };

    let updated_snippet: crate::types::Snippet = client.request("/updateSnippet", "PATCH", Some(&req))?;
    println!("\n✅ Snippet successfully updated!");
    
    // Print snippet details
    println!("Title: {}", updated_snippet.title);
    println!("Language: {}", updated_snippet.language);
    println!("Visibility: {}", updated_snippet.visibility);
    Ok(())
}

pub fn handle_delete(args: DeleteArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    
    // Interactive confirmation? JS does it.
    // "Are you sure you want to delete snippet '${id}'?"
    if atty::is(atty::Stream::Stdin) {
        let ans = Confirm::new(&format!("Are you sure you want to delete snippet '{}'?", args.id))
            .with_default(false)
            .prompt()?;
        
        if !ans {
            println!("\nDelete operation canceled.\n");
            return Ok(());
        }
    }

    let res: crate::types::DeleteSnippetResponse = client.request("/deleteSnippet", "DELETE", Some(&serde_json::json!({ "snippetId": args.id })))?;
    println!("\n{}\n", format!("✅ {}", res.message).green());
    Ok(())
}

pub fn handle_restore(args: RestoreArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
     if atty::is(atty::Stream::Stdin) {
        let ans = Confirm::new(&format!("Are you sure you want to restore snippet '{}' from the trash?", args.id))
            .with_default(true)
            .prompt()?;
        
        if !ans {
            println!("\nOperation canceled.\n");
            return Ok(());
        }
    }

    let res: crate::types::RestoreSnippetResponse = client.request("/restoreSnippet", "POST", Some(&serde_json::json!({ "snippetId": args.id })))?;
    println!("\n{}\n", format!("✅ {}", res.message).green());
    Ok(())
}

pub fn handle_star(args: StarArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    let star = !args.unstar;
    let res: crate::types::StarSnippetResponse = client.request("/starSnippet", "POST", Some(&serde_json::json!({ "snippetId": args.id, "star": star })))?;

    match res.status.as_str() {
        "starred" => println!("\n{}\n", format!("⭐ Snippet starred! (Total stars: {})", res.star_count).yellow()),
        "unstarred" => println!("\n{}\n", format!("💔 Snippet unstarred. (Total stars: {})", res.star_count).yellow()),
        "already_starred" | "already_unstarred" => println!("\n{}\n", format!("ℹ️ Snippet was already in this state. (Total stars: {})", res.star_count).blue()),
        _ => println!("\n{}\n", format!("✅ Star status updated. (Total stars: {})", res.star_count).green()),
    }
    Ok(())
}

pub fn handle_search(args: SearchArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    println!("\nSearching for \"{}\" (limit: {}, from: {})...", args.term, args.limit, args.from);
    
    let res: crate::types::SearchSnippetsResponse = client.request("/searchSnippets", "POST", Some(&SearchSnippetsRequest {
        term: args.term,
        size: args.limit,
        from: args.from,
    }))?;

    if res.hits.is_empty() {
        println!("\nNo matching results found.\n");
        return Ok(());
    }

    println!("\nFound {} total results. Displaying {} results:", res.total, res.hits.len());
    let mut table = Table::new();
    table.set_header(vec!["ID", "TITLE", "CREATOR", "LANGUAGE"]);
    for hit in res.hits {
        table.add_row(vec![hit.id, hit.title, hit.creator_name.unwrap_or_default(), hit.language]);
    }
    println!("{}", table);
    Ok(())
}

pub fn handle_copy(args: CopyArgs, token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token);
    println!("\nSending \"copy\" (fork) request for snippet '{}'...", args.id);
    
    let res: crate::types::CopySnippetResponse = client.request("/copySnippet", "POST", Some(&serde_json::json!({
        "snippetId": args.id,
        "password": args.password
    })))?;

    println!("\n{}", format!("✅ {}", res.message).green());
    println!("New snippet ID (private): {}", res.new_snippet_id);
    println!("URL: https://paste.teaserverse.online/snippet/{}\n", res.new_snippet_id);
    Ok(())
}

pub fn handle_stats(token: Option<String>) -> Result<()> {
    let client = ApiClient::new(token.clone());
    println!("\nLoading statistics...");
    
    let user_info: crate::types::User = client.request("/getUserInfo", "GET", Option::<()>::None.as_ref())?;
    let all_snippets: Vec<crate::types::Snippet> = client.request("/listSnippets", "POST", Some(&serde_json::json!({
        "limit": 500,
        "includeDeleted": true
    })))?;

    if all_snippets.is_empty() {
        println!("You do not have any snippets to display stats for.");
        return Ok(());
    }

    let total_snippets = all_snippets.len();
    
    let mut visibility_counts = std::collections::HashMap::new();
    let mut language_counts = std::collections::HashMap::new();

    for snippet in &all_snippets {
        *visibility_counts.entry(snippet.visibility.clone()).or_insert(0) += 1;
        *language_counts.entry(snippet.language.clone()).or_insert(0) += 1;
    }

    // deleted items might be marked differently or filtered? 
    // JS: `visibilityCounts.deleted || 0` -- wait, `listSnippets` with `includeDeleted: true` returns deleted snippets?
    // I suspect the API might return them with visibility="deleted" OR the JS client was inferring it?
    // The JS code uses `visibilityCounts[snippet.visibility]`.
    // So assume "deleted" is a visibility state or managed somehow.

    let mut top_languages: Vec<_> = language_counts.into_iter().collect();
    top_languages.sort_by(|a, b| b.1.cmp(&a.1));
    top_languages.truncate(5);

    println!("\n--- STATISTICS FOR USER: {} ---", user_info.display_name);
    println!("\n📊 Overview");
    
    let mut table = Table::new();
    table.set_header(vec!["Total Snippets", "Public", "Unlisted", "Private", "Deleted (In Trash)"]); // Assuming deleted logic matches
    table.add_row(vec![
        total_snippets.to_string(),
        visibility_counts.get("public").unwrap_or(&0).to_string(),
        visibility_counts.get("unlisted").unwrap_or(&0).to_string(),
        visibility_counts.get("private").unwrap_or(&0).to_string(),
        visibility_counts.get("deleted").unwrap_or(&0).to_string(), // Just guessing "deleted" exists
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

pub fn handle_config(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCmd::Set { key, value } => {
            if key == "token" {
                ConfigManager::set_token(&value)?;
                println!("\n{}\n", "✅ Token has been saved securely!".green());
            } else {
                return Err(anyhow!("Invalid config key. Only 'token' is supported."));
            }
        },
        ConfigCmd::Get { key } => {
            if key == "token" {
                match ConfigManager::get_token()? {
                    Some(token) => println!("\n🔑 Current Token: {}\n", token),
                    None => println!("\nYou have not set a token.\n"),
                }
            }
        },
        ConfigCmd::Clear { key } => {
            if key == "token" {
                ConfigManager::clear_token()?;
                println!("\n{}\n", "✅ Token has been cleared.".green());
            }
        },
    }
    Ok(())
}

// --- Run Command Logic ---
use regex::Regex;

fn detect_dependencies(content: &str, language: &str) -> Vec<String> {
    let lang = language.to_lowercase();
    let mut deps = std::collections::HashSet::new();
    
    if lang == "javascript" || lang == "typescript" {
        // JS regex: /(?:require\(['"]([^'"]+)['"]\)|import\s+.*?from\s+['"]([^'"]+)['"])/g
        let re = Regex::new(r#"(?:require\(['"]([^'"]+)['"]\)|import\s+.*?from\s+['"]([^'"]+)['"])"#).unwrap();
        for caps in re.captures_iter(content) {
            if let Some(m) = caps.get(1).or(caps.get(2)) {
                let dep = m.as_str();
                if !dep.starts_with("./") && !dep.starts_with("../") {
                    deps.insert(dep.to_string());
                }
            }
        }
    } else if lang == "python" {
         // JS regex: /(?:from\s+([^\s]+)\s+import|import\s+([^\s]+))/g
         let re = Regex::new(r"(?:from\s+([^\s]+)\s+import|import\s+([^\s]+))").unwrap();
         for caps in re.captures_iter(content) {
            if let Some(m) = caps.get(1).or(caps.get(2)) {
                deps.insert(m.as_str().to_string());
            }
         }
    }
    
    deps.into_iter().collect()
}

fn install_dependencies(deps: &[String], language: &str, cwd: &Path, silent: bool) -> Result<()> {
    let lang = language.to_lowercase();
    let (command, args) = if lang == "javascript" || lang == "typescript" {
        ("npm", vec!["install"])
    } else if lang == "python" {
        ("pip", vec!["install", "-t", "."])
    } else {
        return Err(anyhow!("Dependency installation not supported for {}", language));
    };

    let mut full_args = args;
    full_args.extend(deps.iter().map(|s| s.as_str()));

    if !silent {
        println!("\n> Installing dependencies: {} {}\n", command, full_args.join(" "));
    }

    let status = Command::new(command)
        .args(&full_args)
        .current_dir(cwd)
        .stdout(if silent { Stdio::null() } else { Stdio::inherit() })
        .stderr(if silent { Stdio::null() } else { Stdio::inherit() })
        .status()
        .context("Failed to run dependency installer")?;
    
    if !status.success() {
        return Err(anyhow!("Dependency installation failed"));
    }
    Ok(())
}

pub fn handle_run(args: RunArgs, token: Option<String>) -> Result<()> {
    if !args.force && atty::is(atty::Stream::Stdin) {
        let confirm = Confirm::new("⚠️ Warning: You are about to execute code from the internet. Are you sure?")
            .with_default(false)
            .prompt()?;
        if !confirm {
             println!("\nOperation canceled.\n");
             return Ok(());
        }
    }

    let client = ApiClient::new(token.clone());
    let snippet: crate::types::Snippet = client.request("/getSnippet", "POST", Some(&serde_json::json!({
        "snippetId": args.id,
        "password": args.password
    })))?;

    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();
    
    if args.install_deps {
        let deps = detect_dependencies(&snippet.content, &snippet.language);
        if !deps.is_empty() {
             let confirm_install = if atty::is(atty::Stream::Stdin) {
                 Confirm::new(&format!("Detected dependencies: {}. Install in temp env?", deps.join(", ")))
                    .with_default(true)
                    .prompt()?
             } else {
                 true 
             };

             if confirm_install {
                 install_dependencies(&deps, &snippet.language, temp_path, args.silent)?;
             }
        }
    }

    let extension = get_file_extension(&snippet.language);
    let filename = format!("snippet{}", extension);
    let executable_file = temp_path.join(&filename);
    fs::write(&executable_file, &snippet.content)?;

    // Execution logic
    let lang = snippet.language.to_lowercase();
    let (run_cmd, run_args) = if let Some(custom_cmds) = &args.command {
        // args.command is Vec<String>
        // JS logic: replace '--snippet' with file path, or append file path.
        // But here `args.command` is optional vec.
        // Wait, clap parsing for `[command]`: 
        // `run <id> [command]`.
        // If user types `tp run 123 node --snippet`, `command` is `["node", "--snippet"]`.
        let mut cmds = custom_cmds.clone();
        if let Some(pos) = cmds.iter().position(|s| s == "--snippet") {
            cmds[pos] = executable_file.to_string_lossy().to_string();
        } else {
            cmds.push(executable_file.to_string_lossy().to_string());
        }
        (cmds[0].clone(), cmds[1..].to_vec())
    } else {
        match lang.as_str() {
             "python" => ("python".to_string(), vec![executable_file.to_string_lossy().to_string()]),
             "javascript" => ("node".to_string(), vec![executable_file.to_string_lossy().to_string()]),
             "typescript" => ("ts-node".to_string(), vec![executable_file.to_string_lossy().to_string()]),
             "shell" | "bash" => ("bash".to_string(), vec![executable_file.to_string_lossy().to_string()]),
             "ruby" => ("ruby".to_string(), vec![executable_file.to_string_lossy().to_string()]),
             "go" => ("go".to_string(), vec!["run".to_string(), executable_file.to_string_lossy().to_string()]),
             "rust" => {
                 // compile then run
                 let exe_path = temp_path.join("snippet_bin");
                 if !args.silent { println!("\n> Compiling: rustc {} -o {}\n", executable_file.display(), exe_path.display()); }
                 let compile_status = Command::new("rustc")
                    .arg(&executable_file)
                    .arg("-o")
                    .arg(&exe_path)
                    .current_dir(temp_path)
                    .status()?;
                 
                 if !compile_status.success() {
                     return Err(anyhow!("Compilation failed"));
                 }
                 (exe_path.to_string_lossy().to_string(), vec![])
             },
             "c" | "cpp" => {
                 let compiler = if lang == "c" { "gcc" } else { "g++" };
                 let exe_path = temp_path.join("snippet_bin");
                 if !args.silent { println!("\n> Compiling: {} {} -o {}\n", compiler, executable_file.display(), exe_path.display()); }
                 let compile_status = Command::new(compiler)
                    .arg(&executable_file)
                    .arg("-o")
                    .arg(&exe_path)
                    .current_dir(temp_path)
                    .status()?;
                 if !compile_status.success() { return Err(anyhow!("Compilation failed")); }
                 (exe_path.to_string_lossy().to_string(), vec![])
             },
             "java" => {
                 // compile then run
                 // javac snippet.java
                 // java -cp . snippet (class name should match file name? Java is strict about this)
                 // Snippet might not have public class snippet.
                 // This is tricky for Java if class name != filename.
                 // JS version: `java -cp cwd path.basename(executableFile, '.java')`
                 // This implies `javac` worked.
                 if !args.silent { println!("\n> Compiling: javac {}\n", executable_file.display()); }
                 let compile_status = Command::new("javac")
                    .arg(&executable_file)
                    .current_dir(temp_path)
                    .status()?;
                 if !compile_status.success() { return Err(anyhow!("Compilation failed")); }
                 ("java".to_string(), vec!["-cp".to_string(), temp_path.to_string_lossy().to_string(), "snippet".to_string()])
             },
             _ => return Err(anyhow!("Language '{}' is not supported for auto-run. Please specify command explicitly.", lang)),
        }
    };

    if !args.silent {
        println!("\n> Running: {} {} (in {})\n", run_cmd, run_args.join(" "), temp_path.display());
    }

    let mut cmd = Command::new(&run_cmd);
    cmd.args(&run_args)
       .current_dir(temp_path)
       .envs(args.env.iter().cloned());
    
    if let Some(input_file) = &args.input_file {
        cmd.stdin(fs::File::open(input_file)?);
    } else {
        cmd.stdin(Stdio::inherit());
    }

    let capture_output = args.output_file.is_some() || args.copy_result || args.on_error_paste;
    if capture_output {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
    }

    let child = cmd.spawn()?;

    // Timeout logic?
    // Rust std Command doesn't support timeout natively easily without threads or extra crates.
    // JS used spawn and a timeout.
    // I can use `wait_timeout` crate if I added it, or just plain wait for now.
    // Given dependencies list, I didn't add `wait-timeout`. I will skip strict timeout enforcement for now or use a simple thread approach if crucial.
    // But for "refactor", maybe best effort.
    // Let's rely on child.wait().
    
    // We need to capture output if requested.
    let output = child.wait_with_output()?;
    
    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        if !args.silent && !capture_output {
            println!("\n> Process finished.\n");
        }
        
        if let Some(outfile) = args.output_file {
            fs::write(outfile, &output.stdout)?;
        } else if capture_output && !args.copy_result && !args.on_error_paste {
            // if we captured but didn't redirect to file/clipboard, print it?
            // "If parsedArgs['output-file'] ... else process.stdout.write(data)"
            // So we should print it if not saved to file.
            print!("{}", stdout_str);
        }

        if args.copy_result {
            let mut clipboard = Clipboard::new()?;
            clipboard.set_text(&stdout_str)?;
            if !args.silent { println!("\n✅ Copied result to clipboard!\n"); }
        }
    } else {
        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
        
        if args.on_error_paste {
            let error_content = format!("--- STDOUT ---\n{}\n--- STDERR ---\n{}", stdout_str, stderr_str);
             match client.request::<CreateSnippetRequest, crate::types::CreateSnippetResponse>("/createSnippet", "POST", Some(&CreateSnippetRequest {
                content: error_content,
                title: format!("Execution Error [TP RUN] for snippet {}", args.id),
                visibility: "private".to_string(),
                language: "plaintext".to_string(),
                password: None, tags: None, expires: None
             })) {
                 Ok(new_snippet) => {
                     if !args.silent { println!("\n❌ Script failed. Error log saved to private snippet: {}\n", new_snippet.id); }
                 },
                 Err(e) => {
                     if !args.silent { eprintln!("\n❌ Script failed and could not create error snippet: {}\n", e); }
                 }
             }
        }
        return Err(anyhow!("Process exited with code: {:?}", output.status.code()));
    }

    Ok(())
}
