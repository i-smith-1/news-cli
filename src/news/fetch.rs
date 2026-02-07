use super::model::Story;
use crate::config::Feed;
use crate::history::SeenStories;
use anyhow::Result;
use feed_rs::parser;
use futures_util::StreamExt;
use reqwest::Client;
use std::{fs, path::Path, time::Duration};
use url::Url;

pub async fn collect_stories(feeds: &[Feed], history: &SeenStories) -> Result<Vec<Story>> {
    let client = Client::builder()
        .user_agent("news-cli/0.1")
        .gzip(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()?;

    let mut all: Vec<Story> = Vec::new();

    // Fetch sequentially for simplicity; can be optimized later with concurrency
    for f in feeds {
        let source_name = f.name.clone();
        if Path::new(&f.url).is_file() {
            // Local XML file
            match fs::read(&f.url) {
                Ok(bytes) => {
                    if bytes.len() > max_feed_bytes() {
                        eprintln!("Feed too large ({} bytes): {}", bytes.len(), f.url);
                        continue;
                    }
                    match parser::parse(&bytes[..]) {
                        Ok(feed) => push_entries(&mut all, feed, &source_name, None, history),
                        Err(err) => eprintln!("Failed to parse feed {}: {}", f.url, err),
                    }
                }
                Err(err) => eprintln!("failed to read file feed {}: {}", f.url, err),
            }
        } else {
            // Remote URL
            let base = Url::parse(&f.url).ok();
            match client.get(&f.url).send().await {
                Ok(resp) => {
                    // Stream with a max size limit
                    let mut stream = resp.bytes_stream();
                    let mut buf: Vec<u8> = Vec::new();
                    let mut total: usize = 0;
                    let max = max_feed_bytes();
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(c) => {
                                total += c.len();
                                if total > max {
                                    eprintln!("Feed too large (>{} bytes): {}", max, f.url);
                                    buf.clear();
                                    break;
                                }
                                buf.extend_from_slice(&c);
                            }
                            Err(err) => {
                                eprintln!("Failed to read body {}: {}", f.url, err);
                                buf.clear();
                                break;
                            }
                        }
                    }
                    if buf.is_empty() { continue; }
                    match parser::parse(&buf[..]) {
                        Ok(feed) => push_entries(&mut all, feed, &source_name, base.as_ref(), history),
                        Err(err) => eprintln!("Failed to parse feed {}: {}", f.url, err),
                    }
                }
                Err(err) => eprintln!("Failed to fetch {}: {}", f.url, err),
            }
        }
    }

    // Dedupe by link
    all.sort_by(|a, b| a.link.cmp(&b.link));
    all.dedup_by(|a, b| a.link == b.link);

    Ok(all)
}

fn push_entries(
    all: &mut Vec<Story>,
    feed: feed_rs::model::Feed,
    fallback_source: &str,
    base: Option<&Url>,
    history: &SeenStories,
) {
    // Standardize source label to the configured feed name (fallback_source)
    // so ordering and labels match the configuration.
    let source = fallback_source.to_string();
    for entry in feed.entries.into_iter() {
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| "(untitled)".into());

        let raw_link = entry
            .links
            .iter()
            .find(|l| l.rel.as_deref().unwrap_or("") == "alternate")
            .or_else(|| entry.links.first())
            .map(|l| l.href.clone())
            .unwrap_or_else(|| String::from(""));

        if let Some(normalized) = normalize_link(&raw_link, base) {
            let is_new = !history.is_seen(&normalized);
            all.push(Story { 
                title, 
                link: normalized, 
                source: source.clone(),
                is_new,
            });
        }
    }
}

fn normalize_link(candidate: &str, base: Option<&Url>) -> Option<String> {
    if candidate.trim().is_empty() { return None; }
    let resolved = match Url::parse(candidate) {
        Ok(u) => u,
        Err(_) => {
            let b = base?;
            b.join(candidate).ok()?
        }
    };
    match resolved.scheme() {
        "http" | "https" => Some(resolved.into_string()),
        _ => None,
    }
}

fn max_feed_bytes() -> usize {
    // 5 MB cap
    5 * 1024 * 1024
}
