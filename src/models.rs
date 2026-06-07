use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Listing<T> {
    pub data: ListingData<T>,
}

#[derive(Debug, Deserialize)]
pub struct ListingData<T> {
    pub children: Vec<Thing<T>>,
}

#[derive(Debug, Deserialize)]
pub struct Thing<T> {
    pub data: T,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Subreddit {
    pub display_name: String,
    pub title: String,
    pub description: String,
    pub public_description: String,
    pub subscribers: i64,
    pub active_user_count: Option<i64>,
    pub created_utc: f64,
    pub over18: bool,
    pub subreddit_type: String,
    pub url: String,
    pub lang: String,
    pub accounts_active: Option<i64>,
    pub user_is_moderator: Option<bool>,
    pub user_is_subscriber: Option<bool>,
    pub submission_type: Option<String>,
    pub allow_images: Option<bool>,
    pub allow_videos: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct SubredditRule {
    pub short_name: String,
    pub description: String,
    pub kind: String,
    pub violation_reason: String,
    pub priority: i64,
}

#[derive(Debug, Deserialize)]
pub struct RulesResponse {
    pub rules: Vec<SubredditRule>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct PostRequirements {
    pub title_regexes: Vec<String>,
    pub body_regexes: Vec<String>,
    pub body_blacklisted_strings: Vec<String>,
    pub body_required_strings: Vec<String>,
    pub body_text_max_length: Option<i64>,
    pub body_text_min_length: Option<i64>,
    pub body_restriction_policy: Option<String>,
    pub domain_blacklist: Vec<String>,
    pub domain_whitelist: Vec<String>,
    pub gallery_captions_requirement: Option<String>,
    pub gallery_max_items: Option<i64>,
    pub gallery_min_items: Option<i64>,
    pub gallery_urls_requirement: Option<String>,
    pub guidelines_text: Option<String>,
    pub guidelines_display_policy: Option<String>,
    pub is_flair_required: Option<bool>,
    pub link_restriction_policy: Option<String>,
    pub link_repost_age: Option<i64>,
    pub title_blacklisted_strings: Vec<String>,
    pub title_required_strings: Vec<String>,
    pub title_text_max_length: Option<i64>,
    pub title_text_min_length: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub author: String,
    pub subreddit: String,
    pub score: i64,
    pub upvote_ratio: f64,
    pub num_comments: i64,
    pub created_utc: f64,
    pub permalink: String,
    pub url: String,
    pub selftext: String,
}

impl Default for Post {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            author: String::new(),
            subreddit: String::new(),
            score: 0,
            upvote_ratio: 0.0,
            num_comments: 0,
            created_utc: 0.0,
            permalink: String::new(),
            url: String::new(),
            selftext: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub body: String,
    pub score: i64,
    pub created_utc: f64,
    pub depth: u8,
}

impl Default for Comment {
    fn default() -> Self {
        Self {
            id: String::new(),
            author: String::new(),
            body: String::new(),
            score: 0,
            created_utc: 0.0,
            depth: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub created_utc: f64,
    pub link_karma: i64,
    pub comment_karma: i64,
    #[serde(default)]
    pub is_mod: bool,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Serialize)]
pub struct UserReport {
    pub profile: User,
    pub posts: Vec<Post>,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Serialize)]
pub struct PostReport {
    pub post: Post,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Serialize)]
pub struct SubredditContext {
    pub subreddit: Subreddit,
    pub rules: Vec<SubredditRule>,
    pub post_requirements: Option<PostRequirements>,
    pub recent_posts: Vec<Post>,
    pub top_posts: Vec<Post>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Candidate {
    pub username: String,
    pub source_kind: String,
    pub source_id: String,
    pub source_subreddit: String,
    pub source_url: String,
    pub score: i64,
    pub matched_text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DraftMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub source_url: String,
    #[serde(default)]
    pub approved: bool,
}
