# Neighbour groups

## Summary

Blocking items that touch form a group. A game gives an item that stands alone a thin blocked area, so the player walks behind it, and gives every member of a group a larger area, so the group is one solid shape with one outline (`collision.md`, `adr-x10`). The kit supplies the grouping function. The game supplies the areas.

## Status

`crates/game-kit/src/groups.rs`, used by R-Town (`adr-rt9`) and Long Way (`adr-lw14`). Decision: `adr-x20`.

## Terms

- **Item.** One blocking object on the map grid. It covers `size` cells from its anchor cell up and to the right.
- **Neighbours.** With `EdgesAndCorners`, the 8 cells around a cell. With `Edges`, the 4 cells that share an edge.
- **Group.** Items linked by a chain of items whose cells overlap or are neighbours. Kinds do not matter, a mixture is one group.
- **Alone.** A group of size 1.

## Parameters the game supplies

| Parameter | R-Town | Long Way |
|---|---|---|
| Item size in cells | 1 x 1 | per kind: rocks and trees 2 x 2, Zen Garden kinds 2 x 2, 1 x 2 or 1 x 1 |
| Adjacency | `EdgesAndCorners` | `EdgesAndCorners` |
| Area when alone | base band, 4 px deep | tree: trunk band, rock: bottom 10% of the footprint |
| Area in a group | the whole cell | rock: the footprint rule, tree: the trunk cell (`games/long-way/docs/collision.md`, `adr-lw18`) |

## Specification

1. `neighbour_groups(anchors, size, adjacency)` returns one group id per item, in input order. The id is the index of the first item of the group.
2. `group_sizes(groups)` returns the size of each item's group. A size of 1 means the item is alone.
3. Two items are linked when their cells overlap or touch, by the chosen adjacency. A corner contact links items with `EdgesAndCorners`.
4. The game chooses the blocked area from the group size, then registers it with the collision world as usual. The kit outline merges areas that touch or overlap.
5. Every blocking kind goes through the same call. A new blocking kind must be counted in the game's list of blocking objects.
6. Changing the map changes the groups. Adding or removing one item changes the area of its neighbours.

## Usage

- Collect the anchor cells of all blocking objects of the map. Call `neighbour_groups`, then `group_sizes`.
- Pick the area per object with a plain function that takes the group size, so it is unit tested.
- A group member should reach every side where another member touches it, or a gap stays open. Long Way drops the top inset of a rock that has a member above it.

## Limits

- Groups come from the grid cells, not from the drawn art.
- The full-cell footprint is a first rule. `rm7` covers a review by playing.

## Tests

`crates/game-kit/src/groups.rs` (run `./dev-scripts/rust.sh test game-kit`). Game tests: `games/r-town/src/trees.rs`, `games/long-way/src/rocks.rs`, `games/long-way/src/trees.rs`.
