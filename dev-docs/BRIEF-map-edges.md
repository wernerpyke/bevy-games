I want to change how map edges are defined and implemented.

Today, maps are defined as follows:
```
   |0                  |
   |0 1 2 3 4 5 6 7 8 9|
 9 |. . . . . . . . . .|
 8 |@ . . . . . . . . .|
 7 |. . . . . . . . . .|
 6 |. . . .   . . . . .|
 5 |. . . . . . . . . .|
 4 |. . . . . . . . . .|
 3 |. . . . . . . . . .|
 2 |. . . . . . . . . .|
 1 |. . . . . . . . . .|
 0 |. . . . . . . . . .|
```

In this layout:
- `.` - indicates free space
- `  ` - is also allowed to indicate free space as a convenience, for example in col=4, row=6

I want to change this so that `.` indicates the map edge and `  ` or ` ` indicates free space.
We need to retain the `  ` to allow for alignment to the column numbers.

The map edge is then populated with the appropriate sprite as defined below. 
## Functionality to be retained
The following is currently possible and needs to be retained:
- A map exit can be indicated either by an exit marker or by an exit area. It should still be possible to use exit areas.
- The player start position (`@`) can be set to a map edge, this should remain possible. The player is then just placed over an edge frame.

## Map edge sprites

The map edges are drawn using sprites that define the frame to use for a `north`, `north_east`, `east`, `south_east`, `south`, `south_west`, `west`, `north_west` or `center` cell.
See the example at @games/long-way/input/map_edge.aseprite

The collision/blocked area for each sprite is the 50% of the sprite that is opaque. `center` blocks the whole cell.
The corner pieces are diagonal, and collision only has axis-aligned rectangles, so a corner blocks with stair-step rectangles that follow the diagonal. The step size is to be tuned by playing.

## New layout example
Given the change described above the following layout should then be possible:

```
   |0                  |
   |0 1 2 3 4 5 6 7 8 9|
 9 |. . . . . . . . . .|
 8 |@     . . .     . .|
 7 |.                 .|
 6 |.                 .|
 5 |.                 .|
 4 |. .             . .|
 3 |. . .         . . .|
 2 |.           . . . .|
 1 |. . .       . . . .|
 0 |. . . . . . . . . .|
```

In the above example the sprites would then render as in the below. Note that in the below: c=center, n=north, ne=north_east etc. Columns are 3 characters wide so that two-letter names fit.

```
   | 0  1  2  3  4  5  6  7  8  9|
 9 | c  n  n  c  c  c  n  n  c  c|
 8 | w        ne c  nw       ne c|
 7 | w                          e|
 6 | w                          e|
 5 | w                          e|
 4 | c  sw                   se c|
 3 | c  c  w              se c  c|
 2 | w                 se c  c  c|
 1 | c  c  sw          c  c  c  c|
 0 | c  c  c  s  s  s  c  c  c  c|
```

Notice how, in the above example the player start is rendered as a `west`. The player is still placed col=0, row=8.

The drawn example is `games/long-way/input/map_edge_example.aseprite`.

## Choosing the piece

An edge cell gets its piece from its 4 direct neighbours (north, east, south, west). A neighbour is free when it is a blank. A cell outside the map, and an object on the outer ring of the map (see "Cells with objects"), count as edge.

Step 1. The free sides give the piece:

| Free sides | Piece |
|---|---|
| none | `center` |
| S | `north` |
| N | `south` |
| W | `east` |
| E | `west` |
| S and W | `north_east` |
| S and E | `north_west` |
| N and W | `south_east` |
| N and E | `south_west` |
| N, E and S | `west` |
| E, S and W | `north` |
| N, S and W | `east` |
| N, E and W | `south` |

With three free sides, the piece is the one of the closed side.

Step 2. A straight piece (`north`, `east`, `south`, `west`) from step 1 becomes `center` when a neighbour along its wall is a piece other than `center` or the same straight piece. The neighbours along the wall of `north` and `south` are west and east. The neighbours along the wall of `east` and `west` are north and south. Step 2 uses the pieces of step 1, so a change does not spread to the next cell.

The reason: a straight piece is half a cell high where it meets its neighbour. A corner piece, or a straight piece that faces another way, is full height there, so the join would leave a half-cell notch.

In the example, step 2 changes these cells to `center`:
- col=4, row=8: `north` between `north_east` and `north_west`
- col=1, row=3: `north` next to `west`
- col=6, row=1: `east` below `south_east`
- col=1, row=1: `south` next to `south_west`

Not supported:
- Two free sides that are opposite (N and S, or E and W), and four free sides. These are walls one cell thick, or a single edge cell. An edge is always the border of the map, so it has no such cells. Such a cell is a load error that names the line, like other map format errors.
- A `.` inside an exit or entry area. It would close the exit or the entry. It is a load error that names the line of the area.

Known limitation of the two steps:
- **Inside corners.** A cell whose only free neighbour is diagonal gets `center`, which leaves a square corner. Example: col=0, row=9. The quarter rules below remove it.

## Smoother corners with quarter pieces (for the plan)

Drawn examples: `games/long-way/input/map_edge_example.aseprite` (the two steps only) and `games/long-way/input/map_edge_example_smooth.aseprite` (the two steps, then the quarter rules below on the whole map).

**Decision (2026-11-15, `adr-x49`).** The kit supports both: each game passes `EdgeMode::Basic` (the two steps only) or `EdgeMode::Smooth` (the two steps, then the quarter rules). Long Way uses `Smooth`.

**Spike.** The plan has a stand-alone spike step that tries both possibilities: the two steps only, and the two steps with smooth corners (both quarter rules below). The criterion is how cleanly the smooth corners can be built in `game-kit` without spilling a lot of complexity into the game code or the art. The game should still only map pieces to frames and set the z-order. If smooth corners need more from the game or the art than that, the plan keeps the two steps only.

Some cells look better when they are split into four quarters, each drawn with a piece at half size. The half-size piece is the existing frame scaled down with nearest-neighbour sampling, so no new frames are drawn. Its pixels are half the size of the other pieces. Its blocked area is a stair-step with the same step size as a full-size piece, so it has half as many steps.

### Quarter rules

The rules look at the basic rendering (steps 1 and 2) one quarter at a time. A side of a quarter is **closed** when the touching edge of the neighbouring quarter is fully opaque, and **open** when the neighbouring quarter is fully transparent. Outside the map is closed. All rules use the basic rendering, so a change does not spread.

1. **Cut.** A fully opaque quarter with two adjacent open sides, whose other two sides are closed, gets the half-size diagonal piece of its closed sides. It removes a square corner of the wall.
2. **Fill.** A fully transparent quarter with two adjacent closed sides, whose other two sides are not closed, gets the half-size diagonal piece of its closed sides. It fills a square inside corner of the room.
3. A fill is skipped when a neighbouring quarter is cut. This is the case of a step one quarter high, where cut and fill would both fire.

The diagonal piece is named by the closed sides: north and west give `north_west`, and so on.

### Result in the example

The quarters are written `top-left top-right / bottom-left bottom-right`. A full-size letter such as `n` is that quarter of the cell's own full-size piece, and `_` is open.

| Cell | Rule | Quarters |
|---|---|---|
| col=0, row=9 | cut, bottom-right | `c c / c nw` |
| col=2, row=3 (tip of the bump) | cut, both left quarters | `sw _ / nw _` |
| col=0, row=2 (end of the slot) | fill, both right quarters | `c nw / c sw` |
| col=2, row=9 | fill, bottom-right | `n n / _ ne` |
| col=6, row=9 | fill, bottom-left | `n n / nw _` |
| col=7, row=9 | fill, bottom-right | `n n / _ ne` |
| col=9, row=7 | fill, top-left | `ne e / _ e` |
| col=9, row=5 | fill, bottom-left | `_ e / se e` |
| col=0, row=5 | fill, bottom-right | `w _ / w sw` |
| col=3, row=0 | fill, top-left | `sw _ / s s` |
| col=5, row=0 | fill, top-right | `_ se / s s` |

Two fills are skipped because they are next to the cut at col=0, row=9: col=1, row=9 bottom-left and col=0, row=8 top-right.

Each fill continues the diagonal of the neighbouring corner piece, or makes a new 45° cut where two straight walls meet. The cut at col=0, row=9 joins the two square corners of a step one quarter high with one diagonal.

Some corners are not changed, because their quarters are not fully opaque or fully transparent. For example, where a diagonal ends at a straight wall (col=4, row=8, and the west side of col=6, row=1), the joint stays at 135°.

## Cells with objects

A cell with an object (a letter, digit or special character in the grid) has no `.` or blank, so its place in the map decides what it is. The rule is the same for every kind of object: features, pickups, enemy spawns, markers and `@`.

- **On the outer ring** (the first or last row, or the first or last column): an ordinary edge cell. It gets an edge piece, it counts as edge when its neighbours choose their pieces, and it blocks like any other edge cell. The object is drawn as usual, and the edge layer draws over or under it (see "Additional notes"). The object also blocks as it does today.
- **In an entry area** on the outer ring: free space, because an entry area is a gap in the edge. Map format rule 21 allows only `@` there.
- **Inside the map**: free space. It gets no edge piece and no edge collision. The object blocks as it does today.

The outer border of the map keeps the player and the enemies inside the map, as it does today.

Place objects on the outer ring with care. A pickup or an exit marker in the blocked half of an edge piece can be hard or impossible to touch.

**Player start on the edge.** The kit gives a start position for each piece: a point outside the blocked area of the piece after the game grows it by the actor half width. The player starts there, over the edge sprite, and cannot move into the blocked half. In the example, `@` at col=0, row=8 is on a `west` piece. The player starts to the right of the grown blocked area and cannot move left. A `center` piece blocks the whole cell, so it has no such point: the player then starts at the centre of the nearest free cell. Ties are broken in a fixed order, so the result does not change between runs.

**Footprints.** The file is the truth. The other cells of a footprint are drawn with `.` or blanks (map format rule 12). A `.` footprint cell is an edge cell, and a blank one is free. So footprint cells inside the map must be drawn with blanks. The kit does not know footprints (`adr-x13`).

## Additional notes
- I want the z-order of the map edge to be specified per game so that the edge can either be drawn as shrubbery on the ground (below the player) or a tree canopy (above the player).
- `layers.rs` names the two positions. Ground: -300, above the background tiles and below the grid debug, so F3 shows the grid over the edge. Canopy: +150, above the world band and below the debug drawings. Each game uses one of them. Long Way uses the ground (decision changed on 2026-11-15, it was the canopy).

## Kit and game

- `game-kit` owns the rules: the parser change (`".": edge` in the legend; `.` may only be `empty` or `edge`), choosing the piece for each cell, the errors, the blocked rectangles of each piece, and the start position on each piece. These are plain functions with tests in the kit.
- The game owns the art and the drawing: which frame each piece uses, and the z-order of the edge (see "Additional notes").
- R-Town keeps `".": empty` and does not change.

## Not in this brief

- Converting Long Way maps 1 to 3 to the new format. It is a step in the plan.