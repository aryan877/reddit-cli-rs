use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use crate::config::Config;
use crate::models::{
    Comment, Listing, Post, PostReport, PostRequirements, RulesResponse, Subreddit,
    SubredditContext, Thing, User, UserReport,
};
use crate::validation::{extract_post_id, validate_subreddit, validate_username};

#[derive(Debug, thiserror::Error)]
pub enum RedditError {
    #[error("reddit API returned {status}: {body}{rate_limit}")]
    Api {
        status: StatusCode,
        body: String,
        rate_limit: RateLimitInfo,
    },
    #[error("reddit rate limit hit{rate_limit}")]
    RateLimited { rate_limit: RateLimitInfo },
    #[error("reddit authentication failed ({status}): {body}")]
    Auth { status: StatusCode, body: String },
    #[error("reddit compose rejected message: {message}")]
    Compose { message: String },
}

#[derive(Debug, Clone, Default)]
pub struct RateLimitInfo {
    retry_after: Option<String>,
    used: Option<String>,
    remaining: Option<String>,
    reset: Option<String>,
}

impl RateLimitInfo {
    fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            retry_after: header_value(headers, "retry-after"),
            used: header_value(headers, "x-ratelimit-used"),
            remaining: header_value(headers, "x-ratelimit-remaining"),
            reset: header_value(headers, "x-ratelimit-reset"),
        }
    }

    fn is_empty(&self) -> bool {
        self.retry_after.is_none()
            && self.used.is_none()
            && self.remaining.is_none()
            && self.reset.is_none()
    }
}

impl std::fmt::Display for RateLimitInfo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        let mut parts = Vec::new();
        if let Some(value) = &self.retry_after {
            parts.push(format!("retry-after={}s", value));
        }
        if let Some(value) = &self.used {
            parts.push(format!("used={}", value));
        }
        if let Some(value) = &self.remaining {
            parts.push(format!("remaining={}", value));
        }
        if let Some(value) = &self.reset {
            parts.push(format!("reset={}s", value));
        }
        write!(formatter, " | rate limit: {}", parts.join(", "))
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

struct TokenState {
    token: Option<Zeroizing<String>>,
    expires_at: Instant,
}

impl Default for TokenState {
    fn default() -> Self {
        Self {
            token: None,
            expires_at: Instant::now(),
        }
    }
}

pub struct RedditClient {
    http: reqwest::Client,
    config: Config,
    token: Mutex<TokenState>,
}

impl RedditClient {
    pub fn new(config: Config) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;
        Ok(Self {
            http,
            config,
            token: Mutex::new(TokenState::default()),
        })
    }

    pub async fn me(&self) -> Result<User> {
        self.get("/api/v1/me", &[]).await
    }

    pub async fn subreddit_about(&self, subreddit: &str) -> Result<Subreddit> {
        let subreddit = validate_subreddit(subreddit)?;
        let thing: Thing<Subreddit> = self.get(&format!("/r/{}/about", subreddit), &[]).await?;
        Ok(thing.data)
    }

    pub async fn subreddit_rules(
        &self,
        subreddit: &str,
    ) -> Result<Vec<crate::models::SubredditRule>> {
        let subreddit = validate_subreddit(subreddit)?;
        let response: RulesResponse = self
            .get(&format!("/r/{}/about/rules", subreddit), &[])
            .await?;
        Ok(response.rules)
    }

    pub async fn post_requirements(&self, subreddit: &str) -> Result<Option<PostRequirements>> {
        let subreddit = validate_subreddit(subreddit)?;
        let endpoint = format!("/api/v1/{}/post_requirements", subreddit);
        match self.get::<PostRequirements>(&endpoint, &[]).await {
            Ok(requirements) => Ok(Some(requirements)),
            Err(error) => {
                if let Some(RedditError::Api { status, .. }) = error.downcast_ref::<RedditError>()
                    && (*status == StatusCode::NOT_FOUND || *status == StatusCode::FORBIDDEN)
                {
                    return Ok(None);
                }
                Err(error)
            }
        }
    }

    pub async fn subreddit_context(
        &self,
        subreddit: &str,
        recent_limit: u8,
        top_limit: u8,
        time: &str,
    ) -> Result<SubredditContext> {
        let subreddit = validate_subreddit(subreddit)?;
        let about = self.subreddit_about(&subreddit).await?;
        let rules = self.subreddit_rules(&subreddit).await?;
        let post_requirements = self.post_requirements(&subreddit).await?;
        let recent_posts = self.browse(&subreddit, "new", recent_limit, "day").await?;
        let top_posts = self.browse(&subreddit, "top", top_limit, time).await?;

        Ok(SubredditContext {
            subreddit: about,
            rules,
            post_requirements,
            recent_posts,
            top_posts,
        })
    }

    pub async fn browse(
        &self,
        subreddit: &str,
        sort: &str,
        limit: u8,
        time: &str,
    ) -> Result<Vec<Post>> {
        let subreddit = validate_subreddit(subreddit)?;
        let endpoint = format!("/r/{}/{}", subreddit, sort);
        self.posts_from_listing(
            &endpoint,
            &[("limit", limit.to_string()), ("t", time.to_string())],
        )
        .await
    }

    pub async fn search(
        &self,
        query: &str,
        subreddit: Option<&str>,
        sort: &str,
        limit: u8,
        time: &str,
    ) -> Result<Vec<Post>> {
        let endpoint = match subreddit {
            Some(subreddit) => format!("/r/{}/search", validate_subreddit(subreddit)?),
            None => "/search".to_string(),
        };
        let mut params = vec![
            ("q", query.to_string()),
            ("sort", sort.to_string()),
            ("limit", limit.to_string()),
            ("t", time.to_string()),
        ];
        if subreddit.is_some() {
            params.push(("restrict_sr", "1".to_string()));
        }
        self.posts_from_listing(&endpoint, &params).await
    }

    pub async fn post(&self, input: &str, limit: u8, depth: u8) -> Result<PostReport> {
        let id = extract_post_id(input)?;
        let endpoint = format!("/comments/{}", id);
        let value: Value = self
            .get(
                &endpoint,
                &[("limit", limit.to_string()), ("depth", depth.to_string())],
            )
            .await?;

        let arrays = value.as_array().context("unexpected comments response")?;
        let post_listing: Listing<Post> =
            serde_json::from_value(arrays.first().context("missing post listing")?.clone())?;
        let post = post_listing
            .data
            .children
            .into_iter()
            .next()
            .map(|thing| thing.data)
            .context("post not found")?;

        let mut comments = Vec::new();
        if let Some(comment_listing) = arrays.get(1) {
            collect_comments(comment_listing, 0, depth, &mut comments);
        }

        Ok(PostReport { post, comments })
    }

    pub async fn user(
        &self,
        username: &str,
        include_posts: bool,
        include_comments: bool,
        limit: u8,
    ) -> Result<UserReport> {
        let username = validate_username(username)?;
        let profile: Thing<User> = self.get(&format!("/user/{}/about", username), &[]).await?;

        let posts = if include_posts {
            self.posts_from_listing(
                &format!("/user/{}/submitted", username),
                &[("limit", limit.to_string())],
            )
            .await?
        } else {
            Vec::new()
        };

        let comments = if include_comments {
            let listing: Listing<Comment> = self
                .get(
                    &format!("/user/{}/comments", username),
                    &[("limit", limit.to_string())],
                )
                .await?;
            listing
                .data
                .children
                .into_iter()
                .map(|thing| thing.data)
                .collect()
        } else {
            Vec::new()
        };

        Ok(UserReport {
            profile: profile.data,
            posts,
            comments,
        })
    }

    pub async fn send_message(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        let to = validate_username(to)?;
        if subject.trim().is_empty() {
            anyhow::bail!("subject cannot be empty");
        }
        if body.trim().is_empty() {
            anyhow::bail!("body cannot be empty");
        }
        let value: Value = self
            .post_form(
                "/api/compose",
                &[
                    ("api_type", "json"),
                    ("to", to.as_str()),
                    ("subject", subject),
                    ("text", body),
                    ("type", "username"),
                ],
            )
            .await?;
        if let Some(errors) = value
            .get("json")
            .and_then(|json| json.get("errors"))
            .and_then(Value::as_array)
            && !errors.is_empty()
        {
            return Err(RedditError::Compose {
                message: format_reddit_errors(errors),
            }
            .into());
        }
        Ok(())
    }

    async fn posts_from_listing(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
    ) -> Result<Vec<Post>> {
        let listing: Listing<Post> = self.get(endpoint, params).await?;
        Ok(listing
            .data
            .children
            .into_iter()
            .map(|thing| thing.data)
            .collect())
    }

    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let token = self.token().await?;
        let url = format!("https://oauth.reddit.com{}", endpoint);
        let mut query: Vec<(&str, String)> = params.to_vec();
        query.push(("raw_json", "1".to_string()));
        let response = self
            .http
            .get(url)
            .bearer_auth(token.as_str())
            .query(&query)
            .send()
            .await?;
        self.decode_response(response).await
    }

    async fn post_form<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        form: &[(&str, &str)],
    ) -> Result<T> {
        let token = self.token().await?;
        let url = format!("https://oauth.reddit.com{}", endpoint);
        let response = self
            .http
            .post(url)
            .bearer_auth(token.as_str())
            .form(form)
            .send()
            .await?;
        self.decode_response(response).await
    }

    async fn decode_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();
        let rate_limit = RateLimitInfo::from_headers(response.headers());
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(RedditError::RateLimited { rate_limit }.into());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RedditError::Api {
                status,
                body: body.chars().take(1000).collect(),
                rate_limit,
            }
            .into());
        }
        Ok(response.json().await?)
    }

    async fn token(&self) -> Result<Zeroizing<String>> {
        let mut state = self.token.lock().await;
        if let Some(token) = &state.token
            && Instant::now() < state.expires_at
        {
            return Ok(token.clone());
        }

        let response = self
            .http
            .post("https://www.reddit.com/api/v1/access_token")
            .basic_auth(
                &self.config.client_id,
                Some(self.config.client_secret.as_str()),
            )
            .form(&[
                ("grant_type", "password"),
                ("username", self.config.username.as_str()),
                ("password", self.config.password.as_str()),
                ("scope", self.config.scope.as_str()),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RedditError::Auth {
                status,
                body: body.chars().take(1000).collect(),
            }
            .into());
        }

        let data: TokenResponse = response.json().await?;
        let token = Zeroizing::new(data.access_token);
        state.expires_at = Instant::now() + Duration::from_secs(data.expires_in.saturating_sub(60));
        state.token = Some(token.clone());
        Ok(token)
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn format_reddit_errors(errors: &[Value]) -> String {
    errors
        .iter()
        .map(|error| match error.as_array() {
            Some(parts) => parts
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(": "),
            None => error.to_string(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn collect_comments(value: &Value, depth: u8, max_depth: u8, output: &mut Vec<Comment>) {
    if depth > max_depth {
        return;
    }
    let Some(children) = value
        .get("data")
        .and_then(|data| data.get("children"))
        .and_then(Value::as_array)
    else {
        return;
    };

    for child in children {
        if child.get("kind").and_then(Value::as_str) != Some("t1") {
            continue;
        }
        let Some(data) = child.get("data") else {
            continue;
        };
        let mut comment: Comment = serde_json::from_value(data.clone()).unwrap_or_default();
        comment.depth = depth;
        output.push(comment);

        if let Some(replies) = data.get("replies")
            && replies.is_object()
        {
            collect_comments(replies, depth.saturating_add(1), max_depth, output);
        }
    }
}
