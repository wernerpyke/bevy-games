# Aseprite animation tags

Tag names are the contract between art and code (`art-pipeline.md`, rule 7). Names are case sensitive.

| File | Tags |
|---|---|
| `characters/Guy.aseprite` | `Idle`, `Walk`, `Up`, `Down`, `Block`, `Win` |
| `characters/Hunter.aseprite` | `Idle`, `Run`, `Hurt`, `Die` (side view, faces right), `Up` and `Down` (drawn by the user). Layers: `Body`, `Weapon`, `background`; the game draws `Body` and optionally `Weapon`. See `HUNTER-sprites.md`. |
| `features/Green tree.aseprite` | none (all 8 frames loop) |
| `features/Red tree.aseprite` | none (all 8 frames loop) |
| `features/Rocks.aseprite` | `rock_0` (frame 1), `rock_1` (frame 2). Each tag is one frame and is a rock variant, not an animation. |
| `features/Markers.aseprite` | `lantern` (frame 1), `lantern_red` (frame 2), `marker` (frame 3), `marker_red` (frame 4). One frame each, a variant not an animation. Each tag is a map kind of the same name. |

## Map kinds

Kinds are the names a map legend uses (`maps.md`). Each is defined in code (`map.rs`).

| Kind | Art | Tag used |
|---|---|---|
| `rock_0` | `features/Rocks.aseprite` | `rock_0` |
| `rock_1` | `features/Rocks.aseprite` | `rock_1` |
| `lantern` | `features/Markers.aseprite` | `lantern` |
| `lantern_red` | `features/Markers.aseprite` | `lantern_red` |
| `marker` | `features/Markers.aseprite` | `marker` |
| `marker_red` | `features/Markers.aseprite` | `marker_red` |
| `green_tree` | `features/Green tree.aseprite` | none, all frames loop |
| `red_tree` | `features/Red tree.aseprite` | none, all frames loop |
