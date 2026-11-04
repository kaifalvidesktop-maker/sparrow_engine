/// Search engine (not Google/MS). Change if you want.
pub const SEARCH: &str = "https://duckduckgo.com/?q=";

pub fn resolve(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() { return "about:blank".into(); }
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("about:") { return s.to_string(); }
    let host = s.split('/').next().unwrap_or("");
    let looks_like_host = !s.contains(' ') && (host.contains('.') || host.starts_with("localhost"));
    if looks_like_host { format!("https://{s}") } else {
        format!("{SEARCH}{}", url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>())
    }
}
