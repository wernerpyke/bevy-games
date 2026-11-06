# Research: how other engines move between maps

Status: research done. The decisions are in `adr-x30` to `adr-x32` (`dev-docs/adr/EXTENSIONS.md`) and the design is in `dev-docs/extensions/map-transitions.md`. Date: 2026-11-06.

Evidence labels: **[web]** came from a web search in this session. **[memory]** is general knowledge and was not checked in this session. **[code]** was read in this repository. Check a [memory] item before you rely on it.

| Engine / tool | Approach | Notes |
|---|---|---|
| Godot | One scene per level. A door is an `Area2D` with a target scene path. An autoload "scene manager" swaps the scene. The spawn point is a name or a marker node in the new scene. [web] [door and autoload advice](https://forum.godotengine.org/t/the-problem-with-switching-between-scenes/103861), [spawn point thread](https://forum.godotengine.org/t/character-spawn-point-when-changing-scenes/102194) | Keep the player in its own scene and move it between levels. [web] [RPG transitions thread](https://forum.godotengine.org/t/whats-the-best-way-to-do-rpg-transitions-in-godot/55194). Use threaded loading for large scenes. [web] Keep cross-level state in an autoload. [web] |
| Zelda-style rooms (Godot, Unity, GameMaker) | An edge trigger moves the player into the next room. The camera pans to the new room's fixed position. Per-room camera limits are the hard part. [web] [camera limits thread](https://godotforums.org/d/30910-how-to-setup-a-camera-confinder-for-a-zelda-like-room-transition) | Gameplay is usually paused during the pan. [memory] |
| LDtk with `bevy_ecs_ldtk` | Levels sit at positions in one world. A `LevelSet` or `LevelSelection` chooses which levels are spawned, so neighbours can be loaded together. [web] [docs.rs](https://docs.rs/crate/bevy_ecs_ldtk/latest/source/src/components/level_set.rs) | This is the model for seamless maps: each map has a world offset. Portals carry data as entity fields. [memory] |
| Tiled and RPG Maker | A warp object or event has properties: target map, target spawn name or x,y, facing, fade. [memory] | This is the model for scene transitions and for the data in the map file. |
| Unity | Additive scene loading for streaming. A camera confiner (Cinemachine) is swapped per room. [memory] [camera confiner mentioned in thread](https://godotforums.org/d/30910-how-to-setup-a-camera-confinder-for-a-zelda-like-room-transition) | Same split: additive for seamless, single load for a scene change. |
| Unreal | Level streaming volumes and World Partition load and unload sublevels by distance. [memory] | Heavy for this project. The idea to keep: unload when the player is far enough away, not at the seam. |
| Bevy | No built-in map transition. `States` plus state-scoped entities (`DespawnOnExit`) give a clean unload. [code] `bevy-src/crates/bevy_state/src/state_scoped.rs` | We write the lifecycle ourselves. |

### What all of them share

1. A **trigger** in the old map (area, edge, door).
2. A **target**: map id and an entry name. The new map owns where the player stands.
3. A **lifecycle** with fixed steps: lock input, transition out, unload, load, place the player, transition in, unlock.
4. **Entities that survive** (the player, the camera) are kept out of the unload.
5. Some form of **per-map camera limits**.

## What we took from it

- Scene transition: a trigger area with a target map and an entry name, a fade, a fixed lifecycle, and a player that survives the unload (`adr-x31`).
- Seamless transition: every map has a world position, neighbours are loaded together, and a map is removed when it is far enough away and not at the seam (`adr-x32`).
- The camera is limited to the current map, and moves on to the new map after a crossing.

## Bevy asset loader for the maps (not chosen)

The registry keeps our own parser and `include_str!`. A Bevy asset loader would give: a map fetched over HTTP on the web build, so the wasm file is smaller and a map edit needs no rebuild; hot reload on native through the asset watcher; neighbour maps preloaded in the background for seamless links; and load states that fit `bevy_asset_loader`. It would cost: a custom `AssetLoader` for our parser, a map that is not ready on the frame you ask for it (the lifecycle needs a wait state), and errors that arrive as asset events. The maps are tiny, so load time is no gain. `MapRegistry` hides where the text comes from, so the switch later changes only the registry. Decide with roadmap item 3.
