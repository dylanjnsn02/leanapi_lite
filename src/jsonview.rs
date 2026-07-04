use console::Style;

/// Indents raw JSON text without parsing into a value type (which would risk
/// losing key order or numeric precision). Returns the original string and
/// `false` if `raw` is not valid JSON. Mirrors Go's `encoding/json.Indent`.
pub fn indent_json(raw: &str) -> (String, bool) {
    if serde_json::from_str::<serde::de::IgnoredAny>(raw).is_err() {
        return (raw.to_string(), false);
    }
    (reindent(raw), true)
}

fn reindent(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(src.len() * 2);
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;

    let newline_indent = |out: &mut String, depth: usize| {
        out.push('\n');
        for _ in 0..depth {
            out.push_str("  ");
        }
    };

    let mut i = 0;
    while i < n {
        let c = chars[i];

        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            i += 1;
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                let close = if c == '{' { '}' } else { ']' };
                if i + 1 < n && chars[i + 1] == close {
                    out.push(c);
                    out.push(close);
                    i += 2;
                    continue;
                }
                out.push(c);
                depth += 1;
                newline_indent(&mut out, depth);
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                newline_indent(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(c);
                newline_indent(&mut out, depth);
            }
            ':' => {
                out.push(c);
                out.push(' ');
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

/// Color-codes already-indented JSON: a "..." string immediately followed by
/// ':' (ignoring whitespace) is treated as a key, otherwise a value. Mirrors
/// Go's jsonview.Highlight — single lookahead, no full JSON parser.
pub fn highlight(indented: &str) -> String {
    let key_style = Style::new().color256(75).bold();
    let string_style = Style::new().color256(113);
    let number_style = Style::new().color256(214);
    let literal_style = Style::new().color256(176);

    let chars: Vec<char> = indented.chars().collect();
    let n = chars.len();
    let mut out = String::new();

    let mut i = 0;
    while i < n {
        let c = chars[i];

        if c == '"' {
            let mut j = i + 1;
            while j < n {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == '"' {
                    break;
                }
                j += 1;
            }
            let mut end = j;
            if end < n {
                end += 1; // include closing quote
            }
            let s: String = chars[i..end.min(n)].iter().collect();

            let mut k = end;
            while k < n && chars[k].is_whitespace() {
                k += 1;
            }
            if k < n && chars[k] == ':' {
                out.push_str(&key_style.apply_to(&s).to_string());
            } else {
                out.push_str(&string_style.apply_to(&s).to_string());
            }
            i = end;
            continue;
        }

        if c == '-' || c.is_ascii_digit() {
            let mut j = i;
            while j < n && is_number_char(chars[j]) {
                j += 1;
            }
            if j > i {
                let s: String = chars[i..j].iter().collect();
                out.push_str(&number_style.apply_to(&s).to_string());
                i = j;
                continue;
            }
        }

        if let Some(lit) = literal_at(&chars, i) {
            i += lit.chars().count();
            out.push_str(&literal_style.apply_to(&lit).to_string());
            continue;
        }

        out.push(c);
        i += 1;
    }

    out
}

fn is_number_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-'
}

fn literal_at(chars: &[char], i: usize) -> Option<String> {
    for lit in ["true", "false", "null"] {
        let l = lit.chars().count();
        if i + l > chars.len() {
            continue;
        }
        let candidate: String = chars[i..i + l].iter().collect();
        if candidate != lit {
            continue;
        }
        if i + l < chars.len() {
            let next = chars[i + l];
            if next.is_alphanumeric() {
                continue;
            }
        }
        return Some(lit.to_string());
    }
    None
}
