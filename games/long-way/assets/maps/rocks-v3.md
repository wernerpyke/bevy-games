---
map: rocks-v3
cell: 64
origin: bottom-left
pitch: 2
draw: 160
jitter: 8
legend:
  ".": empty
  "0": rock_0
  "1": rock_1
---

# Rocks V3

Placement experiment variant V3: one character per 2 cells, rocks drawn at 160 pixels
(slight overlap), jitter 8. The painting is the same as in `rocks-test.md`. See
`dev-docs/PLAN-forest-collision.md`.

```cells at=21,9
. . 1 1 . .
. 0 0 1 1 .
. 0 0 . . .
. . 0 0 . .
```
