use embedded_svc::{
    http::client::Client,
    http::Headers,
    io::Write,
};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};

#[derive(Debug, Default, Clone)]
pub struct UsageData {
    pub tokens_limit: u32,
    pub tokens_remaining: u32,
    pub requests_limit: u32,
    pub requests_remaining: u32,
    /// ISO-8601 reset timestamp, truncated to 32 chars
    pub reset_at: heapless::String<32>,
}

impl UsageData {
    pub fn tokens_used(&self) -> u32 {
        self.tokens_limit.saturating_sub(self.tokens_remaining)
    }

    /// 0.0 – 1.0 fraction of tokens consumed
    pub fn tokens_fraction(&self) -> f32 {
        if self.tokens_limit == 0 {
            return 0.0;
        }
        self.tokens_used() as f32 / self.tokens_limit as f32
    }
}

const API_URL: &str = "https://api.anthropic.com/v1/messages";
// Minimal body — we only care about the response headers
const BODY: &[u8] = br#"{"model":"claude-haiku-4-5-20251001","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#;

pub fn fetch_usage(token: &str) -> anyhow::Result<UsageData> {
    let config = HttpConfig {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let connection = EspHttpConnection::new(&config)?;
    let mut client = Client::wrap(connection);

    let content_len = BODY.len().to_string();
    let headers: &[(&str, &str)] = &[
        ("content-type", "application/json"),
        ("content-length", &content_len),
        ("x-api-key", token),
        ("anthropic-version", "2023-06-01"),
    ];

    let mut req = client.post(API_URL, headers)?;
    req.write_all(BODY)?;
    req.flush()?;
    let resp = req.submit()?;

    if resp.status() != 200 {
        // Still extract rate-limit headers on 4xx — they're present even on errors
        log::warn!("HTTP {}", resp.status());
    }

    let mut data = UsageData::default();

    if let Some(v) = resp.header("anthropic-ratelimit-tokens-limit") {
        data.tokens_limit = v.parse().unwrap_or(0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-tokens-remaining") {
        data.tokens_remaining = v.parse().unwrap_or(0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-requests-limit") {
        data.requests_limit = v.parse().unwrap_or(0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-requests-remaining") {
        data.requests_remaining = v.parse().unwrap_or(0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-tokens-reset") {
        let _ = data.reset_at.push_str(&v[..v.len().min(32)]);
    }

    // Drain response body to free the connection
    let mut buf = [0u8; 128];
    let mut reader = resp;
    loop {
        use embedded_svc::io::Read;
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            _ => {}
        }
    }

    Ok(data)
}
