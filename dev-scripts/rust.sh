#!/bin/bash
# Thin wrapper around cargo for the workspace, giving build/test/run/lint
# one stable, allowlistable prefix: Bash(./dev-scripts/rust.sh:*)
#
# Games live in games/<name>/ and are workspace members.
#
# Usage:
#   ./dev-scripts/rust.sh build [game]        # build one game, or all
#   ./dev-scripts/rust.sh test [game]         # test one game, or all
#   ./dev-scripts/rust.sh run <game> [map-file] [--debug] [--skip-title]
#                                             # run a game, for example: run long-way. R-Town: run r-town test.txt (F1 reloads the map).
#                                             # --debug turns on the F2 info box and the F3 grid (off without it)
#   ./dev-scripts/rust.sh lint                # cargo fmt --check + clippy -D warnings (all games)
#   ./dev-scripts/rust.sh web [game]          # cargo check for wasm32-unknown-unknown
#   ./dev-scripts/rust.sh script <game> "<commands>" [--debug]
#                                             # run a game with scripted key presses and print its log,
#                                             # for example: script long-way "teleport:225:-150;hold:up:3;quit"
#                                             # add screenshot:<name> to save target/screenshots/<name>.png,
#                                             # for example: script long-way "wait:0.5;screenshot:start;quit"
#                                             # an optional map file after the commands starts the game in that map: script long-way "..." 2.txt
#   ./dev-scripts/rust.sh size                # disk use of target/ and its biggest folders
#   ./dev-scripts/rust.sh prune               # delete the incremental cache and stale binaries (see CLAUDE.md, "Disk use")
#   ./dev-scripts/rust.sh <other> [args...]   # passed straight to cargo
set -euo pipefail
cd "$(dirname "$0")/.."

# "-p <game>" when a game name is given, "--workspace" otherwise.
scope() {
  if [ -n "${1:-}" ]; then echo "-p $1"; else echo "--workspace"; fi
}

case "${1:-}" in
  build|test)
    cmd="$1"; shift
    # shellcheck disable=SC2046
    cargo "$cmd" $(scope "${1:-}") "${@:2}"
    ;;
  run)
    shift
    # --debug and --skip-title are not cargo arguments: they set GAME_KIT_DEBUG and GAME_KIT_SKIP_TITLE, which the kit reads.
    args=(); debug=0
    for arg in "$@"; do
      if [ "$arg" = "--debug" ]; then debug=1
      elif [ "$arg" = "--skip-title" ]; then export GAME_KIT_SKIP_TITLE=1
      else args+=("$arg"); fi
    done
    set -- ${args[@]+"${args[@]}"}
    if [ "$debug" = 1 ]; then export GAME_KIT_DEBUG=1; fi
    if [ -z "${1:-}" ]; then
      echo "✗ Usage: ./dev-scripts/rust.sh run <game> [map-file] [cargo args]" >&2
      exit 1
    fi
    game="$1"; shift
    # An optional map file: a path, or a name in games/<game>/assets/maps/. The game reads it from
    # <GAME>_MAP (only R-Town does so far). F1 in the game reloads it.
    if [ -n "${1:-}" ] && [ "${1#-}" = "$1" ]; then
      map="$1"; shift
      if [ ! -f "$map" ]; then map="games/$game/assets/maps/$map"; fi
      if [ ! -f "$map" ]; then
        echo "✗ Map file not found: $map" >&2
        exit 1
      fi
      variable="$(echo "$game" | tr 'a-z-' 'A-Z_')_MAP"
      export "$variable=$(cd "$(dirname "$map")" && pwd)/$(basename "$map")"
    fi
    cargo run -p "$game" "$@"
    ;;
  script)
    shift
    # A scripted run never shows the title screen: the script drives the player.
    export GAME_KIT_SKIP_TITLE=1
    args=(); debug=0
    for arg in "$@"; do
      if [ "$arg" = "--debug" ]; then debug=1; else args+=("$arg"); fi
    done
    set -- ${args[@]+"${args[@]}"}
    if [ "$debug" = 1 ]; then export GAME_KIT_DEBUG=1; fi
    if [ -z "${1:-}" ] || [ -z "${2:-}" ]; then
      echo "✗ Usage: ./dev-scripts/rust.sh script <game> \"<commands>\"" >&2
      exit 1
    fi
    game="$1"; commands="$2"; shift 2
    # An optional map file after the commands: a path, or a name in games/<game>/assets/maps/. The game starts in
    # that map (<GAME>_MAP), for example: script long-way "wait:0.5;screenshot:two;quit" 2.txt
    map_env=()
    if [ -n "${1:-}" ] && [ "${1#-}" = "$1" ]; then
      map="$1"
      if [ ! -f "$map" ]; then map="games/$game/assets/maps/$map"; fi
      if [ ! -f "$map" ]; then
        echo "✗ Map file not found: $1" >&2
        exit 1
      fi
      map_env=("$(echo "$game" | tr 'a-z-' 'A-Z_')_MAP=$(cd "$(dirname "$map")" && pwd)/$(basename "$map")")
    fi
    # The game reads <GAME>_SCRIPT, for example LONG_WAY_SCRIPT (see crates/game-kit/src/script.rs).
    variable="$(echo "$game" | tr 'a-z-' 'A-Z_')_SCRIPT"
    module="$(echo "$game" | tr '-' '_')"
    cargo build -p "$game" --quiet
    # A 30 second limit stops a stuck game. Output is filtered to the lines that matter; set
    # LOG_FILTER to a different grep pattern to see more.
    env "$variable=$commands" ${map_env[@]+"${map_env[@]}"} CARGO_MANIFEST_DIR="$PWD/games/$game" \
      RUST_LOG="${RUST_LOG:-info,$module=debug,game_kit=debug,wgpu=error,naga=warn}" \
      perl -e 'alarm 30; exec @ARGV' "./target/debug/$game" 2>&1 \
      | sed 's/\x1b\[[0-9;]*m//g' \
      | grep -E "${LOG_FILTER:-script:|collision:|WARN|ERROR|panicked}" || true
    ;;
  lint)
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    ;;
  web)
    shift
    # shellcheck disable=SC2046
    cargo check $(scope "${1:-}") --target wasm32-unknown-unknown
    ;;
  size)
    # Disk use of target/ and its folders. The limit is in CLAUDE.md ("Disk use").
    du -sh target target/* target/debug/* 2>/dev/null | sort -rh | head -12
    ;;
  prune)
    # Deletes what a build makes again on demand: the incremental cache and binaries of crates that no longer
    # exist in games/. Then cargo-sweep (optional, if installed) removes outputs not used for 14 days.
    rm -rf target/debug/incremental target/release/incremental
    for file in target/debug/* target/release/*; do
      [ -f "$file" ] && [ -x "$file" ] || continue
      name="$(basename "$file")"
      if [ ! -d "games/$name" ]; then
        echo "removed stale binary $file"
        rm -f "$file" "$file.d"
      fi
    done
    if command -v cargo-sweep >/dev/null 2>&1; then cargo sweep --time 14; fi
    ./dev-scripts/rust.sh size
    ;;
  "")
    echo "✗ Usage: ./dev-scripts/rust.sh <build|test|run|lint|web|size|prune|cargo-args>" >&2
    exit 1
    ;;
  *)
    cargo "$@"
    ;;
esac
