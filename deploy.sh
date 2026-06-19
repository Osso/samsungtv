#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo install --force --path .

echo "Installed samsungtv to ~/.cargo/bin"
