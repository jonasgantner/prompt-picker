#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
IDENTITY="${PROMPT_PICKER_SIGNING_IDENTITY:-Prompt Picker Local Code Signing}"

cd "$ROOT_DIR"

PATH="$HOME/.cargo/bin:$PATH" bun run tauri build --bundles app --config \
  "{\"bundle\":{\"macOS\":{\"signingIdentity\":\"$IDENTITY\",\"hardenedRuntime\":false}}}"

