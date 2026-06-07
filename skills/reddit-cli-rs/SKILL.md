---
name: reddit-cli-rs
description: Use when doing Reddit research from CLI, browsing/searching subreddits, inspecting Reddit posts/users/comments, or sending a single guarded Reddit private message with the local Rust reddit-cli-rs tool.
---

# reddit-cli-rs

Use the local repo at `/Users/aryankumar/reddit-cli-rs`.

## Commands

```bash
cd /Users/aryankumar/reddit-cli-rs
cargo run -- --help
cargo run -- config init
cargo run -- auth check
```

Research:

```bash
cargo run -- browse rust --sort top --time week --limit 10
cargo run -- search "youtube uploader" --subreddit rust --limit 10
cargo run -- post https://redd.it/POST_ID --depth 3 --limit 50
cargo run -- user USERNAME --posts --comments --limit 5
```

Guarded message workflow:

```bash
cargo run -- message send --to USERNAME --subject "Subject" --body "Message"
cargo run -- message send --to USERNAME --subject "Subject" --body "Message" --yes
```

Without `--yes`, message sending is a local dry-run and should not require credentials. Do not build or run bulk unsolicited DM automation.

## Config

Credentials live in `~/.config/reddit-cli-rs/config.toml` or env vars:

- `REDDIT_CLIENT_ID`
- `REDDIT_CLIENT_SECRET`
- `REDDIT_USERNAME`
- `REDDIT_PASSWORD`
- `REDDIT_USER_AGENT`
- `REDDIT_SCOPE`

Never print secrets or commit config files.
