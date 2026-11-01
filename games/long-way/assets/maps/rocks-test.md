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

A small cluster of rocks for testing collision. Anything outside the `cells` block is
ignored by the game, so notes and sketches can go here.

Each character is one object. Rows run from the top (highest y) down. `at=10,6` is the
cell of the bottom left character, in the same cell numbers as the F2 grid.

```cells at=10,6
. . 1 1 . .
. 0 0 1 1 .
. 0 0 . . .
. . 0 0 . .
```
