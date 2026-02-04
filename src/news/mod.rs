mod fetch;
mod model;

use crate::config::RuntimeConfig;
use crate::open_url::open_url;
use crate::ui::{prompt_index, MenuChoice};
use anyhow::Result;
use std::collections::BTreeMap;

pub async fn run(cfg: &RuntimeConfig) -> Result<()> {
    // Initial fetch
    let stories = fetch::collect_stories(&cfg.feeds).await?;
    news_menu(stories).await
}

async fn news_menu(stories: Vec<model::Story>) -> Result<()> {
    // Group stories by source
    let mut by_source: BTreeMap<String, Vec<model::Story>> = BTreeMap::new();
    for s in stories {
        by_source.entry(s.source.clone()).or_default().push(s);
    }
    // Build a flat list: for each source, a header + first 10 items
    let mut labels: Vec<String> = Vec::new();
    enum Item { Header(String), Story(String, usize) } // (source, idx)
    let mut index_map: Vec<Item> = Vec::new();

    for (source, items) in &by_source {
        let count = items.len();
        labels.push(format!("== {} == ({} entries)", source.to_uppercase(), count));
        index_map.push(Item::Header(source.clone()));
        let show = items.iter().take(10);
        for (idx, it) in show.enumerate() {
            labels.push(format!("  - {}", it.title.replace('\n', " ").trim()));
            index_map.push(Item::Story(source.clone(), idx));
        }
    }

    loop {
        match prompt_index("News (b = back, q = quit). Select a headline; select a source name to see all entries.", &labels, None)? {
            MenuChoice::Back => break,
            MenuChoice::Index(i) => {
                match &index_map[i] {
                    Item::Header(source) => {
                        if let Some(v) = by_source.get(source) { source_menu(source, v).await?; }
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

async fn source_menu(source: &str, entries: &[model::Story]) -> Result<()> {
    let mut labels: Vec<String> = Vec::new();
    for e in entries {
        labels.push(e.title.replace('\n', " ").trim().to_string());
    }
    loop {
        match prompt_index(&format!("{} - all entries (b = back, q = quit)", source), &labels, None)? {
            MenuChoice::Back => break,
            MenuChoice::Index(i) => {
                if let Some(st) = entries.get(i) { let _ = open_url(&st.link); }
            }
        }
    }
    Ok(())
}

pub use model::Story;
