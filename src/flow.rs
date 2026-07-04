use console::Style;

use crate::history::{self, Entry, ResponseSnapshot};
use crate::httpclient::{self, SendOutcome};
use crate::jsonview;
use crate::model::{ApiKeyPlacement, AuthConfig, AuthType, Request, METHODS};
use crate::prompt;

enum FlowAction {
    NewRequest,
    Sent(Request, SendOutcome),
    Quit,
}

pub fn run() {
    println!("{}", Style::new().bold().apply_to("leanapi-lite \u{2014} a terminal HTTP client"));
    println!("{}", Style::new().dim().apply_to("(Ctrl+C at any prompt quits)\n"));

    let mut pending: Option<(Request, SendOutcome)> = None;
    loop {
        let (req, outcome) = match pending.take() {
            Some(pair) => pair,
            None => {
                let req = build_request_interactive(None);
                let outcome = httpclient::send_request(&req);
                (req, outcome)
            }
        };

        render_response(&outcome);
        log_entry(&req, &outcome);

        match post_response_menu(&outcome) {
            FlowAction::NewRequest => continue,
            FlowAction::Sent(r, o) => {
                pending = Some((r, o));
                continue;
            }
            FlowAction::Quit => break,
        }
    }

    println!("\nGoodbye!");
}

fn build_request_interactive(prefill: Option<&Request>) -> Request {
    println!();

    let default_method_idx = prefill
        .and_then(|r| METHODS.iter().position(|m| *m == r.method))
        .unwrap_or(0);
    let method_idx = prompt::select_inline("HTTP Method", &METHODS, default_method_idx);
    let method = METHODS[method_idx].to_string();

    let url = prompt::text_input("URL", prefill.map(|r| r.url.as_str()));

    let params = prompt::quick_entry(
        "Query Params (key=value, comma-separated, blank to skip)",
        '=',
        prefill.map(|r| r.params.as_slice()),
    );

    let headers = prompt::quick_entry(
        "Headers (Key:Value, comma-separated, blank to skip)",
        ':',
        prefill.map(|r| r.headers.as_slice()),
    );

    let auth = build_auth_interactive(prefill.map(|r| &r.auth));

    let cookies = prompt::quick_entry(
        "Cookies (key=value, comma-separated, blank to skip)",
        '=',
        prefill.map(|r| r.cookies.as_slice()),
    );

    let body = prompt::text_input("Body (raw, blank to skip)", prefill.map(|r| r.body.as_str()));

    Request { method, url, params, headers, cookies, auth, body }
}

fn build_auth_interactive(prefill: Option<&AuthConfig>) -> AuthConfig {
    let labels: Vec<&str> = AuthType::ALL.iter().map(|t| t.label()).collect();
    let default_idx = prefill
        .map(|a| AuthType::ALL.iter().position(|t| *t == a.auth_type).unwrap_or(0))
        .unwrap_or(0);
    let idx = prompt::select_inline("Auth Type", &labels, default_idx);
    let auth_type = AuthType::ALL[idx];

    let mut auth = AuthConfig { auth_type, ..Default::default() };

    match auth_type {
        AuthType::Basic => {
            auth.username = prompt::text_input("Username", prefill.map(|a| a.username.as_str()));
            auth.password = prompt::password_input("Password");
        }
        AuthType::Bearer => {
            auth.token = prompt::text_input("Token", prefill.map(|a| a.token.as_str()));
        }
        AuthType::ApiKey => {
            auth.api_key_name = prompt::text_input("Key Name", prefill.map(|a| a.api_key_name.as_str()));
            auth.api_key_value = prompt::text_input("Key Value", prefill.map(|a| a.api_key_value.as_str()));
            let placement_labels: Vec<&str> = ApiKeyPlacement::ALL.iter().map(|p| p.label()).collect();
            let default_placement_idx = prefill
                .map(|a| ApiKeyPlacement::ALL.iter().position(|p| *p == a.api_key_placement).unwrap_or(0))
                .unwrap_or(0);
            let p_idx = prompt::select_inline("Send In", &placement_labels, default_placement_idx);
            auth.api_key_placement = ApiKeyPlacement::ALL[p_idx];
        }
        AuthType::None => {}
    }

    auth
}

fn render_response(outcome: &SendOutcome) {
    println!();
    match &outcome.result {
        Err(e) => {
            println!(
                "{}",
                Style::new().red().bold().apply_to(format!("Error \u{2014} {}ms \u{2014} {}", outcome.duration_ms, e))
            );
        }
        Ok(snap) => {
            let status_style = if snap.status_code >= 200 && snap.status_code < 300 {
                Style::new().color256(34).bold()
            } else if snap.status_code >= 400 {
                Style::new().red().bold()
            } else {
                Style::new().dim()
            };
            println!(
                "{}  \u{2022}  {}ms  \u{2022}  {} bytes",
                status_style.apply_to(&snap.status),
                outcome.duration_ms,
                snap.size
            );

            println!("\n{}", Style::new().dim().apply_to("--- Body ---"));
            println!("{}", body_view(snap, true));

            println!("\n{}", Style::new().dim().apply_to("--- Headers ---"));
            let hv = headers_view(snap, true);
            if hv.is_empty() {
                println!("{}", Style::new().dim().apply_to("No headers."));
            } else {
                print!("{hv}");
            }

            println!("\n{}", Style::new().dim().apply_to("--- Cookies ---"));
            println!("{}", cookies_view(snap, true));
        }
    }
}

fn body_view(snap: &ResponseSnapshot, styled: bool) -> String {
    let (pretty, is_json) = jsonview::indent_json(&snap.body);
    if styled && is_json {
        jsonview::highlight(&pretty)
    } else {
        pretty
    }
}

fn headers_view(snap: &ResponseSnapshot, styled: bool) -> String {
    let mut out = String::new();
    for (k, values) in &snap.headers {
        for v in values {
            if styled {
                out.push_str(&format!("{}: {}\n", Style::new().color256(75).bold().apply_to(k), v));
            } else {
                out.push_str(&format!("{k}: {v}\n"));
            }
        }
    }
    out
}

fn cookies_view(snap: &ResponseSnapshot, styled: bool) -> String {
    let values: Vec<String> = snap
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    if values.is_empty() {
        let msg = "No cookies set in this response.";
        return if styled { Style::new().dim().apply_to(msg).to_string() } else { msg.to_string() };
    }

    let mut out = String::new();
    for (i, raw) in values.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match cookie::Cookie::parse(raw.clone()) {
            Ok(c) => {
                let name = if styled {
                    Style::new().color256(75).bold().apply_to(c.name()).to_string()
                } else {
                    c.name().to_string()
                };
                out.push_str(&format!("{} = {}\n", name, c.value()));
                if let Some(domain) = c.domain() {
                    out.push_str(&format!("  Domain: {domain}\n"));
                }
                if let Some(path) = c.path() {
                    out.push_str(&format!("  Path: {path}\n"));
                }
                if let Some(cookie::Expiration::DateTime(dt)) = c.expires() {
                    out.push_str(&format!("  Expires: {dt}\n"));
                }
                if let Some(max_age) = c.max_age() {
                    out.push_str(&format!("  Max-Age: {}\n", max_age.whole_seconds()));
                }
                if c.secure().unwrap_or(false) {
                    out.push_str("  Secure\n");
                }
                if c.http_only().unwrap_or(false) {
                    out.push_str("  HttpOnly\n");
                }
                if let Some(same_site) = c.same_site() {
                    out.push_str(&format!("  SameSite: {same_site:?}\n"));
                }
            }
            Err(_) => {
                out.push_str(raw);
                out.push('\n');
            }
        }
    }
    out
}

fn log_entry(req: &Request, outcome: &SendOutcome) {
    let entry = Entry {
        id: history::new_id(),
        timestamp: chrono::Utc::now(),
        request: req.clone(),
        response: outcome.result.as_ref().ok().cloned(),
        error: outcome.result.as_ref().err().cloned().unwrap_or_default(),
        duration_ms: outcome.duration_ms,
    };
    if let Err(e) = history::append(&entry) {
        eprintln!("{}", Style::new().color256(214).apply_to(format!("warning: failed to write history: {e}")));
    }
}

fn post_response_menu(outcome: &SendOutcome) -> FlowAction {
    loop {
        let options = ["Send another request", "Copy response to clipboard", "Browse history", "Quit"];
        println!();
        let idx = prompt::select_inline("Next", &options, 0);
        match idx {
            0 => return FlowAction::NewRequest,
            1 => match &outcome.result {
                Ok(snap) => copy_menu(snap),
                Err(_) => println!(
                    "{}",
                    Style::new().red().apply_to("No response body to copy (request errored).")
                ),
            },
            2 => {
                if let Some((req, out)) = history_browser() {
                    return FlowAction::Sent(req, out);
                }
                // else: user chose "Back", re-show this menu
            }
            _ => return FlowAction::Quit,
        }
    }
}

fn copy_menu(snap: &ResponseSnapshot) {
    let options = ["Body", "Headers", "Cookies", "Cancel"];
    let idx = prompt::select_inline("Copy which view", &options, 0);
    let text = match idx {
        0 => body_view(snap, false),
        1 => headers_view(snap, false),
        2 => cookies_view(snap, false),
        _ => return,
    };
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
        Ok(_) => println!("{}", Style::new().color256(34).bold().apply_to("Copied!")),
        Err(_) => println!("{}", Style::new().red().bold().apply_to("Copy failed")),
    }
}

/// Returns Some((request, outcome)) if the user resent an entry from
/// history, or None if they backed out to the caller's menu.
fn history_browser() -> Option<(Request, SendOutcome)> {
    loop {
        let mut entries = match history::load_all() {
            Ok(e) => e,
            Err(e) => {
                println!("{}", Style::new().red().apply_to(format!("Failed to load history: {e}")));
                return None;
            }
        };
        entries.reverse(); // newest first

        if entries.is_empty() {
            println!("{}", Style::new().dim().apply_to("No history yet \u{2014} send a request to see it here."));
            return None;
        }

        let mut labels: Vec<String> = entries.iter().map(format_history_row).collect();
        labels.push("Back".to_string());
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();

        println!();
        let idx = prompt::select_inline("History (newest first)", &label_refs, 0);
        if idx == entries.len() {
            return None;
        }

        let entry = &entries[idx];
        print_entry_detail(entry);

        let action_labels = ["Resend as-is", "Edit & resend", "Back to list"];
        let action_idx = prompt::select_inline("Entry action", &action_labels, 0);
        match action_idx {
            0 => {
                let outcome = httpclient::send_request(&entry.request);
                return Some((entry.request.clone(), outcome));
            }
            1 => {
                let req = build_request_interactive(Some(&entry.request));
                let outcome = httpclient::send_request(&req);
                return Some((req, outcome));
            }
            _ => continue,
        }
    }
}

fn format_history_row(e: &Entry) -> String {
    let status = if let Some(r) = &e.response {
        r.status_code.to_string()
    } else if !e.error.is_empty() {
        "ERR".to_string()
    } else {
        "\u{2014}".to_string()
    };
    let local_ts = e.timestamp.with_timezone(&chrono::Local).format("%m-%d %H:%M:%S");
    format!("{:<14} {:<7} {:<40} {}", local_ts, e.request.method, truncate(&e.request.url, 40), status)
}

fn print_entry_detail(e: &Entry) {
    println!("\n{}", Style::new().bold().apply_to(format!("{} {}", e.request.method, e.request.url)));
    if let Some(snap) = &e.response {
        println!("{}  \u{2022}  {}ms", snap.status, e.duration_ms);
    } else if !e.error.is_empty() {
        println!("{}", Style::new().red().apply_to(format!("Error: {}", e.error)));
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    if n <= 3 {
        return s.chars().take(n).collect();
    }
    let head: String = s.chars().take(n - 3).collect();
    format!("{head}...")
}
