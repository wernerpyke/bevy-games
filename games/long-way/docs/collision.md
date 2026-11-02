# Collision in Long Way

The numbers and object rules Long Way gives to the [collision extension](../../../dev-docs/extensions/collision.md). Decisions: `adr-lw1`, `adr-x4`, `adr-lw8`, `adr-x10`, `adr-lw13`.

## Guy

- Feet: the point at the bottom of Guy's sprite, 30 below the canvas centre (`GUY_FEET_OFFSET`).
- Half width 20 (`GUY_HALF_WIDTH`). Height 60 (`GUY_HEIGHT`).
- The player is clamped to the view after collision (`player.rs`), which can move the feet into a shape.

## Trees (`Trunk`)

Each tree has a `Trunk { left, right, base_y, height }` in world coordinates. Its blocked area for the feet is:

- **Sides:** `left - 20` to `right + 20`, so the body cannot overlap the trunk from left or right.
- **Bottom:** `base_y - 30` (`OVERLAP_FROM_BELOW`, half of Guy's height). Walking up from below, Guy may overlap the trunk by 50% of his body.
- **Top:** `base_y + 10%` of the tree height (`BLOCKED_BOTTOM`). Above this line Guy walks behind the tree and overlaps the trunk and canopy freely. He cannot enter the bottom 10%.

Trunk edges come from art pixels (`TRUNK_LEFT_PX`, `TRUNK_RIGHT_PX` in `trees.rs`). A trunk that overlaps a rock joins the rock's cluster.

## Rocks

- Art: `Rocks.aseprite`, native 32x32, tags `rock_0` and `rock_1`. It is drawn at a whole number scale: `draw / 32` (4 = 128 px, 5 = 160, 6 = 192). Default is 128.
- Footprint: 2x2 cells (128 x 128).
- Blocked area: the footprint inset by 10% on the left, right and top, and not at the bottom. That is 102.4 wide and 115.2 high. It is then grown sideways by Guy's half width (20), giving 142.4 wide.
- No other vertical margin. The bottom line sits on the lowest art pixel. The lowest 4 art rows are empty, so the sprite is placed with its lowest visible pixel on the footprint bottom.
- Sort line: the bottom of the blocked area plus `SORT_BIAS` (1). Guy stopped at a rock is drawn in front of it. Guy above it is behind it.
- Rock `draw` and `cell` sizes must agree, or the sprite and the blocked area drift apart.

### Clusters

- Rocks 2 cells apart have blocked areas that overlap (the inset footprints are 25.6 apart and Guy's 20 on each side closes it), so they merge.
- Rocks 3 or more cells apart leave a gap of about 50 pixels that Guy can walk through, so they are separate clusters.
- Everything inside a cluster is unreachable, so interior rocks cost nothing.
- Drawing larger than the footprint does not change collision.

### Drawn overlap (facts from the art)

- `rock_0.png` and `rock_1.png` were 32x32 art scaled 8x. The opaque part is 90.6% of the width and 81% of the height.
- At 128 px (4x) the rock is about 116 wide, so two neighbours at a pitch of 2 cells do not touch (12 px gap). At 160 px they overlap by about 17 px. At 192 px they overlap by about 46 px.
- Long Way draws at 128 with no jitter (`adr-lw8`).

## Slide values

All in `collision.rs`.

| Constant | Value | Meaning |
|---|---|---|
| `SLIDE_DELAY` | 0.2 s | Push time before help (changed from 0.5 after play testing) |
| `SLIDE_SPEED` | 120 units/s | Slide speed |
| `SLIDE_RANGE_FRACTION` | 0.35 | Share of the face length, from the nearer end |
| `SLIDE_RANGE_CAP` | 64 | Upper limit of that range |
| `SLIDE_CLEARANCE` | 192 | Free distance beyond an end. Assumes a 64 px cell |
| `SLIDE_PAST_CORNER` | 0.5 | Overshoot when the way on is blocked |
| `TOLERANCE` | 0.01 | In `outline.rs` (`game-kit`) |

Accepted by the developer after play testing on 2026-11-02. Options not built: a 96 px cap, or the nearest outline corner with a cap.

Example: pushing up into the bottom face of the test cluster from world x 463, Guy slides to x 440.3 and walks up to y 64.

## Known issues

- The 192 clearance should be written in cells if the cell size changes.
- Not checked in the real game: a slide past a corner of the lower left rock patches, a tree touching a rock, and a flicker check of overlapping rocks over several frames.
- Guy placed inside a shape (teleport) can walk freely inside it.

## Tests

Collision tests are in `collision.rs`, outline tests in `outline.rs`, rock tests in `rocks.rs` (placement, blocked area, sort line). See [testing.md](testing.md).
