#!/bin/bash
# Takes a screenshot of a scripted run and saves a zoomed crop of it, in one call.
#
# Usage:
#   ./dev-scripts/shot.sh <game> <name> "<commands>" [map=<map>] [key=value ...]
#     <commands>  the commands of `rust.sh script`, without screenshot and quit (both are added)
#     map=<map>   start in that map, as `rust.sh script` takes it (a name such as 3 or 3.txt, or a path)
#     key=value   crop options for dev-scripts/aseprite/crop.lua: cx, cy (world centre of the crop), w, h (world
#                 size), scale. Without cx and cy the crop is centred on the camera. Without w and h the whole
#                 window is saved.
#
# The run uses --debug. The world position of the crop comes from the view the script runner logs with the
# screenshot (camera centre, its pixel and pixels per world unit), so it is right wherever the camera is,
# also where it stops at a map edge and below or beside a HUD.
# Output: target/screenshots/<name>.png (whole window) and, with a crop, target/screenshots/<name>_crop.png.
# Delete both after the check (CLAUDE.md, "Disk use").
#
# Example (a 200 by 180 crop around the volcano of map 3, 2x):
#   ./dev-scripts/shot.sh long-way volcano "wait:1.5;teleport:-753:-430" map=3 cx=-736 cy=-330 w=200 h=180 scale=2
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -z "${1:-}" ] || [ -z "${2:-}" ] || [ -z "${3:-}" ]; then
  echo "✗ Usage: ./dev-scripts/shot.sh <game> <name> \"<commands>\" [map=<map>] [key=value ...]" >&2
  exit 1
fi
game="$1"; name="$2"; commands="$3"
shift 3

map=()
crop=()
for arg in "$@"; do
  case "$arg" in
    map=*) map=("${arg#map=}") ;;
    *) crop+=("$arg") ;;
  esac
done

log="$(mktemp)"
trap 'rm -f "$log"' EXIT
./dev-scripts/rust.sh script "$game" "$commands;screenshot:$name;quit" ${map[@]+"${map[@]}"} --debug 2>&1 | tee "$log"

png="target/screenshots/$name.png"
if [ ! -f "$png" ]; then
  echo "✗ No screenshot was saved: $png" >&2
  exit 1
fi
echo "✓ $png"

if [ "${#crop[@]}" -gt 0 ]; then
  view="$(sed -n 's/.*script: screenshot view \(cam_x=.*\)$/\1/p' "$log" | tail -1)"
  if [ -z "$view" ]; then
    echo "✗ The log has no screenshot view, so the crop cannot be placed" >&2
    exit 1
  fi
  echo "✓ view: $view"
  # shellcheck disable=SC2206
  view_args=($view)
  extra=()
  case " ${crop[*]} " in
    *" cx="*) ;;
    *) extra+=("cx=$(sed 's/.*cam_x=\([^ ]*\).*/\1/' <<<"$view")" "cy=$(sed 's/.*cam_y=\([^ ]*\).*/\1/' <<<"$view")") ;;
  esac
  ./dev-scripts/aseprite.sh crop "$png" "${crop[@]}" ${extra[@]+"${extra[@]}"} "${view_args[@]}" "out=target/screenshots/${name}_crop.png"
fi
