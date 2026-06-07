mod config;
mod models;
mod output;
mod pipeline;
mod reddit;
mod validation;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use tokio::time::sleep;

use crate::config::Config;
use crate::models::{Candidate, DraftMessage};
use crate::output::{
    print_candidates, print_comments, print_drafts, print_posts, print_requirements, print_rules,
    print_subreddit, print_subreddit_context, print_user, write_json,
};
use crate::pipeline::{
    CandidateOptions, extract_candidates_from_post, extract_candidates_from_posts, render_drafts,
};
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
    /// Inspect subreddit metadata, rules, and posting requirements.
    Subreddit(SubredditCommand),
    /// Browse subreddit listings.
    Browse(BrowseCommand),
    /// Search Reddit or a subreddit.
    Search(SearchCommand),
    /// Show one post with comments.
    Post(PostCommand),
    /// Show user profile and optional activity.
    User(UserCommand),
    /// Extract outreach/research candidates from posts and comments.
    Candidates(CandidatesCommand),
    /// Generate reviewed message drafts from candidates.
    Drafts(DraftsCommand),
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
struct SubredditCommand {
    #[command(subcommand)]
    command: SubredditSubcommand,
}

#[derive(Subcommand)]
enum SubredditSubcommand {
    /// Show subreddit metadata.
    About {
        /// Subreddit name, with or without r/ prefix.
        subreddit: String,
    },
    /// Show subreddit rules.
    Rules {
        /// Subreddit name, with or without r/ prefix.
        subreddit: String,
    },
    /// Show moderator-designated post requirements.
    Requirements {
        /// Subreddit name, with or without r/ prefix.
        subreddit: String,
    },
    /// Show about, rules, requirements, and recent/top posts together.
    Context {
        /// Subreddit name, with or without r/ prefix.
        subreddit: String,

        #[arg(long, default_value_t = 10)]
        recent_limit: u8,

        #[arg(long, default_value_t = 10)]
        top_limit: u8,

        #[arg(short, long, value_enum, default_value_t = TimeFilter::Month)]
        time: TimeFilter,
    },
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
struct CandidatesCommand {
    #[command(subcommand)]
    command: CandidatesSubcommand,
}

#[derive(Subcommand)]
enum CandidatesSubcommand {
    /// Extract users from one post's author/comments.
    Post {
        post: String,

        #[arg(short, long, default_value_t = 100)]
        limit: u8,

        #[arg(short, long, default_value_t = 5)]
        depth: u8,

        #[arg(long, default_value_t = 1)]
        min_score: i64,

        #[arg(long = "match")]
        matches: Vec<String>,

        #[arg(long)]
        include_post_author: bool,

        #[arg(long)]
        exclude: Vec<String>,
    },
    /// Search posts, then extract post authors and optionally commenters.
    Search {
        query: String,

        #[arg(short = 'r', long)]
        subreddit: Option<String>,

        #[arg(short, long, value_enum, default_value_t = SearchSort::Relevance)]
        sort: SearchSort,

        #[arg(short, long, default_value_t = 25)]
        limit: u8,

        #[arg(short, long, value_enum, default_value_t = TimeFilter::All)]
        time: TimeFilter,

        #[arg(long)]
        with_comments: bool,

        #[arg(long, default_value_t = 30)]
        comments_per_post: u8,

        #[arg(long, default_value_t = 2)]
        depth: u8,

        #[arg(long, default_value_t = 1)]
        min_score: i64,

        #[arg(long = "match")]
        matches: Vec<String>,

        #[arg(long)]
        exclude: Vec<String>,
    },
}

#[derive(Args)]
struct DraftsCommand {
    #[command(subcommand)]
    command: DraftsSubcommand,
}

#[derive(Subcommand)]
enum DraftsSubcommand {
    /// Render message drafts from a candidates JSON file.
    FromCandidates {
        #[arg(long)]
        input: PathBuf,

        #[arg(long)]
        subject: String,

        #[arg(long, conflicts_with = "template_file")]
        template: Option<String>,

        #[arg(long)]
        template_file: Option<PathBuf>,

        #[arg(long)]
        output: Option<PathBuf>,
    },
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
    /// Send approved drafts from a JSON file. Dry-run unless --yes is passed.
    SendDrafts {
        #[arg(long)]
        input: PathBuf,

        #[arg(long, default_value_t = 10)]
        max: usize,

        #[arg(long, default_value_t = 60)]
        delay_seconds: u64,

        #[arg(long)]
        log: Option<PathBuf>,

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
        Commands::Drafts(command) => handle_drafts(command, cli.json).await,
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
        Commands::Message(MessageCommand {
            command:
                MessageSubcommand::SendDrafts {
                    input,
                    max,
                    delay_seconds,
                    log,
                    yes: false,
                },
        }) => {
            let drafts: Vec<DraftMessage> = read_json_file(&input)?;
            print_draft_send_preview(&drafts, max, delay_seconds, log.as_deref());
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

async fn handle_drafts(command: DraftsCommand, json: bool) -> Result<()> {
    match command.command {
        DraftsSubcommand::FromCandidates {
            input,
            subject,
            template,
            template_file,
            output,
        } => {
            let candidates: Vec<Candidate> = read_json_file(&input)?;
            let template = read_template(template, template_file)?;
            let drafts = render_drafts(&candidates, &subject, &template);
            if let Some(path) = output {
                write_json_file(&path, &drafts)?;
                println!("wrote {}", path.display());
            } else if json {
                write_json(&drafts)?;
            } else {
                print_drafts(&drafts);
            }
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
        Commands::Subreddit(command) => match command.command {
            SubredditSubcommand::About { subreddit } => {
                let about = client.subreddit_about(&subreddit).await?;
                if json {
                    write_json(&about)?;
                } else {
                    print_subreddit(&about);
                }
            }
            SubredditSubcommand::Rules { subreddit } => {
                let rules = client.subreddit_rules(&subreddit).await?;
                if json {
                    write_json(&rules)?;
                } else {
                    print_rules(&rules);
                }
            }
            SubredditSubcommand::Requirements { subreddit } => {
                let requirements = client.post_requirements(&subreddit).await?;
                if json {
                    write_json(&requirements)?;
                } else {
                    print_requirements(requirements.as_ref());
                }
            }
            SubredditSubcommand::Context {
                subreddit,
                recent_limit,
                top_limit,
                time,
            } => {
                let context = client
                    .subreddit_context(&subreddit, recent_limit, top_limit, time.as_param())
                    .await?;
                if json {
                    write_json(&context)?;
                } else {
                    print_subreddit_context(&context);
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
        Commands::Candidates(command) => {
            let candidates = match command.command {
                CandidatesSubcommand::Post {
                    post,
                    limit,
                    depth,
                    min_score,
                    matches,
                    include_post_author,
                    exclude,
                } => {
                    let report = client.post(&post, limit, depth).await?;
                    extract_candidates_from_post(
                        &report,
                        CandidateOptions {
                            min_score,
                            matches,
                            exclude,
                            include_post_author,
                        },
                    )
                }
                CandidatesSubcommand::Search {
                    query,
                    subreddit,
                    sort,
                    limit,
                    time,
                    with_comments,
                    comments_per_post,
                    depth,
                    min_score,
                    matches,
                    exclude,
                } => {
                    let posts = client
                        .search(
                            &query,
                            subreddit.as_deref(),
                            sort.as_param(),
                            limit,
                            time.as_param(),
                        )
                        .await?;
                    let options = CandidateOptions {
                        min_score,
                        matches,
                        exclude,
                        include_post_author: true,
                    };
                    if with_comments {
                        let mut all = extract_candidates_from_posts(&posts, options.clone());
                        for post in posts {
                            let report = client.post(&post.id, comments_per_post, depth).await?;
                            all.extend(extract_candidates_from_post(&report, options.clone()));
                        }
                        pipeline::dedupe_candidates(all)
                    } else {
                        extract_candidates_from_posts(&posts, options)
                    }
                }
            };
            if json {
                write_json(&candidates)?;
            } else {
                print_candidates(&candidates);
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
            MessageSubcommand::SendDrafts {
                input,
                max,
                delay_seconds,
                log,
                yes,
            } => {
                let drafts: Vec<DraftMessage> = read_json_file(&input)?;
                if !yes {
                    print_draft_send_preview(&drafts, max, delay_seconds, log.as_deref());
                    return Ok(());
                }
                send_approved_drafts(&client, &drafts, max, delay_seconds, log.as_deref()).await?;
            }
        },
        Commands::Config(_) | Commands::Drafts(_) => {
            unreachable!("handled before API client creation")
        }
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

fn read_template(template: Option<String>, template_file: Option<PathBuf>) -> Result<String> {
    match (template, template_file) {
        (Some(value), None) => Ok(value),
        (None, Some(path)) => Ok(std::fs::read_to_string(path)?),
        _ => anyhow::bail!("provide --template or --template-file"),
    }
}

fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("failed parsing {}", path.display()))
}

fn write_json_file<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(path, format!("{}\n", text))
        .with_context(|| format!("failed writing {}", path.display()))
}

fn print_draft_send_preview(
    drafts: &[DraftMessage],
    max: usize,
    delay_seconds: u64,
    log: Option<&Path>,
) {
    let approved = drafts.iter().filter(|draft| draft.approved).count();
    let will_send = approved.min(max);
    println!("dry-run: would send {} approved draft(s)", will_send);
    println!("approved in file: {}", approved);
    println!("max this run: {}", max);
    println!("delay between sends: {}s", delay_seconds);
    if let Some(log) = log {
        println!("log: {}", log.display());
    }
    println!("\nOnly drafts with approved=true are eligible. Pass --yes to send.");
}

async fn send_approved_drafts(
    client: &RedditClient,
    drafts: &[DraftMessage],
    max: usize,
    delay_seconds: u64,
    log: Option<&Path>,
) -> Result<()> {
    if max == 0 || max > 25 {
        anyhow::bail!("--max must be between 1 and 25");
    }
    if delay_seconds < 30 {
        anyhow::bail!("--delay-seconds must be at least 30 when sending");
    }

    let selected = drafts
        .iter()
        .filter(|draft| draft.approved)
        .take(max)
        .collect::<Vec<_>>();

    if selected.is_empty() {
        anyhow::bail!("no approved drafts found; set approved=true after manual review");
    }

    let mut log_rows = Vec::new();
    for (index, draft) in selected.iter().enumerate() {
        client
            .send_message(&draft.to, &draft.subject, &draft.body)
            .await
            .with_context(|| format!("failed sending to u/{}", draft.to))?;
        println!("sent {}/{} to u/{}", index + 1, selected.len(), draft.to);
        log_rows.push(serde_json::json!({
            "to": draft.to,
            "subject": draft.subject,
            "source_url": draft.source_url,
            "sent": true,
        }));
        if index + 1 < selected.len() {
            sleep(Duration::from_secs(delay_seconds)).await;
        }
    }

    if let Some(path) = log {
        write_json_file(path, &log_rows)?;
    }
    Ok(())
}
