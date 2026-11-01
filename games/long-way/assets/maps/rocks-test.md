---
map: rocks-test
cell: 64
origin: bottom-left
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---

# Rocks test

Notes about this map. Anything outside the `# Features` and `# Map` sections is ignored by the game.

# Features

Each `##` heading defines one feature: a name, optional notes, and one `cells` block. Each
character of the block is one object. Rows run from the top (highest y) down. The bottom left
character is the anchor that the `# Map` section places.

## Rocks test

A small cluster of rocks for testing collision.

```cells
. . 1 1 . .
. 0 0 1 1 .
. 0 0 . . .
. . 0 0 . .
```

## Reusable cluster

A cluster defined once and placed several times. The copies are ordinary rocks, so copies that
overlap merge into one cluster.

```cells
. 0 0 .
0 1 1 0
. 0 0 .
```

# Map

Each bullet places a feature. Positions are cells, in the same numbers as the F2 grid, and are
the cell of the feature's bottom left character.

- 'Rocks test' at (21,9)
- 'Reusable cluster' at (2,12), (2,3) and (6,3)
