use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;

static SUBREDDIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_]{1,21}$").unwrap());
static USERNAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]{3,20}$").unwrap());
static POST_ID_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9]{3,12}$").unwrap());
static FULL_POST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)reddit\.com/r/([^/]+)/comments/([a-z0-9]+)").unwrap());
static SHORT_POST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)redd\.it/([a-z0-9]+)").unwrap());

pub fn validate_subreddit(input: &str) -> Result<String> {
    let value = input.trim().trim_start_matches("r/");
    if value.is_empty() || !SUBREDDIT_RE.is_match(value) {
        bail!("invalid subreddit: {}", input);
    }
    Ok(value.to_string())
}

pub fn validate_username(input: &str) -> Result<String> {
    let value = input.trim().trim_start_matches("u/");
    if value.is_empty() || !USERNAME_RE.is_match(value) {
        bail!("invalid username: {}", input);
    }
    Ok(value.to_string())
}

pub fn extract_post_id(input: &str) -> Result<String> {
    let value = input.trim();
    if let Some(caps) = FULL_POST_RE.captures(value) {
        return validate_post_id(&caps[2]);
    }
    if let Some(caps) = SHORT_POST_RE.captures(value) {
        return validate_post_id(&caps[1]);
    }
    validate_post_id(value)
}

fn validate_post_id(input: &str) -> Result<String> {
    let value = input.trim();
    if value.is_empty() || !POST_ID_RE.is_match(value) {
        bail!("invalid post id: {}", input);
    }
    Ok(value.to_string())
}
