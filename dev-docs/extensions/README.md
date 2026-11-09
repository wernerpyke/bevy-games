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

## Index

| Extension | What it gives | Code today | ADRs |
|---|---|---|---|
| [collision.md](collision.md) | Feet-point collision, merged outlines, forgiving sliding | `game-kit` `collision.rs`, `outline.rs` | lw4, lw10 (lw1, lw13 for Long Way) |
| [collision-research.md](collision-research.md) | Approaches compared, why outlines | none | lw1, lw10 |
| [draw-order.md](draw-order.md) | Depth sort by feet, named layers | `game-kit` `depth.rs`, `layers.rs` | lw2, lw9 (lw12 for Long Way) |
| [map-files.md](map-files.md) | Text map format and parser contract, exits and entries | `game-kit` `map.rs`, `map_areas.rs`, `map_links.rs` | lw8, lw11, x18, x30 |
| [map-transitions.md](map-transitions.md) | Moving between maps: registry, lifecycle, fade, seamless joins, entry placement | `game-kit` `map_transition.rs` | x30, x31, x32, lw19 |
| [groups.md](groups.md) | Single items and neighbour groups for blocked areas | `game-kit` `groups.rs` | x20, rt9, lw14 |
| [projectiles.md](projectiles.md) | Straight-flying projectiles that stop at blocked areas and the map edge | `game-kit` `projectile.rs` | x21, rt11 |
| [nav.md](nav.md) | Grid path finding built from the collision outlines, nearest reachable point | `game-kit` `nav.rs` | x23, x43, rt12 |
| [pointer.md](pointer.md) | Mouse clicks as a world position message | `game-kit` `pointer.rs` | x42, lw30 |
| [enemy-movement.md](enemy-movement.md) | Goals, patrol, pursue and sentry strategies, touch messages, facing tags, hit boxes, seeded random numbers | `game-kit` `movement.rs`, `strategies.rs`, `touch.rs`, `hitbox.rs`, `rng.rs`, `movement_debug.rs` | x24, x26, x27, x40, x41, x43, rt12 |
| [spawners.md](spawners.md) | Spawners with settings in the map legend | `game-kit` `spawner.rs`, `map.rs` | x25, rt12 |
| [debug-tools.md](debug-tools.md) | Grid, info box, collision overlay | `game-kit` `debug.rs`, `collision_debug.rs` | lw3, lw6 |
| [collectables.md](collectables.md) | Items the player collects by touch, `Collected` message | `game-kit` `collectable.rs` | x38, lw27 |
| [dev-scripts.md](dev-scripts.md) | Zoomed screenshots and Aseprite checks (`shot.sh`, `aseprite.sh`) | `dev-scripts/` | |
| [dev-feedback.md](dev-feedback.md) | Plain function tests, logging, scripted input, screenshots | `game-kit` `script.rs`, `dev-scripts/rust.sh` | lw5 |
| [plugin-configuration.md](plugin-configuration.md) | Plugin fields stored as a resource | `game-kit` `collision.rs`, `debug.rs` | none |
| [screen-layout.md](screen-layout.md) | Screen regions, HUD bar, app states, pause | `game-kit` `screen.rs`, `hud.rs`, `app_state.rs`, `camera.rs` | x34, x35, lw24 |
| [pixel-art.md](pixel-art.md) | View height scale, animation tag switching, animation speed, random start frame | `game-kit` `camera.rs`, `animation.rs`, `player.rs` | e7, a6, x39 |
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
