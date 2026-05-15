#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all
cargo check
cargo run -p needle-desktop
