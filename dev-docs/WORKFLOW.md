# Workflow: starting a new game

Repeatable steps for the start of a game. This file is built from real use. The first game to use it is R-Town.

## Steps

1. **Create the game folder.** `games/<name>/` in kebab-case, with `input/` and `docs/`. Add a minimal `Cargo.toml` and `src/main.rs` at once: `members = ["games/*"]` makes every cargo command fail for a folder without a manifest. `games/*/input/` is gitignored (root `.gitignore`), so raw packs never reach git.
2. **Drop material in `input/`.** Art, audio, references. The game never reads `input/`. Used files are copied into `assets/` on request (adr-w5).
3. **Review `input/`.** List the files. View the key sheets. Read the license text. Report sizes, tile grids, animation frames and gaps. Check for facing sprites and walk frames before choosing a player.
4. **Interview.** Rounds of up to 4 questions on one topic each. Offer a recommended option first. "Defer" and "decide later" are valid answers: record them as open questions and do not invent the design. If the developer does not understand a question, stop and explain it with examples, then ask again.
5. **Write the spec as you go.** `games/<name>/docs/SPEC.md`. Update it after every round, so it is always current. Template below.
6. **Record each decision.** One ADR per decision in `games/<name>/docs/adr.md` (prefix in `dev-docs/adr/README.md`). Say why, not how it is coded.
7. **Check the license.** Ask early whether the repo and any web build are public. Record the result in an ADR.
8. **Set the first playable.** Pick the smallest slice that proves the base: player, map, collision, camera. A test map comes before the real level.
9. **List reusable code.** Compare the spec with existing games. Write or update a plan for shared code (`dev-docs/PLAN-*.md`). Ask the developer about each design choice one at a time. Record the answers in `dev-docs/adr/EXTENSIONS.md`.
10. **Scaffold the crate.** Use the shared kit `crates/game-kit` (section below). Run `./dev-scripts/rust.sh lint`, `test` and `web`.
11. **Write the build plan.** `dev-docs/PLAN-<name>-<topic>.md`, small runnable steps. The last step migrates lasting content to docs and deletes the plan.
12. **Build the first playable.** Follow the build plan. Run the game with `./dev-scripts/rust.sh script <game> "..."` before reporting a behaviour as working.

## Wiring a game to the kit

What a new game provides. R-Town (`games/r-town/src/`) is the example.

1. **Dependencies.** `bevy`, `game-kit` and, for Aseprite art, `bevy_aseprite_ultra`, all with `.workspace = true`.
2. **Plugins.** `PixelCameraPlugin { view_height, clear_color }`, `CameraFollowPlugin`, `CollisionPlugin { allow_sliding, cell_size }`, `CollisionDebugPlugin::default()`, `DebugPlugin::new(cell_size, grid_color, grid_origin)` (set `target_name`), `DepthPlugin::default()` and `ScriptPlugin { env_prefix }` (the game name in capitals with underscores, for example `R_TOWN`).
3. **The player entity.** `Controlled`, `DepthSort`, `SlideAssist` and a `Transform`. Move it with `move_feet` and the `CollisionWorld` edges.
4. **Obstacles.** A `Blocker(Rect)` for each blocking object, grown sideways by the actor's half width, and a `DepthSort`.
5. **Maps.** `game_kit::map::parse(file, source, kinds)`. The game keeps its own `Kind` enum. When the map is loaded, insert `CameraBounds(Rect)`.
6. **Game files.** `camera.rs` (view height, cell size, colours), `collision.rs` (actor sizes), `map.rs` (kinds, loading), one file per object type and one for the player.
7. **Check.** Test the kit alone (`./dev-scripts/rust.sh test game-kit`), then run a scripted walk and a screenshot with the display unlocked.

## Spec template

`games/<name>/docs/SPEC.md` has these headings, in order:

1. Summary
2. Targets and timing model (include distribution)
3. Core loop
4. Player
5. World and maps
6. Interaction
7. Combat
8. Progression and save
9. UI
10. Art (source packs, cell size, view height, animation method)
11. Audio
12. Scope (first playable, later, out of scope)
13. Open questions

## Interview checklist

Topics covered for R-Town, in the order that worked:
1. Timing model and core loop
2. License and distribution
3. Art source handling
4. Player: how many characters, which one, how animated
5. Cell size, view height, map size, camera
6. Speed, collision rules, feet box
7. First playable scope and test map
8. Deferred: combat, world structure, progression, interaction, audio, UI

## Lessons

- `games/*/input/` was not gitignored when R-Town started. Step 1 now relies on one rule for all games.
- Ask the distribution question in the first round. It decides whether third-party assets may be committed.
- Third-party packs ship PNG, not `.aseprite`. Decide early how they enter the art pipeline.
- A pack may have one static pose per character. Check this before the animation question.
- Check scale maths before recording it. 240 px height does not scale by a whole number on 1080p (4.5x). Long Way also uses a fractional scale, so this was accepted.
- Explain a question with concrete examples when asked. Abstract trade-offs ("generic parser") did not work.
- Build and test each shared crate on its own (`./dev-scripts/rust.sh test game-kit`). A workspace build unifies Bevy features, so a crate can compile with a missing feature until it is built alone.
- A scripted screenshot can come out completely black when the display is asleep or locked, even though the log looks fine. Check one screenshot of a known good build before trusting a visual check.
- Debug and overlay sizes that are fixed in world units look wrong at another art scale. Scale them by the cell size from the start.
- Write the kit first only when a second game exists (adr-w6). R-Town's plan moved the code of Long Way in 13 small steps, each with lint, tests, the web check and a scripted run, and each committed on its own. That kept Long Way working throughout.
- Game values from the first game (such as `PLAYER_*`) are in that game's art units. A new game needs its own values.
