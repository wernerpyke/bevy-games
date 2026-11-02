# Tests for Long Way

This folder holds no test code. Long Way keeps its unit tests inline, at the bottom of the file they test, in a `#[cfg(test)] mod tests`.

## Why inline

Rust integration tests in `tests/` are compiled as a separate crate. They can only call the public API of a **library** crate. Long Way is a binary crate with no `src/lib.rs`, so a test in this folder could not reach `resolve`, `parse` or `build`. Inline tests are in the same module, so they can test private functions directly.

Game rules are plain functions with no running Bevy app (see `CLAUDE.md`, architecture defaults), so unit tests cover them well.

## Where the tests are

| File | What it tests |
|---|---|
| `src/collision.rs` | the trunk blocked area and its walk-behind rules |
| `crates/game-kit/src/collision.rs` | `resolve`, `move_feet`, sliding, merged cluster cases, float noise at edges |
| `crates/game-kit/src/outline.rs` | merging rectangles into clusters, outline edges, inward corners, `Edge::covers` |
| `src/map.rs` | the bundled map file and the kind names |
| `crates/game-kit/src/map.rs` | the text map parser, including every error message |
| `src/rocks.rs` | rock placement, blocked areas, jitter |
| `crates/game-kit/src/depth.rs` | draw order from the feet |
| `crates/game-kit/src/debug.rs` | the grid line range (also for a moved camera), the visible rect, grid origin, and the info box rows |
| `crates/game-kit/src/collision_debug.rs` | the overlay line boxes |

## How to run

```
./dev-scripts/rust.sh test long-way     # all unit tests
./dev-scripts/rust.sh test long-way outline   # only tests whose name contains "outline"
./dev-scripts/rust.sh lint              # fmt check and clippy, including test code
```

## Checks in the real game

Unit tests do not cover frame timing, drawing or input. Check those in the running game with the scripted input tool and screenshots (`adr-x5`, `dev-docs/extensions/dev-feedback.md`):

```
./dev-scripts/rust.sh script long-way "teleport:460:-100;hold:up:3;quit"
./dev-scripts/rust.sh script long-way "wait:1;screenshot:name;quit"    # saves target/screenshots/name.png
```

To check for flicker, take 12 screenshots in a row and count changed pixels between neighbours (`dev-docs/extensions/dev-feedback.md`).

## When to add an integration test here

If a second game needs the same code, the shared parts move to a library crate (`adr-w6`). Tests that call only the public API of that crate then belong in its `tests/` folder. Until then, add tests inline.
