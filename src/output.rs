use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::{Comment, Post, UserReport};

pub fn write_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_posts(posts: &[Post]) {
    if posts.is_empty() {
        println!("No posts found.");
        return;
    }
    for (index, post) in posts.iter().enumerate() {
        println!("[{}] {}", index + 1, clean(&post.title));
        println!(
            "    r/{} | u/{} | {} pts | {} comments | {}",
            clean(&post.subreddit),
            clean(&post.author),
            post.score,
            post.num_comments,
            time_ago(post.created_utc)
        );
        println!("    https://reddit.com{}", post.permalink);
        if !post.selftext.trim().is_empty() {
            println!("    {}", truncate(&clean(&post.selftext), 180));
        }
        println!();
    }
}

pub fn print_comments(comments: &[Comment]) {
    for comment in comments {
        let indent = "  ".repeat(comment.depth as usize);
        println!(
            "{}u/{} | {} pts | {}",
            indent,
            clean(&comment.author),
            comment.score,
            time_ago(comment.created_utc)
        );
        println!("{}{}", indent, truncate(&clean(&comment.body), 500));
        println!();
    }
}

pub fn print_user(report: &UserReport) {
    let user = &report.profile;
    println!(
        "u/{} | {} link karma | {} comment karma | created {}",
        clean(&user.name),
        user.link_karma,
        user.comment_karma,
        time_ago(user.created_utc)
    );
    if !report.posts.is_empty() {
        println!("\nRecent posts:");
        print_posts(&report.posts);
    }
    if !report.comments.is_empty() {
        println!("\nRecent comments:");
        print_comments(&report.comments);
    }
}

fn time_ago(created_utc: f64) -> String {
    let Some(created) = DateTime::<Utc>::from_timestamp(created_utc as i64, 0) else {
        return "unknown".to_string();
    };
    let secs = (Utc::now() - created).num_seconds().max(0);
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else if secs < 2_592_000 {
        format!("{}d ago", secs / 86400)
    } else if secs < 31_536_000 {
        format!("{}mo ago", secs / 2_592_000)
    } else {
        format!("{}y ago", secs / 31_536_000)
    }
}

fn clean(input: &str) -> String {
    input
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
        .collect::<String>()
        .replace('\n', " ")
        .trim()
        .to_string()
}

fn truncate(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    let mut output = input
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>();
    output.push('…');
    output
}
