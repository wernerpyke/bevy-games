#!/bin/bash
# Takes a screenshot of a scripted run and saves a zoomed crop of it, in one call.
#
# Usage:
#   ./dev-scripts/shot.sh <game> <name> "<commands>" [key=value ...]
#     <commands>  the commands of `rust.sh script`, without screenshot and quit (both are added)
#     key=value   crop options for dev-scripts/aseprite/crop.lua: cx, cy (world centre), w, h (world size),
#                 scale. Without cx and cy the crop is centred on the world origin. Without w and h the
#                 whole window is saved.
#
# The run uses --debug. view_height comes from `pub const VIEW_HEIGHT` in games/<game>/src/camera.rs.
# Output: target/screenshots/<name>.png (whole window) and, with a crop, target/screenshots/<name>_crop.png.
# Delete both after the check (CLAUDE.md, "Disk use").
#
# Example (the player is in the middle of the view, so cx and cy are the player's world position):
#   ./dev-scripts/shot.sh long-way hunter "teleport:225:-150;wait:0.4;hold:up:0.5;wait:0.3" cx=225 cy=-94 w=160 h=160 scale=4
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -z "${1:-}" ] || [ -z "${2:-}" ] || [ -z "${3:-}" ]; then
  echo "✗ Usage: ./dev-scripts/shot.sh <game> <name> \"<commands>\" [key=value ...]" >&2
  exit 1
fi
game="$1"; name="$2"; commands="$3"
shift 3

./dev-scripts/rust.sh script "$game" "$commands;screenshot:$name;quit" --debug

png="target/screenshots/$name.png"
if [ ! -f "$png" ]; then
  echo "✗ No screenshot was saved: $png" >&2
  exit 1
fi
echo "✓ $png"

if [ "$#" -gt 0 ]; then
  camera="games/$game/src/camera.rs"
  view="$(sed -n 's/^pub const VIEW_HEIGHT: f32 = \([0-9.]*\);.*/\1/p' "$camera")"
  if [ -z "$view" ]; then
    echo "✗ No VIEW_HEIGHT in $camera" >&2
    exit 1
  fi
  extra=()
  case " $* " in
    *" cx="*) ;;
    *) extra+=(cx=0 cy=0) ;;
  esac
  ./dev-scripts/aseprite.sh crop "$png" "$@" ${extra[@]+"${extra[@]}"} "view_height=$view" "out=target/screenshots/${name}_crop.png"
fi
