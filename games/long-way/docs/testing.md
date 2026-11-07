# Tests for Long Way

This folder holds no test code. Long Way has no unit tests yet (`adr-lw17`, `adr-w7`). It may get some later, only for game mechanics that are not in `game-kit`. Collision and map rules are tested in `game-kit`.


## Where the tests are

| File | What it tests |
|---|---|
| `crates/game-kit/src/collision.rs` | `resolve`, `move_feet`, sliding, merged cluster cases, float noise at edges |
| `crates/game-kit/src/outline.rs` | merging rectangles into clusters, outline edges, inward corners, `Edge::covers` |
| `crates/game-kit/src/map.rs` | the text map parser, including every error message |
| `crates/game-kit/src/map_areas.rs` | exits and entries of one map file: area notation, edge rules, objects in areas |
| `crates/game-kit/src/map_links.rs` | the checks between maps: target map and entry, equal lengths, seamless edges, `exit_to` targets |
| `crates/game-kit/src/map_transition.rs` | the fade, exit and trigger detection, entry and `player_start` placement and the lifecycle with test hooks |
| `crates/game-kit/src/depth.rs` | draw order from the feet |
| `crates/game-kit/src/debug.rs` | the grid line range (also for a moved camera), the visible rect, grid origin, and the info box rows |
| `crates/game-kit/src/collision_debug.rs` | the overlay line boxes |

## How to run

```
./dev-scripts/rust.sh test game-kit     # all kit unit tests
./dev-scripts/rust.sh test game-kit outline   # only tests whose name contains "outline"
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
