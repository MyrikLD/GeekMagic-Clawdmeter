use esp_idf_hal::{delay::FreeRtos, prelude::*};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use log::info;

mod api;
mod display;
mod ui;
mod wifi;

// ── Hardware pin assignment (GeekMagic SmallTV PRO, ESP32-WROOM-32) ──────────
// If the display stays blank, try the HSPI variant: CLK=14, MOSI=13.
//
//  GPIO 18  SPI CLK  (VSPI)
//  GPIO 23  SPI MOSI (VSPI)
//  GPIO  3  SPI CS
//  GPIO  2  Display DC
//  GPIO  4  Display RST
//  GPIO 21  Backlight (high = on)

// ── Compile-time credentials ──────────────────────────────────────────────────
// Set env vars before `cargo build`:
//   export WIFI_SSID="MyNet"
//   export WIFI_PASS="secret"
//   export ANTHROPIC_TOKEN="sk-ant-..."
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASS: &str = env!("WIFI_PASS");
const ANTHROPIC_TOKEN: &str = env!("ANTHROPIC_TOKEN");

const POLL_MS: u32 = 60_000;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let p = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

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

    info!("connecting wifi: {WIFI_SSID}");
    ui::status(&mut disp, "Connecting WiFi...")?;
    let _wifi = wifi::connect(p.modem, sysloop, nvs, WIFI_SSID, WIFI_PASS)?;
    info!("wifi up");

    loop {
        ui::status(&mut disp, "Fetching...")?;
        match api::fetch_usage(ANTHROPIC_TOKEN) {
            Ok(data) => {
                info!("tokens {}/{}", data.tokens_remaining, data.tokens_limit);
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
