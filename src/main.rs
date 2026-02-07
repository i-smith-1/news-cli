mod config;
mod history;
mod news;
mod open_url;
mod ui;
mod util;

use anyhow::Result;
use std::env;
use console::Term;

#[tokio::main]
async fn main() -> Result<()> {
    // Clear terminal at startup for a clean UI
    let _ = Term::stdout().clear_screen();
    // Parse a minimal CLI: optional --feeds <path>
    let mut args = env::args().skip(1);
    let mut feeds_override: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--feeds" => {
                if let Some(p) = args.next() { feeds_override = Some(p); }
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
    }

    let cfg = config::load(feeds_override)?;
    let mut history = history::SeenStories::load();

    loop {
        let items = vec!["News", "Quit"];
        let sel = ui::prompt_menu(
            "Main Menu (b = back/quit)",
            &items,
            Some(0),
            cfg.header.as_deref(),
        )?;
        match sel {
            ui::MenuChoice::Back => break,
            ui::MenuChoice::Index(0) => {
                let story_links = news::run(&cfg, &history).await?;
                // Mark all fetched stories as seen
                for link in story_links {
                    history.mark_as_seen(&link);
                }
            }
            ui::MenuChoice::Index(1) => break,
            _ => {}
        }
    }

    // Save history on clean exit
    if let Err(e) = history.save() {
        eprintln!("Failed to save history: {}", e);
    }

    Ok(())
}

fn print_help() {
    println!("news-cli");
    println!("Usage: news-cli [--feeds <path>]");
    println!("  --feeds <path>   Path to a config.toml (feeds list) or a local RSS/Atom XML file");
}
