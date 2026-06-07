---
name: reddit-cli-rs
description: Use when doing Reddit research from CLI, gathering subreddit rules/context, browsing/searching subreddits, inspecting Reddit posts/users/comments, extracting candidate users, generating reviewed drafts, or sending guarded Reddit private messages with the local Rust reddit-cli-rs tool.
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
cargo run -- subreddit context SUBREDDIT --recent-limit 10 --top-limit 10
cargo run -- subreddit rules SUBREDDIT
cargo run -- subreddit requirements SUBREDDIT
cargo run -- browse rust --sort top --time week --limit 10
cargo run -- search "youtube uploader" --subreddit rust --limit 10
cargo run -- post https://redd.it/POST_ID --depth 3 --limit 50
cargo run -- user USERNAME --posts --comments --limit 5
```

Candidate and draft workflow:

```bash
cargo run -- candidates post https://redd.it/POST_ID --match keyword --json > candidates.json
cargo run -- candidates search "keyword" --subreddit SUBREDDIT --with-comments --json > candidates.json
cargo run -- drafts from-candidates --input candidates.json --subject "Subject" --template-file message.md --output drafts.json
```

Guarded message workflow:

```bash
cargo run -- message send --to USERNAME --subject "Subject" --body "Message"
cargo run -- message send --to USERNAME --subject "Subject" --body "Message" --yes
cargo run -- message send-drafts --input drafts.json --max 5 --delay-seconds 60
cargo run -- message send-drafts --input drafts.json --max 5 --delay-seconds 60 --yes
```

Without `--yes`, message sending is a local dry-run and should not require credentials. Do not build or run bulk unsolicited DM automation.

Batch draft sending only sends entries with `approved: true`, caps `--max` at 25, and requires at least 30 seconds between real sends.

## Config

Credentials live in `~/.config/reddit-cli-rs/config.toml` or env vars:

- `REDDIT_CLIENT_ID`
- `REDDIT_CLIENT_SECRET`
- `REDDIT_USERNAME`
- `REDDIT_PASSWORD`
- `REDDIT_USER_AGENT`
- `REDDIT_SCOPE`

Never print secrets or commit config files.
