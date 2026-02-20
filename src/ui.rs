use anyhow::{anyhow, Result};
use console::{Key, Term};
use dialoguer::Input;

pub enum MenuChoice {
    Back,
    Quit,
    Index(usize),
}

pub fn prompt_menu(
    prompt: &str,
    items: &[&str],
    default: Option<usize>,
    header: Option<&str>,
) -> Result<MenuChoice> {
    // Clear on menu entry
    let term = Term::stdout();
    let _ = term.clear_screen();

    // Render initial view
    if let Some(h) = header {
        println!("{}", h);
    }
    println!("{}", prompt);
    for (i, it) in items.iter().enumerate() {
        println!("{}: {}", i + 1, it);
    }
    println!("Type a number + Enter, or use arrow keys + Enter. 'b' = back, 'q' = quit.");

    // First key decides input mode: arrow-navigation vs text input
    let key = term.read_key()?;
    match key {
        Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End | Key::PageUp | Key::PageDown => {
            return arrow_select(prompt, items, default, header, None);
        }
        Key::Char('q') | Key::Char('Q') => {
            return Ok(MenuChoice::Quit);
        }
        Key::Char('b') | Key::Char('B') => {
            return Ok(MenuChoice::Back);
        }
        Key::Enter => {
            if let Some(d) = default {
                return Ok(MenuChoice::Index(d));
            }
            return Err(anyhow!("no selection"));
        }
        Key::Char(c) => {
            // Fall back to text input initialized with the first typed char
            let mut builder = Input::new();
            builder = builder.with_prompt("Selection").allow_empty(true);
            if !c.is_control() {
                let init = c.to_string();
                // with_initial_text is available in dialoguer 0.11
                builder = builder.with_initial_text(init);
            }
            let input: String = builder.interact_text()?;
            return parse_selection(&input, items, default);
        }
        _ => {
            // Unknown key -> fallback to plain text input
            let input: String = Input::new()
                .with_prompt("Selection")
                .allow_empty(true)
                .interact_text()?;
            return parse_selection(&input, items, default);
        }
    }
}

pub fn prompt_index(
    prompt: &str,
    labels: &[String],
    default: Option<usize>,
    header: Option<&str>,
    header_indices: Option<&[usize]>,
) -> Result<MenuChoice> {
    let term = Term::stdout();
    let _ = term.clear_screen();
    if let Some(h) = header {
        println!("{}", h);
    }
    println!("{}", prompt);
    for (i, it) in labels.iter().enumerate() {
        println!("{}: {}", i + 1, it);
    }
    println!("Type a number + Enter, or use arrow keys + Enter. 'b' = back, 'q' = quit. Tab = next section");

    let key = term.read_key()?;
    match key {
        Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End | Key::PageUp | Key::PageDown => {
            return arrow_select_ref(prompt, labels, default, header, header_indices);
        }
        Key::Char('q') | Key::Char('Q') => {
            return Ok(MenuChoice::Quit);
        }
        Key::Char('b') | Key::Char('B') => {
            return Ok(MenuChoice::Back);
        }
        Key::Enter => {
            if let Some(d) = default {
                return Ok(MenuChoice::Index(d));
            }
            return Err(anyhow!("no selection"));
        }
        Key::Char(c) => {
            let mut builder = Input::new();
            builder = builder.with_prompt("Selection").allow_empty(true);
            if !c.is_control() {
                builder = builder.with_initial_text(c.to_string());
            }
            let s: String = builder.interact_text()?;
            return parse_selection(
                &s,
                &labels.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                default,
            );
        }
        _ => {
            let s: String = Input::new()
                .with_prompt("Selection")
                .allow_empty(true)
                .interact_text()?;
            return parse_selection(
                &s,
                &labels.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                default,
            );
        }
    }
}

fn parse_selection(input: &str, items: &[&str], default: Option<usize>) -> Result<MenuChoice> {
    let s = input.trim();
    if s.is_empty() {
        if let Some(d) = default {
            return Ok(MenuChoice::Index(d));
        }
        return Err(anyhow!("no selection"));
    }
    if s.eq_ignore_ascii_case("q") {
        return Ok(MenuChoice::Quit);
    }
    if s.eq_ignore_ascii_case("b") {
        return Ok(MenuChoice::Back);
    }
    let idx: usize = s
        .parse::<usize>()
        .map_err(|_| anyhow!("invalid selection"))?;
    if idx == 0 || idx > items.len() {
        return Err(anyhow!("out of range"));
    }
    Ok(MenuChoice::Index(idx - 1))
}

fn arrow_select(
    prompt: &str,
    items: &[&str],
    default: Option<usize>,
    header: Option<&str>,
    header_indices: Option<&[usize]>,
) -> Result<MenuChoice> {
    let term = Term::stdout();
    let mut sel = default.unwrap_or(0).min(items.len().saturating_sub(1));
    let mut top: usize = 0;
    loop {
        term.clear_screen()?;
        if let Some(h) = header {
            println!("{}", h);
        }
        println!("{}", prompt);

        let (rows_u16, _cols_u16) = term.size();
        let rows: usize = rows_u16 as usize;
        let reserved: usize = 2 + if header.is_some() { 1 } else { 0 }; // header + prompt + help
        let mut max_visible: usize = rows.saturating_sub(reserved);
        if max_visible < 3 {
            max_visible = 3;
        }
        if max_visible > items.len() {
            max_visible = items.len();
        }

        // keep selection in viewport
        if sel < top {
            top = sel;
        }
        let end = top + max_visible;
        if sel >= end {
            top = sel + 1 - max_visible;
        }

        let end = (top + max_visible).min(items.len());
        for i in top..end {
            if i == sel {
                println!("> {}: {}", i + 1, items[i]);
            } else {
                println!("  {}: {}", i + 1, items[i]);
            }
        }
        println!("Use arrows + Enter. 'b' = back, 'q' = quit. Tab = next section");

        match term.read_key()? {
            Key::ArrowUp => {
                if sel > 0 {
                    sel -= 1;
                }
            }
            Key::ArrowDown => {
                if sel + 1 < items.len() {
                    sel += 1;
                }
            }
            Key::Home => {
                sel = 0;
            }
            Key::End => {
                if !items.is_empty() {
                    sel = items.len() - 1;
                }
            }
            Key::PageUp => {
                let step: usize = max_visible.saturating_sub(1).max(1);
                sel = sel.saturating_sub(step);
            }
            Key::PageDown => {
                let step: usize = max_visible.saturating_sub(1).max(1);
                sel = (sel + step).min(items.len().saturating_sub(1));
            }
            Key::Tab => {
                if let Some(hidx) = header_indices {
                    if !hidx.is_empty() {
                        // find first header strictly greater than sel
                        let mut next = hidx[0];
                        for &idx in hidx {
                            if idx > sel {
                                next = idx;
                                break;
                            }
                        }
                        sel = next.min(items.len().saturating_sub(1));
                    }
                }
            }
            Key::Enter => {
                return Ok(MenuChoice::Index(sel));
            }
            Key::Char('q') | Key::Char('Q') => {
                return Ok(MenuChoice::Quit);
            }
            Key::Char('b') | Key::Char('B') | Key::Escape => {
                return Ok(MenuChoice::Back);
            }
            _ => {}
        }
    }
}

fn arrow_select_ref(
    prompt: &str,
    labels: &[String],
    default: Option<usize>,
    header: Option<&str>,
    header_indices: Option<&[usize]>,
) -> Result<MenuChoice> {
    let items: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    arrow_select(prompt, &items, default, header, header_indices)
}
