# Tests for Long Way

Long Way tests only its own mechanics: its values, object kinds, weapons, enemies and HUD. Collision, the map parser, map transitions, depth sort and the other shared rules are tested in `game-kit` (`adr-w7`). No test reads a bundled map file, so the maps can change freely (`adr-lw17`). Tests are inline, in a `#[cfg(test)] mod tests` at the bottom of the file they test.

Rules for writing a test (prove that it can fail, exact values, one test layer per rule): [`dev-docs/testing.md`](../../../dev-docs/testing.md). Read it before you add or change a test.

## The test modules

| File | What it tests |
|---|---|
| `weapons/damage.rs` | `Contact`: damage ticks, the first tick at once, a long frame deals one tick with no burst (D14), pushback limits. `pushed_feet` stops at a blocked area and the map edge. `apply_hits` sends `DamageEnemy` and moves the enemy |
| `weapons/ring.rs` | The fire ring: a cast makes a ring, the front hurts once, the burn hurts every tick until the ring ends, no body is skipped at a low frame rate, the pushback limit, the burn starts at the lead radius, the damaging band. A body without `EnemyHp` and a dead enemy with a `HitBox` are not hit (D15) |
| `weapons/values.rs` | Data rules across all weapons. The ring size is a compile-time assert |
| `weapons/mod.rs` | The active weapon is cast from the player's feet with the player's facing, a blank slot casts nothing; no player, no cast |
| `weapon_menu.rs` | Which gauge opens or closes the drawer, the slide, one drawer per weapon and the label length, the default weapons and only built weapons active. Systems: a gauge, a click in the world and an active button click, the buttons or the "no weapons" text, only a primary click on a built weapon loads it into its own slot, the drawer is hidden at slide 0. The drawer sizes are compile-time asserts |
| `enemy_hp.rs` | `EnemyHp::lose`, the hurt colour hex, the tint and its end, one `EnemyDied`, hurt frames instead of the tint, a target without HP is left alone |
| `hp.rs` | The player's points stop at 0, and no `PlayerHurt` is sent at 0 |
| `ghost.rs` | Attack, death and hurt: which animation wins and when the player loses HP. The ghost's systems run before the kit's facing tags. A marker puts the ghost's feet at the cell centre; a bad strategy is skipped |
| `player.rs` | Facing from the walk direction, the sideways flip, no walking while a transition locks the player, the facing after an arrival, the `Hurt` animation |
| `map.rs` | Every legend name has its own `Kind` and the energy names give every `Energy`. `blocking_groups`: group sizes and `neighbour_above`. `touches_top` |
| `rocks.rs` | Placement (whole scale, the sort line), the blocked area in a group (with and without an open top) and alone, the jitter range |
| `trees.rs`, `scenery.rs` | Where the canvas hangs, the trunk band of an object alone and the anchor cell in a group |
| `markers.rs` | The blocked area on whole pixels and the touch area of an exit marker |
| `pickups.rs`, `inventory.rs` | Each pickup kind has the art and the name of its own energy. Counts per energy, the maximum and the fraction; an unknown kind is not counted |
| `hud.rs` | HP icon tags. Systems: each gauge follows its own energy, an active button shows its slot's weapon icon (no label) or is blank, only a primary click on a button or a gauge sends its energy, the HP row. The button frames, the gauge fill and the hover line are kit tests (`game_kit::hud`) |
| `background.rs` | Unknown tile names, the colour rectangle and the tile images of a map |

`test_util.rs` holds the helpers that several modules share: an app with manual time, an app that can load assets, small maps and objects, and the messages of the last update.

## How to run

```
./dev-scripts/rust.sh test long-way                 # all Long Way tests
./dev-scripts/rust.sh test long-way weapons::ring   # only tests whose path contains "weapons::ring"
./dev-scripts/rust.sh test game-kit                 # the kit's tests (collision, maps, transitions)
./dev-scripts/rust.sh lint                          # formats, then clippy, including test code
```

## Checks in the real game

Unit tests do not cover frame timing, drawing or input. Check those in the running game with the scripted input tool and screenshots (`adr-x5`, `dev-docs/extensions/dev-feedback.md`):

```
./dev-scripts/rust.sh script long-way "teleport:460:-100;hold:up:3;quit" --debug
./dev-scripts/rust.sh script long-way "wait:1;screenshot:name;quit" --debug    # saves target/screenshots/name.png
```

To check for flicker, take 12 screenshots in a row and count changed pixels between neighbours (`dev-docs/extensions/dev-feedback.md`).
