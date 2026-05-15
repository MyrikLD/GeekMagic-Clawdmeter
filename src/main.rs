use embedded_svc::{http::Method, io::Write};
use esp_idf_hal::{delay::FreeRtos, peripherals::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::{Configuration as HttpServerConfig, EspHttpServer},
    nvs::EspDefaultNvsPartition,
};
use log::info;

mod api;
mod display;
mod nvs;
mod ui;
mod wifi;

// ── Hardware pin assignment (GeekMagic SmallTV PRO, ESP32-WROOM-32) ──────────
//  GPIO 18  SPI CLK  (VSPI)
//  GPIO 23  SPI MOSI (VSPI)
//  GPIO  3  SPI CS   ← also UART0 RX, so serial input must happen before display init
//  GPIO  2  Display DC
//  GPIO  4  Display RST
//  GPIO 21  Backlight (high = on)

const POLL_MS: u32 = 60_000;

// Must be called before Display::new() — GPIO3 is UART0 RX until SPI takes it over.
fn prompt_line(label: &str) -> String {
    use std::io::{Read, Write};
    info!("{}", label);
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut result = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        match stdin.read(&mut byte) {
            Ok(0) => FreeRtos::delay_ms(50),
            Ok(_) => match byte[0] {
                b'\r' | b'\n' => {
                    if !result.is_empty() {
                        stdout.write_all(b"\r\n").ok();
                        stdout.flush().ok();
                        break;
                    }
                }
                0x08 | 0x7f => {
                    if !result.is_empty() {
                        result.pop();
                        stdout.write_all(b"\x08 \x08").ok();
                        stdout.flush().ok();
                    }
                }
                b => {
                    result.push(b);
                    stdout.write_all(&[b]).ok();
                    stdout.flush().ok();
                }
            },
            Err(_) => FreeRtos::delay_ms(50),
        }
    }
    String::from_utf8(result).unwrap_or_default()
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let p = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // ── WiFi credentials — MUST happen before display init ───────────────────
    // GPIO3 is UART0 RX until Display::new() reconfigures it as SPI CS.
    let (ssid, pass) = match nvs::load_wifi(&nvs)? {
        Some(creds) => {
            info!("wifi creds loaded from NVS");
            creds
        }
        None => {
            info!("=== No WiFi config. Enter credentials in serial monitor ===");
            FreeRtos::delay_ms(200);
            let ssid = prompt_line("SSID:");
            let pass = prompt_line("Password:");
            nvs::save_wifi(&nvs, &ssid, &pass)?;
            (ssid, pass)
        }
    };

    // ── Display init (GPIO3 → SPI CS after this point) ───────────────────────
    info!("init display");
    let mut disp = display::Display::new(
        p.spi2,
        p.pins.gpio18, // CLK
        p.pins.gpio23, // MOSI
        p.pins.gpio3,  // CS
        p.pins.gpio2,  // DC
        p.pins.gpio4,  // RST
        p.pins.gpio21, // BL
    )?;

    ui::splash(&mut disp)?;

    info!("connecting wifi: {}", ssid);
    ui::status(&mut disp, "Connecting WiFi...")?;
    let wifi = wifi::connect(p.modem, sysloop, nvs.clone(), &ssid, &pass)?;
    let ip_str = format!("{}", wifi.wifi().sta_netif().get_ip_info()?.ip);
    info!("wifi up, IP: {}", ip_str);

    let nvs_srv = nvs.clone();
    let mut _server = EspHttpServer::new(&HttpServerConfig::default())?;
    _server.fn_handler("/token", Method::Post, move |mut req| -> anyhow::Result<()> {
        let mut body = Vec::new();
        let mut buf = [0u8; 64];
        loop {
            match req.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => body.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        let token = std::str::from_utf8(&body).unwrap_or("").trim();
        if !token.is_empty() {
            nvs::save_token(&nvs_srv, token).ok();
            log::info!("token updated via HTTP");
        }
        req.into_ok_response()?.write_all(b"OK\n")?;
        Ok(())
    })?;

    ui::status(&mut disp, &format!("POST token to {}/token", ip_str))?;
    FreeRtos::delay_ms(3_000);

    loop {
        match nvs::load_token(&nvs)? {
            None => {
                ui::status(&mut disp, "No token - POST to /token")?;
                FreeRtos::delay_ms(5_000);
            }
            Some(tok) => {
                ui::status(&mut disp, "Fetching...")?;
                match api::fetch_usage(&tok) {
                    Ok(data) => {
                        info!(
                            "5h={:.1}% 7d={:.1}% allowed={}",
                            data.util_5h * 100.0,
                            data.util_7d * 100.0,
                            data.allowed
                        );
                        ui::draw_usage(&mut disp, &data)?;
                    }
                    Err(e) => {
                        log::warn!("fetch error: {e:#}");
                        ui::draw_error(&mut disp, &format!("{e:#}"))?;
                    }
                }
                FreeRtos::delay_ms(POLL_MS);
            }
        }
    }
}
