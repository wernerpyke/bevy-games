# Cell areas

## Summary

A cell area is a grid that says which cells of a map belong to an area, such as a forest. The kit makes the blocked areas from the grid and keeps them up to date when cells change while the game runs. A map file may draw only the outline of an area; the loader fills the inside. The game draws the sprites.

## Status

`crates/game-kit/src/cell_area.rs`. Used by Jet (the forest, `adr-jt14`). Decision: `adr-x62`.

## Terms

- **Grid.** `CellGrid`: one flag per cell, cells numbered from the bottom left as in `MapObject::cell`.
- **Outline.** The cells drawn in the map file with the area's legend character.
- **Inside.** The empty cells that cannot reach an empty cell on the map border, stepping up, down, left or right.
- **Area.** `CellArea`: a grid placed in the world, with its cell size, origin and grow.

## Parameters the game supplies

| Parameter | Jet |
|---|---|
| Legend kind and fill rule | `"F": forest fill=enclosed` |
| Cell size and origin | the map's, 16 px |
| Grow (how far each blocked rectangle is grown, `x` left and right, `y` top and bottom) | the actor size (set with the player's collision) |
| Marker on the area entity | `Forest` |
| Sprites | trees drawn from the filled cells |

## Specification

### Fill

1. `fill_enclosed(size, outline)` gives the outline cells plus the inside.
2. An area at the map edge draws its outline along the border cells. An inside cell that is itself on the border is outside, so it is not filled.
3. A gap of one cell in an outline lets the outside in. A corner contact does not: the flood steps up, down, left and right only.
4. A band with no inside stays as drawn.
5. Limit: a clearing closed in on all sides by an outline is filled too. A "never fill" marker does not exist yet.

### From a map

6. `CellGrid::from_map(file, map, kind)` takes the cells of one legend kind. With the legend parameter `fill=enclosed` it applies the fill; without it the cells are used as drawn. Another value of `fill` is an error with the file and the legend line. A kind with no cells gives an empty grid of the map's size.

### Blocked rectangles

7. `CellGrid::rects` joins each run of filled cells in a row with the same run (same first and last column) in the rows above it. `CellArea::blocked_rects` places them in the world from `origin` and `cell_size`, grown by `grow`.

### In the world

8. Add `CellAreaPlugin` (needs `CollisionPlugin`) and spawn a `CellArea` on its own entity, with a marker of the game.
9. When a `CellArea` is inserted, the kit spawns one `Blocker` child (marked `CellAreaBlocker`) per rectangle at once. So a map's `rebuild` hook, which runs right after its `load` hook, already sees them. It also rebuilds the collision world in the next `Update`.
10. `CellArea::set(cell, filled)` changes a cell and returns true when it changed. A cell outside the grid is ignored. In `CellAreaSet` the kit then replaces the blockers, rebuilds the collision world (the nav grid follows) and sends `CellAreaChanged { area, cells }` with the changed cells in the order they were set. After an insert the message has all filled cells.
11. Change cells only with `set`. Never edit the blockers of an area by hand.

## Usage

```rust
app.add_plugins(game_kit::cell_area::CellAreaPlugin);

// In the map's load hook:
let grid = CellGrid::from_map(file, &map, "forest")?;
commands.spawn((Forest, CellArea::new(grid, CELL_SIZE, origin, grow)));

// Draw the sprites, after CellAreaSet:
fn redraw(mut changed: MessageReader<CellAreaChanged>, areas: Query<&CellArea, With<Forest>>) { .. }
```

## Limits

- The collision rebuild after a change clears the temporary blocked areas (`collision.md`, `add_temporary`), as every rebuild does.
- No "never fill" marker for a closed clearing (rule 5).
- One fill rule. An area drawn cell by cell uses no `fill` parameter.

## Tests

`crates/game-kit/src/cell_area.rs`: the fill (closed outline, outline along the border, gap and corner contact, band), the rectangles, `set`, `from_map` with and without `fill` and its error, the world rectangles, the blockers ready before a load rebuild, and a change during play (blockers, collision world, message). Run `./dev-scripts/rust.sh test game-kit cell_area`.
