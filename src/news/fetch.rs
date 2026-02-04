use super::model::Story;
use crate::config::Feed;
use anyhow::{Context, Result};
use feed_rs::parser;
use reqwest::Client;
use std::{fs, path::Path};

pub async fn collect_stories(feeds: &[Feed]) -> Result<Vec<Story>> {
    let client = Client::builder()
        .user_agent("news-cli/0.1")
        .gzip(true)
        .build()?;

    let mut all: Vec<Story> = Vec::new();

    // Fetch sequentially for simplicity; can be optimized later with concurrency
    for f in feeds {
        let source_name = f.name.clone();
        if Path::new(&f.url).is_file() {
            // Local XML file
            let bytes = fs::read(&f.url)
                .with_context(|| format!("failed to read file feed: {}", f.url))?;
            match parser::parse(&bytes[..]) {
                Ok(feed) => push_entries(&mut all, feed, &source_name),
                Err(err) => eprintln!("Failed to parse feed {}: {}", f.url, err),
            }
        } else {
            // Remote URL
            match client.get(&f.url).send().await {
                Ok(resp) => match resp.bytes().await {
                    Ok(body) => match parser::parse(&body[..]) {
                        Ok(feed) => push_entries(&mut all, feed, &source_name),
                        Err(err) => eprintln!("Failed to parse feed {}: {}", f.url, err),
                    },
                    Err(err) => eprintln!("Failed to read body {}: {}", f.url, err),
                },
                Err(err) => eprintln!("Failed to fetch {}: {}", f.url, err),
            }
        }
    }

    // Dedupe by link
    all.sort_by(|a, b| a.link.cmp(&b.link));
    all.dedup_by(|a, b| a.link == b.link);

    Ok(all)
}

fn push_entries(all: &mut Vec<Story>, feed: feed_rs::model::Feed, fallback_source: &str) {
    let source = feed.title.map(|t| t.content).unwrap_or_else(|| fallback_source.to_string());
    for entry in feed.entries.into_iter() {
        let title = entry.title.as_ref().map(|t| t.content.clone()).unwrap_or_else(|| "(untitled)".into());
        let link = entry
            .links
            .iter()
            .find(|l| l.rel.as_deref().unwrap_or("") == "alternate")
            .or_else(|| entry.links.first())
            .map(|l| l.href.clone())
            .unwrap_or_else(|| String::from(""));
        if link.is_empty() { continue; }
        all.push(Story { title, link, source: source.clone() });
    }
}
