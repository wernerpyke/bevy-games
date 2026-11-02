# Long Way maps

How to write a map file for Long Way. The file format itself is the [map files extension](../../../dev-docs/extensions/map-files.md). This page covers what Long Way adds: the kinds, sizes, files and checks.

- Files are in `games/long-way/assets/maps/`. The current example is `rocks-test.md`.
- The game loads one map. Its name is set in `src/map.rs` (`MAP_NAME` and `MAP_SOURCE`). After you edit the file, rebuild the game.
- Press F2 to show the grid with cell numbers. Use those numbers for positions.
- Cell size is 64 pixels (`cell: 64`).
- Cell (0, 0) is the fixed world position `MAP_ORIGIN` in `camera.rs` (-1024, -576), the corner of a 16:9 view rounded to whole cells (`adr-lw8`).

## Kinds you can use in a legend

| Kind | Art | Notes |
|---|---|---|
| `rock_0`, `rock_1` | `features/Rocks.aseprite` | Variants of a rock |
| `green_tree` | `features/Green tree.aseprite` | Animated, 8 frames |
| `red_tree` | `features/Red tree.aseprite` | Animated, 8 frames |

`draw` (rock size in pixels, a multiple of 32, default 128) applies to rocks only. `jitter` also shifts trees.

## Sizes

One rock covers 2 by 2 cells. The character you write is the bottom left cell of that rock. The rock extends one cell to the right and one cell up. A tree also has a 2 by 2 cell footprint, with its bottom left corner on the cell you write.

Characters that are next to each other overlap by one cell. This is on purpose: it makes a pile of rocks that looks like one cluster. A solid block of 4 by 3 characters gives a cluster of 5 by 4 cells.

## Trees

A tree is a feature with one character, for example `g` for `green_tree`. The trunk blocks Guy. He can walk behind the tree from above and overlap its trunk from below by half of his height (`adr-lw1`). Trunks that touch rocks join the rock cluster.

## Rocks and collision

Guy cannot walk through a rock. The game joins the rocks of a cluster, and Guy walks around the edge of the whole cluster. Rocks in the middle of a cluster have no edge and cost no extra work. The rules and numbers are in [collision.md](collision.md).

## A small complete example

````markdown
---
map: small
cell: 64
origin: bottom-left
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---

# Features

## Boulder pair

Two rocks side by side.

```cells
0 1
```

# Map

- 'Boulder pair' at (4,4) and (10,4)
````

This places two rocks at each of two positions. The pair at (4,4) covers cells 4 to 6 across and 4 to 5 up, because the two rocks overlap by one cell. Each pair is one cluster. The second pair is a separate cluster.

## Checking the result

1. Run the game and press F2.
2. The red grid shows cell numbers. Check that each feature starts where you expect.
3. Yellow lines show the edge that Guy collides with. Cyan lines show rocks in the middle of a cluster. Green squares show the feet of every object.
4. For a picture without the window, run `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:map;quit"`. The picture is saved in `target/screenshots/map.png`.

If the file has a mistake, the game stops when it starts and names the file and line.
