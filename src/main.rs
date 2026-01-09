mod api;
mod cli_args;
mod commands;
mod config;
mod types;
mod utils;

use crate::cli_args::{Cli, Commands};
use crate::commands::*;
use clap::Parser;
use log::LevelFilter;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // In JS, unhandled errors were caught and printed nicely.
    // In Rust, we return Result from main, which prints Debug of error.
    // We can use a wrapper to print nicely.

    if let Err(e) = run(cli) {
        eprintln!("\n❌ Error: {}\n", e);
        std::process::exit(1);
    }
    Ok(())
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let log_level = match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    let token = resolve_token(cli.token)?;

    match cli.command {
        Commands::Edit(args) => handle_edit(args, token),
        Commands::View(args) => handle_view(args, token, cli.json),
        Commands::Clone(args) => handle_clone(args, token),
        Commands::Copy(args) => handle_copy(args, token),
        Commands::Star(args) => handle_star(args, token),
        Commands::Restore(args) => handle_restore(args, token),
        Commands::Run(args) => handle_run(args, token),
        Commands::Stats => handle_stats(token),
        Commands::List(args) => handle_list(args, token, cli.json),
        Commands::Create(args) => handle_create(args, token),
        Commands::Update(args) => handle_update(args, token),
        Commands::Delete(args) => handle_delete(args, token),
        Commands::Search(args) => handle_search(args, token, cli.json),
        Commands::User(args) => handle_user(args, token, cli.json),
        Commands::Config(args) => handle_config(args),
        Commands::Upgrade => handle_upgrade(),
    }
}
