use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::{
    Candidate, Comment, DraftMessage, Post, PostRequirements, Subreddit, SubredditContext,
    SubredditRule, UserReport,
};

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

pub fn print_subreddit(subreddit: &Subreddit) {
    println!("r/{}", clean(&subreddit.display_name));
    if !subreddit.title.trim().is_empty() {
        println!("title: {}", clean(&subreddit.title));
    }
    if !subreddit.public_description.trim().is_empty() {
        println!(
            "description: {}",
            truncate(&clean(&subreddit.public_description), 500)
        );
    }
    println!("subscribers: {}", subreddit.subscribers);
    if let Some(active) = subreddit.active_user_count.or(subreddit.accounts_active) {
        println!("active users: {}", active);
    }
    println!("type: {}", clean(&subreddit.subreddit_type));
    println!("nsfw: {}", subreddit.over18);
    if let Some(submission_type) = &subreddit.submission_type {
        println!("submission type: {}", clean(submission_type));
    }
    println!("url: https://reddit.com{}", subreddit.url);
}

pub fn print_rules(rules: &[SubredditRule]) {
    if rules.is_empty() {
        println!("No subreddit rules returned.");
        return;
    }
    for rule in rules {
        println!(
            "{}. {} ({})",
            rule.priority.saturating_add(1),
            clean(&rule.short_name),
            clean(&rule.kind)
        );
        if !rule.description.trim().is_empty() {
            println!("   {}", truncate(&clean(&rule.description), 700));
        }
        if !rule.violation_reason.trim().is_empty() {
            println!("   report reason: {}", clean(&rule.violation_reason));
        }
        println!();
    }
}

pub fn print_requirements(requirements: Option<&PostRequirements>) {
    let Some(requirements) = requirements else {
        println!(
            "No post requirements returned. The subreddit may not expose them or Reddit returned 404/403."
        );
        return;
    };

    println!("Post requirements:");
    print_optional_bool("flair required", requirements.is_flair_required);
    print_optional_text("title min length", requirements.title_text_min_length);
    print_optional_text("title max length", requirements.title_text_max_length);
    print_optional_text("body min length", requirements.body_text_min_length);
    print_optional_text("body max length", requirements.body_text_max_length);
    print_optional_str(
        "body policy",
        requirements.body_restriction_policy.as_deref(),
    );
    print_optional_str(
        "link policy",
        requirements.link_restriction_policy.as_deref(),
    );
    print_optional_str(
        "guidelines policy",
        requirements.guidelines_display_policy.as_deref(),
    );
    if let Some(text) = &requirements.guidelines_text
        && !text.trim().is_empty()
    {
        println!("guidelines: {}", truncate(&clean(text), 700));
    }
    print_string_list(
        "title required strings",
        &requirements.title_required_strings,
    );
    print_string_list(
        "title blacklisted strings",
        &requirements.title_blacklisted_strings,
    );
    print_string_list("title regexes", &requirements.title_regexes);
    print_string_list("body required strings", &requirements.body_required_strings);
    print_string_list(
        "body blacklisted strings",
        &requirements.body_blacklisted_strings,
    );
    print_string_list("body regexes", &requirements.body_regexes);
    print_string_list("domain whitelist", &requirements.domain_whitelist);
    print_string_list("domain blacklist", &requirements.domain_blacklist);
}

pub fn print_subreddit_context(context: &SubredditContext) {
    print_subreddit(&context.subreddit);
    println!("\nRules:");
    print_rules(&context.rules);
    println!("Posting requirements:");
    print_requirements(context.post_requirements.as_ref());
    if !context.recent_posts.is_empty() {
        println!("\nRecent posts:");
        print_posts(&context.recent_posts);
    }
    if !context.top_posts.is_empty() {
        println!("\nTop posts:");
        print_posts(&context.top_posts);
    }
}

pub fn print_candidates(candidates: &[Candidate]) {
    if candidates.is_empty() {
        println!("No candidates found.");
        return;
    }
    for (index, candidate) in candidates.iter().enumerate() {
        println!(
            "[{}] u/{} | {} | r/{} | {} pts",
            index + 1,
            clean(&candidate.username),
            clean(&candidate.source_kind),
            clean(&candidate.source_subreddit),
            candidate.score
        );
        println!("    {}", candidate.source_url);
        println!("    {}", truncate(&clean(&candidate.matched_text), 220));
        println!();
    }
}

pub fn print_drafts(drafts: &[DraftMessage]) {
    if drafts.is_empty() {
        println!("No drafts generated.");
        return;
    }
    for (index, draft) in drafts.iter().enumerate() {
        println!("[{}] to u/{}", index + 1, clean(&draft.to));
        println!("subject: {}", clean(&draft.subject));
        println!("source: {}", draft.source_url);
        println!("body:\n{}", draft.body);
        println!();
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

fn print_optional_bool(label: &str, value: Option<bool>) {
    if let Some(value) = value {
        println!("{}: {}", label, value);
    }
}

fn print_optional_text<T: std::fmt::Display>(label: &str, value: Option<T>) {
    if let Some(value) = value {
        println!("{}: {}", label, value);
    }
}

fn print_optional_str(label: &str, value: Option<&str>) {
    if let Some(value) = value
        && !value.trim().is_empty()
    {
        println!("{}: {}", label, clean(value));
    }
}

fn print_string_list(label: &str, values: &[String]) {
    let values = values
        .iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| clean(value))
        .collect::<Vec<_>>();
    if !values.is_empty() {
        println!("{}: {}", label, values.join(", "));
    }
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
