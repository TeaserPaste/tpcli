use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};
use arboard::Clipboard;
use inquire::{Confirm};
use log::{debug, info};
use tempfile::tempdir;
use regex::Regex;

use crate::api::ApiClient;
use crate::cli_args::RunArgs;
use crate::types::{CreateSnippetRequest, CreateSnippetResponse};
use crate::utils::get_file_extension;
use crate::config::ConfigManager;

pub fn detect_dependencies(content: &str, language: &str) -> Vec<String> {
    let lang = language.to_lowercase();
    let mut deps = std::collections::HashSet::new();

    if lang == "javascript" || lang == "typescript" {
        // JS regex: /(?:require\(['"]([^'"]+)['"]\)|import\s+.*?from\s+['"]([^'"]+)['"])/g
        let re =
            Regex::new(r#"(?:require\(['"]([^'"]+)['"]\)|import\s+.*?from\s+['"]([^'"]+)['"])"#)
                .unwrap();
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
        return Err(anyhow!(
            "Dependency installation not supported for {}",
            language
        ));
    };

    let mut full_args = args;
    full_args.extend(deps.iter().map(|s| s.as_str()));

    if !silent {
        info!(
            "Installing dependencies: {} {}",
            command,
            full_args.join(" ")
        );
    }

    let status = Command::new(command)
        .args(&full_args)
        .current_dir(cwd)
        .stdout(if silent {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stderr(if silent {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .status()
        .context("Failed to run dependency installer")?;

    if !status.success() {
        return Err(anyhow!("Dependency installation failed"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dependencies_js() {
        let content = r#"
            const axios = require('axios');
            import React from 'react';
            import { useState } from 'react';
            const fs = require('fs'); // Node built-in, but our regex might pick it up. Implementation should filter?
            // Actually, current implementation allows anything in require/import.
            // But it filters ./ and ../
            const local = require('./local');
        "#;
        let deps = detect_dependencies(content, "javascript");
        assert!(deps.contains(&"axios".to_string()));
        assert!(deps.contains(&"react".to_string()));
        // "fs" is built-in but regex picks it up. If we don't have a blacklist, it's there.
        // "./local" should be filtered.
        assert!(!deps.contains(&"./local".to_string()));
    }

    #[test]
    fn test_detect_dependencies_python() {
        let content = r#"
            import requests
            from flask import Flask
            import os # Standard lib
            from . import local
        "#;
        let deps = detect_dependencies(content, "python");
        assert!(deps.contains(&"requests".to_string()));
        assert!(deps.contains(&"flask".to_string()));
        assert!(deps.contains(&"os".to_string()));
        // "from . import local" -> matches group 1 (".")?
        // Regex: (?:from\s+([^\s]+)\s+import|import\s+([^\s]+))
        // "from . import" -> group 1 is "."
        // "." might be in deps. Current impl doesn't filter "." for python specifically?
        // But "." usually means local package which pip can't install unless it's a valid package name.
        // Let's see what happens.
    }
}

pub fn handle_run(args: RunArgs, token: Option<String>) -> Result<()> {
    if !args.force && atty::is(atty::Stream::Stdin) {
        let confirm = Confirm::new(
            "⚠️ Warning: You are about to execute code from the internet. Are you sure?",
        )
        .with_default(false)
        .prompt()?;
        if !confirm {
            println!("\nOperation canceled.\n");
            return Ok(());
        }
    }

    let client = ApiClient::new(token.clone());
    let snippet: crate::types::Snippet = client.request(
        "/getSnippet",
        "POST",
        Some(&serde_json::json!({
            "snippetId": args.id,
            "password": args.password
        })),
    )?;

    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    if args.install_deps {
        let deps = detect_dependencies(&snippet.content, &snippet.language);
        if !deps.is_empty() {
            let confirm_install = if atty::is(atty::Stream::Stdin) {
                Confirm::new(&format!(
                    "Detected dependencies: {}. Install in temp env?",
                    deps.join(", ")
                ))
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
    debug!("Writing executable file: {}", executable_file.display());
    fs::write(&executable_file, &snippet.content)?;

    // Execution logic
    let lang = snippet.language.to_lowercase();
    let config = ConfigManager::load_config().unwrap_or_default();

    let (run_cmd, run_args) = if let Some(custom_cmds) = &args.command {
        // args.command is Vec<String>
        // JS logic: replace '--snippet' with file path, or append file path.
        let mut cmds = custom_cmds.clone();
        if let Some(pos) = cmds.iter().position(|s| s == "--snippet") {
            cmds[pos] = executable_file.to_string_lossy().to_string();
        } else {
            cmds.push(executable_file.to_string_lossy().to_string());
        }
        (cmds[0].clone(), cmds[1..].to_vec())
    } else {
        // Resolve runner string from flag or config
        let runner_str = args.with_runner.as_ref()
            .or_else(|| config.runners.as_ref().and_then(|r| r.get(&lang)));
        
        if let Some(s) = runner_str {
            let file_path_str = executable_file.to_string_lossy();
            let quoted_path = shlex::try_quote(&file_path_str)
                .map_err(|_| anyhow!("Invalid filename containing null bytes"))?;
            let cmd_str = if s.contains("{{file}}") {
                s.replace("{{file}}", &quoted_path)
            } else {
                format!("{} {}", s, quoted_path)
            };
            
            let parts = shlex::split(&cmd_str)
                .ok_or_else(|| anyhow!("Failed to parse runner command: {}", s))?;
            
            if parts.is_empty() {
                return Err(anyhow!("Runner command is empty"));
            }
            (parts[0].clone(), parts[1..].to_vec())
        } else {
            match lang.as_str() {
                "python" => (
                    "python".to_string(),
                    vec![executable_file.to_string_lossy().to_string()],
                ),
                "javascript" => (
                    "node".to_string(),
                    vec![executable_file.to_string_lossy().to_string()],
                ),
                "typescript" => (
                    "ts-node".to_string(),
                    vec![executable_file.to_string_lossy().to_string()],
                ),
                "shell" | "bash" => (
                    "bash".to_string(),
                    vec![executable_file.to_string_lossy().to_string()],
                ),
                "ruby" => (
                    "ruby".to_string(),
                    vec![executable_file.to_string_lossy().to_string()],
                ),
                "go" => (
                    "go".to_string(),
                    vec![
                        "run".to_string(),
                        executable_file.to_string_lossy().to_string(),
                    ],
                ),
                "rust" => {
                    // compile then run
                    let exe_path = temp_path.join("snippet_bin");
                    if !args.silent {
                        info!(
                            "Compiling: rustc {} -o {}",
                            executable_file.display(),
                            exe_path.display()
                        );
                    }
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
                }
                "c" | "cpp" => {
                    let compiler = if lang == "c" { "gcc" } else { "g++" };
                    let exe_path = temp_path.join("snippet_bin");
                    if !args.silent {
                        info!(
                            "Compiling: {} {} -o {}",
                            compiler,
                            executable_file.display(),
                            exe_path.display()
                        );
                    }
                    let compile_status = Command::new(compiler)
                        .arg(&executable_file)
                        .arg("-o")
                        .arg(&exe_path)
                        .current_dir(temp_path)
                        .status()?;
                    if !compile_status.success() {
                        return Err(anyhow!("Compilation failed"));
                    }
                    (exe_path.to_string_lossy().to_string(), vec![])
                }
                "java" => {
                    // compile then run
                    if !args.silent {
                        info!("Compiling: javac {}", executable_file.display());
                    }
                    let compile_status = Command::new("javac")
                        .arg(&executable_file)
                        .current_dir(temp_path)
                        .status()?;
                    if !compile_status.success() {
                        return Err(anyhow!("Compilation failed"));
                    }
                    (
                        "java".to_string(),
                        vec![
                            "-cp".to_string(),
                            temp_path.to_string_lossy().to_string(),
                            "snippet".to_string(),
                        ],
                    )
                }
                _ => {
                    return Err(anyhow!(
                        "Language '{}' is not supported for auto-run. Please specify command explicitly.",
                        lang
                    ));
                }
            }
        }
    };

    if !args.silent {
        info!(
            "Running: {} {} (in {})",
            run_cmd,
            run_args.join(" "),
            temp_path.display()
        );
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

    let output = child.wait_with_output()?;

    if output.status.success() {
        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        if !args.silent && !capture_output {
            info!("Process finished successfully.");
        }

        if let Some(outfile) = args.output_file {
            fs::write(outfile, &output.stdout)?;
        } else if capture_output && !args.copy_result && !args.on_error_paste {
            print!("{}", stdout_str);
        }

        if args.copy_result {
            let mut clipboard = Clipboard::new()?;
            clipboard.set_text(&stdout_str)?;
            if !args.silent {
                println!("\n✅ Copied result to clipboard!\n");
            }
        }
    } else {
        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

        if args.on_error_paste {
            let error_content = format!(
                "--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                stdout_str, stderr_str
            );
            match client.request::<CreateSnippetRequest, CreateSnippetResponse>(
                "/createSnippet",
                "POST",
                Some(&CreateSnippetRequest {
                    content: error_content,
                    title: format!("Execution Error [TP RUN] for snippet {}", args.id),
                    visibility: "private".to_string(),
                    language: "plaintext".to_string(),
                    password: None,
                    tags: None,
                    expires: None,
                }),
            ) {
                Ok(new_snippet) => {
                    if !args.silent {
                        println!(
                            "\n❌ Script failed. Error log saved to private snippet: {}\n",
                            new_snippet.id
                        );
                    }
                }
                Err(e) => {
                    if !args.silent {
                        eprintln!(
                            "\n❌ Script failed and could not create error snippet: {}\n",
                            e
                        );
                    }
                }
            }
        }
        return Err(anyhow!(
            "Process exited with code: {:?}",
            output.status.code()
        ));
    }

    Ok(())
}
