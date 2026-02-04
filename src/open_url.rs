use anyhow::Result;
use std::process::Command;

pub fn open_url(url: &str) -> Result<()> {
    // Try using the system default
    if open::that(url).is_ok() {
        return Ok(());
    }
    // Fallback: try firefox directly
    let _ = Command::new("firefox").arg(url).spawn();
    Ok(())
}
