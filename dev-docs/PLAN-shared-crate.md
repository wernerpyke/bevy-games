# Plan: extract cross-game code from Long Way

Goal: R-Town reuses the Long Way extensions without copying files. Long Way keeps working and keeps its tests.

Trigger: R-Town is the second game (adr-w6). Steps marked **ASK** need developer approval before work starts.

## What exists today (checked in source)

| File | Lines | Generic? | Coupling to remove |
|---|---|---|---|
| `outline.rs` | 826 | Yes | None. Pure geometry on `Rect`, `Edge`, `Cluster` |
| `collision.rs` | 947 | Mostly | `GUY_*` constants and `Trunk` are Long Way specific. `CollisionWorld`, `resolve`, `move_feet`, `SlideAssist`, `CollisionPlugin` are generic. 60+ tests |
| `depth.rs` | 84 | Mostly | Imports `WORLD_HALF_BAND` from `layers.rs` |
| `layers.rs` | 52 | Pattern only | Constants are per game (R-Town needs its own layers) |
| `map.rs` | 908 | Parser yes, `Kind` no | `Kind` enum is hard coded to rocks and trees. `MAP_NAME` and `MAP_SOURCE` are constants |
| `debug.rs` | 294 | Mostly | Imports `Player`, `camera::half_view`, `DepthSort`, `layers` |
| `collision_debug.rs` | 221 | Mostly | Imports `layers`, `DebugSettings`, `register_trunks` |
| `script.rs` | 209 | Mostly | Queries `Player` and `Transform`. Commands are generic |
| `camera.rs` | 41 | Pattern | `VIEW_HEIGHT`, `CELL_SIZE`, `MAP_ORIGIN` are per game values |
| `player.rs` | 144 | No | Game specific (Guy) |
| `trees.rs`, `rocks.rs` | 106, 326 | No | Game specific |

## Decisions (made 2026-11-02)

| Topic | Decision | Record |
|---|---|---|
| Crate | One library crate, `crates/game-kit` | adr-x12 |
| Game values | Plugin fields and marker components. No generics, no Cargo features | adr-x12 |
| Map kinds | Strings in the kit, each game has its own enum | adr-x13 |
| Layers | Kit holds defaults, plugin fields override, games may add layers. Changes adr-x9 | adr-x14 |
| Script runner | In the kit. Game marks its target and can register its own commands | adr-x15 |

Still open: the shape of the generic collision blocker (step 4) and the game command API (step 8).

## Steps

Each step ends with `./dev-scripts/rust.sh lint`, `test` and a Long Way scripted run. Do one step per commit, and ask before each commit.

1. **Baseline.** Run `lint`, `test`, `web` and a scripted screenshot for Long Way. Save the screenshot to compare later.
2. **Create the crate.** `crates/game-kit/Cargo.toml` and empty `lib.rs`. Add `crates/*` to workspace `members` in the root `Cargo.toml`. Add `game-kit` to `[workspace.dependencies]` with a path. Check `rust.sh` handles a crate that is not a game.
3. **Move `outline.rs`.** No coupling. Long Way imports `game_kit::outline`. Tests move with the file.
4. **Split `collision.rs`.** Generic part to the kit: `CollisionWorld`, `resolve`, `move_feet`, `SlideAssist`, `CollisionSettings`, `CollisionPlugin`, the sliding constants and tests that use plain edges. Long Way keeps `Trunk`, `GUY_*` and the tests that use them. Decide where `Trunk`-style registration lives: a generic `Blocker` component with a `blocked_area` is the likely shape (ASK if the shape is unclear).
5. **Move `depth.rs` and `layers.rs`.** `layers.rs` becomes the kit default layers. `DepthPlugin` takes `world_half_band` as a field with that default. Update adr-x9 text in `dev-docs/adr/EXTENSIONS.md` to point at adr-x14. Keep the compile-time check in the kit.
6. **Make `map.rs` generic.** Replace `Kind` with string names (adr-x13). Add a resource for the map name and source. Long Way keeps its `Kind` enum and a `from_name` conversion. Update parser tests. Update `dev-docs/extensions/map-files.md`.
7. **Move `debug.rs` and `collision_debug.rs`.** Replace the `Player` import with a configurable target. Take the grid values and layer z values from the plugin fields.
8. **Move the script runner.** Move `script.rs` with a `ScriptTarget` marker (adr-x15). Add a command registration API only when R-Town needs its first game command.
9. **Camera.** Move only `half_view` and a `spawn_pixel_camera(view_height)` helper. `VIEW_HEIGHT`, `CELL_SIZE` and `MAP_ORIGIN` stay in the game.
10. **Verify Long Way.** Run `lint`, `test`, `web`, a scripted run and compare the screenshot with step 1. Say which parts were not seen on screen.
11. **Wire R-Town.** Create the R-Town crate as in `WORKFLOW.md` step 8 and use the kit. Check that nothing was copied.
12. **Update docs.** Update `dev-docs/extensions/*.md` "Code today" and the index. Update `games/long-way/docs/source-layout.md`. Add the new crate to `CLAUDE.md` and `dev-docs/README.md`. Update `adr-w6` status in `WORKSPACE.md`. Close roadmap item 2 and move it to Completed Items.
13. **Migrate and delete this plan.** Move lasting content to `docs/` and `dev-docs/`, add the lessons to `dev-docs/WORKFLOW.md`, then delete this file.

## Risks

- `collision.rs` has the most tests. Split it with the tests, not after.
- Bevy `Plugin` generics add complexity. Prefer plain fields and a marker component over type parameters.
- `bevy_aseprite_ultra` is used by `player.rs` only. Keep it out of the kit unless a shared animation helper is needed.
- A kit change can break both games. Run both games' `test` from step 4 onward.
