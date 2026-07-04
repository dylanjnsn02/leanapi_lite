# leanapi-lite

A terminal HTTP client — Postman for your terminal — built in Rust. A guided, prompt-driven request builder walks you through method, URL, params, headers, auth, cookies, and body, then shows a pretty-printed, syntax-highlighted response. 

[[Demo]](https://raw.githubusercontent.com/dylanjnsn02/leanapi_lite/refs/heads/main/images/gif.gif)

A lean, no-TUI port of [leanapi](https://github.com/dylanjnsn02/leanapi).

## Features

- **Guided request builder** — cycle through HTTP methods (GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS) with a Tab/arrow-key pill selector, then step through URL, params, headers, auth, cookies, and body one prompt at a time.
- **Quick-entry fields** — Params, Headers, and Cookies accept comma-separated `key=value` (or `Key:Value` for headers) on a single line, blank to skip.
- **Auth** — No Auth, Basic, Bearer Token, or API Key (sent via header or query param), with the right follow-up prompts for each.
- **Pretty, highlighted responses** — status code, timing, and size at a glance, JSON responses indented and syntax-highlighted, response headers sorted and printed, cookies parsed from `Set-Cookie` (domain/path/expiry/flags included).
- **Copy to clipboard** — copy the response body, headers, or cookies view from the post-response menu.
- **Persistent history** — every request/response is logged locally; browse it, resend an entry as-is, or edit and resend with the old values pre-filled.

## Installation

Requires Rust 1.85+ (edition 2024).

```bash
git clone <this-repo-url>
cd leanapi_lite
cargo build --release
./target/release/leanapi-lite
```

Or run it directly without building a binary:

```bash
cargo run
```

## Usage

Each request walks through the same flow:

1. **HTTP Method** — `Tab` / `Shift+Tab` or `←` / `→` cycles the method pill, `Enter` locks it in.
2. **URL**
3. **Query Params** — `key=value, key2=value2`, blank to skip.
4. **Headers** — `Key:Value, Key2:Value2`, blank to skip.
5. **Auth Type** — cycle No Auth / Basic / Bearer Token / API Key, then fill in the fields that type needs.
6. **Cookies** — `key=value, ...`, blank to skip.
7. **Body** — raw text, blank to skip (defaults to `application/json` content type when a body is present and no explicit `Content-Type` header was set).

After the response prints, a menu lets you:

- **Send another request** — start the flow over.
- **Copy response to clipboard** — pick Body / Headers / Cookies.
- **Browse history** — newest-first list of past requests; pick one to resend as-is or edit-and-resend with its old values pre-filled.
- **Quit**

`Ctrl+C` at any prompt exits cleanly.

## Project layout

```
src/
  main.rs         entrypoint
  model.rs         Request/Header/Auth data types
  httpclient.rs    building and sending requests
  jsonview.rs      JSON pretty-printing and syntax highlighting
  history.rs       local JSONL request/response history
  prompt.rs        inline select widget, text/password prompts, quick-entry parsing
  flow.rs          the guided request-builder flow and response/history views
```

## License

MIT — see [LICENSE](LICENSE).
