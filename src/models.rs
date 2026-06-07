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
