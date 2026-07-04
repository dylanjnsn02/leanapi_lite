use std::io::Write;

use console::{Key, Term};
use dialoguer::{Input, Password};

use crate::model::Header;

/// An inline "pill" selector: prints `Label: ‹ Option ›`, Tab/Right advances,
/// Shift+Tab/Left goes back, Enter confirms and reprints the line without
/// the cycling brackets before moving down. Mirrors the Go TUI's method
/// pill, minus the mouse.
pub fn select_inline(label: &str, options: &[&str], default_idx: usize) -> usize {
    let term = Term::stdout();
    let mut idx = default_idx.min(options.len().saturating_sub(1));

    let render = |idx: usize, done: bool| {
        let _ = term.clear_line();
        if done {
            let _ = write!(&term, "{label}: {}\n", options[idx]);
        } else {
            let _ = write!(
                &term,
                "{label}: \u{2039} {} \u{203a}   (tab/\u{2190}\u{2192} to change, enter to confirm)",
                options[idx]
            );
        }
        let _ = term.flush();
    };

    render(idx, false);

    loop {
        match term.read_key() {
            Ok(Key::Tab) | Ok(Key::ArrowRight) => {
                idx = (idx + 1) % options.len();
                render(idx, false);
            }
            Ok(Key::BackTab) | Ok(Key::ArrowLeft) => {
                idx = (idx + options.len() - 1) % options.len();
                render(idx, false);
            }
            Ok(Key::Enter) => {
                render(idx, true);
                break;
            }
            Ok(Key::Char(c)) if c == '\u{3}' => {
                let _ = term.write_line("");
                std::process::exit(130);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    idx
}

/// A plain single-line text prompt. `default`, if set, is used when the user
/// just presses Enter (shown as a hint), letting history "edit & resend"
/// prefill old values without forcing them.
pub fn text_input(label: &str, default: Option<&str>) -> String {
    let mut input = Input::<String>::new();
    input = input.with_prompt(label).allow_empty(true);
    if let Some(d) = default {
        if !d.is_empty() {
            input = input.default(d.to_string());
        }
    }
    match input.interact_text() {
        Ok(v) => v,
        Err(_) => {
            println!();
            std::process::exit(130);
        }
    }
}

pub fn password_input(label: &str) -> String {
    match Password::new().with_prompt(label).allow_empty_password(true).interact() {
        Ok(v) => v,
        Err(_) => {
            println!();
            std::process::exit(130);
        }
    }
}

/// Parses a comma-separated `key<sep>value` quick-entry line into Headers.
/// A bare key with no separator is kept with an empty value. Blank/empty
/// segments are dropped. Every parsed pair is always enabled -- the
/// quick-entry format has no room for a per-row enable/disable toggle.
pub fn parse_pairs(input: &str, sep: char) -> Vec<Header> {
    input
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            match part.split_once(sep) {
                Some((k, v)) => Some(Header::new(k.trim(), v.trim())),
                None => Some(Header::new(part, "")),
            }
        })
        .collect()
}

/// Formats existing pairs back into quick-entry syntax, for prefilling a
/// default when editing a past request.
pub fn format_pairs(pairs: &[Header], sep: char) -> String {
    pairs
        .iter()
        .map(|h| format!("{}{}{}", h.key, sep, h.value))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A quick-entry prompt: one line of comma-separated `key<sep>value` pairs,
/// blank to skip.
pub fn quick_entry(label: &str, sep: char, default: Option<&[Header]>) -> Vec<Header> {
    let default_str = default.map(|d| format_pairs(d, sep));
    let raw = text_input(label, default_str.as_deref());
    if raw.trim().is_empty() {
        Vec::new()
    } else {
        parse_pairs(&raw, sep)
    }
}
