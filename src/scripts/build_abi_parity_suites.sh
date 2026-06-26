#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$ROOT/src/scripts/abi-parity-suite/build_suite.sh" --suite "${1:-all}"
