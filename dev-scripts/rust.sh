#!/bin/bash
# Thin wrapper around cargo for the workspace, giving build/test/run/lint
# one stable, allowlistable prefix: Bash(./dev-scripts/rust.sh:*)
#
# Games live in games/<name>/ and are workspace members.
#
# Usage:
#   ./dev-scripts/rust.sh build [game]        # build one game, or all
#   ./dev-scripts/rust.sh test [game]         # test one game, or all
#   ./dev-scripts/rust.sh run <game>          # run a game, for example: run long-way
#   ./dev-scripts/rust.sh lint                # cargo fmt --check + clippy -D warnings (all games)
#   ./dev-scripts/rust.sh web [game]          # cargo check for wasm32-unknown-unknown
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
    if [ -z "${1:-}" ]; then
      echo "✗ Usage: ./dev-scripts/rust.sh run <game>" >&2
      exit 1
    fi
    game="$1"; shift
    cargo run -p "$game" "$@"
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
  "")
    echo "✗ Usage: ./dev-scripts/rust.sh <build|test|run|lint|web|cargo-args>" >&2
    exit 1
    ;;
  *)
    cargo "$@"
    ;;
esac
