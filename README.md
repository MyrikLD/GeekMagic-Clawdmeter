# SmallTV PRO Clawdmeter

Rust firmware for the **GeekMagic SmallTV PRO** (ESP32-WROOM-32) that displays your [Claude Code](https://claude.ai/code) token and request rate-limit usage on the built-in 240×240 ST7789 display.

Inspired by [Clawdmeter](https://github.com/HermannBjorgvin/Clawdmeter). Instead of a BLE + Python daemon, the ESP32 polls the Anthropic API directly over WiFi.

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
| 3 | SPI CS | also UART0 RX — see note below |
| 2 | Display DC | |
| 4 | Display RST | |
| 21 | Backlight | high = on |

> **Alternative pinout:** some board revisions use CLK=14, MOSI=13 (HSPI). If the display stays blank, change those two lines in `src/main.rs`.

> **GPIO3 note:** GPIO3 doubles as UART0 RX (serial input from host) and SPI CS. Serial input is only possible before the display is initialized. After that, token updates are done over WiFi.

## What it shows

```
┌─────────────────────────┐
│      Claude Usage       │  ← orange title bar
│                         │
│ 5h:  23.4%              │
│ [████░░░░░░░░░░░░░░░░]  │  ← green/yellow/red bar
│                         │
│ 7d:  61.2%              │
│ [████████████░░░░░░░░]  │
│                         │
│        ALLOWED          │  ← green / red "RATE LIMITED"
│          23%            │  ← big 5h utilization
│    5h utilization       │
│  resets in 2h 14m       │
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

### 3. Build and flash

No credentials needed at build time.

```bash
cargo build --release
espflash flash target/xtensa-esp32-espidf/release/clawdmeter-rs --monitor
# or:
./flash.sh
```

### 4. First-time WiFi setup

On first boot the device has no WiFi config. Open `espflash monitor` (or any serial terminal at 115200 baud) **before or immediately after** powering the device. You will be prompted:

```
I (...) clawdmeter_rs: === No WiFi config. Enter credentials in serial monitor ===
I (...) clawdmeter_rs: SSID:
MyNetwork
I (...) clawdmeter_rs: Password:
mysecret
```

WiFi credentials are saved to NVS and survive reboots. To reconfigure, erase NVS:
```bash
espflash erase-flash   # wipes everything; reflash firmware afterwards
```

### 5. Set the Anthropic token

The token is your Claude Code OAuth access token. Find it on the machine where Claude Code is installed:

```bash
python3 -c "import pathlib,json; print(json.loads(pathlib.Path('~/.claude/.credentials.json').expanduser().read_text())['claudeAiOauth']['accessToken'])"
```

Once the device is on Wi-Fi, the IP address is shown on the display and logged to serial. Send the token from any device on the same network:

```bash
curl -X POST http://<device-ip>/token -d "sk-ant-oat01-..."
```

The token is saved to NVS and used immediately. Update it the same way whenever the key rotates.

## Crate stack

| Crate | Role |
|-------|------|
| `esp-idf-hal` | SPI, GPIO, UART, delay |
| `esp-idf-svc` | WiFi, HTTPS (mbedTLS), HTTP server, NVS |
| `embedded-graphics` | 2D drawing primitives, fonts |

No external display driver library — ST7789 is driven directly to avoid version-compatibility issues between `embedded-hal` 1.0 and the `mipidsi`/`display-interface` ecosystem.

## Architecture

```
main loop (every 60s)
  └── api::fetch_usage()        POST /v1/messages (1 token, haiku)
        └── parse anthropic-ratelimit-unified-* headers
  └── ui::draw_usage()          render bars + countdown to display

HTTP server (background, port 80)
  └── POST /token               update Anthropic token in NVS
```

The API call sends a minimal `max_tokens: 1` request to claude-haiku-4-5 purely to receive the `anthropic-ratelimit-unified-*` response headers. The actual response content is discarded.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Display blank | Try CLK=14, MOSI=13 in `src/main.rs` |
| Display colors wrong | Toggle `INVON` in `display.rs::init()` |
| Display rotated | Change `MADCTL` byte in `display.rs::init()` |
| TLS handshake fails | Ensure `CONFIG_MBEDTLS_CERTIFICATE_BUNDLE=y` in `sdkconfig.defaults` |
| Serial prompt not appearing | Open monitor before powering the device |
| Can't type in serial prompt | Must open monitor before display init (GPIO3 conflict) |
| Token not updating | Check device IP in serial logs; ensure same network |
