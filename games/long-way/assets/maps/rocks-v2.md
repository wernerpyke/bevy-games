---
map: rocks-v2
cell: 64
origin: bottom-left
pitch: 2
draw: 160
jitter: 0
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---

# Rocks V2

Placement experiment variant V2: one character per 2 cells, rocks drawn at 160 pixels
(slight overlap), jitter 0. The painting is the same as in `rocks-test.md`. See
`dev-docs/PLAN-forest-collision.md`.

```cells at=21,9
. . 1 1 . .
. 0 0 1 1 .
. 0 0 . . .
. . 0 0 . .
```
