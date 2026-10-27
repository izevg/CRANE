#!/usr/bin/env bash
# docs/product + guidebook/manifest.yaml -> out/CRANE-Architecture-v1.0.pdf (генератор guidebook-render)
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
gb="${GB_RENDER:-$root/../guidebook-render}"
export GB_MMDC="${GB_MMDC:-npx -y @mermaid-js/mermaid-cli}"
mkdir -p "$root/out"
cd "$gb" && uv run gb-build "$root/guidebook" "$root/out/CRANE-Architecture-v1.0.pdf" --strict "$@"
