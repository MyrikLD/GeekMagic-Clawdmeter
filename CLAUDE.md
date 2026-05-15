# clawdmeter-rs — context for Claude Code

ESP32 Rust firmware. Runs on GeekMagic SmallTV PRO (ESP32-D0WDQ6 rev1.0, dual-core 240 MHz, 520 KB SRAM, no PSRAM).
Polls Anthropic API for rate-limit headers and renders them on a 240×240 ST7789 display.

## Build

```bash
export $(grep -v '^#' .env | xargs) && cargo build --release
```

`export-esp.sh` is already sourced by the user before launching Claude Code — do NOT run it in commands.
Credentials (WIFI_SSID, WIFI_PASS, ANTHROPIC_TOKEN) are in `.env`. Load them with `export $(grep -v '^#' .env | xargs)` before building.

Target: `xtensa-esp32-espidf`. Toolchain managed by `espup` — `rust-toolchain.toml` pins it to `esp`.
ESP-IDF version pinned to `v5.2.3` in `.cargo/config.toml`.

Do NOT use `uv run`. Do NOT add `std::thread::sleep` — use `esp_idf_hal::delay::FreeRtos::delay_ms`.

## Module map

| File | Responsibility |
|------|---------------|
| `src/main.rs` | peripherals init, poll loop |
| `src/display.rs` | ST7789 over SPI, `DrawTarget` impl |
| `src/wifi.rs` | `BlockingWifi` connect |
| `src/api.rs` | HTTPS POST to Anthropic, parse headers |
| `src/ui.rs` | `embedded-graphics` rendering |

## Key types

- `display::Display<'d>` — holds `SpiDeviceDriver` + DC pin. Implements `embedded_graphics::DrawTarget<Color = Rgb565>`.
- `api::UsageData` — parsed rate-limit fields: `tokens_limit`, `tokens_remaining`, `requests_limit`, `requests_remaining`, `reset_at`.

## Hardware pin assignment

Defined at the top of `src/main.rs`. Change there only.

```
GPIO 18  SPI CLK   (VSPI) — alternative: GPIO 14 (HSPI)
GPIO 23  SPI MOSI  (VSPI) — alternative: GPIO 13 (HSPI)
GPIO  3  SPI CS
GPIO  2  Display DC
GPIO  4  Display RST
GPIO 21  Backlight
```

## Display driver notes

ST7789 needs `INVON` during init. SPI Mode 3 (CPOL=1, CPHA=1).
`fill_solid` is overridden on `Display` for performance: builds one row buffer, writes it H times. `draw_iter` does per-pixel writes (used for text/shapes only).

No `mipidsi` — driven directly to avoid `embedded-hal` 0.2/1.0 version conflicts.

## HTTPS / TLS

Uses `esp-idf-svc::http::client::EspHttpConnection` with `crt_bundle_attach` (mbedTLS certificate bundle from ESP-IDF). Must have `CONFIG_MBEDTLS_CERTIFICATE_BUNDLE=y` in `sdkconfig.defaults`.

## Credentials

Compiled in via `env!()` macros in `main.rs`. Must be set as environment variables before `cargo build`. Not stored in code or NVS.

## Useful commands

```bash
# Flash and open serial monitor
espflash flash target/xtensa-esp32-espidf/release/clawdmeter-rs --monitor

# Serial monitor only (device already flashed)
espflash monitor

# Check logs without reflashing
cargo run --release 2>&1 | tee flash.log
```
