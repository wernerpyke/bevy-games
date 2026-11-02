---
map: test
cell: 16
origin: bottom-left
legend:
  ".": empty
  "g": tree_big
  "d": tree_dead
---

# R-Town test map

An 80 by 60 cell test map: a scattering of trees on open ground, to try collision, scrolling and art. The hero
starts at the centre, cell (40, 30). The area around it is free of trees. Anything outside the `# Features` and
`# Map` sections is ignored by the game.

The map size is not in this file. It is `MAP_CELLS` in `src/map.rs`.

# Features

## Big tree

`bg_18` in `background-elements.aseprite`. Stands on its cell: the bottom centre of the art is the bottom centre of
the cell. The base blocks the hero, the rest is walked behind.

```cells
g
```

## Dead tree

`bg_19` in `background-elements.aseprite`. Same rules as the big tree.

```cells
d
```

# Map

- 'Big tree' at (72,30), (40,43), (32,5), (31,2), (41,54), (45,42), (62,10), (15,21)
- 'Big tree' at (39,49), (70,18), (57,41), (76,6), (28,35), (55,11), (74,56), (75,25)
- 'Big tree' at (54,29), (65,53), (55,5), (65,20), (8,18), (47,45), (39,19), (45,21)
- 'Big tree' at (56,18), (39,10), (35,54), (59,2), (29,47), (52,26), (13,45), (52,7)
- 'Big tree' at (3,17), (57,35), (19,44), (52,42), (30,27), (32,37), (19,48), (53,56)
- 'Big tree' at (28,4), (64,42), (48,2), (37,7), (36,19), (21,26), (17,51), (21,18)
- 'Big tree' at (31,54), (77,46), (15,7), (76,2), (65,26), (62,49), (33,46), (52,47)
- 'Big tree' at (9,2), (60,13), (19,36), (26,9), (18,30), (33,33), (10,22), (30,42)
- 'Big tree' at (55,1), (51,29), (5,11), (31,18)

- 'Dead tree' at (36,13), (36,40), (7,25), (3,25), (58,50), (36,44), (51,53), (50,16)
- 'Dead tree' at (24,36), (29,11), (74,36), (6,50), (43,48), (12,18), (51,4), (23,45)
- 'Dead tree' at (5,2), (16,41), (9,44), (27,15), (46,12), (19,40)
