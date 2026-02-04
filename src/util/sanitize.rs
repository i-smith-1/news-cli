use regex::Regex;

// Remove ANSI escape sequences and non-printable control chars from untrusted text
// Collapse newlines/tabs to spaces and truncate to a reasonable length for terminal display.
pub fn sanitize_for_terminal(s: &str) -> String {
    // Regex to strip CSI (ESC[ ... cmd) sequences
    // This intentionally keeps it simple; it covers common ANSI sequences used for styling/movement.
    // If the regex fails to compile (shouldn't), we fallback to raw string handling.
    let re = Regex::new(r"\x1B\[[0-9;?]*[ -/]*[@-~]").ok();
    let no_ansi = if let Some(r) = &re {
        r.replace_all(s, "").into_owned()
    } else {
        s.to_string()
    };

    // Remove other control characters (C0 and DEL), keep basic space
    let mut cleaned = String::with_capacity(no_ansi.len());
    for ch in no_ansi.chars() {
        let keep = (ch >= ' ' && ch != '\x7f') || ch == ' ';
        if keep {
            cleaned.push(ch);
        }
    }

    // Normalize whitespace and trim
    let collapsed = cleaned.replace(['\n', '\r', '\t'], " ");
    let trimmed = collapsed.trim();

    // Truncate to 200 chars to avoid overly wide UI
    trimmed.chars().take(200).collect()
}
