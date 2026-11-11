# Aseprite animation tags

Tag names are the contract between art and code (`art-pipeline.md`, rule 7). Names are case sensitive.

| File | Tags |
|---|---|
| `characters/Hunter.aseprite` | `Idle`, `Run`, `Hurt`, `Die` (side view, faces right), `Up` and `Down` (drawn by the user). Layers: `Body`, `Weapon`, `background`; the game draws `Body` and optionally `Weapon`. See `HUNTER-sprites.md`. The player (`player.rs`) plays `Hurt` once when it loses HP (`PlayerHurt` message of `hp.rs`, `adr-lw28`). |
| `characters/ghost.aseprite` | `Down`, `Left`, `Up` (4 frames, 200 ms), `Attack` (2 frames, 200 ms, played once on touch), `Die` (8 frames, 100 ms, played once). `Left` also serves `Right` as a flip. One layer, `Body`. The ghost (`ghost.rs`) uses `Down`, `Left`, `Up`, `Attack` and `Die`. |
| `features/Green tree.aseprite` | none (all 8 frames loop) |
| `features/Red tree.aseprite` | none (all 8 frames loop) |
| `features/Rocks.aseprite` | `rock_0` (frame 1), `rock_1` (frame 2). Each tag is one frame and is a rock variant, not an animation. |
| `features/Markers.aseprite` | `lantern` (frame 1), `lantern_red` (frame 2), `marker` (frame 3), `marker_red` (frame 4). One frame each, a variant not an animation. Each tag is a map kind of the same name. |
| `pickups/pickups.aseprite` | `pickup_blue` (frames 1-8), `pickup_red` (frames 9-16), `pickup_violet` (frames 17-24), `pickup_black` (frames 25-32). Each tag is an 8 frame pulse loop (100 ms per frame, set in the file). One layer, `Pickup`. |
| `ui/hp.aseprite` | `Full` (frame 1), `Empty` (frame 2). One frame each, a state not an animation. One layer, `Icon`. The HUD shows one icon per point of max HP. |
| `ui/weapon_icons.aseprite` | `fireball`, `fire_ring`, `ice_ring`, `fire_wall`, `ice_wall`, `whirlpool` (frames 1 to 6, one frame each, a weapon icon not an animation). One layer, `Icon`. The tag name is `WeaponKind::icon_tag` in `weapon_menu.rs` (`adr-lw31`). |
| `ui/weapon_drawer.aseprite` | none (one frame, the picture of the weapon drawer). One layer, `Background`. |

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
| `energy_blue` | `pickups/pickups.aseprite` | `pickup_blue` |
| `energy_red` | `pickups/pickups.aseprite` | `pickup_red` |
| `energy_violet` | `pickups/pickups.aseprite` | `pickup_violet` |
| `energy_black` | `pickups/pickups.aseprite` | `pickup_black` |
| `spawn_ghost` | `characters/ghost.aseprite` | `Down`, `Left`, `Up`, `Attack`, `Die` |
| `green_tree` | `features/Green tree.aseprite` | none, all frames loop |
| `red_tree` | `features/Red tree.aseprite` | none, all frames loop |
| `green_tree_medium` | `features/green_tree_medium.aseprite` | none, one frame |
| `red_tree_medium` | `features/red_tree_medium.aseprite` | none, one frame |
| `red_tree_small` | `features/red_tree_small.aseprite` | none, one frame |
| `green_shrub` | `features/green_shrub.aseprite` | none, one frame |
| `green_shrub_small` | `features/green_shrub_small.aseprite` | none, one frame |
