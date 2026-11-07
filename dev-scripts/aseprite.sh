#!/bin/bash
# Runs one of the Lua scripts in dev-scripts/aseprite/ with the Aseprite CLI, so a call has one stable,
# allowlistable prefix: Bash(./dev-scripts/aseprite.sh:*)
#
# Usage:
#   ./dev-scripts/aseprite.sh <script> <input-file> [key=value ...]
#     <script>      a name in dev-scripts/aseprite/ without .lua: crop, frame_grid, check, sheet_analysis, set_durations
#     <input-file>  the file the script opens (.aseprite or .png)
#     key=value     passed as --script-param key=value
#
# Examples:
#   ./dev-scripts/aseprite.sh check games/long-way/assets/art/characters/Hunter.aseprite \
#     palette=games/long-way/assets/art/diodrama_palette.aseprite
#   ./dev-scripts/aseprite.sh frame_grid games/long-way/assets/art/characters/Hunter.aseprite \
#     tag=Run scale=3 out=target/screenshots/run.png
#   ./dev-scripts/aseprite.sh sheet_analysis games/long-way/input/hunter/Idle.png frame=96x64
#   ./dev-scripts/aseprite.sh crop target/screenshots/start.png cx=0 cy=0 w=160 h=160 \
#     view_height=576 scale=4 out=target/screenshots/start_crop.png
#
# ASEPRITE overrides the binary path. Scripts that write a file take out=<path>. Screenshots and grids go
# in target/screenshots/ and are deleted after the check (CLAUDE.md, "Disk use").
set -euo pipefail
cd "$(dirname "$0")/.."

ASEPRITE="${ASEPRITE:-/Applications/Aseprite.app/Contents/MacOS/aseprite}"

if [ -z "${1:-}" ] || [ -z "${2:-}" ]; then
  echo "✗ Usage: ./dev-scripts/aseprite.sh <script> <input-file> [key=value ...]" >&2
  exit 1
fi
script="dev-scripts/aseprite/$1.lua"
input="$2"
shift 2
if [ ! -f "$script" ]; then
  echo "✗ No script $script" >&2
  exit 1
fi
if [ ! -f "$input" ]; then
  echo "✗ File not found: $input" >&2
  exit 1
fi
if [ ! -x "$ASEPRITE" ]; then
  echo "✗ Aseprite not found at $ASEPRITE (set ASEPRITE)" >&2
  exit 1
fi

params=()
for pair in "$@"; do
  case "$pair" in
    *=*) params+=(--script-param "$pair") ;;
    *) echo "✗ Not key=value: $pair" >&2; exit 1 ;;
  esac
done

# Make the folder of any out=<path> before the script writes to it.
for pair in "$@"; do
  case "$pair" in
    out=*) mkdir -p "$(dirname "${pair#out=}")" ;;
  esac
done

"$ASEPRITE" -b "$input" ${params[@]+"${params[@]}"} --script "$script"
