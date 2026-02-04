use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Feed {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub feeds: Vec<Feed>,
    pub open_command: Option<String>,
    pub header: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub feeds: Vec<Feed>,
    pub open_command: Option<String>,
    pub header: Option<String>,
}

pub fn load(feeds_override: Option<String>) -> Result<RuntimeConfig> {
    // If an override is provided, try to interpret it:
    if let Some(path_str) = feeds_override {
        let p = PathBuf::from(&path_str);
        if p.is_file() {
            // If it's a TOML, parse as config; otherwise treat as a single local feed
            let lc = path_str.to_ascii_lowercase();
            if lc.ends_with(".toml") {
                let txt = fs::read_to_string(&p)
                    .with_context(|| format!("failed to read config: {}", path_str))?;
                let parsed: AppConfig = toml::from_str(&txt)
                    .with_context(|| format!("failed to parse toml: {}", path_str))?;
                return Ok(RuntimeConfig {
                    feeds: parsed.feeds,
                    open_command: parsed.open_command,
                    header: parsed.header,
                });
            } else {
                let name = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("local-feed")
                    .to_string();
                return Ok(RuntimeConfig {
                    feeds: vec![Feed {
                        name,
                        url: path_str,
                    }],
                    open_command: None,
                    header: None,
                });
            }
        } else {
            // Not a file; if it's likely a URL, wrap as a single feed
            if path_str.starts_with("http://") || path_str.starts_with("https://") {
                return Ok(RuntimeConfig {
                    feeds: vec![Feed {
                        name: "Custom".into(),
                        url: path_str,
                    }],
                    open_command: None,
                    header: None,
                });
            }
        }
    }

    // Otherwise, try default config path
    if let Some(path) = default_config_path() {
        if path.is_file() {
            let txt = fs::read_to_string(&path)
                .with_context(|| format!("failed to read config: {}", path.display()))?;
            let parsed: AppConfig = toml::from_str(&txt)
                .with_context(|| format!("failed to parse toml: {}", path.display()))?;
            return Ok(RuntimeConfig {
                feeds: parsed.feeds,
                open_command: parsed.open_command,
                header: parsed.header,
            });
        }
    }

    // Built-in minimal defaults
    Ok(RuntimeConfig {
        feeds: vec![
            Feed {
                name: "HN Front".into(),
                url: "https://hnrss.org/frontpage".into(),
            },
            Feed {
                name: "BBC World".into(),
                url: "https://feeds.bbci.co.uk/news/world/rss.xml".into(),
            },
        ],
        open_command: None,
        header: None,
    })
}

fn default_config_path() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        let mut p = PathBuf::from(xdg);
        p.push("news-cli");
        p.push("config.toml");
        return Some(p);
    }
    if let Ok(home) = env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("news-cli");
        p.push("config.toml");
        return Some(p);
    }
    None
}
