# Long Way maps

How to write a map file for Long Way. The file format itself is the [map files extension](../../../dev-docs/extensions/map-files.md). This page covers what Long Way adds: the kinds, sizes, files and checks.

- Files are in `games/long-way/assets/maps/`. The current example is `rocks-test.txt`, a 32 by 18 cell map.
- The game loads one map. Its name is set in `src/map.rs` (`MAP_NAME` and `MAP_SOURCE`), which also lists the kind names. After you edit the file, rebuild the game.
- The file uses `origin: bottom-left`, so the row labels in the file are the same numbers as the F3 grid. Press F3 to show the grid with cell numbers.
- Cell size is 64 pixels (`cell: 64`).
- Cell (0, 0) is the fixed world position `MAP_ORIGIN` in `camera.rs` (-1024, -576), the corner of a 16:9 view rounded to whole cells (`adr-lw8`). A map of 32 by 18 cells is two views wide and two high, so the camera scrolls (`adr-lw16`).
- Use a fixed-width font and turn off word wrap. Each cell takes two characters, so a row of 32 cells is 63 characters between the bars.

## Kinds you can use in a legend

| Kind | Art | Notes |
|---|---|---|
| `rock_0`, `rock_1` | `features/Rocks.aseprite` | Variants of a rock |
| `green_tree` | `features/Green tree.aseprite` | Animated, 8 frames |
| `red_tree` | `features/Red tree.aseprite` | Animated, 8 frames |

`draw` (rock size in pixels, a multiple of 32, default 128) applies to rocks only. `jitter` also shifts trees.

## Sizes

One rock covers 2 by 2 cells. The character you write is the bottom left cell of that rock. The rock extends one cell to the right and one cell up. A tree also has a 2 by 2 cell footprint, with its bottom left corner on the cell you write.

You may draw the cells that a rock or tree covers with blanks, to see how far it reaches. A `.` in those cells means the same. The game takes the size from the kind, not from the drawing.

Characters that are next to each other overlap by one cell. This is on purpose: it makes a pile of rocks that looks like one cluster. A solid block of 4 by 3 characters gives a cluster of 5 by 4 cells.

## Trees

A tree is one character, for example `g` for `green_tree`. The trunk blocks the player. The player can walk behind the tree from above and stops at its base line from below (`adr-lw1`). Trunks that touch rocks join the rock cluster. Four trees two cells apart in both directions touch and form a grove (see `rocks-test.txt`, cells (12,2) to (14,4)).

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
