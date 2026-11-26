# Extensions

Our own reusable design decisions, built on top of Bevy. They are not part of Bevy. They are choices that other games in this workspace may want to reuse.

Shared code lives in the `game-kit` crate (`crates/game-kit/`, `adr-x12`). Each document here describes one extension: the general rules are here, the code is named in the index below, and the values and rules that belong to one game are in `games/<game>/docs/`.

## Where things are documented

| Question | Document |
|---|---|
| How does Bevy itself work? | `dev-docs/bevy/concepts.md`, `dev-docs/bevy/notes.md` |
| What is the extension and how do I use it? | This folder |
| Which numbers and rules does Long Way use? | `games/long-way/docs/` |
| Why was a decision made? | `dev-docs/adr/` |

## Symptom index

Read this before you write game code for movement, draw order, collision, maps, projectiles, particles or the HUD. The kit probably has the fix. A kit feature derives, requires or warns about what a spawn site needs (`adr-x56`). If you find yourself repeating a kit constant or rule by hand, the kit is missing something: say so.

| If you see | Use |
|---|---|
| A character is drawn behind an object it stands in front of, or the order changes with x | `SortAtBlocker` + `Blocker` (`depth.rs`, `adr-x53`) |
| Neighbouring objects in a row flicker or swap order | `TieBreakX` (`depth.rs`) |
| A character is not drawn behind a tree it walks behind | `DepthSort` with the real feet offset (`DepthSort` owns it, `adr-x57`) |
| A pickup or other non-blocking object hides a character standing on its base | `SortAtArea(ground rect)` (`depth.rs`) |
| A sprite is under the ground or above the HUD | `layers.rs` constants, never a bare z |
| A debug drawing covers the HUD | A sprite on `FEATURE_DEBUG_Z` or `MOVEMENT_DEBUG_Z`, not a gizmo |
| The map edge hides the character | `EDGE_GROUND_Z` or `EDGE_CANOPY_Z` (`adr-x51`) |
| The player walks through an object, or hits an invisible wall after a map change | `Blocker`, and a collision rebuild in `MapHooks.rebuild` |
| The player sticks on corners | `CollisionPlugin { allow_sliding }`, `SlideTuning`, `SlideAssist` (`collision.rs`) |
| Gaps or wrong blocked areas between neighbours | `groups.rs`, `group_or_alone` |
| A body overlaps the side of an object it stops at | `collision::grow_sideways` by the body half width |
| An enemy walks into walls | `Mover` + `NavGrid` (`movement.rs`, `nav.rs`) |
| An enemy should patrol, chase or guard | `strategies.rs` |
| Contact damage does not fire, or fires too often | `Touchable` + `HitBox`, `ControlledTouched` |
| A shot passes through trees or never leaves the map | `Projectile` + `CameraBounds` |
| A thrown shot sorts wrongly | `ProjectilePath` + `DepthSort` |
| A `ProjectileHit` handler cannot find the projectile | Order it after `ProjectileSet` (`adr-x55`) |
| A pickup is not collected, or is collected twice | `Collectable` + `MapId` |
| An object stays after its map unloads or F1 | Spawn it in the `load` hook (the kit adopts it into the `MapRoot`, `adr-x58`). Spawned later: `#[require(OnCurrentMap)]` for what the player causes, `SpawnedBy` for a spawner's entity, or copy the spawning object's `MapId` (`adr-x60`) |
| Enemies do not appear again | `Spawner`, `SpawnRequest`, `SpawnedBy` |
| Many sprites animate in step | `RandomStartFrame::for_cell` |
| The animation speed is wrong | Frame durations in the `.aseprite` file (`adr-a6`) |
| Pixels are blurred | The kit camera's view height (`camera.rs`) |
| The fade or entry position is wrong at an exit | `map_transition/transition.rs` and `MapHooks` |
| An exit leads nowhere | `map_links.rs` |
| A large blocked area (a forest) is tedious to draw cell by cell, or must change during play | `CellArea` and `fill=enclosed` (`cell_area.rs`, `adr-x62`) |
| Ground tiles change on reload | `map_background::place_tiles` |
| A HUD click moves the player | `BlocksWorldClick` (`pointer.rs`) |
| The HUD overlaps the game | Screen regions (`screen.rs`) |
| The camera must shake, or a whole-view effect fights the camera follow | `CameraShake` (`screen_fx.rs`, `adr-x61`) |
| The game runs while paused | `AppState` and `PlayState` (`app_state.rs`) |
| F1 does nothing | `ReloadPlugin` and a `ReloadMap` handler |
| Scripted runs differ | Kit `Rng` with a seed (`rng.rs`) |
| You need a repeatable bug | Script runner with `--debug`, `shot.sh` |
| Particles do not follow their owner | `Fx::following`, `Follows` (`fx.rs`) |

## Index

| Extension | What it gives | Code today | ADRs |
|---|---|---|---|
| [collision.md](collision.md) | Feet-point collision, merged outlines, forgiving sliding | `game-kit` `collision.rs`, `outline.rs` | x4, x10 (lw1, lw13 for Long Way) |
| [collision-research.md](collision-research.md) | Approaches compared, why outlines | none | lw1, x10 |
| [draw-order.md](draw-order.md) | Depth sort by feet, named layers | `game-kit` `depth.rs`, `layers.rs` | x2, x9 (lw12 for Long Way) |
| [map-files.md](map-files.md) | Text map format and parser contract, exits and entries | `game-kit` `map.rs`, `map_areas.rs`, `map_links.rs`, `map_edge.rs`, `map_background.rs` | lw8, x11, x18, x30 |
| [cell-areas.md](cell-areas.md) | Areas of cells drawn as outlines and filled, with blocked areas that follow changes during play | `game-kit` `cell_area.rs` | x62 |
| [map-transitions.md](map-transitions.md) | Moving between maps: registry, lifecycle, fade, seamless joins, entry placement | `game-kit` `map_transition/` (`MapLifecyclePlugin`, `MapTransitionPlugin`), `reload.rs` | x30, x31, x32, lw19 |
| [groups.md](groups.md) | Single items and neighbour groups for blocked areas | `game-kit` `groups.rs` | x20, rt9, lw14 |
| [projectiles.md](projectiles.md) | Straight-flying projectiles that stop at blocked areas and the map edge | `game-kit` `projectile.rs` | x21, rt11 |
| [nav.md](nav.md) | Grid path finding built from the collision outlines, nearest reachable point | `game-kit` `nav.rs` | x23, x43, rt12 |
| [area-effects.md](area-effects.md) | Ring, band and push geometry for area effects, temporary blocked areas | `game-kit` `area.rs`, `collision.rs` | none yet |
| [particles.md](particles.md) | Enoki particle effects: plugin, glow materials, one-shot and follow-the-owner emitters | `game-kit` `fx.rs`, `fx_strip.rs`, `vendor/bevy_enoki` | none yet |
| [pointer.md](pointer.md) | Mouse clicks as a world position message | `game-kit` `pointer.rs` | x42, lw30 |
| [enemy-movement.md](enemy-movement.md) | Goals, patrol, pursue and sentry strategies, touch messages, facing tags, hit boxes, seeded random numbers | `game-kit` `movement.rs`, `strategies.rs`, `touch.rs`, `hitbox.rs`, `rng.rs`, `movement_debug.rs` | x24, x26, x27, x40, x41, x43, rt12 |
| [spawners.md](spawners.md) | Spawners with settings in the map legend | `game-kit` `spawner.rs`, `map.rs` | x25, rt12 |
| [debug-tools.md](debug-tools.md) | Grid, info box, collision overlay | `game-kit` `debug.rs`, `collision_debug.rs` | x3, x6 |
| [collectables.md](collectables.md) | Items the player collects by touch, `Collected` message | `game-kit` `collectable.rs` | x38, lw27 |
| [dev-scripts.md](dev-scripts.md) | Zoomed screenshots and Aseprite checks (`shot.sh`, `aseprite.sh`) | `dev-scripts/` | |
| [dev-feedback.md](dev-feedback.md) | Plain function tests, logging, scripted input, screenshots | `game-kit` `script.rs`, `dev-scripts/rust.sh` | x5 |
| [plugin-configuration.md](plugin-configuration.md) | Plugin fields stored as a resource | `game-kit` `collision.rs`, `debug.rs` | none |
| [screen-layout.md](screen-layout.md) | Screen regions, HUD bar, app states, pause | `game-kit` `screen.rs`, `hud.rs`, `app_state.rs`, `camera.rs` | x34, x35, x45, lw24, lw34 |
| [screen-fx.md](screen-fx.md) | Effects on the whole view: camera shake | `game-kit` `screen_fx.rs` | x61 |
| [pixel-art.md](pixel-art.md) | View height scale, animation tag switching, animation speed, random start frame | `game-kit` `camera.rs`, `animation.rs` | e7, a6, x39 |
| [tile-sheet-catalogue.md](tile-sheet-catalogue.md) | Make a bought tile sheet readable (rulers and grid, 1:1) and cut sprites by position | `dev-scripts/aseprite/catalogue.lua` | a7 |

## Entry template

Every extension document uses these headings, in this order. Leave out a heading only when it has nothing to say.

1. **Summary.** What it is, in a few lines.
2. **Status.** Where the code is today and whether it is ready to share.
3. **Terms.** Words used in the rules.
4. **Parameters the game supplies.** What a game must provide, with the Long Way value.
5. **Specification.** Numbered rules, grouped by topic.
6. **Usage.** How to add and use it.
7. **Limits.** Known gaps.
8. **Tests.** Where they are.

## Rules for writing an extension document

- State the general rule here. Put game values (sizes, speeds, art pixels) in the game's docs and name the parameter here.
- Put history and alternatives in an ADR and link it. Do not repeat the reasoning.
- Plain explanations of Bevy features belong in `dev-docs/bevy/concepts.md`.
- Do not refer to plans or plan steps. Plans are deleted when done.
- Add the extension to the index above.
