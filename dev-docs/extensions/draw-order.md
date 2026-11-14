# Draw order

Two parts: sorting world sprites by their feet, and a fixed set of named layers.

## Summary

- Bevy 2D sorts sprites by `z` and has no layer system. Equal `z` has no fixed order and flickers.
- `DepthSort` sets `z` from the feet position, so a character walks behind an object above it and in front of an object below it.
- `layers.rs` in the kit names every layer from background to UI. Game code writes no bare z number: it uses these names or its own layers module (adr-x14).

## Status

| Part | Code today | Shared? |
|---|---|---|
| `DepthSort`, `DepthPlugin { world_half_band }` | `crates/game-kit/src/depth.rs` | Shared. |
| Layer constants | `crates/game-kit/src/layers.rs` | Shared defaults. A game overrides a value through a plugin field or adds its own layers. |

Decisions: `adr-x2`, `adr-x9`, and `adr-lw12` for the Long Way layers. The same feet line decides collision ([collision.md](collision.md)), so what is drawn matches where the character can walk.

## Specification

### Sorting by feet

1. A `DepthSort { feet_offset }` component marks a sprite. `feet_offset` is the distance from the sprite centre down to its feet.
2. A system sets `z = -feet_y * Z_PER_Y + x * Z_PER_X`. Lower feet have a higher z and draw in front.
3. `Z_PER_Y` is 0.001. `Z_PER_X` is 1e-8. The x term only breaks ties between equal feet y, the one further right in front. It is far too small to override a difference in y.
3b. The x of the tie break is the middle of the footprint. A sprite whose `Transform` is not there (anchored at a corner) adds `TieBreakX(offset)`, the distance from its x to the footprint middle. Otherwise two objects can tie exactly and swap order from frame to frame (`adr-x2`).
4. z stays inside `WORLD_HALF_BAND` (100), checked by `debug_assert`. That allows feet y up to about 100000.
5. The sort runs in `Update` with no ordering against movement, so z can lag one frame.
6. Objects with a footprint choose their own sort line. The game adds a small bias so a character standing at the object is drawn in front.

### Layers

Back to front:

| Layer | Kind | Constant |
|---|---|---|
| Background | sprite z | `BACKGROUND_Z` |
| Grid debug | sprite z | `GRID_DEBUG_Z` |
| World | sprite z within `WORLD_HALF_BAND` of 0 | features and characters, sorted together |
| Feature debug | sprite z | `FEATURE_DEBUG_Z`, `FEATURE_DEBUG_INTERIOR_Z`, `FEATURE_DEBUG_FEET_Z` |
| Movement debug | sprite z | `MOVEMENT_DEBUG_Z` |
| HUD | `GlobalZIndex` | `HUD_Z_INDEX` |
| UI | `GlobalZIndex` | `UI_Z_INDEX` |

7. Features and characters share the world layer, so a character can walk behind a feature.
8. Draw code takes z and `GlobalZIndex` only from `layers.rs` or the game's own layers module.
9. A background uses a z below `GRID_DEBUG_Z`.
10. Debug drawings are sprites on their layer, not gizmos. 2D gizmos use sort key infinity and are always on top. See [Bevy notes](../bevy/notes.md).
11. `bevy_ui` nodes draw after all sprites and gizmos, so HUD and UI are always above the feature debug layer.

## Usage

- Add `DepthPlugin`.
- Add `DepthSort { feet_offset }` to any sprite that should sort.
- Import layer constants from `game_kit::layers`. A debug sprite goes on `FEATURE_DEBUG_Z`. A HUD node uses `GlobalZIndex(HUD_Z_INDEX)`.

## Limits

- Sorting by one point per sprite. A long object that a character can pass in the middle needs a different sort point.
- The z lag of one frame can show for a frame when two sprites swap order.
- Gizmos cannot be placed in a layer.

## Tests

Inline in `game-kit` `depth.rs`: lower feet in front, ties broken by x, `TieBreakX` moves the tie x, the world band holds far positions, x never beats y. Long Way tests that its scenery breaks ties at the footprint middle (`scenery.rs`).
