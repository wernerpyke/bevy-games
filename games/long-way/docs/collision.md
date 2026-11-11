# Collision in Long Way

The numbers and object rules Long Way gives to the [collision extension](../../../dev-docs/extensions/collision.md). Decisions: `adr-lw1`, `adr-x4`, `adr-lw8`, `adr-x10`, `adr-lw13`.

## The player

- Feet: the point at the bottom of the player sprite, 56 below the canvas centre (`PLAYER_FEET_OFFSET`). The player is the Hunter (`Hunter.aseprite`, 128x128, feet at y 120), see `adr-lw15`.
- Half width 14 (`PLAYER_HALF_WIDTH`). It must stay above 12.8, or rocks two cells apart stop merging.
- The player is clamped to the view after collision (`player.rs`), which can move the feet into a shape.

## Trees (`Trunk`)

Each tree has a `Trunk { left, right, base_y, height }` in world coordinates. Its blocked area for the feet is:

- **Sides:** `left - 20` to `right + 20`, so the body cannot overlap the trunk from left or right.
- **Bottom:** `base_y`, the base of the tree. Walking up from below, the player stops with its feet on the base line, as at the bottom of a rock (changed 2026-11-02, before this it was 30 below the base). A tree sorts at its base plus `SORT_BIAS` (1), as a rock does, so the player standing at the base is drawn in front.
- **Top:** `base_y + 10%` of the tree height (`BLOCKED_BOTTOM`). Above this line the player walks behind the tree and overlaps the trunk and canopy freely. The player cannot enter the bottom 10%.

Trunk edges come from art pixels (`TRUNK_LEFT_PX`, `TRUNK_RIGHT_PX` in `trees.rs`). A trunk that overlaps a rock joins the rock's cluster.

## Rocks

- Art: `Rocks.aseprite`, native 32x32, tags `rock_0` and `rock_1`. It is drawn at a whole number scale: `draw / 32` (4 = 128 px, 5 = 160, 6 = 192). Default is 128.
- Footprint: 2x2 cells (128 x 128).
- Blocked area: the footprint inset by 10% on the left, right and top, and not at the bottom. That is 102.4 wide and 115.2 high. It is then grown sideways by the half width (14), giving 130.4 wide.
- No other vertical margin. The bottom line sits on the lowest art pixel. The lowest 4 art rows are empty, so the sprite is placed with its lowest visible pixel on the footprint bottom.
- Sort line: the bottom of the blocked area plus `SORT_BIAS` (1). The player stopped at a rock is drawn in front of it. The player above it is behind it.
- Rock `draw` and `cell` sizes must agree, or the sprite and the blocked area drift apart.

### Clusters

- Rocks 2 cells apart have blocked areas that overlap (the inset footprints are 25.6 apart and the half width of 14 on each side closes it), so they merge.
- Rocks 3 or more cells apart leave a gap of about 62 pixels that the player can walk through, so they are separate clusters.
- Everything inside a cluster is unreachable, so interior rocks cost nothing.
- Drawing larger than the footprint does not change collision.

### Drawn overlap (facts from the art)

- `rock_0.png` and `rock_1.png` were 32x32 art scaled 8x. The opaque part is 90.6% of the width and 81% of the height.
- At 128 px (4x) the rock is about 116 wide, so two neighbours at a pitch of 2 cells do not touch (12 px gap). At 160 px they overlap by about 17 px. At 192 px they overlap by about 46 px.
- Long Way draws at 128 with no jitter (`adr-lw8`).

## Groups (`adr-lw14`)

Objects whose footprints overlap or touch (also at a corner) form a group, whatever the kinds. Rocks and trees cover 2x2 cells, so their anchors are at most 2 cells apart. The Zen Garden kinds cover their own cell space: `green_tree_medium` 2x2, `red_tree_medium` 1x2, the other three 1x1 (`Kind::footprint`, `adr-lw23`).

- A tree alone: the trunk band above. A rock alone: the bottom 10% of the footprint height (12.8 at a cell size of 64), with the rock sides (inset 10%, grown by 20).
- In a group, a rock blocks the rock footprint area above. A member with a member above it (`Group::neighbour_above`) has no top inset, so the shapes merge.
- In a group, a tree blocks exactly one map cell: the bottom right cell of its footprint, which holds the trunk. The box is 64 by 64 with no widening, so it lines up with the F3 grid and boxes of neighbours touch with no gap (`adr-lw18`). The player can enter the cells between them.
- In a group, each Zen Garden kind blocks exactly its anchor cell (64 by 64), which holds its trunk or base. Alone, a tree blocks the trunk band above and a shrub a band as wide as the red tree of its size.
- Footprints 3 cells apart do not touch, so they stay separate.

## Slide values

The values are `SlideTuning` fields in `crates/game-kit/src/collision.rs`. Long Way passes `cell_size` 64 to `CollisionPlugin`, which gives these values.

| Constant | Value | Meaning |
|---|---|---|
| `delay` | 0.2 s | Push time before help (changed from 0.5 after play testing) |
| `speed` | 120 units/s | Slide speed |
| `range_fraction` | 0.35 | Share of the face length, from the nearer end |
| `range_cap` | 64 | Upper limit of that range |
| `clearance` | 192 | Free distance beyond an end. Assumes a 64 px cell |
| `past_corner` | 0.5 | Overshoot when the way on is blocked |
| `TOLERANCE` | 0.01 | In `outline.rs` (`game-kit`) |

Accepted by the developer after play testing on 2026-11-02. Options not built: a 96 px cap, or the nearest outline corner with a cap.

Example: pushing up into the bottom face of the test cluster from world x 463, the player slides to x 440.3 and walks up to y 64.

## Known issues

- The 192 clearance should be written in cells if the cell size changes.
- Not checked in the real game: a slide past a corner of the lower left rock patches, a tree touching a rock, and a flicker check of overlapping rocks over several frames.
- The player placed inside a shape (teleport) can walk freely inside it.

## Tests

The trunk band is tested through `trees.rs` and `scenery.rs`. Rock tests are in `rocks.rs` (placement, blocked area, sort line, jitter), lantern and marker tests in `markers.rs`. The engine tests (`collision.rs`, `outline.rs`) are in `crates/game-kit`. See [testing.md](testing.md).
