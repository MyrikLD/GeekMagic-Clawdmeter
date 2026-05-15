#!/usr/bin/env bash
set -euo pipefail

cargo build --release
espflash flash "target/xtensa-esp32-espidf/release/clawdmeter-rs" --monitor
