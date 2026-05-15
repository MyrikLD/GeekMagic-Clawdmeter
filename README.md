# clawdmeter-rs

Rust firmware for the **GeekMagic SmallTV PRO** (ESP32-WROOM-32) that displays your [Claude Code](https://claude.ai/code) token and request rate-limit usage on the built-in 240×240 ST7789 display.

Inspired by [Clawdmeter](https://github.com/HermannBjorgvin/Clawdmeter). Instead of a BLE + Python daemon, the ESP32 polls the Anthropic API directly over WiFi every 60 seconds.

## Hardware

| Part | Details |
|------|---------|
| MCU | ESP32-D0WDQ6 rev1.0 (dual-core, 240 MHz, 520 KB SRAM, no PSRAM) |
| Display | ST7789, 240×240, SPI Mode 3 |
| Crystal | 40 MHz |
| Board | GeekMagic SmallTV PRO v02-22 |

### Pin wiring

| GPIO | Function | Notes |
|------|----------|-------|
| 18 | SPI CLK | VSPI default |
| 23 | SPI MOSI | VSPI default |
| 3 | SPI CS | |
| 2 | Display DC | |
| 4 | Display RST | |
| 21 | Backlight | high = on |

> **Alternative pinout:** some board revisions use CLK=14, MOSI=13 (HSPI). If the display stays blank, change those two lines in `src/main.rs`.

## What it shows

```
┌─────────────────────────┐
│      Claude Usage       │  ← orange title bar
│                         │
│ Tokens: 45000/100000    │
│ [████████░░░░░░░░░░░░]  │  ← green/yellow/red bar
│                         │
│ Requests: 8/50          │
│ [████░░░░░░░░░░░░░░░░]  │
│                         │
│        45 000           │  ← big remaining count
│      tokens left        │
│                         │
│   Resets: 14:30         │
└─────────────────────────┘
```

Bar color: green < 60% used, yellow < 85%, red ≥ 85%.

## Setup

### 1. Install the ESP32 Rust toolchain (once)

```bash
cargo install espup
espup install
. $HOME/export-esp.sh      # add to your shell profile
```

### 2. Install espflash

```bash
cargo install espflash
```

### 3. Set credentials

```bash
export WIFI_SSID="MyNet"
export WIFI_PASS="secret"
export ANTHROPIC_TOKEN="sk-ant-..."   # Anthropic API key
```

Credentials are compiled into the binary at build time via `env!()`.

### 4. Build and flash

```bash
# activate ESP toolchain first if not in shell profile
. $HOME/export-esp.sh

cargo run --release
# or flash without monitor:
cargo build --release
espflash flash target/xtensa-esp32-espidf/release/clawdmeter-rs --monitor
```

## Crate stack

| Crate | Role |
|-------|------|
| `esp-idf-hal` | SPI, GPIO, delay |
| `esp-idf-svc` | WiFi, HTTPS (mbedTLS) |
| `embedded-graphics` | 2D drawing primitives, fonts |

No external display driver library — ST7789 is driven directly to avoid version-compatibility issues between `embedded-hal` 1.0 and the `mipidsi`/`display-interface` ecosystem.

## Architecture

```
main loop (every 60s)
  └── api::fetch_usage()        POST /v1/messages (tiny body)
        └── parse rate-limit response headers
  └── ui::draw_usage()          render to display
```

The API call sends a minimal `max_tokens: 1` request to claude-haiku-4-5 purely to receive the `anthropic-ratelimit-*` response headers. No tokens are actually consumed beyond 1 output token per poll.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Display blank | Try CLK=14, MOSI=13 in `main.rs` |
| Display colors wrong | Toggle `INVON` in `display.rs::init()` |
| Display rotated | Change `MADCTL` byte in `display.rs::init()` |
| TLS handshake fails | Ensure `CONFIG_MBEDTLS_CERTIFICATE_BUNDLE=y` in `sdkconfig.defaults` |
| Compile error: env var not set | Export `WIFI_SSID`, `WIFI_PASS`, `ANTHROPIC_TOKEN` before building |
