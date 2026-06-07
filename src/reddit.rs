use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use crate::config::Config;
use crate::models::{Comment, Listing, Post, PostReport, Thing, User, UserReport};
use crate::validation::{extract_post_id, validate_subreddit, validate_username};

#[derive(Debug, thiserror::Error)]
pub enum RedditError {
    #[error("reddit API returned {status}: {body}")]
    Api { status: StatusCode, body: String },
    #[error("reddit rate limit hit")]
    RateLimited,
    #[error("reddit authentication failed")]
    Auth,
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
        let _: Value = self
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
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(RedditError::RateLimited.into());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RedditError::Api {
                status,
                body: body.chars().take(1000).collect(),
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

        if !response.status().is_success() {
            return Err(RedditError::Auth.into());
        }

        let data: TokenResponse = response.json().await?;
        let token = Zeroizing::new(data.access_token);
        state.expires_at = Instant::now() + Duration::from_secs(data.expires_in.saturating_sub(60));
        state.token = Some(token.clone());
        Ok(token)
    }
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
