#!/usr/bin/env bash
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
sed -nE 's/^version *= *"([^"]+)".*/\1/p' "$HERE/../src-gui/Cargo.toml" | head -1
