use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeenStories {
    seen_links: HashSet<String>,
}

impl SeenStories {
    pub fn load() -> Self {
        if let Some(path) = history_file_path() {
            if path.is_file() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    if let Ok(seen) = serde_json::from_str::<SeenStories>(&contents) {
                        return seen;
                    }
                }
            }
        }
        // Return empty history if file doesn't exist or can't be read
        SeenStories::default()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(path) = history_file_path() {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(&path, json)?;
        }
        Ok(())
    }

    pub fn mark_as_seen(&mut self, link: &str) {
        self.seen_links.insert(link.to_string());
    }

    pub fn is_seen(&self, link: &str) -> bool {
        self.seen_links.contains(link)
    }
}

fn history_file_path() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        let mut p = PathBuf::from(xdg);
        p.push("news-cli");
        p.push("seen_stories.json");
        return Some(p);
    }
    if let Ok(home) = env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("news-cli");
        p.push("seen_stories.json");
        return Some(p);
    }
    None
}
