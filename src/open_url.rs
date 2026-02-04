use anyhow::{bail, Result};
use std::process::Command;
use url::Url;

pub fn open_url(url: &str) -> Result<()> {
    // Validate scheme strictly
    let u = Url::parse(url)?;
    match u.scheme() {
        "http" | "https" => {}
        _ => bail!("unsupported URL scheme"),
    }

    // Try using the system default
    if open::that(url).is_ok() {
        return Ok(());
    }
    // Fallback: try firefox directly
    let _ = Command::new("firefox")
        .arg("--new-tab")
        .arg("--")
        .arg(url)
        .spawn();
    Ok(())
}
