# PLAN: port Jet from Pyxel to Bevy

Port the Pyxel game Jet into a new workspace game, `games/jet/`. This is not a 1:1 port: the port also improves the game (larger screen, richer art, path finding, particles), and uses the kit patterns from the start.

Source brief: `~/Dark/pyxel/games/jet/BRIEF-port.md` (outside this repo). Step 0 copies its rules into `games/jet/docs/SPEC.md`, so the game docs do not depend on the Pyxel repo.

## Status (2026-11-26): resume here

Step 0 done (2026-11-26): the crate, `SPEC.md`, `adr.md` (`adr-jt1` to `adr-jt12`), `adr-x61` for `screen_fx`, and the game tables. `ScriptPlugin` is already added. Step 1 built (2026-11-26): `assets/art/` has `palette`, `player`, `spinner`, `house`, `trees`, `shockwave.aseprite` (`art-tags.md`, `adr-jt13`), waiting for the user's check in Aseprite. Next: step 2.

## Designer's intent (from the brief)

> Jet is a simple tower-defence game. However, rather than the tower defending itself, the player character moves around the tower killing enemies that attack the tower.
> - the player character is extremely fast and agile
> - the enemies swarm in on the tower, at first only a few but they increase in number

## Decisions made (2026-11-26)

Record each one as an ADR in `games/jet/docs/adr.md` (prefix `jt`) in step 0.

| Topic | Decision |
|---|---|
| Crate | `jet`, in `games/jet/`. |
| Not a 1:1 port | Improve the game while porting. Keep the player's speed and agility exactly. |
| Screen | 16:9. Window 960 × 540 art pixels (2× on a 1080p screen). One fixed screen, no scrolling. |
| Cell | 16 px (the 8 px Pyxel tiles at 2×). |
| Layout (Q2, 2026-11-26) | A HUD bar of 28 px at the top. Below it the map, 60 × 32 cells (960 × 512 px), at the bottom of the window. The play field is the middle 40 × 32 cells (640 × 512 px), with the house exactly in the centre. The 10 columns on each side are forest (160 px each side). |
| Field edge (2026-11-26) | The side forest is solid trees. The camera shakes when the player hits the field edge: the map edge at the top and bottom, and the side forest at the left and right. Trees inside the field do not shake the camera, as in the original. |
| Screen effects in the kit (Q10, 2026-11-26) | A new kit concept `screen_fx` holds effects on the whole view. Camera shake is the first. The kit has no game values: Jet supplies the time, the distance and the direction. Later effects (for example a flash) go there too. |
| Field size (2026-11-26) | 40 × 32 was chosen so enemies take about as long to reach the house as in the original: 18 cells from the sides (4.8 s), 14 cells from the top and the bottom (3.7 s). A larger field makes the swarm slow to form. |
| Speeds | Same tiles per second as the real Pyxel speeds: player 30 cells/s (480 px/s), Spinner 3.75 cells/s (60 px/s). Then tune by play. |
| Art | The `.aseprite` files are the source. Copy them, resize them 2×, and load them directly. No exported sprite sheets. Not marked `_placeholder`. Files and tags are checked and standardised with the user (step 1). |
| Palette | A new Jet palette (Q1, 2026-11-26). Sprites stay white line art on a black background. The palette starts with the colours the game recolours to: white, black, and the feedback colours of the brief (grey for dying, magenta-red for the kill X and damage level 2, pink for damage level 1, purple for damage level 3, teal-green for the heal charge). More colours are agreed when particles and other effects are added. |
| Map | A text map file in the kit format (`map-files.md`), like Long Way. A dynamic forest comes later, and then the map stops being only a file. |
| Enemies | Real path finding, so Spinners go around trees. Spreading (not stacking) is wanted. A flow field is explored during the port. |
| Game over | On the 4th hit the house is destroyed: play stops, a screen shows the survival time, and a key restarts. |
| HUD | Survival time and kill count. No house HP readout (the house colour shows the damage). |
| Title screen | None in the port. |
| Kit patterns | The player and the enemies use the kit patterns correctly from the start (`Mover`, strategies, `HitBox`, `Rng`, `layers.rs`, `fx`, `AppState`, `ScriptPlugin`, debug tools). |
| Future features | Stub the extension points now: more enemy kinds, pickups that spawn while playing (for example "kill all enemies on screen"), power-ups (for example a larger shockwave), a forest that changes over time. |

From the brief's section 12, taken as decided:

| # | Issue | Decision |
|---|---|---|
| 1 | No game-over | Game-over screen and restart (above). |
| 2, 3 | No path finding, Spinners stuck on trees | Path finding (above). A blocked Spinner finds a new path. It never freezes. |
| 4 | No HUD | Time and kills (above). |
| 5 | A dying enemy can be killed again | An enemy is marked dead at once and skipped by every hit check. |
| 6 | Speeds set by rounding | Use the real speeds in cells per second (above). |
| 7 | Diagonal enemies are faster | The kit `Mover` gives the same speed in every direction. |

## Rules kept from the original

These define the feel. Keep them unless the user changes them:

- Four directions only, no diagonal. Full speed at once, full stop at once.
- Key rules: a press starts or switches direction, one fallback direction is remembered, releasing the current key falls back or stops (brief section 4).
- Running into a wall stops the player and clears the held direction. A key held from before does not start movement.
- The moving player is the weapon. The shockwave sprite in front of the player shows the attack zone. A player standing still kills nothing.
- Enemies cannot hurt the player. Player and enemies pass through each other.
- A kill gives a heal charge for 2 s (teal tint). Running into the house while charged heals one damage level, if the house is damaged.
- The difficulty curve of brief section 9 (group size, spawn gap range, level-up every 10 s), in game time, so pause stops it.

## Open questions (ask at the step named)

| # | Question | Step |
|---|---|---|
| Q3 | How trees go in the map file: one character per tree (2 × 2 footprint, `T` solid and `t` passable), or one character per blocked cell that the game fills with tree sprites (the user's idea of an outline filled at random)? | 2 |
| Q4 | Collision feel: the original tests only the centre line, so the player slides past tree corners. The kit tests a feet point with `allow_sliding`. Which point and which sliding setting feels right? | 3 |
| Q5 | Attack zone: a hit box 3 cells wide and 1 cell deep in front of the player, the size of the shockwave. Tune by play. Should a power-up change it by one number (for example a scale)? | 5 |
| Q6 | When does a Spinner count as hitting the house: when it reaches its target point next to the house, or when it touches the house's blocked area? | 4 |
| Q7 | Spawning: the kit `Spawner` has a fixed interval and one entity per request. Jet needs random edge positions, a random gap and groups. Extend the kit spawner, or keep a Jet "director" in the game? | 4 |
| Q8 | Spreading and the flow field: add them to the kit (a kit change, so ask), or keep them in Jet first? | 7 |
| Q9 | Colour feedback: a sprite tint works while the art is white line art. With coloured art a tint changes every colour. Use a tint now, or a palette swap shader? | 5 |
| Q11 | Pause key (`Escape`, the kit default) and the restart key on the game-over screen. | 6 |

## Facts found while writing this plan

- The Jet source files are `sprites.aseprite` (8 × 16, 9 frames: the player row of the sheet above the trail row, so one frame holds two sprites), `house.aseprite` (32 × 32, 4 frames), `forest.aseprite` (16 × 16, 8 frames, one tree per frame), `shockwave.aseprite` (24 × 8, 4 frames) and `sheet1.aseprite` (256 × 256, 1 frame). Every file has a visible `Background` layer. The only tags are `Loop` tags that do not match the brief, and every frame is 100 ms (the brief says 16 fps, 4 fps for the house). So step 1 must split `sprites.aseprite`, remove or hide the backgrounds, and set tags and durations.
- `collision::move_feet` returns only the new feet position. The game finds "blocked" by comparing it with the wanted position, and finds what blocked it (house, tree, screen edge) itself.
- The kit's bar layout fits the HUD: `ScreenMode::BarTop { ratio }` with ratio 28 / 512 = 0.0546875 gives a game region of 960 × 512 and a 28 px bar. With `VIEW_HEIGHT` 512, the game is a whole-number 2× on a 1080p screen. Long Way uses `BarBottom`, so `BarTop` is computed and tested, but no game uses it yet.
- The kit `Spawner` supports a fixed interval only (`spawners.md`, Limits).
- `Mover` runs at most 4 path searches per frame (`enemy-movement.md`). A large swarm queues its searches.

## Steps

Each step ends with `./dev-scripts/rust.sh lint`, `test` for each changed crate, a scripted run with `--debug` and screenshots where it shows something, the user's play test, and a commit only when the user says so. Run `./dev-scripts/rust.sh size` at the start and after the steps that rebuild a lot.

### 0. Scaffold and docs

- Follow `dev-docs/WORKFLOW.md` steps 1, 5, 6 and 10. Create `games/jet/` with `Cargo.toml` (`bevy`, `game-kit`, `bevy_aseprite_ultra`, all `.workspace = true`), `src/main.rs`, `input/` and `docs/`.
- `docs/SPEC.md` (template in `WORKFLOW.md`): the rules of the brief, converted to 16 px cells and the decisions above. Keep section 12 of the brief as the open-question list.
- `docs/adr.md`: one ADR per decision above. Add the prefix `jt` to `dev-docs/adr/README.md`.
- Add Jet to the games table in `CLAUDE.md`, and a run line to `docs/README.md`.
- Check: `lint`, `build jet`, and `run jet` opens an empty window.

### 1. Art: copy, check and standardise

- Copy `sprites`, `house`, `forest` and `shockwave.aseprite` from `~/Dark/pyxel/games/jet/assets/` into `games/jet/input/`. Ask whether `sheet1.aseprite` is needed.
- Go through each file with the user (`aseprite.sh check`, `frame_grid`). Agree on:
  - files: split `sprites.aseprite` into `player.aseprite` (tags `Idle`, `Move`, `Trail`) and `spinner.aseprite` (tags `Move`, `Die`, `Kill`), or keep one file with clear tags;
  - tags for the house (`Smoke`), trees (one tag or frame per variant) and shockwave (`Wave`);
  - durations: 16 fps is 62.5 ms, so 62 or 63 ms per frame; the house is 250 ms;
  - the background layers: remove them, or keep them hidden.
- Make the palette file `assets/art/palette.aseprite` (`new_sprite` needs an existing palette file, so this needs a small script or the user in Aseprite; the colours of the palette decision; ask whether to keep the brief's exact hex values or adjust them). Then move each file onto it with `remap_colors` and `set_palette`.
- Resize 2× with `dev-scripts/aseprite/resize.lua`, and save into `games/jet/assets/art/`.
- Write `docs/art-tags.md`: every file, frame and tag, as in R-Town.
- The user checks the files in Aseprite before step 2.

### 2. Screen, map and static world

- `camera.rs`: `VIEW_HEIGHT` 512, `CELL_SIZE` 16, colours. Window 960 × 540. `PixelCameraPlugin` with no follow (fixed screen), `ScreenPlugin { mode: BarTop { ratio: 28.0 / 512.0 } }` (an empty bar for now), `CameraBounds` set to the map. Check that the bar is 28 px and the game region 960 × 512 at scale factors 1 and 2.
- Ask Q3. Then write `assets/maps/1.txt` (60 × 32, `origin: top-left`, legend after `adr-x37`: `@` the player start, `H` the house, tree letters as agreed). The middle 40 columns are the field, drafted from the brief's layout (section 3) fitted to 40 × 32 with the house in the centre. The 10 columns on each side are forest. The user edits it.
- The field has a hard edge: the map edge at the top and bottom, and the side forest at the left and right. The side forest is solid trees (2026-11-26), so it blocks like any other tree.
- `map.rs` (kinds, loading), `house.rs` and `trees.rs`: spawn with `Blocker` rects and the right layers. The house's top row is passable, as in the original. Trees keep the random variant (the 5th variant twice as likely) and the small random offset, from the kit `Rng` with a seed per cell, so F1 gives the same forest.
- Kit plugins: `CollisionPlugin`, `CollisionDebugPlugin`, `DebugPlugin` (F2, F3), `DepthPlugin`, `ReloadPlugin` (F1), `ScriptPlugin { env_prefix: "JET" }`, `AnimationHelpPlugin` (random start frames for the house smoke if wanted).
- Check: a screenshot with `--debug` shows the map, the grid and the collision overlay. Compare the blocked cells with the drafted map.

### 3. Player

- `input.rs`: the key rules of brief section 4 as a plain function (pressed, released and held keys in, direction and fallback out). Unit tests for every rule: start, switch, fall back, stop, the fallback dropped, a held key that does not start movement after a wall.
- `player.rs`: `Controlled`, `DepthSort`, `SlideAssist`, a `HitBox`. Move at 480 px/s with `move_feet`. Ask Q4 and tune. Blocked means: stop, clear the held direction, and report what blocked (`PlayerBlocked { by: Edge | Tree | House }` as a message).
- At 480 px/s the player moves 8 px a frame at 60 fps, more on a slow frame. Check that a long frame cannot carry the player through a thin tree band. Test it in the kit if the kit is the cause.
- Animation: `Move` plays once, rotated to the direction, then holds the last frame. `Idle` is not rotated. The trail (one cell behind) and the shockwave (one cell ahead, 3 cells wide) are children, so they move and rotate with the player, and they are removed on stop.
- Field-edge shake with a new kit module `crates/game-kit/src/screen_fx.rs` (Q10):
  - The game asks for a shake with a message (duration, distance, direction). The kit moves the `GameCamera` by an offset, flipping sides each frame, and puts it back exactly when the shake ends.
  - Its order with `CameraFollowPlugin` and the whole-pixel rounding (`adr-x17`) must be right, so a scrolling game can use it too. The offset is in whole art pixels. It uses game time, so it stops while paused.
  - Kit tests: the offset per frame, the flip, the exact end position, a new shake while one runs, a pause. Prove each test can fail (`dev-docs/testing.md`).
  - An extension doc `dev-docs/extensions/screen-fx.md` (added to the index and the symptom index), an ADR in `dev-docs/adr/EXTENSIONS-SCREEN.md`, and a row in the module table of `crates/game-kit/src/lib.rs`.
  - Jet values: 0.1 s, 2 px along the direction of travel. It happens at the top and bottom map edges and at the side forest, so `PlayerBlocked` must tell a side forest tree from a field tree (for example a `SideForest` marker, or the field rectangle).
- Check: scripted runs for each direction, a switch, a fallback, a wall stop at a tree, at the house and at the screen edge.

### 4. Spinners and spawning

- `enemy.rs`: an `EnemyKind` enum (only `Spinner` now) with a values table (speed, hit box, art file), so a new kind is a new row and its own art.
- Spinners use the kit `Mover` (60 px/s), `NavPlugin`, `MovementPlugin` and a Jet strategy `RaidHouse` in `StrategySet`: it picks one of the target points around the house at random, and then goes for the house (ask Q6). An enemy whose search fails searches again later. It never freezes.
- States: moving (`Move` loop), dying (`Die` once, then despawn), hitting the house (`Kill` once, the house takes 1 damage, then despawn). Mark the enemy `Dead` at once, so no hit check sees it again (brief #5). Follow the `FacingSet` order rule of `enemy-movement.md` rule 7b.
- Ask Q7. Then build spawning: each Spinner starts at a random cell on a random edge of the field (top row, bottom row, or the first or last field column next to the side forest; only on free ground), with the difficulty curve of brief section 9 as a plain struct with unit tests (level-ups, limits, group size rounding, the first spawn one gap after the start).
- Check: a scripted run of 90 s with `JET_SEED` set. Spinners come around trees, the spawn rate rises, and the movement debug (F3) shows the paths.

### 5. Combat, house and particles

- Attack zone (Q5): a `HitBox` ahead of the moving player, taken from a `PlayerStats` resource, so a power-up can change it later. Every live enemy in it dies. Test the zone for all four directions, a still player (no kill), and a dead enemy (skipped).
- `house.rs`: damage level 0 to 3, then destroyed (`HouseDestroyed` message). Colour per level (Q9). A pulse of 1.0 → 1.2 → 1.0 over 0.1 s about the centre.
- Heal charge: a kill sets it to 2 s, and each kill resets the time. The player is tinted while charged. Blocked by the house while charged and the house is damaged: heal one level, use the charge, pulse. Not damaged: keep the charge. Unit tests for these rules.
- Particles with `game_kit::fx` (`particles.md`): a blood burst on a kill (it replaces the Pyxel pixel splatter), a burst at the house on damage, a heal effect on the house and on the player. Effect files in `assets/effects/`, particle sprites from `.aseprite` tags through `FxLooks`. The user tunes the look.
- Check: scripted runs of a kill, a house hit, a heal, and a heal with no damage.

### 6. HUD, pause and game over

- `AppFlowPlugin` starts in `Playing` (no title). Pause with the kit pause key (Q11). The timers use game time, so pause stops spawning and level-ups.
- HUD: survival time and kill count in the 28 px top bar (children of the kit `Hud` root, kit `label`).
- `HouseDestroyed` → `AppState::GameOver`. A `state_root` screen shows "house destroyed" and the survival time. A key restarts: a full reset (enemies, house, timers, difficulty, player, HUD counts). Reuse the F1 reset path where it fits.
- Check: a scripted run with a fast difficulty (a test seed or a script command) that reaches the game-over screen and restarts.

### 7. Swarm: flow field and spreading

- Measure first: frame time and path search queue with 50 and 200 Spinners on the kit `Mover`.
- Explore a flow field: one search out from the house (or one per target point) on the nav grid, rebuilt when the collision world changes. Each Spinner reads its next step. Compare it with the `Mover` on cost and look.
- Spreading: a light separation push between Spinners, so they do not stack. Check it with screenshots of a large swarm.
- Ask Q8 before any kit change. If it goes into the kit: an extension doc, an ADR, tests in `game-kit`.

### 8. Extension points (stubs)

Pencil in the shape of each future feature, with no gameplay yet. Each stub compiles, is documented in the SPEC, and has a roadmap item.

- Enemies: the `EnemyKind` table of step 4.
- Pickups: a `PickupKind` enum (for example `ClearScreen`) with the kit `Collectable`, and a pickup director that does not spawn anything yet.
- Power-ups: `PlayerStats` (speed, attack zone size) as the one place that power-ups change. A `PowerUp` enum with no variants built.
- Dynamic forest: a `ForestChanged` path. Spawning or removing a tree at run time rebuilds the collision world and the nav grid. Prove it with one script command (remove a tree, a Spinner then walks through the gap).

### 9. Final check

- `./dev-scripts/rust.sh lint`, `test` (all crates), `build` (all games, so Long Way and R-Town still build).
- A full scripted run: start, kills, a heal, level-ups, game over, restart. Screenshots with `--debug`. Say what was not seen on screen (motion, feel). Delete `target/screenshots/` files after.
- `./dev-scripts/rust.sh size`.

### 10. Migrate and delete this plan

- `games/jet/docs/`: SPEC (current state, open questions left), `adr.md`, `art-tags.md`, a README with the source layout and the run commands.
- `docs/README.md`: the run command and any troubleshooting found.
- `dev-docs/`: kit changes in their extension docs and `adr/EXTENSIONS*.md`, new Bevy patterns in `bevy/notes.md`, lessons in `WORKFLOW.md`.
- `dev-docs/ROADMAP.md`: one item each (next free IDs) for more enemies, pickups, power-ups and the dynamic forest.
- Delete this plan.
