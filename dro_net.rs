use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

pub struct Response {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

static CLIENT: OnceLock<Result<reqwest::blocking::Client, String>> = OnceLock::new();

fn client() -> Result<&'static reqwest::blocking::Client, String> {
    CLIENT
        .get_or_init(|| {
            reqwest::blocking::Client::builder()
                .use_rustls_tls()
                .no_proxy() // direct connection only
                .timeout(Duration::from_secs(20))
                .user_agent("Sparrow/0.1")
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| e.clone())
}

pub fn fetch(url: &str) -> Result<Response, String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Only http/https supported".into());
    }
    let resp = client()?.get(url).send().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let final_url = resp.url().to_string();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let mut body = Vec::new();
    resp.take(20 * 1024 * 1024).read_to_end(&mut body).map_err(|e| e.to_string())?;
    Ok(Response { url: final_url, status, content_type, body })
}

pub fn resolve(base: &str, href: &str) -> Option<String> {
    url::Url::parse(base).ok()?.join(href).ok().map(|u| u.to_string())
}
