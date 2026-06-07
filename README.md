# reddit-cli-rs

Clean Rust Reddit CLI for subreddit context, Reddit research, candidate extraction, reviewed message drafts, and guarded private messages.

This is built for agentic workflows that need Reddit context without opening the browser every time. It is not a bulk-DM bot: message sends are dry-run by default, draft batches require `approved=true`, and real sends are capped and delayed.

## TL;DR

```bash
cargo install --path .
reddit-cli-rs config init
$EDITOR ~/.config/reddit-cli-rs/config.toml
reddit-cli-rs auth check

SUBREDDIT="community_name"
QUERY="topic keywords"
MATCH="important keyword"

reddit-cli-rs subreddit context "$SUBREDDIT" --top-limit 10 --recent-limit 10
reddit-cli-rs search "$QUERY" --subreddit "$SUBREDDIT" --limit 20 --json > posts.json
reddit-cli-rs candidates search "$QUERY" --subreddit "$SUBREDDIT" --with-comments --match "$MATCH" --json > candidates.json
reddit-cli-rs drafts from-candidates --input candidates.json --subject "Quick question" --template-file message.md --output drafts.json
reddit-cli-rs message send-drafts --input drafts.json --max 5
```

Edit `drafts.json`, set `approved: true` only for messages you reviewed, then run with `--yes`.

## What It Does

| Workflow | Command |
| --- | --- |
| Check auth/account | `auth check` |
| Inspect subreddit rules and approval context | `subreddit context`, `subreddit rules`, `subreddit requirements` |
| Browse/search posts | `browse`, `search` |
| Read one post with comments | `post` |
| Inspect a user before contacting | `user` |
| Extract candidate users from posts/comments | `candidates post`, `candidates search` |
| Render personalized draft messages | `drafts from-candidates` |
| Send one reviewed DM or approved draft batch | `message send`, `message send-drafts` |

## Setup

Create a Reddit script app at <https://www.reddit.com/prefs/apps>. Use the app's client id and secret with your Reddit username/password.

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
scope = "identity read submit privatemessages"
```

Environment variables with the same names in uppercase override the config file:

```bash
REDDIT_CLIENT_ID=...
REDDIT_CLIENT_SECRET=...
REDDIT_USERNAME=...
REDDIT_PASSWORD=...
REDDIT_USER_AGENT="macos:reddit-cli-rs:0.1.0 (by /u/your_username)"
REDDIT_SCOPE="identity read submit privatemessages"
```

## Subreddit Context

Before posting or messaging around a community, collect context:

```bash
reddit-cli-rs subreddit about SUBREDDIT
reddit-cli-rs subreddit rules SUBREDDIT
reddit-cli-rs subreddit requirements SUBREDDIT
reddit-cli-rs subreddit context SUBREDDIT --recent-limit 10 --top-limit 10 --time month
```

`subreddit context` combines:

- `/r/{subreddit}/about`
- `/r/{subreddit}/about/rules`
- `/api/v1/{subreddit}/post_requirements`
- recent posts
- top posts for the selected time window

This makes moderator rules, post constraints, and recent community tone visible before an agent writes anything.

## Research And Candidates

Search posts:

```bash
reddit-cli-rs search "topic keywords" --subreddit SUBREDDIT --sort relevance --limit 20
```

Read comments on a post:

```bash
reddit-cli-rs post https://redd.it/POST_ID --depth 4 --limit 100
```

Extract candidates from one post:

```bash
reddit-cli-rs candidates post https://redd.it/POST_ID \
  --match keyword \
  --match phrase \
  --min-score 1 \
  --json > candidates.json
```

Extract candidates from search results and comments:

```bash
reddit-cli-rs candidates search "topic keywords" \
  --subreddit SUBREDDIT \
  --with-comments \
  --comments-per-post 50 \
  --match keyword \
  --exclude your_username \
  --json > candidates.json
```

Candidates are deduped by username, skip `[deleted]` and `AutoModerator`, keep the strongest source, and include the source URL plus matched text for review.

## Drafts And Messaging

Create a template:

```markdown
Hey u/{username},

Saw your {source_kind} in r/{subreddit} and thought this might be relevant.

Source I found: {source_url}

Short personal message here.
```

Render drafts:

```bash
reddit-cli-rs drafts from-candidates \
  --input candidates.json \
  --subject "Quick question" \
  --template-file message.md \
  --output drafts.json
```

Preview sends:

```bash
reddit-cli-rs message send-drafts --input drafts.json --max 5 --delay-seconds 60
```

Send only reviewed drafts:

```bash
reddit-cli-rs message send-drafts \
  --input drafts.json \
  --max 5 \
  --delay-seconds 60 \
  --log sent-log.json \
  --yes
```

Guardrails:

- `message send` is dry-run unless `--yes` is passed.
- `message send-drafts` only sends drafts with `approved: true`.
- `--max` is capped at 25.
- `--delay-seconds` must be at least 30 for real batch sends.
- The CLI does not generate or approve messages on its own.

## Command Reference

```bash
reddit-cli-rs --help
reddit-cli-rs subreddit --help
reddit-cli-rs candidates --help
reddit-cli-rs drafts --help
reddit-cli-rs message --help
```

Common commands:

```bash
reddit-cli-rs auth check
reddit-cli-rs browse rust --sort top --time week --limit 10
reddit-cli-rs search "youtube uploader" --subreddit rust --limit 10
reddit-cli-rs post https://redd.it/POST_ID --depth 3 --limit 50
reddit-cli-rs user some_user --posts --comments --limit 5
reddit-cli-rs message send --to some_user --subject "Quick note" --body "Hello"
reddit-cli-rs message send --to some_user --subject "Quick note" --body "Hello" --yes
```

## API Notes

This repo uses direct `reqwest` + `serde` bindings against Reddit's OAuth API instead of a Rust wrapper. There is no official Rust Reddit SDK, and direct typed calls keep the surface small and auditable.

Docs used:

- Reddit live API docs: <https://www.reddit.com/dev/api/>
- Reddit Data API wiki: <https://support.reddithelp.com/hc/en-us/articles/16160319875092-Reddit-Data-API-Wiki>

Important Reddit requirements from those docs:

- Use OAuth with a registered app.
- Use a unique descriptive `User-Agent`.
- Monitor `X-Ratelimit-Used`, `X-Ratelimit-Remaining`, and `X-Ratelimit-Reset`.
- Remove stored Reddit user/content data that has been deleted from Reddit.

## Troubleshooting

### `reddit authentication failed`

Check client id, client secret, username, password, and app type. This CLI expects a script app/password grant flow.

### `missing REDDIT_CLIENT_ID or config client_id`

Run:

```bash
reddit-cli-rs config init
$EDITOR ~/.config/reddit-cli-rs/config.toml
```

### `reddit rate limit hit`

The error prints any rate-limit headers Reddit returned. Wait for the reset window before retrying, and reduce `--limit`, `--with-comments`, or batch size.

### `reddit compose rejected message`

Reddit rejected the DM body, recipient, subject, captcha requirement, account state, or anti-abuse checks. The CLI prints Reddit's returned error tuple where available.

## Limits

- It does not bypass subreddit rules, Reddit anti-abuse systems, captcha, account limits, moderator decisions, or admin restrictions.
- It does not scrape Reddit without OAuth.
- It does not retain deletion state for you. If you store exports, keep them short-lived and delete stale data.
- It intentionally avoids blind high-volume messaging.

## License

MIT
