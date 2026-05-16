use embedded_svc::{
    http::client::Client,
    io::Write,
};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};

#[derive(Debug, Default, Clone)]
pub struct UsageData {
    /// 5-hour window utilization, 0.0–1.0
    pub util_5h: f32,
    /// 7-day window utilization, 0.0–1.0
    pub util_7d: f32,
    /// Unix timestamp of the 5h window reset
    pub reset_5h: u32,
    /// Unix timestamp when this response was received (from HTTP Date header)
    pub now_ts: u32,
    /// true = allowed, false = rate limited
    pub allowed: bool,
}

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const BODY: &[u8] = br#"{"model":"claude-haiku-4-5","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#;

pub fn fetch_usage(token: &str) -> anyhow::Result<UsageData> {
    let config = HttpConfig {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let connection = EspHttpConnection::new(&config)?;
    let mut client = Client::wrap(connection);

    let content_len = BODY.len().to_string();
    let bearer = format!("Bearer {}", token);
    let headers: &[(&str, &str)] = &[
        ("content-type", "application/json"),
        ("content-length", &content_len),
        ("Authorization", bearer.as_str()),
        ("anthropic-version", "2023-06-01"),
    ];

    let mut req = client.post(API_URL, headers)?;
    req.write_all(BODY)?;
    req.flush()?;
    let resp = req.submit()?;

    if resp.status() == 401 || resp.status() == 403 {
        let mut buf = [0u8; 128];
        let mut reader = resp;
        loop {
            match embedded_svc::io::Read::read(&mut reader, &mut buf) {
                Ok(0) | Err(_) => break,
                _ => {}
            }
        }
        return Err(anyhow::anyhow!("invalid_token"));
    }
    if resp.status() != 200 {
        log::warn!("HTTP {}", resp.status());
    }

    let mut data = UsageData { allowed: true, ..Default::default() };

    if let Some(v) = resp.header("anthropic-ratelimit-unified-5h-utilization") {
        data.util_5h = v.parse().unwrap_or(0.0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-unified-7d-utilization") {
        data.util_7d = v.parse().unwrap_or(0.0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-unified-5h-reset") {
        data.reset_5h = v.parse().unwrap_or(0);
    }
    if let Some(v) = resp.header("anthropic-ratelimit-unified-status") {
        data.allowed = v != "rejected";
    }
    if let Some(v) = resp.header("date") {
        data.now_ts = parse_http_date(v);
    }

    let mut buf = [0u8; 128];
    let mut reader = resp;
    loop {
        match embedded_svc::io::Read::read(&mut reader, &mut buf) {
            Ok(0) | Err(_) => break,
            _ => {}
        }
    }

    Ok(data)
}

/// Parse RFC 7231 date "Thu, 15 May 2026 17:18:08 GMT" → Unix timestamp (approx).
/// Returns 0 on parse failure.
fn parse_http_date(s: &str) -> u32 {
    // format: "Www, DD Mon YYYY HH:MM:SS GMT"
    let parts: heapless::Vec<&str, 8> = s.split_whitespace().collect();
    if parts.len() < 6 {
        return 0;
    }
    let day: u32 = parts[1].parse().unwrap_or(0);
    let mon = match parts[2] {
        "Jan" => 1u32, "Feb" => 2, "Mar" => 3, "Apr" => 4,
        "May" => 5, "Jun" => 6, "Jul" => 7, "Aug" => 8,
        "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12,
        _ => return 0,
    };
    let year: u32 = parts[3].parse().unwrap_or(0);
    let time = parts[4];
    let tparts: heapless::Vec<&str, 4> = time.split(':').collect();
    if tparts.len() < 3 { return 0; }
    let h: u32 = tparts[0].parse().unwrap_or(0);
    let m: u32 = tparts[1].parse().unwrap_or(0);
    let sec: u32 = tparts[2].parse().unwrap_or(0);

    // Days since Unix epoch (1970-01-01)
    let y = year;
    let leap_days = (y - 1) / 4 - (y - 1) / 100 + (y - 1) / 400;
    let year_days = (y - 1970) * 365 + leap_days.saturating_sub(477); // 477 = leap days before 1970
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days: [u32; 12] = [31, if is_leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let yday: u32 = month_days[..((mon - 1) as usize)].iter().sum::<u32>() + day - 1;

    (year_days + yday) * 86400 + h * 3600 + m * 60 + sec
}
