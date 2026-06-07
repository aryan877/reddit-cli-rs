use std::collections::{BTreeMap, BTreeSet};

use crate::models::{Candidate, DraftMessage, Post, PostReport};

#[derive(Clone)]
pub struct CandidateOptions {
    pub min_score: i64,
    pub matches: Vec<String>,
    pub exclude: Vec<String>,
    pub include_post_author: bool,
}

pub fn extract_candidates_from_post(
    report: &PostReport,
    options: CandidateOptions,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let exclude = normalized_set(&options.exclude);
    let needles = normalize_needles(&options.matches);

    if options.include_post_author
        && !is_skipped_author(&report.post.author, &exclude)
        && text_matches(
            &format!("{} {}", report.post.title, report.post.selftext),
            &needles,
        )
    {
        candidates.push(Candidate {
            username: report.post.author.clone(),
            source_kind: "post".to_string(),
            source_id: report.post.id.clone(),
            source_subreddit: report.post.subreddit.clone(),
            source_url: reddit_url(&report.post.permalink),
            score: report.post.score,
            matched_text: first_non_empty(&[&report.post.selftext, &report.post.title]),
        });
    }

    for comment in &report.comments {
        if comment.score < options.min_score
            || is_skipped_author(&comment.author, &exclude)
            || !text_matches(&comment.body, &needles)
        {
            continue;
        }
        candidates.push(Candidate {
            username: comment.author.clone(),
            source_kind: "comment".to_string(),
            source_id: comment.id.clone(),
            source_subreddit: report.post.subreddit.clone(),
            source_url: reddit_url(&report.post.permalink),
            score: comment.score,
            matched_text: comment.body.clone(),
        });
    }

    dedupe_candidates(candidates)
}

pub fn extract_candidates_from_posts(posts: &[Post], options: CandidateOptions) -> Vec<Candidate> {
    let exclude = normalized_set(&options.exclude);
    let needles = normalize_needles(&options.matches);
    let candidates = posts
        .iter()
        .filter(|post| !is_skipped_author(&post.author, &exclude))
        .filter(|post| text_matches(&format!("{} {}", post.title, post.selftext), &needles))
        .map(|post| Candidate {
            username: post.author.clone(),
            source_kind: "post".to_string(),
            source_id: post.id.clone(),
            source_subreddit: post.subreddit.clone(),
            source_url: reddit_url(&post.permalink),
            score: post.score,
            matched_text: first_non_empty(&[&post.selftext, &post.title]),
        })
        .collect();
    dedupe_candidates(candidates)
}

pub fn dedupe_candidates(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut by_user: BTreeMap<String, Candidate> = BTreeMap::new();
    for candidate in candidates {
        let key = candidate.username.to_ascii_lowercase();
        match by_user.get(&key) {
            Some(existing) if existing.score >= candidate.score => {}
            _ => {
                by_user.insert(key, candidate);
            }
        }
    }
    by_user.into_values().collect()
}

pub fn render_drafts(
    candidates: &[Candidate],
    subject_template: &str,
    body_template: &str,
) -> Vec<DraftMessage> {
    candidates
        .iter()
        .map(|candidate| DraftMessage {
            to: candidate.username.clone(),
            subject: render_template(subject_template, candidate),
            body: render_template(body_template, candidate),
            source_url: candidate.source_url.clone(),
            approved: false,
        })
        .collect()
}

fn render_template(template: &str, candidate: &Candidate) -> String {
    template
        .replace("{username}", &candidate.username)
        .replace("{subreddit}", &candidate.source_subreddit)
        .replace("{source_url}", &candidate.source_url)
        .replace("{source_kind}", &candidate.source_kind)
        .replace("{matched_text}", &candidate.matched_text)
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn normalize_needles(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn text_matches(text: &str, needles: &[String]) -> bool {
    if needles.is_empty() {
        return true;
    }
    let haystack = text.to_ascii_lowercase();
    needles.iter().any(|needle| haystack.contains(needle))
}

fn is_skipped_author(author: &str, exclude: &BTreeSet<String>) -> bool {
    let normalized = author.to_ascii_lowercase();
    normalized == "[deleted]" || normalized == "automoderator" || exclude.contains(&normalized)
}

fn reddit_url(permalink: &str) -> String {
    if permalink.starts_with("http") {
        permalink.to_string()
    } else {
        format!("https://reddit.com{}", permalink)
    }
}

fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .find(|value| !value.trim().is_empty())
        .map(|value| (*value).to_string())
        .unwrap_or_default()
}
