mod fetch;
mod model;

use crate::config::RuntimeConfig;
use crate::history::SeenStories;
use crate::open_url::open_url;
use crate::ui::{prompt_index, MenuChoice};
use crate::util::sanitize::sanitize_for_terminal;
use anyhow::Result;
use console;

pub async fn run(cfg: &RuntimeConfig, history: &SeenStories) -> Result<Vec<String>> {
    // Initial fetch
    let stories = fetch::collect_stories(&cfg.feeds, history).await?;
    
    // Collect all story links for later marking as seen
    let story_links: Vec<String> = stories.iter().map(|s| s.link.clone()).collect();
    
    news_menu(cfg, stories).await?;
    
    Ok(story_links)
}

async fn news_menu(cfg: &RuntimeConfig, stories: Vec<model::Story>) -> Result<()> {
    use std::collections::{HashMap, HashSet};
    // Group stories by source
    let mut by_source: HashMap<String, Vec<model::Story>> = HashMap::new();
    for s in stories {
        by_source.entry(s.source.clone()).or_default().push(s);
    }
    // Build a flat list following config feed order
    let mut labels: Vec<String> = Vec::new();
    enum Item { Header(String), Story(String, usize) } // (source, idx)
    let mut index_map: Vec<Item> = Vec::new();
    let mut header_indices: Vec<usize> = Vec::new();

    let mut seen: HashSet<String> = HashSet::new();
    for f in &cfg.feeds {
        let source = &f.name;
        if let Some(items) = by_source.get(source) {
            seen.insert(source.clone());
            let count = items.len();
            let safe_source = sanitize_for_terminal(&source.to_uppercase());
            header_indices.push(labels.len());
            labels.push(format!("== {} == ({} entries)", safe_source, count));
            index_map.push(Item::Header(source.clone()));
            let show = items.iter().take(10);
            for (idx, it) in show.enumerate() {
                let safe_title = sanitize_for_terminal(&it.title);
                let label = if it.is_new {
                    format!("  - {} {}", console::style("[NEW]").green().bold(), safe_title)
                } else {
                    format!("  - {}", safe_title)
                };
                labels.push(label);
                index_map.push(Item::Story(source.clone(), idx));
            }
        }
    }

    // Append any sources not in config order (defensive)
    for (source, items) in by_source.iter() {
        if seen.contains(source) { continue; }
        let count = items.len();
        let safe_source = sanitize_for_terminal(&source.to_uppercase());
        header_indices.push(labels.len());
        labels.push(format!("== {} == ({} entries)", safe_source, count));
        index_map.push(Item::Header(source.clone()));
        for (idx, it) in items.iter().take(10).enumerate() {
            let safe_title = sanitize_for_terminal(&it.title);
            let label = if it.is_new {
                format!("  - {} {}", console::style("[NEW]").green().bold(), safe_title)
            } else {
                format!("  - {}", safe_title)
            };
            labels.push(label);
            index_map.push(Item::Story(source.clone(), idx));
        }
    }

    loop {
        match prompt_index(
            "News (b = back, q = quit). Select a headline; select a source name to see all entries.",
            &labels,
            None,
            cfg.header.as_deref(),
            Some(&header_indices),
        )? {
            MenuChoice::Back => break,
            MenuChoice::Index(i) => {
                match &index_map[i] {
                    Item::Header(source) => {
                        if let Some(v) = by_source.get(source) { source_menu(cfg.header.as_deref(), source, v).await?; }
                    }
                    Item::Story(source, idx) => {
                        if let Some(v) = by_source.get(source) {
                            if let Some(st) = v.get(*idx) { let _ = open_url(&st.link); }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

async fn source_menu(global_header: Option<&str>, source: &str, entries: &[model::Story]) -> Result<()> {
    let mut labels: Vec<String> = Vec::new();
    for e in entries {
        let safe_title = sanitize_for_terminal(&e.title);
        let label = if e.is_new {
            format!("{} {}", console::style("[NEW]").green().bold(), safe_title)
        } else {
            safe_title
        };
        labels.push(label);
    }
    loop {
        match prompt_index(
            &format!("{} - all entries (b = back, q = quit)", source),
            &labels,
            None,
            global_header,
            None,
        )? {
            MenuChoice::Back => break,
            MenuChoice::Index(i) => {
                if let Some(st) = entries.get(i) { let _ = open_url(&st.link); }
            }
        }
    }
    Ok(())
}

pub use model::Story;
