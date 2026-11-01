# Game maps

How to write a map file for Long Way. A map is a markdown file. You can read it, edit it and see it in the game. The game reads only the parts described here. Everything else is for people.

- Files are in `games/long-way/assets/maps/`.
- The game loads one map. Its name is set in `games/long-way/src/map.rs` (`MAP_NAME` and `MAP_SOURCE`). After you edit the file, rebuild the game.
- The F2 key shows a grid with cell numbers. Use these numbers for positions in the map.
- The current example is `rocks-test.md`.

## The three parts of a map file

1. **Frontmatter.** Settings at the top, between two `---` lines.
2. **Features.** The section `# Features`. It describes what can be drawn.
3. **Map.** The section `# Map`. It says where each feature goes.

Any other heading, text, list or code block is a note for people. The game ignores it. Write what you like around the three parts.

## 1. Frontmatter

```yaml
---
map: rocks-test
cell: 64
origin: bottom-left
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---
```

| Key | Needed | Meaning |
|---|---|---|
| `map` | yes | The name of the map. It appears in log and error messages. |
| `cell` | yes | The size of one grid cell in pixels. Long Way uses 64. |
| `origin` | yes | Must be `bottom-left`. Cell (0, 0) is the bottom left cell, as on the F2 grid. |
| `legend` | yes | Says what each character in a `cells` block means. See below. |
| `pitch` | no | Cells between two characters. Default 1. |
| `draw` | no | Drawn size of a rock in pixels. Default 128. Use a multiple of 32. Rocks only. |
| `jitter` | no | Largest random shift of each object in pixels. Default 0. |

Rules for the frontmatter:
- Write one `key: value` per line.
- A `#` starts a comment when it is at the start of a line or after a space.
- A legend character must be one character. Put it in quotes, for example `"#"`.
- The character `.` always means empty. Write it as `".": empty`.
- The kinds you can use now are `rock_0`, `rock_1`, `green_tree` and `red_tree`.

## 2. Features

A feature is a drawing in text. It can be one rock or a whole pile of rocks. You define it once. You can place it as many times as you like.

Write each feature like this:

````markdown
# Features

## Shrub patch

Notes about this feature. Write what you like here.

```cells
. 0 0 .
0 1 1 0
. 0 0 .
```
````

Rules:
- The `##` heading is the name of the feature. Use the same name in the `# Map` section.
- A feature has exactly one `cells` block. Start it with three backticks and the word `cells`. Do not add anything else on that line.
- Each character is one object. Put a space between characters.
- Use the characters from the legend. A `.` is an empty cell.
- Every row must have the same number of characters. No blank lines inside the block.
- Write the rows from top to bottom. The first row is the highest.
- The bottom left character is the **anchor**. When you place the feature, the anchor goes on the position you give.

### How big is a rock?

One rock covers 2 by 2 cells. The character you write is the bottom left cell of that rock. The rock extends one cell to the right and one cell up. A tree also has a 2 by 2 cell footprint, with its bottom left corner on the cell you write.

Characters that are next to each other overlap by one cell. This is on purpose. It makes a pile of rocks that looks like one cluster. A solid block of 4 by 3 characters gives a cluster of 5 by 4 cells.

### Trees

A tree is a feature with one character, for example `g` for `green_tree`. The trunk blocks Guy. Guy can walk behind the tree from above and overlap its trunk from below by half of his height (`adr-lw1`). Trunks that touch rocks join the rock cluster. `draw` applies to rocks only. `jitter` also shifts trees.

### Rocks and collision

Guy cannot walk through a rock. The game joins the rocks of a cluster and Guy walks around the edge of the whole cluster. Rocks in the middle of a cluster have no edge and cost no extra work.

## 3. Map

The `# Map` section places features. Write one bullet for each feature.

````markdown
# Map

- 'Shrub patch' at (2,12), (2,3) and (6,3)
- 'Rocks test' at (21,9)
````

Rules:
- Start the bullet with `-` or `*`.
- Write the feature name in single or double quotes. It must match the `##` heading exactly, with the same capital letters.
- Write `at`, then one or more positions in parentheses: `(x,y)`.
- Separate positions with commas. You can use `and` before the last one.
- A position is a cell number from the F2 grid. It is where the anchor goes.
- Positions can be negative.
- Copies can overlap. Overlapping copies join into one cluster.
- You can write the `# Features` section after the `# Map` section if you prefer.

## Errors

If the file has a mistake, the game stops when it starts and tells you the file and line. For example:

```
assets/maps/rocks-test.md:15: character `x` is not in the legend
```

The most common mistakes:
- A character that is not in the legend.
- A row with a different number of characters than the first row.
- Two characters with no space between them.
- A blank line inside a `cells` block.
- A `cells` block that is not under a `##` heading in `# Features`.
- A feature with no `cells` block, or with two.
- Two features with the same name.
- A bullet that uses a feature name that is not defined.
- A position that is not two whole numbers, for example `(2,x)`.
- A code block that is not closed.

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

This map places two rocks at each of two positions. The pair at (4,4) covers cells 4 to 6 across and 4 to 5 up, because the two rocks overlap by one cell. Each pair is one cluster. The second pair is a separate cluster.

## Checking the result

1. Run the game and press F2.
2. The red grid shows cell numbers. Check that each feature starts where you expect.
3. Yellow lines show the edge that Guy collides with. Cyan lines show rocks in the middle of a cluster. Green squares show the feet of every object.
4. For a picture without the window, use `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:map;quit"`. The picture is saved in `target/screenshots/map.png`.

## Where the rules are

- The parser: `games/long-way/src/map.rs`, with tests.
- How rocks become collision: `dev-docs/BEVY-ADDITIONS.md`, section "Collision and sliding rules".
- Why the format is like this: `dev-docs/adr/LONG-WAY.md`, `adr-lw8`.
