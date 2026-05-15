use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};

const NS: &str = "clawdmeter";

pub fn load_wifi(part: &EspDefaultNvsPartition) -> anyhow::Result<Option<(String, String)>> {
    let nvs = EspNvs::<NvsDefault>::new(part.clone(), NS, true)?;
    let mut sbuf = [0u8; 64];
    let ssid = match nvs.get_str("ssid", &mut sbuf)? {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Ok(None),
    };
    let mut pbuf = [0u8; 64];
    let pass = match nvs.get_str("pass", &mut pbuf)? {
        Some(s) => s.to_string(),
        None => return Ok(None),
    };
    Ok(Some((ssid, pass)))
}

pub fn save_wifi(part: &EspDefaultNvsPartition, ssid: &str, pass: &str) -> anyhow::Result<()> {
    let nvs = EspNvs::<NvsDefault>::new(part.clone(), NS, true)?;
    nvs.set_str("ssid", ssid)?;
    nvs.set_str("pass", pass)?;
    Ok(())
}

pub fn load_token(part: &EspDefaultNvsPartition) -> anyhow::Result<Option<String>> {
    let nvs = EspNvs::<NvsDefault>::new(part.clone(), NS, true)?;
    let mut buf = [0u8; 300];
    Ok(nvs.get_str("token", &mut buf)?.map(|s| s.to_string()))
}

pub fn save_token(part: &EspDefaultNvsPartition, token: &str) -> anyhow::Result<()> {
    let nvs = EspNvs::<NvsDefault>::new(part.clone(), NS, true)?;
    nvs.set_str("token", token)?;
    Ok(())
}
