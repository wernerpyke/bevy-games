# Plan: extract cross-game code from Long Way

Goal: R-Town reuses the Long Way extensions without copying files. Long Way keeps working and keeps its tests.

Trigger: R-Town is the second game (adr-w6). Steps marked **ASK** need developer approval before work starts.

## What exists today (checked in source)

| File | Lines | Generic? | Coupling to remove |
|---|---|---|---|
| `outline.rs` | 826 | Yes | Moved to `crates/game-kit` (step 3). Its tests used `GUY_HALF_WIDTH`, now a local test constant |
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
3. **Move `outline.rs`.** Done. Long Way imports `game_kit::outline`. The tests moved with the file. The only coupling was `GUY_HALF_WIDTH` in the tests, replaced by a local `ACTOR_HALF_WIDTH` of 20. Steps 1 to 3 also added an R-Town crate stub, because `members = ["games/*"]` fails without a manifest in `games/r-town`.
4. **Split `collision.rs`.** Done. The engine moved to `crates/game-kit/src/collision.rs`: `CollisionWorld`, `Blocker(Rect)` with `register_blockers` (adr-x12), `resolve`, `move_feet`, `SlideAssist`, `CollisionSettings`, `SlideTuning` and `CollisionPlugin { allow_sliding, cell_size }`. `SlideTuning::for_cell_size` scales the slide values from the cell size, so Long Way passes 64 and keeps its values. Long Way keeps `Trunk`, `GUY_*` and six trunk rule tests. `Trunk::blocker()` gives the component trees spawn. `dev-scripts/rust.sh` now logs `game_kit=debug` too.
5. **Move `depth.rs` and `layers.rs`.** Done. Both are in `crates/game-kit`. `layers.rs` is the kit default layers. `DepthPlugin { world_half_band }` has a `Default` that uses `WORLD_HALF_BAND`, and Long Way uses `DepthPlugin::default()`. The compile-time checks stay in the kit. `debug.rs` and `collision_debug.rs` still import `game_kit::layers` directly. Step 7 turns those into plugin fields. adr-x9 and CLAUDE.md now point at adr-x14.
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
