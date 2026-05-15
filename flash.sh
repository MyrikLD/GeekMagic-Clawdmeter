#!/usr/bin/env bash
set -euo pipefail

while IFS='=' read -r key value || [[ -n "$key" ]]; do
    [[ -z "$key" || "$key" == \#* ]] && continue
    export "$key"="$value"
done < .env

cargo build --release
espflash flash "target/xtensa-esp32-espidf/release/clawdmeter-rs" --monitor
