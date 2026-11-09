# PLAN: the ghost enemy and player HP in Long Way

Status: planned, all open points settled, not started. Written 2026-11-09 from the review of the R-Town enemy code.

## Goal

Add the ghost (`games/long-way/assets/art/characters/ghost.aseprite`) as the first enemy of Long Way, and give the player 3 HP.

## Decisions (from the user, 2026-11-09)

- Enemies are placed from map markers **when a map is loaded**, once. No timer, no respawn while the map stays loaded. When the player leaves and comes back the map loads again and the enemies are placed again. No state is saved.
- The ghost patrols a small area. When it sees the player (clear line, inside a range) it pursues. It **never gives up**.
- That is one composite kit strategy: `Sentry` (approved).
- The player has 3 HP, shown in the HUD. The user supplies the HP icon.
- When a ghost touches the player: the player loses 1 HP, then the ghost dies. The user draws `Die` frames in `ghost.aseprite`.
- Ghosts get killable by weapons later (a future enhancement). Not part of this plan.

## What already exists in `game-kit`

Spawner timer, `NavGrid`, `Mover` and `MoveGoal`, `RandomPatrol`, `Pursue`, `HitBox` and `overlaps`, `Rng`, `MovementDebugPlugin`, the Aseprite tag helpers. See `dev-docs/extensions/enemy-movement.md`. Long Way does not use the nav and movement plugins yet.

## Settled points (user, 2026-11-09)

1. **HP at 0.** Long Way has no game over screen. Log it, keep the player alive and stop taking damage at 0 until a game over screen exists (roadmap item 13).
2. **Damage rules.** No invulnerability time, because the ghost dies on the touch, so one ghost can never hit twice. Two ghosts touching in the same frame each take 1 HP.
3. **Ghost values.** The ghost is a **slow** enemy. Take the speed as a small share of the player's speed (start at 0.3 x, as R-Town does with shares) and tune it in the scripted run. Measure the hit box from the art. Tune patrol radius and sight range in the scripted run.
4. **Sight and blockers.** Everything in `CollisionWorld` blocks sight (rocks, trees, blocked areas), the same as it blocks walking.
5. **Alert cue.** None. The `Attack` tag stays unused for now.
6. **Die frames.** Added to `ghost.aseprite` (checked on 2026-11-09, see "Art facts").

## Art facts (`./dev-scripts/aseprite.sh check`, 2026-11-09)

- Canvas 60x60, 22 frames, one layer `Body`.
- Tags: `Down` 1-4, `Left` 5-8, `Up` 9-12 (4 frames each, 200 ms), `Attack` 13-14 (200 ms, unused), `Die` 15-22 (8 frames, 100 ms, forward).
- Body about 27x43 px. The feet are not at the bottom of the canvas: the lowest visible row is y 49 to 53 in the walk frames, so the sprite needs an offset to stand on the feet point. During `Die` the body shrinks from 43 to 35 px high and sinks (feet y 52 down to 37).
- `Die` must play once and then the entity is removed. It must not loop.

## Steps

### 1. Kit: sight rule and `Sentry` strategy

- `strategies.rs`: pure function `can_see(from, to, range, edges) -> bool` (range check, then `nav::segment_blocked`).
- `Sentry { radius, sight, alerted }`: patrols like `RandomPatrol` (reuse `pick_target`) until `can_see` is true for the `Controlled` entity. Then `alerted` stays true and it sets the goal like `Pursue`. It stands still when no `Controlled` entity exists after the alert.
- `Sentry::from_params`: `radius` and `sight`, both above 0, errors that name the parameter (same style as `RandomPatrol::from_params`).
- Register the system in `StrategySet`. Needs `CollisionWorld` for the edges.
- Tests in the kit: a hidden player behind a wall does not alert, a visible player in range alerts, outside the range does not, it never goes back to patrol, parameters. Headless app test: a sentry walks around, then reaches the player.
- Docs: update `dev-docs/extensions/enemy-movement.md` (strategy list, the "no sight rule" limit). ADR in `dev-docs/adr/EXTENSIONS.md` (next free `adr-x*`): why one composite strategy, why a one-way latch.

### 2. Kit: enemies from map markers

- A helper that takes the markers of the enemy kinds out of the parsed map objects and hands `(cell, kind, Params)` to the game, so the game builds the entity and tags it with the `MapId` (cleanup on map unload is already in `map_transition.rs`).
- One shared `strategy_from_params` for `patrol`, `pursue` and `sentry`, with the error format `file:line: spawner ...` that R-Town uses.
- Ask the user before step 2 if the shape of the helper is unclear when the code is read: R-Town's spawners are timer based and its map reload differs from Long Way's `MapHooks`. R-Town keeps its own spawners unless the user says otherwise.
- Tests in the kit. Legend entry example: `spawn_ghost strategy=sentry radius=40 sight=90`. The legend character follows the convention in `dev-docs/extensions/map-files.md` (enemies are special characters).

### 3. Kit: enemy touch message

- A system that sends `PlayerTouched { enemy }` when the `HitBox` of the `Controlled` entity overlaps the `HitBox` of an entity marked as touchable. The game decides what the touch does. Tests in the kit.

### 4. Kit: facing to animation tag

- A helper that maps `MoveState.facing` to a tag name and a flip flag, for characters with `Down`, `Left` and `Up` tags where `Right` is `Left` flipped (`adr-a6` style, frame durations stay in the `.aseprite` file). Used with `play_if_changed`. Tests in the kit.

### 5. Long Way: player HP

- `Hp { current, max }` component on the player (3 and 3 at start), with a `PlayerHit` or damage message so other systems do not call into `player.rs`.
- HUD: a horizontal row of icons from `art/ui/hp.aseprite` (32x32, taller than the 24 px energy icons, so the row gets its own vertical position; tags `Full` frame 1 and `Empty` frame 2, one layer `Icon`; the user draws it). One icon per point of **max HP**. A point the player has lost shows the `Empty` frame, the others show `Full`. The row sits in the HUD bar next to the energy row (`hud.rs`), spawned with the same Aseprite tag and `StaticFrame` helpers, and updated when `Hp` changes. The row follows the max, so a later change of max HP (for example a pickup) needs no HUD change. The icon draws at 1 art pixel per HUD pixel.
- At 0 HP: log it and keep the player alive. Damage stops at 0 until a game over screen exists.
- Tests only for rules that are not in the kit (HP cannot go below 0).
- ADR in `games/long-way/docs/adr.md` (next free `adr-lw*`) for HP and the touch rule.

### 6. Long Way: the ghost

- Add `NavPlugin`, `MovementPlugin`, `StrategiesPlugin`, `RngPlugin` and `MovementDebugPlugin` to `main.rs`. Check that `CameraBounds` is set for each map, and that the nav grid works with `MAP_ORIGIN` and rebuilds on a map change.
- `ghost.rs`: kind values (speed, hit box, canvas), the entity built from the marker (`MapId` tag, `Mover`, `HitBox`, `DepthSort`, the strategy component, the Aseprite sprite with `Down`, `Left` and `Up` tags), animation from `MoveState` through the step 4 helper.
- On touch: send the damage message (1 HP), stop the ghost (remove its strategy and goal, and its touch marker so it cannot hit again), play the `Die` tag once, and despawn the entity when the animation ends. Check how `bevy_aseprite_ultra` reports the end of a non-looping tag (look at its source and examples, then record the pattern in `dev-docs/bevy/notes.md`).
- Anchor the sprite so the feet sit on the entity's feet point (the canvas has about 7 px of empty space below the feet). Measure the hit box from the body (27 px wide, 43 px high), then shrink it to the part that should hurt.
- Add a `spawn_ghost` entry to a Long Way map legend for testing.
- Document the object kind and values in `games/long-way/docs/` (source layout and object kinds). Update `art-tags.md` and `art-pipeline.md` to say the ghost is used in the game.

### 7. Verify in the running game

- Reproduce with `./dev-scripts/rust.sh script long-way "..." --debug` (patrol, spot, chase, touch, HP going from 3 to 2). Take screenshots and delete them afterwards (`target/screenshots/`).
- Write a failing test for any cause found, fix it, run the script again.
- Say which parts were not seen on screen.
- Run `./dev-scripts/rust.sh lint`, `test game-kit` and `web` (the new plugins must work on `wasm32`; check `RngPlugin` and the time source).

### 8. Migrate and clean up

- Move user-facing content (the legend entry for `spawn_ghost`, HP in the HUD) to `docs/` and `games/long-way/docs/`.
- Move technical context to `dev-docs/` (`extensions/enemy-movement.md`, `extensions/map-files.md`, `extensions/README.md`).
- Update `dev-docs/bevy/notes.md` with any verified Bevy patterns. Add a section to `dev-docs/bevy/concepts.md` for any concept used for the first time.
- Move the roadmap items that this finishes to **Completed Items** and renumber.
- Delete this plan file.
- Commit only after the user confirms. Do not push.

Suggested commits (each one asks first): kit steps 1 to 4, then HP (step 5), then the ghost (steps 6 and 7), then the docs (step 8).
