# clawdmeter-rs — context for Claude Code

ESP32 Rust firmware. Runs on GeekMagic SmallTV PRO (ESP32-D0WDQ6 rev1.0, dual-core 240 MHz, 520 KB SRAM, no PSRAM).
Polls Anthropic API for rate-limit headers and renders them on a 240×240 ST7789 display.

## Build

```bash
cargo build --release
```

No env vars needed for build — credentials are stored in NVS at runtime.

`export-esp.sh` is already sourced by the user before launching Claude Code — do NOT run it in commands.

Target: `xtensa-esp32-espidf`. Toolchain managed by `espup` — `rust-toolchain.toml` pins it to `esp`.
ESP-IDF version pinned to `v5.2.3` in `.cargo/config.toml`.

Do NOT use `uv run`. Do NOT add `std::thread::sleep` — use `esp_idf_hal::delay::FreeRtos::delay_ms`.

## Module map

| File | Responsibility |
|------|---------------|
| `src/main.rs` | peripherals init, serial prompt, HTTP server, poll loop |
| `src/display.rs` | ST7789 over SPI, `DrawTarget` impl |
| `src/wifi.rs` | `BlockingWifi` connect |
| `src/api.rs` | HTTPS POST to Anthropic, parse headers |
| `src/ui.rs` | `embedded-graphics` rendering |
| `src/nvs.rs` | NVS read/write for WiFi creds and token |

## Key types

- `display::Display<'d>` — holds `SpiDeviceDriver` + DC pin. Implements `embedded_graphics::DrawTarget<Color = Rgb565>`.
- `api::UsageData` — `util_5h`, `util_7d` (0.0–1.0 utilization), `reset_5h` (Unix ts), `now_ts`, `allowed` (bool).

## Hardware pin assignment

Defined at the top of `src/main.rs`. Change there only.

```
GPIO 18  SPI CLK   (VSPI) — alternative: GPIO 14 (HSPI)
GPIO 23  SPI MOSI  (VSPI) — alternative: GPIO 13 (HSPI)
GPIO  3  SPI CS    ← also UART0 RX — serial input must happen before Display::new()
GPIO  2  Display DC
GPIO  4  Display RST
GPIO 21  Backlight
```

**GPIO3 conflict**: GPIO3 is both SPI CS and UART0 RX on the PCB. Once `Display::new()` configures it as output (SPI CS), UART0 can no longer receive bytes from the host. The serial prompt in `main()` is intentionally placed before `Display::new()`.

## Credentials

Stored in NVS (namespace `clawdmeter`), never compiled in.

- **WiFi** (SSID + password): entered once via serial monitor on first boot (before display init). Survives reboots.
- **Anthropic token**: updated over-the-air any time the device is on WiFi:
  ```bash
  curl -X POST http://<device-ip>/token -d "sk-ant-..."
  ```

## Display driver notes

ST7789 needs `INVON` during init. SPI Mode 3 (CPOL=1, CPHA=1).
`fill_solid` is overridden on `Display` for performance: builds one row buffer, writes it H times. `draw_iter` does per-pixel writes (used for text/shapes only).

No `mipidsi` — driven directly to avoid `embedded-hal` 0.2/1.0 version conflicts.

## HTTPS / TLS

Uses `esp-idf-svc::http::client::EspHttpConnection` with `crt_bundle_attach` (mbedTLS certificate bundle from ESP-IDF). Must have `CONFIG_MBEDTLS_CERTIFICATE_BUNDLE=y` in `sdkconfig.defaults`.

## Useful commands

```bash
# Flash and open serial monitor
espflash flash target/xtensa-esp32-espidf/release/clawdmeter-rs --monitor

# Build + flash via script
./flash.sh

# Serial monitor only
espflash monitor
```
