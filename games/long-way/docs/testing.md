# Tests for Long Way

This folder holds no test code. Long Way keeps its unit tests inline, at the bottom of the file they test, in a `#[cfg(test)] mod tests`.

## Why inline

Rust integration tests in `tests/` are compiled as a separate crate. They can only call the public API of a **library** crate. Long Way is a binary crate with no `src/lib.rs`, so a test in this folder could not reach `resolve`, `parse` or `build`. Inline tests are in the same module, so they can test private functions directly.

Game rules are plain functions with no running Bevy app (see `CLAUDE.md`, architecture guidance), so unit tests cover them well.

## Where the tests are

| File | What it tests |
|---|---|
| `src/collision.rs` | `resolve`, `move_feet`, sliding, the trunk and merged cluster cases, float noise at edges |
| `src/outline.rs` | merging rectangles into clusters, outline edges, inward corners, `Edge::covers` |
| `src/map.rs` | the markdown map parser, including every error message and the bundled map file |
| `src/rocks.rs` | rock placement, blocked areas, jitter |
| `src/depth.rs` | draw order from the feet |
| `src/debug.rs` | the grid line range, grid origin, and the info box rows |

## How to run

```
./dev-scripts/rust.sh test long-way     # all unit tests
./dev-scripts/rust.sh test long-way outline   # only tests whose name contains "outline"
./dev-scripts/rust.sh lint              # fmt check and clippy, including test code
```

## Checks in the real game

Unit tests do not cover frame timing, drawing or input. Check those in the running game with the scripted input tool and screenshots (`adr-lw5`, `dev-docs/extensions/dev-feedback.md`):

```
./dev-scripts/rust.sh script long-way "teleport:460:-100;hold:up:3;quit"
./dev-scripts/rust.sh script long-way "wait:1;screenshot:name;quit"    # saves target/screenshots/name.png
```

To check for flicker, take 12 screenshots in a row and count changed pixels between neighbours (`dev-docs/extensions/dev-feedback.md`).

## When to add an integration test here

If a second game needs the same code, the shared parts move to a library crate (`adr-w6`). Tests that call only the public API of that crate then belong in its `tests/` folder. Until then, add tests inline.
