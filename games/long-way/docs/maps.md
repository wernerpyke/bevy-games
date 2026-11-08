# Long Way maps

How to write a map file for Long Way. The file format itself is the [map files extension](../../../dev-docs/extensions/map-files.md). This page covers what Long Way adds: the kinds, sizes, files and checks.

- Files are in `games/long-way/assets/maps/`. The current map is `1.txt`, a 10 by 10 cell map.
- The game registers all three maps (`MAPS` in `src/map.rs`, ids `1`, `2`, `3`) and starts in map 1. Add a map there and in the legend list. `rust.sh run long-way 2.txt` starts in map 2 at its `@`. The name list is set in `src/map.rs` (`MAP_NAME` and `MAP_SOURCE`), which also lists the kind names. After you edit the file, rebuild the game.
- The file uses `origin: bottom-left`, so the row labels in the file are the same numbers as the F3 grid. Press F3 to show the grid with cell numbers.
- Cell size is 64 pixels (`cell: 64`).
- Cell (0, 0) is the fixed world position `MAP_ORIGIN` in `camera.rs` (-1024, -576). The view is 8 by 8 cells (`adr-lw17`). A map larger than 8 cells scrolls (`adr-lw16`), and a smaller one is centred. The size is read from the file, so you can change it freely. The player starts at the `player_start` marker (`adr-lw19`).
- Use a fixed-width font and turn off word wrap. Each cell takes two characters, so a row of 32 cells is 63 characters between the bars.

## Exits

Long Way leaves a map by touching a marker (`adr-lw21`, `adr-x33`, rule 24 of the [map files extension](../../../dev-docs/extensions/map-files.md)). The legend entry names the target map, and the player arrives at the `@` of that map. The maps have no `exits:` or `entries:` blocks.

| Map | Legend entry | Leads to |
|---|---|---|
| 1 | `"m": marker exit_to=2` | map 2 |
| 1 | `"M": marker_red exit_to=3` | map 3 |
| 2 | `"m": marker exit_to=3` | map 3 |
| 3 | none | no way out |

- Touching means the feet reach the touch area of the marker: its blocked area plus 2 pixels (`TOUCH_MARGIN`), made twice as wide and high around the same centre (`TOUCH_SCALE`, both in `markers.rs`). That is 52 by 44 pixels. The screen fades, the target map loads, and the player stands on its `@` with the same facing as before (`dev-docs/extensions/map-transitions.md`).
- Every map needs exactly one `@`, because a marker may lead to it. The game stops at start if a link names an unknown map or a map without one `@`.
- A marker without `exit_to`, and every lantern, only blocks the player.
- All links are scene transitions. Exit areas and seamless joins (`adr-x30`, `adr-x32`) still work in the kit, but no Long Way map uses them. Do not put a marker with `exit_to` inside an exit area.
- Start a scripted run in a map with `rust.sh script long-way "<commands>" 2.txt --debug`. `goto:<map>` in a script arrives at the `@`.

## Kinds you can use in a legend

| Kind | Art | Notes |
|---|---|---|
| `rock_0`, `rock_1` | `features/Rocks.aseprite` | Variants of a rock. Written `O` and `o` |
| `green_tree` | `features/Green tree.aseprite` | Animated, 8 frames |
| `red_tree` | `features/Red tree.aseprite` | Animated, 8 frames |
| `green_tree_medium` | `features/green_tree_medium.aseprite` | Zen Garden. Cell space 2x2, canvas 128x128 with its bottom left corner on the cell, like `green_tree`. Blocks the trunk foot |
| `red_tree_medium` | `features/red_tree_medium.aseprite` | Zen Garden. Cell space 1x2, canvas 84x128 (wider than the cell), trunk foot at the bottom middle. Blocks the trunk foot |
| `red_tree_small`, `green_shrub`, `green_shrub_small` | `features/red_tree_small.aseprite`, `green_shrub.aseprite`, `green_shrub_small.aseprite` | Zen Garden. Cell space 1x1, canvas 64x64, bottom of the art on the bottom of the cell. Shrubs block like a lone rock, the small tree like a tree trunk |
| `lantern`, `lantern_red`, `marker`, `marker_red` | `features/Markers.aseprite` | 28x35 px (70% of the player's body height), drawn at scale 1 and centred in its cell. Blocks the middle 80% of its width and the bottom half of its height, rounded to whole pixels (22x18) (`adr-lw20`). Not part of a group. `exit_to=<map id>` on the legend entry makes a marker a door (see Exits). |
| `energy_blue`, `energy_red`, `energy_violet`, `energy_black` | `pickups/pickups.aseprite` | Pickups, legend digits `1` to `4`. Drawn at a quarter of the art size (16x24), standing on the bottom of their cell. They do not block. Touching its box (52 wide, 64 high, centred on its base: 32 pixels below it and above it) collects the pickup and adds 1 to its count in the `Inventory` (`adr-lw27`). They come back when the map loads again. |
| `player_start` | none | A marker. The player starts at the centre of its cell (`adr-lw19`), and arrives there from another map (`adr-lw21`). It blocks nothing. Write it `"@"` in the legend. |

`draw` (rock size in pixels, a multiple of 32, default 128) applies to rocks only. `jitter` also shifts trees.

## Sizes

One rock covers 2 by 2 cells. The character you write is the bottom left cell of that rock. The rock extends one cell to the right and one cell up. A tree also has a 2 by 2 cell footprint, with its bottom left corner on the cell you write.

You may draw the cells that a rock or tree covers with blanks, to see how far it reaches. A `.` in those cells means the same. The game takes the size from the kind, not from the drawing.

Characters that are next to each other overlap by one cell. This is on purpose: it makes a pile of rocks that looks like one cluster. A solid block of 4 by 3 characters gives a cluster of 5 by 4 cells.

## Backgrounds

Every map has a `background:` block before `legend:` (`adr-lw26`, rule 25 of the [map files extension](../../../dev-docs/extensions/map-files.md)):

```
background:
  color: "#292829"
  mode: random
  fill: 0.2
  jitter: 16
  scale: 2
  tiles: grass_0 grass_1 grass_2 grass_3 flowers_0 flowers_1
```

- `color` is the ground. Use a colour of the game palette. Quote it.
- `mode` is `random` (the only mode for now).
- `fill` is the chance that a cell gets a tile image. `0.2` gives about one cell in five, and the number varies from map to map. A map with `fill: 0` has the colour only.
- `jitter` shifts each image by up to that many pixels, so the images do not sit on a visible grid. 16 pixels is a quarter of a cell.
- `scale` sets the image size as a multiple of its art size. The images are 32 by 32 art pixels, so `2` is 64 pixels (one cell), `1` is 32 pixels, `0.5` is 16 pixels and `4` is 128 pixels. A scale that is not a whole number draws some art pixels wider than others.
- `tiles` lists the images. Names: `grass_0` to `grass_3` and `flowers_0`, `flowers_1` (tags in `art/backgrounds/ground_tiles.aseprite`). The game stops at start and names the file and line if a map lists another name.
- `seed` (optional, default 0) re-rolls the ground. Change it if you do not like where the images landed.
- The ground is the same on every load, on F1 reload and when you re-enter a map. Making a map bigger keeps the images of the cells it had.
- The ground is a rectangle the size of the map, with the images above it. Both are below the F3 grid and the world. Code: `background.rs` (drawing) and `game_kit::map_background` (placement).

## Scenery (Zen Garden trees and shrubs)

The five Zen Garden kinds are one character each, for example `g` for `green_tree_medium`, `r` for `red_tree_medium`, `t` for `red_tree_small`, `b` for `green_shrub` and `s` for `green_shrub_small`. They are in the legend of every map. The character is the bottom left cell of the object's cell space (2x2 for `green_tree_medium`, 1x2 for `red_tree_medium`, 1x1 for the rest). The canvas of `green_tree_medium` has its bottom left corner on that cell, like `green_tree`. The other four hang from the bottom middle of their cell space. Trees block like `green_tree`: the foot of the trunk (the lowest 12 rows of the art, plus the player's half width on each side) and the bottom 10% of the canvas height. Shrubs use the same rule, with a foot as wide as the one of the red tree of their size (`green_shrub` like `red_tree_medium`, `green_shrub_small` like `red_tree_small`), centred on the art, and a band of 10% of the height of the drawn shrub. All five join the groups of rocks and trees (`adr-lw23`): each is grouped by its own cell space, and in a group it blocks exactly its anchor cell, 64 by 64. Code: `scenery.rs` (pixel values of each kind are in its `art` table).

## Pickups

A pickup is one digit: `1` `energy_blue`, `2` `energy_red`, `3` `energy_violet`, `4` `energy_black`. Its character is its cell. See `1.txt` row 5. The HUD line under the title shows what the player has collected.

## Lanterns and markers

A lantern or marker is one character, for example `l` for `lantern`, `L` for `lantern_red`, `m` for `marker` or `M` for `marker_red`. All four have the same size and collision. Its character is its cell, and the art is centred in that cell. The player cannot walk through its bottom half and walks behind the top half. Lanterns and markers do not count in a cluster of rocks and trees (`adr-lw20`).

## Trees

A tree is one character, for example `G` for `green_tree` (and `R` for `red_tree`). The trunk blocks the player. The player can walk behind the tree from above and stops at its base line from below (`adr-lw1`). A tree that touches another tree or a rock is in a cluster and then blocks one cell, the bottom right cell of its footprint (the trunk cell) (`adr-lw18`). Four trees two cells apart in both directions touch and form a grove (see `1.txt`, cells (1,1) to (3,3)).

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
  "O": rock_0
  "o": rock_1
---
   |0 1 2 3 4 5 6 7 8 9 0 1|
 3 |. . . . . . . . . . . .|
 2 |. . . . . . . . . . . .|
 1 |. . . . O o . . . O o .|
 0 |. . . . . . . . . . . .|
```

This places two rocks at (4,1) and (5,1), and two more at (9,1) and (10,1). Each pair overlaps by one cell, so each pair covers 3 cells across and 2 up. The pairs are 3 cells apart, so each pair is its own cluster.

## Checking the result

1. Run the game and press F3.
2. The red grid shows cell numbers. Check that each object is where you expect.
3. Yellow lines show the edge that the player collides with. Cyan lines show rocks in the middle of a cluster. Green squares show the feet of every object.
4. For a picture without the window, run `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:map;quit"`. The picture is saved in `target/screenshots/map.png`.

If the file has a mistake, the game stops when it starts and names the file and line.
