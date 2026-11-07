# Long Way maps

How to write a map file for Long Way. The file format itself is the [map files extension](../../../dev-docs/extensions/map-files.md). This page covers what Long Way adds: the kinds, sizes, files and checks.

- Files are in `games/long-way/assets/maps/`. The current map is `1.txt`, a 10 by 10 cell map.
- The game registers all three maps (`MAPS` in `src/map.rs`, ids `1`, `2`, `3`) and starts in map 1. Add a map there and in the legend list. `rust.sh run long-way 2.txt` starts in map 2 at its `@`. The name list is set in `src/map.rs` (`MAP_NAME` and `MAP_SOURCE`), which also lists the kind names. After you edit the file, rebuild the game.
- The file uses `origin: bottom-left`, so the row labels in the file are the same numbers as the F3 grid. Press F3 to show the grid with cell numbers.
- Cell size is 64 pixels (`cell: 64`).
- Cell (0, 0) is the fixed world position `MAP_ORIGIN` in `camera.rs` (-1024, -576). The view is 8 by 8 cells (`adr-lw17`). A map larger than 8 cells scrolls (`adr-lw16`), and a smaller one is centred. The size is read from the file, so you can change it freely. The player starts at the `player_start` marker (`adr-lw19`).
- Use a fixed-width font and turn off word wrap. Each cell takes two characters, so a row of 32 cells is 63 characters between the bars.

## Exits and entries

The format is in the [map files extension](../../../dev-docs/extensions/map-files.md) (rules 16 to 23, `adr-x30`). Long Way has three maps joined like this:

| From | Exit | Area | To | Entry | Area |
|---|---|---|---|---|---|
| 1 | `to_2` | `3..4,9` (top edge) | 2 | `from_1` | `3..4,0` (bottom edge) |
| 1 | `to_3` | `9,7` (right edge) | 3 | `from_1` | `0,7` (left edge) |
| 2 | `to_3` | `9,4..5` (right edge) | 3 | `from_2` | `0,8..9` (left edge) |

- Walking into an exit fades the screen, loads the target map and puts the player in the entry at the same position along the strip (`dev-docs/extensions/map-transitions.md`). The 1 to 2 exit is `transition=seamless`: map 2 is loaded above map 1, aligned strip on strip, and the player walks across (`adr-x32`). The two other exits are `scene`. Map 3 is reached from maps 1 and 2, so only one link to it can be seamless unless the three maps fit together without overlapping.
- Every exit and its entry have the same length. The player keeps their position along the strip.
- Write the cells of an exit or entry area as blanks in the grid. Keep trees and rocks out of them.
- Map 1 has the real start (`@` at cell (4,0)). Map 2 also has an `@`, at the cell of its entry, so a scripted run can start there.
- In a seamless join the player can leave a map only through the doorway (the exit strip). Everywhere else the map edge is a wall. The first map is removed after it has been out of view for 1 second.

## Kinds you can use in a legend

| Kind | Art | Notes |
|---|---|---|
| `rock_0`, `rock_1` | `features/Rocks.aseprite` | Variants of a rock |
| `green_tree` | `features/Green tree.aseprite` | Animated, 8 frames |
| `red_tree` | `features/Red tree.aseprite` | Animated, 8 frames |
| `lantern`, `lantern_red`, `marker`, `marker_red` | `features/Markers.aseprite` | 28x35 px (70% of the player's body height), drawn at scale 1 and centred in its cell. Blocks the middle 80% of its width and the bottom half of its height, rounded to whole pixels (22x18) (`adr-lw20`). Not part of a group. |
| `player_start` | none | A marker. The player starts at the centre of its cell (`adr-lw19`). It blocks nothing. Write it `"@"` in the legend. |

`draw` (rock size in pixels, a multiple of 32, default 128) applies to rocks only. `jitter` also shifts trees.

## Sizes

One rock covers 2 by 2 cells. The character you write is the bottom left cell of that rock. The rock extends one cell to the right and one cell up. A tree also has a 2 by 2 cell footprint, with its bottom left corner on the cell you write.

You may draw the cells that a rock or tree covers with blanks, to see how far it reaches. A `.` in those cells means the same. The game takes the size from the kind, not from the drawing.

Characters that are next to each other overlap by one cell. This is on purpose: it makes a pile of rocks that looks like one cluster. A solid block of 4 by 3 characters gives a cluster of 5 by 4 cells.

## Lanterns and markers

A lantern or marker is one character, for example `l` for `lantern`, `L` for `lantern_red`, `m` for `marker` or `M` for `marker_red`. All four have the same size and collision. Its character is its cell, and the art is centred in that cell. The player cannot walk through its bottom half and walks behind the top half. Lanterns and markers do not count in a cluster of rocks and trees (`adr-lw20`).

## Trees

A tree is one character, for example `g` for `green_tree`. The trunk blocks the player. The player can walk behind the tree from above and stops at its base line from below (`adr-lw1`). A tree that touches another tree or a rock is in a cluster and then blocks one cell, the bottom right cell of its footprint (the trunk cell) (`adr-lw18`). Four trees two cells apart in both directions touch and form a grove (see `1.txt`, cells (1,1) to (3,3)).

## Rocks and collision

The player cannot walk through a rock. The game joins the rocks of a cluster, and the player walks around the edge of the whole cluster. Rocks in the middle of a cluster have no edge and cost no extra work. The rules and numbers are in [collision.md](collision.md).

## A small complete example

```
---
map: small
cell: 64
origin: bottom-left
size: 12x4
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---
   |0 1 2 3 4 5 6 7 8 9 0 1|
 3 |. . . . . . . . . . . .|
 2 |. . . . . . . . . . . .|
 1 |. . . . 0 1 . . . 0 1 .|
 0 |. . . . . . . . . . . .|
```

This places two rocks at (4,1) and (5,1), and two more at (9,1) and (10,1). Each pair overlaps by one cell, so each pair covers 3 cells across and 2 up. The pairs are 3 cells apart, so each pair is its own cluster.

## Checking the result

1. Run the game and press F3.
2. The red grid shows cell numbers. Check that each object is where you expect.
3. Yellow lines show the edge that the player collides with. Cyan lines show rocks in the middle of a cluster. Green squares show the feet of every object.
4. For a picture without the window, run `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:map;quit"`. The picture is saved in `target/screenshots/map.png`.

If the file has a mistake, the game stops when it starts and names the file and line.
