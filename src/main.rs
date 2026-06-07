mod config;
mod models;
mod output;
mod reddit;
mod validation;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::Config;
use crate::output::{print_comments, print_posts, print_user, write_json};
use crate::reddit::RedditClient;

#[derive(Parser)]
#[command(
    name = "reddit-cli-rs",
    version,
    about = "Clean Rust Reddit CLI for subreddit research and guarded account actions"
)]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print config paths or create a config template.
    Config(ConfigCommand),
    /// Check authenticated Reddit API access.
    Auth(AuthCommand),
    /// Browse subreddit listings.
    Browse(BrowseCommand),
    /// Search Reddit or a subreddit.
    Search(SearchCommand),
    /// Show one post with comments.
    Post(PostCommand),
    /// Show user profile and optional activity.
    User(UserCommand),
    /// Guarded private-message workflows.
    Message(MessageCommand),
}

#[derive(Args)]
struct ConfigCommand {
    #[command(subcommand)]
    command: ConfigSubcommand,
}

#[derive(Subcommand)]
enum ConfigSubcommand {
    /// Show the default config path.
    Path,
    /// Write a config template.
    Init {
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
struct AuthCommand {
    #[command(subcommand)]
    command: AuthSubcommand,
}

#[derive(Subcommand)]
enum AuthSubcommand {
    /// Call /api/v1/me with the configured account.
    Check,
}

#[derive(Args)]
struct BrowseCommand {
    /// Subreddit name, with or without r/ prefix.
    subreddit: String,

    #[arg(short, long, value_enum, default_value_t = ListingSort::Hot)]
    sort: ListingSort,

    #[arg(short, long, default_value_t = 25)]
    limit: u8,

    #[arg(short, long, value_enum, default_value_t = TimeFilter::Day)]
    time: TimeFilter,
}

#[derive(Args)]
struct SearchCommand {
    query: String,

    #[arg(short = 'r', long)]
    subreddit: Option<String>,

    #[arg(short, long, value_enum, default_value_t = SearchSort::Relevance)]
    sort: SearchSort,

    #[arg(short, long, default_value_t = 25)]
    limit: u8,

    #[arg(short, long, value_enum, default_value_t = TimeFilter::All)]
    time: TimeFilter,
}

#[derive(Args)]
struct PostCommand {
    /// Reddit post id, redd.it URL, or reddit.com comments URL.
    post: String,

    #[arg(short, long, default_value_t = 50)]
    limit: u8,

    #[arg(short, long, default_value_t = 3)]
    depth: u8,
}

#[derive(Args)]
struct UserCommand {
    username: String,

    #[arg(long)]
    posts: bool,

    #[arg(long)]
    comments: bool,

    #[arg(short, long, default_value_t = 10)]
    limit: u8,
}

#[derive(Args)]
struct MessageCommand {
    #[command(subcommand)]
    command: MessageSubcommand,
}

#[derive(Subcommand)]
enum MessageSubcommand {
    /// Send one private message. Defaults to dry-run unless --yes is passed.
    Send {
        #[arg(long)]
        to: String,

        #[arg(long)]
        subject: String,

        #[arg(long, conflicts_with = "body_file")]
        body: Option<String>,

        #[arg(long)]
        body_file: Option<PathBuf>,

        #[arg(long)]
        yes: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum ListingSort {
    Hot,
    New,
    Top,
    Rising,
    Controversial,
}

impl ListingSort {
    fn as_path(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::New => "new",
            Self::Top => "top",
            Self::Rising => "rising",
            Self::Controversial => "controversial",
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum SearchSort {
    Relevance,
    Hot,
    Top,
    New,
    Comments,
}

impl SearchSort {
    fn as_param(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Hot => "hot",
            Self::Top => "top",
            Self::New => "new",
            Self::Comments => "comments",
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum TimeFilter {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}

impl TimeFilter {
    fn as_param(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
            Self::All => "all",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config(command) => handle_config(command).await,
        Commands::Message(MessageCommand {
            command:
                MessageSubcommand::Send {
                    to,
                    subject,
                    body,
                    body_file,
                    yes: false,
                },
        }) => {
            let body = read_message_body(body, body_file)?;
            print_message_dry_run(&to, &subject, &body);
            Ok(())
        }
        command => {
            let config = Config::load(cli.config.as_deref())?;
            let client = RedditClient::new(config)?;
            handle_api_command(client, command, cli.json).await
        }
    }
}

async fn handle_config(command: ConfigCommand) -> Result<()> {
    match command.command {
        ConfigSubcommand::Path => {
            println!("{}", Config::default_path()?.display());
        }
        ConfigSubcommand::Init { force } => {
            let path = Config::write_template(force)?;
            println!("wrote {}", path.display());
        }
    }
    Ok(())
}

async fn handle_api_command(client: RedditClient, command: Commands, json: bool) -> Result<()> {
    match command {
        Commands::Auth(command) => match command.command {
            AuthSubcommand::Check => {
                let me = client.me().await?;
                if json {
                    write_json(&me)?;
                } else {
                    println!(
                        "u/{} | {} link karma | {} comment karma",
                        me.name, me.link_karma, me.comment_karma
                    );
                }
            }
        },
        Commands::Browse(command) => {
            let posts = client
                .browse(
                    &command.subreddit,
                    command.sort.as_path(),
                    command.limit,
                    command.time.as_param(),
                )
                .await?;
            if json {
                write_json(&posts)?;
            } else {
                print_posts(&posts);
            }
        }
        Commands::Search(command) => {
            let posts = client
                .search(
                    &command.query,
                    command.subreddit.as_deref(),
                    command.sort.as_param(),
                    command.limit,
                    command.time.as_param(),
                )
                .await?;
            if json {
                write_json(&posts)?;
            } else {
                print_posts(&posts);
            }
        }
        Commands::Post(command) => {
            let post = client
                .post(&command.post, command.limit, command.depth)
                .await?;
            if json {
                write_json(&post)?;
            } else {
                println!("# {}\n", post.post.title);
                print_posts(std::slice::from_ref(&post.post));
                if !post.comments.is_empty() {
                    println!("\nComments:");
                    print_comments(&post.comments);
                }
            }
        }
        Commands::User(command) => {
            let user = client
                .user(
                    &command.username,
                    command.posts,
                    command.comments,
                    command.limit,
                )
                .await?;
            if json {
                write_json(&user)?;
            } else {
                print_user(&user);
            }
        }
        Commands::Message(command) => match command.command {
            MessageSubcommand::Send {
                to,
                subject,
                body,
                body_file,
                yes,
            } => {
                let body = read_message_body(body, body_file)?;
                if !yes {
                    print_message_dry_run(&to, &subject, &body);
                    return Ok(());
                }
                client.send_message(&to, &subject, &body).await?;
                println!("sent message to u/{}", validation::validate_username(&to)?);
            }
        },
        Commands::Config(_) => unreachable!("config handled before API client creation"),
    }
    Ok(())
}

fn read_message_body(body: Option<String>, body_file: Option<PathBuf>) -> Result<String> {
    match (body, body_file) {
        (Some(value), None) => Ok(value),
        (None, Some(path)) => Ok(std::fs::read_to_string(path)?),
        _ => anyhow::bail!("provide --body or --body-file"),
    }
}

fn print_message_dry_run(to: &str, subject: &str, body: &str) {
    println!("dry-run: would send one private message");
    println!("to: {}", to);
    println!("subject: {}", subject);
    println!("body:\n{}", body);
    println!(
        "\nPass --yes to send. Bulk or unsolicited DM automation is intentionally not supported."
    );
}
