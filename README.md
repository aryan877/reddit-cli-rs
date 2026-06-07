# reddit-cli-rs

A cleaner Rust Reddit CLI for subreddit research and guarded account actions.

This is not a bulk-DM bot. Message sending is one-recipient-at-a-time and dry-run by default.

## Setup

Create a Reddit script app at https://www.reddit.com/prefs/apps and save credentials:

```bash
cargo run -- config init
$EDITOR ~/.config/reddit-cli-rs/config.toml
```

Required config:

```toml
client_id = "..."
client_secret = "..."
username = "..."
password = "..."
user_agent = "macos:reddit-cli-rs:0.1.0 (by /u/your_username)"
scope = "identity read privatemessages"
```

Environment variables with the same names in uppercase override the config file.

## Commands

```bash
cargo run -- auth check
cargo run -- browse rust --sort top --time week --limit 10
cargo run -- search "youtube uploader" --subreddit rust --limit 10
cargo run -- post https://redd.it/POST_ID --depth 3 --limit 50
cargo run -- user spez --posts --comments --limit 5
```

Guarded message workflow:

```bash
cargo run -- message send --to username --subject "Quick note" --body "Hello"
cargo run -- message send --to username --subject "Quick note" --body "Hello" --yes
```

The first command is dry-run. `--yes` is required to actually send.

## Notes From alceal/reddit-cli

The reference project is already a Rust CLI with browse/search/post/user/comments commands and good validation. This version keeps the useful shape but makes the account-action boundary explicit, uses a config template command, and prevents accidental DM sends.
