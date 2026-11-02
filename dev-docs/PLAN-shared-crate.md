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
6. **Make `map.rs` generic.** Done. The parser and its tests (23) are in `crates/game-kit/src/map.rs`: `parse(file, source, kinds)`, `Map`, `MapObject` (kind is a `String`) and `MapError`. Long Way keeps `Kind`, `Kind::of(&MapObject)`, the kind name list, `MAP_NAME`/`MAP_SOURCE` and `MapPlugin` in its own `map.rs`. `draw_size` is `Option<f32>`, and rocks use 128 when it is missing. The plugin that loads the file stays in the game, because `include_str!` needs the game's path.
7. **Move `debug.rs` and `collision_debug.rs`.** Done. Both are in `crates/game-kit`. A new kit marker `Controlled` replaces `Player` (the info box and, in step 8, the script runner use it). The grid reads the camera's position and orthographic `area`, so it follows a scrolling camera, and its edge labels are pinned each frame. `DebugPlugin::new(cell_size, grid_color, grid_origin)` has the fields `target_name`, `grid_z` and `hud_z_index`. `CollisionDebugPlugin` has `outline_z`, `interior_z` and `feet_z` with kit defaults (adr-x14). `half_view` stays in `camera.rs` for the player clamp (step 9).
8. **Move the script runner.** Done. `script.rs` is in `crates/game-kit` with `ScriptPlugin { env_prefix }` (Long Way: "LONG_WAY", so `LONG_WAY_SCRIPT` and `LONG_WAY_SCREENSHOT_DIR` are unchanged). It moves the `Controlled` entity and takes the feet from its `DepthSort`, not from a Guy constant. The command registration API is not built, as adr-x15 says. Three parser tests were added.
9. **Camera.** Done. `crates/game-kit/src/camera.rs` has `PixelCameraPlugin { view_height, clear_color }` (a plugin, not a helper function, to match adr-x12) and `half_view(window_width, window_height, view_height)`. Long Way's `camera.rs` keeps `VIEW_HEIGHT`, `CLEAR_COLOR`, `CELL_SIZE`, `MAP_ORIGIN` and a `half_view(window)` wrapper for the player clamp. No camera follow system yet: R-Town needs one (step 11). Found while testing: `game-kit` did not build alone, because `debug.rs` uses UI types and the workspace only worked through feature unification. The kit now declares `bevy` feature `ui_api`. Run `./dev-scripts/rust.sh test game-kit` and `web game-kit` for the kit alone.
10. **Verify Long Way.** Done. Lint, test (126 tests: 103 in the kit, 23 in Long Way), web, kit alone, a scripted walk (blocked at the tree base, slide at the edge) and a screenshot compared with the step 1 baseline by eye. The only visible difference is the tree outline, which is smaller on purpose (trees stop at the base line like rocks, commit `75f926f`). The rocks, trees, Guy, grid, labels and info box are the same. A pixel level comparison was not done.
11. **Wire R-Town.** Done (2026-11-02). R-Town uses the kit for the camera (240 px, `CameraFollowPlugin` with `CameraBounds` for the 80x60 map), collision (cell 16, slide values scaled), depth sort, debug tools, script runner (`R_TOWN_SCRIPT`) and the map parser. It has its own `camera.rs`, `collision.rs`, `map.rs`, `trees.rs` and `hero.rs`. The follow system and the debug size scaling (adr-x17) are new kit code written for this step. Art: `characters.aseprite` and `background-elements.aseprite`, catalogue in `games/r-town/docs/art-tags.md`. No source file was copied from Long Way.
12. **Update docs.** Update `dev-docs/extensions/*.md` "Code today" and the index. Update `games/long-way/docs/source-layout.md`. Add the new crate to `CLAUDE.md` and `dev-docs/README.md`. Update `adr-w6` status in `WORKSPACE.md`. Close roadmap item 2 and move it to Completed Items.
13. **Migrate and delete this plan.** Move lasting content to `docs/` and `dev-docs/`, add the lessons to `dev-docs/WORKFLOW.md`, then delete this file.

## Risks

- `collision.rs` has the most tests. Split it with the tests, not after.
- Bevy `Plugin` generics add complexity. Prefer plain fields and a marker component over type parameters.
- `bevy_aseprite_ultra` is used by `player.rs` only. Keep it out of the kit unless a shared animation helper is needed.
- A kit change can break both games. Run both games' `test` from step 4 onward.
