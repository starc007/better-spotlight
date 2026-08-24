#!/bin/bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_PATH="${PROJECT_DIR}/dist/Better Spotlight.app"

"${PROJECT_DIR}/scripts/package.sh"
open "${APP_PATH}"
