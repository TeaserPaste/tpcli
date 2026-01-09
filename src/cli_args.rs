use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tp")]
#[command(
    about = "CLI Client for TeaserPaste - View, create, and manage snippets.",
    version = "0.1.0"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose mode. Increase for more detail (e.g., -v, -vv, -vvv)
    #[arg(long = "verbose", short = 'v', action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Helper to set token globally
    #[arg(long, global = true)]
    pub token: Option<String>,

    /// Output results as JSON
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Edit a snippet
    Edit(EditArgs),
    /// View a snippet
    View(ViewArgs),
    /// Download snippet content to a file
    Clone(CloneArgs),
    /// Copy (fork) a snippet to your account
    Copy(CopyArgs),
    /// Star a snippet
    Star(StarArgs),
    /// Restore a deleted snippet (from trash)
    Restore(RestoreArgs),
    /// Execute a snippet
    Run(RunArgs),
    /// View statistics about your snippets
    Stats,
    /// List your snippets
    List(ListArgs),
    /// Create a new snippet
    Create(CreateArgs),
    /// Update an existing snippet
    Update(UpdateArgs),
    /// Delete a snippet
    Delete(DeleteArgs),
    /// Search public snippets
    Search(SearchArgs),
    /// User commands
    User(UserArgs),
    /// Manage CLI configuration
    Config(ConfigArgs),
    /// Upgrade the CLI tool
    Upgrade,
}

#[derive(Args)]
pub struct EditArgs {
    pub id: String,
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct ViewArgs {
    /// Snippet ID
    pub id: String,
    /// Only print the raw content of the snippet
    #[arg(long)]
    pub raw: bool,
    /// Copy snippet content to clipboard
    #[arg(long)]
    pub copy: bool,
    /// Show the URL of the snippet
    #[arg(long)]
    pub url: bool,
    /// Password for the snippet
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct CloneArgs {
    pub id: String,
    pub filename: Option<String>,
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct CopyArgs {
    pub id: String,
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct StarArgs {
    pub id: String,
    /// Unstar instead of starring
    #[arg(long)]
    pub unstar: bool,
}

#[derive(Args)]
pub struct RestoreArgs {
    pub id: String,
}

#[derive(Args)]
pub struct RunArgs {
    pub id: String,
    pub command: Option<Vec<String>>,

    #[arg(long)]
    pub install_deps: bool,
    #[arg(long, value_parser = parse_key_val::<String, String>)]
    pub env: Vec<(String, String)>,
    #[arg(long)]
    pub input_file: Option<String>,
    #[arg(long)]
    pub output_file: Option<String>,
    #[arg(long)]
    pub copy_result: bool,
    #[arg(long)]
    pub on_error_paste: bool,
    #[arg(long)]
    pub timeout: Option<String>,
    #[arg(short, long)]
    pub force: bool,
    #[arg(short, long)]
    pub silent: bool,
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Args)]
pub struct ListArgs {
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub visibility: Option<String>,
    #[arg(short = 'd', long)]
    pub include_deleted: bool,
}

#[derive(Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub content: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub visibility: Option<String>,
    #[arg(long)]
    pub password: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
    #[arg(long)]
    pub expires: Option<String>,
    #[arg(short, long)]
    pub interactive: bool,
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(Args)]
pub struct UpdateArgs {
    pub id: String,

    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub content: Option<String>,
    #[arg(long)]
    pub language: Option<String>,
    #[arg(long)]
    pub visibility: Option<String>,
    #[arg(long)]
    pub password: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
    #[arg(long)]
    pub expires: Option<String>,
}

#[derive(Args)]
pub struct DeleteArgs {
    pub id: String,
}

#[derive(Args)]
pub struct SearchArgs {
    pub term: String,
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub from: u32,
}

#[derive(Subcommand)]
pub enum UserCmd {
    View {
        #[arg(short, long)]
        s: bool,
    },
}

#[derive(Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCmd,
}

#[derive(Subcommand)]
pub enum ConfigCmd {
    Set { key: String, value: String },
    Get { key: String },
    Clear { key: String },
}

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCmd,
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
    U: std::str::FromStr,
    U::Err: std::fmt::Display,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((
        s[..pos].parse().map_err(|e| format!("{}", e))?,
        s[pos + 1..].parse().map_err(|e| format!("{}", e))?,
    ))
}
